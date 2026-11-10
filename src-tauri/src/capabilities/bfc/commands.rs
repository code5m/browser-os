//! BFC integration commands. BrowserOS reads only the user's rendered Bilibili
//! favorites page and forwards a completed snapshot to the loopback BFC service.
use super::runtime::{
    BfcImportPayload, BfcImportResult, BfcReadbackPayload, BfcReadbackStatus, BfcReadbackTicket,
    BfcRuntime,
};
use crate::bridge::AppState;
use crate::shared::invocation::check_invocation_source;
use reqwest::header::{HeaderName, HeaderValue, ORIGIN};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashSet, time::Duration};
use tauri::{AppHandle, Manager, Webview};
use url::Url;

const BFC_ORIGIN: &str = "http://127.0.0.1:4198";
const BFC_SESSION_URL: &str = "http://127.0.0.1:4198/api/session";
const BFC_HEALTH_URL: &str = "http://127.0.0.1:4198/api/health";
const BFC_CAPABILITIES_URL: &str = "http://127.0.0.1:4198/api/capabilities";
const BFC_IMPORT_URL: &str = "http://127.0.0.1:4198/api/readback/import";
const MAX_REMOTE_READBACK_BYTES: usize = 4_500_000;

#[derive(Clone, Debug, Serialize)]
pub struct BfcServiceStatus {
    pub online: bool,
    pub local_only: Option<bool>,
    pub bilibili_network_requests: Option<u64>,
    pub read_snapshot_only: Option<bool>,
    pub complete_snapshot: Option<bool>,
    pub message_code: Option<String>,
}

