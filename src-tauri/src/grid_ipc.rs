//! 宫格 IPC 协议（Phase 2）：主进程 ↔ 宫格子进程 的 UDS 线协议。
//!
//! 传输：Unix Domain Socket + 行分隔 JSON（serde_json，无新依赖）。
//! 双向三种帧：Request（主→子，带 seq）/ Response（子→主，带 seq）/ Event（子→主，异步）。
//! 子进程死亡/重启时连接断开，主进程 reader 线程置断连标记并唤醒挂起请求。

use serde::{Deserialize, Serialize};

/// 绝对屏幕物理像素矩形（主进程已按主窗 inner_position + scale_factor 换算）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct IpcRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl IpcRect {
    /// Preserve negative monitor/offscreen coordinates but reject invalid
    /// float-to-native conversions and unbounded renderer viewports.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !(i32::MIN as f64..=i32::MAX as f64).contains(&self.x)
            || !(i32::MIN as f64..=i32::MAX as f64).contains(&self.y)
            || !(1.0..=32768.0).contains(&self.w)
            || !(1.0..=32768.0).contains(&self.h)
        {
            return Err("GRID_NATIVE_RECT_INVALID".into());
        }
        Ok(())
    }
}

// Separate ordinary-tab and Grid route namespaces without a second process
// manager or a second IPC transport. A tab route is never a Grid cell index.
const TAB_RENDERER_BIT: u32 = 1 << 31;
pub(crate) const MAX_ISOLATED_TAB_RENDERERS: usize = 8;
/// Hard kernel-accounted per ordinary renderer domain. This does not include
/// GPU/broker allocations outside the owned cgroup or Job.
pub(crate) const TAB_RENDERER_MEMORY_LIMIT: usize = 2 * 1024 * 1024 * 1024;
/// Reserve estimated startup memory while native children are still warming.
/// This is host admission accounting, NOT an engine allocation hard limit.
pub(crate) const RENDERER_STARTUP_RESERVE: std::time::Duration =
    std::time::Duration::from_secs(10);

/// One sorted, process-owned observation used by telemetry and Agent
/// metadata. Epoch avoids treating a recycled PID as the previous renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RendererProcess {
    pub index: u32,
    pub pid: u32,
    pub spawn_epoch: u64,
    pub ready: bool,
    pub startup_reserved: bool,
}

pub(crate) fn tab_renderer_index(id: &str) -> Option<u32> {
    let n: u32 = id.strip_prefix("tab-")?.parse().ok()?;
    if n == 0 || n >= TAB_RENDERER_BIT || id != format!("tab-{n}") {
        return None;
    }
    Some(TAB_RENDERER_BIT | n)
}

pub(crate) fn is_tab_renderer(index: u32) -> bool {
    index > TAB_RENDERER_BIT
}

pub(crate) fn renderer_label(index: u32) -> String {
    if is_tab_renderer(index) {
        format!("tab-{}", index & !TAB_RENDERER_BIT)
    } else {
        format!("grid-{index}")
    }
}

const RENDERER_PROTOCOL: &str = "browseros-renderer-v3";

/// Parent-owned native identity, never supplied by a remote page or frontend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RendererBinding {
    pub index: u32,
    pub owner: (u32, u64),
    pub generation: u64,
    pub navigation_revision: u64,
}
pub(crate) const MAX_RENDERER_CAPTURE_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const RENDERER_CAPTURE_CHUNK_BYTES: usize = 192 * 1024;

pub(crate) fn renderer_ready(index: u32) -> Wire {
    Wire::Event {
        name: "renderer-ready".into(),
        payload: serde_json::json!({ "index": index, "protocol": RENDERER_PROTOCOL }),
    }
}

pub(crate) fn is_renderer_ready(index: u32, frame: &Wire) -> bool {
    matches!(
        frame,
        Wire::Event { name, payload }
            if name == "renderer-ready"
                && payload["index"].as_u64() == Some(u64::from(index))
                && payload["protocol"].as_str() == Some(RENDERER_PROTOCOL)
    )
}

/// A denied popup is not a corrupt renderer connection. The parent may
/// reopen an ordinary address but cannot transfer a transient document,
/// opener, script URL, or remote-page access to local files.
pub(crate) fn popup_address_supported(address: &str, source: Option<&str>) -> bool {
    if address.is_empty() || address.len() > 8192 || address.trim() != address {
        return false;
    }
    let Ok(url) = url::Url::parse(address) else { return false; };
    match url.scheme() {
        "http" | "https" => url.has_host(),
        "about" => address == "about:blank",
        "file" => source.and_then(|source| url::Url::parse(source).ok())
            .is_some_and(|source| source.scheme() == "file"),
        _ => false,
    }
}

