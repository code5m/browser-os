//! Scoped, in-memory owner for user-authorized Bilibili DOM readback.
//! It never reads cookies, makes Bilibili network requests, or writes user data.
//! The main window alone can finalize a complete snapshot to the local BFC service.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Mutex,
    time::{Duration, Instant},
};
use url::Url;
use uuid::Uuid;

const SESSION_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_SESSIONS: usize = 8;
const MAX_PAGES: u32 = 300;
const MAX_ITEMS: usize = 5_000;
const MAX_TITLE_BYTES: usize = 1_000;
const MAX_IMPORT_BYTES: usize = 4_500_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BfcLoginStatus {
    LoggedIn,
    LoggedOut,
    AccessLimited,
    Unknown,
}

impl BfcLoginStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::LoggedIn => "logged_in",
            Self::LoggedOut => "logged_out",
            Self::AccessLimited => "access_limited",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BfcItem {
    pub bvid: String,
    pub title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BfcPage {
    pub page: u32,
    pub items: Vec<BfcItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BfcReadbackPayload {
    pub kind: String,
    pub login_status: Option<String>,
    pub mid: Option<String>,
    pub expected_pages: Option<u32>,
    pub reported_total: Option<u32>,
    pub pages_captured: Option<u32>,
    pub items_captured: Option<u32>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub message_code: Option<String>,
    pub pages: Option<Vec<BfcPage>>,
}

impl Default for BfcReadbackPayload {
    fn default() -> Self {
        Self {
            kind: String::new(),
            login_status: None,
            mid: None,
            expected_pages: None,
            reported_total: None,
            pages_captured: None,
            items_captured: None,
            folder_id: None,
            folder_name: None,
            message_code: None,
            pages: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BfcReadbackTicket {
    pub ticket: String,
    pub scan_id: String,
    pub expires_in_seconds: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BfcReadbackStatus {
    pub state: String,
    pub login_status: Option<String>,
    pub mid: Option<String>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub pages_captured: usize,
    pub expected_pages: Option<u32>,
    pub reported_total: Option<u32>,
    pub items_captured: usize,
    pub message_code: Option<String>,
    pub imported: Option<BfcImportResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BfcImportResult {
    pub scan_id: String,
    pub pages: u32,
    pub cards: u32,
    pub unique: u32,
    pub gap: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BfcImportPayload {
    pub source: &'static str,
    #[serde(rename = "scanId")]
    pub scan_id: String,
    #[serde(rename = "reportedTotal")]
    pub reported_total: u32,
    #[serde(rename = "expectedPages")]
    pub expected_pages: u32,
    #[serde(rename = "traversalFinished")]
    pub traversal_finished: bool,
    pub pages: BTreeMap<String, BfcImportPage>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BfcImportPage {
    pub items: Vec<BfcItem>,
}

#[derive(Clone, Debug)]
struct Capture {
    complete: bool,
    expected_pages: u32,
    reported_total: u32,
    folder_id: String,
    folder_name: String,
    pages: BTreeMap<u32, Vec<BfcItem>>,
}

#[derive(Clone, Debug)]
struct Progress {
    expected_pages: u32,
    reported_total: u32,
    pages_captured: u32,
    items_captured: u32,
    folder_id: String,
    folder_name: String,
}

#[derive(Clone, Debug)]
struct Session {
    tab_id: String,
    scan_id: String,
    expires_at: Instant,
    login_status: Option<BfcLoginStatus>,
    mid: Option<String>,
    capture: Option<Capture>,
    progress: Option<Progress>,
    state: String,
    message_code: Option<String>,
    imported: Option<BfcImportResult>,
}

#[derive(Default)]
pub struct BfcRuntime {
    sessions: Mutex<HashMap<String, Session>>,
}

impl BfcRuntime {
    pub fn begin(&self, tab_id: &str) -> Result<BfcReadbackTicket, String> {
        if tab_id.len() > 64 || !tab_id.starts_with("tab-") {
            return Err("BFC_TAB_ID_INVALID".into());
        }
        let now = Instant::now();
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        sessions.retain(|_, session| session.expires_at > now);
        if sessions.len() >= MAX_SESSIONS {
            return Err("BFC_READBACK_SESSION_LIMIT".into());
        }
        let ticket = Uuid::new_v4().to_string();
        let scan_id = Uuid::new_v4().to_string();
        sessions.insert(
            ticket.clone(),
            Session {
                tab_id: tab_id.to_string(),
                scan_id: scan_id.clone(),
                expires_at: now + SESSION_TTL,
                login_status: None,
                mid: None,
                capture: None,
                progress: None,
                state: "awaiting_login_probe".into(),
                message_code: None,
                imported: None,
            },
        );
        Ok(BfcReadbackTicket {
            ticket,
            scan_id,
            expires_in_seconds: SESSION_TTL.as_secs(),
        })
    }

    pub fn submit(
        &self,
        ticket: &str,
        tab_id: &str,
        page_url: &str,
        payload: BfcReadbackPayload,
    ) -> Result<(), String> {
        let url = Url::parse(page_url).map_err(|_| "BFC_PAGE_URL_INVALID")?;
        if url.scheme() != "https"
            || url.port_or_known_default() != Some(443)
            || !url.username().is_empty()
            || url.password().is_some()
            || !matches!(
                url.host_str(),
                Some("www.bilibili.com" | "space.bilibili.com")
            )
        {
            return Err("BFC_PAGE_SOURCE_DENIED".into());
        }
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if sessions
            .get(ticket)
            .is_some_and(|session| session.expires_at <= Instant::now())
        {
            sessions.remove(ticket);
            return Err("BFC_READBACK_TICKET_EXPIRED".into());
        }
        let session = sessions
            .get_mut(ticket)
            .ok_or("BFC_READBACK_TICKET_UNKNOWN")?;
        if session.tab_id != tab_id {
            return Err("BFC_TAB_ID_MISMATCH".into());
        }
        if session.imported.is_some()
            || session.state == "importing"
            || session.state == "outcome_unknown"
        {
            return Err("BFC_READBACK_ALREADY_FINALIZED".into());
        }

        match payload.kind.as_str() {
            "login" => {
                if session.state != "awaiting_login_probe" {
                    return Err("BFC_LOGIN_PROBE_ALREADY_USED".into());
                }
                let status = match payload.login_status.as_deref() {
                    Some("logged_in") => BfcLoginStatus::LoggedIn,
                    Some("logged_out") => BfcLoginStatus::LoggedOut,
                    Some("access_limited") => BfcLoginStatus::AccessLimited,
                    _ => BfcLoginStatus::Unknown,
                };
                let mid = payload.mid.filter(|value| {
                    !value.is_empty()
                        && value.len() <= 20
                        && value.bytes().all(|byte| byte.is_ascii_digit())
                });
                if matches!(status, BfcLoginStatus::LoggedIn) && mid.is_none() {
                    session.login_status = Some(BfcLoginStatus::Unknown);
                    session.mid = None;
                    session.state = "unknown".into();
                    session.message_code = Some("BFC_LOGIN_IDENTITY_NOT_FOUND".into());
                    return Ok(());
                }
                session.state = status.as_str().into();
                session.login_status = Some(status);
                session.mid = mid;
                session.message_code = if matches!(status, BfcLoginStatus::Unknown) {
                    Some("BFC_LOGIN_IDENTITY_NOT_FOUND".into())
                } else if matches!(status, BfcLoginStatus::AccessLimited) {
                    Some("BFC_BILIBILI_ACCESS_LIMITED".into())
                } else {
                    None
                };
                Ok(())
            }
            "abort" => {
                if session.state == "ready" {
                    return Err("BFC_READBACK_ALREADY_READY".into());
                }
                let code = payload
                    .message_code
                    .filter(|value| {
                        !value.is_empty()
                            && value.len() <= 96
                            && value.bytes().all(|byte| {
                                byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
                            })
                    })
                    .unwrap_or_else(|| "BFC_READBACK_ABORTED".into());
                session.state =
                    if code == "BFC_BILIBILI_ACCESS_LIMITED" || code == "BFC_BILIBILI_HTTP_412" {
                        "access_limited".into()
                    } else {
                        "failed".into()
                    };
                session.message_code = Some(code);
                Ok(())
            }
            "favorites_start" => {
                if session.state != "logged_in"
                    || !matches!(session.login_status, Some(BfcLoginStatus::LoggedIn))
                {
                    return Err("BFC_LOGIN_REQUIRED".into());
                }
                let mid = session
                    .mid
                    .as_deref()
                    .ok_or("BFC_LOGIN_IDENTITY_NOT_FOUND")?;
                let mut segments = url.path().split('/').filter(|part| !part.is_empty());
                if url.host_str() != Some("space.bilibili.com")
                    || segments.next() != Some(mid)
                    || segments.next() != Some("favlist")
                {
                    return Err("BFC_FAVORITES_PAGE_REQUIRED".into());
                }
                session.state = "reading_favorites".into();
                Ok(())
            }
            "progress" => {
                if !matches!(session.state.as_str(), "logged_in" | "reading_favorites")
                    || !matches!(session.login_status, Some(BfcLoginStatus::LoggedIn))
                {
                    return Err("BFC_LOGIN_REQUIRED".into());
                }
                let mid = session
                    .mid
                    .as_deref()
                    .ok_or("BFC_LOGIN_IDENTITY_NOT_FOUND")?;
                let mut segments = url.path().split('/').filter(|segment| !segment.is_empty());
                if url.host_str() != Some("space.bilibili.com")
                    || segments.next() != Some(mid)
                    || segments.next() != Some("favlist")
                {
                    return Err("BFC_FAVORITES_PAGE_REQUIRED".into());
                }
                let expected_pages = payload.expected_pages.ok_or("BFC_PAGINATION_UNKNOWN")?;
                let reported_total = payload.reported_total.ok_or("BFC_TOTAL_UNKNOWN")?;
                let pages = payload.pages_captured.ok_or("BFC_PAGINATION_UNKNOWN")?;
                let items = payload.items_captured.ok_or("BFC_PAGINATION_UNKNOWN")?;
                let folder_id = payload.folder_id.unwrap_or_else(|| "unknown".into());
                let folder_name = payload.folder_name.unwrap_or_else(|| "当前收藏夹".into());
                if expected_pages == 0
                    || expected_pages > MAX_PAGES
                    || pages == 0
                    || pages > expected_pages
                    || reported_total as usize > MAX_ITEMS
                    || items > reported_total
                    || folder_id.len() > 64
                    || folder_name.len() > 128
                {
                    return Err("BFC_READBACK_BOUNDS_INVALID".into());
                }
                if let Some(previous) = &session.progress {
                    if previous.expected_pages != expected_pages
                        || previous.reported_total != reported_total
                        || previous.folder_id != folder_id
                        || previous.folder_name != folder_name
                        || pages != previous.pages_captured + 1
                        || items <= previous.items_captured
                    {
                        return Err("BFC_READBACK_PROGRESS_INVALID".into());
                    }
                } else if pages != 1 {
                    return Err("BFC_READBACK_PROGRESS_INVALID".into());
                }
                session.progress = Some(Progress {
                    expected_pages,
                    reported_total,
                    pages_captured: pages,
                    items_captured: items,
                    folder_id,
                    folder_name,
                });
                Ok(())
            }
            "favorites" | "partial" | "checkpoint" => {
                let partial = payload.kind != "favorites";
                let checkpoint = payload.kind == "checkpoint";
                if !matches!(session.state.as_str(), "logged_in" | "reading_favorites")
                    || !matches!(session.login_status, Some(BfcLoginStatus::LoggedIn))
                {
                    return Err("BFC_LOGIN_REQUIRED".into());
                }
                let mid = session
                    .mid
                    .as_deref()
                    .ok_or("BFC_LOGIN_IDENTITY_NOT_FOUND")?;
                let path = url.path();
                let mut segments = path.split('/').filter(|segment| !segment.is_empty());
                if url.host_str() != Some("space.bilibili.com")
                    || segments.next() != Some(mid)
                    || segments.next() != Some("favlist")
                {
                    return Err("BFC_FAVORITES_PAGE_REQUIRED".into());
                }
                let expected_pages = payload.expected_pages.ok_or("BFC_PAGINATION_UNKNOWN")?;
                let reported_total = payload.reported_total.ok_or("BFC_TOTAL_UNKNOWN")?;
                let pages = payload.pages.ok_or("BFC_READBACK_PAGES_MISSING")?;
                let folder_id = payload.folder_id.unwrap_or_else(|| "unknown".into());
                let folder_name = payload.folder_name.unwrap_or_else(|| "当前收藏夹".into());
                if expected_pages == 0
                    || expected_pages > MAX_PAGES
                    || pages.is_empty()
                    || pages.len() > expected_pages as usize
                    || (!partial && pages.len() != expected_pages as usize)
                    || reported_total as usize > MAX_ITEMS
                    || folder_id.len() > 64
                    || folder_name.len() > 128
                {
                    return Err("BFC_READBACK_BOUNDS_INVALID".into());
                }
                let mut collected = BTreeMap::new();
                let mut global_ids = HashSet::new();
                let mut total = 0usize;
                for page in pages {
                    if page.page == 0 || page.page > expected_pages || page.items.len() > 100 {
                        return Err("BFC_READBACK_PAGE_INVALID".into());
                    }
                    // Page-index reuse is a protocol error even when its items
                    // also duplicate. Classify it before item-level checks.
                    if collected.contains_key(&page.page) {
                        return Err("BFC_READBACK_PAGE_DUPLICATE".into());
                    }
                    let mut seen = HashSet::new();
                    for item in &page.items {
                        if !valid_bvid(&item.bvid)
                            || item.title.len() > MAX_TITLE_BYTES
                            || item.title.contains('\0')
                            || !seen.insert(item.bvid.clone())
                            || !global_ids.insert(item.bvid.clone())
                        {
                            return Err("BFC_READBACK_ITEM_INVALID".into());
                        }
                    }
                    total = total.saturating_add(page.items.len());
                    if total > MAX_ITEMS {
                        return Err("BFC_READBACK_BOUNDS_INVALID".into());
                    }
                    if collected.insert(page.page, page.items).is_some() {
                        return Err("BFC_READBACK_PAGE_DUPLICATE".into());
                    }
                }
                if let Some(progress) = &session.progress {
                    if progress.expected_pages != expected_pages
                        || progress.reported_total != reported_total
                        || progress.pages_captured as usize != collected.len()
                        || progress.items_captured as usize != total
                        || progress.folder_id != folder_id
                        || progress.folder_name != folder_name
                    {
                        return Err("BFC_READBACK_PROGRESS_INVALID".into());
                    }
                }
                if total > reported_total as usize
                    || (!partial && total != reported_total as usize)
                    || (1..=collected.len() as u32).any(|page| !collected.contains_key(&page))
                {
                    return Err("BFC_READBACK_INCOMPLETE".into());
                }
                session.capture = Some(Capture {
                    complete: !partial,
                    expected_pages,
                    reported_total,
                    folder_id,
                    folder_name,
                    pages: collected,
                });
                session.state = if checkpoint {
                    "reading_favorites"
                } else if partial {
                    "partial"
                } else {
                    "ready"
                }
                .into();
                session.message_code = None;
                Ok(())
            }
            _ => Err("BFC_READBACK_KIND_INVALID".into()),
        }
    }

    pub fn status(&self, ticket: &str) -> Result<BfcReadbackStatus, String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let now = Instant::now();
        if sessions
            .get(ticket)
            .is_some_and(|session| session.expires_at <= now)
        {
            sessions.remove(ticket);
            return Err("BFC_READBACK_TICKET_EXPIRED".into());
        }
        let session = sessions.get(ticket).ok_or("BFC_READBACK_TICKET_UNKNOWN")?;
        let capture = session.capture.as_ref();
        Ok(BfcReadbackStatus {
            state: session.state.clone(),
            login_status: session
                .login_status
                .as_ref()
                .map(|value| value.as_str().to_string()),
            mid: session.mid.clone(),
            folder_id: capture.map(|value| value.folder_id.clone()).or_else(|| {
                session
                    .progress
                    .as_ref()
                    .map(|value| value.folder_id.clone())
            }),
            folder_name: capture.map(|value| value.folder_name.clone()).or_else(|| {
                session
                    .progress
                    .as_ref()
                    .map(|value| value.folder_name.clone())
            }),
            pages_captured: capture
                .map(|value| value.pages.len())
                .or_else(|| {
                    session
                        .progress
                        .as_ref()
                        .map(|value| value.pages_captured as usize)
                })
                .unwrap_or(0),
            expected_pages: capture
                .map(|value| value.expected_pages)
                .or_else(|| session.progress.as_ref().map(|value| value.expected_pages)),
            reported_total: capture
                .map(|value| value.reported_total)
                .or_else(|| session.progress.as_ref().map(|value| value.reported_total)),
            items_captured: capture
                .map(|value| value.pages.values().map(Vec::len).sum())
                .or_else(|| {
                    session
                        .progress
                        .as_ref()
                        .map(|value| value.items_captured as usize)
                })
                .unwrap_or(0),
            message_code: session.message_code.clone(),
            imported: session.imported.clone(),
        })
    }

    pub fn prepare_import(&self, ticket: &str) -> Result<(String, BfcImportPayload), String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let session = sessions
            .get_mut(ticket)
            .ok_or("BFC_READBACK_TICKET_UNKNOWN")?;
        if session.expires_at <= Instant::now() {
            return Err("BFC_READBACK_TICKET_EXPIRED".into());
        }
        if !matches!(session.state.as_str(), "ready" | "partial" | "failed") {
            return Err("BFC_READBACK_NOT_READY".into());
        }
        let capture = session.capture.as_ref().ok_or("BFC_READBACK_NOT_READY")?;
        let mut pages = BTreeMap::new();
        for (page, items) in &capture.pages {
            pages.insert(
                page.to_string(),
                BfcImportPage {
                    items: items.clone(),
                },
            );
        }
        let payload = BfcImportPayload {
            source: "authorized_browser_readback",
            scan_id: session.scan_id.clone(),
            reported_total: capture.reported_total,
            expected_pages: capture.expected_pages,
            traversal_finished: capture.complete,
            pages,
        };
        let encoded = serde_json::to_vec(&payload).map_err(|_| "BFC_IMPORT_SERIALIZE")?;
        if encoded.len() > MAX_IMPORT_BYTES {
            return Err("BFC_IMPORT_TOO_LARGE".into());
        }
        session.state = "importing".into();
        Ok((session.scan_id.clone(), payload))
    }

    pub fn mark_imported(&self, ticket: &str, result: BfcImportResult) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let session = sessions
            .get_mut(ticket)
            .ok_or("BFC_READBACK_TICKET_UNKNOWN")?;
        session.state = "imported".into();
        session.imported = Some(result);
        Ok(())
    }

    pub fn reset_ready(&self, ticket: &str) {
        if let Some(session) = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_mut(ticket)
        {
            if session.state == "importing" {
                session.state = if session
                    .capture
                    .as_ref()
                    .is_some_and(|capture| capture.complete)
                {
                    "ready"
                } else {
                    "partial"
                }
                .into();
            }
        }
    }

    pub fn mark_failed(&self, ticket: &str, code: &str) {
        if let Some(session) = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_mut(ticket)
        {
            session.state = "failed".into();
            session.message_code = Some(code.to_string());
        }
    }

    pub fn mark_outcome_unknown(&self, ticket: &str) {
        if let Some(session) = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_mut(ticket)
        {
            session.state = "outcome_unknown".into();
            session.message_code = Some("BFC_IMPORT_OUTCOME_UNKNOWN".into());
        }
    }

    pub fn cancel(&self, ticket: &str) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let session = sessions.get(ticket).ok_or("BFC_READBACK_TICKET_UNKNOWN")?;
        if matches!(
            session.state.as_str(),
            "importing" | "imported" | "outcome_unknown"
        ) {
            // The loopback write may already have completed. Retain its receipt
            // and do not tell the user cancellation was successful.
            return Err("BFC_IMPORT_CANCEL_UNSAFE".into());
        }
        sessions.remove(ticket);
        Ok(())
    }
}

fn valid_bvid(value: &str) -> bool {
    value.len() == 12
        && value.starts_with("BV")
        && value
            .bytes()
            .skip(2)
            .all(|byte| byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> (BfcRuntime, String, String) {
        let runtime = BfcRuntime::default();
        let ticket = runtime.begin("tab-7").unwrap();
        (runtime, ticket.ticket, ticket.scan_id)
    }

    #[test]
    fn partial_checkpoint_survives_abort_and_never_claims_complete() {
        let (runtime, ticket, _) = runtime();
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://www.bilibili.com/",
                BfcReadbackPayload {
                    kind: "login".into(),
                    login_status: Some("logged_in".into()),
                    mid: Some("42".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let payload = BfcReadbackPayload {
            kind: "checkpoint".into(),
            expected_pages: Some(136),
            reported_total: Some(4875),
            folder_id: Some("99".into()),
            folder_name: Some("fixture".into()),
            pages: Some(vec![BfcPage {
                page: 1,
                items: vec![BfcItem {
                    bvid: "BV1234567890".into(),
                    title: "fixture".into(),
                }],
            }]),
            ..Default::default()
        };
        let url = "https://space.bilibili.com/42/favlist?fid=99";
        runtime
            .submit(&ticket, "tab-7", url, payload.clone())
            .unwrap();
        assert!(runtime.prepare_import(&ticket).is_err());
        runtime
            .submit(
                &ticket,
                "tab-7",
                url,
                BfcReadbackPayload {
                    kind: "abort".into(),
                    message_code: Some("BFC_PAGINATION_NOT_ADVANCED".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(runtime.status(&ticket).unwrap().items_captured, 1);
        let (_, imported) = runtime.prepare_import(&ticket).unwrap();
        assert!(!imported.traversal_finished);
        assert_eq!(imported.reported_total, 4875);
        assert_eq!(imported.pages.len(), 1);
        runtime.reset_ready(&ticket);
        assert_eq!(runtime.status(&ticket).unwrap().state, "partial");
    }

    #[test]
    fn login_probe_distinguishes_logged_out_limited_and_unknown() {
        for (value, expected) in [
            ("logged_out", "logged_out"),
            ("access_limited", "access_limited"),
            ("unknown", "unknown"),
        ] {
            let (runtime, ticket, _) = runtime();
            runtime
                .submit(
                    &ticket,
                    "tab-7",
                    "https://www.bilibili.com/",
                    BfcReadbackPayload {
                        kind: "login".into(),
                        login_status: Some(value.into()),
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(
                runtime.status(&ticket).unwrap().login_status.as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn favorites_readback_requires_login_and_complete_ordered_pages() {
        let (runtime, ticket, _) = runtime();
        let pages = vec![
            BfcPage {
                page: 1,
                items: vec![BfcItem {
                    bvid: "BV1234567890".into(),
                    title: "one".into(),
                }],
            },
            BfcPage {
                page: 2,
                items: vec![BfcItem {
                    bvid: "BV1234567891".into(),
                    title: "two".into(),
                }],
            },
        ];
        let payload = BfcReadbackPayload {
            kind: "favorites".into(),
            expected_pages: Some(2),
            reported_total: Some(2),
            folder_id: Some("123".into()),
            folder_name: Some("我的收藏".into()),
            pages: Some(pages),
            ..Default::default()
        };
        assert_eq!(
            runtime.submit(
                &ticket,
                "tab-7",
                "https://space.bilibili.com/42/favlist?pn=1",
                payload.clone()
            ),
            Err("BFC_LOGIN_REQUIRED".into())
        );
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://www.bilibili.com/",
                BfcReadbackPayload {
                    kind: "login".into(),
                    login_status: Some("logged_in".into()),
                    mid: Some("42".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://space.bilibili.com/42/favlist?pn=1",
                payload,
            )
            .unwrap();
        assert_eq!(runtime.status(&ticket).unwrap().state, "ready");
        assert_eq!(runtime.status(&ticket).unwrap().pages_captured, 2);
    }

    #[test]
    fn rejects_wrong_tab_wrong_origin_and_partial_or_duplicate_pages() {
        let (runtime, ticket, _) = runtime();
        let login = BfcReadbackPayload {
            kind: "login".into(),
            login_status: Some("logged_in".into()),
            mid: Some("42".into()),
            ..Default::default()
        };
        runtime
            .submit(&ticket, "tab-7", "https://www.bilibili.com/", login)
            .unwrap();
        let page = BfcPage {
            page: 1,
            items: vec![BfcItem {
                bvid: "BV1234567890".into(),
                title: "one".into(),
            }],
        };
        let base = BfcReadbackPayload {
            kind: "favorites".into(),
            expected_pages: Some(2),
            reported_total: Some(2),
            pages: Some(vec![page.clone(), page]),
            ..Default::default()
        };
        assert_eq!(
            runtime.submit(
                &ticket,
                "tab-8",
                "https://space.bilibili.com/42/favlist",
                base.clone()
            ),
            Err("BFC_TAB_ID_MISMATCH".into())
        );
        assert_eq!(
            runtime.submit(
                &ticket,
                "tab-7",
                "https://example.com/42/favlist",
                base.clone()
            ),
            Err("BFC_PAGE_SOURCE_DENIED".into())
        );
        assert_eq!(
            runtime.submit(
                &ticket,
                "tab-7",
                "https://space.bilibili.com/42/favlist",
                base
            ),
            Err("BFC_READBACK_PAGE_DUPLICATE".into())
        );
    }

    #[test]
    fn logged_in_wrong_native_tab_never_becomes_logged_out() {
        let (runtime, ticket, _) = runtime();
        let login = BfcReadbackPayload {
            kind: "login".into(),
            login_status: Some("logged_in".into()),
            mid: Some("471851304".into()),
            ..Default::default()
        };
        assert_eq!(
            runtime.submit(
                &ticket,
                "tab-8",
                "https://space.bilibili.com/471851304/favlist",
                login.clone()
            ),
            Err("BFC_TAB_ID_MISMATCH".into())
        );
        let unchanged = runtime.status(&ticket).unwrap();
        assert_eq!(unchanged.state, "awaiting_login_probe");
        assert!(unchanged.login_status.is_none());
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://space.bilibili.com/471851304/favlist",
                login,
            )
            .unwrap();
        let accepted = runtime.status(&ticket).unwrap();
        assert_eq!(accepted.state, "logged_in");
        assert_eq!(accepted.mid.as_deref(), Some("471851304"));
    }

    #[test]
    fn injection_failure_does_not_claim_logged_out() {
        let (runtime, ticket, _) = runtime();
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://www.bilibili.com/",
                BfcReadbackPayload {
                    kind: "abort".into(),
                    message_code: Some("BFC_LOGIN_PROBE_INJECTION_FAILED".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let status = runtime.status(&ticket).unwrap();
        assert_eq!(status.state, "failed");
        assert_eq!(status.login_status, None);
        assert_eq!(
            status.message_code.as_deref(),
            Some("BFC_LOGIN_PROBE_INJECTION_FAILED")
        );
    }

    #[test]
    fn readback_page_progress_is_monotonic_and_final_snapshot_validated() {
        let (runtime, ticket, _) = runtime();
        let url = "https://space.bilibili.com/471851304/favlist";
        runtime
            .submit(
                &ticket,
                "tab-7",
                url,
                BfcReadbackPayload {
                    kind: "login".into(),
                    mid: Some("471851304".into()),
                    login_status: Some("logged_in".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let progress = |page, items| BfcReadbackPayload {
            kind: "progress".into(),
            pages_captured: Some(page),
            items_captured: Some(items),
            expected_pages: Some(2),
            reported_total: Some(2),
            folder_id: Some("default".into()),
            folder_name: Some("默认收藏夹".into()),
            ..Default::default()
        };
        let first_page_item_count = 1u32;
        runtime
            .submit(&ticket, "tab-7", url, progress(1, first_page_item_count))
            .unwrap();
        let status = runtime.status(&ticket).unwrap();
        assert_eq!(status.pages_captured, 1);
        assert_eq!(status.items_captured, 1);
        assert_eq!(status.expected_pages, Some(2));
        assert_eq!(status.folder_name.as_deref(), Some("默认收藏夹"));
        assert_eq!(
            runtime.submit(&ticket, "tab-7", url, progress(1, first_page_item_count)),
            Err("BFC_READBACK_PROGRESS_INVALID".into())
        );
        runtime
            .submit(&ticket, "tab-7", url, progress(2, 2))
            .unwrap();
        let pages = vec![
            BfcPage {
                page: 1,
                items: vec![BfcItem {
                    bvid: "BV1234567890".into(),
                    title: "one".into(),
                }],
            },
            BfcPage {
                page: 2,
                items: vec![BfcItem {
                    bvid: "BV1234567891".into(),
                    title: "two".into(),
                }],
            },
        ];
        runtime
            .submit(
                &ticket,
                "tab-7",
                url,
                BfcReadbackPayload {
                    kind: "favorites".into(),
                    expected_pages: Some(2),
                    reported_total: Some(2),
                    folder_id: Some("default".into()),
                    folder_name: Some("默认收藏夹".into()),
                    pages: Some(pages),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(runtime.status(&ticket).unwrap().state, "ready");
    }

    #[test]
    fn access_limited_abort_is_never_reported_as_logged_out() {
        let (runtime, ticket, _) = runtime();
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://www.bilibili.com/",
                BfcReadbackPayload {
                    kind: "abort".into(),
                    message_code: Some("BFC_BILIBILI_ACCESS_LIMITED".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let status = runtime.status(&ticket).unwrap();
        assert_eq!(status.state, "access_limited");
        assert_eq!(status.login_status, None);
        assert_eq!(
            status.message_code.as_deref(),
            Some("BFC_BILIBILI_ACCESS_LIMITED")
        );
    }

    #[test]
    fn cancel_cannot_discard_import_inflight_or_ambiguous_receipt() {
        let (runtime, ticket, _) = runtime();
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://www.bilibili.com/",
                BfcReadbackPayload {
                    kind: "login".into(),
                    login_status: Some("logged_in".into()),
                    mid: Some("42".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://space.bilibili.com/42/favlist",
                BfcReadbackPayload {
                    kind: "favorites".into(),
                    expected_pages: Some(1),
                    reported_total: Some(1),
                    pages: Some(vec![BfcPage {
                        page: 1,
                        items: vec![BfcItem {
                            bvid: "BV1234567890".into(),
                            title: "one".into(),
                        }],
                    }]),
                    ..Default::default()
                },
            )
            .unwrap();
        let _ = runtime.prepare_import(&ticket).unwrap();
        assert_eq!(
            runtime.cancel(&ticket),
            Err("BFC_IMPORT_CANCEL_UNSAFE".into())
        );
        assert_eq!(runtime.status(&ticket).unwrap().state, "importing");
        runtime.mark_outcome_unknown(&ticket);
        assert_eq!(
            runtime.cancel(&ticket),
            Err("BFC_IMPORT_CANCEL_UNSAFE".into())
        );
        assert_eq!(runtime.status(&ticket).unwrap().state, "outcome_unknown");
    }

    #[test]
    fn rejects_cross_page_duplicate_bvid_and_nonstandard_port() {
        let (runtime, ticket, _) = runtime();
        runtime
            .submit(
                &ticket,
                "tab-7",
                "https://www.bilibili.com/",
                BfcReadbackPayload {
                    kind: "login".into(),
                    login_status: Some("logged_in".into()),
                    mid: Some("42".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let repeated = BfcItem {
            bvid: "BV1234567890".into(),
            title: "same".into(),
        };
        let duplicate_pages = BfcReadbackPayload {
            kind: "favorites".into(),
            expected_pages: Some(2),
            reported_total: Some(2),
            pages: Some(vec![
                BfcPage {
                    page: 1,
                    items: vec![repeated.clone()],
                },
                BfcPage {
                    page: 2,
                    items: vec![repeated],
                },
            ]),
            ..Default::default()
        };
        assert_eq!(
            runtime.submit(
                &ticket,
                "tab-7",
                "https://space.bilibili.com:444/42/favlist",
                duplicate_pages.clone()
            ),
            Err("BFC_PAGE_SOURCE_DENIED".into())
        );
        assert_eq!(
            runtime.submit(
                &ticket,
                "tab-7",
                "https://space.bilibili.com/42/favlist",
                duplicate_pages
            ),
            Err("BFC_READBACK_ITEM_INVALID".into())
        );
    }
}
