//! BFC integration commands. BrowserOS reads only the user's rendered Bilibili
//! favorites page and forwards a completed snapshot to the loopback BFC service.
use super::runtime::{
    BfcImportPayload, BfcImportResult, BfcReadbackPayload, BfcReadbackStatus, BfcReadbackTicket,
    BfcRuntime,
};
use crate::bridge::AppState;
use crate::shared::invocation::check_invocation_source;
use reqwest::header::{HeaderName, HeaderValue, ORIGIN};
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, path::PathBuf, time::Duration};
use tauri::{AppHandle, Manager, Webview};
use url::Url;

const BFC_ORIGIN: &str = "http://127.0.0.1:4198";
const BFC_BROWSEROS_SESSION_PATH: &str = "/api/browseros/session";
const BFC_IDENTITY_PATH: &str = "/api/identity";
const BFC_HEALTH_PATH: &str = "/api/health";
const BFC_CAPABILITIES_PATH: &str = "/api/capabilities";
const BFC_IMPORT_PATH: &str = "/api/readback/import";
const BFC_IDENTITY_FILE: &str = ".browseros-service-identity.json";
const BFC_PROTOCOL_VERSION: u32 = 1;
const MAX_REMOTE_READBACK_BYTES: usize = 4_500_000;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BfcServiceIdentity {
    protocol_version: u32,
    service_id: String,
    token: String,
}

fn bfc_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("BILIBILI_FAVORITES_DATA_DIR")
        .or_else(|| std::env::var_os("BROWSEROS_BFC_DATA_DIR"))
        .filter(|path| !path.is_empty())
    {
        return PathBuf::from(path);
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        return base.join("BrowserOS").join("Bilibili-Favorites-Review");
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        home.join("Documents").join("Bilibili-Favorites-Review")
    }
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    value.as_bytes().chunks_exact(2).map(|pair| {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        Some(((high << 4) | low) as u8)
    }).collect()
}

fn load_bfc_service_identity() -> Result<BfcServiceIdentity, &'static str> {
    let directory = bfc_data_dir();
    let path = directory.join(BFC_IDENTITY_FILE);
    let bytes = crate::capabilities::workspace::safe_paths::read_regular(
        &path, &[directory], 4096,
    ).map_err(|_| "BFC_LOCAL_IDENTITY_UNAVAILABLE")?;
    let identity: BfcServiceIdentity =
        serde_json::from_slice(&bytes).map_err(|_| "BFC_LOCAL_IDENTITY_INVALID")?;
    if identity.protocol_version != BFC_PROTOCOL_VERSION
        || uuid::Uuid::parse_str(&identity.service_id).is_err()
        || decode_hex(&identity.token).is_none_or(|token| token.len() != 32)
    {
        return Err("BFC_LOCAL_IDENTITY_INVALID");
    }
    Ok(identity)
}

