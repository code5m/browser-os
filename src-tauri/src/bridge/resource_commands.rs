//! Browser resource collection, redaction, event reporting and scanning
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

/// 插件上报的原始资源事件（browser-tabs://event payload，未脱敏，仅进程内使用）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawResourceEvent {
    pub url: String,
    pub method: Option<String>,
    pub status: Option<u32>,
    pub mime: Option<String>,
    pub size_bytes: Option<u64>,
    pub started_at: i64,
    pub finished_at: i64,
}

/// 资源记录缓冲：每 tab 一个 FIFO 队列（容量上限 N），全局再一层 FIFO（上限 M）。
/// `order` 记录全局插入序，全局驱逐时跳过已被 per-tab 驱逐/清理的过期项。
#[derive(Default)]
pub struct ResourceBuffer {
    per_tab: HashMap<String, std::collections::VecDeque<ResourceReceived>>,
    order: std::collections::VecDeque<(String, String)>, // (tab_id, record_id)
    pub(super) total: usize,
    /// 每 tab 因容量上限被丢弃的累计条数（前端超限提示）
    evicted_per_tab: HashMap<String, u64>,
    /// 全局因总上限被丢弃的累计条数
    pub(super) evicted_global: u64,
}

impl ResourceBuffer {
    /// 入库一条记录，执行 per-tab 与全局两级 FIFO 驱逐。
    pub fn push(&mut self, rec: ResourceReceived, max_per_tab: usize, max_total: usize) {
        let tab = rec.tab_id.clone();
        let rid = rec.id.clone();
        {
            let q = self.per_tab.entry(tab.clone()).or_default();
            q.push_back(rec);
            while q.len() > max_per_tab {
                q.pop_front();
                *self.evicted_per_tab.entry(tab.clone()).or_insert(0) += 1;
                self.total = self.total.saturating_sub(1);
            }
        }
        self.order.push_back((tab, rid));
        self.total += 1;
        // 全局 FIFO：从 order 头部驱逐，跳过已被 per-tab 驱逐/清理的过期项
        while self.total > max_total {
            match self.order.pop_front() {
                None => break,
                Some((t, rid)) => {
                    if let Some(q) = self.per_tab.get_mut(&t) {
                        if q.front().map(|r| r.id.as_str()) == Some(rid.as_str()) {
                            q.pop_front();
                            self.total = self.total.saturating_sub(1);
                            self.evicted_global += 1;
                        }
                        // 过期项（该记录已被 per-tab 驱逐）：仅丢弃 order 项
                        if q.is_empty() {
                            self.per_tab.remove(&t);
                        }
                    }
                }
            }
        }
    }

