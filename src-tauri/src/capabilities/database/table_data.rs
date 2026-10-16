//! Bounded table data operations. SQL identifiers come from the live catalog;
//! cell values are always driver parameters, never interpolated SQL. Original
//! rows and preview tickets stay on the session worker, not in application logs.
use super::database::{DbPool, QueryCancel};
use crate::{domain::DbConnectionConfig, security_policy as sp};
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::{Duration, Instant},
};

const MAX_COLUMNS: usize = 128;
const MAX_ROWS: usize = 100;
const MAX_CELL: usize = 65_536;
const MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct TableRequest {
    pub name: String,
    pub schema: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub filters: Vec<TableFilter>,
    #[serde(default)]
    pub order: Vec<TableOrder>,
    #[serde(default)]
    pub changes: Vec<TableChange>,
    pub ticket: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TableFilter {
    pub column: String,
    pub operator: String,
    pub value: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TableOrder {
    pub column: String,
    #[serde(default)]
    pub descending: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TableChange {
    pub action: String,
    pub row_id: Option<String>,
    #[serde(default)]
    pub values: BTreeMap<String, Option<String>>,
}
#[derive(Clone, Debug, Serialize)]
struct Column {
    name: String,
    datatype: String,
    nullable: bool,
    generated: bool,
    primary: bool,
    binary: bool,
}
#[derive(Clone, Debug, Serialize)]
struct Catalog {
    name: String,
    schema: Option<String>,
    columns: Vec<Column>,
    transactional: bool,
    is_table: bool,
}
#[derive(Clone)]
struct OriginalRow {
    table_digest: String,
    values: Vec<Option<String>>,
}
#[derive(Clone)]
struct PreparedChange {
    action: String,
    sql: String,
    parameters: Vec<Option<String>>,
    original: Option<Vec<Option<String>>>,
}
struct Preview {
    catalog: Catalog,
    digest: String,
    statements: Vec<PreparedChange>,
    expires: Instant,
}
#[derive(Default)]
pub struct TableState {
    originals: HashMap<String, OriginalRow>,
    previews: HashMap<String, Preview>,
}
struct Rows {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
    text_only: bool,
}

fn identifier(name: &str, mysql: bool) -> Result<String, String> {
    if name.is_empty() || name.len() > 255 || name.chars().any(|c| c.is_control()) {
        return Err("DB_TABLE_IDENTIFIER_INVALID".into());
    }
    let q = if mysql { '`' } else { '"' };
    Ok(format!("{q}{}{q}", name.replace(q, &format!("{q}{q}"))))
}
fn table_name(catalog: &Catalog, mysql: bool) -> Result<String, String> {
    let name = identifier(&catalog.name, mysql)?;
    match &catalog.schema {
        Some(schema) => Ok(format!("{}.{}", identifier(schema, mysql)?, name)),
        None => Ok(name),
    }
}
fn digest(catalog: &Catalog) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(catalog).map_err(|_| "DB_TABLE_CATALOG_INVALID")?)
    ))
}
fn stop(cancel: &QueryCancel) -> Result<(), String> {
    if cancel.is_cancelled() {
        Err("DB_TABLE_CANCELLED".into())
    } else {
        Ok(())
    }
}
fn bounded_text(value: Option<String>) -> Result<Option<String>, String> {
    if value
        .as_ref()
        .is_some_and(|v| v.len() > MAX_CELL || v.contains('\0'))
    {
        return Err("DB_TABLE_CELL_TOO_LARGE_OR_BINARY".into());
    }
    Ok(value)
}
fn mysql_value(value: mysql::Value, text_only: &mut bool) -> Option<String> {
    use mysql::Value::*;
    match value {
        NULL => None,
        Int(v) => Some(v.to_string()),
        UInt(v) => Some(v.to_string()),
        Float(v) => Some(v.to_string()),
        Double(v) => Some(v.to_string()),
        Bytes(v) => match String::from_utf8(v) {
            Ok(text) => Some(text),
            Err(e) => {
                *text_only = false;
                Some(format!("<binary {} bytes>", e.as_bytes().len()))
            }
        },
        Date(y, m, d, h, n, s, us) => {
            Some(format!("{y:04}-{m:02}-{d:02} {h:02}:{n:02}:{s:02}.{us:06}"))
        }
        Time(negative, days, h, m, s, us) => Some(format!(
            "{}{}:{m:02}:{s:02}.{us:06}",
            if negative { "-" } else { "" },
            days as u64 * 24 + h as u64
        )),
    }
}
/// Only internally constructed bounded statements reach this helper.
fn query(
    pool: &mut DbPool,
    sql: &str,
    parameters: &[Option<String>],
    max: usize,
    cancel: &QueryCancel,
) -> Result<Rows, String> {
    stop(cancel)?;
    let mut output = Rows {
        columns: Vec::new(),
        rows: Vec::new(),
        text_only: true,
    };
    let mut bytes = 0usize;
    match pool {
        DbPool::Sqlite(conn) => {
            let mut statement = conn.prepare(sql).map_err(|_| "DB_TABLE_QUERY_FAILED")?;
            output.columns = statement
                .column_names()
                .iter()
                .map(|v| v.to_string())
                .collect();
            let width = statement.column_count();
            let values = parameters.iter().map(|v| match v {
                Some(text) => rusqlite::types::Value::Text(text.clone()),
                None => rusqlite::types::Value::Null,
            });
            let mut rows = statement
                .query(rusqlite::params_from_iter(values))
                .map_err(|_| "DB_TABLE_QUERY_FAILED")?;
            while let Some(row) = rows.next().map_err(|_| "DB_TABLE_QUERY_FAILED")? {
                stop(cancel)?;
                if output.rows.len() >= max {
                    return Err("DB_TABLE_RESULT_LIMIT".into());
                }
                let mut values = Vec::with_capacity(width);
                for i in 0..width {
                    use rusqlite::types::ValueRef;
                    let value = match row.get_ref(i).map_err(|_| "DB_TABLE_VALUE_FAILED")? {
                        ValueRef::Null => None,
                        ValueRef::Integer(v) => Some(v.to_string()),
                        ValueRef::Real(v) => Some(v.to_string()),
                        ValueRef::Text(v) => Some(
                            std::str::from_utf8(v)
                                .map_err(|_| "DB_TABLE_NON_UTF8_VALUE")?
                                .to_owned(),
                        ),
                        ValueRef::Blob(v) => {
                            output.text_only = false;
                            Some(format!("<binary {} bytes>", v.len()))
                        }
                    };
                    let value = bounded_text(value)?;
                    bytes += value.as_ref().map_or(0, String::len);
                    if bytes > MAX_BYTES {
                        return Err("DB_TABLE_RESULT_LIMIT".into());
                    }
                    values.push(value);
                }
                output.rows.push(values);
            }
        }
        DbPool::MySql(conn, _) => {
            let values = parameters
                .iter()
                .map(|v| match v {
                    Some(text) => mysql::Value::Bytes(text.as_bytes().to_vec()),
                    None => mysql::Value::NULL,
                })
                .collect::<Vec<_>>();
            let mut result = conn
                .exec_iter(sql, mysql::Params::Positional(values))
                .map_err(|_| "DB_TABLE_QUERY_FAILED")?;
            output.columns = result
                .columns()
                .as_ref()
                .iter()
                .map(|c| c.name_str().into_owned())
                .collect();
            for row in result.by_ref() {
                stop(cancel)?;
                if output.rows.len() >= max {
                    return Err("DB_TABLE_RESULT_LIMIT".into());
                }
                let row = row.map_err(|_| "DB_TABLE_QUERY_FAILED")?;
                let mut values = Vec::with_capacity(row.len());
                for value in row.unwrap() {
                    let value = bounded_text(mysql_value(value, &mut output.text_only))?;
                    bytes += value.as_ref().map_or(0, String::len);
                    if bytes > MAX_BYTES {
                        return Err("DB_TABLE_RESULT_LIMIT".into());
                    }
                    values.push(value);
                }
                output.rows.push(values);
            }
        }
        DbPool::Postgres(_) => return Err("DB_TABLE_POSTGRES_ADAPTER_NOT_READY".into()),
    }
    Ok(output)
}
fn execute(pool: &mut DbPool, sql: &str, parameters: &[Option<String>]) -> Result<u64, String> {
    match pool {
        DbPool::Sqlite(conn) => {
            let values = parameters.iter().map(|v| match v {
                Some(text) => rusqlite::types::Value::Text(text.clone()),
                None => rusqlite::types::Value::Null,
            });
            conn.execute(sql, rusqlite::params_from_iter(values))
                .map(|n| n as u64)
                .map_err(|_| "DB_TABLE_MUTATION_FAILED".to_owned())
        }
        DbPool::MySql(conn, _) => {
            // Only fixed transaction controls use the text protocol. MySQL 8
            // rejects START TRANSACTION in the prepared-statement protocol.
            // All mutations, including zero-parameter inserts, stay prepared.
            if parameters.is_empty() && matches!(sql, "START TRANSACTION" | "ROLLBACK") {
                conn.query_drop(sql)
                    .map_err(|_| "DB_TABLE_MUTATION_FAILED".to_owned())?;
                return Ok(0);
            }
            let values = parameters
                .iter()
                .map(|v| match v {
                    Some(text) => mysql::Value::Bytes(text.as_bytes().to_vec()),
                    None => mysql::Value::NULL,
                })
                .collect::<Vec<_>>();
            conn.exec_drop(sql, mysql::Params::Positional(values))
                .map_err(|_| "DB_TABLE_MUTATION_FAILED".to_owned())?;
            Ok(conn.affected_rows())
        }
        DbPool::Postgres(_) => Err("DB_TABLE_POSTGRES_ADAPTER_NOT_READY".into()),
    }
}
fn value(row: &[Option<String>], index: usize) -> &str {
    row.get(index).and_then(Option::as_deref).unwrap_or("")
}
fn catalog(
    pool: &mut DbPool,
    request: &TableRequest,
    cancel: &QueryCancel,
) -> Result<Catalog, String> {
    identifier(&request.name, matches!(pool, DbPool::MySql(..)))?;
    if request.schema.is_some() {
        return Err("DB_TABLE_USE_SELECTED_DATABASE".into());
    }
    let parameter = [Some(request.name.clone())];
    let mut result = Catalog {
        name: request.name.clone(),
        schema: None,
        columns: Vec::new(),
        transactional: false,
        is_table: false,
    };
    match pool {
        DbPool::Sqlite(_) => {
            let kind = query(pool, "SELECT type,sql FROM sqlite_master WHERE name=? AND type IN ('table','view') LIMIT 1", &parameter, 1, cancel)?;
            let row = kind.rows.first().ok_or("DB_TABLE_NOT_FOUND")?;
            result.is_table = value(row, 0) == "table"
                && !value(row, 1)
                    .trim_start()
                    .to_ascii_uppercase()
                    .starts_with("CREATE VIRTUAL TABLE");
            result.transactional = result.is_table;
            let columns = query(pool, "SELECT name,type,\"notnull\",pk,hidden FROM pragma_table_xinfo(?) ORDER BY cid LIMIT 129", &parameter, MAX_COLUMNS+1, cancel)?;
            for row in columns.rows {
                let datatype = value(&row, 1).to_string();
                result.columns.push(Column {
                    name: value(&row, 0).into(),
                    datatype: datatype.clone(),
                    nullable: value(&row, 2) == "0",
                    primary: value(&row, 3).parse::<i64>().unwrap_or(0) > 0,
                    generated: value(&row, 4) != "0",
                    binary: datatype.to_ascii_uppercase().contains("BLOB"),
                });
            }
        }
        DbPool::MySql(..) => {
            let kind = query(pool, "SELECT TABLE_TYPE,ENGINE FROM information_schema.tables WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME=? LIMIT 1", &parameter, 1, cancel)?;
            let row = kind.rows.first().ok_or("DB_TABLE_NOT_FOUND")?;
            result.is_table = value(row, 0) == "BASE TABLE";
            result.transactional = value(row, 1).eq_ignore_ascii_case("InnoDB");
            let columns = query(pool, "SELECT COLUMN_NAME,COLUMN_TYPE,IS_NULLABLE,COLUMN_KEY,EXTRA,DATA_TYPE FROM information_schema.columns WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME=? ORDER BY ORDINAL_POSITION LIMIT 129", &parameter, MAX_COLUMNS+1, cancel)?;
            for row in columns.rows {
                let family = value(&row, 5).to_ascii_lowercase();
                result.columns.push(Column {
                    name: value(&row, 0).into(),
                    datatype: value(&row, 1).into(),
                    nullable: value(&row, 2) == "YES",
                    primary: value(&row, 3) == "PRI",
                    generated: value(&row, 4).to_ascii_uppercase().contains("GENERATED"),
                    binary: matches!(
                        family.as_str(),
                        "blob"
                            | "tinyblob"
                            | "mediumblob"
                            | "longblob"
                            | "binary"
                            | "varbinary"
                            | "geometry"
                            | "bit"
                    ),
                });
            }
        }
        DbPool::Postgres(_) => return Err("DB_TABLE_POSTGRES_ADAPTER_NOT_READY".into()),
    }
    if result.columns.is_empty() || result.columns.len() > MAX_COLUMNS {
        return Err("DB_TABLE_COLUMN_LIMIT".into());
    }
    Ok(result)
}
fn column<'a>(catalog: &'a Catalog, name: &str) -> Result<&'a Column, String> {
    catalog
        .columns
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| String::from("DB_TABLE_UNKNOWN_COLUMN"))
}
fn readable_columns(catalog: &Catalog, mysql: bool) -> Result<String, String> {
    catalog
        .columns
        .iter()
        .map(|c| identifier(&c.name, mysql))
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.join(","))
}
fn writable(catalog: &Catalog, cfg: &DbConnectionConfig, pool: &DbPool) -> bool {
    catalog.is_table
        && catalog.transactional
        && catalog.columns.iter().any(|c| c.primary)
        && !catalog.columns.iter().any(|c| c.binary)
        && cfg.allow_write
        && sp::is_production_database(&sp::build_db_signals(cfg, pool.encrypted()))
            == sp::ProductionVerdict::NonProduction
}
fn where_filters(
    catalog: &Catalog,
    filters: &[TableFilter],
    mysql: bool,
) -> Result<(String, Vec<Option<String>>), String> {
    if filters.len() > 16 {
        return Err("DB_TABLE_FILTER_LIMIT".into());
    }
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    for filter in filters {
        let col = column(catalog, &filter.column)?;
        if col.binary {
            return Err("DB_TABLE_BINARY_FILTER_DENIED".into());
        }
        let name = identifier(&col.name, mysql)?;
        let clause = match filter.operator.as_str() {
            "is_null" => format!("{name} IS NULL"),
            "not_null" => format!("{name} IS NOT NULL"),
            "eq" | "ne" | "lt" | "le" | "gt" | "ge" => {
                if filter.value.is_none() {
                    return Err("DB_TABLE_USE_NULL_OPERATOR".into());
                }
                let op = match filter.operator.as_str() {
                    "eq" => "=",
                    "ne" => "<>",
                    "lt" => "<",
                    "le" => "<=",
                    "gt" => ">",
                    _ => ">=",
                };
                parameters.push(bounded_text(filter.value.clone())?);
                format!("{name} {op} ?")
            }
            "contains" => {
                let text = filter
                    .value
                    .as_deref()
                    .ok_or("DB_TABLE_FILTER_VALUE_REQUIRED")?;
                let pattern = format!(
                    "%{}%",
                    text.replace('!', "!!")
                        .replace('%', "!%")
                        .replace('_', "!_")
                );
                parameters.push(bounded_text(Some(pattern))?);
                format!("{name} LIKE ? ESCAPE '!'")
            }
            _ => return Err("DB_TABLE_FILTER_OPERATOR_DENIED".into()),
        };
        clauses.push(clause);
    }
    Ok((
        if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        },
        parameters,
    ))
}
fn ordering(
    catalog: &Catalog,
    requested: &[TableOrder],
    mysql: bool,
) -> Result<(String, bool), String> {
    if requested.len() > 8 {
        return Err("DB_TABLE_SORT_LIMIT".into());
    }
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for sort in requested {
        let col = column(catalog, &sort.column)?;
        if col.binary {
            return Err("DB_TABLE_BINARY_SORT_DENIED".into());
        }
        if seen.insert(col.name.clone()) {
            items.push(format!(
                "{} {}",
                identifier(&col.name, mysql)?,
                if sort.descending { "DESC" } else { "ASC" }
            ));
        }
    }
    let mut stable = false;
    for col in &catalog.columns {
        if col.primary {
            stable = true;
            if seen.insert(col.name.clone()) {
                items.push(format!("{} ASC", identifier(&col.name, mysql)?));
            }
        }
    }
    if items.is_empty() {
        if let Some(col) = catalog.columns.iter().find(|c| !c.binary) {
            items.push(format!("{} ASC", identifier(&col.name, mysql)?));
        }
    }
    Ok((
        if items.is_empty() {
            String::new()
        } else {
            format!(" ORDER BY {}", items.join(","))
        },
        stable,
    ))
}
pub fn read(
    pool: &mut DbPool,
    cfg: &DbConnectionConfig,
    state: &mut TableState,
    request: &TableRequest,
    cancel: &QueryCancel,
) -> Result<Value, String> {
    if request.offset > 50_000 || request.offset % 100 != 0 {
        return Err("DB_TABLE_OFFSET_LIMIT".into());
    }
    let catalog = catalog(pool, request, cancel)?;
    let mysql = matches!(pool, DbPool::MySql(..));
    let table = table_name(&catalog, mysql)?;
    let (filter, parameters) = where_filters(&catalog, &request.filters, mysql)?;
    let (order, stable) = ordering(&catalog, &request.order, mysql)?;
    let sql = format!(
        "SELECT {} FROM {table}{filter}{order} LIMIT {} OFFSET {}",
        readable_columns(&catalog, mysql)?,
        MAX_ROWS + 1,
        request.offset
    );
    sp::evaluate_db_query_gate(&sql, cfg, pool.encrypted(), false)
        .map_err(|c| c.as_str().to_string())?;
    let mut rows = query(pool, &sql, &parameters, MAX_ROWS + 1, cancel)?;
    if rows.columns.len() != catalog.columns.len() {
        return Err("DB_TABLE_ROW_SHAPE_CHANGED".into());
    }
    let more = rows.rows.len() > MAX_ROWS;
    rows.rows.truncate(MAX_ROWS);
    let editable = writable(&catalog, cfg, pool) && rows.text_only;
    let table_digest = digest(&catalog)?;
    // Changing the loaded dataset invalidates old cell edits and old previews.
    state.originals.clear();
    state.previews.clear();
    let values = rows
        .rows
        .into_iter()
        .map(|values| {
            let id = uuid::Uuid::new_v4().to_string();
            if editable {
                state.originals.insert(
                    id.clone(),
                    OriginalRow {
                        table_digest: table_digest.clone(),
                        values: values.clone(),
                    },
                );
            }
            json!({"row_id":id,"values":values})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"table":catalog,"rows":values,"offset":request.offset,"page_size":MAX_ROWS,"has_more":more,"stable_order":stable,"editable":editable,"read_only_reason":if editable{""}else{"requires_nonproduction_write_permission_primary_key_transactional_table_and_text_values"},"sql":sql,"parameters":parameters}),
    )
}
fn row_predicate(
    catalog: &Catalog,
    values: &[Option<String>],
    mysql: bool,
) -> Result<(String, Vec<Option<String>>), String> {
    if values.len() != catalog.columns.len() {
        return Err("DB_TABLE_ROW_SHAPE_CHANGED".into());
    }
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    for (i, col) in catalog
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.primary)
    {
        let name = identifier(&col.name, mysql)?;
        if values[i].is_none() {
            return Err("DB_TABLE_NULL_PRIMARY_KEY".into());
        }
        clauses.push(format!("{name} = ?"));
        parameters.push(values[i].clone());
    }
    if clauses.is_empty() {
        return Err("DB_TABLE_PRIMARY_KEY_REQUIRED".into());
    }
    Ok((clauses.join(" AND "), parameters))
}
pub fn preview(
    pool: &mut DbPool,
    cfg: &DbConnectionConfig,
    state: &mut TableState,
    request: &TableRequest,
    cancel: &QueryCancel,
) -> Result<Value, String> {
    if request.changes.is_empty() || request.changes.len() > MAX_ROWS {
        return Err("DB_TABLE_CHANGE_LIMIT".into());
    }
    if serde_json::to_vec(&request.changes)
        .map_err(|_| "DB_TABLE_CHANGES_INVALID")?
        .len()
        > MAX_BYTES
    {
        return Err("DB_TABLE_CHANGE_BYTES_LIMIT".into());
    }
    let catalog = catalog(pool, request, cancel)?;
    if !writable(&catalog, cfg, pool) {
        return Err("DB_TABLE_WRITE_DENIED".into());
    }
    let table_digest = digest(&catalog)?;
    let mysql = matches!(pool, DbPool::MySql(..));
    let table = table_name(&catalog, mysql)?;
    let mut statements = Vec::new();
    let mut seen = HashSet::new();
    for change in &request.changes {
        if !["insert", "update", "delete"].contains(&change.action.as_str()) {
            return Err("DB_TABLE_CHANGE_ACTION_DENIED".into());
        }
        let original = if change.action == "insert" {
            if change.row_id.is_some() {
                return Err("DB_TABLE_INSERT_HAS_ROW_ID".into());
            }
            None
        } else {
            let id = change.row_id.as_ref().ok_or("DB_TABLE_ROW_ID_REQUIRED")?;
            if !seen.insert(id) {
                return Err("DB_TABLE_DUPLICATE_ROW_EDIT".into());
            }
            let row = state
                .originals
                .get(id)
                .ok_or("DB_TABLE_ROW_EXPIRED_RELOAD_REQUIRED")?;
            if row.table_digest != table_digest {
                return Err("DB_TABLE_SCHEMA_CHANGED".into());
            }
            Some(row.values.clone())
        };
        let mut parameters = Vec::new();
        let mut names = Vec::new();
        for (name, value) in &change.values {
            let col = column(&catalog, name)?;
            if col.generated || col.binary {
                return Err("DB_TABLE_COLUMN_NOT_EDITABLE".into());
            }
            if value.is_none() && !col.nullable && !col.primary {
                return Err("DB_TABLE_NULL_NOT_ALLOWED".into());
            }
            names.push(identifier(name, mysql)?);
            parameters.push(bounded_text(value.clone())?);
        }
        let sql = match change.action.as_str() {
            "insert" => {
                if names.is_empty() {
                    if mysql {
                        format!("INSERT INTO {table} () VALUES ()")
                    } else {
                        format!("INSERT INTO {table} DEFAULT VALUES")
                    }
                } else {
                    format!(
                        "INSERT INTO {table} ({}) VALUES ({})",
                        names.join(","),
                        vec!["?"; names.len()].join(",")
                    )
                }
            }
            "update" => {
                if names.is_empty() {
                    return Err("DB_TABLE_EMPTY_UPDATE".into());
                }
                let (predicate, keys) = row_predicate(
                    &catalog,
                    original.as_ref().ok_or("DB_TABLE_ROW_REQUIRED")?,
                    mysql,
                )?;
                parameters.extend(keys);
                format!(
                    "UPDATE {table} SET {} WHERE {predicate}",
                    names
                        .iter()
                        .map(|n| format!("{n} = ?"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
            _ => {
                if !names.is_empty() {
                    return Err("DB_TABLE_DELETE_HAS_VALUES".into());
                }
                let (predicate, keys) = row_predicate(
                    &catalog,
                    original.as_ref().ok_or("DB_TABLE_ROW_REQUIRED")?,
                    mysql,
                )?;
                parameters = keys;
                format!("DELETE FROM {table} WHERE {predicate}")
            }
        };
        sp::evaluate_db_query_gate(&sql, cfg, pool.encrypted(), true)
            .map_err(|c| c.as_str().to_string())?;
        statements.push(PreparedChange {
            action: change.action.clone(),
            sql,
            parameters,
            original,
        });
    }
    state.previews.retain(|_, p| p.expires > Instant::now());
    if state.previews.len() >= 4 {
        return Err("DB_TABLE_PREVIEW_LIMIT".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let display = statements
        .iter()
        .map(|s| json!({"action":s.action,"sql":s.sql,"parameters":s.parameters}))
        .collect::<Vec<_>>();
    state.previews.insert(
        id.clone(),
        Preview {
            catalog,
            digest: table_digest,
            statements,
            expires: Instant::now() + Duration::from_secs(300),
        },
    );
    Ok(
        json!({"ticket":id,"statements":display,"expires_in_seconds":300,"requires_confirmation":true,"requires_separate_commit":true}),
    )
}
pub fn apply(
    pool: &mut DbPool,
    cfg: &DbConnectionConfig,
    state: &mut TableState,
    request: &TableRequest,
    confirmed: bool,
    cancel: &QueryCancel,
) -> Result<Value, String> {
    if !confirmed {
        return Err("DB_TABLE_CONFIRMATION_REQUIRED".into());
    }
    let ticket = state
        .previews
        .remove(
            request
                .ticket
                .as_deref()
                .ok_or("DB_TABLE_TICKET_REQUIRED")?,
        )
        .ok_or("DB_TABLE_PREVIEW_EXPIRED")?;
    if ticket.expires <= Instant::now() {
        return Err("DB_TABLE_PREVIEW_EXPIRED".into());
    }
    let current = catalog(
        pool,
        &TableRequest {
            name: ticket.catalog.name.clone(),
            schema: ticket.catalog.schema.clone(),
            ..Default::default()
        },
        cancel,
    )?;
    if digest(&current)? != ticket.digest {
        return Err("DB_TABLE_SCHEMA_CHANGED".into());
    }
    if !writable(&current, cfg, pool) {
        return Err("DB_TABLE_WRITE_DENIED".into());
    }
    let mysql = matches!(pool, DbPool::MySql(..));
    let table = table_name(&current, mysql)?;
    execute(
        pool,
        if mysql {
            "START TRANSACTION"
        } else {
            "BEGIN IMMEDIATE"
        },
        &[],
    )?;
    let operation = (|| -> Result<usize, String> {
        for statement in &ticket.statements {
            stop(cancel)?;
            // Lock and compare the entire original row before applying any edit.
            // All prior edits in this batch are rolled back on a later conflict.
            if let Some(original) = &statement.original {
                let (predicate, keys) = row_predicate(&current, original, mysql)?;
                let sql = format!(
                    "SELECT {} FROM {table} WHERE {predicate} LIMIT 2{}",
                    readable_columns(&current, mysql)?,
                    if mysql { " FOR UPDATE" } else { "" }
                );
                let rows = query(pool, &sql, &keys, 2, cancel)?;
                if !rows.text_only || rows.rows.len() != 1 || rows.rows[0] != *original {
                    return Err("DB_TABLE_ROW_CHANGED_BATCH_ROLLED_BACK".into());
                }
            }
            sp::evaluate_db_query_gate(&statement.sql, cfg, pool.encrypted(), true)
                .map_err(|c| c.as_str().to_string())?;
            let changed = execute(pool, &statement.sql, &statement.parameters)?;
            if changed > 1
                || statement.action == "delete" && changed != 1
                || statement.action == "insert" && changed != 1
            {
                return Err("DB_TABLE_AFFECTED_ROW_MISMATCH".into());
            }
            stop(cancel)?;
        }
        Ok(ticket.statements.len())
    })();
    match operation {
        Ok(count) => {
            state.originals.clear();
            state.previews.clear();
            Ok(
                json!({"applied":count,"transaction":"read_write","requires_commit":true,"committed":false}),
            )
        }
        Err(error) => {
            if execute(pool, "ROLLBACK", &[]).is_err() {
                return Err("DB_TABLE_ROLLBACK_OUTCOME_UNKNOWN_CLOSE_SESSION".into());
            }
            Err(error)
        }
    }
}

pub fn cancel_preview(state: &mut TableState, ticket: Option<&str>) -> Value {
    let removed = ticket.is_some_and(|id| state.previews.remove(id).is_some());
    json!({"cancelled":removed})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> DbConnectionConfig {
        serde_json::from_value(json!({"id":"table-fixture","name":"test","kind":"sqlite","host":null,"port":null,"database":"/tmp/table-fixture.db","username":null,"ssl_mode":"disable","allow_write":true,"production_hint":false,"enabled":true,"created_at":"2026-10-16T00:00:00Z","updated_at":"2026-10-16T00:00:00Z"})).unwrap()
    }
    fn fixture() -> DbPool {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE item(id INTEGER PRIMARY KEY, value TEXT, n INTEGER)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO item VALUES(1,'a',1),(2,'b',2)", [])
            .unwrap();
        DbPool::Sqlite(conn)
    }
    fn req() -> TableRequest {
        TableRequest {
            name: "item".into(),
            ..Default::default()
        }
    }
    #[test]
    fn studio_table_filters_values_are_parameters() {
        let mut pool = fixture();
        let catalog = catalog(&mut pool, &req(), &QueryCancel::new()).unwrap();
        let (sql, p) = where_filters(
            &catalog,
            &[TableFilter {
                column: "value".into(),
                operator: "eq".into(),
                value: Some("x' OR 1=1 --".into()),
            }],
            false,
        )
        .unwrap();
        assert_eq!(sql, " WHERE \"value\" = ?");
        assert_eq!(p[0].as_deref(), Some("x' OR 1=1 --"));
    }
    #[test]
    fn studio_table_sort_has_primary_key_tie_breaker() {
        let mut pool = fixture();
        let catalog = catalog(&mut pool, &req(), &QueryCancel::new()).unwrap();
        let (sql, stable) = ordering(
            &catalog,
            &[TableOrder {
                column: "value".into(),
                descending: true,
            }],
            false,
        )
        .unwrap();
        assert!(stable);
        assert_eq!(sql, " ORDER BY \"value\" DESC,\"id\" ASC");
    }
    #[test]
    fn studio_table_edit_uses_exact_preview_and_separate_commit() {
        let mut pool = fixture();
        let mut state = TableState::default();
        let cfg = config();
        let cancel = QueryCancel::new();
        let page = read(&mut pool, &cfg, &mut state, &req(), &cancel).unwrap();
        let id = page["rows"][0]["row_id"].as_str().unwrap();
        let mut edit = req();
        edit.changes.push(TableChange {
            action: "update".into(),
            row_id: Some(id.into()),
            values: BTreeMap::from([("value".into(), Some("O'Reilly; DROP TABLE item".into()))]),
        });
        let p = preview(&mut pool, &cfg, &mut state, &edit, &cancel).unwrap();
        let mut apply_request = req();
        apply_request.ticket = Some(p["ticket"].as_str().unwrap().into());
        assert!(apply(&mut pool, &cfg, &mut state, &apply_request, false, &cancel).is_err());
        let result = apply(&mut pool, &cfg, &mut state, &apply_request, true, &cancel).unwrap();
        assert_eq!(result["committed"], false);
        let rows = query(
            &mut pool,
            "SELECT value FROM item WHERE id=1",
            &[],
            1,
            &cancel,
        )
        .unwrap();
        assert_eq!(
            rows.rows[0][0].as_deref(),
            Some("O'Reilly; DROP TABLE item")
        );
        execute(&mut pool, "ROLLBACK", &[]).unwrap();
        let rows = query(
            &mut pool,
            "SELECT value FROM item WHERE id=1",
            &[],
            1,
            &cancel,
        )
        .unwrap();
        assert_eq!(rows.rows[0][0].as_deref(), Some("a"));
        assert!(apply(&mut pool, &cfg, &mut state, &apply_request, true, &cancel).is_err());
    }
    #[test]
    fn studio_table_late_conflict_rolls_back_earlier_edit() {
        let mut pool = fixture();
        let mut state = TableState::default();
        let cfg = config();
        let cancel = QueryCancel::new();
        let page = read(&mut pool, &cfg, &mut state, &req(), &cancel).unwrap();
        let mut edit = req();
        for row in page["rows"].as_array().unwrap() {
            edit.changes.push(TableChange {
                action: "update".into(),
                row_id: Some(row["row_id"].as_str().unwrap().into()),
                values: BTreeMap::from([("value".into(), Some("edited".into()))]),
            });
        }
        let p = preview(&mut pool, &cfg, &mut state, &edit, &cancel).unwrap();
        execute(
            &mut pool,
            "UPDATE item SET value='external' WHERE id=2",
            &[],
        )
        .unwrap();
        let mut apply_request = req();
        apply_request.ticket = Some(p["ticket"].as_str().unwrap().into());
        assert_eq!(
            apply(&mut pool, &cfg, &mut state, &apply_request, true, &cancel).unwrap_err(),
            "DB_TABLE_ROW_CHANGED_BATCH_ROLLED_BACK"
        );
        let rows = query(
            &mut pool,
            "SELECT value FROM item ORDER BY id",
            &[],
            2,
            &cancel,
        )
        .unwrap();
        assert_eq!(
            rows.rows,
            vec![vec![Some("a".into())], vec![Some("external".into())]]
        );
    }
    #[test]
    fn studio_table_transaction_control_preserves_bound_values_until_rollback() {
        let mut pool = fixture();
        let literal = "quoted ' value; DROP TABLE item; -- literal";
        execute(&mut pool, "BEGIN IMMEDIATE", &[]).unwrap();
        match &pool {
            DbPool::Sqlite(conn) => assert!(!conn.is_autocommit()),
            _ => unreachable!(),
        }
        execute(
            &mut pool,
            "UPDATE item SET value=? WHERE id=?",
            &[Some(literal.into()), Some("1".into())],
        )
        .unwrap();
        let inside = query(
            &mut pool,
            "SELECT value FROM item WHERE id=1",
            &[],
            1,
            &QueryCancel::new(),
        )
        .unwrap();
        assert_eq!(inside.rows[0][0].as_deref(), Some(literal));
        execute(&mut pool, "ROLLBACK", &[]).unwrap();
        match &pool {
            DbPool::Sqlite(conn) => assert!(conn.is_autocommit()),
            _ => unreachable!(),
        }
        let after = query(
            &mut pool,
            "SELECT value FROM item WHERE id=1",
            &[],
            1,
            &QueryCancel::new(),
        )
        .unwrap();
        assert_eq!(after.rows[0][0].as_deref(), Some("a"));
    }
    #[test]
    fn studio_table_constraint_error_rolls_back_earlier_edit() {
        let mut pool = fixture();
        let mut state = TableState::default();
        let cfg = config();
        let cancel = QueryCancel::new();
        let page = read(&mut pool, &cfg, &mut state, &req(), &cancel).unwrap();
        let mut request = req();
        request.changes = vec![
            TableChange {
                action: "update".into(),
                row_id: Some(page["rows"][0]["row_id"].as_str().unwrap().into()),
                values: BTreeMap::from([("value".into(), Some("must roll back".into()))]),
            },
            TableChange {
                action: "insert".into(),
                row_id: None,
                values: BTreeMap::from([
                    ("id".into(), Some("2".into())),
                    ("value".into(), Some("duplicate key".into())),
                ]),
            },
        ];
        let preview = preview(&mut pool, &cfg, &mut state, &request, &cancel).unwrap();
        request.ticket = Some(preview["ticket"].as_str().unwrap().into());
        assert_eq!(
            apply(&mut pool, &cfg, &mut state, &request, true, &cancel).unwrap_err(),
            "DB_TABLE_MUTATION_FAILED"
        );
        let after = query(
            &mut pool,
            "SELECT value FROM item ORDER BY id",
            &[],
            2,
            &cancel,
        )
        .unwrap();
        assert_eq!(
            after.rows,
            vec![vec![Some("a".into())], vec![Some("b".into())]]
        );
        match &pool {
            DbPool::Sqlite(conn) => assert!(conn.is_autocommit()),
            _ => unreachable!(),
        }
        assert_eq!(
            apply(&mut pool, &cfg, &mut state, &request, true, &cancel).unwrap_err(),
            "DB_TABLE_PREVIEW_EXPIRED"
        );
    }
    #[test]
    fn studio_table_production_is_read_only() {
        let mut pool = fixture();
        let mut state = TableState::default();
        let mut cfg = config();
        cfg.production_hint = Some(true);
        let page = read(&mut pool, &cfg, &mut state, &req(), &QueryCancel::new()).unwrap();
        assert_eq!(page["editable"], false);
    }
    #[test]
    fn studio_table_reload_invalidates_original_row_tokens() {
        let mut pool = fixture();
        let mut state = TableState::default();
        let cfg = config();
        let cancel = QueryCancel::new();
        let old = read(&mut pool, &cfg, &mut state, &req(), &cancel).unwrap();
        read(&mut pool, &cfg, &mut state, &req(), &cancel).unwrap();
        let mut edit = req();
        edit.changes.push(TableChange {
            action: "delete".into(),
            row_id: Some(old["rows"][0]["row_id"].as_str().unwrap().into()),
            values: BTreeMap::new(),
        });
        assert!(preview(&mut pool, &cfg, &mut state, &edit, &cancel).is_err());
    }
}
