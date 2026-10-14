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

use std::collections::HashMap;
use std::path::Path;
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
}

impl Default for DbConnectionRegistry {
    fn default() -> Self {
        DbConnectionRegistry {
            configs: Mutex::new(HashMap::new()),
            queries: Mutex::new(HashMap::new()),
        }
    }
}


const MAX_DB_SAVED_CONNECTIONS: usize = 64;

fn db_registry_path(app: &AppHandle) -> std::path::PathBuf {
    crate::workspace::data_dir(app).join("db_connections.json")
}

/// Persistent metadata only. Passwords are absent from DbConnectionConfig
/// by construction and remain exclusively in the OS keyring.
fn persist_configs_at(path: &Path, configs: &[DbConnectionConfig]) -> Result<(), String> {
    if configs.len() > MAX_DB_SAVED_CONNECTIONS { return Err("DB_CONNECTION_LIMIT".into()); }
    let parent = path.parent().ok_or("DB_CONFIG_PATH_INVALID")?;
    std::fs::create_dir_all(parent).map_err(|_| "DB_CONFIG_DIR_WRITE_FAILED")?;
    crate::workspace::save_json_list_at(path, configs, "数据库连接配置")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "DB_CONFIG_PERMISSION_FAILED")?;
    }
    Ok(())
}

fn persist_registry(app: &AppHandle, configs: &HashMap<String, DbConnectionConfig>) -> Result<(), String> {
    let mut values: Vec<_> = configs.values().cloned().collect();
    values.sort_by(|a,b| a.id.cmp(&b.id));
    persist_configs_at(&db_registry_path(app), &values)
}

