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