async fn verify_local_bfc_service(
    client: &reqwest::Client,
    origin: &str,
    identity: &BfcServiceIdentity,
) -> Result<(Value, Value), &'static str> {
    let health: Value = client.get(format!("{origin}{BFC_HEALTH_PATH}"))
        .send().await.map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?
        .error_for_status().map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?
        .json().await.map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
    if health.get("ok").and_then(Value::as_bool) != Some(true)
        || health.get("localOnly").and_then(Value::as_bool) != Some(true)
        || health.get("bilibiliNetworkRequests").and_then(Value::as_u64) != Some(0)
        || health.get("protocolVersion").and_then(Value::as_u64)
            != Some(identity.protocol_version as u64)
        || health.get("serviceId").and_then(Value::as_str) != Some(identity.service_id.as_str())
    {
        return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
    }

    let capabilities: Value = client.get(format!("{origin}{BFC_CAPABILITIES_PATH}"))
        .send().await.map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?
        .error_for_status().map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?
        .json().await.map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
    if capabilities.get("version").and_then(Value::as_u64)
            != Some(identity.protocol_version as u64)
        || capabilities.get("protocolVersion").and_then(Value::as_u64)
            != Some(identity.protocol_version as u64)
        || capabilities.get("serviceId").and_then(Value::as_str)
            != Some(identity.service_id.as_str())
        || capabilities.pointer("/bilibili/readSnapshotOnly").and_then(Value::as_bool) != Some(true)
        || capabilities.pointer("/bilibili/accountWrites").and_then(Value::as_bool) != Some(false)
    {
        return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
    }

    // The token is read from the per-user identity file protected by the
    // service's Windows ACL / Unix 0600 mode. A port occupant can spoof public
    // health/capability JSON, but cannot produce this challenge proof.
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let identity_response: Value = client.get(format!("{origin}{BFC_IDENTITY_PATH}"))
        .header("x-browseros-service-token", &identity.token)
        .header("x-browseros-challenge", &nonce)
        .send().await.map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?
        .error_for_status().map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?
        .json().await.map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
    let proof = identity_response.get("proof").and_then(Value::as_str)
        .and_then(decode_hex).ok_or("BFC_LOCAL_SERVICE_UNTRUSTED")?;
    if identity_response.get("ok").and_then(Value::as_bool) != Some(true)
        || identity_response.get("serviceId").and_then(Value::as_str)
            != Some(identity.service_id.as_str())
        || identity_response.get("protocolVersion").and_then(Value::as_u64)
            != Some(identity.protocol_version as u64)
    {
        return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
    }
    let token = decode_hex(&identity.token).ok_or("BFC_LOCAL_IDENTITY_INVALID")?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, &token);
    let transcript = format!(
        "browseros-bfc-identity|{nonce}|{}|{}",
        identity.service_id, identity.protocol_version
    );
    hmac::verify(&key, transcript.as_bytes(), &proof)
        .map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
    Ok((health, capabilities))
}

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
    let identity = match load_bfc_service_identity() {
        Ok(identity) => identity,
        Err(code) => return Ok(BfcServiceStatus {
            online: false,
            local_only: None,
            bilibili_network_requests: None,
            read_snapshot_only: None,
            complete_snapshot: None,
            message_code: Some(code.into()),
        }),
    };
    let client = match client() {
        Ok(client) => client,
        Err(_) => return Ok(BfcServiceStatus {
            online: false,
            local_only: None,
            bilibili_network_requests: None,
            read_snapshot_only: None,
            complete_snapshot: None,
            message_code: Some("BFC_HTTP_CLIENT_UNAVAILABLE".into()),
        }),
    };
    match verify_local_bfc_service(&client, BFC_ORIGIN, &identity).await {
        Ok((health, capabilities)) => Ok(BfcServiceStatus {
            online: true,
            local_only: health.get("localOnly").and_then(Value::as_bool),
            bilibili_network_requests: health.get("bilibiliNetworkRequests").and_then(Value::as_u64),
            read_snapshot_only: capabilities.pointer("/bilibili/readSnapshotOnly").and_then(Value::as_bool),
            complete_snapshot: capabilities.pointer("/bilibili/completeSnapshot").and_then(Value::as_bool),
            message_code: None,
        }),
        Err(code) => Ok(BfcServiceStatus {
            online: false,
            local_only: None,
            bilibili_network_requests: None,
            read_snapshot_only: None,
            complete_snapshot: None,
            message_code: Some(code.into()),
        }),
    }
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

async fn import_to_local_bfc(
    payload: &BfcImportPayload,
) -> Result<BfcImportResult, ImportFailure> {
    let identity = load_bfc_service_identity()
        .map_err(ImportFailure::NotSent)?;
    let client = client().map_err(|_| ImportFailure::NotSent("BFC_HTTP_CLIENT_UNAVAILABLE"))?;
    import_to_local_bfc_with_identity(&client, payload, &identity, BFC_ORIGIN).await
}

async fn import_to_local_bfc_with_identity(
    client: &reqwest::Client,
    payload: &BfcImportPayload,
    identity: &BfcServiceIdentity,
    origin: &str,
) -> Result<BfcImportResult, ImportFailure> {
    // Complete identity, protocol and capability checks before any snapshot
    // body is serialized or transmitted to the loopback port.
    verify_local_bfc_service(client, origin, identity)
        .await.map_err(ImportFailure::NotSent)?;
    let origin_header = HeaderValue::from_str(origin)
        .map_err(|_| ImportFailure::NotSent("BFC_HTTP_ORIGIN_INVALID"))?;
    let session_response = client
        .get(format!("{origin}{BFC_BROWSEROS_SESSION_PATH}"))
        .header(ORIGIN, origin_header.clone())
        .header("x-browseros-service-token", &identity.token)
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
    let csrf = session.get("token").and_then(Value::as_str)
        .filter(|value| decode_hex(value).is_some_and(|bytes| bytes.len() == 32))
        .ok_or(ImportFailure::NotSent("BFC_LOCAL_SESSION_INVALID"))?;
    if session.get("serviceId").and_then(Value::as_str) != Some(identity.service_id.as_str())
        || session.get("protocolVersion").and_then(Value::as_u64)
            != Some(identity.protocol_version as u64)
    {
        return Err(ImportFailure::NotSent("BFC_LOCAL_SERVICE_UNTRUSTED"));
    }

    let response = client
        .post(format!("{origin}{BFC_IMPORT_PATH}"))
        .header(ORIGIN, origin_header)
        .header("x-browseros-service-token", &identity.token)
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
    parse_import_receipt(payload, &body)
}

