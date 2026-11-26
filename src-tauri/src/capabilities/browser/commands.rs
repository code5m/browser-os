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
    if crate::bridge::isolated_tabs::route(label).is_some() {
        return Err("TAB_RENDERER_EMBEDDED_FALLBACK_DENIED".into());
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
    if crate::bridge::isolated_tabs::mode_enabled() {
        // Synchronous embedded-only paths cannot silently bypass the
        // experimental process backend or block the UI on renderer IPC.
        return Err("TAB_RENDERER_REQUIRES_ADMITTED_WORKER".into());
    }
    create_tab_with_completion(app, url, None)
}

pub(crate) fn create_tab_receipted(app: AppHandle, url: &str) -> Result<TabInfo, String> {
    if crate::bridge::isolated_tabs::mode_enabled() {
        return Err("TAB_RENDERER_REQUIRES_ADMITTED_WORKER".into());
    }
    use crate::shared::dispatch_receipt::{DispatchReceipt, WaitError};
    let completion = std::sync::Arc::new(DispatchReceipt::new()?);
    let info = create_tab_with_completion(app, url, Some(completion.clone()))?;
    completion
        .wait(std::time::Duration::from_secs(20))
        .map_err(|error| match error {
            // The tab metadata may already exist. This is a terminal partial
            // failure requiring authoritative tab-list reconciliation, not a
            // claim that no tab was created.
            WaitError::Revoked => "TAB_CREATE_LAYOUT_REVOKED",
            WaitError::Lost => "TAB_CREATE_CALLBACK_LOST",
        })?;
    Ok(info)
}

