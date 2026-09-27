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
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::database::{credential_key, DbPool, QueryCancel, DbQueryResult};
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
    {
        let reg = app.state::<DbConnectionRegistry>();
        if reg.configs.lock().unwrap().len() >= 64 {
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
    let _probe = DbPool::connect(&cfg, password.as_deref(), &roots)
        .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;
    // G-1：凭据（若有）写入系统密钥库，键必须带 `db:` 前缀，与 git 的 repo_id 命名空间隔离。
    if let Some(p) = password {
        KeyringStore::save_token(&credential_key(&conn_id), &p)?;
    }
    {
        let reg = app.state::<DbConnectionRegistry>();
        reg.configs.lock().unwrap().insert(conn_id.clone(), cfg);
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
        guard.remove(&conn_id).is_some()
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