enum ImportFailure {
    NotSent(&'static str),
    Rejected(&'static str),
    Unknown,
}

fn main_only(app: &AppHandle, webview: &Webview, scope: &str) -> Result<(), String> {
    check_invocation_source(webview, scope, None, app)
}

// Validate against the native Owner, not a stale frontend tab metadata list.
// Child WebViews belong to TabManagerState, not app.get_webview(label).
fn validate_bfc_tab_binding(
    tab_id: &str,
    active_id: Option<&str>,
    known: bool,
) -> Result<(), String> {
    if active_id.is_none() {
        return Err("BFC_TAB_NOT_ACTIVE".into());
    }
    if !tab_id.starts_with("tab-")
        || tab_id[4..].is_empty()
        || !tab_id[4..].bytes().all(|b| b.is_ascii_digit())
        || active_id != Some(tab_id)
        || !known
    {
        return Err("BFC_TAB_ID_MISMATCH".into());
    }
    Ok(())
}

fn verify_native_bfc_tab(app: &AppHandle, tab_id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let active = state.active_tab.lock().unwrap().clone();
    let url = state
        .tabs
        .lock()
        .unwrap()
        .get(tab_id)
        .map(|t| t.url.clone());
    validate_bfc_tab_binding(tab_id, active.as_deref(), url.is_some())?;
    // These are plugin-owned native child WebViews, not app.get_webview entries.
    let owned = app
        .state::<tauri_plugin_browser_tabs::TabManagerState>()
        .get_tab_ids()
        .iter()
        .any(|id| id == tab_id);
    if !owned {
        return Err("BFC_TAB_ID_MISMATCH".into());
    }
    let url = Url::parse(url.as_deref().unwrap_or_default())
        .map_err(|_| "BFC_FAVORITES_PAGE_REQUIRED")?;
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !matches!(
            url.host_str(),
            Some("www.bilibili.com" | "space.bilibili.com")
        )
    {
        return Err("BFC_FAVORITES_PAGE_REQUIRED".into());
    }
    Ok(())
}

#[tauri::command]
pub fn bfc_readback_begin(
    app: AppHandle,
    webview: Webview,
    tab_id: String,
) -> Result<BfcReadbackTicket, String> {
    main_only(&app, &webview, "bfc_readback_begin")?;
    verify_native_bfc_tab(&app, &tab_id)?;
    app.state::<BfcRuntime>().begin(&tab_id)
}

/// This is the sole remote-page command for BFC. It is available only through
/// the dedicated HTTPS Bilibili tab-only bfc-readback ACL, and the runtime additionally binds the
/// one-use ticket to a tab-N label, HTTPS Bilibili host, account MID and TTL.
#[tauri::command]
pub fn bfc_readback_submit(
    app: AppHandle,
    webview: Webview,
    ticket: String,
    payload: BfcReadbackPayload,
) -> Result<(), String> {
    let label = webview.label().to_string();
    if !label.starts_with("tab-") {
        return Err("BFC_READBACK_SOURCE_DENIED".into());
    }
    if ticket.len() != 36
        || ticket
            .bytes()
            .any(|byte| !(byte.is_ascii_hexdigit() || byte == b'-'))
    {
        return Err("BFC_READBACK_TICKET_INVALID".into());
    }
    let payload_bytes = serde_json::to_vec(&payload).map_err(|_| "BFC_READBACK_PAYLOAD_INVALID")?;
    if payload_bytes.len() > MAX_REMOTE_READBACK_BYTES {
        return Err("BFC_READBACK_PAYLOAD_TOO_LARGE".into());
    }
    let url = webview.url().map_err(|_| "BFC_PAGE_URL_INVALID")?;
    app.state::<BfcRuntime>()
        .submit(&ticket, &label, url.as_str(), payload)
}

#[tauri::command]
pub fn bfc_readback_status(
    app: AppHandle,
    webview: Webview,
    ticket: String,
) -> Result<BfcReadbackStatus, String> {
    main_only(&app, &webview, "bfc_readback_status")?;
    app.state::<BfcRuntime>().status(&ticket)
}

#[tauri::command]
pub fn bfc_readback_cancel(app: AppHandle, webview: Webview, ticket: String) -> Result<(), String> {
    main_only(&app, &webview, "bfc_readback_cancel")?;
    app.state::<BfcRuntime>().cancel(&ticket)
}

#[tauri::command]
pub async fn bfc_service_status(
    app: AppHandle,
    webview: Webview,
) -> Result<BfcServiceStatus, String> {
    main_only(&app, &webview, "bfc_service_status")?;
    let client = match client() {
        Ok(client) => client,
        Err(_) => {
            return Ok(BfcServiceStatus {
                online: false,
                local_only: None,
                bilibili_network_requests: None,
                read_snapshot_only: None,
                complete_snapshot: None,
                message_code: Some("BFC_LOCAL_SERVICE_UNAVAILABLE".into()),
            });
        }
    };
    let health = match client.get(BFC_HEALTH_URL).send().await {
        Ok(response) if response.status().is_success() => response.json::<Value>().await.ok(),
        _ => None,
    };
    let Some(health) = health else {
        return Ok(BfcServiceStatus {
            online: false,
            local_only: None,
            bilibili_network_requests: None,
            read_snapshot_only: None,
            complete_snapshot: None,
            message_code: Some("BFC_LOCAL_SERVICE_UNAVAILABLE".into()),
        });
    };
    let capabilities = match client.get(BFC_CAPABILITIES_URL).send().await {
        Ok(response) if response.status().is_success() => response.json::<Value>().await.ok(),
        _ => None,
    };
    Ok(BfcServiceStatus {
        online: health.get("ok").and_then(Value::as_bool).unwrap_or(true),
        local_only: health.get("localOnly").and_then(Value::as_bool),
        bilibili_network_requests: health
            .get("bilibiliNetworkRequests")
            .and_then(Value::as_u64),
        read_snapshot_only: capabilities
            .as_ref()
            .and_then(|value| value.get("bilibili"))
            .and_then(|value| value.get("readSnapshotOnly"))
            .and_then(Value::as_bool),
        complete_snapshot: capabilities
            .as_ref()
            .and_then(|value| value.get("bilibili"))
            .and_then(|value| value.get("completeSnapshot"))
            .and_then(Value::as_bool),
        message_code: None,
    })
}

#[tauri::command]
pub async fn bfc_readback_finish(
    app: AppHandle,
    webview: Webview,
    ticket: String,
) -> Result<BfcImportResult, String> {
    main_only(&app, &webview, "bfc_readback_finish")?;
    let runtime = app.state::<BfcRuntime>();
    let (_scan_id, payload) = runtime.prepare_import(&ticket)?;
    match import_to_local_bfc(&payload).await {
        Ok(result) => {
            runtime.mark_imported(&ticket, result.clone())?;
            Ok(result)
        }
        Err(ImportFailure::NotSent(code)) => {
            runtime.reset_ready(&ticket);
            Err(code.into())
        }
        Err(ImportFailure::Rejected(code)) => {
            runtime.mark_failed(&ticket, code);
            Err(code.into())
        }
        Err(ImportFailure::Unknown) => {
            runtime.mark_outcome_unknown(&ticket);
            Err("BFC_IMPORT_OUTCOME_UNKNOWN".into())
        }
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(5))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "BFC_HTTP_CLIENT_UNAVAILABLE".into())
}

async fn import_to_local_bfc(payload: &BfcImportPayload) -> Result<BfcImportResult, ImportFailure> {
    let client = client().map_err(|_| ImportFailure::NotSent("BFC_HTTP_CLIENT_UNAVAILABLE"))?;
    let origin = HeaderValue::from_static(BFC_ORIGIN);
    let session_response = client
        .get(BFC_SESSION_URL)
        .header(ORIGIN, origin.clone())
        .send()
        .await
        .map_err(|_| ImportFailure::NotSent("BFC_LOCAL_SERVICE_UNAVAILABLE"))?;
    if !session_response.status().is_success() {
        return Err(ImportFailure::NotSent("BFC_LOCAL_SESSION_UNAVAILABLE"));
    }
    let session: Value = session_response
        .json()
        .await
        .map_err(|_| ImportFailure::NotSent("BFC_LOCAL_SESSION_INVALID"))?;
    let csrf = session
        .get("token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(ImportFailure::NotSent("BFC_LOCAL_SESSION_INVALID"))?;

    let response = client
        .post(BFC_IMPORT_URL)
        .header(ORIGIN, origin)
        .header(HeaderName::from_static("x-local-csrf"), csrf)
        .json(payload)
        .send()
        .await
        .map_err(|_| ImportFailure::Unknown)?;
    let status = response.status();
    if status.is_client_error() {
        return Err(ImportFailure::Rejected("BFC_LOCAL_IMPORT_REJECTED"));
    }
    if status.is_server_error() || !status.is_success() {
        return Err(ImportFailure::Unknown);
    }
    let body: Value = response.json().await.map_err(|_| ImportFailure::Unknown)?;
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(ImportFailure::Unknown);
    }
    let returned_scan = body.get("scanId").and_then(Value::as_str);
    let returned_pages = body.get("pages").and_then(Value::as_u64);
    let returned_cards = body.get("cards").and_then(Value::as_u64);
    if returned_scan != Some(payload.scan_id.as_str())
        || returned_pages != Some(payload.expected_pages as u64)
        || returned_cards != Some(payload.reported_total as u64)
    {
        return Err(ImportFailure::Unknown);
    }

    let unique = payload
        .pages
        .values()
        .flat_map(|page| page.items.iter().map(|item| item.bvid.as_str()))
        .collect::<HashSet<_>>()
        .len() as u32;
    let gap = body
        .get("gap")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    Ok(BfcImportResult {
        scan_id: payload.scan_id.clone(),
        pages: payload.expected_pages,
        cards: payload.reported_total,
        unique,
        gap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_native_tab_must_match_even_when_other_bilibili_tab_is_logged_in() {
        assert_eq!(
            validate_bfc_tab_binding("tab-9", Some("tab-8"), true),
            Err("BFC_TAB_ID_MISMATCH".into())
        );
        assert_eq!(
            validate_bfc_tab_binding("tab-9", Some("tab-9"), false),
            Err("BFC_TAB_ID_MISMATCH".into())
        );
        assert_eq!(
            validate_bfc_tab_binding("tab-9", None, true),
            Err("BFC_TAB_NOT_ACTIVE".into())
        );
        assert!(validate_bfc_tab_binding("tab-9", Some("tab-9"), true).is_ok());
    }

    #[test]
    fn local_bfc_endpoint_is_fixed_to_loopback() {
        assert_eq!(BFC_ORIGIN, "http://127.0.0.1:4198");
        assert!(BFC_IMPORT_URL.starts_with(BFC_ORIGIN));
        assert!(!BFC_IMPORT_URL.contains("bilibili.com"));
    }
}