/// Only existing renderer events may cross the process boundary.
pub(crate) fn valid_renderer_event(index: u32, name: &str, payload: &serde_json::Value) -> bool {
    let label = renderer_label(index);
    let index_text = index.to_string();
    let valid_url = || {
        payload["url"].as_str().is_some_and(|url| !url.is_empty() && url.len() <= 8192)
    };
    match name {
        "grid-child-focus" | "grid-child-blur" => {
            payload["index"].as_u64() == Some(u64::from(index))
        }
        "tab-navigated" | "new-tab-request" => {
            payload["id"].as_str() == Some(label.as_str()) && valid_url()
        }
        "grid-load-failed" => {
            payload["index"].as_str() == Some(index_text.as_str()) && valid_url()
        }
        "renderer-resource" => is_tab_renderer(index)
            && payload["id"].as_str() == Some(label.as_str())
            && payload["resource"].is_object(),
        "tab-title" => is_tab_renderer(index)
            && payload["id"].as_str() == Some(label.as_str())
            && valid_url()
            && payload["title"].as_str().is_some_and(|title| title.len() <= 8192),
        "browser-resources" => is_tab_renderer(index)
            && payload["id"].as_str() == Some(label.as_str())
            && payload["page_url"].as_str().is_some_and(|url| url.len() <= 8192)
            && payload["items"].as_array().is_some_and(|items| items.len() <= 100),
        "browser-tabs://shell-shortcut" => is_tab_renderer(index)
            && payload["id"].as_str() == Some(label.as_str())
            && matches!(payload["action"].as_str(), Some("immersive" | "compact" | "address")),
        "open-terminal" => payload.is_null(),
        "note-saved" => payload.is_string(),
        "artifact-collected" => payload.is_object(),
        _ => false,
    }
}

/// 主进程 → 子进程 命令。
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum GridCmd {
    /// 在子进程主窗内 add_child 创建宫格 webview（id 即 label，如 "grid-0"）。
    CreateTab {
        id: String,
        url: String,
    },
    /// 移动/缩放子进程窗口到绝对屏幕矩形（收到后若窗口隐藏则 show）。
    UpdateRect {
        id: String,
        rect: IpcRect,
    },
    /// 隐藏子进程整个窗口（切视图/失焦，等价旧方案的移出屏幕）。
    HideWindow {
        id: String,
    },
    /// 在宫格 webview 内执行 JS。
    Eval {
        id: String,
        js: String,
    },
    ReadReplies {
        id: String,
    },
    FillCredential { id: String, origin: String, username: String, password: String },
    CaptureStart { id: String, ticket: String },
    CaptureRead { id: String, ticket: String, offset: usize },
    CaptureRelease { id: String, ticket: String },
    /// Read-only native WebView safety probe before discarding an empty Grid.
    /// Absence of an explicit safe response must fail closed.
    HibernationProbe {
        id: String,
    },
    /// 宫格 webview 导航到 url。
    Navigate {
        id: String,
        url: String,
    },
    /// 设置宫格缩放（WebKitGTK zoom_level）。
    SetZoom {
        id: String,
        zoom: f64,
    },
    /// 销毁宫格 webview（子进程随后通常被 kill）。
    CloseTab {
        id: String,
    },
    Ping,
}

impl std::fmt::Debug for GridCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GridCmd(<private payload>)")
    }
}

impl GridCmd {
    /// Only a newly created blank document with no subsequent page-affecting
    /// native command can be considered for automatic ordinary-tab reclaim.
    pub(crate) fn preserves_pristine_blank(&self) -> bool {
        matches!(self, Self::CreateTab { url, .. } if url == "about:blank")
            || matches!(self, Self::UpdateRect { .. } | Self::HideWindow { .. }
                | Self::HibernationProbe { .. } | Self::Ping)
    }

    pub(crate) fn allowed_during_blank_reclaim(&self) -> bool {
        matches!(self, Self::HideWindow { .. } | Self::HibernationProbe { .. })
    }

