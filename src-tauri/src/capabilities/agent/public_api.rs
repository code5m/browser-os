//! Agent-owned read-only Coding LSP status gateway for Agent Channel host.
//! Keeps the framework implementation's existing native caller validation.
use serde_json::Value;
use tauri::{AppHandle, Webview};
pub(crate) fn coding_lsp_summary(
    app: AppHandle,
    webview: Webview,
    root: &str,
) -> Result<Value, String> {
    crate::framework::ai::coding_lsp_status_for_agent(app, webview, root)
}
