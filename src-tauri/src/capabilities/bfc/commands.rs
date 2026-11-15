//! BFC integration commands. BrowserOS reads only the user's rendered Bilibili
//! favorites page and forwards a completed snapshot to the loopback BFC service.
use super::runtime::{
    BfcImportPayload, BfcImportResult, BfcReadbackPayload, BfcReadbackStatus, BfcReadbackTicket,
    BfcRuntime,
};
use crate::bridge::AppState;
use crate::shared::invocation::check_invocation_source;
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, io, net::SocketAddr, path::PathBuf, time::Duration};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
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

const MAX_BFC_HTTP_RESPONSE_BYTES: usize = 1024 * 1024;

fn bfc_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 4198))
}

/// A single authenticated loopback connection. All preflight, challenge,
/// session and snapshot requests use this exact TCP socket. If it closes,
/// requests fail closed; we never reconnect to a new process after proof.
struct BfcHttpConnection {
    reader: BufReader<TcpStream>,
    origin: String,
    host: String,
}

impl BfcHttpConnection {
    async fn connect(addr: SocketAddr) -> Result<Self, &'static str> {
        if !addr.ip().is_loopback() {
            return Err("BFC_LOCAL_ORIGIN_NOT_LOOPBACK");
        }
        let stream = tokio::time::timeout(
            Duration::from_secs(1), TcpStream::connect(addr)
        ).await.map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?
            .map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?;
        stream.set_nodelay(true).map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?;
        let peer = stream.peer_addr().map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?;
        if !peer.ip().is_loopback() || peer.port() != addr.port() {
            return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
        }
        let host = addr.to_string();
        Ok(Self {
            reader: BufReader::new(stream),
            origin: format!("http://{host}"),
            host,
        })
    }

    async fn read_line_limited(
        reader: &mut BufReader<TcpStream>,
        max_bytes: usize,
    ) -> Result<Vec<u8>, &'static str> {
        let mut line = Vec::new();
        loop {
            let (chunk, complete) = {
                let available = reader.fill_buf().await
                    .map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?;
                if available.is_empty() {
                    return Err("BFC_LOCAL_SERVICE_UNAVAILABLE");
                }
                let count = available.iter().position(|byte| *byte == b'\n')
                    .map(|index| index + 1).unwrap_or(available.len());
                if line.len().saturating_add(count) > max_bytes {
                    return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
                }
                (available[..count].to_vec(), available[count - 1] == b'\n')
            };
            let count = chunk.len();
            reader.consume(count);
            line.extend_from_slice(&chunk);
            if complete { return Ok(line); }
        }
    }

    async fn request_json(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Result<(u16, Value), &'static str> {
        if !matches!(method, "GET" | "POST")
            || !path.starts_with('/')
            || path.contains('\r') || path.contains('\n')
            || headers.iter().any(|(name, value)|
                name.is_empty() || name.contains(['\r', '\n', ':'])
                || value.contains(['\r', '\n']))
        {
            return Err("BFC_HTTP_REQUEST_INVALID");
        }
        let body = body.unwrap_or_default();
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\nContent-Length: {}\r\n",
            self.host, body.len(),
        );
        for (name, value) in headers {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");

        // Do not let reqwest/Hyper silently establish a new connection after
        // the identity challenge. The payload is written only on this socket.
        let stream = self.reader.get_mut();
        tokio::time::timeout(Duration::from_secs(5), async {
            stream.write_all(head.as_bytes()).await?;
            if !body.is_empty() { stream.write_all(body).await?; }
            stream.flush().await
        }).await.map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?
            .map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?;

        let status_line = Self::read_line_limited(&mut self.reader, 8192).await?;
        let status_line = std::str::from_utf8(&status_line)
            .map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
        let mut status_parts = status_line.split_whitespace();
        let protocol = status_parts.next().unwrap_or("");
        let status = status_parts.next().and_then(|value| value.parse::<u16>().ok())
            .ok_or("BFC_LOCAL_SERVICE_UNTRUSTED")?;
        if !protocol.starts_with("HTTP/1.") {
            return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
        }

        let mut content_length = None;
        let mut chunked = false;
        let mut connection_close = false;
        let mut content_type_json = false;
        let mut header_bytes = status_line.len();
        loop {
            let line = Self::read_line_limited(&mut self.reader, 8192).await?;
            header_bytes = header_bytes.saturating_add(line.len());
            if header_bytes > 64 * 1024 {
                return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
            }
            if line == b"\r\n" { break; }
            let text = std::str::from_utf8(&line)
                .map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
            let (name, value) = text.trim_end_matches(['\r', '\n']).split_once(':')
                .ok_or("BFC_LOCAL_SERVICE_UNTRUSTED")?;
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => {
                    let parsed = value.trim().parse::<usize>()
                        .map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
                    if content_length.replace(parsed).is_some() {
                        return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
                    }
                }
                "transfer-encoding" => chunked = true,
                "connection" if value.trim().eq_ignore_ascii_case("close") => connection_close = true,
                "content-type" if value.trim().to_ascii_lowercase().starts_with("application/json") => content_type_json = true,
                _ => {}
            }
        }
        if chunked || connection_close || !content_type_json {
            return Err("BFC_LOCAL_CONNECTION_NOT_PERSISTENT");
        }
        let length = content_length.ok_or("BFC_LOCAL_SERVICE_UNTRUSTED")?;
        if length == 0 || length > MAX_BFC_HTTP_RESPONSE_BYTES {
            return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
        }
        let mut bytes = vec![0u8; length];
        tokio::time::timeout(Duration::from_secs(5), self.reader.read_exact(&mut bytes))
            .await.map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?
            .map_err(|_| "BFC_LOCAL_SERVICE_UNAVAILABLE")?;
        let value = serde_json::from_slice(&bytes)
            .map_err(|_| "BFC_LOCAL_SERVICE_UNTRUSTED")?;
        Ok((status, value))
    }
}