fn create_tab_with_completion(
    app: AppHandle,
    url: &str,
    completion: Option<std::sync::Arc<crate::shared::dispatch_receipt::DispatchReceipt<()>>>,
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
    let dispatch_receipt = completion.clone();
    let dispatch = app.run_on_main_thread(move || {
        let entered = match completion.as_ref() {
            Some(receipt) => match receipt.enter() {
                Some(entered) => Some(entered),
                None => return, // revoked: never touch the native views
            },
            None => None,
        };
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
        if let Some(entered) = entered {
            entered.complete(());
        }
    });
    if dispatch.is_err() {
        // A rejected dispatch may still be queued. Revoke only before entry;
        // if already entered, the worker must wait for its real receipt.
        if let Some(receipt) = dispatch_receipt {
            receipt.revoke_pending();
        }
    }
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
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if let Some(index) = std::env::var("GRID_CHILD_INDEX").ok().and_then(|v| v.parse::<u32>().ok()) {
        let id = webview.label();
        if !crate::grid_ipc::is_tab_renderer(index) || crate::grid_ipc::renderer_label(index) != id {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let actual = webview.url().map_err(|_| "TAB_RENDERER_DOCUMENT_UNAVAILABLE")?.to_string();
        if actual != page_url || actual.len() > 8192 {
            return Err("TAB_RENDERER_DOCUMENT_STALE".into());
        }
        let mut budget = 96 * 1024usize;
        let items: Vec<_> = items.into_iter().filter(|item|
            item.res_type.len() <= 32 && item.absolute.len() <= 8192
            && (item.absolute.starts_with("http://") || item.absolute.starts_with("https://")))
            .take(100)
            .filter_map(|mut item| {
                item.absolute = sp::redact_sensitive_url(&item.absolute);
                item.url = item.absolute.clone();
                let cost = item.absolute.len().saturating_mul(2).saturating_add(item.res_type.len() + 64);
                if cost > budget { return None; }
                budget -= cost;
                Some(item)
            }).collect();
        crate::enqueue_renderer_report("browser-resources", serde_json::json!({
            "id": id, "page_url": actual, "items": items
        }));
        return Ok(());
    }
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
enum CapturePhase {
    Running,
    CancelRequested,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}
impl CapturePhase {
    fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    #[default]
    CurrentDocument,
    IndependentAddress,
    InteractiveAddress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum InteractiveStage {
    Opening,
    AwaitingUser,
    CaptureRequested,
}

#[derive(Clone, serde::Serialize)]
struct CaptureReceipt {
    ticket: String,
    tab_id: String,
    mode: CaptureMode,
    phase: CapturePhase,
    path: Option<String>,
    error: Option<String>,
}
struct CaptureSlot {
    receipt: CaptureReceipt,
    cancellation: CaptureCancellation,
    renderer_binding: Option<crate::grid_ipc::RendererBinding>,
    since: std::time::Instant,
    // A live native callback return is not a shared renderer exit receipt.
    native_returned: bool,
    publishing: bool,
    // Cancellation is committed under the owner mutex, even when the
    // receipt must remain Unknown until the original renderer retires.
    cancel_intent: bool,
    interactive_stage: Option<InteractiveStage>,
}

const CAPTURE_DOMAIN_STALL: std::time::Duration = std::time::Duration::from_secs(60);
const CAPTURE_RECOVERY_TTL: std::time::Duration = std::time::Duration::from_secs(15);

#[derive(Clone, Debug, PartialEq, Eq)]
struct CaptureRecoveryTarget {
    epoch: String,
    sequence: u64,
    ticket: String,
    renderer_only: bool,
}

fn capture_recovery_scope_matches(requested: Option<&str>, renderer_only: bool) -> bool {
    match requested {
        Some("renderer") => renderer_only,
        Some("application") | None => !renderer_only,
        _ => false,
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureDomainRecoveryIntent {
    target: CaptureRecoveryTarget,
    issued: std::time::Instant,
}

struct CaptureOwner {
    epoch: String,
    next_sequence: u64,
    slot: Option<CaptureSlot>,
    last_completed: Option<CaptureReceipt>,
    recovery_intent: Option<CaptureDomainRecoveryIntent>,
    recovery_claimed: bool,
}
impl CaptureOwner {
    fn recovery_target(&self, now: std::time::Instant) -> Option<CaptureRecoveryTarget> {
        let slot = self.slot.as_ref()?;
        if self.recovery_claimed
            || slot.native_returned
            || slot.publishing
            || slot.receipt.phase.terminal()
        {
            return None;
        }
        let eligible = match slot.receipt.mode {
            CaptureMode::CurrentDocument if slot.renderer_binding.is_some() => {
                slot.receipt.phase == CapturePhase::Unknown
            }
            CaptureMode::CurrentDocument => slot.cancellation.native_entered_at().is_some_and(|entered| {
                slot.receipt.phase == CapturePhase::Unknown
                    || now.saturating_duration_since(entered) >= CAPTURE_DOMAIN_STALL
            }),
            // Only the independent process owner reports this retirement
            // uncertainty. Lost frontend IPC alone is not authority.
            CaptureMode::IndependentAddress | CaptureMode::InteractiveAddress => {
                slot.receipt.phase == CapturePhase::Unknown
                    && slot.receipt.error.as_deref() == Some("CAPTURE_ISOLATED_EXIT_PENDING")
            }
        };
        eligible.then(|| CaptureRecoveryTarget {
            epoch: self.epoch.clone(),
            sequence: self.next_sequence,
            ticket: slot.receipt.ticket.clone(),
            renderer_only: slot.renderer_binding.is_some(),
        })
    }

    fn valid_intent(&self, intent: &CaptureDomainRecoveryIntent, now: std::time::Instant) -> bool {
        now.saturating_duration_since(intent.issued) < CAPTURE_RECOVERY_TTL
            && self.recovery_target(now).as_ref() == Some(&intent.target)
    }

    fn request_recovery(
        &mut self,
        target: CaptureRecoveryTarget,
        now: std::time::Instant,
    ) -> Result<(), String> {
        if self.recovery_target(now).as_ref() != Some(&target) {
            return Err("CAPTURE_DOMAIN_RECOVERY_NOT_ELIGIBLE".into());
        }
        if self.recovery_intent.as_ref().is_some_and(|intent| self.valid_intent(intent, now)) {
            return Ok(());
        }
        self.recovery_intent = Some(CaptureDomainRecoveryIntent { target, issued: now });
        Ok(())
    }

    fn claim_recovery(&mut self, intent: &CaptureDomainRecoveryIntent, now: std::time::Instant) -> bool {
        if self.recovery_intent.as_ref() != Some(intent) || !self.valid_intent(intent, now) {
            return false;
        }
        self.recovery_intent = None;
        self.recovery_claimed = true;
        true
    }

    fn interactive_slot(&mut self, ticket: &str) -> Result<&mut CaptureSlot, String> {
        if self.recovery_claimed {
            return Err("CAPTURE_ISOLATED_CANCELLED".into());
        }
        let slot = self.slot.as_mut()
            .filter(|slot| slot.receipt.ticket == ticket
                && slot.receipt.mode == CaptureMode::InteractiveAddress)
            .ok_or("CAPTURE_STALE_TICKET")?;
        if slot.cancel_intent || slot.cancellation.is_cancelled()
            || slot.receipt.phase != CapturePhase::Running || slot.native_returned
        {
            return Err("CAPTURE_ISOLATED_CANCELLED".into());
        }
        Ok(slot)
    }

    fn mark_interactive_ready(&mut self, ticket: &str) -> Result<(), String> {
        let slot = self.interactive_slot(ticket)?;
        if slot.interactive_stage != Some(InteractiveStage::Opening) {
            return Err("CAPTURE_INTERACTIVE_STATE_INVALID".into());
        }
        slot.interactive_stage = Some(InteractiveStage::AwaitingUser);
        Ok(())
    }

    fn request_interactive_capture(
        &mut self,
        epoch: &str,
        sequence: &str,
        ticket: &str,
    ) -> Result<(), String> {
        if self.epoch != epoch {
            return Err("CAPTURE_STALE_EPOCH".into());
        }
        if self.next_sequence.to_string() != sequence {
            return Err("CAPTURE_STALE_SEQUENCE".into());
        }
        let slot = self.interactive_slot(ticket)?;
        match slot.interactive_stage {
            Some(InteractiveStage::AwaitingUser) => {
                slot.interactive_stage = Some(InteractiveStage::CaptureRequested);
                Ok(())
            }
            // A lost control reply may be inspected or repeated without
            // dispatching a second renderer request.
            Some(InteractiveStage::CaptureRequested) => Ok(()),
            _ => Err("CAPTURE_INTERACTIVE_NOT_READY".into()),
        }
    }

    fn interactive_capture_requested(&mut self, ticket: &str) -> Result<bool, String> {
        let slot = self.interactive_slot(ticket)?;
        Ok(slot.interactive_stage == Some(InteractiveStage::CaptureRequested))
    }

    fn mark_returned(&mut self, ticket: &str) -> Result<(), String> {
        let slot = self.slot.as_mut().filter(|slot| slot.receipt.ticket == ticket)
            .ok_or("CAPTURE_STALE_TICKET")?;
        if slot.receipt.phase.terminal() || slot.native_returned {
            return Err("CAPTURE_NOT_RUNNING".into());
        }
        slot.native_returned = true;
        self.recovery_intent = None;
        Ok(())
    }

    fn reserve_publication(&mut self, ticket: &str) -> Result<(), String> {
        let slot = self.slot.as_mut().filter(|slot| slot.receipt.ticket == ticket)
            .ok_or("CAPTURE_STALE_TICKET")?;
        if self.recovery_claimed || slot.cancel_intent
            || slot.receipt.phase == CapturePhase::CancelRequested
            || slot.cancellation.is_cancelled()
        {
            return Err("CAPTURE_CANCELLED_BEFORE_PUBLICATION".into());
        }
        if !slot.native_returned || slot.publishing || slot.receipt.phase.terminal() {
            return Err("CAPTURE_PUBLICATION_NOT_ADMITTED".into());
        }
        slot.publishing = true;
        Ok(())
    }
}

static CAPTURE_OWNER: std::sync::LazyLock<std::sync::Mutex<CaptureOwner>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(CaptureOwner {
            epoch: uuid::Uuid::new_v4().to_string(),
            next_sequence: 1,
            slot: None,
            last_completed: None,
            recovery_intent: None,
            recovery_claimed: false,
        })
    });
#[derive(serde::Serialize)]
struct CaptureView {
    epoch: String,
    next_sequence: String,
    receipt: Option<CaptureReceipt>,
    last_completed: Option<CaptureReceipt>,
    domain_recovery_available: bool,
    recovery_scope: Option<&'static str>,
    publishing: bool,
    interactive_stage: Option<InteractiveStage>,
}
/// Non-blocking metadata only: no document, image or filesystem path.
pub(crate) fn capture_diagnostic() -> serde_json::Value {
    match CAPTURE_OWNER.try_lock() {
        Ok(owner) => match &owner.slot {
            Some(slot) => {
                serde_json::json!({ "ticket": slot.receipt.ticket, "phase": slot.receipt.phase,
                "age_ms": slot.since.elapsed().as_millis().min(u64::MAX as u128) as u64,
                "cancel_requested": slot.cancel_intent || slot.cancellation.is_cancelled(),
                "native_returned": slot.native_returned, "publishing": slot.publishing })
            }
            None => serde_json::json!({"phase":"idle"}),
        },
        Err(_) => serde_json::json!({"phase":"owner_unavailable"}),
    }
}
fn capture_view() -> Result<CaptureView, String> {
    let owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
    let recovery = owner.recovery_target(std::time::Instant::now())
        .filter(|target| target.renderer_only || capture_supervisor_ready());
    Ok(CaptureView {
        epoch: owner.epoch.clone(),
        next_sequence: owner.next_sequence.to_string(),
        receipt: owner.slot.as_ref().map(|slot| slot.receipt.clone()),
        last_completed: owner.last_completed.clone(),
        domain_recovery_available: recovery.is_some(),
        recovery_scope: recovery.as_ref().map(|target| {
            if target.renderer_only { "renderer" } else { "application" }
        }),
        publishing: owner.slot.as_ref().is_some_and(|slot| slot.publishing),
        interactive_stage: owner.slot.as_ref()
            .filter(|slot| slot.receipt.phase == CapturePhase::Running && !slot.native_returned)
            .and_then(|slot| slot.interactive_stage),
    })
}

fn capture_supervisor_ready() -> bool {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        crate::native_supervisor::domain_recovery_ready()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

pub(crate) fn pending_capture_domain_recovery() -> Option<CaptureDomainRecoveryIntent> {
    let mut owner = CAPTURE_OWNER.try_lock().ok()?;
    let intent = owner.recovery_intent.clone()?;
    if owner.valid_intent(&intent, std::time::Instant::now()) {
        Some(intent)
    } else {
        owner.recovery_intent = None;
        None
    }
}

pub(crate) struct CaptureDomainRecoveryClaim;
impl Drop for CaptureDomainRecoveryClaim {
    fn drop(&mut self) {
        if let Ok(mut owner) = CAPTURE_OWNER.lock() {
            owner.recovery_claimed = false;
        }
    }
}

pub(crate) fn claim_capture_domain_recovery(
    intent: &CaptureDomainRecoveryIntent,
) -> Option<CaptureDomainRecoveryClaim> {
    let mut owner = CAPTURE_OWNER.try_lock().ok()?;
    if !owner.claim_recovery(intent, std::time::Instant::now()) {
        return None;
    }
    drop(owner);
    Some(CaptureDomainRecoveryClaim)
}

fn capture_returned(ticket: &str) -> Result<(), String> {
    CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?.mark_returned(ticket)
}

fn capture_publish(app: &AppHandle, ticket: &str, bytes: &[u8]) -> Result<String, String> {
    {
        let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
        if app.state::<crate::ExitLifecycleState>().started.load(Ordering::Acquire) {
            return Err("CAPTURE_CANCELLED_BEFORE_PUBLICATION".into());
        }
        owner.reserve_publication(ticket)?;
    }
    // Never hold the native owner across filesystem I/O.
    save_capture(&crate::workspace::data_dir(app), bytes)
}

fn capture_notify(app: &AppHandle, receipt: CaptureReceipt) {
    let _ = app.emit_to("main", "browser-capture-receipt", receipt);
}
fn capture_finish(
    app: &AppHandle,
    ticket: &str,
    phase: CapturePhase,
    result: Result<String, String>,
) {
    let receipt = CAPTURE_OWNER.lock().ok().and_then(|mut owner| {
        let slot = owner
            .slot
            .as_mut()
            .filter(|slot| slot.receipt.ticket == ticket)?;
        if slot.receipt.phase.terminal() {
            return None;
        }
        slot.receipt.phase = phase;
        if phase.terminal() {
            slot.publishing = false;
        }
        match result {
            Ok(path) => {
                slot.receipt.path = Some(path);
                slot.receipt.error = None;
            }
            Err(error) => {
                slot.receipt.error = Some(error);
                slot.receipt.path = None;
            }
        }
        Some(slot.receipt.clone())
    });
    if let Some(receipt) = receipt {
        capture_notify(app, receipt);
    }
}
struct CaptureWorkerGuard {
    app: AppHandle,
    ticket: String,
    completed: bool,
}
impl Drop for CaptureWorkerGuard {
    fn drop(&mut self) {
        if !self.completed {
            capture_finish(
                &self.app,
                &self.ticket,
                CapturePhase::Unknown,
                Err("CAPTURE_WORKER_OUTCOME_UNKNOWN".into()),
            );
        }
    }
}
fn capture_cancel(app: &AppHandle, ticket: &str) -> Result<(), String> {
    let (cancellation, receipt) = {
        let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
        let slot = owner
            .slot
            .as_mut()
            .filter(|slot| slot.receipt.ticket == ticket)
            .ok_or("CAPTURE_STALE_TICKET")?;
        if slot.receipt.phase.terminal() {
            return Ok(());
        }
        slot.cancel_intent = true;
        if !slot.native_returned
            && !slot.publishing
            && slot.receipt.mode == CaptureMode::CurrentDocument
            && slot.renderer_binding.is_none()
            && slot.cancellation.revoke_pending_native()
        {
            // This is proof that neither isolated-world layout measurement
            // nor the renderer screenshot can ever be started by this
            // callback. Terminal cancellation is now safe, even if the
            // dispatcher never sends a reply.
            slot.receipt.phase = CapturePhase::Cancelled;
            slot.receipt.error = Some("CAPTURE_CANCELLED_BEFORE_DISPATCH".into());
            slot.receipt.path = None;
        } else if slot.receipt.phase == CapturePhase::Running {
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
    if owner.recovery_claimed {
        return Err("CAPTURE_DOMAIN_RECOVERY_CLAIMED".into());
    }
    if owner
        .last_completed
        .as_ref()
        .is_some_and(|receipt| receipt.ticket == ticket)
    {
        return Ok(());
    }
    let slot = owner
        .slot
        .as_ref()
        .filter(|slot| slot.receipt.ticket == ticket)
        .ok_or("CAPTURE_STALE_TICKET")?;
    if !slot.receipt.phase.terminal() {
        return Err("CAPTURE_NOT_TERMINAL".into());
    }
    // An acknowledgement also fences any delayed start which observed the
    // sequence while the old terminal slot was still present.
    owner.next_sequence = owner
        .next_sequence
        .checked_add(1)
        .ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
    owner.last_completed = owner.slot.take().map(|slot| slot.receipt);
    Ok(())
}
static LIVE_CAPTURE_DISPATCH: crate::shared::blocking::BlockingLane =
    crate::shared::blocking::BlockingLane::new(
        1,
        "CAPTURE_NATIVE_DISPATCH_BUSY",
        "CAPTURE_NATIVE_DISPATCH_FAILED",
    );

async fn run_capture(
    app: AppHandle,
    id: String,
    ticket: String,
    document: CaptureDocument,
    cancellation: CaptureCancellation,
) {
    let mut guard = CaptureWorkerGuard {
        app: app.clone(),
        ticket: ticket.clone(),
        completed: false,
    };
    let manager = app.state::<TabManagerState>().inner().clone();
    if cancellation.is_cancelled() {
        capture_finish(
            &app,
            &ticket,
            CapturePhase::Cancelled,
            Err("CAPTURE_CANCELLED_BEFORE_DISPATCH".into()),
        );
        guard.completed = true;
        return;
    }
    // A permanently blocked native dispatch must not consume another async
    // runtime thread or allow unbounded replacement blocking workers.
    let permit = match LIVE_CAPTURE_DISPATCH.reserve() {
        Ok(permit) => permit,
        Err(error) => {
            capture_finish(&app, &ticket, CapturePhase::Failed, Err(error));
            guard.completed = true;
            return;
        }
    };
    let (send, mut recv) = tokio::sync::oneshot::channel();
    let native_document = document.clone();
    let native_id = id.clone();
    let native_cancel = cancellation.clone();
    let _ = tauri::async_runtime::spawn(async move {
        let _ = permit
            .run(move || {
                if native_cancel.is_cancelled() {
                    return Ok(());
                }
                let _dispatch = manager.capture_bound_document(
                    &native_id,
                    native_document,
                    native_cancel,
                    move |result| {
                        let _ = send.send(result);
                    },
                );
                // Dropping a dispatch error cannot revoke an entered callback.
                Ok(())
            })
            .await;
    });
    // A lost completion after native entry remains UNKNOWN.
    let native = match tokio::time::timeout(std::time::Duration::from_secs(30), &mut recv).await {
        Ok(value) => value,
        Err(_) => {
            let _ = capture_cancel(&app, &ticket);
            if cancellation.native_dispatch_revoked() {
                guard.completed = true;
                return;
            }
            // Timeout after native entry is a request to stop, not a
            // completion receipt. Retain this owner through engine return.
            recv.await
        }
    };
    let Ok(result) = native else {
        // A dropped dispatch callback before entry is revocable. If it
        // entered the renderer, a lost completion still proves nothing.
        let _ = capture_cancel(&app, &ticket);
        if cancellation.native_dispatch_revoked() {
            guard.completed = true;
        }
        return;
    };
    if capture_returned(&ticket).is_err() {
        // A pre-dispatch terminal cancellation may already be acknowledged.
        // Never publish a late result for a replacement slot.
        guard.completed = true;
        return;
    }
    if cancellation.is_cancelled() {
        capture_finish(
            &app,
            &ticket,
            CapturePhase::Cancelled,
            Err("CAPTURE_CANCELLED_AT_COMPLETION".into()),
        );
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
    if !app
        .state::<TabManagerState>()
        .is_current_capture_document(&id, &document)
    {
        capture_finish(
            &app,
            &ticket,
            CapturePhase::Failed,
            Err("CAPTURE_DOCUMENT_STALE".into()),
        );
        guard.completed = true;
        return;
    }
    let publish_app = app.clone();
    let publish_ticket = ticket.clone();
    let storage = tauri::async_runtime::spawn_blocking(move || {
        capture_publish(&publish_app, &publish_ticket, &bytes)
    }).await;
    // A cancel arriving after publication admission cannot erase a file
    // that was actually committed. Report the outcome faithfully.
    match storage {
        Ok(Ok(path)) => capture_finish(&app, &ticket, CapturePhase::Succeeded, Ok(path)),
        Ok(Err(error)) if error == "CAPTURE_CANCELLED_BEFORE_PUBLICATION" => {
            capture_finish(&app, &ticket, CapturePhase::Cancelled, Err(error))
        }
        Ok(Err(error)) => capture_finish(&app, &ticket, CapturePhase::Failed, Err(error)),
        Err(_) => return,
    }
    guard.completed = true;
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
async fn run_independent_capture(
    app: AppHandle,
    ticket: String,
    address: String,
    cancellation: CaptureCancellation,
    interactive: bool,
) {
    let mut guard = CaptureWorkerGuard {
        app: app.clone(),
        ticket: ticket.clone(),
        completed: false,
    };
    let worker_app = app.clone();
    let worker_ticket = ticket.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let cancelled = || cancellation.is_cancelled();
        let pending = || {
            capture_finish(
                &worker_app,
                &worker_ticket,
                CapturePhase::Unknown,
                Err("CAPTURE_ISOLATED_EXIT_PENDING".into()),
            );
        };
        let bytes = if interactive {
            let ready = || CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?
                .mark_interactive_ready(&worker_ticket);
            let requested = || {
                let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
                if worker_app.state::<crate::ExitLifecycleState>().started.load(Ordering::Acquire)
                    || worker_app.state::<AppState>().shutdown_requested.load(Ordering::Acquire)
                {
                    return Err("CAPTURE_ISOLATED_CANCELLED".into());
                }
                owner.interactive_capture_requested(&worker_ticket)
            };
            crate::isolated_capture::render_interactive_job(
                &address,
                cancelled,
                pending,
                crate::isolated_capture::InteractiveControl {
                    ready: &ready,
                    capture_requested: &requested,
                },
            )
        } else {
            crate::isolated_capture::render_job(&address, cancelled, pending)
        };
        // The owner returns only before launch or after confirmed retirement.
        capture_returned(&worker_ticket)?;
        let bytes = bytes?;
        capture_publish(&worker_app, &worker_ticket, &bytes)
    })
    .await;
    match result {
        Ok(Ok(path)) => capture_finish(&app, &ticket, CapturePhase::Succeeded, Ok(path)),
        Ok(Err(error)) if matches!(error.as_str(), "CAPTURE_ISOLATED_CANCELLED" | "CAPTURE_CANCELLED_BEFORE_PUBLICATION") => {
            capture_finish(&app, &ticket, CapturePhase::Cancelled, Err(error))
        }
        Ok(Err(error)) => capture_finish(&app, &ticket, CapturePhase::Failed, Err(error)),
        Err(_) => return,
    }
    guard.completed = true;
}

/// The renderer returns only a byte count and bounded chunks of the same
/// native document. The existing main capture owner alone publishes PNGs.
async fn run_renderer_capture(
    app: AppHandle, id: String, ticket: String,
    binding: crate::grid_ipc::RendererBinding, cancellation: CaptureCancellation,
) {
    let mut guard = CaptureWorkerGuard { app: app.clone(), ticket: ticket.clone(), completed: false };
    let worker_app = app.clone();
    let worker_ticket = ticket.clone();
    let worker_cancel = cancellation.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        use base64::Engine as _;
        use crate::grid_ipc::{GridCmd, MAX_RENDERER_CAPTURE_BYTES, RENDERER_CAPTURE_CHUNK_BYTES};
        let state = worker_app.state::<AppState>();
        let manager = &state.grid_manager;
        if worker_cancel.is_cancelled() {
            capture_returned(&worker_ticket)?;
            return Err("CAPTURE_CANCELLED_BEFORE_DISPATCH".into());
        }
        let started = manager.renderer_request_data(binding, GridCmd::CaptureStart {
            id: id.clone(), ticket: worker_ticket.clone(),
        }, 30000);
        // Lost native capture completion is not a terminal receipt. Only
        // a genuine reply or explicit retirement of the original process
        // resolves the old native work.
        if started.as_ref().is_err_and(|error| error.contains("OUTCOME_UNKNOWN")) {
            return Err("CAPTURE_RENDERER_OUTCOME_UNKNOWN".into());
        }
        capture_returned(&worker_ticket)?;
        let total = started?.and_then(|size| size.parse::<usize>().ok())
            .filter(|size| *size > 0 && *size <= MAX_RENDERER_CAPTURE_BYTES)
            .ok_or("CAPTURE_IMAGE_LIMIT")?;
        let collect = (|| -> Result<Vec<u8>, String> {
            let mut bytes = Vec::new();
            bytes.try_reserve_exact(total).map_err(|_| "CAPTURE_IMAGE_LIMIT")?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while bytes.len() < total {
                if worker_cancel.is_cancelled() { return Err("CAPTURE_CANCELLED_AT_COMPLETION".into()); }
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() { return Err("CAPTURE_CHUNK_DEADLINE".into()); }
                let offset = bytes.len();
                let data = manager.renderer_request_data(binding, GridCmd::CaptureRead {
                    id: id.clone(), ticket: worker_ticket.clone(), offset,
                }, remaining.as_millis().min(3000).max(1) as u64)?.ok_or("CAPTURE_CHUNK_MISSING")?;
                let expected = (total - offset).min(RENDERER_CAPTURE_CHUNK_BYTES);
                // Bound the encoded input before allocating the decoded chunk.
                if data.len() > (expected + 2) / 3 * 4 {
                    return Err("CAPTURE_CHUNK_INVALID".into());
                }
                let chunk = base64::engine::general_purpose::STANDARD.decode(data)
                    .map_err(|_| "CAPTURE_CHUNK_INVALID")?;
                if chunk.len() != expected { return Err("CAPTURE_CHUNK_INVALID".into()); }
                bytes.extend_from_slice(&chunk);
            }
            Ok(bytes)
        })();
        // Always release an acknowledged completed capture. The renderer
        // retains at most one bounded buffer if the connection is lost.
        let release = manager.renderer_request_data(binding, GridCmd::CaptureRelease {
            id, ticket: worker_ticket.clone(),
        }, 3000);
        let bytes = collect?;
        release?;
        if manager.renderer_binding(binding.index)? != binding {
            return Err("CAPTURE_DOCUMENT_STALE".into());
        }
        if worker_cancel.is_cancelled() {
            return Err("CAPTURE_CANCELLED_AT_COMPLETION".into());
        }
        capture_publish(&worker_app, &worker_ticket, &bytes)
    }).await;
    match result {
        Ok(Ok(path)) => capture_finish(&app, &ticket, CapturePhase::Succeeded, Ok(path)),
        Ok(Err(error)) if error == "CAPTURE_RENDERER_OUTCOME_UNKNOWN" =>
            capture_finish(&app, &ticket, CapturePhase::Unknown, Err(error)),
        Ok(Err(error)) if error.starts_with("CAPTURE_CANCELLED") =>
            capture_finish(&app, &ticket, CapturePhase::Cancelled, Err(error)),
        Ok(Err(error)) => capture_finish(&app, &ticket, CapturePhase::Failed, Err(error)),
        Err(_) => return,
    }
    guard.completed = true;
}

/// Existing recovery confirmation can retire one independent renderer,
/// rather than escalating an unknown capture to the entire application.
fn recover_renderer_capture(app: &AppHandle, target: &CaptureRecoveryTarget) -> Result<bool, String> {
    let binding = {
        let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
        if owner.recovery_target(std::time::Instant::now()).as_ref() != Some(target) {
            return Err("CAPTURE_DOMAIN_RECOVERY_NOT_ELIGIBLE".into());
        }
        let Some(binding) = owner.slot.as_ref().and_then(|slot| slot.renderer_binding) else {
            return Ok(false);
        };
        owner.recovery_intent = None;
        owner.recovery_claimed = true;
        binding
    };
    let worker_app = app.clone();
    let ticket = target.ticket.clone();
    let started = std::thread::Builder::new().name("capture-renderer-recovery".into()).spawn(move || {
        let result = worker_app.state::<AppState>().grid_manager
            .retire_bound_tab_renderer(binding.index, binding.owner);
        let receipt = CAPTURE_OWNER.lock().ok().and_then(|mut owner| {
            owner.recovery_claimed = false;
            let slot = owner.slot.as_mut().filter(|slot| slot.receipt.ticket == ticket)?;
            if slot.receipt.phase != CapturePhase::Unknown { return None; }
            if result.is_ok() {
                slot.native_returned = true;
                slot.receipt.phase = if slot.cancel_intent { CapturePhase::Cancelled } else { CapturePhase::Failed };
                slot.receipt.error = Some("CAPTURE_RENDERER_RETIRED_REACTIVATE".into());
            } else {
                slot.receipt.error = Some("CAPTURE_RENDERER_CLEANUP_PENDING".into());
            }
            Some(slot.receipt.clone())
        });
        if let Some(receipt) = receipt { capture_notify(&worker_app, receipt); }
    });
    if started.is_err() {
        CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?.recovery_claimed = false;
        return Err("CAPTURE_RENDERER_RECOVERY_WORKER_FAILED".into());
    }
    Ok(true)
}

enum CaptureWork {
    Current(CaptureDocument),
    Renderer(crate::grid_ipc::RendererBinding),
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    Independent(String, bool),
}

fn begin_capture(
    app: AppHandle,
    id: String,
    ticket: String,
    epoch: &str,
    sequence: &str,
    mode: CaptureMode,
    expected_url: Option<String>,
) -> Result<(), String> {
    crate::shared::invocation::check_tab_id(&id)?;
    if ticket.len() != 36
        || uuid::Uuid::parse_str(&ticket).map_or(true, |id| id.get_version_num() != 4)
    {
        return Err("CAPTURE_INVALID_TICKET".into());
    }
    if app
        .state::<AppState>()
        .shutdown_requested
        .load(Ordering::Acquire)
    {
        return Err("CAPTURE_APPLICATION_SHUTTING_DOWN".into());
    }
    let work = match mode {
        CaptureMode::CurrentDocument => {
            if expected_url.is_some() {
                return Err("CAPTURE_MODE_INVALID".into());
            }
            if let Some(index) = crate::bridge::isolated_tabs::route(&id) {
                let state = app.state::<AppState>();
                if !state.tabs.lock().unwrap().contains_key(&id) {
                    return Err("CAPTURE_TAB_UNAVAILABLE".into());
                }
                CaptureWork::Renderer(state.grid_manager.renderer_binding(index)?)
            } else {
                CaptureWork::Current(
                    app.state::<TabManagerState>()
                        .capture_document(&id)
                        .map_err(|_| "CAPTURE_TAB_UNAVAILABLE")?,
                )
            }
        }
        CaptureMode::IndependentAddress | CaptureMode::InteractiveAddress => {
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                let expected_url = expected_url.ok_or("CAPTURE_ADDRESS_REQUIRED")?;
                // Bind the address explicitly selected by the main UI to the
                // existing Browser Owner. Never open an unrelated caller URL.
                let state = app.state::<AppState>();
                let tabs = state
                    .tabs
                    .try_lock()
                    .map_err(|_| "CAPTURE_TAB_UNAVAILABLE")?;
                let current = tabs.get(&id).ok_or("CAPTURE_TAB_UNAVAILABLE")?;
                if current.url != expected_url {
                    return Err("CAPTURE_ADDRESS_CHANGED".into());
                }
                CaptureWork::Independent(
                    crate::isolated_capture::checked_url(&expected_url)?,
                    mode == CaptureMode::InteractiveAddress,
                )
            }
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            {
                return Err("CAPTURE_INDEPENDENT_UNSUPPORTED".into());
            }
        }
    };
    let cancellation = CaptureCancellation::new();
    {
        let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
        // The exit preflight claims its latch before taking this same lock.
        // Thus it sees any admitted worker, and no new worker can slip past it.
        if app
            .state::<crate::ExitLifecycleState>()
            .started
            .load(Ordering::Acquire)
            || app
                .state::<AppState>()
                .shutdown_requested
                .load(Ordering::Acquire)
        {
            return Err("CAPTURE_APPLICATION_SHUTTING_DOWN".into());
        }
        if epoch != owner.epoch {
            return Err("CAPTURE_STALE_EPOCH".into());
        }
        if owner.recovery_claimed {
            return Err("CAPTURE_DOMAIN_RECOVERY_CLAIMED".into());
        }
        if let Some(slot) = &owner.slot {
            if slot.receipt.ticket == ticket
                && slot.receipt.tab_id == id
                && slot.receipt.mode == mode
            {
                return Ok(());
            }
            return Err("CAPTURE_BUSY".into());
        }
        if sequence != owner.next_sequence.to_string() {
            return Err("CAPTURE_STALE_SEQUENCE".into());
        }
        owner.next_sequence = owner
            .next_sequence
            .checked_add(1)
            .ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
        owner.slot = Some(CaptureSlot {
            receipt: CaptureReceipt {
                ticket: ticket.clone(),
                tab_id: id.clone(),
                mode,
                phase: CapturePhase::Running,
                path: None,
                error: None,
            },
            cancellation: cancellation.clone(),
            renderer_binding: match &work { CaptureWork::Renderer(binding) => Some(*binding), _ => None },
            since: std::time::Instant::now(),
            native_returned: false,
            publishing: false,
            cancel_intent: false,
            interactive_stage: (mode == CaptureMode::InteractiveAddress)
                .then_some(InteractiveStage::Opening),
        });
    }
    // Spawn independent work before returning the receipt. No callback needs
    // the original invoke future or its frontend to continue.
    match work {
        CaptureWork::Current(document) => {
            let _ =
                tauri::async_runtime::spawn(run_capture(app, id, ticket, document, cancellation));
        }
        CaptureWork::Renderer(binding) => {
            let _ = tauri::async_runtime::spawn(run_renderer_capture(app, id, ticket, binding, cancellation));
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        CaptureWork::Independent(address, interactive) => {
            let _ = tauri::async_runtime::spawn(run_independent_capture(
                app,
                ticket,
                address,
                cancellation,
                interactive,
            ));
        }
    }
    Ok(())
}

/// Reversible exit preflight for a separately owned renderer. If actual
/// cleanup is not confirmed, keep the application and its process handles
/// alive. This does not change the ordinary live-tab shutdown semantics.
pub(crate) fn prepare_capture_exit(app: &AppHandle) -> Result<(), String> {
    let ticket = {
        let owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
        owner
            .slot
            .as_ref()
            .filter(|slot| {
                slot.receipt.mode != CaptureMode::CurrentDocument
                    && !slot.receipt.phase.terminal()
            })
            .map(|slot| slot.receipt.ticket.clone())
    };
    let Some(ticket) = ticket else {
        return Ok(());
    };
    if let Err(error) = capture_cancel(app, &ticket) {
        // The terminal receipt may already have been acknowledged.
        if error != "CAPTURE_STALE_TICKET" {
            return Err(error);
        }
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        {
            let owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
            if !owner
                .slot
                .as_ref()
                .is_some_and(|slot| slot.receipt.ticket == ticket && !slot.receipt.phase.terminal())
            {
                return Ok(());
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err("CAPTURE_EXIT_CLEANUP_PENDING".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum CaptureRequest {
    Inspect,
    Fence {
        epoch: String,
        sequence: String,
    },
    CancelPending {
        id: String,
        ticket: String,
        epoch: String,
        sequence: String,
        #[serde(default)]
        mode: CaptureMode,
    },
    Start {
        id: String,
        ticket: String,
        epoch: String,
        sequence: String,
        #[serde(default)]
        mode: CaptureMode,
        #[serde(default)]
        expected_url: Option<String>,
    },
    Cancel {
        ticket: String,
    },
    Ack {
        ticket: String,
    },
    CaptureNow {
        epoch: String,
        sequence: String,
        ticket: String,
        confirmed: bool,
    },
    RecoverDomain {
        epoch: String,
        sequence: String,
        ticket: String,
        confirmed: bool,
        #[serde(default)]
        scope: Option<String>,
    },
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
    if webview.label() != "main" {
        return Err("CAPTURE_MAIN_ONLY".into());
    }
    crate::shared::invocation::check_invocation_source(&webview, "capture_full_page", None, &app)?;
    if let Some(request) = request {
        match request {
            CaptureRequest::Inspect => {}
            CaptureRequest::Fence { epoch, sequence } => {
                let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
                if owner.slot.is_some() {
                    return Err("CAPTURE_BUSY".into());
                }
                if epoch != owner.epoch {
                    return Err("CAPTURE_STALE_EPOCH".into());
                }
                if sequence != owner.next_sequence.to_string() {
                    return Err("CAPTURE_STALE_SEQUENCE".into());
                }
                owner.next_sequence = owner
                    .next_sequence
                    .checked_add(1)
                    .ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
            }
            CaptureRequest::CancelPending {
                id,
                ticket,
                epoch,
                sequence,
                mode,
            } => {
                crate::shared::invocation::check_tab_id(&id)?;
                if ticket.len() != 36
                    || uuid::Uuid::parse_str(&ticket).map_or(true, |id| id.get_version_num() != 4)
                {
                    return Err("CAPTURE_INVALID_TICKET".into());
                }
                let cancelled_before_start = {
                    let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
                    if epoch != owner.epoch {
                        return Err("CAPTURE_STALE_EPOCH".into());
                    }
                    if owner
                        .last_completed
                        .as_ref()
                        .is_some_and(|r| r.ticket == ticket)
                    {
                        true
                    } else if let Some(slot) = &owner.slot {
                        if slot.receipt.ticket != ticket
                            || slot.receipt.tab_id != id
                            || slot.receipt.mode != mode
                        {
                            return Err("CAPTURE_BUSY".into());
                        }
                        false
                    } else {
                        if sequence != owner.next_sequence.to_string() {
                            return Err("CAPTURE_STALE_SEQUENCE".into());
                        }
                        owner.next_sequence = owner
                            .next_sequence
                            .checked_add(1)
                            .ok_or("CAPTURE_SEQUENCE_EXHAUSTED")?;
                        owner.last_completed = Some(CaptureReceipt {
                            ticket: ticket.clone(),
                            tab_id: id,
                            mode,
                            phase: CapturePhase::Cancelled,
                            path: None,
                            error: Some("CAPTURE_CANCELLED_BEFORE_DISPATCH".into()),
                        });
                        true
                    }
                };
                if !cancelled_before_start {
                    capture_cancel(&app, &ticket)?;
                }
            }
            CaptureRequest::Start {
                id,
                ticket,
                epoch,
                sequence,
                mode,
                expected_url,
            } => begin_capture(
                app.clone(),
                id,
                ticket,
                &epoch,
                &sequence,
                mode,
                expected_url,
            )?,
            CaptureRequest::Cancel { ticket } => capture_cancel(&app, &ticket)?,
            CaptureRequest::Ack { ticket } => capture_ack(&ticket)?,
            CaptureRequest::CaptureNow { epoch, sequence, ticket, confirmed } => {
                if !confirmed {
                    return Err("CAPTURE_INTERACTIVE_CONFIRMATION_REQUIRED".into());
                }
                // Check shutdown under the same capture-owner lock used by
                // exit preflight, before publishing the one-shot request.
                let mut owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
                if app.state::<crate::ExitLifecycleState>().started.load(Ordering::Acquire)
                    || app.state::<AppState>().shutdown_requested.load(Ordering::Acquire)
                {
                    return Err("CAPTURE_APPLICATION_SHUTTING_DOWN".into());
                }
                owner.request_interactive_capture(&epoch, &sequence, &ticket)?;
            }
            CaptureRequest::RecoverDomain { epoch, sequence, ticket, confirmed, scope } => {
                if !confirmed {
                    return Err("CAPTURE_DOMAIN_RECOVERY_CONFIRMATION_REQUIRED".into());
                }
                let sequence = sequence.parse::<u64>().map_err(|_| "CAPTURE_STALE_SEQUENCE")?;
                let target = {
                    let owner = CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?;
                    let target = owner.recovery_target(std::time::Instant::now())
                        .filter(|target| target.epoch == epoch && target.sequence == sequence
                            && target.ticket == ticket)
                        .ok_or("CAPTURE_DOMAIN_RECOVERY_NOT_ELIGIBLE")?;
                    if !capture_recovery_scope_matches(scope.as_deref(), target.renderer_only) {
                        return Err("CAPTURE_DOMAIN_RECOVERY_SCOPE_CHANGED".into());
                    }
                    target
                };
                if target.renderer_only {
                    // Never escalate a renderer-only confirmation to application restart.
                    if !recover_renderer_capture(&app, &target)? {
                        return Err("CAPTURE_DOMAIN_RECOVERY_SCOPE_CHANGED".into());
                    }
                } else {
                    if !capture_supervisor_ready() {
                        return Err("CAPTURE_DOMAIN_SUPERVISOR_UNAVAILABLE".into());
                    }
                    CAPTURE_OWNER.lock().map_err(|_| "CAPTURE_OWNER_UNKNOWN")?
                        .request_recovery(target, std::time::Instant::now())?;
                }
            }
        }
        return serde_json::to_value(capture_view()?).map_err(|_| "CAPTURE_RECEIPT_FAILED".into());
    }
    let id = id.ok_or("CAPTURE_TAB_UNAVAILABLE")?;
    let view = capture_view()?;
    let ticket = uuid::Uuid::new_v4().to_string();
    begin_capture(
        app.clone(),
        id,
        ticket.clone(),
        &view.epoch,
        &view.next_sequence,
        CaptureMode::CurrentDocument,
        None,
    )?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let current = capture_view()?;
        let receipt = current
            .receipt
            .filter(|receipt| receipt.ticket == ticket)
            .or_else(|| {
                current
                    .last_completed
                    .filter(|receipt| receipt.ticket == ticket)
            })
            .ok_or("CAPTURE_OUTCOME_UNKNOWN")?;
        if receipt.phase.terminal() {
            capture_ack(&ticket)?;
            return receipt
                .path
                .map(serde_json::Value::String)
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
    // Full PNG verification also covers the live-document path, not just
    // the isolated worker. Reject truncated output before any publication.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    crate::isolated_capture::checked_png(bytes)?;
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

    fn capture_fixture() -> CaptureOwner {
        CaptureOwner {
            epoch: "fixture".into(),
            next_sequence: 7,
            slot: Some(CaptureSlot {
                receipt: CaptureReceipt {
                    ticket: "old".into(),
                    tab_id: "tab-1".into(),
                    mode: CaptureMode::IndependentAddress,
                    phase: CapturePhase::Unknown,
                    path: None,
                    error: Some("CAPTURE_ISOLATED_EXIT_PENDING".into()),
                },
                cancellation: CaptureCancellation::new(),
                renderer_binding: None,
                since: std::time::Instant::now(),
                native_returned: false,
                publishing: false,
                cancel_intent: false,
                interactive_stage: None,
            }),
            last_completed: None,
            recovery_intent: None,
            recovery_claimed: false,
        }
    }

    #[test]
    fn renderer_recovery_claim_rechecks_exact_owner_and_return_boundary() {
        let mut owner = capture_fixture();
        let now = std::time::Instant::now();
        let target = owner.recovery_target(now).unwrap();
        owner.request_recovery(target.clone(), now).unwrap();
        let intent = owner.recovery_intent.clone().unwrap();
        assert!(!owner.claim_recovery(&intent, now + CAPTURE_RECOVERY_TTL));
        owner.mark_returned("old").unwrap();
        assert!(!owner.claim_recovery(&intent, now));
        assert!(owner.recovery_target(now).is_none());
        owner.reserve_publication("old").unwrap();
        assert!(owner.reserve_publication("old").is_err());
        assert!(owner.request_recovery(target, now).is_err());
    }

    #[test]
    fn capture_publication_is_fenced_by_cancel_and_recovery_claim() {
        let mut owner = capture_fixture();
        let now = std::time::Instant::now();
        let target = owner.recovery_target(now).unwrap();
        owner.request_recovery(target, now).unwrap();
        let intent = owner.recovery_intent.clone().unwrap();
        assert!(owner.claim_recovery(&intent, now));
        owner.mark_returned("old").unwrap();
        assert_eq!(owner.reserve_publication("old").unwrap_err(), "CAPTURE_CANCELLED_BEFORE_PUBLICATION");
        owner.recovery_claimed = false;
        // The independent renderer's Unknown receipt must remain visible.
        // The mutex-protected cancel intent still fences publication, even
        // before the out-of-lock native cancellation notification is sent.
        owner.slot.as_mut().unwrap().cancel_intent = true;
        assert_eq!(owner.slot.as_ref().unwrap().receipt.phase, CapturePhase::Unknown);
        assert_eq!(owner.reserve_publication("old").unwrap_err(), "CAPTURE_CANCELLED_BEFORE_PUBLICATION");
        assert!(owner.reserve_publication("replacement").is_err());
    }

    #[test]
    fn queued_live_capture_and_ordinary_lost_reply_are_not_recovery_authority() {
        let mut owner = capture_fixture();
        let slot = owner.slot.as_mut().unwrap();
        slot.receipt.mode = CaptureMode::CurrentDocument;
        assert!(owner.recovery_target(std::time::Instant::now()).is_none());
        let slot = owner.slot.as_mut().unwrap();
        slot.receipt.mode = CaptureMode::IndependentAddress;
        slot.receipt.error = Some("CAPTURE_WORKER_OUTCOME_UNKNOWN".into());
        assert!(owner.recovery_target(std::time::Instant::now()).is_none());
    }

    #[test]
    fn interactive_capture_requires_ready_exact_owner_and_one_request() {
        let mut owner = capture_fixture();
        let slot = owner.slot.as_mut().unwrap();
        slot.receipt.mode = CaptureMode::InteractiveAddress;
        slot.receipt.phase = CapturePhase::Running;
        slot.receipt.error = None;
        slot.interactive_stage = Some(InteractiveStage::Opening);
        assert_eq!(
            owner.request_interactive_capture("fixture", "7", "old").unwrap_err(),
            "CAPTURE_INTERACTIVE_NOT_READY"
        );
        assert!(owner.mark_interactive_ready("replacement").is_err());
        owner.mark_interactive_ready("old").unwrap();
        assert!(!owner.interactive_capture_requested("old").unwrap());
        assert!(owner.mark_interactive_ready("old").is_err());
        assert!(owner.request_interactive_capture("new", "7", "old").is_err());
        assert!(owner.request_interactive_capture("fixture", "6", "old").is_err());
        assert!(owner.request_interactive_capture("fixture", "7", "replacement").is_err());
        owner.request_interactive_capture("fixture", "7", "old").unwrap();
        owner.request_interactive_capture("fixture", "7", "old").unwrap();
        assert!(owner.interactive_capture_requested("old").unwrap());
        owner.slot.as_mut().unwrap().cancel_intent = true;
        assert!(owner.interactive_capture_requested("old").is_err());
        assert!(owner.request_interactive_capture("fixture", "7", "old").is_err());
    }

    #[test]
    fn capture_recovery_confirmation_never_crosses_scope() {
        assert!(capture_recovery_scope_matches(Some("renderer"), true));
        assert!(!capture_recovery_scope_matches(Some("renderer"), false));
        assert!(!capture_recovery_scope_matches(Some("application"), true));
        assert!(!capture_recovery_scope_matches(None, true));
        assert!(capture_recovery_scope_matches(None, false));
        assert!(capture_recovery_scope_matches(Some("application"), false));
        assert!(!capture_recovery_scope_matches(Some("other"), false));
    }

    #[test]
    fn renderer_capture_recovery_requires_unknown_native_outcome() {
        let mut owner = capture_fixture();
        let slot = owner.slot.as_mut().unwrap();
        slot.receipt.mode = CaptureMode::CurrentDocument;
        slot.renderer_binding = Some(crate::grid_ipc::RendererBinding {
            index: crate::grid_ipc::tab_renderer_index("tab-1").unwrap(),
            owner: (42, 1), generation: 2, navigation_revision: 0,
        });
        slot.receipt.phase = CapturePhase::Running;
        assert!(owner.recovery_target(std::time::Instant::now()).is_none());
        owner.slot.as_mut().unwrap().receipt.phase = CapturePhase::Unknown;
        assert!(owner.recovery_target(std::time::Instant::now()).is_some());
        owner.slot.as_mut().unwrap().native_returned = true;
        assert!(owner.recovery_target(std::time::Instant::now()).is_none());
    }

    #[test]
    fn old_capture_protocol_defaults_to_current_document() {
        let old: CaptureRequest = serde_json::from_value(serde_json::json!({
            "phase":"start","id":"tab-1","ticket":uuid::Uuid::new_v4().to_string(),
            "epoch":"old","sequence":"1"
        }))
        .unwrap();
        assert!(matches!(
            old,
            CaptureRequest::Start {
                mode: CaptureMode::CurrentDocument,
                expected_url: None,
                ..
            }
        ));
        let independent: CaptureRequest = serde_json::from_value(serde_json::json!({
            "phase":"start","id":"tab-1","ticket":uuid::Uuid::new_v4().to_string(),
            "epoch":"old","sequence":"1","mode":"independent_address","expected_url":"https://example.com/"
        })).unwrap();
        assert!(matches!(
            independent,
            CaptureRequest::Start {
                mode: CaptureMode::IndependentAddress,
                expected_url: Some(_),
                ..
            }
        ));
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

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn complete_png_can_publish_through_the_existing_owner_storage() {
        let base = std::env::temp_dir().join(format!("browseros-capture-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&base).unwrap();
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0; 12]).unwrap();
            writer.finish().unwrap();
        }
        let path = save_capture(&base, &bytes).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn truncated_png_never_creates_capture_storage() {
        let base = std::env::temp_dir().join(format!("browseros-capture-{}", uuid::Uuid::new_v4()));
        assert_eq!(
            save_capture(&base, &image_header(2, 2)),
            Err("CAPTURE_INVALID_PNG".into())
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
        // Use a complete PNG so the no-link storage check is exercised
        // rather than short-circuiting at the stricter PNG verifier.
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0; 12]).unwrap();
            writer.finish().unwrap();
        }
        crate::isolated_capture::checked_png(&bytes).unwrap();
        assert!(save_capture(&base, &bytes).is_err());
        assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0);
        std::fs::remove_dir_all(base).unwrap();
    }
}
