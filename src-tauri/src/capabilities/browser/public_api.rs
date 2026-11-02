//! Stable, read-only Browser Owner API for the Agent Channel host.
//! Only redacted tab metadata leaves this module; no page content or URL.
use serde_json::{json, Value};
use tauri::AppHandle;
pub(crate) fn agent_tab_summary(app: AppHandle) -> Value {
    let tabs = crate::bridge::tab_list_native(app);
    let count = tabs.len();
    json!({"count":count,"items":tabs.into_iter().take(100)
        .map(|tab| json!({"id":tab.id,"title":tab.title})).collect::<Vec<_>>()})
}
