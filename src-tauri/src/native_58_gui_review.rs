//! Installed-client GUI evidence runner for the frozen legacy Native 58 surface.
//!
//! This is an opt-in, isolated test path. It exercises the real Tauri IPC bridge
//! from the installed main WebView; remote-only telemetry is exercised from a
//! separately-created HTTP tab. No test command is added to the production IPC
//! registry, and all disk/XDG effects are confined by the outer test runner.
use std::{fs, path::PathBuf, thread, time::Duration};

use tauri::{Listener, Manager};

pub fn start(app: tauri::AppHandle) -> Result<(), String> {
    let base_url = std::env::var("NATIVE_58_REVIEW_BASE_URL")
        .map_err(|_| "NATIVE_58_REVIEW_BASE_URL_REQUIRED")?;
    let report_path = std::env::var("NATIVE_58_REVIEW_REPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/browseros-native-58-review.json"));
    let url = url::Url::parse(&base_url).map_err(|_| "NATIVE_58_REVIEW_INVALID_URL")?;
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || url.port().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("NATIVE_58_REVIEW_BASE_URL_MUST_BE_LOOPBACK".into());
    }

    // Isolated deterministic fixtures. The Python runner supplies a temporary
    // XDG_DATA_HOME/HOME and an xdg-settings stub, so no host browser setting or
    // real user data is changed by this review.
    let workspace = crate::workspace::workspace_dir(&app);
    fs::write(
        workspace.join("native-58-review.md"),
        "# Native 58 GUI review\n\nA deterministic, non-sensitive fixture.\n",
    )
    .map_err(|e| format!("NATIVE_58_FIXTURE_WRITE:{e}"))?;
    {
        let state = app.state::<crate::AppState>();
        let mut drafts = state.session_drafts.lock().unwrap();
        for id in ["tab-review-discard", "tab-review-flush"] {
            drafts.insert(
                id.to_string(),
                crate::domain::SessionDraft {
                    tab_id: id.to_string(),
                    url: format!("{base_url}/page?name={id}"),
                    title: id.to_string(),
                },
            );
        }
    }
    crate::bridge::handle_open_url(&app, "https://example.invalid/native-58-pending");

    let report_app = app.clone();
    let report_path_for_event = report_path.clone();
    app.listen("native-58-review-finished", move |event| {
        let mut report: serde_json::Value = serde_json::from_str(event.payload())
            .unwrap_or_else(|_| serde_json::json!({"status":"FAIL","error":"INVALID_REPORT_JSON"}));
        let commands_pass = report["commands"].as_array().is_some_and(|items| {
            items.len() == 58 && items.iter().all(|item| item["status"] == "PASS")
        });
        let state = report_app.state::<crate::AppState>();
        let resource_tab = report["resourceTabId"].as_str().unwrap_or("");
        let resource_cleared = !resource_tab.is_empty()
            && state
                .resource_buffer
                .lock()
                .unwrap()
                .records(resource_tab)
                .is_empty();
        let discard_draft_removed = !state
            .session_drafts
            .lock()
            .unwrap()
            .contains_key("tab-review-discard");
        let flush_draft_removed = !state
            .session_drafts
            .lock()
            .unwrap()
            .contains_key("tab-review-flush");
        let pending_urls_drained = state.pending_open_urls.lock().unwrap().is_empty();
        let hibernation_restored = !state
            .hibernation_enabled
            .load(std::sync::atomic::Ordering::SeqCst);
        let tools_closed = report_app
            .webview_windows()
            .keys()
            .all(|label| !label.starts_with("tool-"));
        let tab_closed = report["resourceTabId"]
            .as_str()
            .is_some_and(|id| !state.tabs.lock().unwrap().contains_key(id));
        let restored_tab_closed = report["restoredTabId"]
            .as_str()
            .is_none_or(|id| !state.tabs.lock().unwrap().contains_key(id));
        let intent_tokens_consumed = report_app
            .state::<crate::security_policy::IntentRegistry>()
            .pending()
            == 0;
        let session_id = report["sessionId"].as_str().unwrap_or("");
        let session_deleted = !session_id.is_empty()
            && !crate::session::list_sessions(&crate::workspace::sessions_dir(&report_app))
                .iter()
                .any(|item| item.id == session_id);
        let snippet_id = report["snippetId"].as_str().unwrap_or("");
        let snippet_deleted = !snippet_id.is_empty()
            && !crate::workspace::load_snippets(&report_app)
                .iter()
                .any(|item| item.id == snippet_id);
        let bookmark_id = report["bookmarkId"].as_str().unwrap_or("");
        let bookmark_deleted = !bookmark_id.is_empty()
            && !crate::workspace::load_bookmarks(&report_app)
                .iter()
                .any(|item| item.id == bookmark_id);
        let xdg_log = std::env::var("NATIVE_58_XDG_LOG").unwrap_or_default();
        let xdg_settings_stubbed = !xdg_log.is_empty() && PathBuf::from(&xdg_log).is_file();
        let postconditions = serde_json::json!({
            "resource_buffer_cleared": resource_cleared,
            "session_discard_removed_draft": discard_draft_removed,
            "session_flush_removed_draft": flush_draft_removed,
            "pending_open_urls_drained": pending_urls_drained,
            "hibernation_setting_restored": hibernation_restored,
            "tool_windows_closed": tools_closed,
            "resource_tab_closed": tab_closed,
            "restored_tab_closed": restored_tab_closed,
            "one_shot_intents_consumed": intent_tokens_consumed,
            "session_file_deleted": session_deleted,
            "snippet_file_deleted": snippet_deleted,
            "bookmark_removed": bookmark_deleted,
            "xdg_settings_stub_present": xdg_settings_stubbed
        });
        let postconditions_pass = postconditions
            .as_object()
            .is_some_and(|items| items.values().all(|value| value == true));
        report["nativePostconditions"] = postconditions;
        report["nativePostconditionsStatus"] = if postconditions_pass {
            serde_json::json!("PASS")
        } else {
            serde_json::json!("FAIL")
        };
        report["status"] = if commands_pass && postconditions_pass {
            serde_json::json!("PASS")
        } else {
            serde_json::json!("FAIL")
        };
        report["testedWebview"] = "main-installed-tauri-webview".into();
        report["remoteWebviewRequired"] = true.into();
        if let Some(parent) = report_path_for_event.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            &report_path_for_event,
            serde_json::to_vec_pretty(&report).unwrap_or_default(),
        );
        eprintln!("NATIVE_58_GUI_REVIEW_STATUS={}", report["status"]);
        report_app.exit(if report["status"] == "PASS" { 0 } else { 1 });
    });

    let timeout_app = app.clone();
    let timeout_path = report_path.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(150));
        if !timeout_path.exists() {
            let report = serde_json::json!({
                "status":"FAIL",
                "error":"NATIVE_58_GUI_REVIEW_TIMEOUT",
                "expected_commands":58,
                "completed_commands":0
            });
            if let Some(parent) = timeout_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(
                &timeout_path,
                serde_json::to_vec_pretty(&report).unwrap_or_default(),
            );
            timeout_app.exit(2);
        }
    });

    let script = include_str!("../../scripts/native-58-gui-review.js").replace(
        "__NATIVE58_BASE_URL__",
        &serde_json::to_string(&base_url).unwrap(),
    );
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(4));
        let Some(main) = app.get_webview("main") else {
            eprintln!("NATIVE_58_GUI_REVIEW_FAIL=MAIN_WEBVIEW_MISSING");
            app.exit(2);
            return;
        };
        if let Err(error) = main.eval(&script) {
            eprintln!("NATIVE_58_GUI_REVIEW_FAIL=INJECT:{error}");
            app.exit(2);
        }
    });
    Ok(())
}
