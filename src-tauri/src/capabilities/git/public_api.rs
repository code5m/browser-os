//! Stable read-only Git Owner API for the Agent Channel host.
//! Reuses native permission checks and intentionally exposes no write/confirm entry.
use serde_json::{json, Value};
use tauri::{AppHandle, Webview};
pub(crate) fn agent_status(
    app: AppHandle,
    webview: Webview,
    repo_id: String,
) -> Result<Value, String> {
    let statuses = super::commands::git_status(app, webview, repo_id)?;
    Ok(json!({"count":statuses.len(),"items":statuses.into_iter().take(100).collect::<Vec<_>>()}))
}
pub(crate) fn agent_diff(
    app: AppHandle,
    webview: Webview,
    repo_id: String,
    path: Option<String>,
) -> Result<Value, String> {
    if path.as_ref().is_some_and(|v| v.len() > 1024) {
        return Err("AGENT_EXPERIMENT_PATH_DENIED".into());
    }
    let diff = super::commands::git_diff(app, webview, repo_id, path, Some(4096))?;
    serde_json::to_value(diff).map_err(|_| "AGENT_EXPERIMENT_SERIALIZE".into())
}