async fn verify_local_bfc_service(
    connection: &mut BfcHttpConnection,
    identity: &BfcServiceIdentity,
) -> Result<(Value, Value), &'static str> {
    let (health_status, health) = connection
        .request_json("GET", BFC_HEALTH_PATH, &[], None).await?;
    if health_status != 200
        || health.get("ok").and_then(Value::as_bool) != Some(true)
        || health.get("localOnly").and_then(Value::as_bool) != Some(true)
        || health.get("bilibiliNetworkRequests").and_then(Value::as_u64) != Some(0)
        || health.get("protocolVersion").and_then(Value::as_u64)
            != Some(identity.protocol_version as u64)
        || health.get("serviceId").and_then(Value::as_str) != Some(identity.service_id.as_str())
    {
        return Err("BFC_LOCAL_SERVICE_UNTRUSTED");
    }

    let (cap_status, capabilities) = connection
        .request_json("GET", BFC_CAPABILITIES_PATH, &[], None).await?;
    if cap_status != 200
        || capabilities.get("version").and_then(Value::as_u64)
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

    // Prove service identity on this same TCP connection. No token is sent
    // until the challenge response has been verified locally.
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let challenge_headers = [("X-BrowserOS-Challenge", nonce.as_str())];
    let (identity_status, identity_response) = connection
        .request_json("GET", BFC_IDENTITY_PATH, &challenge_headers, None).await?;
    let proof = identity_response.get("proof").and_then(Value::as_str)
        .and_then(decode_hex).ok_or("BFC_LOCAL_SERVICE_UNTRUSTED")?;
    if identity_status != 200
        || identity_response.get("ok").and_then(Value::as_bool) != Some(true)
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
    let mut connection = match BfcHttpConnection::connect(bfc_addr()).await {
        Ok(connection) => connection,
        Err(code) => return Ok(BfcServiceStatus {
            online: false,
            local_only: None,
            bilibili_network_requests: None,
            read_snapshot_only: None,
            complete_snapshot: None,
            message_code: Some(code.into()),
        }),
    };
    match verify_local_bfc_service(&mut connection, &identity).await {
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

async fn import_to_local_bfc(
    payload: &BfcImportPayload,
) -> Result<BfcImportResult, ImportFailure> {
    let identity = load_bfc_service_identity()
        .map_err(ImportFailure::NotSent)?;
    let connection = BfcHttpConnection::connect(bfc_addr()).await
        .map_err(ImportFailure::NotSent)?;
    import_to_local_bfc_with_identity(connection, payload, &identity).await
}

async fn import_to_local_bfc_with_identity(
    mut connection: BfcHttpConnection,
    payload: &BfcImportPayload,
    identity: &BfcServiceIdentity,
) -> Result<BfcImportResult, ImportFailure> {
    // Preflight, HMAC challenge, authenticated session and import all remain
    // on one TCP socket. If the server exits or the socket closes, we fail
    // rather than reconnecting to a process that may have rebound port 4198.
    verify_local_bfc_service(&mut connection, identity).await
        .map_err(ImportFailure::NotSent)?;

    let origin = connection.origin.clone();
    let session_headers = [
        ("Origin", origin.as_str()),
        ("X-BrowserOS-Service-Token", identity.token.as_str()),
    ];
    let (session_status, session) = connection
        .request_json("GET", BFC_BROWSEROS_SESSION_PATH, &session_headers, None)
        .await.map_err(ImportFailure::NotSent)?;
    if session_status != 200 {
        return Err(ImportFailure::NotSent("BFC_LOCAL_SESSION_UNAVAILABLE"));
    }
    let csrf = session.get("token").and_then(Value::as_str)
        .filter(|value| decode_hex(value).is_some_and(|bytes| bytes.len() == 32))
        .ok_or(ImportFailure::NotSent("BFC_LOCAL_SESSION_INVALID"))?;
    if session.get("serviceId").and_then(Value::as_str) != Some(identity.service_id.as_str())
        || session.get("protocolVersion").and_then(Value::as_u64)
            != Some(identity.protocol_version as u64)
    {
        return Err(ImportFailure::NotSent("BFC_LOCAL_SERVICE_UNTRUSTED"));
    }

    // Serialize only after the server proved its identity on this socket.
    let body = serde_json::to_vec(payload)
        .map_err(|_| ImportFailure::NotSent("BFC_LOCAL_PAYLOAD_INVALID"))?;
    let import_headers = [
        ("Origin", origin.as_str()),
        ("Content-Type", "application/json"),
        ("X-BrowserOS-Service-Token", identity.token.as_str()),
        ("X-Local-CSRF", csrf),
    ];
    let (status, response) = connection
        .request_json("POST", BFC_IMPORT_PATH, &import_headers, Some(&body))
        .await.map_err(|_| ImportFailure::Unknown)?;
    if (400..500).contains(&status) {
        return Err(ImportFailure::Rejected("BFC_LOCAL_IMPORT_REJECTED"));
    }
    if status >= 500 || !(200..300).contains(&status) {
        return Err(ImportFailure::Unknown);
    }
    parse_import_receipt(payload, &response)
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
        let request_headers = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen = Arc::clone(&requests);
        let seen_headers = Arc::clone(&request_headers);
        let expected_service = service_id.clone();
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().expect("fake service accepts request");
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut header = String::new();
                let mut captured_headers = String::new();
                loop {
                    header.clear();
                    if reader.read_line(&mut header).unwrap() == 0 || header == "\r\n" {
                        break;
                    }
                    captured_headers.push_str(&header);
                }
                seen_headers.lock().unwrap().push(captured_headers);
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
        assert!(request_headers.lock().unwrap().iter().all(|headers|
            !headers.lines().any(|line| line.to_ascii_lowercase().starts_with("x-browseros-service-token:"))
        ), "service token must not be disclosed before identity proof");
        assert!(!observed.iter().any(|request| request.starts_with("POST /api/readback/import ")),
            "snapshot payload must never be sent to a port occupant that cannot prove service identity");
    }

    #[test]
    fn local_bfc_endpoint_is_fixed_to_loopback() {
        assert_eq!(BFC_ORIGIN, "http://127.0.0.1:4198");
        assert_eq!(BFC_IMPORT_PATH, "/api/readback/import");
        assert!(!BFC_IMPORT_PATH.contains("bilibili.com"));
        assert_eq!(BFC_BROWSEROS_SESSION_PATH, "/api/browseros/session");
        assert_eq!(BFC_IDENTITY_PATH, "/api/identity");
    }
}