    /// 某 tab 的记录快照（按入库顺序）。
    pub fn records(&self, tab_id: &str) -> Vec<ResourceReceived> {
        self.per_tab
            .get(tab_id)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 清空某 tab 的记录，返回清除条数。`order` 中的过期项留给全局驱逐时惰性跳过。
    pub fn clear_tab(&mut self, tab_id: &str) -> usize {
        let removed = self.per_tab.remove(tab_id).map(|q| q.len()).unwrap_or(0);
        self.total = self.total.saturating_sub(removed);
        self.evicted_per_tab.remove(tab_id);
        removed
    }

    /// tab 关闭时清理（语义同 clear_tab）。
    pub fn remove_tab(&mut self, tab_id: &str) {
        self.clear_tab(tab_id);
    }

    pub fn clear_all(&mut self) {
        self.per_tab.clear();
        self.order.clear();
        self.total = 0;
        self.evicted_per_tab.clear();
        self.evicted_global = 0;
    }

    pub fn evicted_of(&self, tab_id: &str) -> u64 {
        self.evicted_per_tab.get(tab_id).copied().unwrap_or(0)
    }
}

/// 资源类型归类：优先按真实 MIME，缺失/未知时按 URL 后缀兜底。
/// 注意（边界声明）：WebKitGTK 信号不提供 initiator 类型，XHR/Fetch 与
/// 普通数据响应无法从信号区分，json/xml/text 启发式归入 xhr_fetch，可能含误判。
pub fn classify_resource_kind(mime: Option<&str>, url: &str) -> ResourceKind {
    let m = mime.unwrap_or_default().to_ascii_lowercase();
    if !m.is_empty() {
        if m.contains("html") {
            return ResourceKind::Document;
        }
        if m.contains("javascript") || m.contains("ecmascript") {
            return ResourceKind::Script;
        }
        if m == "text/css" {
            return ResourceKind::Stylesheet;
        }
        if m.starts_with("image/") {
            return ResourceKind::Image;
        }
        if m.starts_with("font/") || m.contains("font") {
            return ResourceKind::Font;
        }
        if m.starts_with("audio/") || m.starts_with("video/") {
            return ResourceKind::Media;
        }
        if m.contains("json") || m.contains("xml") || m.starts_with("text/") {
            return ResourceKind::XhrFetch;
        }
        return ResourceKind::Other;
    }
    let path = url::Url::parse(url)
        .map(|u| u.path().to_ascii_lowercase())
        .unwrap_or_default();
    let has_ext = |exts: &[&str]| exts.iter().any(|e| path.ends_with(e));
    if has_ext(&[".js", ".mjs"]) {
        ResourceKind::Script
    } else if has_ext(&[".css"]) {
        ResourceKind::Stylesheet
    } else if has_ext(&[
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".ico", ".avif",
    ]) {
        ResourceKind::Image
    } else if has_ext(&[".woff", ".woff2", ".ttf", ".otf", ".eot"]) {
        ResourceKind::Font
    } else if has_ext(&[".mp4", ".webm", ".mp3", ".ogg", ".wav", ".m3u8"]) {
        ResourceKind::Media
    } else if has_ext(&[".html", ".htm", "/"]) {
        ResourceKind::Document
    } else if has_ext(&[".json"]) {
        ResourceKind::XhrFetch
    } else {
        ResourceKind::Other
    }
}

/// 原始事件 → 脱敏 DTO（纯函数，便于单测）。
/// 仅 http/https 入库（file:/data:/blob:/about: 一律丢弃）；返回 None 表示不入库。
pub fn build_resource_received(
    tab_id: &str,
    raw: &RawResourceEvent,
    max_url_bytes: usize,
) -> Option<ResourceReceived> {
    let lower = raw.url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return None;
    }
    let mut url = crate::security_policy::redact_sensitive_url(&raw.url);
    // 双保险：settings 可调低 max_url_bytes（不允许调高过策略硬上限）
    let cap = max_url_bytes.min(crate::security_policy::MAX_RESOURCE_URL_BYTES);
    if url.len() > cap {
        let mut end = cap;
        while end > 0 && !url.is_char_boundary(end) {
            end -= 1;
        }
        url = format!("{}…", &url[..end]);
    }
    let method = raw
        .method
        .as_deref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .unwrap_or("GET")
        .to_string();
    // 未知长度映射：0 视为未知（WebKit content_length 未知时返回 0），不伪造
    let size_bytes = raw.size_bytes.filter(|n| *n > 0);
    let duration_ms = if raw.finished_at >= raw.started_at {
        Some((raw.finished_at - raw.started_at) as u64)
    } else {
        None
    };
    Some(ResourceReceived {
        id: uuid::Uuid::new_v4().to_string(),
        tab_id: tab_id.to_string(),
        resource_type: classify_resource_kind(raw.mime.as_deref(), &url),
        url,
        method,
        status: raw.status,
        mime: raw.mime.clone(),
        size_bytes,
        started_at: raw.started_at,
        finished_at: Some(raw.finished_at),
        duration_ms,
    })
}

/// 插件原生资源事件入口（main.rs 事件转发调用）。
/// enabled=false 时直接丢弃（不采集）；入库后向主窗口前端推 `resource-received`。
pub fn on_resource_received(app: &AppHandle, tab_id: &str, raw: RawResourceEvent) {
    let state = app.state::<AppState>();
    let (enabled, max_per_tab, max_total, max_url_bytes) = {
        let s = state.resource_capture.lock().unwrap();
        (s.enabled, s.max_per_tab, s.max_total, s.max_url_bytes)
    };
    if !enabled {
        return;
    }
    let Some(rec) = build_resource_received(tab_id, &raw, max_url_bytes) else {
        return;
    };
    state
        .resource_buffer
        .lock()
        .unwrap()
        .push(rec.clone(), max_per_tab, max_total);
    let _ = app.emit("resource-received", rec);
}

