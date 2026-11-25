//!
//! Browser 能力：页签 / WebView 的创建与基础原语。
//! PHASE 2 从 bridge.rs 物理迁移（native-physical-batch-browser-create-tab）。
//! 仅做模块归属，不改任何行为；WebView/Grid 生命周期、active tab 权威、
//! session persistence 派生（从 Browser 权威 tabs 表）等冻结语义均不变。
use crate::domain::*;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

/// A non-tiny offscreen/hidden initial viewport avoids a 1x1 WebKitGTK
/// relayout while the first navigation and native tab positioning race.
/// Keep these at a modest stable size; the frontend applies final geometry.
pub const INITIAL_HIDDEN_WEBVIEW_WIDTH: f64 = 800.0;
pub const INITIAL_HIDDEN_WEBVIEW_HEIGHT: f64 = 600.0;

/// 在主窗口内创建一个子 Webview（方案 D：同窗口多 webview）。
/// 底层已迁移到 tauri-plugin-browser-tabs：全链路 Logical(CSS) 坐标，
/// Linux 下由插件强制触发 WebKitGTK size_allocate，修复子 webview 卡初始尺寸问题。
/// The initial WebView is offscreen rather than hidden with WebKitGTK hide().
/// window.open / target="_blank" 由插件统一拦截为 browser-tabs://event，
/// 再在 main.rs 转发为前端既有的 new-tab-request。
/// label 全局唯一（页签 tab-N / 宫格 grid-N）。
pub fn spawn_child_window(
    app: &AppHandle,
    label: &str,
    url: &str,
    _css_x: f64,
    _css_y: f64,
    css_w: f64,
    css_h: f64,
) -> Result<(), String> {
    use tauri_plugin_browser_tabs::{CreateTabOptions, LogicalRect, TabManagerState};
    if app
        .state::<crate::AppState>()
        .shutdown_requested
        .load(Ordering::Acquire)
    {
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }
    let init_script = include_str!("../../../injected/collect.js");
    let manager = app.state::<TabManagerState>();
    manager
        .create_tab(CreateTabOptions {
            id: label.to_string(),
            url: url.to_string(),
            // Construct offscreen with a real viewport. Do not invoke the
            // plugin's initial webview.hide() path after WebKit starts loading;
            // the caller positions the WebView when the frontend is ready.
            // -30000 is already the proven X11-safe offscreen coordinate.
            rect: LogicalRect::new(
                -30000.0,
                -30000.0,
                css_w.round().max(INITIAL_HIDDEN_WEBVIEW_WIDTH),
                css_h.round().max(INITIAL_HIDDEN_WEBVIEW_HEIGHT),
            ),
            visible: true, // offscreen, not WebKitGTK hide() on a live view
            auto_resize: true,
            user_agent: None,
            transparent: false,
            initialization_script: Some(init_script.to_string()),
        })
        .map_err(|e| format!("创建子 webview 失败: {e}"))?;
    if app
        .state::<crate::AppState>()
        .shutdown_requested
        .load(Ordering::Acquire)
    {
        // The plugin remains the native owner; do not publish a late WebView
        // after the session snapshot was accepted for exit.
        manager
            .close_tab(&label.to_string())
            .map_err(|_| "TAB_CANCELLED_CLEANUP_FAILED".to_string())?;
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }
    eprintln!("[spawn_child_window] 子 webview 已创建 label={}", label);
    Ok(())
}
/// 记住某个子窗口的内容区布局矩形（CSS 坐标），供 move/resize 时重定位。
pub fn remember_layout(app: &AppHandle, id: &str, x: f64, y: f64, w: f64, h: f64) {
    let state = app.state::<crate::AppState>();
    let mut layouts = state.child_layouts.lock().unwrap();
    layouts.insert(id.to_string(), (x, y, w, h));
    let mut revisions = state.layout_revisions.lock().unwrap();
    let revision = revisions.entry(id.to_string()).or_default();
    *revision = revision.wrapping_add(1);
}

