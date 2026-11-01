//! Stable read-only Database Owner API for the Agent Channel host.
//! The command still checks the original native caller and its own policy.
use serde_json::{json, Value};
use tauri::{AppHandle, Webview};
pub(crate) fn agent_connection_summary(app: AppHandle, webview: Webview) -> Result<Value, String> {
    let list = super::commands::db_list_connections(app, webview)?;
    let count = list.len();
    Ok(json!({"count":count,"items":list.into_iter().take(100)
        .map(|c|json!({"id":c.id,"name":c.name,"kind":c.kind})).collect::<Vec<_>>() }))
}