fn parse_import_receipt(
    payload: &BfcImportPayload,
    body: &Value,
) -> Result<BfcImportResult, ImportFailure> {
    if body.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(ImportFailure::Unknown);
    }
    let returned_scan = body.get("scanId").and_then(Value::as_str);
    let returned_pages = body.get("pages").and_then(Value::as_u64);
    let returned_cards = body.get("cards").and_then(Value::as_u64);
    let captured_pages = payload.pages.len() as u32;
    let captured_cards = payload
        .pages
        .values()
        .map(|page| page.items.len() as u32)
        .sum::<u32>();
    if returned_scan != Some(payload.scan_id.as_str())
        || returned_pages != Some(captured_pages as u64)
        || returned_cards != Some(captured_cards as u64)
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
        pages: captured_pages,
        cards: captured_cards,
        unique,
        gap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_receipt_matches_captured_counts_not_reported_totals() {
        use super::super::runtime::{BfcImportPage, BfcItem};
        let payload = BfcImportPayload {
            source: "authorized_browser_readback",
            scan_id: "fixture".into(),
            reported_total: 4875,
            expected_pages: 136,
            traversal_finished: false,
            pagination_stable: false,
            folder_id: "99".into(),
            folder_name: "测试收藏夹".into(),
            pages: [(
                "1".into(),
                BfcImportPage {
                    items: vec![BfcItem {
                        bvid: "BV1234567890".into(),
                        title: "fixture".into(),
                    }],
                },
            )]
            .into(),
        };
        let serialized = serde_json::to_value(&payload).unwrap();
        assert_eq!(serialized["folderId"], "99");
        assert_eq!(serialized["folderName"], "测试收藏夹");
        assert!(serialized.get("mid").is_none());
        let body = serde_json::json!({"ok":true,"scanId":"fixture","pages":1,"cards":1});
        let receipt = parse_import_receipt(&payload, &body).ok().unwrap();
        assert_eq!(receipt.pages, 1);
        assert_eq!(receipt.cards, 1);
        assert!(parse_import_receipt(
            &payload,
            &serde_json::json!({"ok":true,"scanId":"fixture","pages":136,"cards":4875})
        )
        .is_err());
        assert!(!payload.traversal_finished);
    }

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

    #[tokio::test]
    async fn fake_loopback_port_occupant_cannot_receive_snapshot_before_identity_proof() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;
        use std::sync::{Arc, Mutex};
        use std::thread;
        use super::super::runtime::{BfcImportPage, BfcItem};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let service_id = uuid::Uuid::new_v4().to_string();
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen = Arc::clone(&requests);
        let expected_service = service_id.clone();
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().expect("fake service accepts request");
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut header = String::new();
                loop {
                    header.clear();
                    if reader.read_line(&mut header).unwrap() == 0 || header == "\r\n" {
                        break;
                    }
                }
                let path = request_line.split_whitespace().nth(1).unwrap_or("");
                seen.lock().unwrap().push(request_line.trim().to_string());
                let body = match path {
                    "/api/health" => serde_json::json!({
                        "ok":true,"localOnly":true,"bilibiliNetworkRequests":0,
                        "protocolVersion":1,"serviceId":expected_service
                    }),
                    "/api/capabilities" => serde_json::json!({
                        "version":1,"protocolVersion":1,"serviceId":expected_service,
                        "bilibili":{"readSnapshotOnly":true,"accountWrites":false}
                    }),
                    "/api/identity" => serde_json::json!({
                        "ok":true,"serviceId":expected_service,"protocolVersion":1,
                        "proof":"0000000000000000000000000000000000000000000000000000000000000000"
                    }),
                    _ => panic!("unexpected request reached fake service: {request_line}"),
                }.to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                ).unwrap();
            }
        });

        let identity = BfcServiceIdentity {
            protocol_version: 1,
            service_id,
            token: "a".repeat(64),
        };
        let payload = BfcImportPayload {
            source: "authorized_browser_readback",
            scan_id: "fake-port-fixture".into(),
            reported_total: 1,
            expected_pages: 1,
            traversal_finished: true,
            pagination_stable: true,
            folder_id: "9".into(),
            folder_name: "CI fake port".into(),
            pages: [(
                "1".into(),
                BfcImportPage {
                    items: vec![BfcItem {
                        bvid: "BV1234567890".into(),
                        title: "must-not-leak".into(),
                    }],
                },
            )].into(),
        };
        let client = client().unwrap();
        let result = import_to_local_bfc_with_identity(&client, &payload, &identity, &origin).await;
        assert!(matches!(
            result,
            Err(ImportFailure::NotSent("BFC_LOCAL_SERVICE_UNTRUSTED"))
        ));
        server.join().unwrap();
        let observed = requests.lock().unwrap().clone();
        assert_eq!(observed.len(), 3);
        assert!(observed.iter().all(|request| request.starts_with("GET ")));
        assert!(observed.iter().any(|request| request.starts_with("GET /api/identity ")));
        assert!(!observed.iter().any(|request| request.starts_with("POST /api/readback/import ")),
            "snapshot payload must never be sent to a port occupant that cannot prove service identity");
    }

    #[test]
    fn local_bfc_endpoint_is_fixed_to_loopback() {
        assert_eq!(BFC_ORIGIN, "http://127.0.0.1:4198");
        assert!(BFC_IMPORT_URL.starts_with(BFC_ORIGIN));
        assert!(!BFC_IMPORT_URL.contains("bilibili.com"));
    }
}
