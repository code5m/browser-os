//! Database 能力原生后端命令（M4-3 / M4-2.s，Lane A4）。
//!
//! 命令体只做：来源校验 + 安全闸门 + 委托 `crate::database` 内核；不反向依赖
//! `crate::bridge`。从 `bridge.rs` 迁入本 capability-owned 命令模块
//! （Native Physical Boundary 分解）。
//!
//! 安全闸门接线（A3 的 `DbPool::query` 不判写，写闸门全链路归本层）：
//! `db_query` 在**任何语句实际执行前**必须过 `security_policy::evaluate_db_query_gate`，
//! 任一步拒绝即短路返回，绝不降级放行（F1 / F4 / 契约 G-4 / G-5）。
//!
//! 连接模型：因 `DbPool` 包裹的驱动句柄非 `Send`，不能驻留 Tauri 全局 managed state，
//! 故采用「按需重连」模型——`db_connect` 仅校验可达性/凭据并登记配置进
//! `DbConnectionRegistry`、凭据写入系统密钥库（键 = `db:<conn_id>`）；`db_query` 每次
//! 从登记簿取配置 + 从密钥库取凭据即时建连执行；`db_disconnect` 撤销登记并删除密钥。
//!
//! 凭据：password 永不进 `DbConnectionConfig`（F2），不进审计 detail（G-1 / G-3）。

use super::profiles;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::database::{credential_key, DbPool, DbQueryResult, QueryCancel};
use crate::domain::{DbConnectionConfig, SupportedDb};
use crate::keyring_store::KeyringStore;
use crate::security_policy as sp;
use crate::shared::invocation::check_invocation_source;
use crate::workspace::{allowed_roots, log_audit};

/// 数据库连接配置登记簿（不含非 Send 的池句柄）。注册为 Tauri managed state。
pub struct DbConnectionRegistry {
    pub configs: Mutex<HashMap<String, DbConnectionConfig>>,
    pub queries: Mutex<HashMap<String, (String, QueryCancel)>>,
    credential_refs: Mutex<HashMap<String, String>>,
    loaded: Mutex<bool>,
}

impl Default for DbConnectionRegistry {
    fn default() -> Self {
        DbConnectionRegistry {
            configs: Mutex::new(HashMap::new()),
            queries: Mutex::new(HashMap::new()),
            credential_refs: Mutex::new(HashMap::new()),
            loaded: Mutex::new(false),
        }
    }
}

/// Load once per process while holding the initialized lock.
/// Metadata never includes passwords or JDBC URLs; Keyring remains the sole
/// credential source, including after BrowserOS is restarted.
pub(crate) fn ensure_profiles_loaded(app: &AppHandle) -> Result<(), String> {
    let reg = app.state::<DbConnectionRegistry>();
    let mut loaded = reg.loaded.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    if *loaded {
        return Ok(());
    }
    let file = profiles::path_for(app)?;
    let records = profiles::read_records(&file)?;
    let mut configs = reg.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let mut refs = reg.credential_refs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    for record in records {
        refs.insert(record.config.id.clone(), record.reference()?);
        configs.insert(record.config.id.clone(), record.config);
    }
    *loaded = true;
    Ok(())
}
fn persist_snapshot(
    app: &AppHandle,
    map: &HashMap<String, DbConnectionConfig>,
    refs: &HashMap<String, String>,
) -> Result<(), String> {
    let mut records = map.values().map(|cfg| profiles::StoredProfile {
        config: cfg.clone(), credential_ref: refs.get(&cfg.id).cloned().unwrap_or_default(),
    }).collect::<Vec<_>>();
    records.sort_by(|a, b| a.config.id.cmp(&b.config.id));
    profiles::write_records(&profiles::path_for(app)?, &records)
}
fn reconcile_credentials(app: &AppHandle) -> Result<(), String> {
    let path = profiles::path_for(app)?;
    let live = profiles::read_records(&path)?.iter().filter(|record| record.config.kind != SupportedDb::Sqlite).map(|record| record.reference())
        .collect::<Result<HashSet<_>, _>>()?;
    crate::shared::credential_versions::reconcile(&path, "db", &live)
}
fn destination(cfg: &DbConnectionConfig) -> serde_json::Value {
    serde_json::json!([cfg.kind, cfg.host, cfg.port, cfg.database, cfg.username, cfg.ssl_mode])
}

