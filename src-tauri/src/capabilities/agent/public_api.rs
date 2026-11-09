//! Agent-owned read-only Coding LSP status gateway for Agent Channel host.
//! Keeps the framework implementation's existing native caller validation.
use serde_json::Value;
use tauri::{AppHandle, Webview};
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
    if ["lsp_status", "lsp_request", "lsp_close"].contains(&action) {
        return super::coding_lsp::execute(root, request);
    }
    if [
        "skills_list",
        "skill_read",
        "knowledge_search",
        "mcp_list",
        "mcp_tools",
        "mcp_call",
    ]
    .contains(&action)
    {
        return super::coding_context::execute(root, storage, request);
    }
    super::coding::execute(root, storage, request, provider)
}

pub(crate) fn coding_cancelled(root: &std::path::Path) -> bool {
    super::coding::cancelled_for_root(root)
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