/// 归一化用户输入的网址：支持以下写法
/// - 空 -> 默认引导页（百度）
/// - www.baidu.com / baidu.com / example.com -> 自动补 https://
/// - http://... https://... -> 原样
/// - 带路径 baidu.com/s?wd=x -> 自动补 https://
/// - 非 URL 的单词（如 "天气"）-> 走搜索引擎
pub fn normalize_url(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return "https://www.baidu.com".to_string();
    }
    // 已带协议
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("file://") {
        return s.to_string();
    }
    // 含协议分隔但非 http（ftp 等），以及不含 :// 的 about: URL 原样。
    if s.contains("://") || s.starts_with("about:") {
        return s.to_string();
    }
    // 像搜索词（含空格、中文、或不是 域名.后缀 形态）-> 搜索引擎
    let looks_like_domain = s.contains('.')
        && !s.contains(' ')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    if !looks_like_domain {
        // 百度搜索
        return format!("https://www.baidu.com/s?wd={}", urlencoding::encode(s));
    }
    // 裸域名：补 https://
    format!("https://{}", s)
}
/// 把子窗口移出可视区（隐藏态），用于非激活页签/宫格。
/// 注意：不能用 manager.set_visible(false)（即 webview.hide()）——对正在渲染的
/// WebKitGTK 子 webview 调 hide 会阻塞主线程事件循环导致死锁（实测新建第二个
/// 页签时卡在 hide(tab-1)）。改为把子 webview 移到屏幕外（等价隐藏，不卡死）。
pub fn hide_bounds(app: &AppHandle, id: &str) {
    #[cfg(not(target_os = "linux"))]
    use tauri_plugin_browser_tabs::LogicalRect;
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    // 隐藏 = 只移到屏幕外，【保持原尺寸不变】。
    // 关键教训：对正在渲染的 WebKitGTK 子 webview，把尺寸缩到 1x1 会触发 WebKit
    // 视口重布局，与主线程死锁（实测卡死）。只移动位置（视口尺寸不变）则安全。
    // 坐标用 -30000（X11 int16 安全范围 -32768~32767 内）。
    let cur = app
        .state::<crate::AppState>()
        .child_layouts
        .lock()
        .unwrap()
        .get(id)
        .copied();
    // Prefer the plugin's last submitted real viewport, including the
    // initial offscreen geometry when the bridge has no layout record.
    #[cfg(target_os = "linux")]
    {
        match manager.hide_offscreen(&id.to_string(), |rect| {
            remember_layout(app, id, rect.x, rect.y, rect.width, rect.height);
        }) {
            Ok(()) => return,
            Err(e) => {
                eprintln!("[hide_bounds] id={} park failed: {e}", id);
                // Preserve a known hide intent for the next bounded sweep
                // if a same-tab dispatch was temporarily busy.
                if let Some((_x, y, w, h)) = cur {
                    remember_layout(app, id, -30000.0, y, w, h);
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    if let Some((_x, y, w, h)) = cur {
        // 诊断：移出失败（TabNotFound 等）时留日志，排查"切视图后残留"问题
        if let Err(e) = manager.update_rect(&id.to_string(), LogicalRect::new(-30000.0, y, w, h)) {
            eprintln!("[hide_bounds] id={} 移出失败: {e}", id);
        }
        // 记住【隐藏态】坐标（-30000），不是原坐标：
        // 布局守护线程每 400ms 按 child_layouts 重放纠偏，GTK 布局循环会把子 webview
        // 漂回"自然位置"（实测 (0,400,1200,400) 下半屏）。若这里记原坐标，守护线程会
        // 把已隐藏的 webview 拉回可视区造成残留。恢复显示由前端随后重新定位完成。
        remember_layout(app, id, -30000.0, y, w, h);
    } else {
        eprintln!("[hide_bounds] id={} 无布局记录，跳过（可能从未定位过）", id);
    }
}
/// 创建一个浏览器页签（独立子窗口，方案 B），返回其信息并设为激活页签。
pub fn create_tab(app: AppHandle, url: &str) -> Result<TabInfo, String> {
    create_tab_with_completion(app, url, None)
}

pub(crate) fn create_tab_receipted(app: AppHandle, url: &str) -> Result<TabInfo, String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let info = create_tab_with_completion(app, url, Some(tx))?;
    rx.recv().map_err(|_| "TAB_CREATE_CALLBACK_LOST")?;
    Ok(info)
}

fn create_tab_with_completion(
    app: AppHandle, url: &str, completion: Option<std::sync::mpsc::SyncSender<()>>,
) -> Result<TabInfo, String> {
    if app
        .state::<crate::AppState>()
        .shutdown_requested
        .load(Ordering::Acquire)
    {
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }
    let target = normalize_url(url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;

    // 生成唯一 id
    let state0 = app.state::<crate::AppState>();
    let mut counter = state0.tab_counter.lock().unwrap();
    *counter += 1;
    let id = format!("tab-{}", *counter);
    drop(counter);

    let title0 = match Url::parse(&target) {
        Ok(u) => u.host_str().unwrap_or(&target).to_string(),
        Err(_) => target.clone(),
    };

    // 方案 B：异步创建独立子窗口。先在屏幕外预热真实视口，避免
    // 在渲染过程调用 WebKitGTK hide() 或缩至 1x1，再由前端定位。
    spawn_child_window(
        &app,
        &id,
        &target,
        0.0,
        0.0,
        INITIAL_HIDDEN_WEBVIEW_WIDTH,
        INITIAL_HIDDEN_WEBVIEW_HEIGHT,
    )?;
    // Record the initial native offscreen viewport. A hide arriving before
    // the first position now has a concrete target on both platforms.
    remember_layout(
        &app,
        &id,
        -30000.0,
        -30000.0,
        INITIAL_HIDDEN_WEBVIEW_WIDTH,
        INITIAL_HIDDEN_WEBVIEW_HEIGHT,
    );
    eprintln!(
        "[create_tab] 子窗口已提交创建 label={} init=(800x600) offscreen=true",
        id
    );

    let info = TabInfo {
        id: id.clone(),
        url: target.clone(),
        title: title0.clone(),
    };
    let published = {
        // Exit accepts its snapshot while holding this same authoritative lock.
        let mut tabs = state0.tabs.lock().unwrap();
        if state0.shutdown_requested.load(Ordering::Acquire) {
            false
        } else {
            tabs.insert(id.clone(), info.clone());
            true
        }
    };
    if !published {
        use tauri_plugin_browser_tabs::TabManagerState;
        app.state::<TabManagerState>()
            .close_tab(&id)
            .map_err(|_| "TAB_CANCELLED_CLEANUP_FAILED".to_string())?;
        state0.child_layouts.lock().unwrap().remove(&id);
        state0.layout_revisions.lock().unwrap().remove(&id);
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }
    *app.state::<crate::AppState>().active_tab.lock().unwrap() = Some(id.clone());
    // 休眠计时：新页签激活，旧页签从 now 起算 idle
    {
        let state = app.state::<crate::AppState>();
        let mut idle = state.tab_idle_since.lock().unwrap();
        idle.remove(&id);
        let now = std::time::Instant::now();
        for k in state.tabs.lock().unwrap().keys() {
            if *k != id {
                idle.entry(k.clone()).or_insert(now);
            }
        }
    }

    // 非激活页签异步隐藏（必须走主线程，且不能在当前 command 同步等待，否则与
    // 队列里的 webview build 互相等待死锁）。这里用 run_on_main_thread 排队执行。
    let app_hide = app.clone();
    let active_id = id.clone();
    let wait_for_completion = completion.is_some();
    let dispatch = app.run_on_main_thread(move || {
        let complete = || { if let Some(tx) = completion { let _ = tx.send(()); } };
        // A superseded hide intent is a finished host callback, not a new effect.
        let hide = || {
        let st = app_hide.state::<crate::AppState>();
        if st.shutdown_requested.load(Ordering::Acquire)
            || st.active_tab.lock().unwrap().as_deref() != Some(active_id.as_str())
        {
            return;
        }
        let ids: Vec<String> = st.tabs.lock().unwrap().keys().cloned().collect();
        for k in ids {
            if st.active_tab.lock().unwrap().as_deref() != Some(active_id.as_str()) {
                return;
            }
            if k != active_id {
                hide_bounds(&app_hide, &k);
            }
        }
        };
        hide();
        complete();
    });
    if wait_for_completion { dispatch.map_err(|_| "TAB_CREATE_CALLBACK_LOST")?; }
    eprintln!("[create_tab] run_on_main_thread 已排队 label={}", id);

    eprintln!("[create_tab] tab created label={}", id);
    Ok(info)
}

// ---------------------------------------------------------------------------
// Native Physical Boundary closeout: resource_collection commands migrated from bridge.rs.
// These are browser tab/resource-waterfall inspection + capture settings + browser-resources
// event reporting — i.e. Browser subdomain, not an independent capability (see closeout).
// Owner reconciled to `browser`; no allowed_callers, no second truth.
use crate::invocation::{check_invocation_source, check_tab_id};
use crate::workspace;
use crate::AppState;

/// 查询某 tab 的资源瀑布记录（含容量驱逐计数）。
#[tauri::command]
pub fn list_tab_resources(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
) -> Result<TabResourceList, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "list_tab_resources")?;
    crate::shared::native_legacy_policy::bound_text("list_tab_resources", "tab_id", &tab_id)?;

    check_invocation_source(&webview, "list_tab_resources", None, &app)?;
    check_tab_id(&tab_id)?;
    let state = app.state::<AppState>();
    let enabled = state.resource_capture.lock().unwrap().enabled;
    let buf = state.resource_buffer.lock().unwrap();
    Ok(TabResourceList {
        records: buf.records(&tab_id),
        evicted: buf.evicted_of(&tab_id),
        enabled,
    })
}

/// 清空某 tab 的资源瀑布记录。审计只记 tab_id 与计数，不记任何 URL。
#[tauri::command]
pub fn clear_tab_resources(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "clear_tab_resources")?;
    crate::shared::native_legacy_policy::bound_text("clear_tab_resources", "tab_id", &tab_id)?;

    check_invocation_source(&webview, "clear_tab_resources", None, &app)?;
    check_tab_id(&tab_id)?;
    let removed = app
        .state::<AppState>()
        .resource_buffer
        .lock()
        .unwrap()
        .clear_tab(&tab_id);
    workspace::log_audit(
        &app,
        "resource_clear",
        format!("tab_id={tab_id} removed={removed}"),
    );
    Ok(())
}

/// 查询资源采集设置。
#[tauri::command]
pub fn get_resource_capture_settings(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<ResourceCaptureSettings, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "get_resource_capture_settings")?;

    check_invocation_source(&webview, "get_resource_capture_settings", None, &app)?;
    Ok(app
        .state::<AppState>()
        .resource_capture
        .lock()
        .unwrap()
        .clone())
}