/// tab_id 形态校验：只接受 `tab-N` 页签（宫格 grid-N 在子进程，不在本缓冲范围）。
/// id 形态校验：只允许 `[A-Za-z0-9_-]`，长度 ≤ `IMAGE_ID_MAX_LEN`。
///
/// 背景（M1-ACCEPT 审计挂账项）：`session_get/delete/export/restore` 的 id 会被直接

// M1-8 tab 资源命令组（list/clear_tab_resources、get/set_resource_capture_settings）
// 已迁移至 capabilities/browser/commands.rs（owner 收口为 browser，resource_collection 子域归并）。

/// 由子窗口内 JS 经 invoke 回传的真实页面标题，转成 tab-title 事件推给前端。
#[tauri::command]
pub fn report_title(app: AppHandle, webview: tauri::Webview, title: String) -> Result<(), String> {
    use crate::security_policy as sp;
    check_invocation_source(&webview, "report_title", None, &app)?;
    sp::check_text_field("title", &title, sp::MAX_TEXT_FIELD_BYTES).map_err(|e| e.to_string())?;
    let id = webview.label().to_string();
    if !is_tab_label(&id) {
        return Err("page title must originate from a browser tab".to_string());
    }
    let page_url = webview.url().map_err(|e| e.to_string())?.to_string();
    report_title_inner(app, id, page_url, title)
}

fn report_title_inner(
    app: AppHandle,
    id: String,
    page_url: String,
    title: String,
) -> Result<(), String> {
    let t = title.trim().to_string();
    if t.is_empty() {
        return Ok(());
    }
    // Bind title to the invoking WebView, not whichever tab became active
    // while this asynchronous IPC message was in flight.
    if !app
        .state::<AppState>()
        .tabs
        .lock()
        .unwrap()
        .contains_key(&id)
    {
        return Ok(());
    }
    let _ = app.emit(
        "tab-title",
        serde_json::json!({ "id": id, "url": page_url, "title": t }),
    );
    Ok(())
}

/// 对指定子窗口执行 JS 资源扫描，并回传真实标题，通过事件推送给前端。
/// 注意：WebviewWindow 的 eval 在 Tauri 2 主线程执行即可；资源/标题的回传目前通过
/// webview 内已注入的 `window.ipc.postMessage` 通道（见 resources_eval.js 改造）由前端监听，
/// 这里仅负责触发执行。插件的 Webview dispatcher 支持从扫描线程调用。
fn scan_resources(app: &AppHandle, id: &str) -> Result<(), String> {
    // 执行资源扫描脚本（脚本内部会把结果经 ipc 回传，由前端 onBrowserResources 接收）
    let js = include_str!("../../injected/resources_eval.js");
    plugin_eval(app, id, js)?;
    // 标题回传：执行一段脚本把 document.title 经 invoke 回传（见 report_title command）
    plugin_eval(
        app,
        id,
        "window.__TAURI__ && window.__TAURI__.core.invoke('report_title', { title: document.title })",
    )?;
    Ok(())
}

/// 启动进程级唯一后台线程：每 2 秒扫描一次当前激活页签的资源。
pub(crate) fn start_resource_scanner(app: AppHandle) {
    let state = app.state::<AppState>();
    if state.browser_scanner_started.swap(true, Ordering::SeqCst) {
        return;
    }
    let app_thread = app.clone();
    std::thread::spawn(move || loop {
        let state = app_thread.state::<AppState>();
        if state.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }
        let active = state.active_tab.lock().unwrap().clone();
        if let Some(id) = active {
            if let Err(error) = scan_resources(&app_thread, &id) {
                eprintln!("[scan_resources] id={id} eval failed: {error}");
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}
