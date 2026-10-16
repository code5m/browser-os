//! Database sessions own one driver connection on one worker thread.
//! The UI never owns a socket or transaction. Every user statement uses the
//! existing production/write classifier; closing or expiry rolls back first.
use super::table_data::{self, TableRequest, TableState};
// DEVELOPER_STUDIO_TABLE_ACTOR
use super::{
    commands::{ensure_profiles_loaded, DbConnectionRegistry},
    database::{credential_key, DbError, DbPool, DbQueryResult as DriverQueryResult, QueryCancel},
};
use crate::{
    domain::{DbConnectionConfig, DbErrorCode, SupportedDb},
    keyring_store::KeyringStore,
    security_policy as sp,
    shared::invocation::check_invocation_source,
};
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, Manager, Webview};

const MAX_SESSIONS: usize = 4;
const IDLE_SECONDS: u64 = 300;
const CALL_SECONDS: u64 = 75;
#[derive(Clone, Deserialize, Default)]
pub struct SessionRequest {
    pub action: String,
    pub sql: Option<String>,
    #[serde(default)]
    pub statements: Vec<String>,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub writable: bool,
    pub timeout_secs: Option<u64>,
    pub table: Option<TableRequest>,
}
struct Envelope {
    request: SessionRequest,
    response: SyncSender<Result<Value, String>>,
}
struct SessionHandle {
    conn_id: String,
    config_digest: String,
    sender: SyncSender<Envelope>,
    busy: AtomicBool,
    closing: AtomicBool,
    closed: AtomicBool,
    cancel: Mutex<Option<QueryCancel>>,
}
#[derive(Default)]
pub struct DatabaseStudioRuntime {
    sessions: Arc<Mutex<HashMap<String, Arc<SessionHandle>>>>,
}
impl Drop for DatabaseStudioRuntime {
    fn drop(&mut self) {
        if let Ok(sessions) = self.sessions.lock() {
            for handle in sessions.values() {
                handle.closing.store(true, Ordering::Release);
                if let Ok(cancel) = handle.cancel.lock() {
                    if let Some(cancel) = cancel.as_ref() {
                        cancel.cancel();
                    }
                }
                let (response, _) = mpsc::sync_channel(1);
                let _ = handle.sender.try_send(Envelope {
                    request: SessionRequest {
                        action: "close".into(),
                        ..Default::default()
                    },
                    response,
                });
            }
        }
    }
}
fn digest(cfg: &DbConnectionConfig) -> Result<String, String> {
    let bytes = serde_json::to_vec(cfg).map_err(|_| "DB_SESSION_CONFIG_INVALID")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn config(app: &AppHandle, id: &str) -> Result<DbConnectionConfig, String> {
    ensure_profiles_loaded(app)?;
    app.state::<DbConnectionRegistry>()
        .configs
        .lock()
        .map_err(|_| "DB_SESSION_LOCK_FAILED")?
        .get(id)
        .filter(|cfg| cfg.enabled)
        .cloned()
        .ok_or_else(|| "DB_SESSION_SOURCE_DISABLED_OR_REMOVED".into())
}
fn verify_config(app: &AppHandle, handle: &SessionHandle) -> Result<DbConnectionConfig, String> {
    let cfg = config(app, &handle.conn_id)?;
    if digest(&cfg)? != handle.config_digest {
        return Err("DB_SESSION_SOURCE_CHANGED_REOPEN_REQUIRED".into());
    }
    Ok(cfg)
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TransactionState {
    Idle,
    ReadOnly,
    ReadWrite,
    Failed,
    Unknown,
}
struct ConnectionOwner {
    pool: DbPool,
    transaction: TransactionState,
    tables: TableState,
    reusable: bool,
}
impl ConnectionOwner {
    fn new(pool: DbPool) -> Self {
        Self {
            pool,
            transaction: TransactionState::Idle,
            tables: TableState::default(),
            reusable: true,
        }
    }
}
impl Drop for ConnectionOwner {
    fn drop(&mut self) {
        if self.reusable && self.transaction != TransactionState::Idle {
            let _ = control(&mut self.pool, "ROLLBACK");
        }
    }
}
/// Fixed internal transaction statements, never caller-supplied SQL.
fn control(pool: &mut DbPool, text: &str) -> Result<(), String> {
    if ![
        "BEGIN",
        "BEGIN READ ONLY",
        "START TRANSACTION READ ONLY",
        "START TRANSACTION",
        "COMMIT",
        "ROLLBACK",
    ]
    .contains(&text)
    {
        return Err("DB_SESSION_CONTROL_DENIED".into());
    }
    match pool {
        DbPool::Sqlite(conn) => conn
            .execute(text, [])
            .map(|_| ())
            .map_err(|_| "DB_SESSION_TRANSACTION_FAILED".into()),
        DbPool::MySql(conn, _) => conn
            .query_drop(text)
            .map_err(|_| "DB_SESSION_TRANSACTION_FAILED".into()),
        DbPool::Postgres(conn) => conn
            .execute(text, &[])
            .map(|_| ())
            .map_err(|_| "DB_SESSION_TRANSACTION_FAILED".into()),
    }
}
fn authorize_transaction(
    cfg: &DbConnectionConfig,
    pool: &DbPool,
    writable: bool,
    confirmed: bool,
) -> Result<(), String> {
    if writable {
        // Reuse the exact production verdict rather than treating an explicit
        // nonproduction checkbox as stronger than unknown host/TLS signals.
        let signals = sp::build_db_signals(cfg, pool.encrypted());
        let verdict = sp::is_production_database(&signals);
        if !confirmed || !cfg.allow_write || verdict != sp::ProductionVerdict::NonProduction {
            return Err("DB_SESSION_WRITE_TRANSACTION_DENIED".into());
        }
    }
    Ok(())
}
fn begin(
    owner: &mut ConnectionOwner,
    cfg: &DbConnectionConfig,
    writable: bool,
    confirmed: bool,
) -> Result<(), String> {
    if owner.transaction != TransactionState::Idle {
        return Err("DB_SESSION_TRANSACTION_ALREADY_ACTIVE".into());
    }
    authorize_transaction(cfg, &owner.pool, writable, confirmed)?;
    let text = match (&owner.pool, writable) {
        (DbPool::MySql(_, _), false) => "START TRANSACTION READ ONLY",
        (DbPool::MySql(_, _), true) => "START TRANSACTION",
        (DbPool::Postgres(_), false) => "BEGIN READ ONLY",
        _ => "BEGIN",
    };
    if let Err(message) = control(&mut owner.pool, text) {
        return Err(terminal_fault(
            owner,
            "DB_SESSION_BEGIN_OUTCOME_UNKNOWN",
            &message,
        ));
    }
    owner.transaction = if writable {
        TransactionState::ReadWrite
    } else {
        TransactionState::ReadOnly
    };
    Ok(())
}
/// Only ordinary DML is allowed inside an owned transaction. DDL/admin
/// statements may implicitly commit on some drivers and are not atomic batches.
fn studio_transactional_sql(sql: &str) -> bool {
    if sp::classify_sql_risk(sql).is_read_only() {
        return true;
    }
    let word = sql
        .trim_start()
        .split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE")
}
/// Versioned structured failure inside the existing string-error IPC contract.
/// Retire unusable connections before serialization; preserve unknown outcomes.
fn fault(owner: &ConnectionOwner, code: &str, message: &str, discard: bool) -> String {
    json!({"kind":"database_session_fault","version":1,"code":code,"message":message,
        "discard_connection":discard,"transaction":owner.transaction,
        "outcome_unknown":owner.transaction == TransactionState::Unknown})
    .to_string()
}
fn terminal_fault(owner: &mut ConnectionOwner, code: &str, message: &str) -> String {
    owner.reusable = false;
    owner.transaction = TransactionState::Unknown;
    owner.tables = TableState::default();
    fault(owner, code, message, true)
}
fn finish_control(owner: &mut ConnectionOwner, command: &str) -> Result<(), String> {
    if !owner.reusable {
        return Err(fault(
            owner,
            "DB_SESSION_CONNECTION_NOT_REUSABLE",
            "Close this session and verify the database outcome",
            true,
        ));
    }
    if let Err(message) = control(&mut owner.pool, command) {
        let code = if command == "COMMIT" {
            "DB_SESSION_COMMIT_OUTCOME_UNKNOWN_CHECK_DATABASE"
        } else {
            "DB_SESSION_ROLLBACK_OUTCOME_UNKNOWN_CLOSE_SESSION"
        };
        return Err(terminal_fault(owner, code, &message));
    }
    owner.transaction = TransactionState::Idle;
    owner.tables = TableState::default();
    Ok(())
}
fn rollback_active(owner: &mut ConnectionOwner) -> Result<(), String> {
    if !owner.reusable {
        return Err(fault(
            owner,
            "DB_SESSION_CONNECTION_NOT_REUSABLE",
            "A rollback cannot be confirmed on this connection",
            true,
        ));
    }
    if owner.transaction != TransactionState::Idle {
        finish_control(owner, "ROLLBACK")?;
    }
    Ok(())
}
fn finish_query(
    owner: &mut ConnectionOwner,
    result: Result<DriverQueryResult, DbError>,
    cancel: &QueryCancel,
    write: bool,
) -> Result<Value, String> {
    // Never erase DbError::discard_connection by mapping to text first.
    if let Err(error) = &result {
        if error.discard_connection {
            return Err(terminal_fault(owner, error.code_str(), &error.message));
        }
    }
    let interrupted = cancel.is_cancelled()
        || result
            .as_ref()
            .err()
            .is_some_and(|e| matches!(e.code, DbErrorCode::Cancelled | DbErrorCode::Timeout));
    if interrupted {
        if write && owner.transaction == TransactionState::Idle {
            return Err(terminal_fault(
                owner,
                "DB_SESSION_WRITE_OUTCOME_UNKNOWN_CHECK_DATABASE",
                "Do not retry an interrupted autocommit write",
            ));
        }
        rollback_active(owner)?;
        return Err(fault(
            owner,
            "DB_SESSION_CANCELLED_OR_TIMED_OUT",
            "Operation stopped; any owned transaction rollback was acknowledged",
            false,
        ));
    }
    match result {
        Ok(result) => serde_json::to_value(result).map_err(|_| "DB_SESSION_RESULT_INVALID".into()),
        Err(error) => {
            if owner.transaction != TransactionState::Idle {
                owner.transaction = TransactionState::Failed;
            }
            Err(fault(owner, error.code_str(), &error.message, false))
        }
    }
}
fn run_sql(
    owner: &mut ConnectionOwner,
    cfg: &DbConnectionConfig,
    sql: &str,
    confirmed: bool,
    cancel: &QueryCancel,
    timeout: Option<u64>,
) -> Result<Value, String> {
    if !owner.reusable {
        return Err(fault(
            owner,
            "DB_SESSION_CONNECTION_NOT_REUSABLE",
            "Close the retired session",
            true,
        ));
    }
    if owner.transaction == TransactionState::Failed {
        return Err("DB_SESSION_ROLLBACK_REQUIRED".into());
    }
    if owner.transaction != TransactionState::Idle && !studio_transactional_sql(sql) {
        return Err("DB_SESSION_NON_TRANSACTIONAL_STATEMENT_DENIED".into());
    }
    let write = !sp::classify_sql_risk(sql).is_read_only();
    if owner.transaction == TransactionState::ReadOnly && write {
        return Err("DB_SESSION_READ_ONLY_TRANSACTION".into());
    }
    sp::evaluate_db_query_gate(sql, cfg, owner.pool.encrypted(), confirmed)
        .map_err(|code| code.as_str().to_string())?;
    if cancel.is_cancelled() {
        return Err("DB_SESSION_CANCELLED_BEFORE_EXECUTION".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let result = owner.pool.query(sql, cancel, timeout, &id);
    finish_query(owner, result, cancel, write)
}
fn transact(
    owner: &mut ConnectionOwner,
    cfg: &DbConnectionConfig,
    request: &SessionRequest,
    cancel: &QueryCancel,
) -> Result<Value, String> {
    if !owner.reusable && request.action != "status" {
        return Err(fault(
            owner,
            "DB_SESSION_CONNECTION_NOT_REUSABLE",
            "Close the retired session",
            true,
        ));
    }
    match request.action.as_str() {
        "table_read" => table_data::read(
            &mut owner.pool,
            cfg,
            &mut owner.tables,
            request.table.as_ref().ok_or("DB_TABLE_REQUEST_REQUIRED")?,
            cancel,
        ),
        "table_preview" => {
            if owner.transaction != TransactionState::Idle {
                return Err("DB_TABLE_FINISH_TRANSACTION_FIRST".into());
            }
            table_data::preview(
                &mut owner.pool,
                cfg,
                &mut owner.tables,
                request.table.as_ref().ok_or("DB_TABLE_REQUEST_REQUIRED")?,
                cancel,
            )
        }
        "table_cancel" => Ok(table_data::cancel_preview(
            &mut owner.tables,
            request.table.as_ref().and_then(|t| t.ticket.as_deref()),
        )),
        "table_apply" => {
            if owner.transaction != TransactionState::Idle {
                return Err("DB_TABLE_FINISH_TRANSACTION_FIRST".into());
            }
            authorize_transaction(cfg, &owner.pool, true, request.confirmed)?;
            match table_data::apply(
                &mut owner.pool,
                cfg,
                &mut owner.tables,
                request.table.as_ref().ok_or("DB_TABLE_REQUEST_REQUIRED")?,
                request.confirmed,
                cancel,
            ) {
                Ok(reply) => {
                    owner.transaction = TransactionState::ReadWrite;
                    Ok(reply)
                }
                Err(error) => {
                    if error.contains("OUTCOME_UNKNOWN") {
                        return Err(terminal_fault(
                            owner,
                            "DB_TABLE_ROLLBACK_OUTCOME_UNKNOWN_CLOSE_SESSION",
                            &error,
                        ));
                    }
                    Err(error)
                }
            }
        }
        "status" => Ok(json!({"transaction":owner.transaction})),
        "begin" => {
            begin(owner, cfg, request.writable, request.confirmed)?;
            Ok(json!({"transaction":owner.transaction}))
        }
        "commit" => {
            if owner.transaction == TransactionState::Failed {
                return Err("DB_SESSION_ROLLBACK_REQUIRED".into());
            }
            if owner.transaction == TransactionState::Idle {
                return Err("DB_SESSION_NO_TRANSACTION".into());
            }
            if !request.confirmed {
                return Err("DB_SESSION_COMMIT_CONFIRMATION_REQUIRED".into());
            }
            if owner.transaction == TransactionState::ReadWrite {
                authorize_transaction(cfg, &owner.pool, true, request.confirmed)?;
            }
            finish_control(owner, "COMMIT")?;
            Ok(json!({"transaction":owner.transaction,"committed":true}))
        }
        "rollback" => {
            rollback_active(owner)?;
            owner.tables = TableState::default();
            Ok(json!({"transaction":owner.transaction,"rolled_back":true}))
        }
        "execute" => {
            let result = run_sql(
                owner,
                cfg,
                request.sql.as_deref().ok_or("DB_SESSION_SQL_REQUIRED")?,
                request.confirmed,
                cancel,
                request.timeout_secs,
            )?;
            Ok(json!({"result":result,"transaction":owner.transaction}))
        }
        "batch" => {
            if request.statements.is_empty()
                || request.statements.len() > 32
                || request.statements.iter().map(String::len).sum::<usize>() > 65536
            {
                return Err("DB_SESSION_BATCH_LIMIT".into());
            }
            if owner.transaction != TransactionState::Idle {
                return Err("DB_SESSION_BATCH_REQUIRES_IDLE_SESSION".into());
            }
            let writable = request
                .statements
                .iter()
                .any(|s| !sp::classify_sql_risk(s).is_read_only());
            // Validate the entire batch before starting; a later denied statement
            // must not follow earlier successful mutations.
            for sql in &request.statements {
                if !studio_transactional_sql(sql) {
                    return Err("DB_SESSION_NON_TRANSACTIONAL_BATCH_DENIED".into());
                }
                sp::evaluate_db_query_gate(sql, cfg, owner.pool.encrypted(), request.confirmed)
                    .map_err(|c| c.as_str().to_string())?;
            }
            begin(owner, cfg, writable, request.confirmed)?;
            let mut results = Vec::new();
            let mut bytes = 0usize;
            for sql in &request.statements {
                match run_sql(
                    owner,
                    cfg,
                    sql,
                    request.confirmed,
                    cancel,
                    request.timeout_secs,
                ) {
                    Ok(result) => {
                        bytes += result.to_string().len();
                        if bytes > 4 * 1024 * 1024 {
                            rollback_active(owner)?;
                            return Err("DB_SESSION_BATCH_RESULT_LIMIT_ROLLED_BACK".into());
                        }
                        results.push(result);
                    }
                    Err(error) => {
                        if owner.reusable {
                            rollback_active(owner)?;
                        }
                        return Err(error);
                    }
                }
            }
            // Read batches can finish automatically. Write batches remain in the
            // same explicit transaction until a separate confirmed Commit.
            if !writable {
                finish_control(owner, "COMMIT")?;
            }
            Ok(
                json!({"results":results,"transaction":owner.transaction,"requires_commit":writable}),
            )
        }
        _ => Err("DB_SESSION_ACTION_DENIED".into()),
    }
}
fn actor(
    app: AppHandle,
    id: String,
    cfg: DbConnectionConfig,
    handle: Arc<SessionHandle>,
    receiver: mpsc::Receiver<Envelope>,
    ready: SyncSender<Result<Value, String>>,
    sessions: Arc<Mutex<HashMap<String, Arc<SessionHandle>>>>,
) {
    let run = || -> Result<(), String> {
        let password = if cfg.kind == SupportedDb::Sqlite {
            None
        } else {
            KeyringStore::get_token(&credential_key(&cfg.id)).ok()
        };
        let pool = DbPool::connect(
            &cfg,
            password.as_deref(),
            &crate::workspace::allowed_roots(&app),
        )
        .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;
        drop(password);
        let encrypted = pool.encrypted();
        let mut owner = ConnectionOwner::new(pool);
        if ready.send(Ok(json!({"session_id":id,"conn_id":cfg.id,"idle_timeout_seconds":IDLE_SECONDS,"encrypted":encrypted,"transaction":"idle"}))).is_err(){return Ok(());}
        while !handle.closing.load(Ordering::Acquire) {
            let envelope = match receiver.recv_timeout(Duration::from_secs(IDLE_SECONDS)) {
                Ok(value) => value,
                Err(_) => break,
            };
            if envelope.request.action == "close" || handle.closing.load(Ordering::Acquire) {
                let _ = envelope.response.send(Ok(json!({"closed":true})));
                break;
            }
            let current = verify_config(&app, &handle);
            if let Err(error) = current {
                let _ = envelope.response.send(Err(error));
                break;
            }
            let cancel = QueryCancel::new();
            if let Ok(mut active) = handle.cancel.lock() {
                *active = Some(cancel.clone());
            }
            let outcome = transact(&mut owner, &cfg, &envelope.request, &cancel);
            if let Ok(mut active) = handle.cancel.lock() {
                *active = None;
            }
            crate::workspace::log_audit(
                &app,
                "db.session.operation",
                format!(
                    "conn_id={} action={} success={}",
                    cfg.id,
                    envelope.request.action,
                    outcome.is_ok()
                ),
            );
            let retired = !owner.reusable;
            if retired {
                handle.closing.store(true, Ordering::Release);
            }
            if envelope.response.send(outcome).is_err() || retired {
                break;
            }
        }
        // ConnectionOwner::drop rolls back any uncommitted transaction before
        // driver resources leave this worker. No commit is performed on close.
        Ok(())
    };
    if let Err(error) = run() {
        let _ = ready.send(Err(error));
    }
    handle.closed.store(true, Ordering::Release);
    handle.busy.store(false, Ordering::Release);
    if let Ok(mut map) = sessions.lock() {
        map.remove(&id);
    }
}
#[tauri::command]
pub async fn db_studio_open(
    app: AppHandle,
    webview: Webview,
    conn_id: String,
) -> Result<Value, String> {
    check_invocation_source(&webview, "db_studio_open", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = config(&app, &conn_id)?;
        let id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = mpsc::sync_channel(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let handle = Arc::new(SessionHandle {
            conn_id,
            config_digest: digest(&cfg)?,
            sender,
            busy: AtomicBool::new(false),
            closing: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            cancel: Mutex::new(None),
        });
        let sessions = app.state::<DatabaseStudioRuntime>().sessions.clone();
        {
            let mut map = sessions.lock().map_err(|_| "DB_SESSION_LOCK_FAILED")?;
            map.retain(|_, h| !h.closed.load(Ordering::Acquire));
            if map.len() >= MAX_SESSIONS {
                return Err("DB_SESSION_LIMIT".into());
            }
            map.insert(id.clone(), handle.clone());
        }
        let worker_handle = handle.clone();
        let worker_id = id.clone();
        let worker_sessions = sessions.clone();
        if thread::Builder::new()
            .name("browseros-db-session".into())
            .spawn(move || {
                actor(
                    app,
                    worker_id,
                    cfg,
                    worker_handle,
                    receiver,
                    ready_tx,
                    worker_sessions,
                )
            })
            .is_err()
        {
            if let Ok(mut map) = sessions.lock() {
                map.remove(&id);
            }
            return Err("DB_SESSION_WORKER_UNAVAILABLE".into());
        }
        match ready_rx.recv_timeout(Duration::from_secs(15)) {
            Ok(result) => result,
            Err(_) => {
                handle.closing.store(true, Ordering::Release);
                Err("DB_SESSION_CONNECT_TIMEOUT".into())
            }
        }
    })
    .await
    .map_err(|_| "DB_SESSION_WORKER_FAILED".to_string())?
}
struct BusyGuard(Arc<SessionHandle>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::Release);
    }
}
#[tauri::command]
pub async fn db_studio_call(
    app: AppHandle,
    webview: Webview,
    session_id: String,
    request: SessionRequest,
) -> Result<Value, String> {
    check_invocation_source(&webview, "db_studio_call", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let handle = app
            .state::<DatabaseStudioRuntime>()
            .sessions
            .lock()
            .map_err(|_| "DB_SESSION_LOCK_FAILED")?
            .get(&session_id)
            .cloned()
            .ok_or("DB_SESSION_EXPIRED_OR_CLOSED")?;
        if handle.closing.load(Ordering::Acquire) || handle.closed.load(Ordering::Acquire) {
            return Err("DB_SESSION_CLOSED".into());
        }
        verify_config(&app, &handle)?;
        if handle
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("DB_SESSION_BUSY".into());
        }
        let _busy = BusyGuard(handle.clone());
        let (response, receive) = mpsc::sync_channel(1);
        handle
            .sender
            .try_send(Envelope { request, response })
            .map_err(|_| "DB_SESSION_QUEUE_UNAVAILABLE")?;
        match receive.recv_timeout(Duration::from_secs(CALL_SECONDS)) {
            Ok(result) => result,
            Err(_) => {
                handle.closing.store(true, Ordering::Release);
                if let Ok(cancel) = handle.cancel.lock() {
                    if let Some(cancel) = cancel.as_ref() {
                        cancel.cancel();
                    }
                }
                Err("DB_SESSION_RESULT_UNCERTAIN_DO_NOT_RETRY_WRITES".into())
            }
        }
    })
    .await
    .map_err(|_| "DB_SESSION_WORKER_FAILED".to_string())?
}
#[tauri::command]
pub fn db_studio_close(
    app: AppHandle,
    webview: Webview,
    session_id: String,
) -> Result<Value, String> {
    check_invocation_source(&webview, "db_studio_close", None, &app)?;
    let handle = app
        .state::<DatabaseStudioRuntime>()
        .sessions
        .lock()
        .map_err(|_| "DB_SESSION_LOCK_FAILED")?
        .get(&session_id)
        .cloned();
    if let Some(handle) = handle {
        handle.closing.store(true, Ordering::Release);
        if let Ok(cancel) = handle.cancel.lock() {
            if let Some(cancel) = cancel.as_ref() {
                cancel.cancel();
            }
        }
        let (response, _) = mpsc::sync_channel(1);
        let _ = handle.sender.try_send(Envelope {
            request: SessionRequest {
                action: "close".into(),
                ..Default::default()
            },
            response,
        });
        return Ok(
            json!({"close_requested":true,"closed":handle.closed.load(Ordering::Acquire),"pending_rollback":true}),
        );
    }
    Ok(json!({"closed":true,"pending_rollback":false}))
}
#[tauri::command]
pub fn db_studio_cancel(
    app: AppHandle,
    webview: Webview,
    session_id: String,
) -> Result<Value, String> {
    check_invocation_source(&webview, "db_studio_cancel", None, &app)?;
    let handle = app
        .state::<DatabaseStudioRuntime>()
        .sessions
        .lock()
        .map_err(|_| "DB_SESSION_LOCK_FAILED")?
        .get(&session_id)
        .cloned()
        .ok_or("DB_SESSION_EXPIRED_OR_CLOSED")?;
    let cancel = handle.cancel.lock().map_err(|_| "DB_SESSION_LOCK_FAILED")?;
    if let Some(cancel) = cancel.as_ref() {
        cancel.cancel();
        Ok(json!({"cancel_requested":true,"completed":false}))
    } else {
        Ok(json!({"cancel_requested":false,"completed":!handle.busy.load(Ordering::Acquire)}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cfg() -> DbConnectionConfig {
        serde_json::from_value(json!({"id":"session-fixture","name":"test","kind":"sqlite","host":null,"port":null,"database":"/tmp/browseros-test.db","username":null,"ssl_mode":"disable","allow_write":true,"production_hint":false,"enabled":true,"created_at":"2026-10-16T00:00:00Z","updated_at":"2026-10-16T00:00:00Z"})).unwrap()
    }
    fn owner() -> ConnectionOwner {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE fixture(id INTEGER PRIMARY KEY,value TEXT)",
            [],
        )
        .unwrap();
        ConnectionOwner::new(DbPool::Sqlite(conn))
    }
    #[test]
    fn studio_fatal_driver_error_retires_connection() {
        let mut owner = owner();
        let error = DbError::fatal(DbErrorCode::Timeout, "fixture hard timeout");
        let failure = finish_query(&mut owner, Err(error), &QueryCancel::new(), false).unwrap_err();
        let payload: Value = serde_json::from_str(&failure).unwrap();
        assert_eq!(payload["discard_connection"], true);
        assert_eq!(payload["transaction"], "unknown");
        assert!(!owner.reusable);
        assert!(run_sql(
            &mut owner,
            &cfg(),
            "SELECT 1",
            false,
            &QueryCancel::new(),
            None
        )
        .is_err());
    }
    #[test]
    fn studio_failed_rollback_never_reports_idle() {
        let mut owner = owner();
        owner.transaction = TransactionState::ReadWrite;
        assert!(finish_control(&mut owner, "ROLLBACK").is_err());
        assert_eq!(owner.transaction, TransactionState::Unknown);
        assert!(!owner.reusable);
    }
    #[test]
    fn studio_commit_failure_is_unknown_and_not_retried() {
        let mut owner = owner();
        if let DbPool::Sqlite(conn) = &owner.pool {
            conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        }
        begin(&mut owner, &cfg(), true, true).unwrap();
        if let DbPool::Sqlite(conn) = &owner.pool {
            conn.execute("INSERT INTO child VALUES(9)", []).unwrap();
        }
        let failure = finish_control(&mut owner, "COMMIT").unwrap_err();
        let payload: Value = serde_json::from_str(&failure).unwrap();
        assert_eq!(payload["outcome_unknown"], true);
        assert_eq!(owner.transaction, TransactionState::Unknown);
        assert!(!owner.reusable);
        assert!(transact(
            &mut owner,
            &cfg(),
            &SessionRequest {
                action: "commit".into(),
                confirmed: true,
                ..Default::default()
            },
            &QueryCancel::new()
        )
        .is_err());
    }
    #[test]
    fn studio_cancelled_transaction_checks_real_rollback() {
        let mut owner = owner();
        begin(&mut owner, &cfg(), true, true).unwrap();
        if let DbPool::Sqlite(conn) = &owner.pool {
            conn.execute("INSERT INTO fixture VALUES(1,'temporary')", [])
                .unwrap();
        }
        let error = DbError::new(DbErrorCode::Cancelled, "fixture cancellation");
        assert!(finish_query(&mut owner, Err(error), &QueryCancel::new(), true).is_err());
        assert_eq!(owner.transaction, TransactionState::Idle);
        assert!(owner.reusable);
        if let DbPool::Sqlite(conn) = &owner.pool {
            let count: i64 = conn
                .query_row("SELECT count(*) FROM fixture", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 0);
        }
    }
    #[test]
    fn studio_transaction_rollback_preserves_database() {
        let mut owner = owner();
        let cfg = cfg();
        begin(&mut owner, &cfg, true, true).unwrap();
        run_sql(
            &mut owner,
            &cfg,
            "INSERT INTO fixture VALUES(1,'temporary')",
            true,
            &QueryCancel::new(),
            None,
        )
        .unwrap();
        transact(
            &mut owner,
            &cfg,
            &SessionRequest {
                action: "rollback".into(),
                ..Default::default()
            },
            &QueryCancel::new(),
        )
        .unwrap();
        let result = run_sql(
            &mut owner,
            &cfg,
            "SELECT count(*) FROM fixture",
            false,
            &QueryCancel::new(),
            None,
        )
        .unwrap();
        assert_eq!(result["rows"][0][0]["i64"], 0);
    }
    #[test]
    fn studio_read_transaction_denies_write_even_with_confirmation() {
        let mut owner = owner();
        let cfg = cfg();
        begin(&mut owner, &cfg, false, false).unwrap();
        assert!(run_sql(
            &mut owner,
            &cfg,
            "INSERT INTO fixture VALUES(1,'denied')",
            true,
            &QueryCancel::new(),
            None
        )
        .is_err());
    }
    #[test]
    fn studio_write_batch_does_not_implicitly_commit() {
        let mut owner = owner();
        let cfg = cfg();
        let reply = transact(
            &mut owner,
            &cfg,
            &SessionRequest {
                action: "batch".into(),
                statements: vec![
                    "INSERT INTO fixture VALUES(1,'a')".into(),
                    "INSERT INTO fixture VALUES(2,'b')".into(),
                ],
                confirmed: true,
                ..Default::default()
            },
            &QueryCancel::new(),
        )
        .unwrap();
        assert_eq!(reply["requires_commit"], true);
        assert_eq!(owner.transaction, TransactionState::ReadWrite);
        assert!(transact(
            &mut owner,
            &cfg,
            &SessionRequest {
                action: "commit".into(),
                ..Default::default()
            },
            &QueryCancel::new()
        )
        .is_err());
    }
    #[test]
    fn studio_batch_error_rolls_back_every_statement() {
        let mut owner = owner();
        let cfg = cfg();
        assert!(transact(
            &mut owner,
            &cfg,
            &SessionRequest {
                action: "batch".into(),
                statements: vec![
                    "INSERT INTO fixture VALUES(1,'a')".into(),
                    "INSERT INTO missing_table VALUES(2,'b')".into()
                ],
                confirmed: true,
                ..Default::default()
            },
            &QueryCancel::new()
        )
        .is_err());
        let result = run_sql(
            &mut owner,
            &cfg,
            "SELECT count(*) FROM fixture",
            false,
            &QueryCancel::new(),
            None,
        )
        .unwrap();
        assert_eq!(result["rows"][0][0]["i64"], 0);
    }
    #[test]
    fn studio_production_transaction_is_not_authorized_by_checkbox() {
        let mut owner = owner();
        let mut cfg = cfg();
        cfg.production_hint = Some(true);
        assert!(begin(&mut owner, &cfg, true, true).is_err());
    }
    #[test]
    fn studio_config_digest_observes_identity_and_permissions() {
        let a = cfg();
        let mut b = a.clone();
        b.allow_write = false;
        assert_ne!(digest(&a).unwrap(), digest(&b).unwrap());
    }
}

#[cfg(test)]
mod table_actor_tests {
    use super::*;
    #[test]
    fn studio_atomic_batch_rejects_implicit_commit_statements() {
        for sql in [
            "CREATE TABLE x(id INT)",
            "ALTER TABLE x ADD v INT",
            "DROP TABLE x",
            "TRUNCATE TABLE x",
            "SET autocommit=1",
            "COMMIT",
        ] {
            assert!(!studio_transactional_sql(sql), "{sql}");
        }
        assert!(studio_transactional_sql("SELECT 1"));
        assert!(studio_transactional_sql("UPDATE x SET v=1 WHERE id=1"));
    }
}