// A session retains secret bytes only in native memory. The revision includes
// the immutable credential reference, so rotation invalidates existing sessions.
pub(super) struct ConnectionSnapshot {
    pub config: DbConnectionConfig,
    pub password: Option<String>,
    pub revision: String,
}
fn profile_revision(cfg: &DbConnectionConfig, reference: &str) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(&(cfg, reference)).map_err(|_| "DB_PROFILE_ENCODE_FAILED")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn read_connection_snapshot(app: &AppHandle, id: &str, include_secret: bool) -> Result<ConnectionSnapshot, String> {
    ensure_profiles_loaded(app)?;
    let registry = app.state::<DbConnectionRegistry>();
    let configs = registry.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let config = configs.get(id).filter(|cfg| cfg.enabled).cloned()
        .ok_or("DB_SESSION_SOURCE_DISABLED_OR_REMOVED")?;
    let refs = registry.credential_refs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let reference = if config.kind == SupportedDb::Sqlite { String::new() }
        else { refs.get(id).cloned().unwrap_or_else(|| credential_key(id)) };
    let password = if include_secret && config.kind != SupportedDb::Sqlite {
        Some(KeyringStore::get_token(&reference).map_err(|_| "DB_CREDENTIAL_UNAVAILABLE")?)
    } else { None };
    let revision = profile_revision(&config, &reference)?;
    Ok(ConnectionSnapshot { config, password, revision })
}
pub(super) fn connection_snapshot(app: &AppHandle, id: &str) -> Result<ConnectionSnapshot, String> {
    read_connection_snapshot(app, id, true)
}
pub(super) fn connection_revision(app: &AppHandle, id: &str) -> Result<String, String> {
    Ok(read_connection_snapshot(app, id, false)?.revision)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectResult {
    pub conn_id: String,
    pub kind: SupportedDb,
    pub credential_cleanup_pending: bool,
}

#[tauri::command]
pub fn db_list_connections(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<DbConnectionConfig>, String> {
    check_invocation_source(&webview, "db_list_connections", None, &app)?;
    ensure_profiles_loaded(&app)?;
    let reg = app.state::<DbConnectionRegistry>();
    let guard = reg.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let _ = reconcile_credentials(&app);
    let mut configs: Vec<_> = guard.values().cloned().collect();
    configs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(configs)
}

/// Import safe connection metadata from user-approved IDEA/DataGrip XML.
/// No authentication or SQL runs in this command. Entries remain disabled until
/// a user supplies credentials and verifies them through db_connect.
#[derive(Debug, Clone, Deserialize)]
pub struct DbImportCandidate {
    name: String,
    kind: SupportedDb,
    host: Option<String>,
    port: Option<u16>,
    database: String,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DbImportResult {
    imported: usize,
    duplicates: usize,
}

#[tauri::command]
pub fn db_import_profiles(
    app: AppHandle,
    webview: tauri::Webview,
    candidates: Vec<DbImportCandidate>,
) -> Result<DbImportResult, String> {
    check_invocation_source(&webview, "db_import_profiles", None, &app)?;
    ensure_profiles_loaded(&app)?;
    if candidates.is_empty() || candidates.len() > 32 {
        return Err("DB_IMPORT_BATCH_LIMIT".into());
    }
    let roots = allowed_roots(&app);
    let reg = app.state::<DbConnectionRegistry>();
    let mut guard = reg.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let refs = reg.credential_refs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let mut updated = guard.clone();
    let mut imported = 0;
    let mut duplicates = 0;
    for item in candidates {
        let name = item.name.trim();
        let database = item.database.trim();
        if name.is_empty()
            || name.len() > 128
            || database.is_empty()
            || database.len() > 4096
            || name.chars().any(char::is_control)
            || database.chars().any(char::is_control)
        {
            return Err("DB_IMPORT_INVALID_METADATA".into());
        }
        if item.kind.is_remote() {
            let host = item.host.as_deref().unwrap_or("").trim();
            if host.is_empty()
                || host.len() > 253
                || host
                    .chars()
                    .any(|ch| ch.is_control() || ch.is_whitespace() || ch == '/' || ch == '@')
                || database.len() > 128
            {
                return Err("DB_IMPORT_INVALID_MYSQL".into());
            }
        } else {
            sp::check_path_within_roots(database, &roots)
                .map_err(|_| "DB_IMPORT_SQLITE_NOT_AUTHORIZED".to_string())?;
        }
        if item.username.as_deref().unwrap_or("").len() > 128 {
            return Err("DB_IMPORT_USERNAME_TOO_LONG".into());
        }
        let host = if item.kind == SupportedDb::Sqlite {
            None
        } else {
            item.host
        };
        let port = if item.kind == SupportedDb::Sqlite {
            None
        } else {
            item.port.or(Some(3306))
        };
        let username = if item.kind == SupportedDb::Sqlite {
            None
        } else {
            item.username
        };
        if updated.values().any(|existing| {
            existing.kind == item.kind
                && existing.host == host
                && existing.port == port
                && existing.database == database
                && existing.username == username
        }) {
            duplicates += 1;
            continue;
        }
        if updated.len() >= 64 {
            return Err("DB_CONNECTION_LIMIT".into());
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        updated.insert(
            id.clone(),
            DbConnectionConfig {
                id,
                name: name.to_string(),
                kind: item.kind,
                host,
                port,
                database: database.to_string(),
                username,
                ssl_mode: crate::domain::DbSslMode::Prefer,
                allow_write: false,
                production_hint: None,
                enabled: false,
                created_at: now,
                updated_at: now,
            },
        );
        imported += 1;
    }
    persist_snapshot(&app, &updated, &refs)?;
    *guard = updated;
    log_audit(
        &app,
        "db.import_profiles",
        format!("imported={} duplicates={}", imported, duplicates),
    );
    Ok(DbImportResult {
        imported,
        duplicates,
    })
}

#[tauri::command]
pub fn db_cancel(
    app: AppHandle,
    webview: tauri::Webview,
    query_id: String,
) -> Result<bool, String> {
    check_invocation_source(&webview, "db_cancel", None, &app)?;
    let reg = app.state::<DbConnectionRegistry>();
    let queries = reg.queries.lock().unwrap();
    if let Some((_, cancel)) = queries.get(&query_id) {
        cancel.cancel();
        return Ok(true);
    }
    Ok(false)
}

// Slow remote handshake and OS Secret Service access run off the GTK event
// loop. Source authorization stays synchronous before worker dispatch.
#[tauri::command]
pub async fn db_connect(
    app: AppHandle,
    webview: tauri::Webview,
    cfg: DbConnectionConfig,
    password: Option<String>,
) -> Result<DbConnectResult, String> {
    check_invocation_source(&webview, "db_connect", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || db_connect_blocking(app, cfg, password))
        .await
        .map_err(|_| "DB_CONNECT_WORKER_FAILED".to_string())?
}

fn db_connect_blocking(
    app: AppHandle,
    cfg: DbConnectionConfig,
    password: Option<String>,
) -> Result<DbConnectResult, String> {
    ensure_profiles_loaded(&app)?;
    let conn_id = cfg.id.clone();
    Uuid::parse_str(&conn_id).map_err(|_| "DB_INVALID_CONNECTION_ID")?;
    let supplied = password.filter(|value| !value.is_empty());
    let reg = app.state::<DbConnectionRegistry>();
    let (before, previous_ref, secret) = {
        let configs = reg.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
        if configs.len() >= 64 && !configs.contains_key(&conn_id) { return Err("DB_CONNECTION_LIMIT".into()); }
        if super::studio::profile_in_use(&app, &conn_id)? { return Err("DB_SESSION_BUSY".into()); }
        if reg.queries.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?
            .values().any(|(id, _)| id == &conn_id) { return Err("DB_QUERY_BUSY".into()); }
        let old = configs.get(&conn_id).cloned();
        let refs = reg.credential_refs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
        let previous_ref = old.as_ref().map(|_| refs.get(&conn_id).cloned()
            .unwrap_or_else(|| credential_key(&conn_id)));
        let secret = if cfg.kind == SupportedDb::Sqlite { None }
            else if let Some(secret) = &supplied { Some(secret.clone()) }
            else {
                let old = old.as_ref().ok_or("DB_CREDENTIAL_REQUIRED")?;
                if destination(old) != destination(&cfg) { return Err("DB_ENDPOINT_CHANGE_REQUIRES_CREDENTIAL".into()); }
                Some(KeyringStore::get_token(previous_ref.as_deref().ok_or("DB_CREDENTIAL_REQUIRED")?)
                    .map_err(|_| "DB_CREDENTIAL_UNAVAILABLE")?)
            };
        (old.map(|cfg| serde_json::to_value(cfg)).transpose().map_err(|_| "DB_PROFILE_ENCODE_FAILED")?,
            previous_ref, secret)
    };
    let roots = allowed_roots(&app);
    let kind = cfg.kind;
    let _probe = DbPool::connect(&cfg, secret.as_deref(), &roots)
        .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;
    // Publish only if the configuration reviewed before the probe is still current.
    let mut configs = reg.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let current = configs.get(&conn_id).map(serde_json::to_value).transpose()
        .map_err(|_| "DB_PROFILE_ENCODE_FAILED")?;
    if current != before { return Err("DB_PROFILE_CONFLICT".into()); }
    if super::studio::profile_in_use(&app, &conn_id)? { return Err("DB_SESSION_BUSY".into()); }
    if reg.queries.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?
        .values().any(|(id, _)| id == &conn_id) { return Err("DB_QUERY_BUSY".into()); }
    let mut refs = reg.credential_refs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    let current_ref = configs.get(&conn_id).map(|_| refs.get(&conn_id).cloned()
        .unwrap_or_else(|| credential_key(&conn_id)));
    if current_ref != previous_ref { return Err("DB_PROFILE_CONFLICT".into()); }
    if configs.len() >= 64 && !configs.contains_key(&conn_id) { return Err("DB_CONNECTION_LIMIT".into()); }
    reconcile_credentials(&app)?;
    let path = profiles::path_for(&app)?;
    let retired = previous_ref.clone().into_iter().collect::<Vec<_>>();
    let reference = if cfg.kind == SupportedDb::Sqlite {
        crate::shared::credential_versions::record_retirement(&path, "db", &retired)?;
        String::new()
    } else if let Some(secret) = supplied.as_deref() {
        crate::shared::credential_versions::stage(&path, "db", &conn_id, secret, &retired)?
    } else { previous_ref.ok_or("DB_CREDENTIAL_REQUIRED")? };
    let mut updated = configs.clone();
    let mut updated_refs = refs.clone();
    updated.insert(conn_id.clone(), cfg);
    if reference.is_empty() { updated_refs.remove(&conn_id); }
    else { updated_refs.insert(conn_id.clone(), reference); }
    if let Err(error) = persist_snapshot(&app, &updated, &updated_refs) {
        // Reload published state if rename succeeded but directory sync failed.
        if let Ok(records) = profiles::read_records(&path) {
            configs.clear(); refs.clear();
            for record in records {
                refs.insert(record.config.id.clone(), record.reference()?);
                configs.insert(record.config.id.clone(), record.config);
            }
            let _ = reconcile_credentials(&app);
        }
        return Err(error);
    }
    *configs = updated;
    *refs = updated_refs;
    let credential_cleanup_pending = reconcile_credentials(&app).is_err();
    drop(refs); drop(configs);
    log_audit(&app, "db.connect", format!("conn_id={} kind={:?}", conn_id, kind));
    Ok(DbConnectResult { conn_id, kind, credential_cleanup_pending })
}

// The actual worker owns its admission, including when the invoking future is
// dropped. A queued or still-running query cannot release its budget early.
struct QueryRegistration { app: AppHandle, id: String }
impl Drop for QueryRegistration {
    fn drop(&mut self) {
        let registry = self.app.state::<DbConnectionRegistry>();
        if let Ok(mut queries) = registry.queries.lock() { queries.remove(&self.id); };
    }
}
#[tauri::command]
pub async fn db_query(
    app: AppHandle,
    webview: tauri::Webview,
    conn_id: String,
    sql: String,
    timeout_secs: Option<u64>,
    confirm_write: bool,
    query_id: Option<String>,
) -> Result<DbQueryResult, String> {
    check_invocation_source(&webview, "db_query", None, &app)?;
    let query_id = query_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    Uuid::parse_str(&query_id).map_err(|_| "DB_INVALID_QUERY_ID")?;
    let cancel = QueryCancel::new();
    // Admit before dispatch: queued requests are cancellable and count toward
    // the same four-worker budget. No disk or Keyring work runs on this task.
    {
        let registry = app.state::<DbConnectionRegistry>();
        let mut queries = registry.queries.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
        if queries.len() >= 4 || queries.contains_key(&query_id) { return Err("DB_QUERY_BUSY".into()); }
        queries.insert(query_id.clone(), (conn_id.clone(), cancel.clone()));
    }
    let registration = QueryRegistration { app: app.clone(), id: query_id.clone() };
    tauri::async_runtime::spawn_blocking(move || {
        let _registration = registration;
        if cancel.is_cancelled() { return Err("DB_CANCELLED".into()); }
        ensure_profiles_loaded(&app)?;
        // Snapshot configuration AND credential bytes under one Owner lock.
        // Neither endpoint nor credential is re-read after the snapshot.
        let (cfg, password) = {
            let registry = app.state::<DbConnectionRegistry>();
            let configs = registry.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
            let cfg = configs.get(&conn_id).cloned().ok_or("DB_NOT_CONNECTED")?;
            let refs = registry.credential_refs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
            let password = if cfg.kind == SupportedDb::Sqlite { None } else {
                let reference = refs.get(&conn_id).cloned().unwrap_or_else(|| credential_key(&conn_id));
                Some(KeyringStore::get_token(&reference).map_err(|_| "DB_CREDENTIAL_UNAVAILABLE")?)
            };
            (cfg, password)
        };
        if cancel.is_cancelled() { return Err("DB_CANCELLED".into()); }
        let roots = allowed_roots(&app);
        let mut pool = DbPool::connect(&cfg, password.as_deref(), &roots)
            .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;
        drop(password);

        let encrypted = pool.encrypted();
        sp::evaluate_db_query_gate(&sql, &cfg, encrypted, confirm_write)
            .map_err(|code| code.as_str().to_string())?;
        let result = pool
            .query(&sql, &cancel, timeout_secs, &query_id)
            .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;
        log_audit(
            &app,
            "db.query",
            format!("conn_id={} rows={} truncated={} field_truncated={}",
                conn_id, result.row_count, result.truncated, result.field_truncated),
        );
        Ok(result)
    })
    .await
    .map_err(|_| "DB_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub fn db_disconnect(
    app: AppHandle,
    webview: tauri::Webview,
    conn_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "db_disconnect", None, &app)?;
    ensure_profiles_loaded(&app)?;
    let reg = app.state::<DbConnectionRegistry>();
    let mut configs = reg.configs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    if super::studio::profile_in_use(&app, &conn_id)? { return Err("DB_SESSION_BUSY".into()); }
    if reg.queries.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?
        .values().any(|(id, _)| id == &conn_id) { return Err("DB_QUERY_BUSY".into()); }
    if !configs.contains_key(&conn_id) { return Err("DB_NOT_CONNECTED".into()); }
    let mut refs = reg.credential_refs.lock().map_err(|_| "DB_REGISTRY_LOCK_FAILED")?;
    reconcile_credentials(&app)?;
    let reference = refs.get(&conn_id).cloned().unwrap_or_else(|| credential_key(&conn_id));
    let path = profiles::path_for(&app)?;
    crate::shared::credential_versions::record_retirement(&path, "db", &[reference])?;
    let mut updated = configs.clone();
    let mut updated_refs = refs.clone();
    updated.remove(&conn_id);
    updated_refs.remove(&conn_id);
    if let Err(error) = persist_snapshot(&app, &updated, &updated_refs) {
        if let Ok(records) = profiles::read_records(&path) {
            configs.clear(); refs.clear();
            for record in records {
                refs.insert(record.config.id.clone(), record.reference()?);
                configs.insert(record.config.id.clone(), record.config);
            }
            let _ = reconcile_credentials(&app);
        }
        return Err(error);
    }
    *configs = updated; *refs = updated_refs;
    let cleanup = reconcile_credentials(&app);
    drop(refs); drop(configs);
    log_audit(&app, "db.disconnect", format!("conn_id={} removed=true", conn_id));
    cleanup.map_err(|_| "DB_DISCONNECTED_CREDENTIAL_CLEANUP_PENDING".to_string())
}

#[cfg(test)]
mod credential_revision_tests {
    use super::*;
    #[test]
    fn session_revision_binds_credential_rotation_and_destination() {
        let mut cfg: DbConnectionConfig = serde_json::from_value(serde_json::json!({
            "id":Uuid::new_v4().to_string(),"name":"fixture","kind":"mysql","host":"one.invalid",
            "port":3306,"database":"fixture","username":"reader","ssl_mode":"require",
            "allow_write":false,"production_hint":true,"enabled":true,
            "created_at":"2026-11-22T00:00:00Z","updated_at":"2026-11-22T00:00:00Z"
        })).unwrap();
        let first = profile_revision(&cfg, "db:fixture:version-one").unwrap();
        assert_ne!(first, profile_revision(&cfg, "db:fixture:version-two").unwrap());
        let original_destination = destination(&cfg);
        cfg.host = Some("two.invalid".into());
        assert_ne!(original_destination, destination(&cfg));
        assert_ne!(first, profile_revision(&cfg, "db:fixture:version-one").unwrap());
    }
}