/// 设置资源采集开关与每 tab 容量（会话内生效，不持久化；max_total/url 上限
/// 不开放调整，防绕过容量红线）。开启/关闭/调整均写审计（不含 URL）。
#[tauri::command]
pub fn set_resource_capture_settings(
    app: AppHandle,
    webview: tauri::Webview,
    enabled: bool,
    max_per_tab: Option<usize>,
) -> Result<ResourceCaptureSettings, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "set_resource_capture_settings")?;

    check_invocation_source(&webview, "set_resource_capture_settings", None, &app)?;
    // max_per_tab 合法区间 [10, 1000]：过小无意义，过大失去容量保护
    let clamped = max_per_tab.map(|n| n.clamp(10, 1000));
    let snapshot = {
        let state = app.state::<AppState>();
        let mut s = state.resource_capture.lock().unwrap();
        s.enabled = enabled;
        if let Some(n) = clamped {
            s.max_per_tab = n.min(s.max_total);
        }
        s.clone()
    };
    workspace::log_audit(
        &app,
        "resource_capture",
        format!("enabled={enabled} max_per_tab={:?}", snapshot.max_per_tab),
    );
    Ok(snapshot)
}

/// M0-3.b：远程上报入口的统一来源校验。未登记/伪造 label（含残留的 `browser`）一律拒绝。
#[tauri::command]
pub fn report_resources(
    app: AppHandle,
    webview: tauri::Webview,
    page_url: String,
    items: Vec<ResourceItem>,
) -> Result<(), String> {
    use crate::security_policy as sp;
    // 上报类命令无副作用，只做来源校验与载荷边界（防事件洪水与内存放大）。
    check_invocation_source(&webview, "report_resources", None, &app)?;
    sp::check_text_field("page_url", &page_url, sp::MAX_TEXT_FIELD_BYTES)
        .map_err(|e| e.to_string())?;
    sp::check_items_count(items.len(), sp::MAX_RESOURCE_ITEMS).map_err(|e| e.to_string())?;
    let _ = app.emit(
        "browser-resources",
        serde_json::json!({ "page_url": page_url, "items": items }),
    );
    Ok(())
}

