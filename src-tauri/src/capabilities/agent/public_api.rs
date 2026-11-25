//! Agent-owned facade for approved Coding requests and native lifecycle.
//! User-request routes keep source/approval validation; startup and shutdown
//! helpers are Rust-internal and are never registered as Tauri commands.
use serde_json::Value;
use tauri::{AppHandle, Webview};

pub(crate) use super::execution_owner::ProductionExecution;
pub(crate) fn register_coding_run(id: &str) -> Result<super::coding::RunRequestGuard, String> {
    super::coding::register_run_request(id)
}
pub(crate) fn cancel_coding_run(id: &str) -> Result<bool, String> {
    super::coding::cancel_run_request(id)
}
pub(crate) fn coding_lsp_summary(
    app: AppHandle,
    webview: Webview,
    root: &str,
) -> Result<Value, String> {
    if webview.label() != "main" || root.len() > 4096 {
        return Err("AGENT_SOURCE_DENIED".into());
    }
    crate::shared::invocation::check_invocation_source(&webview, "coding_service", None, &app)?;
    let root = std::path::Path::new(root);
    if !root.is_absolute() || !root.is_dir() {
        return Err("AGENT_ROOT_INVALID".into());
    }
    let canonical = root.canonicalize().map_err(|_| "AGENT_ROOT_INVALID")?;
    super::coding_lsp::execute(&canonical, serde_json::json!({"action":"lsp_status"}))
}

/// Bootstrap a durable Coding session *before* creating its Foundation task.
/// Never let the framework invent a session ID different from the persisted
/// Coding Owner identity. Read-only and non-Coding actions create no session.
/// The owner serializes and atomically persists the new session first; if that
/// fails, no task is started and no model/provider operation is invoked.
pub(crate) fn prepare_coding_session(
    root: &std::path::Path,
    storage: &std::path::Path,
    request: &mut Value,
) -> Result<Option<String>, String> {
    let action = request["action"].as_str().unwrap_or("");
    if !matches!(
        action,
        "session"
            | "chat"
            | "propose"
            | "discard"
            | "apply"
            | "claim_operation"
            | "resolve_operation"
            | "reconcile_operation"
            | "export_changes"
    ) {
        return Ok(None);
    }
    match request.get("session_id") {
        Some(Value::String(id)) if !id.is_empty() => {
            uuid::Uuid::parse_str(id).map_err(|_| "AGENT_SESSION_INVALID")?;
            return Ok(None);
        }
        None | Some(Value::Null) | Some(Value::String(_)) => {}
        _ => return Err("AGENT_SESSION_INVALID".into()),
    }
    let initialized = super::coding::execute(
        root,
        storage,
        serde_json::json!({"action":"session"}),
        |_, _| Err("AGENT_BOOTSTRAP_PROVIDER_DENIED".into()),
    )?;
    let id = initialized["session_id"]
        .as_str()
        .ok_or("AGENT_SESSION_BOOTSTRAP_FAILED")?
        .to_owned();
    uuid::Uuid::parse_str(&id).map_err(|_| "AGENT_SESSION_BOOTSTRAP_FAILED")?;
    request
        .as_object_mut()
        .ok_or("AGENT_REQUEST_INVALID")?
        .insert("session_id".into(), Value::String(id.clone()));
    Ok(Some(id))
}

/// Single Owner-reviewed coding dispatch for Framework model transport.
/// Framework is allowed to supply the provider turn, not bypass Owner action checks.
/// All side effects (including file proposals/apply and MCP operations) stay in Coding Owner.
pub(crate) fn dispatch_coding_request(
    root: &std::path::Path,
    storage: &std::path::Path,
    request: Value,
    provider: impl FnMut(&[Value], &[Value]) -> Result<Value, String>,
) -> Result<Value, String> {
    if !root.is_absolute() || root.parent().is_none() {
        return Err("AGENT_ROOT_INVALID".into());
    }
    let action = request["action"].as_str().unwrap_or("");
    if matches!(action, "sandbox_recovery_status" | "sandbox_recovery_retry") {
        return sandbox_recovery_request(action, request["approved"] == true);
    }
    if [
        "lsp_status",
        "lsp_start",
        "lsp_request",
        "lsp_diagnostics",
        "lsp_close",
    ]
    .contains(&action)
    {
        return super::coding_lsp::execute(root, request);
    }
    if [
        "skills_list",
        "skill_read",
        "knowledge_search",
        "mcp_list",
        "mcp_review",
        "mcp_tools",
        "mcp_call",
    ]
    .contains(&action)
    {
        return super::coding_context::execute(root, storage, request);
    }
    super::coding::execute(root, storage, request, provider)
}