    /// Each child owns exactly one renderer identity. A transport connection
    /// is not permission to address another WebView or create arbitrary tabs.
    pub(crate) fn targets(&self, index: u32) -> bool {
        if matches!(self, Self::FillCredential { .. } | Self::CaptureStart { .. }
            | Self::CaptureRead { .. } | Self::CaptureRelease { .. })
            && !is_tab_renderer(index)
        {
            return false;
        }
        let id = match self {
            Self::CreateTab { id, .. }
            | Self::UpdateRect { id, .. }
            | Self::HideWindow { id }
            | Self::Eval { id, .. }
            | Self::ReadReplies { id }
            | Self::FillCredential { id, .. }
            | Self::CaptureStart { id, .. }
            | Self::CaptureRead { id, .. }
            | Self::CaptureRelease { id, .. }
            | Self::HibernationProbe { id }
            | Self::Navigate { id, .. }
            | Self::SetZoom { id, .. }
            | Self::CloseTab { id } => id,
            Self::Ping => return true,
        };
        id == &renderer_label(index)
    }
}

/// The sole request reader fences the exact worker route and strictly
/// increasing sequence before any native dispatch, including async probes.
/// A duplicate/invalid frame is a protocol fault, not a second operation.
#[derive(Default)]
pub(crate) struct GridRequestFence {
    last: u64,
}

impl GridRequestFence {
    pub(crate) fn admit(&mut self, index: u32, seq: u64, cmd: &GridCmd) -> Result<(), &'static str> {
        if seq == 0 || seq <= self.last {
            return Err("GRID_REQUEST_SEQUENCE_REJECTED");
        }
        if !cmd.targets(index) {
            return Err("GRID_REQUEST_TARGET_REJECTED");
        }
        self.last = seq;
        Ok(())
    }
}

pub(crate) fn next_request_sequence(
    counter: &std::sync::atomic::AtomicU64,
) -> Result<u64, String> {
    use std::sync::atomic::Ordering;
    counter
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
            if n == 0 { None } else { n.checked_add(1) }
        })
        .map_err(|_| "GRID_REQUEST_SEQUENCE_EXHAUSTED".into())
}

/// Linux UDS credentials are supplied by the kernel, not by a JSON frame
/// or the socket filename. Require the same effective UID and a live PID.
#[cfg(target_os = "linux")]
pub(crate) fn linux_peer_pid(stream: &std::os::unix::net::UnixStream) -> std::io::Result<u32> {
    use std::os::fd::AsRawFd;
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if len as usize != std::mem::size_of::<libc::ucred>()
        || cred.pid <= 0
        || cred.uid != unsafe { libc::geteuid() }
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "GRID_PEER_CREDENTIALS_REJECTED",
        ));
    }
    Ok(cred.pid as u32)
}

/// 线协议帧。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Wire {
    Request {
        seq: u64,
        cmd: GridCmd,
    },
    Response {
        seq: u64,
        ok: bool,
        err: Option<String>,
        #[serde(default)]
        data: Option<String>,
    },
    /// 子进程 → 主进程 异步事件（导航完成/新窗口请求/桥命令事件转发）。
    Event {
        name: String,
        payload: serde_json::Value,
    },
}

impl Wire {
    pub fn ok(seq: u64) -> Self {
        Wire::Response {
            seq,
            ok: true,
            err: None,
            data: None,
        }
    }
    pub fn err(seq: u64, e: impl Into<String>) -> Self {
        Wire::Response {
            seq,
            ok: false,
            err: Some(e.into()),
            data: None,
        }
    }
}

// Cross-platform IPC bound. Remote/webpage data may be embedded in messages;
// refuse oversized frames before allocation or a synchronous pipe write.
pub const MAX_GRID_WIRE_BYTES: usize = 1024 * 1024;

/// 写一帧（JSON + '\n'）。
pub fn write_wire<W: std::io::Write>(w: &mut W, msg: &Wire) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if line.len() >= MAX_GRID_WIRE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GRID_WIRE_LIMIT",
        ));
    }
    line.push(b'\n');
    w.write_all(&line)?;
    w.flush()
}

/// Read at most one bounded JSON line without allocating for untrusted
/// oversized messages. Preserve BufRead's extra bytes for the next frame.
pub fn read_wire<R: std::io::BufRead>(r: &mut R) -> std::io::Result<Option<Wire>> {
    use std::io::{Error, ErrorKind};
    let mut bytes = Vec::new();
    loop {
        let available = r.fill_buf()?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(Error::new(ErrorKind::UnexpectedEof, "GRID_WIRE_TRUNCATED"));
        }
        let n = available
            .iter()
            .position(|b| *b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(available.len());
        if bytes.len().saturating_add(n) > MAX_GRID_WIRE_BYTES {
            return Err(Error::new(ErrorKind::InvalidData, "GRID_WIRE_LIMIT"));
        }
        let terminated = available[n - 1] == b'\n';
        bytes.extend_from_slice(&available[..n]);
        r.consume(n);
        if terminated {
            break;
        }
    }
    let message = serde_json::from_slice::<Wire>(&bytes[..bytes.len() - 1])
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    Ok(Some(message))
}