// One native capture owner replaces the former IPC-waiter-owned lane.
// It retains admission through engine callback, encoding and publication.
// A disconnected caller is never evidence of renderer termination.
use tauri_plugin_browser_tabs::{CaptureCancellation, CaptureDocument, TabManagerState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum CapturePhase { Running, CancelRequested, Succeeded, Failed, Cancelled, Unknown }
impl CapturePhase {
    fn terminal(self) -> bool { matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled) }
}
#[derive(Clone, serde::Serialize)]
struct CaptureReceipt {
    ticket: String,
    tab_id: String,
    phase: CapturePhase,
    path: Option<String>,
    error: Option<String>,
}
struct CaptureSlot { receipt: CaptureReceipt, cancellation: CaptureCancellation }
struct CaptureOwner {
    epoch: String,
    next_sequence: u64,
    slot: Option<CaptureSlot>,
    last_completed: Option<CaptureReceipt>,
}
static CAPTURE_OWNER: std::sync::LazyLock<std::sync::Mutex<CaptureOwner>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(CaptureOwner {
        epoch: uuid::Uuid::new_v4().to_string(), next_sequence: 1, slot: None, last_completed: None,
    }));
#[derive(serde::Serialize)]
struct CaptureView {
    epoch: String,
    next_sequence: String,
    receipt: Option<CaptureReceipt>,
    last_completed: Option<CaptureReceipt>,
}
fn capture_view() -> Result<CaptureView, String> {
    let owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
    Ok(CaptureView {
        epoch: owner.epoch.clone(), next_sequence: owner.next_sequence.to_string(),
        receipt: owner.slot.as_ref().map(|slot| slot.receipt.clone()),
        last_completed: owner.last_completed.clone(),
    })
}
fn capture_notify(app: &AppHandle, receipt: CaptureReceipt) {
    let _ = app.emit_to("main", "browser-capture-receipt", receipt);
}
fn capture_finish(app: &AppHandle, ticket: &str, phase: CapturePhase, result: Result<String, String>) {
    let receipt = CAPTURE_OWNER.lock().ok().and_then(|mut owner| {
        let slot = owner.slot.as_mut().filter(|slot| slot.receipt.ticket == ticket)?;
        if slot.receipt.phase.terminal() { return None; }
        slot.receipt.phase = phase;
        match result {
            Ok(path) => slot.receipt.path = Some(path),
            Err(error) => slot.receipt.error = Some(error),
        }
        Some(slot.receipt.clone())
    });
    if let Some(receipt) = receipt { capture_notify(app, receipt); }
}
struct CaptureWorkerGuard { app: AppHandle, ticket: String, completed: bool }
impl Drop for CaptureWorkerGuard {
    fn drop(&mut self) {
        if !self.completed {
            capture_finish(&self.app, &self.ticket, CapturePhase::Unknown, Err("CAPTURE_WORKER_OUTCOME_UNKNOWN".into()));
        }
    }
}
fn capture_cancel(app: &AppHandle, ticket: &str) -> Result<(), String> {
    let (cancellation, receipt) = {
        let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
        let slot = owner.slot.as_mut().filter(|slot| slot.receipt.ticket == ticket)
            .ok_or("CAPTURE_STALE_TICKET")?;
        if slot.receipt.phase.terminal() { return Ok(()); }
        if slot.receipt.phase == CapturePhase::Running {
            slot.receipt.phase = CapturePhase::CancelRequested;
        }
        (slot.cancellation.clone(), slot.receipt.clone())
    };
    cancellation.cancel();
    capture_notify(app, receipt);
    Ok(())
}
fn capture_ack(ticket: &str) -> Result<(), String> {
    let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
    if owner.last_completed.as_ref().is_some_and(|receipt| receipt.ticket == ticket) { return Ok(()); }
    let slot = owner.slot.as_ref().filter(|slot| slot.receipt.ticket == ticket).ok_or("CAPTURE_STALE_TICKET")?;
    if !slot.receipt.phase.terminal() { return Err("CAPTURE_NOT_TERMINAL".into()); }
    // An acknowledgement also fences any delayed start which observed the
    // sequence while the old terminal slot was still present.
    owner.next_sequence = owner.next_sequence.checked_add(1).ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
    owner.last_completed = owner.slot.take().map(|slot| slot.receipt);
    Ok(())
}
async fn run_capture(app: AppHandle, id: String, ticket: String, document: CaptureDocument, cancellation: CaptureCancellation) {
    let mut guard = CaptureWorkerGuard { app: app.clone(), ticket: ticket.clone(), completed: false };
    let manager = app.state::<TabManagerState>();
    if cancellation.is_cancelled() {
        capture_finish(&app, &ticket, CapturePhase::Cancelled, Err("CAPTURE_CANCELLED_BEFORE_DISPATCH".into()));
        guard.completed = true;
        return;
    }
    let (send, mut recv) = tokio::sync::oneshot::channel();
    let _dispatch = manager.capture_bound_document(&id, document.clone(), cancellation.clone(), move |result| {
        let _ = send.send(result);
    });
    // Even a dispatch error must not discard a callback which may already
    // have been queued. A closed reply channel without a receipt is UNKNOWN.
    drop(manager);
    let native = match tokio::time::timeout(std::time::Duration::from_secs(30), &mut recv).await {
        Ok(value) => value,
        Err(_) => {
            let _ = capture_cancel(&app, &ticket);
            // Timeout is a request to stop, not a completed operation.
            // Windows CDP may return much later; hold the same owner.
            recv.await
        }
    };
    let Ok(result) = native else { return; };
    if cancellation.is_cancelled() {
        capture_finish(&app, &ticket, CapturePhase::Cancelled, Err("CAPTURE_CANCELLED_AT_COMPLETION".into()));
        guard.completed = true;
        return;
    }
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => {
            capture_finish(&app, &ticket, CapturePhase::Failed, Err(error));
            guard.completed = true;
            return;
        }
    };
    if !app.state::<TabManagerState>().is_current_capture_document(&id, &document) {
        capture_finish(&app, &ticket, CapturePhase::Failed, Err("CAPTURE_DOCUMENT_STALE".into()));
        guard.completed = true;
        return;
    }
    let base = crate::workspace::data_dir(&app);
    let storage = tauri::async_runtime::spawn_blocking(move || save_capture(&base, &bytes)).await;
    // A cancel arriving during publication cannot erase a file that was
    // actually committed. Report the publication outcome faithfully.
    match storage {
        Ok(Ok(path)) => capture_finish(&app, &ticket, CapturePhase::Succeeded, Ok(path)),
        Ok(Err(error)) => capture_finish(&app, &ticket, CapturePhase::Failed, Err(error)),
        Err(_) => return,
    }
    guard.completed = true;
}
fn begin_capture(app: AppHandle, id: String, ticket: String, epoch: &str, sequence: &str) -> Result<(), String> {
    crate::shared::invocation::check_tab_id(&id)?;
    if ticket.len() != 36 || uuid::Uuid::parse_str(&ticket).map_or(true, |id| id.get_version_num() != 4) {
        return Err("CAPTURE_INVALID_TICKET".into());
    }
    if app.state::<AppState>().shutdown_requested.load(Ordering::Acquire) {
        return Err("CAPTURE_APPLICATION_SHUTTING_DOWN".into());
    }
    let document = app.state::<TabManagerState>().capture_document(&id)
        .map_err(|_| "CAPTURE_TAB_UNAVAILABLE")?;
    let cancellation = CaptureCancellation::new();
    {
        let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
        if epoch != owner.epoch { return Err("CAPTURE_STALE_EPOCH".into()); }
        if let Some(slot) = &owner.slot {
            if slot.receipt.ticket == ticket && slot.receipt.tab_id == id { return Ok(()); }
            return Err("CAPTURE_BUSY".into());
        }
        if sequence != owner.next_sequence.to_string() { return Err("CAPTURE_STALE_SEQUENCE".into()); }
        owner.next_sequence = owner.next_sequence.checked_add(1).ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
        owner.slot = Some(CaptureSlot {
            receipt: CaptureReceipt { ticket: ticket.clone(), tab_id: id.clone(), phase: CapturePhase::Running,
                path: None, error: None },
            cancellation: cancellation.clone(),
        });
    }
    // Spawn independent work before returning the receipt. No callback needs
    // the original invoke future or its frontend to continue.
    let _ = tauri::async_runtime::spawn(run_capture(app, id, ticket, document, cancellation));
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum CaptureRequest {
    Inspect,
    Fence { epoch: String, sequence: String },
    CancelPending { id: String, ticket: String, epoch: String, sequence: String },
    Start { id: String, ticket: String, epoch: String, sequence: String },
    Cancel { ticket: String },
    Ack { ticket: String },
}

/// Main-only, native current-document capture. Old callers keep the string
/// return; the optional task protocol exposes cancellation and terminal
/// receipts through this same permission/command, not a second capture lane.
#[tauri::command]
pub async fn capture_full_page(
    webview: tauri::Webview,
    app: AppHandle,
    id: Option<String>,
    request: Option<CaptureRequest>,
) -> Result<serde_json::Value, String> {
    if webview.label() != "main" { return Err("CAPTURE_MAIN_ONLY".into()); }
    if let Some(request) = request {
        match request {
            CaptureRequest::Inspect => {}
            CaptureRequest::Fence { epoch, sequence } => {
                let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
                if owner.slot.is_some() { return Err("CAPTURE_BUSY".into()); }
                if epoch != owner.epoch { return Err("CAPTURE_STALE_EPOCH".into()); }
                if sequence != owner.next_sequence.to_string() { return Err("CAPTURE_STALE_SEQUENCE".into()); }
                owner.next_sequence = owner.next_sequence.checked_add(1).ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
            }
            CaptureRequest::CancelPending { id, ticket, epoch, sequence } => {
                crate::shared::invocation::check_tab_id(&id)?;
                if ticket.len() != 36 || uuid::Uuid::parse_str(&ticket).map_or(true, |id| id.get_version_num() != 4) {
                    return Err("CAPTURE_INVALID_TICKET".into());
                }
                let cancelled_before_start = {
                    let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
                    if epoch != owner.epoch { return Err("CAPTURE_STALE_EPOCH".into()); }
                    if owner.last_completed.as_ref().is_some_and(|r| r.ticket == ticket) { true }
                    else if let Some(slot) = &owner.slot {
                        if slot.receipt.ticket != ticket || slot.receipt.tab_id != id { return Err("CAPTURE_BUSY".into()); }
                        false
                    } else {
                        if sequence != owner.next_sequence.to_string() { return Err("CAPTURE_STALE_SEQUENCE".into()); }
                        owner.next_sequence = owner.next_sequence.checked_add(1).ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
                        owner.last_completed = Some(CaptureReceipt { ticket: ticket.clone(), tab_id: id,
                            phase: CapturePhase::Cancelled, path: None, error: Some("CAPTURE_CANCELLED_BEFORE_DISPATCH".into()) });
                        true
                    }
                };
                if !cancelled_before_start { capture_cancel(&app, &ticket)?; }
            }
            CaptureRequest::Start { id, ticket, epoch, sequence } =>
                begin_capture(app.clone(), id, ticket, &epoch, &sequence)?,
            CaptureRequest::Cancel { ticket } => capture_cancel(&app, &ticket)?,
            CaptureRequest::Ack { ticket } => capture_ack(&ticket)?,
        }
        return serde_json::to_value(capture_view()?).map_err(|_| "CAPTURE_RECEIPT_FAILED".into());
    }
    let id = id.ok_or("CAPTURE_TAB_UNAVAILABLE")?;
    let view = capture_view()?;
    let ticket = uuid::Uuid::new_v4().to_string();
    begin_capture(app.clone(), id, ticket.clone(), &view.epoch, &view.next_sequence)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let current = capture_view()?;
        let receipt = current.receipt.filter(|receipt| receipt.ticket == ticket)
            .or_else(|| current.last_completed.filter(|receipt| receipt.ticket == ticket))
            .ok_or("CAPTURE_OUTCOME_UNKNOWN")?;
        if receipt.phase.terminal() {
            capture_ack(&ticket)?;
            return receipt.path.map(serde_json::Value::String)
                .ok_or_else(|| receipt.error.unwrap_or_else(|| "CAPTURE_FAILED".into()));
        }
        if std::time::Instant::now() >= deadline {
            let _ = capture_cancel(&app, &ticket);
            return Err("CAPTURE_OUTCOME_UNKNOWN".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
}

fn save_capture(base: &std::path::Path, bytes: &[u8]) -> Result<String, String> {
    const MAX_BYTES: usize = 32 * 1024 * 1024;
    if bytes.len() < 24 || bytes.len() > MAX_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("CAPTURE_INVALID_PNG".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap_or([0; 4]));
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap_or([0; 4]));
    if width == 0
        || height == 0
        || width > 16_384
        || height > 16_384
        || u64::from(width) * u64::from(height) > 16_000_000
    {
        return Err("CAPTURE_DIMENSION_LIMIT".into());
    }
    use crate::capabilities::workspace::safe_paths;
    let roots = vec![base.to_path_buf()];
    let directory = base.join("screenshots");
    safe_paths::create_directory_all(&directory, &roots)
        .map_err(|_| "CAPTURE_STORAGE_UNAVAILABLE")?;
    safe_paths::ensure_private_directory(&directory).map_err(|_| "CAPTURE_STORAGE_UNAVAILABLE")?;
    let file = directory.join(format!("webpage-{}.png", uuid::Uuid::new_v4()));
    safe_paths::create_new_file(&file, &roots, bytes).map_err(|_| "CAPTURE_SAVE_FAILED")?;
    Ok(file.to_string_lossy().into_owned())
}

#[cfg(test)]
mod capture_storage_tests {
    use super::*;

    fn image_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    #[test]
    fn cancellation_request_and_unknown_do_not_release_capture_owner() {
        assert!(!CapturePhase::Running.terminal());
        assert!(!CapturePhase::CancelRequested.terminal());
        assert!(!CapturePhase::Unknown.terminal());
        assert!(CapturePhase::Cancelled.terminal());
        assert!(CapturePhase::Succeeded.terminal());
        assert!(CapturePhase::Failed.terminal());
    }

    #[test]
    fn rejects_oversized_images_before_creating_storage() {
        let base = std::env::temp_dir().join(format!("browseros-capture-{}", uuid::Uuid::new_v4()));
        assert_eq!(
            save_capture(&base, &image_header(16_385, 1)),
            Err("CAPTURE_DIMENSION_LIMIT".into())
        );
        assert_eq!(
            save_capture(&base, &image_header(10_000, 10_000)),
            Err("CAPTURE_DIMENSION_LIMIT".into())
        );
        assert!(!base.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn screenshot_storage_rejects_symlink_directory() {
        let base = std::env::temp_dir().join(format!("browseros-capture-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&base).unwrap();
        let elsewhere = base.join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, base.join("screenshots")).unwrap();
        assert!(save_capture(&base, &image_header(2, 2)).is_err());
        assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0);
        std::fs::remove_dir_all(base).unwrap();
    }
}