pub(crate) fn coding_cancelled(root: &std::path::Path, session_id: &str) -> bool {
    super::coding::cancelled(root, session_id)
}

#[cfg(test)]
mod coding_bootstrap_scope_tests {
    use super::*;

    #[test]
    fn initial_session_is_persisted_before_task_and_reused_without_random_identity() {
        let root = std::env::temp_dir().join(format!(
            "browseros-coding-bootstrap-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&root).unwrap();
        let storage = root.join("state");
        let mut request = serde_json::json!({"action":"session"});
        let session_id = prepare_coding_session(&root, &storage, &mut request)
            .unwrap()
            .expect("new session must be persisted");
        let id = uuid::Uuid::parse_str(&session_id).unwrap();
        assert_eq!(request["session_id"], session_id);
        let saved_path = storage.join("coding-sessions").join(format!("{id}.json"));
        let saved: Value = serde_json::from_slice(&std::fs::read(saved_path).unwrap()).unwrap();
        assert_eq!(saved["session_id"], session_id);
        assert_eq!(saved["status"], "ready");

        let execution = super::super::execution_owner::ProductionExecution::begin_coding(
            &root,
            Some(&session_id),
        )
        .unwrap();
        let record = execution.tasks().get(execution.id()).unwrap();
        assert_eq!(record.session_id, id);
        assert_ne!(record.workspace_id, uuid::Uuid::nil());
        execution.finish(true).unwrap();

        assert_eq!(
            prepare_coding_session(&root, &storage, &mut request).unwrap(),
            None
        );
        let mut readonly = serde_json::json!({"action":"tree"});
        assert_eq!(
            prepare_coding_session(&root, &storage, &mut readonly).unwrap(),
            None
        );
        assert!(readonly.get("session_id").is_none());
        let mut wrong = serde_json::json!({"action":"chat","session_id":17});
        assert_eq!(
            prepare_coding_session(&root, &storage, &mut wrong).unwrap_err(),
            "AGENT_SESSION_INVALID"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// Application lifecycle facade. Keep framework code out of native LSP and
/// AppContainer internals; these functions perform no user request dispatch.
pub(crate) fn request_native_shutdown() {
    #[cfg(target_os = "windows")]
    {
        super::coding_lsp::request_shutdown();
        super::windows_sandbox::request_shutdown();
    }
}

/// Independent coordinator tasks receive the same deadline. A busy LSP owner
/// must not postpone the Sandbox owner's completion report until time runs out.
pub(crate) fn shutdown_lsp_until(deadline: std::time::Instant) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        super::coding_lsp::shutdown_until(deadline)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = deadline;
        Ok(())
    }
}

pub(crate) fn shutdown_sandbox_until(deadline: std::time::Instant) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        super::windows_sandbox::shutdown_until(deadline)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = deadline;
        Ok(())
    }
}

/// Uses only the recovery store installed at trusted application startup.
/// Neither a caller-supplied path nor a record ID can become a deletion target.
fn sandbox_recovery_request(action: &str, approved: bool) -> Result<Value, String> {
    if action == "sandbox_recovery_retry" && !approved {
        return Err("SANDBOX_RECOVERY_APPROVAL_REQUIRED".into());
    }
    #[cfg(target_os = "windows")]
    {
        let status = if action == "sandbox_recovery_retry" {
            super::windows_sandbox::retry_recovery(
                std::time::Instant::now() + std::time::Duration::from_secs(10),
            )
        } else {
            super::windows_sandbox::recovery_status(
                std::time::Instant::now() + std::time::Duration::from_millis(750),
            )
        }?;
        serde_json::to_value(status).map_err(|_| "SANDBOX_RECOVERY_STATUS_INVALID".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(serde_json::json!({
            "state": "unsupported", "canStart": true, "retryAvailable": false,
            "lastCode": null, "lastReport": {
                "recovered": 0, "active": 0, "pending": 0, "rejected": 0
            },
            "pendingCleanup": 0, "records": [], "truncated": false
        }))
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn prepare_native_recovery(
    private_data: &std::path::Path,
    recover_abandoned: bool,
    deadline: std::time::Instant,
) -> Result<(), String> {
    super::windows_sandbox::initialize_recovery_store(private_data)?;
    if recover_abandoned {
        let report = super::windows_sandbox::recover_abandoned_profiles(private_data, deadline)?;
        eprintln!(
            "[sandbox-recovery] recovered={} active={} pending={} rejected={}",
            report.recovered, report.active, report.pending, report.rejected
        );
    }
    Ok(())
}