fn load_registry_if_empty(app: &AppHandle) {
    let reg = app.state::<DbConnectionRegistry>();
    let mut current = reg.configs.lock().unwrap();
    if !current.is_empty() { return; }
    // The workspace reader preserves corrupted originals with a .corrupt
    // suffix instead of overwriting them with an empty registry.
    let saved: Vec<DbConnectionConfig> = crate::workspace::load_json_list_at(
        &db_registry_path(app), "数据库连接配置");
    for cfg in saved.into_iter().take(MAX_DB_SAVED_CONNECTIONS) {
        if Uuid::parse_str(&cfg.id).is_ok() && !cfg.id.is_empty() {
            current.insert(cfg.id.clone(), cfg);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectResult {
    pub conn_id: String,
    pub kind: SupportedDb,
}

#[tauri::command]
pub fn db_list_connections(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<DbConnectionConfig>, String> {
    check_invocation_source(&webview, "db_list_connections", None, &app)?;
    load_registry_if_empty(&app);
    let mut configs: Vec<_> = app
        .state::<DbConnectionRegistry>()
        .configs
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();
    configs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(configs)
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

#[tauri::command]
pub fn db_connect(
    app: AppHandle,
    webview: tauri::Webview,
    cfg: DbConnectionConfig,
    password: Option<String>,
) -> Result<DbConnectResult, String> {
    check_invocation_source(&webview, "db_connect", None, &app)?;
    load_registry_if_empty(&app);
    {
        let reg = app.state::<DbConnectionRegistry>();
        let configs = reg.configs.lock().unwrap();
        if configs.len() >= MAX_DB_SAVED_CONNECTIONS && !configs.contains_key(&cfg.id) {
            return Err("DB_CONNECTION_LIMIT".into());
        }
        if reg
            .queries
            .lock()
            .unwrap()
            .values()
            .any(|(id, _)| id == &cfg.id)
        {
            return Err("DB_QUERY_BUSY".into());
        }
    }
    // 实际打通一次以校验配置/凭据/可达性；连接不驻留全局（非 Send），校验后即弃。
    let roots = allowed_roots(&app);
    let kind = cfg.kind;
    let conn_id = cfg.id.clone();
    // When reconnecting an imported connection, reuse only its own existing
    // keyring entry; never read the JetBrains Password Safe.
    let known_password = if cfg.kind != SupportedDb::Sqlite
        && password.as_deref().unwrap_or_default().is_empty() {
        KeyringStore::get_token(&credential_key(&conn_id)).ok()
    } else { None };
    let probe_password = password.as_deref().filter(|p| !p.is_empty())
        .or(known_password.as_deref());
    let _probe = DbPool::connect(&cfg, probe_password, &roots)
        .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;
    // G-1：凭据（若有）写入系统密钥库，键必须带 `db:` 前缀，与 git 的 repo_id 命名空间隔离。
    if let Some(p) = password.filter(|p| !p.is_empty()) {
        KeyringStore::save_token(&credential_key(&conn_id), &p)?;
    }
    {
        let reg = app.state::<DbConnectionRegistry>();
        let mut guard = reg.configs.lock().unwrap();
        let prior = guard.insert(conn_id.clone(), cfg);
        if let Err(e) = persist_registry(&app, &guard) {
            if let Some(original) = prior {
                guard.insert(conn_id.clone(), original);
            } else {
                guard.remove(&conn_id);
            }
            return Err(e);
        }
    }
    // 审计 detail 禁含 SQL/凭据（G-3）：只记连接标识与类型。
    log_audit(
        &app,
        "db.connect",
        format!("conn_id={} kind={:?}", conn_id, kind),
    );
    Ok(DbConnectResult { conn_id, kind })
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
    load_registry_if_empty(&app);

    // 取登记配置（db_connect 未登记即视为未连接）。
    let cfg = {
        let reg = app.state::<DbConnectionRegistry>();
        let guard = reg.configs.lock().unwrap();
        guard
            .get(&conn_id)
            .cloned()
            .ok_or_else(|| "DB_NOT_CONNECTED".to_string())?
    };

    let query_id = query_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    Uuid::parse_str(&query_id).map_err(|_| "DB_INVALID_QUERY_ID")?;
    let cancel = QueryCancel::new();
    {
        let registry = app.state::<DbConnectionRegistry>();
        let mut queries = registry.queries.lock().unwrap();
        if queries.len() >= 4 || queries.contains_key(&query_id) {
            return Err("DB_QUERY_BUSY".into());
        }
        queries.insert(query_id.clone(), (conn_id.clone(), cancel.clone()));
    }
    let worker_app = app.clone();
    let worker_id = query_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let app = worker_app;
        let query_id = worker_id;
        // SQLite 不需要密码；其余从密钥库取（键 = db:<conn_id>）。
        let password = if cfg.kind == SupportedDb::Sqlite {
            None
        } else {
            KeyringStore::get_token(&credential_key(&conn_id)).ok()
        };

        let roots = allowed_roots(&app);
        let mut pool = DbPool::connect(&cfg, password.as_deref(), &roots)
            .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;

        // ===== 安全闸门（A3 的 query 不判写，全链路归此处）=====
        let encrypted = pool.encrypted();
        sp::evaluate_db_query_gate(&sql, &cfg, encrypted, confirm_write)
            .map_err(|code| code.as_str().to_string())?;

        // 执行取数。
        let result = pool
            .query(&sql, &cancel, timeout_secs, &query_id)
            .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;

        // 审计 detail 禁含 SQL 原文与凭据（G-3）：只记连接标识与截断标记。
        log_audit(
            &app,
            "db.query",
            format!(
                "conn_id={} rows={} truncated={} field_truncated={}",
                conn_id, result.row_count, result.truncated, result.field_truncated
            ),
        );
        Ok(result)
    })
    .await
    .map_err(|_| "DB_WORKER_FAILED".to_string());
    app.state::<DbConnectionRegistry>()
        .queries
        .lock()
        .unwrap()
        .remove(&query_id);
    result?
}

#[tauri::command]
pub fn db_disconnect(
    app: AppHandle,
    webview: tauri::Webview,
    conn_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "db_disconnect", None, &app)?;
    load_registry_if_empty(&app);
    if app
        .state::<DbConnectionRegistry>()
        .queries
        .lock()
        .unwrap()
        .values()
        .any(|(id, _)| id == &conn_id)
    {
        return Err("DB_QUERY_BUSY".into());
    }
    let removed = {
        let reg = app.state::<DbConnectionRegistry>();
        let mut guard = reg.configs.lock().unwrap();
        if let Some(prior) = guard.remove(&conn_id) {
            if let Err(e) = persist_registry(&app, &guard) {
                guard.insert(conn_id.clone(), prior);
                return Err(e);
            }
            true
        } else { false }
    };
    // 一并撤销密钥库中的凭据（G-1：命名空间隔离，不影响 git 的 repo_id）。
    let _ = KeyringStore::delete_token(&credential_key(&conn_id));
    log_audit(
        &app,
        "db.disconnect",
        format!("conn_id={} removed={}", conn_id, removed),
    );
    if removed {
        Ok(())
    } else {
        Err("DB_NOT_CONNECTED".to_string())
    }
}

#[cfg(test)]
mod persistent_registry_tests {
    use super::*;
    use crate::domain::DbSslMode;
    #[test]
    fn database_metadata_survives_roundtrip_without_passwords() {
        let path = std::env::temp_dir().join(format!("browseros-db-registry-{}.json", Uuid::new_v4()));
        let cfg = DbConnectionConfig {
            id: Uuid::new_v4().to_string(),
            name: "IDEA imported MySQL".into(),
            kind: SupportedDb::MySql,
            host: Some("127.0.0.1".into()),
            port: Some(3306),
            database: "sample".into(),
            username: Some("reader".into()),
            ssl_mode: DbSslMode::Require,
            allow_write: false,
            production_hint: None,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        persist_configs_at(&path, &[cfg.clone()]).expect("persist config");
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("password"));
        let next: Vec<DbConnectionConfig> = crate::workspace::load_json_list_at(
            &path, "test db registry");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, cfg.id);
        assert_eq!(next[0].database, "sample");
        assert!(!next[0].allow_write);
        let _ = std::fs::remove_file(path);
    }
}
