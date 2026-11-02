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