#[cfg(test)]
mod framing_tests {
    use super::*;
    #[test]
    fn ordinary_renderer_and_grid_routes_never_alias() {
        let tab = tab_renderer_index("tab-2").unwrap();
        assert!(is_tab_renderer(tab));
        assert_ne!(tab, 2);
        assert_eq!(renderer_label(tab), "tab-2");
        assert_eq!(renderer_label(2), "grid-2");
        assert!(tab_renderer_index("tab-02").is_none());
        assert!(tab_renderer_index("tab-0").is_none());
        assert!(tab_renderer_index("grid-2").is_none());
        assert!(tab_renderer_index("tab-2147483648").is_none());
        let navigate = GridCmd::Navigate { id: "tab-2".into(), url: "about:blank".into() };
        assert!(navigate.targets(tab));
        assert!(!navigate.targets(2));
        assert!(!GridCmd::HideWindow { id: "grid-2".into() }.targets(tab));
        assert!(valid_renderer_event(tab, "tab-navigated",
            &serde_json::json!({"id":"tab-2","url":"about:blank"})));
        assert!(!valid_renderer_event(tab, "tab-navigated",
            &serde_json::json!({"id":"grid-2","url":"about:blank"})));
    }

    #[test]
    fn only_pristine_blank_and_layout_commands_preserve_automatic_reclaim() {
        let id = "tab-1".to_owned();
        assert!(GridCmd::CreateTab { id: id.clone(), url: "about:blank".into() }.preserves_pristine_blank());
        assert!(!GridCmd::CreateTab { id: id.clone(), url: "https://example.test".into() }.preserves_pristine_blank());
        assert!(!GridCmd::Navigate { id: id.clone(), url: "about:blank".into() }.preserves_pristine_blank());
        assert!(!GridCmd::Eval { id: id.clone(), js: "history.back()".into() }.preserves_pristine_blank());
        assert!(!GridCmd::CaptureStart { id: id.clone(), ticket: "fixture".into() }.allowed_during_blank_reclaim());
        assert!(GridCmd::HideWindow { id: id.clone() }.allowed_during_blank_reclaim());
        assert!(GridCmd::HibernationProbe { id }.allowed_during_blank_reclaim());
        assert!(!GridCmd::Ping.allowed_during_blank_reclaim());
    }

    #[test]
    fn private_services_are_route_bound_and_debug_redacted() {
        let index = tab_renderer_index("tab-4").unwrap();
        let fill = GridCmd::FillCredential { id: "tab-4".into(), origin: "https://example.test".into(),
            username: "fixture-user".into(), password: "fixture-secret".into() };
        assert!(fill.targets(index));
        assert!(!fill.targets(4));
        assert!(!fill.targets(tab_renderer_index("tab-5").unwrap()));
        assert!(!format!("{fill:?}").contains("fixture-secret"));
        let capture = GridCmd::CaptureRead { id: "tab-4".into(), ticket: "fixture".into(), offset: 0 };
        assert!(capture.targets(index));
        assert!(!capture.targets(4));
        assert!(!valid_renderer_event(4, "renderer-resource", &serde_json::json!({"id":"grid-4","resource":{}})));
    }

    #[test]
    fn renderer_ready_and_event_route_are_exact() {
        use serde_json::json;
        let ready = renderer_ready(2);
        assert!(is_renderer_ready(2, &ready));
        assert!(!is_renderer_ready(3, &ready));
        assert!(!is_renderer_ready(2, &Wire::ok(1)));
        assert!(valid_renderer_event(2, "tab-navigated", &json!({"id":"grid-2","url":"about:blank"})));
        assert!(!valid_renderer_event(2, "tab-navigated", &json!({"id":"tab-2","url":"about:blank"})));
        assert!(!valid_renderer_event(2, "grid-load-failed", &json!({"index":"3","url":"about:blank"})));
        assert!(!valid_renderer_event(2, "renderer-ready", &json!({"index":2})));
        assert!(!valid_renderer_event(2, "arbitrary-host-event", &json!(null)));
        assert!(valid_renderer_event(2, "open-terminal", &json!(null)));
        let tab = tab_renderer_index("tab-2").unwrap();
        assert!(valid_renderer_event(tab, "browser-tabs://shell-shortcut", &json!({"id":"tab-2","action":"address"})));
        assert!(!valid_renderer_event(tab, "browser-tabs://shell-shortcut", &json!({"id":"tab-3","action":"address"})));
        assert!(!valid_renderer_event(tab, "browser-tabs://shell-shortcut", &json!({"id":"tab-2","action":"arbitrary"})));
        assert!(!valid_renderer_event(2, "browser-tabs://shell-shortcut", &json!({"id":"grid-2","action":"address"})));
    }

    #[test]
    fn invalid_native_geometry_is_rejected() {
        let rect = IpcRect { x: -30000.0, y: 10.0, w: 800.0, h: 600.0 };
        assert!(rect.validate().is_ok());
        assert!(IpcRect { w: f64::NAN, ..rect }.validate().is_err());
        assert!(IpcRect { h: 0.0, ..rect }.validate().is_err());
        assert!(IpcRect { x: f64::INFINITY, ..rect }.validate().is_err());
        assert!(IpcRect { w: 32769.0, ..rect }.validate().is_err());
    }

    #[test]
    fn worker_route_and_sequence_are_fenced_before_native_dispatch() {
        let mut fence = GridRequestFence::default();
        let valid = GridCmd::Navigate { id: "grid-2".into(), url: "about:blank".into() };
        let foreign = GridCmd::CreateTab { id: "tab-1".into(), url: "about:blank".into() };
        assert_eq!(fence.admit(2, 0, &valid), Err("GRID_REQUEST_SEQUENCE_REJECTED"));
        assert!(fence.admit(2, 1, &valid).is_ok());
        assert_eq!(fence.admit(2, 1, &valid), Err("GRID_REQUEST_SEQUENCE_REJECTED"));
        assert_eq!(fence.admit(2, 2, &foreign), Err("GRID_REQUEST_TARGET_REJECTED"));
        assert!(fence.admit(2, 3, &GridCmd::Ping).is_ok());
        assert_eq!(fence.admit(2, 2, &valid), Err("GRID_REQUEST_SEQUENCE_REJECTED"));
        assert!(!GridCmd::HideWindow { id: "grid-3".into() }.targets(2));
    }

    #[test]
    fn sequence_exhaustion_never_wraps_or_reuses_a_request_identity() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let counter = AtomicU64::new(u64::MAX - 1);
        assert_eq!(next_request_sequence(&counter).unwrap(), u64::MAX - 1);
        assert!(next_request_sequence(&counter).is_err());
        assert_eq!(counter.load(Ordering::SeqCst), u64::MAX);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_peer_credentials_come_from_kernel() {
        let (first, second) = std::os::unix::net::UnixStream::pair().unwrap();
        assert_eq!(linux_peer_pid(&first).unwrap(), std::process::id());
        assert_eq!(linux_peer_pid(&second).unwrap(), std::process::id());
    }

    #[test]
    fn two_requests_survive_shared_bufreader_without_losing_prefetched_frames() {
        let mut buffer = Vec::new();
        write_wire(
            &mut buffer,
            &Wire::Request {
                seq: 1,
                cmd: GridCmd::Ping,
            },
        )
        .unwrap();
        write_wire(
            &mut buffer,
            &Wire::Request {
                seq: 2,
                cmd: GridCmd::Ping,
            },
        )
        .unwrap();
        let mut reader = std::io::BufReader::new(buffer.as_slice());
        assert!(matches!(
            read_wire(&mut reader).unwrap(),
            Some(Wire::Request { seq: 1, .. })
        ));
        assert!(matches!(
            read_wire(&mut reader).unwrap(),
            Some(Wire::Request { seq: 2, .. })
        ));
        assert!(read_wire(&mut reader).unwrap().is_none());
    }
    #[test]
    fn rejects_oversized_line_and_truncated_frame() {
        let huge = vec![b'x'; MAX_GRID_WIRE_BYTES + 2];
        let mut reader = std::io::BufReader::new(huge.as_slice());
        assert_eq!(
            read_wire(&mut reader).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        let mut reader = std::io::BufReader::new(b"{".as_slice());
        assert_eq!(
            read_wire(&mut reader).unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
