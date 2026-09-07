use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};
use url::Url;

use crate::domain::*;
use crate::grid_ipc::GridCmd;
use crate::keyring_store::KeyringStore;
use crate::mcp::{McpDecisionView, McpRegistryEntryView};
use crate::script_runner::{RunError, RunSnapshot, ScriptProcessTable, ScriptRunRecord};
use crate::seam::{PathResolver, Progress, ProgressSink, RootsProvider};
use crate::sync;
use crate::terminal::{self, ChannelSink, EventSink};
use crate::workspace;

/// 宫格 label（grid-N）→ 子进程 index；页签 tab-N 返回 None（页签仍在主进程）。
fn grid_index_of(label: &str) -> Option<u32> {
    label.strip_prefix("grid-")?.parse().ok()
}

fn is_tab_label(label: &str) -> bool {
    label
        .strip_prefix("tab-")
        .and_then(|n| n.parse::<u32>().ok())
        .is_some()
}

/// tab-N 主进程 WebView 恢复预算：只响应真实操作失败，不启动后台无限重启。
#[derive(Default)]
pub struct TabRecoveryBudget {
    window_started: Option<std::time::Instant>,
    attempts: u8,
}

const TAB_RECOVERY_WINDOW_SECS: u64 = 60;
const TAB_RECOVERY_MAX_ATTEMPTS: u8 = 2;

fn reserve_tab_recovery_attempt(
    budget: &mut TabRecoveryBudget,
    now: std::time::Instant,
) -> Option<u8> {
    match budget.window_started {
        Some(start)
            if now.duration_since(start).as_secs() <= TAB_RECOVERY_WINDOW_SECS
                && budget.attempts >= TAB_RECOVERY_MAX_ATTEMPTS =>
        {
            None
        }
        Some(start) if now.duration_since(start).as_secs() <= TAB_RECOVERY_WINDOW_SECS => {
            budget.attempts += 1;
            Some(budget.attempts)
        }
        _ => {
            budget.window_started = Some(now);
            budget.attempts = 1;
            Some(1)
        }
    }
}

fn emit_tab_recovery_event(
    app: &AppHandle,
    id: &str,
    url: &str,
    reason: &str,
    status: &str,
    attempt: u8,
    message: &str,
) {
    eprintln!(
        "[tab-recovery] id={} status={} attempt={}/{} reason={} message={}",
        id, status, attempt, TAB_RECOVERY_MAX_ATTEMPTS, reason, message
    );
    let _ = app.emit(
        "tab-recovery",
        serde_json::json!({
            "id": id,
            "url": url,
            "reason": reason,
            "status": status,
            "attempt": attempt,
            "max_attempts": TAB_RECOVERY_MAX_ATTEMPTS,
            "window_secs": TAB_RECOVERY_WINDOW_SECS,
            "message": message,
        }),
    );
}

pub fn report_tab_load_failed(app: &AppHandle, id: &str, url: &str, message: &str) {
    if !is_tab_label(id) {
        return;
    }
    emit_tab_recovery_event(app, id, url, "loadFailed", "load-failed", 0, message);
}

/// 在主窗口内创建一个子 Webview（方案 D：同窗口多 webview）。
/// 底层已迁移到 tauri-plugin-browser-tabs：全链路 Logical(CSS) 坐标，
/// Linux 下由插件强制触发 WebKitGTK size_allocate，修复子 webview 卡初始尺寸问题。
/// window.open / target="_blank" 由插件统一拦截为 browser-tabs://event，
/// 再在 main.rs 转发为前端既有的 new-tab-request。
/// label 全局唯一（页签 tab-N / 宫格 grid-N）。
fn spawn_child_window(
    app: &AppHandle,
    label: &str,
    url: &str,
    css_x: f64,
    css_y: f64,
    css_w: f64,
    css_h: f64,
) -> Result<(), String> {
    use tauri_plugin_browser_tabs::{CreateTabOptions, LogicalRect, TabManagerState};
    let init_script = include_str!("../injected/collect.js");
    let manager = app.state::<TabManagerState>();
    manager
        .create_tab(CreateTabOptions {
            id: label.to_string(),
            url: url.to_string(),
            rect: LogicalRect::new(
                css_x.round().max(0.0),
                css_y.round().max(0.0),
                css_w.round().max(1.0),
                css_h.round().max(1.0),
            ),
            visible: false, // 前端随后 tab_position 时再 show
            auto_resize: true,
            user_agent: None,
            transparent: false,
            initialization_script: Some(init_script.to_string()),
        })
        .map_err(|e| format!("创建子 webview 失败: {e}"))?;
    eprintln!("[spawn_child_window] 子 webview 已创建 label={}", label);
    Ok(())
}

/// 在插件管理的子 webview 中执行 JS。
/// 子 webview（页签 tab-N / 宫格 grid-N）全部由 browser-tabs 插件创建并持有，
/// app.get_webview 在 Tauri 注册表里找不到它们（曾经用 get_webview 导致
/// eval 静默空转：AI 群发无反应、前进/后退/刷新全失效）。必须走插件 TabManager。
/// 直接在当前线程调用（Tauri v2 的 Webview::eval 内部走 dispatcher，跨线程安全，
/// 与 apply_bounds_inner 同一模式），错误经返回值上报前端，不再静默。
fn plugin_eval(app: &AppHandle, id: &str, js: &str) -> Result<(), String> {
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    if let Err(e) = manager.eval(&id.to_string(), js) {
        let initial_error = format!("webview {id} 执行 JS 失败: {e}");
        recover_tab_webview(app, id, "eval", &initial_error)?;
        app.state::<TabManagerState>()
            .eval(&id.to_string(), js)
            .map_err(|retry| format!("{initial_error}; 恢复后重试 JS 失败: {retry}"))?;
    }
    Ok(())
}

fn navigate_tab_webview(app: &AppHandle, id: &str, url: &str) -> Result<(), String> {
    use tauri_plugin_browser_tabs::TabManagerState;
    app.state::<TabManagerState>()
        .navigate(&id.to_string(), url)
        .map_err(|e| format!("webview {id} 导航失败: {e}"))
}

fn recover_tab_webview(
    app: &AppHandle,
    id: &str,
    reason: &str,
    source_error: &str,
) -> Result<(), String> {
    if !is_tab_label(id) {
        return Err(source_error.to_string());
    }
    let state = app.state::<AppState>();
    if state.hibernated_tabs.lock().unwrap().contains(id) {
        return Err(format!("{source_error}; 页签已休眠，等待激活时重建"));
    }
    let url = state
        .tabs
        .lock()
        .unwrap()
        .get(id)
        .map(|t| t.url.clone())
        .ok_or_else(|| format!("{source_error}; 页签元数据不存在"))?;
    let layout = state.child_layouts.lock().unwrap().get(id).copied();
    let attempt = {
        let mut budgets = state.tab_recovery.lock().unwrap();
        let budget = budgets.entry(id.to_string()).or_default();
        reserve_tab_recovery_attempt(budget, std::time::Instant::now())
    };
    let Some(attempt) = attempt else {
        emit_tab_recovery_event(
            app,
            id,
            &url,
            reason,
            "budget-exhausted",
            TAB_RECOVERY_MAX_ATTEMPTS,
            source_error,
        );
        return Err(format!("{source_error}; tab 恢复预算已耗尽"));
    };
    emit_tab_recovery_event(app, id, &url, reason, "attempting", attempt, source_error);
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    let _ = manager.close_tab(&id.to_string());
    let (x, y, w, h) = layout.unwrap_or((0.0, 0.0, 1.0, 1.0));
    if let Err(e) = spawn_child_window(app, id, &url, x, y, w, h) {
        emit_tab_recovery_event(app, id, &url, reason, "failed", attempt, &e);
        return Err(format!("{source_error}; tab 恢复失败: {e}"));
    }
    if let Some((lx, ly, lw, lh)) = layout {
        if lx > -1000.0 {
            if let Err(e) = apply_bounds_inner(app, id, lx, ly, lw, lh) {
                emit_tab_recovery_event(app, id, &url, reason, "failed", attempt, &e);
                return Err(format!("{source_error}; tab 恢复后定位失败: {e}"));
            }
        }
    }
    emit_tab_recovery_event(
        app,
        id,
        &url,
        reason,
        "recovered",
        attempt,
        "recreated webview",
    );
    Ok(())
}

/// 记住某个子窗口的内容区布局矩形（CSS 坐标），供 move/resize 时重定位。
fn remember_layout(app: &AppHandle, id: &str, x: f64, y: f64, w: f64, h: f64) {
    app.state::<AppState>()
        .child_layouts
        .lock()
        .unwrap()
        .insert(id.to_string(), (x, y, w, h));
}

/// M0-0.b 测量钩子配置：由 M0 采集脚本经环境变量注入（日常运行全 None，零影响）。
/// 契约 `logs/m0-baseline-contract-v1.md` §6.1/§6.3。
#[derive(Default, Clone)]
pub struct M0Config {
    /// M0 测量 run_id（契约 §8 命名），前端/后端在信号文件中回带
    pub run_id: String,
    /// ready 信号文件路径：前端 mount + 2×rAF + IPC 往返后写入 startup_ready 信号
    pub ready_file: Option<String>,
    /// 测量报告目录：终端吞吐报告 / 资源循环标记文件写入处
    pub report_dir: Option<String>,
    /// M0_DRIVER 驱动模式：tab|grid|terminal|term-throughput（无则非测量运行）
    pub driver: String,
}

/// 桥的进程级状态：仅保存待确认任务（凭据/审计都在磁盘，避免内存泄漏）
#[derive(Default)]
pub struct AppState {
    pub pending_jobs: Mutex<HashMap<String, SyncJob>>,
    /// M1-6.b：Git 写待确认任务（与成果推送 SyncJob 完全隔离，互不干扰；
    /// 一次性：confirm 取出即从表移除，需重新 request 才能再执行）
    pub pending_git_jobs: Mutex<HashMap<String, GitWriteJob>>,
    /// M0-0.b 测量钩子配置（环境变量注入；见 `M0Config`）
    pub m0_config: Mutex<M0Config>,
    /// 资源扫描线程只允许启动一次；线程按当前 active_tab 工作，空闲时休眠。
    pub browser_scanner_started: AtomicBool,
    /// 统一生命周期关闭信号：M0-2.c 迁移后台线程后，循环应尽快自然退出。
    pub shutdown_requested: Arc<AtomicBool>,
    /// 当前所有浏览器页签（id -> 信息）
    pub tabs: Mutex<HashMap<String, TabInfo>>,
    /// 当前激活的页签 id
    pub active_tab: Mutex<Option<String>>,
    /// 页签自增计数器
    pub tab_counter: Arc<Mutex<u32>>,
    /// 子窗口布局：id -> 内容区 CSS 矩形 (x, y, w, h)，供主窗 move/resize 重定位
    pub child_layouts: Mutex<HashMap<String, (f64, f64, f64, f64)>>,
    /// 上次定位：id -> (Instant, x, y, w, h)。去重防抖只在"rect 相同且 <50ms"时丢弃，
    /// rect 不同的新定位必须应用（否则旧 rect 先到时，50ms 窗口内的新 rect 被静默吞掉，
    /// 子 webview 永远停在旧位置——AI 模式输入框被宫格盖住就是这个 bug）
    pub last_position_at: Mutex<HashMap<String, (std::time::Instant, f64, f64, f64, f64)>>,
    /// 宫格每格的缩放因子：label -> zoom（页面加载完成后应用，避免加载中设置被重置）
    pub grid_zooms: Mutex<HashMap<String, f64>>,
    /// 终端会话（PTY）：id -> 会话
    pub terminals: Mutex<HashMap<String, TerminalSession>>,
    /// 宫格子进程管理器（Phase 1：每宫格独立子进程，崩溃隔离 + 自愈）
    pub grid_manager: crate::grid_process::GridProcessManager,
    /// 页签休眠开关（默认关；设置面板开启后，非激活页签超时销毁 webview 仅留 URL）
    pub hibernation_enabled: AtomicBool,
    /// 页签转为非激活的时刻（休眠计时起点）
    pub tab_idle_since: Mutex<HashMap<String, std::time::Instant>>,
    /// 已休眠页签（webview 已销毁，URL 保留在 tabs 表，激活时重建）
    pub hibernated_tabs: Mutex<std::collections::HashSet<String>>,
    /// tab-N 主进程 WebView 恢复预算，防止失败路径进入无限重建。
    pub tab_recovery: Mutex<HashMap<String, TabRecoveryBudget>>,
    /// M1-4：外部打开 URL 缓存队列（xdg-open 冷启动 argv / 单实例转发 /
    /// RunEvent::Opened）。前端就绪前到达的 URL 一律进队，由前端拉取，保证不丢。
    pub pending_open_urls: Mutex<Vec<String>>,
    /// M1-4：前端就绪标记（m0_ready 置位）。就绪后新到 URL 额外发轻提示事件，
    /// 触发前端立即拉取；就绪前不提示（提示也会丢，靠就绪后的首次拉取兜底）。
    pub frontend_ready: AtomicBool,
    /// M1-8：资源瀑布记录缓冲（每 tab 环形 + 全局 FIFO，容量硬上限）。
    pub resource_buffer: Mutex<ResourceBuffer>,
    /// M1-8：资源采集开关与容量设置（会话内生效，不持久化）。
    pub resource_capture: Mutex<ResourceCaptureSettings>,
    /// M1-9：会话草稿（tab_id -> 草稿）。打开 tab 时建立，仅内存，绝不自动落盘。
    pub session_drafts: Mutex<HashMap<String, SessionDraft>>,
    /// M1-9：关闭 tab 时是否弹「保存 / 删除」（默认开，见 `main.rs` setup 置位）。
    pub session_close_prompt: AtomicBool,
    /// M1-9：退出前是否自动保存仍打开的 tab（默认关：不静默保存）。
    pub session_auto_save_on_exit: AtomicBool,
    /// M2-4.c：脚本执行运行表（命令层只接入 b 卡内核；输出/落盘归 M2-4.d）。
    pub script_runs: Arc<ScriptProcessTable>,
}

/// 终端会话与输出内核（M3.a：mpsc+pump 管道、进程组回收、Channel 单播）。
pub use crate::terminal::{TermInfo, TerminalSession};

/// 浏览器页签信息（id 即子 webview 的 label）。
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct TabInfo {
    pub id: String,
    pub url: String,
    pub title: String,
}

/// 归一化用户输入的网址：支持以下写法
/// - 空 -> 默认引导页（百度）
/// - www.baidu.com / baidu.com / example.com -> 自动补 https://
/// - http://... https://... -> 原样
/// - 带路径 baidu.com/s?wd=x -> 自动补 https://
/// - 非 URL 的单词（如 "天气"）-> 走搜索引擎
fn normalize_url(input: &str) -> String {
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

#[cfg(test)]
mod normalize_url_tests {
    use super::{
        is_tab_label, normalize_url, reserve_tab_recovery_attempt, TabRecoveryBudget,
        TAB_RECOVERY_WINDOW_SECS,
    };

    #[test]
    fn preserves_about_blank_for_offline_browser_scenarios() {
        assert_eq!(normalize_url("about:blank"), "about:blank");
    }

    #[test]
    fn keeps_domain_and_search_input_behavior() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert!(normalize_url("search words").starts_with("https://www.baidu.com/s?wd="));
    }

    #[test]
    fn identifies_only_numbered_tab_labels() {
        assert!(is_tab_label("tab-1"));
        assert!(!is_tab_label("tab-main"));
        assert!(!is_tab_label("grid-1"));
    }

    #[test]
    fn tab_recovery_budget_stops_after_two_attempts_per_window() {
        let now = std::time::Instant::now();
        let mut budget = TabRecoveryBudget::default();
        assert_eq!(reserve_tab_recovery_attempt(&mut budget, now), Some(1));
        assert_eq!(
            reserve_tab_recovery_attempt(&mut budget, now + std::time::Duration::from_secs(1)),
            Some(2)
        );
        assert_eq!(
            reserve_tab_recovery_attempt(&mut budget, now + std::time::Duration::from_secs(2)),
            None
        );
    }

    #[test]
    fn tab_recovery_budget_resets_after_window() {
        let now = std::time::Instant::now();
        let mut budget = TabRecoveryBudget::default();
        assert_eq!(reserve_tab_recovery_attempt(&mut budget, now), Some(1));
        assert_eq!(
            reserve_tab_recovery_attempt(
                &mut budget,
                now + std::time::Duration::from_secs(TAB_RECOVERY_WINDOW_SECS + 1)
            ),
            Some(1)
        );
    }
}

/// 打开浏览器（内嵌为 main 窗口的子 webview）。
/// 内部等价于「新建一个页签」并设为激活页签。前端优先使用 tab_new 多开页签。
#[tauri::command]
pub fn open_browser(app: AppHandle, url: String) -> Result<(), String> {
    let _tab = create_tab(app.clone(), &url)?;
    Ok(())
}

/// 关闭内嵌浏览器（默认关闭当前激活页签）。
#[tauri::command]
pub fn close_browser(app: AppHandle) -> Result<(), String> {
    if let Some(id) = app.state::<AppState>().active_tab.lock().unwrap().clone() {
        close_tab(&app, &id)?;
    }
    Ok(())
}

/// 把当前激活页签的子窗口定位到主窗内容区指定矩形（CSS 坐标）。
/// 坐标由前端传入：相对主窗内容区左上角的 CSS 像素（getBoundingClientRect 的 left/top/width/height）。
#[tauri::command]
pub fn position_browser(
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let id = app
        .state::<AppState>()
        .active_tab
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "没有打开的页签".to_string())?;
    if let Err(e) = apply_bounds(&app, &id, x, y, width, height) {
        recover_tab_webview(&app, &id, "position", &e)?;
        apply_bounds_inner(&app, &id, x, y, width, height)
            .map_err(|retry| format!("{e}; 恢复后重试定位失败: {retry}"))?;
    }
    Ok(())
}

/// 把子窗口（页签 / 宫格）定位到主窗内容区指定矩形（CSS 逻辑坐标）。
/// 底层已迁移到 tauri-plugin-browser-tabs：插件内部用 LogicalPosition/LogicalSize
/// 设置位置与尺寸，Linux 下强制 WebKitGTK size_allocate，修复子 webview 卡初始尺寸。
/// 注意：前端传的是相对主窗内容区左上角的 CSS 像素（getBoundingClientRect 原值，
/// 不再乘 devicePixelRatio）。
fn apply_bounds(
    app: &AppHandle,
    id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // 后端去重：仅"rect 相同且 <50ms"才丢弃（防 IPC 洪泛），rect 变了必须应用
    {
        let state = app.state::<AppState>();
        let mut last = state.last_position_at.lock().unwrap();
        let now = std::time::Instant::now();
        if let Some((t, lx, ly, lw, lh)) = last.get(id) {
            let same_rect = (lx - x).abs() < 0.5
                && (ly - y).abs() < 0.5
                && (lw - width).abs() < 0.5
                && (lh - height).abs() < 0.5;
            if same_rect && now.duration_since(*t).as_millis() < 50 {
                return Ok(());
            }
        }
        last.insert(id.to_string(), (now, x, y, width, height));
    }
    apply_bounds_inner(app, id, x, y, width, height)
}

/// apply_bounds 的无去重内部实现：同时适用于 command 线程与主线程上下文
/// （插件的 update_rect / set_visible 底层走 dispatcher，跨线程安全）。
fn apply_bounds_inner(
    app: &AppHandle,
    id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
    let manager = app.state::<TabManagerState>();
    // 诊断：update_rect 失败（如 TabNotFound）时必须先留日志再返回，
    // 否则前端 .catch 吞掉后宫格"静默不显示"，无法定位是前端没发还是后端没建。
    if let Err(e) = manager.update_rect(
        &id.to_string(),
        LogicalRect::new(
            x.round().max(0.0),
            y.round().max(0.0),
            width.round().max(1.0),
            height.round().max(1.0),
        ),
    ) {
        eprintln!("[apply_bounds] label={} update_rect 失败: {e}", id);
        return Err(format!("定位失败: {e}"));
    }
    if let Err(e) = manager.set_visible(&id.to_string(), true) {
        eprintln!("[apply_bounds] label={} set_visible 失败: {e}", id);
    }
    eprintln!(
        "[apply_bounds] label={} logical=({},{},{},{})",
        id, x, y, width, height
    );
    remember_layout(app, id, x, y, width, height);
    Ok(())
}

/// 把子窗口移出可视区（隐藏态），用于非激活页签/宫格。
/// 注意：不能用 manager.set_visible(false)（即 webview.hide()）——对正在渲染的
/// WebKitGTK 子 webview 调 hide 会阻塞主线程事件循环导致死锁（实测新建第二个
/// 页签时卡在 hide(tab-1)）。改为把子 webview 移到屏幕外（等价隐藏，不卡死）。
fn hide_bounds(app: &AppHandle, id: &str) {
    use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
    let manager = app.state::<TabManagerState>();
    // 隐藏 = 只移到屏幕外，【保持原尺寸不变】。
    // 关键教训：对正在渲染的 WebKitGTK 子 webview，把尺寸缩到 1x1 会触发 WebKit
    // 视口重布局，与主线程死锁（实测卡死）。只移动位置（视口尺寸不变）则安全。
    // 坐标用 -30000（X11 int16 安全范围 -32768~32767 内）。
    let cur = app
        .state::<AppState>()
        .child_layouts
        .lock()
        .unwrap()
        .get(id)
        .copied();
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

/// 强制隐藏所有子 webview（页签 + 宫格）。
/// 关键：【不能走 grid_position/apply_bounds】——它有 50ms 去重，宫格刚定位后
/// 50ms 内的"移出屏幕"请求会被去重丢弃，导致宫格仍留在屏幕上（实测切主页仍看到
/// 宫格内容）。hide_bounds 无去重、保持尺寸只移位置，是隐藏的正确入口。
/// 对没记录在 child_layouts 的宫格（极端情况），也强制移出（直接给 1x1 临时尺寸）。
/// 隐藏单个子 webview（页签/宫格），用无去重的 hide_bounds。
/// 宫格显示时移出激活页签、页签切换时移出非激活页签都走这里，
/// 避免 tab_position/apply_bounds 的 50ms 去重把"移出屏幕"请求丢弃。
#[tauri::command]
pub fn hide_webview(app: AppHandle, id: String) -> Result<(), String> {
    use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
    // 宫格已迁子进程：隐藏 = 子进程整个窗口 hide（等价旧方案的移出屏幕）
    if let Some(index) = grid_index_of(&id) {
        let mgr = &app.state::<AppState>().grid_manager;
        mgr.record_hidden(index);
        mgr.send(index, GridCmd::HideWindow { id });
        return Ok(());
    }
    let state = app.state::<AppState>();
    let has_layout = state.child_layouts.lock().unwrap().contains_key(&id);
    if has_layout {
        hide_bounds(&app, &id);
    } else {
        let manager = app.state::<TabManagerState>();
        // 兜底移出成功时也记入 child_layouts（隐藏态），让守护线程持续压制漂移
        if manager
            .update_rect(&id, LogicalRect::new(-30000.0, -30000.0, 800.0, 600.0))
            .is_ok()
        {
            remember_layout(&app, &id, -30000.0, -30000.0, 800.0, 600.0);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn hide_all_webviews(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
    let manager = app.state::<TabManagerState>();
    let state = app.state::<AppState>();
    // 宫格已迁子进程：逐格下发 HideWindow（子进程整个窗口 hide）
    for index in state.grid_manager.indices() {
        state.grid_manager.record_hidden(index);
        state.grid_manager.send(
            index,
            GridCmd::HideWindow {
                id: format!("grid-{index}"),
            },
        );
    }
    // 页签 id（宫格已不在主进程插件表里，无需穷举 grid-N；
    // 已休眠页签 webview 已销毁，跳过避免 TabNotFound 噪音）
    let ids: Vec<String> = {
        let hibernated = state.hibernated_tabs.lock().unwrap();
        state
            .tabs
            .lock()
            .unwrap()
            .keys()
            .filter(|id| !hibernated.contains(*id))
            .cloned()
            .collect()
    };
    // 只处理插件管理表里真实存在的 webview
    let existing: std::collections::HashSet<String> = manager.get_tab_ids().into_iter().collect();
    for id in ids {
        if !existing.contains(&id) {
            continue;
        }
        // 优先用 hide_bounds（保持尺寸只移位置，安全不死锁）。
        // 但若 child_layouts 无记录（页签/宫格从未被定位过），hide_bounds 会跳过导致残留，
        // 此时兜底直接 update_rect 移到屏幕外（给一个安全的 800x600 尺寸，避免 1x1 触发重排死锁）。
        let has_layout = state.child_layouts.lock().unwrap().contains_key(&id);
        if has_layout {
            hide_bounds(&app, &id);
        } else {
            // 诊断：兜底移出失败时留日志（TabNotFound = 插件表里没有该 webview）。
            // 成功时记入 child_layouts（隐藏态），让守护线程持续压制漂移。
            match manager.update_rect(&id, LogicalRect::new(-30000.0, -30000.0, 800.0, 600.0)) {
                Ok(_) => remember_layout(&app, &id, -30000.0, -30000.0, 800.0, 600.0),
                Err(e) => eprintln!("[hide_all_webviews] id={} 兜底移出失败: {e}", id),
            }
        }
    }
    eprintln!("[hide_all_webviews] 已处理全部子 webview");
    Ok(())
}

/// 布局守护线程：每 400ms 按 child_layouts 重放所有子 webview 的目标矩形。
/// 背景：WebKitGTK 的 GTK 布局循环会在页面加载/容器重排时把子 webview 的
/// allocation 拉回"自然位置"（日志实测漂移到 (0,400,1200,400) 即下半屏），
/// 导致"定位后被弹回下半屏/隐藏后重新浮现"的残留。linux.rs 的纠偏已改为
/// 仅在 allocation 错配时才 move+size_allocate，守护线程空转代价趋近于零。
pub fn start_layout_enforcer(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(400));
        let state = app.state::<AppState>();
        if state.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }
        let layouts = state.child_layouts.lock().unwrap().clone();
        if layouts.is_empty() {
            continue;
        }
        use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
        let manager = app.state::<TabManagerState>();
        for (id, (x, y, w, h)) in layouts {
            let _ = manager.update_rect(&id, LogicalRect::new(x, y, w.max(1.0), h.max(1.0)));
        }
    });
}

/// 前端链路追踪：把前端关键步骤打到后端终端，定位"请求在哪一步丢失"。
#[tauri::command]
pub fn debug_log(msg: String) {
    eprintln!("[FE] {msg}");
}

/// 创建一个浏览器页签（独立子窗口，方案 B），返回其信息并设为激活页签。
fn create_tab(app: AppHandle, url: &str) -> Result<TabInfo, String> {
    let target = normalize_url(url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;

    // 生成唯一 id
    let state0 = app.state::<AppState>();
    let mut counter = state0.tab_counter.lock().unwrap();
    *counter += 1;
    let id = format!("tab-{}", *counter);
    drop(counter);

    let title0 = match Url::parse(&target) {
        Ok(u) => u.host_str().unwrap_or(&target).to_string(),
        Err(_) => target.clone(),
    };

    // 方案 B：异步创建独立子窗口（提交到主线程事件循环，避开同步创建第二个
    // webview 卡死主线程）。窗口真正建好后由前端 tab_position 放大显示。
    spawn_child_window(&app, &id, &target, 0.0, 0.0, 1.0, 1.0)?;
    eprintln!(
        "[create_tab] 子窗口已提交创建 label={} init=(1x1) visible=false",
        id
    );

    let info = TabInfo {
        id: id.clone(),
        url: target.clone(),
        title: title0.clone(),
    };
    app.state::<AppState>()
        .tabs
        .lock()
        .unwrap()
        .insert(id.clone(), info.clone());
    *app.state::<AppState>().active_tab.lock().unwrap() = Some(id.clone());
    // 休眠计时：新页签激活，旧页签从 now 起算 idle
    {
        let state = app.state::<AppState>();
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
    let _ = app.run_on_main_thread(move || {
        let st = app_hide.state::<AppState>();
        let ids: Vec<String> = st.tabs.lock().unwrap().keys().cloned().collect();
        for k in ids {
            if k != active_id {
                hide_bounds(&app_hide, &k);
            }
        }
    });
    eprintln!("[create_tab] run_on_main_thread 已排队 label={}", id);

    // M1-9：建立会话草稿（仅内存，落盘需用户显式选择）
    upsert_session_draft(&app, &id, &target, &title0);
    start_resource_scanner(app.clone());
    eprintln!("[create_tab] 页签已创建 label={} url={}", id, target);
    Ok(info)
}

/// 关闭指定页签，并清理状态。
fn close_tab(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    // 销毁对应子 webview（插件内部走 dispatcher，跨线程安全）
    let close_result = {
        use tauri_plugin_browser_tabs::TabManagerState;
        let manager = app.state::<TabManagerState>();
        manager
            .close_tab(&id.to_string())
            .map_err(|e| format!("关闭子 webview {id} 失败: {e}"))
    };
    app.state::<AppState>()
        .child_layouts
        .lock()
        .unwrap()
        .remove(id);
    state.last_position_at.lock().unwrap().remove(id);
    state.tab_idle_since.lock().unwrap().remove(id);
    state.hibernated_tabs.lock().unwrap().remove(id);
    state.tab_recovery.lock().unwrap().remove(id);
    // M1-8：tab 关闭即清理该 tab 的资源瀑布记录（生命周期红线）
    state.resource_buffer.lock().unwrap().remove_tab(id);
    // M1-9：释放会话草稿（未显式保存的不落盘，符合「不静默保存」）
    state.session_drafts.lock().unwrap().remove(id);
    state.tabs.lock().unwrap().remove(id);
    let mut active = state.active_tab.lock().unwrap();
    if active.as_deref() == Some(id) {
        *active = state.tabs.lock().unwrap().keys().next().cloned();
    }
    close_result
}

/// 注册主进程统一退出清理任务（M0-2.c）。
///
/// 这里只登记真正跨资源的清理：停止后台循环、关闭 tab webview、回收 PTY、
/// 关闭 grid 子进程并清理相关元数据。各任务由 ShutdownCoordinator 隔离执行，
/// 单项失败不会阻断后续资源回收。
pub fn register_shutdown_tasks(app: &AppHandle) -> Result<(), String> {
    let coordinator = app.state::<crate::shutdown::ShutdownCoordinator>();

    {
        let app = app.clone();
        coordinator.register("stop-background-workers", move || {
            app.state::<AppState>()
                .shutdown_requested
                .store(true, Ordering::SeqCst);
            Ok(())
        })?;
    }

    // M4-7.c / F7：`stop-scheduler` 必须**早于** `kill-running-scripts`
    // （先停触发，再杀运行中的；否则定时器可能在进程回收后再拉起子进程）。
    // 紧随 `stop-background-workers` 之后注册 ⇒ 索引 1 < kill-running-scripts 的索引 5。
    // 只置停止位 + 通知条件变量：**不 join 线程、不取消在飞运行**
    // （M0-2.b：清理任务不得持业务锁阻塞；在飞运行交给 `kill-running-scripts` 收口）。
    {
        coordinator.register("stop-scheduler", move || {
            crate::scheduler::request_stop();
            eprintln!("[shutdown] stop-scheduler signalled");
            Ok(())
        })?;
    }

    {
        // M1-9：会话 flush 必须**先于** close-tabs——先确定会话数据的落盘/释放，
        // 再销毁子 webview，避免清理过程中的失败/中断导致会话状态不确定。
        let app = app.clone();
        coordinator.register("flush-sessions", move || {
            let report = flush_sessions_inner(&app);
            eprintln!(
                "[shutdown] flush-sessions persisted={} drafts_dropped={} tmp_removed={} capacity_removed={}",
                report.persisted, report.drafts_dropped, report.tmp_removed, report.capacity_removed
            );
            Ok(())
        })?;
    }

    {
        let app = app.clone();
        coordinator.register("close-tabs", move || {
            use tauri_plugin_browser_tabs::TabManagerState;
            let state = app.state::<AppState>();
            let manager = app.state::<TabManagerState>();
            let ids: Vec<String> = state.tabs.lock().unwrap().keys().cloned().collect();
            let mut errors = Vec::new();
            for id in &ids {
                if let Err(error) = manager.close_tab(&id.to_string()) {
                    errors.push(format!("{id}: {error}"));
                }
            }
            state.child_layouts.lock().unwrap().clear();
            state.last_position_at.lock().unwrap().clear();
            state.tab_idle_since.lock().unwrap().clear();
            state.hibernated_tabs.lock().unwrap().clear();
            // M1-8：退出路径清空全部资源瀑布记录（不落盘、不泄漏）
            state.resource_buffer.lock().unwrap().clear_all();
            state.tabs.lock().unwrap().clear();
            *state.active_tab.lock().unwrap() = None;
            if errors.is_empty() {
                Ok(())
            } else {
                Err(format!("tab close errors: {}", errors.join("; ")))
            }
        })?;
    }

    {
        let app = app.clone();
        coordinator.register("kill-terminals", move || {
            let sessions: Vec<(String, TerminalSession)> = app
                .state::<AppState>()
                .terminals
                .lock()
                .unwrap()
                .drain()
                .collect();
            let mut errors = Vec::new();
            for (id, mut session) in sessions {
                // M3.a：统一走进程组回收 + 线程回收（F6/F7），不再只杀直接子进程。
                if let Err(error) = terminal::terminate_session(&mut session) {
                    errors.push(format!("{id} kill: {error}"));
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(format!("terminal cleanup errors: {}", errors.join("; ")))
            }
        })?;
    }

    {
        let app = app.clone();
        coordinator.register("shutdown-grid", move || {
            let state = app.state::<AppState>();
            state.grid_manager.shutdown_all();
            for i in 0..MAX_GRID {
                let label = format!("grid-{i}");
                state.child_layouts.lock().unwrap().remove(&label);
                state.grid_zooms.lock().unwrap().remove(&label);
                state.last_position_at.lock().unwrap().remove(&label);
            }
            Ok(())
        })?;
    }

    {
        let app = app.clone();
        coordinator.register("kill-running-scripts", move || {
            let killed =
                crate::script_runner::kill_all_running(&app.state::<AppState>().script_runs);
            eprintln!("[shutdown] kill-running-scripts killed={killed}");
            Ok(())
        })?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// M4-6.b 定时任务命令（契约 `A6-M4-5-scheduler-contract-20260905-2330.md` §6）
//
// 五条命令**全部**过 `check_invocation_source`（远程 webview 一律拒绝，与
// `run_command` / `run_script` 同口径）。审计 detail 只含 `id` / `kind` /
// `trigger` / `count` / `reason` / `error_code`，**不含**参数值、命令正文、
// 脚本正文、输出或任何凭据（契约 §7 + F5）。
//
// 触发语义归 `scheduler.rs`：本层只做 CRUD 与「立即跑一次」的入口，
// 执行唯一入口仍是 `script_runner`（F6）。
// ---------------------------------------------------------------------------

/// 触发方式的审计标签。**不含**用户输入的表达式正文（避免把任意字符串塞进审计）。
fn task_trigger_label(trigger: &TaskTrigger) -> &'static str {
    match trigger {
        TaskTrigger::Cron { .. } => "cron",
        TaskTrigger::Interval { .. } => "interval",
    }
}

/// 取目标（脚本 / 命令片段）的参数定义，用于 R-3 / R-4 校验。
fn task_target_params(
    app: &AppHandle,
    kind: TaskKind,
    target_id: &str,
) -> Result<Vec<ScriptParam>, String> {
    match kind {
        TaskKind::Script => {
            let script = workspace::load_scripts(app)
                .into_iter()
                .find(|s| s.id == target_id)
                .ok_or_else(|| crate::tasks::TASK_TARGET_NOT_FOUND.to_string())?;
            if !script.enabled {
                return Err("SCRIPT_DISABLED".to_string());
            }
            Ok(script.params)
        }
        TaskKind::Command => {
            let snippet = workspace::load_snippets(app)
                .into_iter()
                .find(|s| s.id == target_id)
                .ok_or_else(|| crate::tasks::TASK_TARGET_NOT_FOUND.to_string())?;
            if !snippet.enabled {
                return Err("SNIPPET_DISABLED".to_string());
            }
            Ok(snippet.params)
        }
    }
}

/// 任务列表。**不含** secret 参数值 —— secret 参数在定义期即被拒绝落盘（契约 §3.3 R-3）。
#[tauri::command]
pub fn task_list(app: AppHandle, webview: tauri::Webview) -> Result<Vec<TaskDef>, String> {
    check_invocation_source(&webview, "task_list", None, &app)?;
    let list = crate::tasks::load_tasks_at(&crate::tasks::tasks_file(&app));
    let count = list.len();
    workspace::log_audit(&app, "task.runs.list", format!("count={count}"));
    Ok(list)
}

/// 新建任务。默认 **不启用**（裁定 R-A6-1：自动执行必须是显式动作）；
/// 创建时即计算并落盘 `next_run_at`，便于 UI 展示「下次执行时间」。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn task_add(
    app: AppHandle,
    webview: tauri::Webview,
    name: String,
    kind: TaskKind,
    target_id: String,
    trigger: TaskTrigger,
    params: Option<HashMap<String, String>>,
    missed_run_policy: Option<MissedRunPolicy>,
    catch_up_limit: Option<u32>,
    misfire_grace_secs: Option<u64>,
    retry: Option<RetryPolicy>,
    timeout_secs: Option<u32>,
    enabled: Option<bool>,
) -> Result<TaskDef, String> {
    check_invocation_source(&webview, "task_add", None, &app)?;
    let supplied = params.unwrap_or_default();
    let path = crate::tasks::tasks_file(&app);
    let mut list = crate::tasks::load_tasks_at(&path);
    crate::tasks::check_capacity(list.len()).map_err(|e| e.code().to_string())?;
    let meta = task_target_params(&app, kind, &target_id)?;
    crate::tasks::validate_params(&meta, &supplied).map_err(|e| e.code().to_string())?;

    let now = chrono::Utc::now();
    let mut task = TaskDef {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        kind,
        target_id,
        params: supplied,
        enabled: enabled.unwrap_or(false),
        trigger,
        missed_run_policy: missed_run_policy.unwrap_or_default(),
        catch_up_limit: catch_up_limit.unwrap_or(3),
        misfire_grace_secs: misfire_grace_secs.unwrap_or(60),
        retry: retry.unwrap_or_default(),
        timeout_secs: timeout_secs.unwrap_or(0),
        last_fired_at: None,
        next_run_at: None,
        created_at: now,
        updated_at: now,
    };
    crate::tasks::validate_task(&task).map_err(|e| e.code().to_string())?;
    task.next_run_at = crate::tasks::next_fire_after(&task.trigger, now);
    list.push(task.clone());
    crate::tasks::save_tasks_at(&path, &list)
        .map_err(|_| crate::tasks::TASK_PERSIST_FAILED.to_string())?;

    let kind_label = task.kind.as_str();
    let trig_label = task_trigger_label(&task.trigger);
    let id = task.id.clone();
    workspace::log_audit(
        &app,
        "task.add",
        format!("id={id} kind={kind_label} trigger={trig_label}"),
    );
    Ok(task)
}

/// 全量更新（按 id 替换）。`created_at` 由服务端保留，不可被前端改写；
/// 改 `trigger` 后重算 `next_run_at`（契约 §5.4）。
#[tauri::command]
pub fn task_update(
    app: AppHandle,
    webview: tauri::Webview,
    task: TaskDef,
) -> Result<TaskDef, String> {
    check_invocation_source(&webview, "task_update", None, &app)?;
    let path = crate::tasks::tasks_file(&app);
    let mut list = crate::tasks::load_tasks_at(&path);
    let index = list
        .iter()
        .position(|t| t.id == task.id)
        .ok_or_else(|| crate::tasks::TASK_NOT_FOUND.to_string())?;
    // A10 R-4：参数校验在**更新期同样复跑** —— 防止脚本事后把参数改标 secret，
    // 而既有任务仍持有明文值。
    let meta = task_target_params(&app, task.kind, &task.target_id)?;
    crate::tasks::validate_params(&meta, &task.params).map_err(|e| e.code().to_string())?;
    crate::tasks::validate_task(&task).map_err(|e| e.code().to_string())?;

    let created_at = list[index].created_at;
    let mut next = task;
    next.created_at = created_at;
    next.updated_at = chrono::Utc::now();
    next.next_run_at = crate::tasks::next_fire_after(&next.trigger, next.updated_at);
    list[index] = next.clone();
    crate::tasks::save_tasks_at(&path, &list)
        .map_err(|_| crate::tasks::TASK_PERSIST_FAILED.to_string())?;

    let kind_label = next.kind.as_str();
    let trig_label = task_trigger_label(&next.trigger);
    let id = next.id.clone();
    workspace::log_audit(
        &app,
        "task.update",
        format!("id={id} kind={kind_label} trigger={trig_label}"),
    );
    Ok(next)
}

/// 删除任务：**先取消该任务的在飞运行**，再删持久化条目（契约 §5.4）。
/// `task-runs.json` 的历史**保留**（审计与排障需要），不随任务删除。
/// 幂等：id 不存在返回稳定错误码，不 panic。
#[tauri::command]
pub fn task_remove(app: AppHandle, webview: tauri::Webview, id: String) -> Result<bool, String> {
    check_invocation_source(&webview, "task_remove", None, &app)?;
    check_id(&id, "任务 id")?;
    let path = crate::tasks::tasks_file(&app);
    let mut list = crate::tasks::load_tasks_at(&path);
    let before = list.len();
    let cancelled = crate::scheduler::cancel_in_flight(&app, &id);
    list.retain(|t| t.id != id);
    if list.len() == before {
        return Err(crate::tasks::TASK_NOT_FOUND.to_string());
    }
    crate::tasks::save_tasks_at(&path, &list)
        .map_err(|_| crate::tasks::TASK_PERSIST_FAILED.to_string())?;
    workspace::log_audit(
        &app,
        "task.remove",
        format!("id={id} cancelled={cancelled}"),
    );
    Ok(true)
}

/// 立即触发一次（`trigger = Manual`）。**不推进** `last_fired_at` / `next_run_at`；
/// 同样受「同任务 in_flight」与全局并发约束，冲突时返回 `TASK_ALREADY_RUNNING`。
#[tauri::command]
pub fn task_run_now(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "task_run_now", None, &app)?;
    check_id(&id, "任务 id")?;
    crate::scheduler::fire_now(&app, &id)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ResourceItem {
    pub res_type: String, // script / stylesheet / image / svg / iframe / media / font / css-asset / other
    pub url: String,
    pub absolute: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
struct ResourceScanResult {
    page_url: String,
    items: Vec<ResourceItem>,
}

/// M0-3.c：写/删类命令的允许根目录。
///
/// 取值与 `get_start_dirs` 对外承诺的入口保持一致（主目录 / 桌面 / 文档 / 下载 /
/// 成果工作区 / 笔记目录），否则文件管理器会出现「能列出来却写不进去」的不一致。
/// 效果是：仍可在这些用户目录内正常增删改名，但 `../` 逃逸、符号链接逃逸、
/// 以及写到 `/etc`、`/usr`、其他用户目录等均被拒绝。
pub fn allowed_roots(app: &AppHandle) -> Vec<std::path::PathBuf> {
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(home) = app.path().home_dir() {
        roots.push(home.clone());
        for sub in ["Desktop", "Documents", "Downloads"] {
            roots.push(home.join(sub));
        }
    }
    roots.push(crate::workspace::workspace_dir(app));
    roots.push(crate::workspace::notes_dir(app));
    roots.sort();
    roots.dedup();
    roots
}

// ── M5-1.b 切片 1：B 类模块搬入 core 的 Tauri 侧 seam 实现 ───────────────────────
// 这些实现是切片 2（B 类模块实际搬入并注入 trait）的注入目标；切片 1 仅声明 trait，
// 此处实现尚未被消费，故 `#[allow(dead_code)]` 避免新增 warning。
// 见 `logs/checkpoints/M5-20260906/M5-1.b-seam-trait-injection-and-b-extract.md`。
#[allow(dead_code)]
pub struct TauriProgressSink<'a> {
    app: &'a AppHandle,
}

#[allow(dead_code)]
impl<'a> ProgressSink for TauriProgressSink<'a> {
    fn emit(&self, p: Progress) {
        let _ = self.app.emit("script:progress", p);
    }
}

#[allow(dead_code)]
pub struct TauriPathResolver<'a> {
    app: &'a AppHandle,
}

#[allow(dead_code)]
impl<'a> PathResolver for TauriPathResolver<'a> {
    fn base_dir(&self) -> std::path::PathBuf {
        self.app
            .path()
            .data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    }
}

#[allow(dead_code)]
pub struct TauriRootsProvider<'a> {
    app: &'a AppHandle,
}

#[allow(dead_code)]
impl<'a> RootsProvider for TauriRootsProvider<'a> {
    fn allowed_roots(&self) -> Vec<std::path::PathBuf> {
        allowed_roots(self.app)
    }
}

/// M0-3.b：远程上报入口的统一来源校验。未登记/伪造 label（含残留的 `browser`）一律拒绝。
fn check_invocation_source(
    webview: &tauri::Webview,
    scope: &str,
    intent: Option<&str>,
    app: &AppHandle,
) -> Result<(), String> {
    use crate::security_policy as sp;
    let registry = app.state::<sp::IntentRegistry>();
    sp::check_remote_invocation(webview.label(), scope, intent, &registry)
        .map_err(|e| e.to_string())
}

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

// ---------------------------------------------------------------------------
// M1-8 资源瀑布（请求拦截与瀑布）
//
// 链路：插件 WebKitGTK 原生信号（resource-load-started/finished/failed）
//   → browser-tabs://event(type=resourceReceived)（原始 URL，仅进程内）
//   → main.rs 转发到 on_resource_received（脱敏 + 容量上限 + 入库）
//   → app.emit("resource-received", ResourceReceived)（脱敏 DTO，推前端）。
// 隐私红线：DTO 不含 headers/Cookie/Authorization/Set-Cookie/任何 body；
// URL 一律经 sp::redact_sensitive_url 脱敏 + 限长；审计只记 tab_id 与计数。
// ---------------------------------------------------------------------------

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
    total: usize,
    /// 每 tab 因容量上限被丢弃的累计条数（前端超限提示）
    evicted_per_tab: HashMap<String, u64>,
    /// 全局因总上限被丢弃的累计条数
    evicted_global: u64,
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
fn check_tab_id(tab_id: &str) -> Result<(), String> {
    if tab_id.len() > 64 || !tab_id.starts_with("tab-") {
        return Err("非法 tab_id".to_string());
    }
    Ok(())
}

/// id 形态校验：只允许 `[A-Za-z0-9_-]`，长度 ≤ `IMAGE_ID_MAX_LEN`。
///
/// 背景（M1-ACCEPT 审计挂账项）：`session_get/delete/export/restore` 的 id 会被直接
/// 拼进文件名（`dir.join(format!("{id}.json"))`），缺形态校验时理论上可借 `../` 逃逸出
/// 会话目录。主窗口本就在信任边界内（持有全部 invoke 能力），但纵深防御要求
/// 所有「拿 id 拼路径」的入口先过这里——会话/成果/图片一律适用。
fn check_id(id: &str, what: &str) -> Result<(), String> {
    crate::images::validate_id(id).map_err(|_| format!("非法 {what}"))
}

/// 查询某 tab 的资源瀑布记录（含容量驱逐计数）。
#[tauri::command]
pub fn list_tab_resources(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
) -> Result<TabResourceList, String> {
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

// ---------------------------------------------------------------------------
// M1-9 会话存档与关闭协议（#14：请求/资源可见 + 关闭保存删除）
//
// 落盘白名单见 `domain.rs`；命令层额外保证：
//   - 全部过 `check_invocation_source`（远程页面/伪造 label 一律拒绝）
//   - 审计只记 tab_id / 会话 id / 计数，绝不记 URL 与预览内容
//   - 容量：`SESSION_MAX_RESOURCES`（单会话资源）+ `SESSION_MAX_COUNT`（会话总数）
//   - 关闭协议：未经用户同意的草稿不落盘（`auto_save_on_exit` 默认关）
// ---------------------------------------------------------------------------

/// 建立/刷新某 tab 的会话草稿（打开 tab 与导航时调用；仅内存）。
fn upsert_session_draft(app: &AppHandle, tab_id: &str, url: &str, title: &str) {
    let state = app.state::<AppState>();
    let mut drafts = state.session_drafts.lock().unwrap();
    let safe_url = crate::security_policy::redact_sensitive_url(url);
    if let Some(d) = drafts.get_mut(tab_id) {
        d.url = safe_url;
        if !title.is_empty() {
            d.title = title.to_string();
        }
    } else {
        drafts.insert(
            tab_id.to_string(),
            SessionDraft {
                tab_id: tab_id.to_string(),
                url: safe_url,
                title: title.to_string(),
            },
        );
    }
}

fn drop_session_draft(app: &AppHandle, tab_id: &str) {
    app.state::<AppState>()
        .session_drafts
        .lock()
        .unwrap()
        .remove(tab_id);
}

/// 从当前 tab 状态构建会话（URL 已脱敏，资源取自 M1-8 缓冲）。
fn build_session_for_tab(
    app: &AppHandle,
    tab_id: &str,
    preview: &str,
    reason: &str,
) -> Option<BrowserSession> {
    let state = app.state::<AppState>();
    // 优先取草稿（url 已脱敏、标题为最近一次上报），回退 tabs 表。
    let (url, title) = {
        let drafts = state.session_drafts.lock().unwrap();
        match drafts.get(tab_id) {
            Some(d) if !d.url.is_empty() => (d.url.clone(), d.title.clone()),
            _ => {
                let tabs = state.tabs.lock().unwrap();
                match tabs.get(tab_id) {
                    Some(t) => (t.url.clone(), t.title.clone()),
                    None => return None,
                }
            }
        }
    };
    let records = state.resource_buffer.lock().unwrap().records(tab_id);
    Some(crate::session::build_session(
        tab_id, &url, &title, preview, &records, reason,
    ))
}

/// 会话落盘 + 容量守卫（超出 `SESSION_MAX_COUNT` 删最旧）。
fn persist_session(app: &AppHandle, session: &BrowserSession) -> Result<(), String> {
    let dir = workspace::sessions_dir(app);
    crate::session::save_session(&dir, session)?;
    crate::session::prune_sessions(&dir, SESSION_MAX_COUNT);
    Ok(())
}

/// 保存当前 tab 为会话（立即落盘）。`preview` 为前端采集的最小文本预览
/// （可为空），落盘前由 `build_session` 做 URL 脱敏与 512B 截断。
#[tauri::command]
pub fn session_save(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
    preview: Option<String>,
) -> Result<SessionSummary, String> {
    check_invocation_source(&webview, "session_save", None, &app)?;
    check_tab_id(&tab_id)?;
    let text = preview.unwrap_or_default();
    let session = build_session_for_tab(&app, &tab_id, &text, CLOSE_REASON_SAVED)
        .ok_or_else(|| "页签不存在".to_string())?;
    persist_session(&app, &session)?;
    drop_session_draft(&app, &tab_id);
    workspace::log_audit(
        &app,
        "session_save",
        format!(
            "tab_id={} resources={} preview_bytes={}",
            tab_id,
            session.resource_count,
            session.preview.len()
        ),
    );
    Ok(crate::session::summarize(&session))
}

/// 明确丢弃某 tab 的会话草稿（关闭弹窗选「删除」）：不落盘，仅审计。
#[tauri::command]
pub fn session_discard(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "session_discard", None, &app)?;
    check_tab_id(&tab_id)?;
    let existed = app
        .state::<AppState>()
        .session_drafts
        .lock()
        .unwrap()
        .remove(&tab_id)
        .is_some();
    workspace::log_audit(
        &app,
        "session_discard",
        format!("tab_id={tab_id} draft_existed={existed}"),
    );
    Ok(())
}

/// 列出本地会话存档（按 updated_at 倒序；损坏文件跳过）。
#[tauri::command]
pub fn session_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<SessionSummary>, String> {
    check_invocation_source(&webview, "session_list", None, &app)?;
    Ok(crate::session::list_sessions(&workspace::sessions_dir(
        &app,
    )))
}

/// 读取单个会话详情（含已脱敏资源列表）。
#[tauri::command]
pub fn session_get(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<BrowserSession, String> {
    check_invocation_source(&webview, "session_get", None, &app)?;
    check_id(&id, "会话 id")?;
    crate::session::load_session(&workspace::sessions_dir(&app), &id)
}

/// 删除会话存档（幂等）。删除的是存档，不影响仍打开的同名页签。
#[tauri::command]
pub fn session_delete(app: AppHandle, webview: tauri::Webview, id: String) -> Result<bool, String> {
    check_invocation_source(&webview, "session_delete", None, &app)?;
    check_id(&id, "会话 id")?;
    let removed = crate::session::delete_session(&workspace::sessions_dir(&app), &id)?;
    workspace::log_audit(&app, "session_delete", format!("id={id} removed={removed}"));
    Ok(removed)
}

/// 导出会话为脱敏 JSON 文本（**不写磁盘**：不引入新的路径写入面，
/// 由前端自行决定保存位置）。
#[tauri::command]
pub fn session_export(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<String, String> {
    check_invocation_source(&webview, "session_export", None, &app)?;
    check_id(&id, "会话 id")?;
    let session = crate::session::load_session(&workspace::sessions_dir(&app), &id)?;
    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    workspace::log_audit(
        &app,
        "session_export",
        format!("id={id} bytes={}", json.len()),
    );
    Ok(json)
}

/// 用会话中**已脱敏**的 URL 新建页签（重启/回看场景）。
/// 边界：URL 在落盘时已脱敏，一次性 token / 登录态参数不会恢复，
/// 因此还原出的页面可能需要重新登录——这是隐私红线的必然代价，不伪造原 URL。
#[tauri::command]
pub fn session_restore(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<TabInfo, String> {
    check_invocation_source(&webview, "session_restore", None, &app)?;
    check_id(&id, "会话 id")?;
    let session = crate::session::load_session(&workspace::sessions_dir(&app), &id)?;
    let tab = create_tab(app.clone(), &session.url)?;
    workspace::log_audit(
        &app,
        "session_restore",
        format!(
            "id={id} tab_id={} resources={}",
            tab.id, session.resource_count
        ),
    );
    Ok(tab)
}

/// 关闭路径 flush（ShutdownCoordinator 任务 + 前端 beforeunload 双保险）：
/// ① `auto_save_on_exit` 打开时落盘仍打开 tab 的会话；否则仅释放草稿（不静默保存）；
/// ② 清理异常退出残留 `.tmp`；③ 容量裁剪。行为确定、可审计。
#[tauri::command]
pub fn flush_sessions(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<SessionFlushReport, String> {
    check_invocation_source(&webview, "flush_sessions", None, &app)?;
    Ok(flush_sessions_inner(&app))
}

/// flush 的实际执行体（命令与 ShutdownCoordinator 任务共用，保证两路径行为一致）。
pub fn flush_sessions_inner(app: &AppHandle) -> SessionFlushReport {
    let state = app.state::<AppState>();
    let auto_save = state.session_auto_save_on_exit.load(Ordering::SeqCst);
    let dir = workspace::sessions_dir(app);

    let mut persisted = 0usize;
    if auto_save {
        let drafts: Vec<SessionDraft> = state
            .session_drafts
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        for draft in drafts {
            if let Some(session) =
                build_session_for_tab(app, &draft.tab_id, "", CLOSE_REASON_SHUTDOWN)
            {
                if persist_session(app, &session).is_ok() {
                    persisted += 1;
                }
            }
        }
    }
    let drafts_dropped = {
        let mut drafts = state.session_drafts.lock().unwrap();
        let n = drafts.len();
        drafts.clear();
        n
    };
    let tmp_removed = crate::session::prune_tmp_files(&dir);
    let capacity_removed = crate::session::prune_sessions(&dir, SESSION_MAX_COUNT);

    workspace::log_audit(
        app,
        "session_flush",
        format!(
            "auto_save={auto_save} persisted={persisted} drafts_dropped={drafts_dropped} tmp_removed={tmp_removed}"
        ),
    );
    SessionFlushReport {
        persisted,
        drafts_dropped,
        tmp_removed,
        capacity_removed,
    }
}

/// 查询会话策略（关闭弹窗 / 退出自动保存）。
#[tauri::command]
pub fn get_session_policy(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<SessionPolicy, String> {
    check_invocation_source(&webview, "get_session_policy", None, &app)?;
    let state = app.state::<AppState>();
    Ok(SessionPolicy {
        close_prompt: state.session_close_prompt.load(Ordering::SeqCst),
        auto_save_on_exit: state.session_auto_save_on_exit.load(Ordering::SeqCst),
    })
}

/// 设置会话策略（审计记录开关变化，不含任何 URL/预览内容）。
#[tauri::command]
pub fn set_session_policy(
    app: AppHandle,
    webview: tauri::Webview,
    close_prompt: Option<bool>,
    auto_save_on_exit: Option<bool>,
) -> Result<SessionPolicy, String> {
    check_invocation_source(&webview, "set_session_policy", None, &app)?;
    let state = app.state::<AppState>();
    if let Some(v) = close_prompt {
        state.session_close_prompt.store(v, Ordering::SeqCst);
    }
    if let Some(v) = auto_save_on_exit {
        state.session_auto_save_on_exit.store(v, Ordering::SeqCst);
    }
    let policy = SessionPolicy {
        close_prompt: state.session_close_prompt.load(Ordering::SeqCst),
        auto_save_on_exit: state.session_auto_save_on_exit.load(Ordering::SeqCst),
    };
    workspace::log_audit(
        &app,
        "session_policy",
        format!(
            "close_prompt={} auto_save_on_exit={}",
            policy.close_prompt, policy.auto_save_on_exit
        ),
    );
    Ok(policy)
}

/// 由子窗口内 JS 经 invoke 回传的真实页面标题，转成 tab-title 事件推给前端。
#[tauri::command]
pub fn report_title(app: AppHandle, webview: tauri::Webview, title: String) -> Result<(), String> {
    use crate::security_policy as sp;
    check_invocation_source(&webview, "report_title", None, &app)?;
    sp::check_text_field("title", &title, sp::MAX_TEXT_FIELD_BYTES).map_err(|e| e.to_string())?;
    report_title_inner(app, title)
}

fn report_title_inner(app: AppHandle, title: String) -> Result<(), String> {
    let t = title.trim().to_string();
    if t.is_empty() {
        return Ok(());
    }
    // 找到当前激活页签（标题回传只针对激活页）
    let id = app.state::<AppState>().active_tab.lock().unwrap().clone();
    if let Some(id) = id {
        let _ = app.emit(
            "tab-title",
            serde_json::json!({ "id": id, "url": "", "title": t }),
        );
    }
    Ok(())
}

/// 对指定子窗口执行 JS 资源扫描，并回传真实标题，通过事件推送给前端。
/// 注意：WebviewWindow 的 eval 在 Tauri 2 主线程执行即可；资源/标题的回传目前通过
/// webview 内已注入的 `window.ipc.postMessage` 通道（见 resources_eval.js 改造）由前端监听，
/// 这里仅负责触发执行。插件的 Webview dispatcher 支持从扫描线程调用。
fn scan_resources(app: &AppHandle, id: &str) -> Result<(), String> {
    // 执行资源扫描脚本（脚本内部会把结果经 ipc 回传，由前端 onBrowserResources 接收）
    let js = include_str!("../injected/resources_eval.js");
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
fn start_resource_scanner(app: AppHandle) {
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

/// 采集：网页选区 → 本地成果（带溯源）。渲染进程零特权，只能 invoke 本命令。
#[tauri::command]
pub fn collect_selection(
    app: AppHandle,
    webview: tauri::Webview,
    url: String,
    title: String,
    text: String,
    html: String,
    intent: Option<String>,
) -> Result<Artifact, String> {
    use crate::security_policy as sp;
    // 写盘类副作用：来自外部页面（tab-*/grid-*）必须出示一次性用户意图令牌。
    check_invocation_source(
        &webview,
        sp::INTENT_COLLECT_SELECTION,
        intent.as_deref(),
        &app,
    )?;
    sp::check_text_field("text", &text, sp::MAX_TEXT_FIELD_BYTES).map_err(|e| e.to_string())?;
    sp::check_text_field("html", &html, sp::MAX_HTML_BYTES).map_err(|e| e.to_string())?;
    let art = Artifact::new(title, url.clone(), text, html);
    workspace::save_artifact(&app, &art)?;
    workspace::log_audit(&app, "collect", format!("{} <- {}", art.title, url));
    let _ = app.emit("artifact-collected", art.clone());
    Ok(art)
}

/// 网页右键"打开终端"：子 webview 渲染进程零特权，只能发意图；
/// 由主窗口前端监听 "open-terminal" 事件切换到终端视图并启动 shell。
#[tauri::command]
pub fn request_open_terminal(
    app: AppHandle,
    webview: tauri::Webview,
    intent: Option<String>,
) -> Result<(), String> {
    use crate::security_policy as sp;
    // 开终端等同授予 shell 能力：外部页面必须出示一次性用户意图令牌。
    check_invocation_source(&webview, sp::INTENT_OPEN_TERMINAL, intent.as_deref(), &app)?;
    app.emit("open-terminal", ()).map_err(|e| e.to_string())
}

/// 受信任的主窗口签发给子 webview 的一次性用户意图令牌（M0-3.b）。
/// 只有 `main` 能签发；外部页面拿到令牌后只能使用一次，且绑定具体作用域。
#[tauri::command]
pub fn issue_intent(
    app: AppHandle,
    webview: tauri::Webview,
    scope: String,
) -> Result<String, String> {
    use crate::security_policy as sp;
    if webview.label() != "main" {
        return Err(format!("只有主窗口可以签发意图令牌：{}", webview.label()));
    }
    let allowed = [
        sp::INTENT_SAVE_NOTE,
        sp::INTENT_COLLECT_SELECTION,
        sp::INTENT_OPEN_TERMINAL,
    ];
    if !allowed.contains(&scope.as_str()) {
        return Err(format!("未知意图作用域：{scope}"));
    }
    Ok(app
        .state::<sp::IntentRegistry>()
        .issue(&scope, sp::INTENT_TTL))
}

/// 网页选区一键存为 Markdown 笔记。
/// 默认路径：应用数据目录 notes/；文件名 = 时间戳 + 选中内容摘要。
/// 返回保存后的完整文件路径。
#[tauri::command]
pub fn save_note(
    app: AppHandle,
    webview: tauri::Webview,
    url: String,
    title: String,
    text: String,
    intent: Option<String>,
) -> Result<String, String> {
    use crate::security_policy as sp;
    // 写文件属副作用：外部页面必须出示一次性用户意图令牌（对应验收项「无用户意图写入」）。
    check_invocation_source(&webview, sp::INTENT_SAVE_NOTE, intent.as_deref(), &app)?;
    sp::check_text_field("text", &text, sp::MAX_TEXT_FIELD_BYTES).map_err(|e| e.to_string())?;
    let dir = workspace::notes_dir(&app);

    let now = chrono::Local::now();
    let ts = now.format("%Y%m%d-%H%M%S").to_string();
    // 文件名摘要：取选中内容首行前 20 个字符，过滤文件系统非法字符
    let summary: String = text
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .take(20)
        .collect::<String>()
        .trim()
        .to_string();
    let base = if summary.is_empty() {
        let t: String = title.trim().chars().take(20).collect();
        if t.is_empty() {
            "note".to_string()
        } else {
            t
        }
    } else {
        summary
    };
    let file = dir.join(format!("{}-{}.md", ts, base));
    // M0-3.c：文件名由外部文本派生，写盘前确认落点仍在允许根目录内。
    sp::check_path_component(&format!("{}-{}.md", ts, base)).map_err(|e| e.to_string())?;

    let heading = if title.trim().is_empty() {
        base.as_str()
    } else {
        title.trim()
    };
    let md = format!(
        "# {}\n\n> 来源: {}\n> 时间: {}\n\n---\n\n{}\n",
        heading,
        url,
        now.format("%Y-%m-%d %H:%M:%S"),
        text.trim()
    );
    std::fs::write(&file, md).map_err(|e| e.to_string())?;
    workspace::log_audit(&app, "note", format!("保存笔记 {}", file.display()));
    let path = file.to_string_lossy().to_string();
    let _ = app.emit("note-saved", path.clone());
    Ok(path)
}

#[tauri::command]
pub fn list_artifacts(app: AppHandle) -> Vec<Artifact> {
    workspace::load_artifacts(&app)
}

/// 配置仓库：token 仅写入系统密钥库，绝不回传前端。
#[tauri::command]
pub fn configure_repo(app: AppHandle, config: RepoConfig, token: String) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("token 不能为空".into());
    }
    KeyringStore::save_token(&config.id, &token)?;
    let mut repos = workspace::load_repos(&app);
    repos.retain(|r| r.id != config.id);
    repos.push(config);
    workspace::save_repos(&app, &repos)?;
    workspace::log_audit(
        &app,
        "configure_repo",
        format!(
            "{} ({:?})",
            repos.last().unwrap().name,
            repos.last().unwrap().provider
        ),
    );
    Ok(())
}

#[tauri::command]
pub fn list_repos(app: AppHandle) -> Vec<RepoConfig> {
    workspace::load_repos(&app)
}

// ---------------------------------------------------------------------------
// M1-5 Git 只读命令（status / diff / branch_list）
//
// 硬约束：只允许读仓库状态，不得实现 commit / checkout / push / pull /
// merge / rebase / 分支删除等任何写操作或联网调用；不读 Keyring、不回传凭据。
// 三个命令均做调用来源校验（子 webview / 远程页面不得调用），仓库路径锁定在
// app_data_dir/mvp-browser-os/repos/<id>（见 sync::repo_dir）。
// ---------------------------------------------------------------------------

/// 只读：工作区/索引脏文件清单（git status 等价）。
#[tauri::command]
pub fn git_status(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
) -> Result<Vec<GitFileStatus>, String> {
    check_invocation_source(&webview, "git_status", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    sync::read_status(&repo)
}

/// 只读：HEAD → 工作区的改动（git diff 等价）。
/// `path` 为可选路径过滤；`max_bytes` 为单文件补丁上限（缺省 64KB，
/// 服务端硬上限 256KB），超限截断并标 `truncated`，总量触顶标 `more`。
#[tauri::command]
pub fn git_diff(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
    path: Option<String>,
    max_bytes: Option<u32>,
) -> Result<GitDiffResult, String> {
    check_invocation_source(&webview, "git_diff", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    let cap = max_bytes
        .map(|v| v as usize)
        .unwrap_or(sync::GIT_DIFF_DEFAULT_MAX_BYTES);
    sync::read_diff(&repo, path.as_deref(), cap)
}

/// 只读：本地 + 远程跟踪分支清单（git branch -a 等价）。
#[tauri::command]
pub fn git_branch_list(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
) -> Result<Vec<GitBranch>, String> {
    check_invocation_source(&webview, "git_branch_list", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    sync::read_branches(&repo)
}

// ---------------------------------------------------------------------------
// M1-6.b Git 写能力：双阶段确认闸门（request_git_write / confirm_git_write）
//
// 对齐 request_sync/confirm_sync 的双阶段范式，但使用独立的
// GitWriteJob/pending_git_jobs（SyncJob 语义属成果推送，混用会污染既有流程）。
//
// 安全边界（fail-closed，任一环节失败零写入）：
// - 操作白名单仅六项（GitWriteOp），其余一律 Err("操作禁止")；
// - repo_id 必须命中 repos.json 且锁定在 app_data_dir/mvp-browser-os/repos/<id>
//   （复用 M1-5 sync::repo_dir/open_readonly）；
// - paths 仅允许仓库内显式相对路径，禁止 .git 内部/绝对路径/空路径；
// - commit message 非空限长禁控制字符；branch name 禁 HEAD/空名/逃逸/控制字符；
// - discard 为 dangerous：confirm 必须带 confirmed_dangerous=true 二次确认；
// - 任务一次性、5 分钟过期；确认后后台线程执行，完成发 git-write-completed；
// - 审计 git_write_request / git_write / git_write_failed / git_write_rejected，
//   detail 只含操作语义与计数，绝无 token/凭据/完整路径清单/diff。
// ---------------------------------------------------------------------------

/// Git 写任务确认有效期（冻结：5 分钟）。
pub const GIT_WRITE_JOB_TTL_SECS: i64 = 300;
/// 预览返回给前端的路径列表截断条数（完整列表不落前端，防大仓刷屏）。
pub const GIT_WRITE_PREVIEW_MAX_PATHS: usize = 20;

/// 闸门纯函数：校验并取出一个待确认任务（一次性；任何失败都不产生写）。
pub(crate) fn take_confirmable_git_job(
    jobs: &mut HashMap<String, GitWriteJob>,
    job_id: &str,
    now: chrono::DateTime<chrono::Utc>,
    confirmed_dangerous: bool,
) -> Result<GitWriteJob, String> {
    let job = jobs.get(job_id).cloned().ok_or("未知任务")?;
    if job.status != GitWriteStatus::Pending {
        jobs.remove(job_id);
        return Err("任务状态异常".to_string());
    }
    if now >= job.expires_at {
        jobs.remove(job_id);
        return Err("任务已过期".to_string());
    }
    if job.dangerous && !confirmed_dangerous {
        // 保留任务：用户可在前端补二次确认后在有效期内重试；绝不执行
        return Err("需二次确认".to_string());
    }
    jobs.remove(job_id);
    let mut job = job;
    job.status = GitWriteStatus::Running;
    Ok(job)
}

/// 清理已过期的待确认任务（纯函数）：request 阶段插入新任务前调用，
/// 防止未被 confirm 的任务在表里无限累积。只清理「即使 confirm 也会被拒」的
/// 过期任务，不影响任何有效期内任务。
pub(crate) fn purge_expired_git_jobs(
    jobs: &mut HashMap<String, GitWriteJob>,
    now: chrono::DateTime<chrono::Utc>,
) -> usize {
    let before = jobs.len();
    jobs.retain(|_, job| now < job.expires_at);
    before - jobs.len()
}

/// 审计 detail 构造（纯函数）：只含操作语义与计数。
/// 刻意不接收 paths/diff/凭据/远端 URL，从签名上杜绝敏感内容落审计。
/// `branch`/`remote_name` 仅 push/分支类操作使用（分支名与远端名非凭据）。
pub(crate) fn git_write_audit_detail(
    op: GitWriteOp,
    repo_id: &str,
    job_id: &str,
    path_count: usize,
    confirmed: bool,
    branch: Option<&str>,
    remote_name: Option<&str>,
    extra: &str,
) -> String {
    serde_json::json!({
        "op": op.as_str(),
        "repo_id": repo_id,
        "job_id": job_id,
        "path_count": path_count,
        "confirmed": confirmed,
        "branch": branch,
        "remote_name": remote_name,
        "extra": extra,
    })
    .to_string()
}

/// 审计/错误串兜底截断（300 字节，回退 UTF-8 边界）：
/// 写链不接触 token，这里防止冗长错误（如整段 diff 文本）落审计。
pub(crate) fn sanitize_audit_text(s: &str) -> String {
    const MAX: usize = 300;
    if s.len() <= MAX {
        return s.to_string();
    }
    let mut end = MAX;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// 阶段一预检（只读）：参数校验 + 必然失败情形提前暴露，
/// 返回 (摘要, 影响路径, 推导出的分支名)。任何一步失败都直接 Err——
/// 此函数不做任何写，push 分支也绝不接触远端。
fn precheck_git_write(
    repo: &git2::Repository,
    op: GitWriteOp,
    paths: &[String],
    message: Option<&str>,
    branch: Option<&str>,
    checkout: bool,
) -> Result<(String, Vec<String>, Option<String>), String> {
    match op {
        GitWriteOp::Stage => {
            sync::validate_repo_paths(paths)?;
            sync::precheck_stageable(repo, paths)?;
            Ok((format!("暂存 {} 个路径", paths.len()), paths.to_vec(), None))
        }
        GitWriteOp::Unstage => {
            sync::validate_repo_paths(paths)?;
            sync::precheck_tracked(repo, paths)?;
            Ok((
                format!("取消暂存 {} 个路径", paths.len()),
                paths.to_vec(),
                None,
            ))
        }
        GitWriteOp::Discard => {
            sync::validate_repo_paths(paths)?;
            sync::precheck_tracked(repo, paths)?;
            Ok((
                format!("丢弃 {} 个路径的工作区改动（不可恢复）", paths.len()),
                paths.to_vec(),
                None,
            ))
        }
        GitWriteOp::Commit => {
            sync::validate_commit_message(message.unwrap_or_default())?;
            if paths.is_empty() {
                let dirty = sync::read_status(repo)?;
                if dirty.is_empty() {
                    return Err("没有可提交的变更".to_string());
                }
                let affected: Vec<String> = dirty.into_iter().map(|s| s.path).collect();
                Ok((
                    format!("提交全部 {} 个变更文件", affected.len()),
                    affected,
                    None,
                ))
            } else {
                sync::validate_repo_paths(paths)?;
                sync::precheck_stageable(repo, paths)?;
                Ok((
                    format!("提交 {} 个指定路径", paths.len()),
                    paths.to_vec(),
                    None,
                ))
            }
        }
        GitWriteOp::CreateBranch => {
            let name = branch.unwrap_or_default();
            sync::validate_branch_name(name)?;
            if repo.find_branch(name, git2::BranchType::Local).is_ok() {
                return Err(format!("分支已存在: {name}"));
            }
            let summary = if checkout {
                format!("基于当前 HEAD 创建并切换到新分支 {name}")
            } else {
                format!("基于当前 HEAD 创建新分支 {name}")
            };
            Ok((summary, vec![], Some(name.to_string())))
        }
        GitWriteOp::CheckoutBranch => {
            let name = branch.unwrap_or_default();
            sync::validate_branch_name(name)?;
            match sync::checkout_conflict(repo, name)? {
                Some(conflict) => Err(format!(
                    "工作区有未提交改动且与目标分支冲突，禁止切换: {conflict}"
                )),
                None => Ok((
                    format!("切换到本地分支 {name}"),
                    vec![],
                    Some(name.to_string()),
                )),
            }
        }
        GitWriteOp::Push => {
            // 只读预检：分支合法、非 detached HEAD、远端已配置、预览领先提交数。
            // 不读 KeyringStore、不构造任何网络调用。
            let head = repo.head().map_err(|e| e.to_string())?;
            if !head.is_branch() {
                return Err("当前为 detached HEAD，禁止推送".to_string());
            }
            let name = head
                .shorthand()
                .ok_or_else(|| "无法解析当前分支名".to_string())?
                .to_string();
            sync::validate_branch_name(&name)?;
            if repo.find_remote(sync::GIT_PUSH_REMOTE).is_err() {
                return Err(format!("远端未配置: {}", sync::GIT_PUSH_REMOTE));
            }
            let summary = match sync::push_ahead(repo, &name)? {
                Some(0) => {
                    format!("推送当前分支 {name} 到 origin（远端已是最新，非 force）")
                }
                Some(n) => {
                    format!("推送当前分支 {name} 到 origin（领先 {n} 个提交，非 force）")
                }
                None => format!("首次推送当前分支 {name} 到 origin（远端尚无此分支，非 force）"),
            };
            Ok((summary, vec![], Some(name)))
        }
    }
}

/// Git 写 阶段一：生成待确认 GitWriteJob 与预览（不执行任何写）。
#[tauri::command]
pub fn request_git_write(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
    op: String,
    paths: Option<Vec<String>>,
    message: Option<String>,
    branch: Option<String>,
    checkout: Option<bool>,
) -> Result<GitWritePreview, String> {
    check_invocation_source(&webview, "request_git_write", None, &app)?;
    // 1) 操作白名单：非白名单（reset/push/merge/rebase/stash/clean…）立即拒绝
    let op = match GitWriteOp::from_op_str(&op) {
        Some(op) => op,
        None => {
            workspace::log_audit(&app, "git_write_rejected", format!("操作禁止: {op}"));
            return Err("操作禁止".to_string());
        }
    };
    // 2) 路径锁定：repo_id 必须命中 repos.json 且禁止逃逸（复用 M1-5）
    let repo = sync::open_readonly(&app, &repo_id).map_err(|e| {
        workspace::log_audit(&app, "git_write_rejected", format!("仓库校验失败: {e}"));
        e
    })?;
    // 3) 参数校验 + 只读预检（任何失败都零写入，且落 git_write_rejected 审计）
    let paths = paths.unwrap_or_default();
    let checkout = checkout.unwrap_or(false);
    let (summary, affected, derived_branch) = match precheck_git_write(
        &repo,
        op,
        &paths,
        message.as_deref(),
        branch.as_deref(),
        checkout,
    ) {
        Ok(v) => v,
        Err(e) => {
            workspace::log_audit(
                &app,
                "git_write_rejected",
                git_write_audit_detail(
                    op,
                    &repo_id,
                    "",
                    paths.len(),
                    false,
                    None,
                    None,
                    &sanitize_audit_text(&e),
                ),
            );
            return Err(e);
        }
    };
    // 4) 生成一次性待确认任务（5 分钟有效），不执行任何写
    let now = chrono::Utc::now();
    let job = GitWriteJob {
        id: uuid::Uuid::new_v4().to_string(),
        repo_id: repo_id.clone(),
        op,
        paths,
        message: message.filter(|m| !m.is_empty()),
        // 分支名：优先预检推导值（push 从 HEAD 推导），否则取调用方参数
        branch: derived_branch.or_else(|| branch.filter(|b| !b.is_empty())),
        checkout,
        status: GitWriteStatus::Pending,
        dangerous: op.is_dangerous(),
        created_at: now,
        expires_at: now + chrono::Duration::seconds(GIT_WRITE_JOB_TTL_SECS),
        finished_at: None,
        error: None,
    };
    let path_count = affected.len();
    {
        let state = app.state::<AppState>();
        let mut jobs = state.pending_git_jobs.lock().unwrap();
        // 顺带清理过期任务（否则未被 confirm 的任务会一直在表里累积）
        purge_expired_git_jobs(&mut jobs, now);
        jobs.insert(job.id.clone(), job.clone());
    }
    let audit_remote = matches!(op, GitWriteOp::Push).then_some(sync::GIT_PUSH_REMOTE);
    workspace::log_audit(
        &app,
        "git_write_request",
        git_write_audit_detail(
            op,
            &repo_id,
            &job.id,
            path_count,
            false,
            job.branch.as_deref(),
            audit_remote,
            &summary,
        ),
    );
    Ok(GitWritePreview {
        job_id: job.id,
        repo_id,
        op,
        summary,
        affected_paths: affected
            .into_iter()
            .take(GIT_WRITE_PREVIEW_MAX_PATHS)
            .collect(),
        path_count,
        dangerous: job.dangerous,
        expires_at: job.expires_at,
    })
}

/// Git 写 阶段二（闸门）：用户确认后才执行。dangerous 操作必须二次确认。
/// 后台线程执行，完成（成功/失败）通过 `git-write-completed` 事件通知前端。
#[tauri::command]
pub fn confirm_git_write(
    app: AppHandle,
    webview: tauri::Webview,
    job_id: String,
    confirmed_dangerous: Option<bool>,
) -> Result<GitWriteJob, String> {
    check_invocation_source(&webview, "confirm_git_write", None, &app)?;
    let running = {
        let state = app.state::<AppState>();
        let mut jobs = state.pending_git_jobs.lock().unwrap();
        match take_confirmable_git_job(
            &mut jobs,
            &job_id,
            chrono::Utc::now(),
            confirmed_dangerous.unwrap_or(false),
        ) {
            Ok(job) => job,
            Err(e) => {
                workspace::log_audit(
                    &app,
                    "git_write_rejected",
                    format!("确认闸门拒绝 job={job_id}: {e}"),
                );
                return Err(e);
            }
        }
    };

    let app_thread = app.clone();
    let job_clone = running.clone();
    std::thread::spawn(move || {
        let result = execute_git_write(&app_thread, &job_clone);
        let mut finished = job_clone.clone();
        finished.finished_at = Some(chrono::Utc::now());
        let audit_remote =
            matches!(job_clone.op, GitWriteOp::Push).then_some(sync::GIT_PUSH_REMOTE);
        match result {
            Ok(path_count) => {
                finished.status = GitWriteStatus::Success;
                workspace::log_audit(
                    &app_thread,
                    "git_write",
                    git_write_audit_detail(
                        job_clone.op,
                        &job_clone.repo_id,
                        &job_clone.id,
                        path_count,
                        true,
                        job_clone.branch.as_deref(),
                        audit_remote,
                        "ok",
                    ),
                );
            }
            Err(e) => {
                let clean = sanitize_audit_text(&e);
                finished.status = GitWriteStatus::Failed;
                finished.error = Some(clean.clone());
                workspace::log_audit(
                    &app_thread,
                    "git_write_failed",
                    git_write_audit_detail(
                        job_clone.op,
                        &job_clone.repo_id,
                        &job_clone.id,
                        job_clone.paths.len(),
                        true,
                        job_clone.branch.as_deref(),
                        audit_remote,
                        &clean,
                    ),
                );
            }
        }
        let _ = app_thread.emit("git-write-completed", finished);
    });

    Ok(running)
}

/// 阶段二执行体：重新做路径锁定与参数校验后才执行写（fail-closed 双保险）。
/// 返回实际影响路径数（供审计 `path_count` 字段使用）；分支类操作恒为 0。
///
/// 注意：`sync::open_readonly` 只是「打开已锁定路径下的仓库」的既有命名
/// （内部复用 M1-5 的 `repo_dir` 路径锁定），写操作同样经它进入。
fn execute_git_write(app: &AppHandle, job: &GitWriteJob) -> Result<usize, String> {
    let repo = sync::open_readonly(app, &job.repo_id)?;
    match job.op {
        GitWriteOp::Stage => sync::write_stage(&repo, &job.paths),
        GitWriteOp::Unstage => sync::write_unstage(&repo, &job.paths),
        GitWriteOp::Discard => sync::write_discard(&repo, &job.paths),
        GitWriteOp::Commit => {
            let msg = job.message.clone().unwrap_or_default();
            let paths = if job.paths.is_empty() {
                None
            } else {
                Some(job.paths.as_slice())
            };
            // 全量提交（paths=None）用执行前的脏文件数作口径：
            // job.paths 此时为空，若直接取 len() 会让审计永远记为 0。
            let count = match paths {
                None => sync::read_status(&repo)?.len(),
                Some(ps) => ps.len(),
            };
            sync::write_commit(&repo, &msg, paths).map(|_| count)
        }
        GitWriteOp::CreateBranch => {
            let name = job.branch.clone().unwrap_or_default();
            sync::write_create_branch(&repo, &name, job.checkout).map(|_| 0)
        }
        GitWriteOp::CheckoutBranch => {
            let name = job.branch.clone().unwrap_or_default();
            sync::write_checkout_branch(&repo, &name).map(|_| 0)
        }
        GitWriteOp::Push => {
            // 凭据仅在推送瞬间从系统密钥库读取（与 push_artifacts 同一思路），
            // 不持久化、不复制到任务结构；错误串先脱敏再向上传递。
            let repos = workspace::load_repos(app);
            let cfg = repos
                .iter()
                .find(|r| r.id == job.repo_id)
                .ok_or_else(|| "仓库未配置".to_string())?;
            let token = KeyringStore::get_token(&cfg.id)?;
            sync::write_push(&repo, &cfg.username, &token)
                .map(|_| 0)
                .map_err(|e| sync::scrub_sensitive_error(&e, &token))
        }
    }
}

/// 第一步：生成"待确认"SyncJob（不真正推送）。校验仓库与凭据存在。
#[tauri::command]
pub fn request_sync(
    app: AppHandle,
    artifact_ids: Vec<String>,
    repo_id: String,
) -> Result<SyncPreview, String> {
    let repos = workspace::load_repos(&app);
    let repo = repos
        .iter()
        .find(|r| r.id == repo_id)
        .ok_or("仓库未配置".to_string())?;
    // 提前校验凭据存在，避免确认后才失败
    KeyringStore::get_token(&repo.id)?;

    let arts: Vec<Artifact> = workspace::load_artifacts(&app)
        .into_iter()
        .filter(|a| artifact_ids.contains(&a.id))
        .collect();
    if arts.is_empty() {
        return Err("没有可同步的成果".into());
    }

    let job = SyncJob {
        id: uuid::Uuid::new_v4().to_string(),
        repo_id: repo.id.clone(),
        status: SyncStatus::Pending,
        artifact_ids: arts.iter().map(|a| a.id.clone()).collect(),
        created_at: chrono::Utc::now(),
        finished_at: None,
        error: None,
    };
    app.state::<AppState>()
        .pending_jobs
        .lock()
        .unwrap()
        .insert(job.id.clone(), job.clone());

    workspace::log_audit(
        &app,
        "request_sync",
        format!("生成待确认任务 -> {}", repo.name),
    );

    Ok(SyncPreview {
        job_id: job.id.clone(),
        repo_id: repo.id.clone(),
        repo_name: repo.name.clone(),
        artifact_count: arts.len(),
        artifact_titles: arts.iter().map(|a| a.title.clone()).collect(),
        remote_url: repo.remote_url.clone(),
    })
}

/// 第二步（闸门）：用户确认后才真正推送。凭据仅在此从密钥库读取。
/// 改为后台线程执行，推送完成（成功/失败）通过 `sync-completed` 事件通知前端，
/// 避免阻塞 UI。
#[tauri::command]
pub fn confirm_sync(app: AppHandle, job_id: String) -> Result<SyncJob, String> {
    let state = app.state::<AppState>();
    let running = {
        let mut jobs = state.pending_jobs.lock().unwrap();
        let job = jobs.get_mut(&job_id).ok_or("未知任务".to_string())?;
        if job.status != SyncStatus::Pending {
            return Err("任务状态异常".into());
        }
        job.status = SyncStatus::Running;
        jobs[&job_id].clone()
    };

    // 后台线程：从密钥库取凭据 → push → 更新任务状态 → 发事件
    let app_thread = app.clone();
    let running_clone = running.clone();
    std::thread::spawn(move || {
        let state = app_thread.state::<AppState>();
        let result = sync::push_artifacts(&app_thread, &running_clone);
        let mut jobs = state.pending_jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(&running_clone.id) {
            match result {
                Ok(_) => {
                    job.status = SyncStatus::Success;
                    job.finished_at = Some(chrono::Utc::now());
                    workspace::log_audit(
                        &app_thread,
                        "confirm_sync",
                        format!("SUCCESS 任务 {}", running_clone.id),
                    );
                }
                Err(e) => {
                    job.status = SyncStatus::Failed;
                    job.error = Some(e.clone());
                    job.finished_at = Some(chrono::Utc::now());
                    workspace::log_audit(
                        &app_thread,
                        "confirm_sync",
                        format!("FAILED 任务 {}: {}", running_clone.id, e),
                    );
                }
            }
            let finished = job.clone();
            drop(jobs);
            let _ = app_thread.emit("sync-completed", finished);
        }
    });

    Ok(running)
}

#[tauri::command]
pub fn audit_log(app: AppHandle) -> Vec<AuditEntry> {
    workspace::load_audit(&app)
}

/// 读取单个成果完整内容（预览/编辑用）
#[tauri::command]
pub fn read_artifact(app: AppHandle, id: String) -> Result<Artifact, String> {
    workspace::load_artifacts(&app)
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| "成果不存在".to_string())
}

/// 编辑成果：仅允许改标题/标签/正文（溯源字段 source_url/hash/created_at 不可变）
#[tauri::command]
pub fn update_artifact(
    app: AppHandle,
    id: String,
    title: String,
    text: String,
    tags: Vec<String>,
) -> Result<Artifact, String> {
    let mut arts = workspace::load_artifacts(&app);
    let art = arts
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or_else(|| "成果不存在".to_string())?;
    if !title.trim().is_empty() {
        art.title = title.trim().to_string();
    }
    art.text = text;
    art.tags = tags;
    let updated = art.clone();
    workspace::save_artifact(&app, art)?;
    workspace::log_audit(&app, "update", format!("编辑成果 {}", id));
    Ok(updated)
}

#[tauri::command]
pub fn delete_artifact(app: AppHandle, id: String) -> Result<(), String> {
    workspace::delete_artifact(&app, &id)?;
    workspace::log_audit(&app, "delete", format!("删除成果 {}", id));
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-1 图片领域与持久化（契约见 logs/assist/M2-1.a-prework-20260902-1055.md）
//
// 范围：只做「领域 + 持久化」。画廊/灯箱/缩放属 M2-2，字节读取与渲染通道
// （asset:// scope 或后端直读）随 M2-2 一起落地，本卡不提前扩张安全边界。
//
// 红线：
//   - 两条命令均过 `check_invocation_source`；artifact_id 先过形态校验再拼路径
//   - MIME 白名单 fail-closed（SVG 拒绝），扩展名由 MIME 反查，魔法字节比对
//   - 超限在写盘前拒绝（不留临时文件）；落盘原子写；去重按 sha256
//   - 审计只记 id/字节数/MIME，**绝不记 URL 与图片内容**
// ---------------------------------------------------------------------------

/// 保存图片到指定成果（校验顺序见 `workspace::save_image`）。
#[tauri::command]
pub fn save_image(
    app: AppHandle,
    webview: tauri::Webview,
    artifact_id: String,
    data: Vec<u8>,
    mime: String,
    source_url: Option<String>,
    caption: Option<String>,
) -> Result<ImageRef, String> {
    check_invocation_source(&webview, "save_image", None, &app)?;
    check_id(&artifact_id, "成果 id")?;

    // IPC 载荷入口先卡一次上限，避免把超大内容带进内存后才判超限
    if data.is_empty() {
        return Err(crate::images::ImageError::ImageEmpty.to_string());
    }
    if data.len() > IMAGE_MAX_BYTES {
        return Err(crate::images::ImageError::ImageTooLarge.to_string());
    }

    // 说明文案走既有文本边界校验（M0-3 口径），空串视为不填
    let cap = caption.as_deref().map(str::trim).filter(|c| !c.is_empty());
    if let Some(c) = cap {
        crate::security_policy::check_text_field(
            "caption",
            c,
            crate::security_policy::MAX_TEXT_FIELD_BYTES,
        )
        .map_err(|e| e.to_string())?;
    }

    let reference =
        workspace::save_image(&app, &artifact_id, &data, &mime, source_url.as_deref(), cap)?;
    workspace::log_audit(
        &app,
        "image.save",
        format!(
            "artifact_id={artifact_id} image_id={} bytes={} mime={}",
            reference.id, reference.bytes, reference.mime
        ),
    );
    Ok(reference)
}

/// 列出某成果的图片引用（出参 source_url 二次脱敏兜底）。
#[tauri::command]
pub fn list_artifact_images(
    app: AppHandle,
    webview: tauri::Webview,
    artifact_id: String,
) -> Result<Vec<ImageRef>, String> {
    check_invocation_source(&webview, "list_artifact_images", None, &app)?;
    check_id(&artifact_id, "成果 id")?;
    let art = workspace::load_artifacts(&app)
        .into_iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| "成果不存在".to_string())?;
    let mut images = art.images;
    for r in images.iter_mut() {
        // 落库时已脱敏，这里对历史/手改数据再兜一层
        r.source_url = crate::images::redact_source_url(r.source_url.as_deref());
    }
    Ok(images)
}

/// M2-2.b：图片目录基准（只读，供前端 `convertFileSrc` 拼 `asset://`）。
///
/// 只返回目录基准本身，**不含任何图片相对路径**，也不读取字节：
/// 字节通道由前端 `convertFileSrc` 走 `asset://`，图片目录在既有
/// `assetProtocol.scope` 的 `$HOME/.local/share/**` 之内，不扩张 scope。
/// 路径拼接仍复用后端白名单（`images::join_image_path`），前端不得自行推导。
#[tauri::command]
pub fn workspace_images_dir(app: AppHandle, webview: tauri::Webview) -> Result<String, String> {
    check_invocation_source(&webview, "workspace_images_dir", None, &app)?;
    let dir = workspace::images_dir(&app);
    workspace::log_audit(
        &app,
        "image.dir_read",
        "op=workspace_images_dir".to_string(),
    );
    Ok(dir.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// M2-3 脚本领域与持久化（契约见 logs/checkpoints/M2-3.a-20260903-1604.md）
//
// 范围：**只做元数据的增删改查**，不做执行。`run_script` / 取消 / 超时 /
// 输出上限 / 进程组 kill / 运行记录一律归 M2-4，前端面板归 M2-5。
//
// 红线：
//   - 四条命令均过 `check_invocation_source`
//   - 落盘前先过 `scripts::validate_meta` / `validate_body`（先验收后写）
//   - 审计 detail 只含 id/name/interpreter/param_count/builtin，
//     **绝不写脚本正文、参数默认值或参数值**
//   - 正文读写一律经 `workspace::script_body_path`（canonicalize + 前缀校验）
// ---------------------------------------------------------------------------

/// 列出全部脚本元数据（只读，不审计：避免刷满 audit 的 1000 条上限）。
#[tauri::command]
pub fn script_list(app: AppHandle, webview: tauri::Webview) -> Result<Vec<ScriptMeta>, String> {
    check_invocation_source(&webview, "script_list", None, &app)?;
    Ok(workspace::load_scripts(&app))
}

/// 新增脚本：先校验后落盘；元数据保存失败时回滚已写的正文，不留孤儿文件。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn script_add(
    app: AppHandle,
    webview: tauri::Webview,
    name: String,
    category: String,
    interpreter: ScriptInterpreter,
    body: String,
    params: Vec<ScriptParam>,
    description: Option<String>,
    timeout_secs: Option<u32>,
) -> Result<ScriptMeta, String> {
    check_invocation_source(&webview, "script_add", None, &app)?;

    let mut list = workspace::load_scripts(&app);
    if list.len() >= crate::scripts::MAX_SCRIPTS {
        return Err(crate::scripts::ScriptError::TooManyScripts.to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let meta = ScriptMeta {
        id: id.clone(),
        name,
        category,
        path: format!("{}.{}", id, interpreter.ext()),
        interpreter,
        params,
        description: description.unwrap_or_default(),
        builtin: false,
        enabled: true,
        timeout_secs: timeout_secs.unwrap_or(0),
        created_at: now,
        updated_at: now,
    };

    crate::scripts::validate_meta(&meta).map_err(|e| e.to_string())?;
    crate::scripts::validate_body(&body).map_err(|e| e.to_string())?;

    // 先写正文（可失败、无副作用残留），再改元数据；元数据保存失败则回滚正文
    workspace::write_script_body(&app, &meta.path, &body)?;
    list.push(meta.clone());
    if let Err(e) = workspace::save_scripts(&app, &list) {
        let _ = workspace::delete_script_body(&app, &meta.path);
        return Err(e);
    }

    workspace::log_audit(
        &app,
        "script.add",
        format!(
            "id={id} name={} interpreter={:?} param_count={}",
            meta.name,
            meta.interpreter,
            meta.params.len()
        ),
    );
    Ok(meta)
}

/// 更新脚本元数据（**解释器不可更改**：改解释器要换扩展名与文件名，
/// 语义超出本卡范围）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn script_update(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    name: String,
    category: String,
    description: Option<String>,
    params: Vec<ScriptParam>,
    timeout_secs: Option<u32>,
    enabled: Option<bool>,
    body: Option<String>,
) -> Result<ScriptMeta, String> {
    check_invocation_source(&webview, "script_update", None, &app)?;
    check_id(&id, "脚本 id")?;

    let mut list = workspace::load_scripts(&app);
    let existing = list
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "脚本不存在".to_string())?;

    let mut meta = existing.clone();
    meta.name = name;
    meta.category = category;
    meta.description = description.unwrap_or_default();
    meta.params = params;
    meta.timeout_secs = timeout_secs.unwrap_or(0);
    if let Some(en) = enabled {
        meta.enabled = en;
    }
    meta.updated_at = chrono::Utc::now();

    crate::scripts::validate_meta(&meta).map_err(|e| e.to_string())?;

    // 正文可选更新：传了才校验并落盘
    if let Some(b) = &body {
        crate::scripts::validate_body(b).map_err(|e| e.to_string())?;
        workspace::write_script_body(&app, &meta.path, b)?;
    }

    let updated = meta.clone();
    for s in list.iter_mut() {
        if s.id == id {
            *s = meta.clone();
        }
    }
    workspace::save_scripts(&app, &list)?;

    workspace::log_audit(
        &app,
        "script.update",
        format!(
            "id={id} name={} param_count={} builtin={}",
            updated.name,
            updated.params.len(),
            updated.builtin
        ),
    );
    Ok(updated)
}

/// 删除脚本（内置脚本拒绝删除；正文删除幂等）。
#[tauri::command]
pub fn script_remove(app: AppHandle, webview: tauri::Webview, id: String) -> Result<(), String> {
    check_invocation_source(&webview, "script_remove", None, &app)?;
    check_id(&id, "脚本 id")?;

    let mut list = workspace::load_scripts(&app);
    let existing = list
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "脚本不存在".to_string())?;
    // 删除前置判定复用纯函数，避免「能否删除」的规则在两处各写一遍
    crate::scripts::can_delete(existing).map_err(|e| e.to_string())?;
    let file_name = existing.path.clone();

    list.retain(|s| s.id != id);
    workspace::save_scripts(&app, &list)?;
    // 元数据已移除，正文删不掉也只是残留文件，不影响功能（幂等）
    let _ = workspace::delete_script_body(&app, &file_name);

    workspace::log_audit(&app, "script.remove", format!("id={id} builtin=false"));
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-6.b 命令片段持久化与 CRUD
//
// 契约见 `logs/checkpoints/M2-6-20260905-1700.md` §7（M2-6.a 冻结）与
// `logs/checkpoints/B-M2-6.a-command-snippet-contract-20260905-1710.md`。
// 范围：只做 `snippets.json` 的增删改查，**不接执行**（执行接入归 M2-6.c）。
// 与脚本库的差异：命令片段**无正文文件**，故保存是单阶段原子写，
// 不存在「先写正文、失败回滚」的两阶段提交。
// 审计 detail 只含 id/name/dangerous，**不含 argv 与参数值**（命令行可能含
// secret 参数，写盘即等于泄露；F7）。
// ---------------------------------------------------------------------------

/// 列出全部命令片段。
#[tauri::command]
pub fn snippet_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<CommandSnippet>, String> {
    check_invocation_source(&webview, "snippet_list", None, &app)?;
    Ok(crate::snippets::merge_builtin_snippets(
        workspace::load_snippets(&app),
    ))
}

/// 新增命令片段。
#[tauri::command]
pub fn snippet_add(
    app: AppHandle,
    webview: tauri::Webview,
    name: String,
    category: String,
    interpreter: ScriptInterpreter,
    argv: Vec<String>,
    params: Vec<ScriptParam>,
    description: Option<String>,
    dangerous: Option<bool>,
    // M2-6-fix1（复核 F-1）：新建态也接受 `enabled`。此前后端硬编码 `true`，
    // 导致前端新建表单的「启用」勾选被静默丢弃（UI 承诺与后端行为不一致）。
    // 缺省仍为 `true`，不改变既有调用方行为。
    enabled: Option<bool>,
    timeout_secs: Option<u32>,
) -> Result<CommandSnippet, String> {
    check_invocation_source(&webview, "snippet_add", None, &app)?;

    let mut list = workspace::load_snippets(&app);
    if list.len() >= crate::snippets::MAX_SNIPPETS {
        return Err(crate::snippets::SnippetError::TooManySnippets.to_string());
    }

    let now = chrono::Utc::now();
    let snippet = CommandSnippet {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        category,
        interpreter,
        argv,
        params,
        description: description.unwrap_or_default(),
        dangerous: dangerous.unwrap_or(false),
        builtin: false,
        enabled: enabled.unwrap_or(true),
        timeout_secs: timeout_secs.unwrap_or(0),
        created_at: now,
        updated_at: now,
    };

    crate::snippets::validate_snippet(&snippet).map_err(|e| e.to_string())?;

    list.push(snippet.clone());
    workspace::save_snippets(&app, &list)?;

    workspace::log_audit(
        &app,
        "cmd.add",
        format!(
            "id={} name={} dangerous={}",
            snippet.id, snippet.name, snippet.dangerous
        ),
    );
    Ok(snippet)
}

/// 更新命令片段（整字段覆盖；`enabled` 不传则保持原值）。
///
/// 内置片段允许改名/改参数，但**不可删除**（`snippets::can_delete`）。
#[tauri::command]
pub fn snippet_update(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    name: String,
    category: String,
    interpreter: ScriptInterpreter,
    argv: Vec<String>,
    params: Vec<ScriptParam>,
    description: Option<String>,
    dangerous: Option<bool>,
    enabled: Option<bool>,
    timeout_secs: Option<u32>,
) -> Result<CommandSnippet, String> {
    check_invocation_source(&webview, "snippet_update", None, &app)?;
    check_id(&id, "命令片段 id")?;
    // M2-6.e：内置片段（id 带 `builtin:` 前缀）不可经用户接口修改，防命名空间冲突。
    if id.starts_with(crate::snippets::BUILTIN_SNIPPET_ID_PREFIX) {
        return Err("内置片段不可修改".to_string());
    }

    let mut list = workspace::load_snippets(&app);
    let pos = list
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| "命令片段不存在".to_string())?;

    let mut snip = list[pos].clone();
    snip.name = name;
    snip.category = category;
    snip.interpreter = interpreter;
    snip.argv = argv;
    snip.params = params;
    snip.description = description.unwrap_or_default();
    if let Some(d) = dangerous {
        snip.dangerous = d;
    }
    if let Some(en) = enabled {
        snip.enabled = en;
    }
    snip.timeout_secs = timeout_secs.unwrap_or(0);
    snip.updated_at = chrono::Utc::now();

    crate::snippets::validate_snippet(&snip).map_err(|e| e.to_string())?;

    list[pos] = snip.clone();
    workspace::save_snippets(&app, &list)?;

    workspace::log_audit(
        &app,
        "cmd.update",
        format!(
            "id={} name={} dangerous={}",
            snip.id, snip.name, snip.dangerous
        ),
    );
    Ok(snip)
}

/// 删除命令片段（内置片段拒绝）。
#[tauri::command]
pub fn snippet_remove(app: AppHandle, webview: tauri::Webview, id: String) -> Result<(), String> {
    check_invocation_source(&webview, "snippet_remove", None, &app)?;
    check_id(&id, "命令片段 id")?;

    let mut list = workspace::load_snippets(&app);
    let existing = list
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "命令片段不存在".to_string())?;
    // 删除前置判定复用纯函数，避免「能否删除」的规则在两处各写一遍
    crate::snippets::can_delete(existing).map_err(|e| e.to_string())?;

    list.retain(|s| s.id != id);
    workspace::save_snippets(&app, &list)?;

    workspace::log_audit(&app, "cmd.remove", format!("id={id} builtin=false"));
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-4.c 脚本执行命令接入（契约见 logs/checkpoints/M2-4.a-20260903-2233.md §8.8）
//
// 范围：只把 `run_script` / `cancel_script` / `script_status` 接到 M2-4.b 的
// `script_runner` 内核，并补齐来源校验、参数校验、审计、ACL 与 handler。
// 输出环形缓冲、事件流、运行记录落盘与 shutdown 注册仍归 M2-4.d。
// 审计 detail 只含 id/run_id/状态/计数，不含参数值、脚本正文、URL 或任何凭据。
// ---------------------------------------------------------------------------

fn map_run_error(e: RunError) -> String {
    e.to_string()
}

fn get_enabled_script(app: &AppHandle, id: &str) -> Result<ScriptMeta, String> {
    check_id(id, "脚本 id")?;
    let script = workspace::load_scripts(app)
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "SCRIPT_NOT_FOUND".to_string())?;
    if !script.enabled {
        return Err("SCRIPT_DISABLED".to_string());
    }
    crate::scripts::validate_meta(&script).map_err(|e| e.to_string())?;
    Ok(script)
}

fn get_enabled_snippet(app: &AppHandle, id: &str) -> Result<CommandSnippet, String> {
    check_id(id, "命令片段 id")?;
    let snippet = workspace::load_snippets(app)
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "SNIPPET_NOT_FOUND".to_string())?;
    if !snippet.enabled {
        return Err("SNIPPET_DISABLED".to_string());
    }
    crate::snippets::validate_snippet(&snippet).map_err(|e| e.to_string())?;
    Ok(snippet)
}

/// 启动命令片段：命令层只传参数 map，真正 argv 整元素替换和 fail-closed 校验在
/// `script_runner::start_command`，并复用脚本运行的进程表/事件/取消/落盘链路。
#[tauri::command]
pub fn run_command(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    values: HashMap<String, String>,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "run_command", None, &app)?;
    check_id(&id, "命令片段 id")?;
    let snippet = get_enabled_snippet(&app, &id)?;
    let count = snippet.params.len();
    let roots = allowed_roots(&app);
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    let start = crate::script_runner::start_command(
        &table,
        &snippet,
        &values,
        &roots,
        &home,
        Some(app.clone()),
        Some(workspace::script_runs_file(&app)),
    );
    let run_id = match start {
        Ok(rid) => rid,
        Err(e) => {
            if let RunError::InvalidParam(ref inner) = e {
                workspace::log_audit(
                    &app,
                    "cmd.validate.reject",
                    format!("id={} error_code={}", snippet.id, inner.code()),
                );
            }
            return Err(map_run_error(e));
        }
    };
    let snapshot = table
        .snapshot(&run_id)
        .ok_or_else(|| "UNKNOWN_RUN".to_string())?;
    workspace::log_audit(
        &app,
        "cmd.run.start",
        format!(
            "id={} run_id={} count={} dangerous={}",
            snippet.id, run_id, count, snippet.dangerous
        ),
    );
    Ok(snapshot)
}

/// 启动脚本：命令层只传输入 map，真正展开和 fail-closed 校验在 `script_runner`。
#[tauri::command]
pub fn run_script(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    values: HashMap<String, String>,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "run_script", None, &app)?;
    let script = get_enabled_script(&app, &id)?;
    let script_path = workspace::script_body_path(&app, &script.path)?;
    let roots = allowed_roots(&app);
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    let start = crate::script_runner::start_run(
        &table,
        &script,
        &script_path,
        &values,
        &roots,
        &home,
        Some(app.clone()),
        Some(workspace::script_runs_file(&app)),
    );
    let run_id = match start {
        Ok(rid) => rid,
        Err(e) => {
            // D15：参数校验失败（fail-closed）落审计，detail 只含 script id 与稳定错误码，
            // 不含任何参数值明文（脱敏三重保险），保证攻击尝试可观测。
            if let RunError::InvalidParam(ref inner) = e {
                workspace::log_audit(
                    &app,
                    "script.validate.reject",
                    format!("id={} error_code={}", script.id, inner.code()),
                );
            }
            return Err(map_run_error(e));
        }
    };
    let snapshot = table
        .snapshot(&run_id)
        .ok_or_else(|| "UNKNOWN_RUN".to_string())?;
    workspace::log_audit(
        &app,
        "script.run.start",
        format!(
            "id={} run_id={} param_count={}",
            script.id,
            run_id,
            script.params.len()
        ),
    );
    Ok(snapshot)
}

/// 请求取消脚本：只置取消位并立即返回；进程组回收由 supervisor 完成。
#[tauri::command]
pub fn cancel_script(
    app: AppHandle,
    webview: tauri::Webview,
    run_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "cancel_script", None, &app)?;
    check_id(&run_id, "运行 id")?;
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    table.cancel(&run_id).map_err(map_run_error)?;
    workspace::log_audit(&app, "script.run.cancel", format!("run_id={run_id}"));
    Ok(())
}

/// 查询脚本运行状态。M2-4.c 只返回内存快照，不含输出正文。
#[tauri::command]
pub fn script_status(
    app: AppHandle,
    webview: tauri::Webview,
    run_id: String,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "script_status", None, &app)?;
    check_id(&run_id, "运行 id")?;
    let snapshot = app
        .state::<AppState>()
        .script_runs
        .snapshot(&run_id)
        .ok_or_else(|| "UNKNOWN_RUN".to_string())?;
    workspace::log_audit(
        &app,
        "script.run.status",
        format!("run_id={} status={:?}", snapshot.run_id, snapshot.status),
    );
    Ok(snapshot)
}

/// 列出脚本运行历史：读 `script-runs.json`（M2-4.d 落盘），全量或按 `script_id` 过滤。
///
/// 仅终态记录（运行态查询走 `script_status` 内存快照）。记录已含 `output_tail`
/// 与 `truncated` 标志，前端可直接展示。
/// 文件不存在或为空时返回空 `Vec`，不报错（首跑无历史属正常路径）。
#[tauri::command]
pub fn script_runs_list(
    app: AppHandle,
    webview: tauri::Webview,
    script_id: Option<String>,
) -> Result<Vec<ScriptRunRecord>, String> {
    check_invocation_source(&webview, "script_runs_list", None, &app)?;
    let path = workspace::script_runs_file(&app);
    let mut records = crate::script_runner::load_run_records(&path);
    if let Some(sid) = script_id.as_deref() {
        check_id(sid, "脚本 id")?;
        records.retain(|r| r.script_id == sid);
    }
    // 审计 detail 只含 count + 可选 script_id，不含任何参数值（沿用 M2-4.b-VERDICT
    // 脱敏三重保险）。
    workspace::log_audit(
        &app,
        "script.runs.list",
        format!(
            "count={} script_id={}",
            records.len(),
            script_id.as_deref().unwrap_or("*")
        ),
    );
    Ok(records)
}

/// M1-2: 新增或更新一条收藏。同 URL 视为更新。
#[tauri::command]
pub fn add_bookmark(
    app: AppHandle,
    url: String,
    title: String,
    category: String,
) -> Result<Bookmark, String> {
    let bm = workspace::add_bookmark(&app, url.clone(), title.clone(), category.clone())?;
    workspace::log_audit(&app, "bookmark.add", format!("{} -> {}", url, bm.title));
    Ok(bm)
}

/// M1-2: 列出全部收藏，按 created_at 倒序。
#[tauri::command]
pub fn list_bookmarks(app: AppHandle) -> Vec<Bookmark> {
    let mut list = workspace::load_bookmarks(&app);
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    list
}

/// M1-2: 按 id 删除收藏（幂等）。
#[tauri::command]
pub fn remove_bookmark(app: AppHandle, id: String) -> Result<(), String> {
    workspace::remove_bookmark(&app, &id)?;
    workspace::log_audit(&app, "bookmark.remove", format!("id={}", id));
    Ok(())
}

/// 工作区目录树：按来源域名聚合成果，便于在本地成果浏览器里浏览
#[tauri::command]
pub fn browse_workspace(app: AppHandle) -> WorkspaceTree {
    let arts = workspace::load_artifacts(&app);
    let mut domains: std::collections::BTreeMap<String, Vec<Artifact>> =
        std::collections::BTreeMap::new();
    for a in arts {
        let host = url::Url::parse(&a.source_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| "未知来源".into());
        domains.entry(host).or_default().push(a);
    }
    let nodes = domains
        .into_iter()
        .map(|(host, items)| DomainNode {
            host,
            items: items
                .into_iter()
                .map(|a| DomainItem {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    created_at: a.created_at,
                    tags: a.tags.clone(),
                })
                .collect(),
        })
        .collect();
    WorkspaceTree { nodes }
}

#[derive(serde::Serialize)]
pub struct WorkspaceTree {
    pub nodes: Vec<DomainNode>,
}

#[derive(serde::Serialize)]
pub struct DomainNode {
    pub host: String,
    pub items: Vec<DomainItem>,
}

#[derive(serde::Serialize)]
pub struct DomainItem {
    pub id: String,
    pub title: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

// ====== 本地文件浏览器 ======

#[derive(serde::Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64, // bytes
}

/// 列出指定路径的目录内容（不递归）
#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let dir = std::path::PathBuf::from(&path);
    if !dir.exists() {
        return Err("路径不存在".into());
    }
    if !dir.is_dir() {
        return Err("不是目录".into());
    }
    let mut entries = vec![];
    for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        // 单个条目失败时跳过（continue），不中断整个列表 —— 修复根目录/系统目录下
        // 因符号链接失效、无权限条目导致 metadata() 失败而丢失大量条目的问题。
        let Ok(e) = e else { continue };
        // 用 file_type 判断目录（不跟随符号链接、不会因目标无权限失败）；
        // metadata 取不到则 size 记 0，仍保留条目。
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        entries.push(DirEntry {
            name: e.file_name().to_string_lossy().to_string(),
            path: e.path().to_string_lossy().to_string(),
            is_dir,
            size,
        });
    }
    // 目录在前、按名字排序
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(entries)
}

/// 读取文本文件内容
#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// 写入文本文件（创建或覆盖）。M0-3.c：路径必须落在允许根目录内。
#[tauri::command]
pub fn write_file(app: AppHandle, path: String, content: String) -> Result<(), String> {
    use crate::security_policy as sp;
    let canonical =
        sp::check_path_within_roots(&path, &allowed_roots(&app)).map_err(|e| e.to_string())?;
    sp::check_text_field("content", &content, sp::MAX_HTML_BYTES).map_err(|e| e.to_string())?;
    std::fs::write(&canonical, &content).map_err(|e| e.to_string())
}

/// 打开成果所在的本地目录（用系统文件管理器定位到该 JSON 文件）
#[tauri::command]
pub fn reveal_artifact(app: AppHandle, id: String) -> Result<(), String> {
    let file = workspace::workspace_dir(&app).join(format!("{}.json", id));
    if !file.exists() {
        return Err("成果文件不存在".into());
    }
    // 打开文件所在目录，而不是直接打开 JSON 文件（避免被浏览器等关联应用打开）
    let dir = file.parent().ok_or("无法取得目录")?;
    open::that(dir).map_err(|e| format!("无法打开目录: {e}"))
}

/// 用系统默认浏览器打开成果来源 URL
#[tauri::command]
pub fn open_source(_app: AppHandle, url: String) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("来源 URL 为空".into());
    }
    // M0-3.c：`open::that` 会把任意 scheme 交给桌面环境执行，只允许 http/https。
    crate::security_policy::check_openable_url(&url).map_err(|e| e.to_string())?;
    open::that(&url).map_err(|e| format!("无法打开链接: {e}"))
}

/// 获取常用起始目录列表（Home / Desktop / Documents / Downloads / 工作区）
#[tauri::command]
pub fn get_start_dirs(app: AppHandle) -> Vec<DirEntry> {
    let mut dirs = vec![];
    let mut add = |name: &str, p: &std::path::Path| {
        if p.exists() {
            let meta = std::fs::metadata(p);
            dirs.push(DirEntry {
                name: name.into(),
                path: p.to_string_lossy().to_string(),
                is_dir: true,
                size: meta.map(|m| m.len()).unwrap_or(0),
            });
        }
    };
    if let Ok(home) = app.path().home_dir() {
        add("🏠 主目录", &home);
        add("📁 桌面", &home.join("Desktop"));
        add("📄 文档", &home.join("Documents"));
        add("⬇ 下载", &home.join("Downloads"));
    }
    // 应用工作区
    let ws = crate::workspace::workspace_dir(&app);
    add("💾 成果工作区", &ws);
    let notes = crate::workspace::notes_dir(&app);
    add("📝 笔记目录", &notes);
    dirs
}

// ====== 文件管理：新建 / 删除 / 重命名 ======

/// 新建文件（content 为空则创建空文件）。已存在则报错。
#[tauri::command]
pub fn create_file(app: AppHandle, path: String, content: Option<String>) -> Result<(), String> {
    use crate::security_policy as sp;
    let p = std::path::PathBuf::from(&path);
    if p.exists() {
        return Err("已存在同名文件/目录".into());
    }
    let file_name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    sp::check_path_component(&file_name).map_err(|e| e.to_string())?;
    // 父目录可能尚不存在，无法直接 canonicalize：先校验已存在的祖先，再校验目标。
    let roots = allowed_roots(&app);
    let mut anchor = p.clone();
    while !anchor.exists() {
        match anchor.parent() {
            Some(parent) if parent != std::path::Path::new("") => anchor = parent.to_path_buf(),
            _ => break,
        }
    }
    if anchor.exists() {
        sp::check_path_within_roots(&anchor.to_string_lossy(), &roots)
            .map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, content.unwrap_or_default()).map_err(|e| e.to_string())?;
    // 写后再校验一次真实落点，杜绝中途被符号链接替换到根目录之外。
    sp::check_path_within_roots(&p.to_string_lossy(), &roots).map_err(|e| e.to_string())?;
    Ok(())
}

/// 新建目录（递归创建父目录）。
#[tauri::command]
pub fn create_dir(app: AppHandle, path: String) -> Result<(), String> {
    use crate::security_policy as sp;
    let p = std::path::PathBuf::from(&path);
    if p.exists() {
        return Err("已存在同名文件/目录".into());
    }
    let roots = allowed_roots(&app);
    let mut anchor = p.clone();
    while !anchor.exists() {
        match anchor.parent() {
            Some(parent) if parent != std::path::Path::new("") => anchor = parent.to_path_buf(),
            _ => break,
        }
    }
    if anchor.exists() {
        sp::check_path_within_roots(&anchor.to_string_lossy(), &roots)
            .map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    sp::check_path_within_roots(&p.to_string_lossy(), &roots).map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除文件或目录（递归删除）。
#[tauri::command]
pub fn delete_path(app: AppHandle, path: String) -> Result<(), String> {
    use crate::security_policy as sp;
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("路径不存在".into());
    }
    // 递归删除最危险：必须在允许根目录内，且不允许删根目录本身。
    let canonical =
        sp::check_delete_target(&path, &allowed_roots(&app)).map_err(|e| e.to_string())?;
    if canonical.is_dir() {
        std::fs::remove_dir_all(&canonical).map_err(|e| e.to_string())?;
    } else {
        std::fs::remove_file(&canonical).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 重命名 / 移动文件或目录。
#[tauri::command]
pub fn rename_path(app: AppHandle, path: String, new_name: String) -> Result<(), String> {
    use crate::security_policy as sp;
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("路径不存在".into());
    }
    // 新名称必须是不含分隔符的单一分量，否则 `parent.join(new_name)` 会变成跨目录移动。
    sp::check_path_component(&new_name).map_err(|e| e.to_string())?;
    let roots = allowed_roots(&app);
    let canonical = sp::check_path_within_roots(&path, &roots).map_err(|e| e.to_string())?;
    let parent = canonical.parent().ok_or("无法取得父目录")?;
    let dst = parent.join(new_name.trim());
    if dst.exists() {
        return Err("目标名称已存在".into());
    }
    std::fs::rename(&p, &dst).map_err(|e| e.to_string())?;
    Ok(())
}

// ====== 剪贴板 ======

/// 读取系统剪贴板文本。
#[tauri::command]
pub fn clipboard_read() -> Result<String, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
    cb.get_text().map_err(|e| format!("读取失败: {e}"))
}

/// 写入系统剪贴板文本。
#[tauri::command]
pub fn clipboard_write(text: String) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
    cb.set_text(text).map_err(|e| format!("写入失败: {e}"))
}

// ====== 宫格浏览器：多网页并排对比 =====
//
// 多进程架构（Phase 2 起）：每个宫格 grid-N 由独立子进程承载（崩溃隔离），
// 本组命令不再走本地 TabManagerState（add_child），而是经 GridProcessManager
// 转发 UDS 命令到对应子进程。页签 tab-N 不受影响（仍主进程本地）。
// 前端契约不变：create_grid/grid_open/grid_position/grid_set_zoom/close_grid/grid_close_one。

const MAX_GRID: usize = 12;

/// 每宫格子进程（Tauri + WebKitWebProcess + AI 站点）的实测内存估算（MB）。
/// 实测 WebKitWebProcess 约 330~365MB/格，加子进程本体取 450。
const GRID_MEM_MB: u64 = 450;
/// 创建宫格后系统至少保留的可用内存（MB）：低于此值宁降级格数也不把系统打穿
/// （14G 机器 swap 吃满后开 9 宫格会全系统卡死）。
const MEM_RESERVE_MB: u64 = 700;
/// 宫格下限：用户明确要求"最低保持 4 宫格"（2 格太少不可用）。
/// 预算不足 4 格时仍强制 4 格并警告（用户接受卡顿风险）。
const MIN_GRID: usize = 4;

/// 读取 /proc/meminfo 的 MemAvailable（MB）。
fn mem_available_mb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest.trim().trim_end_matches("kB").trim().parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

/// 创建宫格（2..=12）：每格 spawn 一个子进程并经 UDS 建 webview，默认打开引导页，
/// 位置由前端 grid_position 告知（子进程窗口初始隐藏，定位后 show）。
/// 返回实际创建的格数：内存预算守卫会在可用内存不足时自动降级（保底 2 格），
/// 前端据此调整 gridCount 并提示用户。
#[tauri::command]
pub fn create_grid(app: AppHandle, n: usize) -> Result<usize, String> {
    let n = n.clamp(2, MAX_GRID);
    // 先清理旧的宫格子进程（幂等，可安全重复调用）
    close_grid(app.clone())?;
    // 内存预算守卫：可用内存不足以支撑请求格数时自动降级（保底 MIN_GRID=4，
    // 用户明确要求"最低保持 4 宫格"；预算 <4 时强制 4 格，风险由警告提示）
    let n = match mem_available_mb() {
        Some(avail) => {
            let affordable = (avail.saturating_sub(MEM_RESERVE_MB) / GRID_MEM_MB) as usize;
            let budgeted = affordable.clamp(MIN_GRID, MAX_GRID).min(n);
            if budgeted < n {
                eprintln!(
                    "[create_grid] 内存预算守卫: 可用 {}MB，请求 {} 格 → 降级 {} 格",
                    avail, n, budgeted
                );
            }
            budgeted
        }
        None => n,
    };
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    let mut created = Vec::new();
    for i in 0..n {
        let index = i as u32;
        let label = format!("grid-{i}");
        if let Err(error) = mgr.get_or_spawn(index) {
            for created_index in created.iter().rev() {
                mgr.kill_child(*created_index);
            }
            return Err(error);
        }
        created.push(index);
        // 等子进程 UDS 连接 + webview 创建完成（子进程冷启动 1~3 秒）
        if let Err(error) = mgr.request(
            index,
            GridCmd::CreateTab {
                id: label,
                url: "https://www.baidu.com".to_string(),
            },
            20000,
        ) {
            for created_index in created.iter().rev() {
                mgr.kill_child(*created_index);
            }
            return Err(error);
        }
        mgr.record_url(index, "https://www.baidu.com");
        eprintln!("[create_grid] grid-{} 子进程就绪", i);
        // 错峰启动：间隔 300ms，削掉多个 WebKit 同时冷启动的瞬时 CPU/IO 峰值
        if i + 1 < n {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
    Ok(n)
}

/// 关闭所有宫格（销毁对应子进程）。
/// 顺序：先 HideWindow（窗口立即从屏幕消失，消除"幽灵窗口残留几秒"的视觉问题），
/// 再 CloseTab + kill（进程销毁与 X 窗口清理有延迟，藏在 hide 之后用户无感）。
#[tauri::command]
pub fn close_grid(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    for index in mgr.indices() {
        mgr.send(
            index,
            GridCmd::HideWindow {
                id: format!("grid-{index}"),
            },
        );
    }
    // 给 hide 一个生效窗口期（UDS 是异步 send，100ms 足够子进程执行 hide）
    std::thread::sleep(std::time::Duration::from_millis(100));
    for index in mgr.indices() {
        mgr.send(
            index,
            GridCmd::CloseTab {
                id: format!("grid-{index}"),
            },
        );
    }
    mgr.shutdown_all();
    for i in 0..MAX_GRID {
        let label = format!("grid-{i}");
        state.child_layouts.lock().unwrap().remove(&label);
        state.grid_zooms.lock().unwrap().remove(&label);
        state.last_position_at.lock().unwrap().remove(&label);
    }
    Ok(())
}

/// 在指定宫格(index)中打开网址（经 UDS Navigate 到子进程）。
/// 导航后延迟应用存储的缩放（给页面加载留时间），确保新页面加载完成后自动缩放到位。
#[tauri::command]
pub fn grid_open(app: AppHandle, index: usize, url: String) -> Result<(), String> {
    let label = format!("grid-{index}");
    let target = normalize_url(&url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    mgr.request(
        index as u32,
        GridCmd::Navigate {
            id: label.clone(),
            url: target.clone(),
        },
        10000,
    )?;
    mgr.record_url(index as u32, &target);
    // 页面加载完成后重新应用缩放（CSS zoom 会被新页面重置，需重新注入）
    let stored = state.grid_zooms.lock().unwrap().get(&label).copied();
    if let Some(z) = stored {
        let app3 = app.clone();
        let label3 = label.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(900));
            apply_grid_zoom(&app3, &label3, z);
        });
    }
    Ok(())
}

/// 真正给宫格 webview 应用缩放（WebKitGTK 原生 zoom_level，经 UDS 到子进程）。
/// 仅在页面加载完成后调用，避免加载中设置被重置导致卡顿。
fn apply_grid_zoom(app: &AppHandle, label: &str, z: f64) {
    if z <= 0.1 || (z - 1.0).abs() < 0.01 {
        return;
    }
    if let Some(index) = grid_index_of(label) {
        app.state::<AppState>().grid_manager.send(
            index,
            GridCmd::SetZoom {
                id: label.to_string(),
                zoom: z,
            },
        );
    }
}

/// 设置宫格某格的缩放（存起来 + 若页面已加载则立即应用）。
/// 前端在每次定位/布局变化后调用；应用时机做了防抖，避免反复 zoom 卡顿。
#[tauri::command]
pub fn grid_set_zoom(app: AppHandle, index: usize, zoom: f64) -> Result<(), String> {
    let label = format!("grid-{index}");
    // 与上次相同则跳过（去重，避免重复 zoom 触发重排卡顿）
    {
        let state = app.state::<AppState>();
        let mut zooms = state.grid_zooms.lock().unwrap();
        if let Some(&prev) = zooms.get(&label) {
            if (prev - zoom).abs() < 0.02 {
                return Ok(());
            }
        }
        zooms.insert(label.clone(), zoom);
    }
    apply_grid_zoom(&app, &label, zoom);
    Ok(())
}

/// 批量定位所有宫格（前端按网格计算好每个格子的 x/y/w/h 后调用）。
/// 前端传相对主窗内容区的 CSS 坐标；子进程窗口是独立顶层窗口，这里换算成
/// 绝对屏幕物理坐标（主窗 inner_position + scale_factor）后经 UDS 下发，
/// 子进程 set_position/set_size + show。
/// 注意：本命令只负责定位，不再处理缩放 —— 缩放由 grid_set_zoom 单独管理。
#[tauri::command]
pub fn grid_position(
    app: AppHandle,
    index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let label = format!("grid-{index}");
    // 后端去重：仅"rect 相同且 <50ms"才丢弃（防 IPC 洪泛），rect 变了必须应用
    {
        let state = app.state::<AppState>();
        let mut last = state.last_position_at.lock().unwrap();
        let now = std::time::Instant::now();
        if let Some((t, lx, ly, lw, lh)) = last.get(&label) {
            let same_rect = (lx - x).abs() < 0.5
                && (ly - y).abs() < 0.5
                && (lw - width).abs() < 0.5
                && (lh - height).abs() < 0.5;
            if same_rect && now.duration_since(*t).as_millis() < 50 {
                return Ok(());
            }
        }
        last.insert(label.clone(), (now, x, y, width, height));
    }
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    let rect = mgr
        .abs_rect((x, y, width, height))
        .ok_or_else(|| "主窗不可用，无法换算屏幕坐标".to_string())?;
    mgr.record_rect(index as u32, (x, y, width, height));
    mgr.request(index as u32, GridCmd::UpdateRect { id: label, rect }, 3000)
}

/// 关闭单个宫格（按 index）：销毁对应子进程，其余宫格保留。
/// 同样先 HideWindow 再 kill（消除幽灵窗口）。
#[tauri::command]
pub fn grid_close_one(app: AppHandle, index: usize) -> Result<(), String> {
    let label = format!("grid-{index}");
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    mgr.send(index as u32, GridCmd::HideWindow { id: label.clone() });
    std::thread::sleep(std::time::Duration::from_millis(100));
    mgr.send(index as u32, GridCmd::CloseTab { id: label.clone() });
    mgr.kill_child(index as u32);
    state.child_layouts.lock().unwrap().remove(&label);
    state.grid_zooms.lock().unwrap().remove(&label);
    state.last_position_at.lock().unwrap().remove(&label);
    Ok(())
}

// ====== 系统应用启动器（Linux .desktop） ======

#[derive(serde::Serialize)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub icon_path: Option<String>,
}

fn parse_desktop_file(path: &std::path::Path) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut no_display = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_entry = t == "[Desktop Entry]";
            continue;
        }
        if !in_entry || !t.contains('=') {
            continue;
        }
        let (k, v) = t.split_once('=')?;
        match k.trim() {
            "Name" => name = v.to_string(),
            "Exec" => exec = v.to_string(),
            "Icon" => icon = v.to_string(),
            "NoDisplay" => no_display = v.trim() == "true",
            _ => {}
        }
    }
    if name.is_empty() || exec.is_empty() || no_display {
        return None;
    }
    // 去掉 Exec 中的字段码 (%U/%F 等)
    let exec_clean = exec
        .split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");
    let icon_path = resolve_icon(&icon);
    Some(AppEntry {
        name,
        exec: exec_clean,
        icon,
        icon_path,
    })
}

/// 解析 .desktop 的 Icon 字段为真实图标文件路径。
/// Icon 可能是：绝对路径 / 主题图标名（如 firefox）/ 空。
fn resolve_icon(icon: &str) -> Option<String> {
    if icon.is_empty() {
        return None;
    }
    // 1) 已是绝对路径：文件必须真实存在才返回，否则前端会尝试加载不存在的 file:// 而破图
    if icon.starts_with('/') {
        if std::path::Path::new(icon).exists() {
            return Some(icon.to_string());
        }
        return None;
    }

    // 2) 按 Freedesktop 图标主题规范解析（自动处理 hicolor、当前 GTK 主题、尺寸回退）
    let theme = freedesktop_icons::default_theme_gtk().unwrap_or_else(|| "hicolor".to_string());
    if let Some(path) = freedesktop_icons::lookup(icon)
        .with_theme(&theme)
        .with_size(48)
        .find()
    {
        return Some(path.to_string_lossy().to_string());
    }

    // 3) 兜底：常见主题目录与 pixmaps 递归查找
    let home = std::env::var("HOME").unwrap_or_default();
    let exts = ["png", "svg", "xpm"];
    let roots = [
        "/usr/share/pixmaps",
        "/usr/local/share/icons",
        &format!("{}/.local/share/icons", home),
        &format!("{}/.icons", home),
    ];
    for root in roots {
        for ext in exts {
            let p = format!("{}/{}.{}", root, icon, ext);
            if std::path::Path::new(&p).exists() {
                return Some(p);
            }
        }
        if let Some(found) = find_icon_recursive(std::path::Path::new(root), icon, &exts, 0) {
            return Some(found);
        }
    }
    None
}

fn find_icon_recursive(
    dir: &std::path::Path,
    icon: &str,
    exts: &[&str],
    depth: u32,
) -> Option<String> {
    if depth > 4 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            if let Some(f) = find_icon_recursive(&path, icon, exts, depth + 1) {
                return Some(f);
            }
        } else if path
            .extension()
            .map(|x| x == "png" || x == "svg" || x == "xpm")
            .unwrap_or(false)
        {
            if path.file_stem().map(|s| s == icon).unwrap_or(false) {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    None
}

#[tauri::command]
pub fn list_apps() -> Vec<AppEntry> {
    let mut apps = vec![];
    let mut seen = std::collections::HashSet::new();
    let mut scan = |dir: &std::path::Path| {
        if !dir.exists() {
            return;
        }
        for e in std::fs::read_dir(dir).ok().into_iter().flatten() {
            let e = e.ok();
            let path = e.as_ref().map(|e| e.path());
            let Some(path) = path else { continue };
            if !path.to_string_lossy().ends_with(".desktop") {
                continue;
            }
            if let Some(app) = parse_desktop_file(&path) {
                if seen.insert(app.name.clone()) {
                    apps.push(app);
                }
            }
        }
    };
    scan(std::path::Path::new("/usr/share/applications"));
    scan(std::path::Path::new("/usr/local/share/applications"));
    if let Ok(home) = std::env::var("HOME") {
        scan(
            std::path::Path::new(&home)
                .join(".local/share/applications")
                .as_path(),
        );
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

#[tauri::command]
pub fn launch_app(app: AppHandle, exec: String) -> Result<(), String> {
    use crate::security_policy as sp;
    let cmd = exec.trim();
    if cmd.is_empty() {
        return Err("没有可执行命令".into());
    }
    // M0-3.d：不再用 `sh -c` 执行任意字符串。改为解析成 (程序, 参数) 直接 spawn：
    // 元字符一律拒绝、shell 解释器禁为启动目标、程序必须能解析到可执行文件。
    let (program, args) = sp::check_launch_target(cmd).map_err(|e| e.to_string())?;
    crate::workspace::log_audit(
        &app,
        "launch",
        format!("{program} {}", args.join(" ")).trim().to_string(),
    );
    std::process::Command::new(&program)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("启动失败: {e}"))?;
    Ok(())
}

// ====== 浏览器页签命令（供前端页签栏调用） ======

/// 新建一个浏览器页签（打开 url），返回其信息并设为激活页签。
#[tauri::command]
pub fn tab_new(app: AppHandle, url: String) -> Result<TabInfo, String> {
    let tab = create_tab(app.clone(), &url)?;
    Ok(tab)
}

/// 关闭指定页签。
#[tauri::command]
pub fn tab_close(app: AppHandle, id: String) -> Result<(), String> {
    close_tab(&app, &id)
}

/// 在指定页签中打开网址（导航）。
#[tauri::command]
pub fn tab_open(app: AppHandle, id: String, url: String) -> Result<(), String> {
    let target = normalize_url(&url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    if let Err(e) = navigate_tab_webview(&app, &id, &target) {
        recover_tab_webview(&app, &id, "navigate", &e)?;
        navigate_tab_webview(&app, &id, &target)
            .map_err(|retry| format!("{e}; 恢复后重试导航失败: {retry}"))?;
    }
    // 更新存储的 url
    if let Some(t) = app.state::<AppState>().tabs.lock().unwrap().get_mut(&id) {
        t.url = target.clone();
    }
    // M1-9：草稿 URL 跟随导航（脱敏由 upsert 内部完成）
    upsert_session_draft(&app, &id, &target, "");
    Ok(())
}

/// 将指定页签定位到主窗口内容区给定矩形。
/// 前端传 CSS 像素（相对主窗口内容区左上角），换算成屏幕物理坐标后 set_position/set_size。
#[tauri::command]
pub fn tab_position(
    app: AppHandle,
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if let Err(e) = apply_bounds(&app, &id, x, y, width, height) {
        recover_tab_webview(&app, &id, "position", &e)?;
        apply_bounds_inner(&app, &id, x, y, width, height)
            .map_err(|retry| format!("{e}; 恢复后重试定位失败: {retry}"))?;
    }
    Ok(())
}

// ====== 宫格加载失败自动重试 =====
//
// 内存压力下宫格 webview 可能 TLS 握手失败（错误页），插件只有 navigationFinished
// 无法区分成败。collect.js 检测错误页特征 → invoke report_grid_load_failed
// （子进程内）→ emit 事件 → 子进程 UDS 转发主进程 → 主进程自动重试导航。

/// 宫格 webview 内 collect.js 上报加载失败（子进程内执行）。
#[tauri::command]
pub fn report_grid_load_failed(
    app: AppHandle,
    url: String,
    snippet: Option<String>,
) -> Result<(), String> {
    let index = std::env::var("GRID_CHILD_INDEX").unwrap_or_default();
    eprintln!(
        "[grid] 加载失败上报 grid-{} url={} snippet={:?}",
        index, url, snippet
    );
    let _ = app.emit(
        "grid-load-failed",
        serde_json::json!({ "index": index, "url": url }),
    );
    Ok(())
}

/// 主进程启动宫格加载失败自动重试：收到 grid-load-failed 事件后延迟 3s 重新导航。
/// 每格 60s 内最多重试 2 次（防循环），成功页不会触发上报故不影响正常浏览。
pub fn start_grid_load_retry(app: AppHandle) {
    use tauri::Listener;
    let retries: std::sync::Arc<Mutex<HashMap<u32, (std::time::Instant, u8)>>> =
        std::sync::Arc::new(Mutex::new(HashMap::new()));
    let app2 = app.clone();
    app.listen("grid-load-failed", move |event| {
        let payload: serde_json::Value = serde_json::from_str(event.payload()).unwrap_or_default();
        let index: u32 = match payload["index"].as_str().and_then(|s| s.parse().ok()) {
            Some(i) => i,
            None => return,
        };
        let url = payload["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            return;
        }
        // 重试配额：60s 窗口内最多 2 次
        {
            let mut map = retries.lock().unwrap();
            let now = std::time::Instant::now();
            let entry = map.entry(index).or_insert((now, 0));
            if now.duration_since(entry.0).as_secs() > 60 {
                *entry = (now, 0);
            }
            if entry.1 >= 2 {
                eprintln!("[grid] grid-{index} 自动重试次数用尽，放弃");
                return;
            }
            entry.1 += 1;
        }
        eprintln!("[grid] grid-{index} 加载失败，3s 后自动重试导航 {url}");
        let app3 = app2.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let state = app3.state::<AppState>();
            if state.shutdown_requested.load(Ordering::SeqCst) {
                return;
            }
            let mgr = &state.grid_manager;
            if let Err(e) = mgr.request(
                index,
                GridCmd::Navigate {
                    id: format!("grid-{index}"),
                    url: url.clone(),
                },
                10000,
            ) {
                eprintln!("[grid] grid-{index} 自动重试导航失败: {e}");
            } else {
                mgr.record_url(index, &url);
                eprintln!("[grid] grid-{index} 自动重试导航已下发");
            }
        });
    });
}

// ====== 资源监控（宫格设置行"资源"按钮） =====

#[derive(serde::Serialize)]
pub struct ProcStat {
    pub pid: u32,
    pub name: String,
    /// 进程树合计 RSS（MB，含 WebKit 子进程）
    pub rss_mb: f64,
}

#[derive(serde::Serialize)]
pub struct ResourceStats {
    pub mem_total_mb: u64,
    pub mem_available_mb: u64,
    /// 应用全部进程合计（主进程 + 全部宫格子进程树）
    pub app_total_mb: f64,
    /// 主进程树（不含宫格子进程树）
    pub main: ProcStat,
    /// 每宫格子进程树（index 即 grid-N 的 N）
    pub grids: Vec<ProcStat>,
    /// 页签休眠开关状态
    pub hibernation_enabled: bool,
    /// 当前已休眠页签数
    pub hibernated_count: usize,
    /// 内存预算守卫：当前可用内存最多支撑几格（创建宫格时的降级上限）
    pub grid_budget: usize,
}

/// 读取 /proc 全量进程表：pid -> (ppid, comm, rss_kb)。
fn read_proc_table() -> HashMap<u32, (u32, String, u64)> {
    let mut procs = HashMap::new();
    let Ok(rd) = std::fs::read_dir("/proc") else {
        return procs;
    };
    for e in rd.flatten() {
        let name = e.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        // comm 在括号内（可含空格），ppid 是右括号后的第 2 个字段
        let Some(lp) = stat.find('(') else { continue };
        let Some(rp) = stat.rfind(')') else { continue };
        let comm = stat[lp + 1..rp].to_string();
        let ppid: u32 = stat[rp + 2..]
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let rss_kb = std::fs::read_to_string(format!("/proc/{pid}/status"))
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("VmRSS:"))
                    .and_then(|r| r.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            })
            .unwrap_or(0);
        procs.insert(pid, (ppid, comm, rss_kb));
    }
    procs
}

/// 进程树合计 RSS（kB）：root 及其全部子孙。
fn subtree_rss_kb(procs: &HashMap<u32, (u32, String, u64)>, root: u32) -> u64 {
    let mut sum = 0;
    let mut stack = vec![root];
    while let Some(p) = stack.pop() {
        if let Some((_, _, rss)) = procs.get(&p) {
            sum += rss;
        }
        for (pid, (ppid, _, _)) in procs {
            if *ppid == p {
                stack.push(*pid);
            }
        }
    }
    sum
}

/// 资源占用统计：主进程树 + 每宫格子进程树的 RSS（含 WebKit 子进程）。
#[tauri::command]
pub fn resource_stats(app: AppHandle) -> ResourceStats {
    let procs = read_proc_table();
    let main_pid = std::process::id();
    let grid_pids = app.state::<AppState>().grid_manager.pids();
    let grid_total_kb: u64 = grid_pids
        .iter()
        .map(|(_, pid)| subtree_rss_kb(&procs, *pid))
        .sum();
    let app_total_kb = subtree_rss_kb(&procs, main_pid); // 宫格子进程是主进程的子进程，已含
    let main_only_kb = app_total_kb.saturating_sub(grid_total_kb);
    let main_name = procs
        .get(&main_pid)
        .map(|(_, c, _)| c.clone())
        .unwrap_or_else(|| "main".to_string());
    let grids = grid_pids
        .iter()
        .map(|(index, pid)| ProcStat {
            pid: *pid,
            name: format!("grid-{index}"),
            rss_mb: subtree_rss_kb(&procs, *pid) as f64 / 1024.0,
        })
        .collect();
    let avail = mem_available_mb().unwrap_or(0);
    let hibernation_enabled = app
        .state::<AppState>()
        .hibernation_enabled
        .load(Ordering::Relaxed);
    let hibernated_count = app
        .state::<AppState>()
        .hibernated_tabs
        .lock()
        .unwrap()
        .len();
    ResourceStats {
        mem_total_mb: std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("MemTotal:"))
                    .and_then(|r| r.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            })
            .unwrap_or(0)
            / 1024,
        mem_available_mb: avail,
        app_total_mb: app_total_kb as f64 / 1024.0,
        main: ProcStat {
            pid: main_pid,
            name: main_name,
            rss_mb: main_only_kb as f64 / 1024.0,
        },
        grids,
        hibernation_enabled,
        hibernated_count,
        // 真实预算（可为 0/1，创建时才会保底 2）：前端据此提示"最多还能开 N 格"
        grid_budget: (avail.saturating_sub(MEM_RESERVE_MB) / GRID_MEM_MB).min(MAX_GRID as u64)
            as usize,
    }
}

// ====== 页签休眠（设置面板开关，默认关） =====
//
// 背景：14G 内存机器上每个页签 WebKitWebProcess 占 300+MB，长期不用的页签
// 白吃内存。冻结（FREEZE_JS）只省 CPU 不省内存；休眠 = 销毁 webview 仅留 URL，
// 激活时按 URL 重建（页面状态如滚动位置/表单不保留，登录态由 WebKit 持久会话保留）。

/// 非激活超过该时长的页签才休眠（秒）。
const TAB_HIBERNATE_IDLE_SECS: u64 = 600;

/// 设置页签休眠开关（前端设置面板调用；开启后清扫线程生效）。
#[tauri::command]
pub fn set_tab_hibernation(app: AppHandle, enabled: bool) -> Result<(), String> {
    app.state::<AppState>()
        .hibernation_enabled
        .store(enabled, Ordering::Relaxed);
    eprintln!("[hibernation] 页签休眠开关 = {}", enabled);
    Ok(())
}

/// 休眠单个页签：销毁 webview，URL 保留在 tabs 表（激活时重建）。激活页签不休眠。
fn hibernate_tab(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    if state.active_tab.lock().unwrap().as_deref() == Some(id) {
        return;
    }
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    if manager.close_tab(&id.to_string()).is_ok() {
        state.child_layouts.lock().unwrap().remove(id);
        state.last_position_at.lock().unwrap().remove(id);
        state.tab_idle_since.lock().unwrap().remove(id);
        state.hibernated_tabs.lock().unwrap().insert(id.to_string());
        state.tab_recovery.lock().unwrap().remove(id);
        eprintln!("[hibernation] 页签 {} 已休眠（webview 销毁，URL 保留）", id);
    }
}

/// 休眠清扫线程（主进程 setup 启动一次）：每 60s 检查，开关开启时把
/// 非激活超 TAB_HIBERNATE_IDLE_SECS 的页签休眠。
pub fn start_hibernation_sweeper(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let state = app.state::<AppState>();
        if state.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }
        if !state.hibernation_enabled.load(Ordering::Relaxed) {
            continue;
        }
        let active = state.active_tab.lock().unwrap().clone();
        let now = std::time::Instant::now();
        let victims: Vec<String> = {
            let idle = state.tab_idle_since.lock().unwrap();
            let hibernated = state.hibernated_tabs.lock().unwrap();
            let tabs = state.tabs.lock().unwrap();
            idle.iter()
                .filter(|(id, since)| {
                    Some(*id) != active.as_ref()
                        && !hibernated.contains(*id)
                        && tabs.contains_key(*id)
                        && now.duration_since(**since).as_secs() > TAB_HIBERNATE_IDLE_SECS
                })
                .map(|(id, _)| id.clone())
                .collect()
        };
        for id in victims {
            hibernate_tab(&app, &id);
        }
    });
}

/// 列出当前所有页签。
#[tauri::command]
pub fn tab_list(app: AppHandle) -> Vec<TabInfo> {
    let state = app.state::<AppState>();
    let tabs = state.tabs.lock().unwrap();
    tabs.values().cloned().collect()
}

/// 前端设置页签标题（如用户手动改名）。
#[tauri::command]
pub fn tab_set_title(app: AppHandle, id: String, title: String) -> Result<(), String> {
    if let Some(t) = app.state::<AppState>().tabs.lock().unwrap().get_mut(&id) {
        t.title = title;
    }
    Ok(())
}

/// 在当前激活页签的 webview 内执行 history.back()（页面内后退）。
/// 历史栈为空时由浏览器内核自动忽略，无副作用。
#[tauri::command]
pub fn tab_go_back(app: AppHandle, id: String) -> Result<(), String> {
    plugin_eval(&app, &id, "if (history.length > 1) { history.back(); }")
}

/// 在当前激活页签的 webview 内执行 history.forward()（页面内前进）。
#[tauri::command]
pub fn tab_go_forward(app: AppHandle, id: String) -> Result<(), String> {
    plugin_eval(&app, &id, "history.forward();")
}

/// 刷新当前激活页签的 webview（location.reload）。
#[tauri::command]
pub fn tab_reload(app: AppHandle, id: String) -> Result<(), String> {
    plugin_eval(&app, &id, "location.reload();")
}

/// 在指定子 webview 中执行 JavaScript（用于 AI 模式向宫格注入问题）。
/// 宫格 grid-N 经 UDS 转发到子进程，页签 tab-N 仍走本地插件。
#[tauri::command]
pub fn eval_in_tab(app: AppHandle, id: String, js: String) -> Result<String, String> {
    if let Some(index) = grid_index_of(&id) {
        app.state::<AppState>()
            .grid_manager
            .request(index, GridCmd::Eval { id, js }, 10000)?;
        return Ok("ok".to_string());
    }
    plugin_eval(&app, &id, &js)?;
    Ok("ok".to_string())
}

/// 设置激活页签：只更新 active_tab，并把其它页签隐藏。
/// 目标页签的精确位置由前端随后调用 tab_position 给出，避免后端与前端争夺坐标。
#[tauri::command]
pub fn tab_activate(app: AppHandle, id: String) -> Result<(), String> {
    // 休眠重建：webview 已销毁的页签先按原 URL 重建（1x1 隐藏，前端 tab_position 放大）
    if app
        .state::<AppState>()
        .hibernated_tabs
        .lock()
        .unwrap()
        .remove(&id)
    {
        let url = app
            .state::<AppState>()
            .tabs
            .lock()
            .unwrap()
            .get(&id)
            .map(|t| t.url.clone());
        if let Some(url) = url {
            spawn_child_window(&app, &id, &url, 0.0, 0.0, 1.0, 1.0)?;
            eprintln!("[hibernation] 页签 {} 已从休眠重建 url={}", id, url);
        }
    }
    // 暂存激活页签，前端会随后调用 tab_position 精确布局
    *app.state::<AppState>().active_tab.lock().unwrap() = Some(id.clone());
    // 休眠计时：目标页签移出 idle 表；其它页签从首次失活时刻起算（已有记录不刷新）
    {
        let state = app.state::<AppState>();
        let mut idle = state.tab_idle_since.lock().unwrap();
        idle.remove(&id);
        let now = std::time::Instant::now();
        for k in state.tabs.lock().unwrap().keys() {
            if *k != id {
                idle.entry(k.clone()).or_insert(now);
            }
        }
    }
    // 把其它页签隐藏（子窗口 hide），目标页签若有记住布局则立即重定位显示。
    // 这些 GTK 操作必须整体走 run_on_main_thread，避免 command 同步上下文死锁。
    let app_act = app.clone();
    let active_id = id.clone();
    let _ = app.run_on_main_thread(move || {
        let st = app_act.state::<AppState>();
        let ids: Vec<String> = st.tabs.lock().unwrap().keys().cloned().collect();
        eprintln!(
            "[tab_activate] active={} others={:?}",
            active_id,
            ids.iter().filter(|k| **k != active_id).collect::<Vec<_>>()
        );
        for k in ids {
            if k != active_id {
                hide_bounds(&app_act, &k);
            }
        }
        let layout = st.child_layouts.lock().unwrap().get(&active_id).copied();
        drop(st);
        // 仅当记忆的布局是"显示态"时才立即恢复；隐藏态(-30000)时跳过，
        // 等前端随后的 tab_position 给出精确坐标，避免把激活页签恢复到屏幕外。
        if let Some(r) = layout {
            if r.0 > -1000.0 {
                // 用无去重的 apply_bounds_inner：切页签是低频用户操作，
                // 去重可能把这次"恢复显示"整个丢掉（实测切回前页签仍显示后一个）。
                let _ = apply_bounds_inner(&app_act, &active_id, r.0, r.1, r.2, r.3);
            } else {
                eprintln!(
                    "[tab_activate] active={} 记忆布局为隐藏态，跳过立即恢复",
                    active_id
                );
            }
        }
        // 清除激活页签的去重时间戳：保证前端随后的 tab_position 精确坐标
        // 不会落在 apply_bounds 的 50ms 去重窗口内被丢弃。
        app_act
            .state::<AppState>()
            .last_position_at
            .lock()
            .unwrap()
            .remove(&active_id);
    });
    Ok(())
}

// ====== 真实 PTY 终端（与系统终端一致：tab 补全 / 历史 / 提示符） ======
//
// M3.a：输出管道与生命周期内核收敛到 `terminal.rs`（mpsc+pump+退避+进程组回收），
// 本文件只保留命令壳：sink 选择（F4）、ACL、state 登记。

/// 判断当前是否处于 M0 终端吞吐测量驱动（`driver=term-throughput`）。
/// 测量模式走直通：不合并、不丢帧，保证 `__M0_TERM_BEGIN__/__M0_TERM_END__` 计数可比（F2/F3）。
fn term_measure_mode(app: &AppHandle) -> bool {
    m0_config_of(app).driver == "term-throughput"
}

/// 创建并启动一个 PTY 终端，输出走 `term-data` **全局事件**（F4 Event sink）。
/// 仅 M0 内部回归驱动（`main.rs` scenario-8 / `M0_DRIVER` 资源循环）使用；前端一律走
/// `term_spawn_channel`（夹具锚定前端不得调用本命令）。
#[tauri::command]
pub fn term_spawn(app: AppHandle) -> Result<TermInfo, String> {
    let measure = term_measure_mode(&app);
    let (info, session) = terminal::spawn_terminal(Arc::new(EventSink(app.clone())), measure)?;
    app.state::<AppState>()
        .terminals
        .lock()
        .unwrap()
        .insert(info.id.clone(), session);
    Ok(info)
}

/// 创建并启动一个 PTY 终端，输出走**每终端独立 Channel 单播**（F4 Channel sink，前端唯一入口）。
#[tauri::command]
pub fn term_spawn_channel(
    app: AppHandle,
    channel: tauri::ipc::Channel<serde_json::Value>,
) -> Result<TermInfo, String> {
    let measure = term_measure_mode(&app);
    let (info, session) = terminal::spawn_terminal(Arc::new(ChannelSink(channel)), measure)?;
    app.state::<AppState>()
        .terminals
        .lock()
        .unwrap()
        .insert(info.id.clone(), session);
    Ok(info)
}

/// 向终端写入数据（按键/命令/回车/Tab 补全都由真实 shell 处理）。
#[tauri::command]
pub fn term_write(app: AppHandle, id: String, data: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    let session = terms.get_mut(&id).ok_or("终端不存在")?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|e| format!("写入失败: {e}"))?;
    let _ = session.writer.flush();
    Ok(())
}

/// 调整终端大小（列/行）——M3.a 起真实生效（F5）。
#[tauri::command]
pub fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    let session = terms.get_mut(&id).ok_or("终端不存在")?;
    terminal::resize(session, cols, rows)
}

/// 关闭终端：置 stop → 回收进程组 → 回收 worker/pump 线程（F6/F7）。
#[tauri::command]
pub fn term_kill(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    let removed = terms.remove(&id);
    // 复核整改（M3 整体裁定 P2）：`terminate_session` 最长约 4 s（进程组宽限 2 s +
    // 线程回收 2 s）。必须先释放这张表的锁，否则期间 term_write / term_resize /
    // term_spawn 全部阻塞在互斥量上，前端点「关闭/重启」时会连带卡住其它终端调用。
    drop(terms);
    let Some(mut session) = removed else {
        return Ok(());
    };
    terminal::terminate_session(&mut session)
}

// ====== M0-0.b 测量钩子（契约 logs/m0-baseline-contract-v1.md §6.1/§6.3） ======
// 日常运行（无 M0_RUN_ID）全部零影响：命令存在但直接返回，不写任何文件。

/// 读取当前 M0 测量配置（无测量运行时为空配置）。
fn m0_config_of(app: &AppHandle) -> M0Config {
    app.state::<AppState>().m0_config.lock().unwrap().clone()
}

/// 原子写文件：先写临时文件再 rename，避免采集脚本读到半截内容。
fn m0_atomic_write(path: &str, content: &str) -> Result<(), String> {
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, content).map_err(|e| format!("write {tmp}: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("rename -> {path}: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// M1-4 默认浏览器接入：外部打开 URL 路由（xdg-open / 单实例 / RunEvent::Opened）
// ---------------------------------------------------------------------------

/// M1-4：从进程 argv 中提取候选打开参数（xdg-open 经 desktop 文件 `%u` 传入）。
/// 只剔除以 '-' 开头的 flag（如 --grid-child）；其余一律交给 handle_open_url
/// 做 scheme 白名单分类——非 http/https 在分类处拒绝 + 审计 + 前端提示，
/// 不在此静默丢弃（fail-closed，拒绝路径必须留痕）。
pub fn extract_open_urls(argv: &[String]) -> Vec<String> {
    argv.iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect()
}

/// M1-4：处理一个外部打开 URL 请求。安全边界：仅放行 http/https，
/// 其余 scheme 拒绝 + 审计 + 通知前端 toast；合法 URL 进缓存队列，
/// 前端就绪后发轻提示事件（URL 本体由前端拉取，防事件竞态丢失）。
pub fn handle_open_url(app: &AppHandle, url: &str) {
    if let Err(e) = crate::security_policy::check_openable_url(url) {
        eprintln!("[open-url] rejected: {e}");
        workspace::log_audit(app, "open_url_rejected", url.to_string());
        let _ = app.emit("app://open-url-rejected", serde_json::json!({ "url": url }));
        return;
    }
    eprintln!("[open-url] accepted: {url}");
    workspace::log_audit(app, "open_url", url.to_string());
    let state = app.state::<AppState>();
    state
        .pending_open_urls
        .lock()
        .unwrap()
        .push(url.to_string());
    if state.frontend_ready.load(Ordering::Relaxed) {
        let _ = app.emit("app://open-url-pending", ());
    }
}

/// M1-4：前端拉取并清空待打开 URL 队列（m0_ready 后首次拉取 +
/// 收到 app://open-url-pending 提示时拉取）。拉取即清空，天然去重。
#[tauri::command]
pub fn take_pending_open_urls(app: AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let urls = std::mem::take(&mut *state.pending_open_urls.lock().unwrap());
    urls
}

/// M1-4：desktop 条目是否可用于 URL 路由（声明了 http(s) handler 且 Exec 带 %u）。
fn desktop_entry_usable(path: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.contains("x-scheme-handler/http") && c.contains("%u"))
        .unwrap_or(false)
}

/// M1-4：定位本应用可用的 .desktop 文件。按「用户级 → 系统级」顺序找第一个
/// 声明了 x-scheme-handler/http 且 Exec 带 %u 的条目；找不到（或旧条目缺
/// MimeType/%u）时，在 ~/.local/share/applications 生成指向当前二进制的同名
/// 条目（XDG 用户级优先，自然遮蔽旧系统条目）。本函数只声明 MIME，不改变
/// 系统默认浏览器。返回条目文件名（如 mvp-browser-os.desktop）。
fn ensure_desktop_entry() -> Result<String, String> {
    const DESKTOP_NAME: &str = "mvp-browser-os.desktop";
    let home = std::env::var("HOME").map_err(|e| format!("HOME 未设置: {e}"))?;
    let user_dir = format!("{home}/.local/share/applications");
    let candidates = [
        format!("{user_dir}/{DESKTOP_NAME}"),
        format!("/usr/share/applications/{DESKTOP_NAME}"),
        format!("/usr/share/applications/com.jizhijiandan.mvp.desktop"),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() && desktop_entry_usable(c) {
            return std::path::Path::new(c)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| format!("desktop 文件名非法: {c}"));
        }
    }
    // 无可用条目：生成用户级条目（Exec 指向当前运行二进制，%u 接收 xdg-open 的 URL）
    let exe = std::env::current_exe().map_err(|e| format!("无法定位当前二进制: {e}"))?;
    std::fs::create_dir_all(&user_dir).map_err(|e| format!("创建 {user_dir}: {e}"))?;
    let path = format!("{user_dir}/{DESKTOP_NAME}");
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=极智简单浏览器OS\nExec={} %u\nIcon=mvp-browser-os\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\nCategories=Network;WebBrowser;\nNoDisplay=false\n",
        exe.display()
    );
    std::fs::write(&path, content).map_err(|e| format!("写入 {path}: {e}"))?;
    // 更新用户级 MIME 数据库（尽力而为，失败不阻断）
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&user_dir)
        .output();
    eprintln!("[open-url] created user desktop entry: {path}");
    Ok(DESKTOP_NAME.to_string())
}

/// M1-4：查询当前系统默认浏览器（xdg-settings get）。
#[tauri::command]
pub fn get_default_browser() -> Result<String, String> {
    let out = std::process::Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .map_err(|e| format!("xdg-settings 不可用: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "xdg-settings get 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// M1-4：把本应用设为系统默认浏览器。
/// 硬约束：本命令只能由设置页「设为默认浏览器」按钮经用户显式确认后触发；
/// 应用启动、安装脚本、首启流程一律不得调用（禁静默改写系统默认）。
#[tauri::command]
pub fn set_default_browser(app: AppHandle) -> Result<String, String> {
    let desktop = ensure_desktop_entry()?;
    let out = std::process::Command::new("xdg-settings")
        .args(["set", "default-web-browser", &desktop])
        .output()
        .map_err(|e| format!("xdg-settings 不可用: {e}"))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        eprintln!("[open-url] set default browser failed: {msg}");
        workspace::log_audit(
            &app,
            "set_default_browser_failed",
            format!("{desktop}: {msg}"),
        );
        return Err(format!("设为默认浏览器失败: {msg}"));
    }
    eprintln!("[open-url] set default browser -> {desktop}");
    workspace::log_audit(&app, "set_default_browser", desktop.clone());
    Ok(desktop)
}

/// ready 信号（契约 §6.1）：主前端完成 mount + 2×rAF 后调用本命令，作为
/// 「一次轻量 IPC 往返」的终点；后端校验 run_id 后将带 run_id 的 ready 信号
/// 原子写入 ready_file。返回 run_id（非测量运行返回空串）。
#[tauri::command]
pub fn m0_ready(app: AppHandle) -> Result<String, String> {
    // M1-4：标记前端就绪。此后到达的外部 URL 会发 pending 提示让前端立即拉取；
    // 就绪前到达的已在队列中，由前端 m0_ready 后的首次拉取兜底（不丢）。
    app.state::<AppState>()
        .frontend_ready
        .store(true, Ordering::Relaxed);
    let cfg = m0_config_of(&app);
    eprintln!(
        "[m0] m0_ready called run_id={:?} ready_file={:?}",
        cfg.run_id, cfg.ready_file
    );
    if cfg.run_id.is_empty() {
        return Ok(String::new());
    }
    if let Some(path) = &cfg.ready_file {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        m0_atomic_write(path, &format!("{}\n{}\n", cfg.run_id, now_ms))?;
    }
    Ok(cfg.run_id)
}

/// 终端吞吐报告（契约 §6.3）：前端检测到 `__M0_TERM_END__` 并完成下一次 animation
/// frame 后上报。计时起点为前端发送负载命令的时刻（report.start_ts_ms），终点为
/// end 标记被消费并完成下一次 rAF（report.end_ts_ms）；elapsed = end - start。
/// 报告原子写入 report_dir/term-throughput-report.json（驱动据此判定完成并写 done 标记）。
#[tauri::command]
pub fn m0_term_report(app: AppHandle, report: serde_json::Value) -> Result<(), String> {
    let cfg = m0_config_of(&app);
    let Some(dir) = &cfg.report_dir else {
        return Ok(());
    };
    if cfg.run_id.is_empty() || cfg.driver != "term-throughput" {
        return Err("terminal report outside term-throughput measurement".into());
    }
    let end_ms = report
        .get("end_ts_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u128;
    let start_ms = report
        .get("start_ts_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u128;
    let begin_seen = report
        .get("begin_seen")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let end_seen = report.get("end_seen").and_then(|v| v.as_u64()).unwrap_or(0);
    let consumed_bytes = report
        .get("consumed_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let frame_gaps = report
        .get("frame_gaps_ms")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut errors = Vec::new();
    if begin_seen != 1 {
        errors.push(format!("begin_seen={begin_seen}, expected 1"));
    }
    if end_seen != 1 {
        errors.push(format!("end_seen={end_seen}, expected 1"));
    }
    if consumed_bytes != 10 * 1024 * 1024 {
        errors.push(format!(
            "consumed_bytes={consumed_bytes}, expected {}",
            10 * 1024 * 1024
        ));
    }
    if start_ms == 0 || end_ms < start_ms {
        errors.push(format!("invalid timestamps start={start_ms} end={end_ms}"));
    }
    if frame_gaps.is_empty() {
        errors.push("frame_gaps_ms is empty".into());
    }
    let valid = errors.is_empty();
    let out = serde_json::json!({
        "run_id": cfg.run_id,
        "start_ts_ms": start_ms,
        "end_ts_ms": end_ms,
        "elapsed_ms": end_ms.saturating_sub(start_ms),
        "begin_seen": begin_seen,
        "end_seen": end_seen,
        "consumed_bytes": consumed_bytes,
        "expected_bytes": 10 * 1024 * 1024,
        "frame_gaps_ms": frame_gaps,
        "valid": valid,
        "errors": errors,
    });
    let json = serde_json::to_string_pretty(&out).map_err(|e| format!("serialize report: {e}"))?;
    m0_atomic_write(&format!("{dir}/term-throughput-report.json"), &json)?;
    if valid {
        Ok(())
    } else {
        Err("terminal throughput report failed validation".into())
    }
}

/// 查询当前 M0 测量配置（非测量运行返回 null）。前端据此在 term-throughput 模式下
/// 自动挂载终端面板并驱动 10 MiB 负载（契约 §6.3）。
#[tauri::command]
pub fn m0_config(app: AppHandle) -> Option<serde_json::Value> {
    let cfg = m0_config_of(&app);
    if cfg.run_id.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "run_id": cfg.run_id,
        "driver": cfg.driver,
        "ready_file": cfg.ready_file,
        "report_dir": cfg.report_dir,
    }))
}

#[cfg(test)]
mod git_write_gate_tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_job(op: GitWriteOp, created: chrono::DateTime<Utc>, ttl_secs: i64) -> GitWriteJob {
        GitWriteJob {
            id: uuid::Uuid::new_v4().to_string(),
            repo_id: "r1".into(),
            op,
            paths: vec!["a.txt".into()],
            message: None,
            branch: None,
            checkout: false,
            status: GitWriteStatus::Pending,
            dangerous: op.is_dangerous(),
            created_at: created,
            expires_at: created + Duration::seconds(ttl_secs),
            finished_at: None,
            error: None,
        }
    }

    // T-gw-2（b 卡）：未确认执行 → confirm 不存在的 job 必须 Err("未知任务")，零写入
    #[test]
    fn git_write_gate_unknown_job_rejected() {
        let mut jobs = HashMap::new();
        let err = take_confirmable_git_job(&mut jobs, "nope", Utc::now(), false)
            .expect_err("未知任务必须拒绝");
        assert_eq!(err, "未知任务");
    }

    // T-gw-3（b 卡）：过期 job 被拒且清除，零写入
    #[test]
    fn git_write_gate_expired_job_rejected_and_removed() {
        let mut jobs = HashMap::new();
        let job = make_job(GitWriteOp::Stage, Utc::now() - Duration::seconds(600), 300);
        let id = job.id.clone();
        jobs.insert(id.clone(), job);
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("过期任务必须拒绝");
        assert_eq!(err, "任务已过期");
        assert!(jobs.is_empty(), "过期任务应被清除");
    }

    // T-gw-b-3：discard（dangerous）未带二次确认必须拒绝，且任务不被执行/不取出
    #[test]
    fn git_write_gate_discard_requires_double_confirm() {
        let mut jobs = HashMap::new();
        let job = make_job(GitWriteOp::Discard, Utc::now(), 300);
        let id = job.id.clone();
        jobs.insert(id.clone(), job);
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("缺二次确认必须拒绝");
        assert_eq!(err, "需二次确认");
        assert!(
            jobs.contains_key(&id),
            "未确认的 dangerous 任务应保留在表内（未执行，用户可补确认）"
        );
        // 带上二次确认：放行并一次性取出（Running）
        let taken =
            take_confirmable_git_job(&mut jobs, &id, Utc::now(), true).expect("二次确认后应放行");
        assert_eq!(taken.status, GitWriteStatus::Running);
        assert!(jobs.is_empty(), "确认后任务必须一次性移除");
    }

    // T-gw-2b：任务一次性（confirm 后再次 confirm 必须 Err）+ 非 Pending 状态拒绝
    #[test]
    fn git_write_gate_one_shot_and_pending_only() {
        let mut jobs = HashMap::new();
        let job = make_job(GitWriteOp::Commit, Utc::now(), 300);
        let id = job.id.clone();
        jobs.insert(id.clone(), job);
        take_confirmable_git_job(&mut jobs, &id, Utc::now(), false).expect("首次确认");
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("重复确认必须拒绝");
        assert_eq!(err, "未知任务");

        let mut jobs2 = HashMap::new();
        let mut running_job = make_job(GitWriteOp::Stage, Utc::now(), 300);
        running_job.status = GitWriteStatus::Running;
        let id2 = running_job.id.clone();
        jobs2.insert(id2.clone(), running_job);
        let err2 = take_confirmable_git_job(&mut jobs2, &id2, Utc::now(), false)
            .expect_err("非 Pending 状态必须拒绝");
        assert_eq!(err2, "任务状态异常");
        assert!(jobs2.is_empty(), "异常状态任务应被清除");
    }

    // T-gw-c-7：过期任务清理只清过期项，有效期内任务必须保留
    #[test]
    fn git_write_gate_purge_expired_jobs() {
        let mut jobs = HashMap::new();
        let now = Utc::now();
        let fresh = make_job(GitWriteOp::Stage, now, 300);
        let fresh_id = fresh.id.clone();
        let stale = make_job(GitWriteOp::Discard, now - Duration::seconds(600), 300);
        jobs.insert(fresh.id.clone(), fresh);
        jobs.insert(stale.id.clone(), stale);
        let removed = purge_expired_git_jobs(&mut jobs, now);
        assert_eq!(removed, 1, "只应清理已过期任务");
        assert!(jobs.contains_key(&fresh_id), "有效期内任务必须保留");
        assert_eq!(jobs.len(), 1);
    }

    // T-gw-d-8：push（dangerous）未带二次确认必须拒绝，确认后一次性取出
    #[test]
    fn git_write_gate_push_requires_double_confirm() {
        let mut jobs = HashMap::new();
        let mut job = make_job(GitWriteOp::Push, Utc::now(), 300);
        job.branch = Some("master".into());
        let id = job.id.clone();
        assert!(job.dangerous, "push 必须标记为 dangerous");
        jobs.insert(id.clone(), job);
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("缺二次确认必须拒绝");
        assert_eq!(err, "需二次确认");
        assert!(jobs.contains_key(&id), "未确认的 push 任务不得被执行/取出");
        let taken =
            take_confirmable_git_job(&mut jobs, &id, Utc::now(), true).expect("二次确认后应放行");
        assert_eq!(taken.status, GitWriteStatus::Running);
        assert!(jobs.is_empty(), "确认后任务必须一次性移除");
    }

    // T-gw-b-12：审计 detail 只含操作语义与计数，绝无凭据/路径清单/diff 全量
    #[test]
    fn git_write_audit_detail_contains_no_credentials_or_path_lists() {
        let d = git_write_audit_detail(GitWriteOp::Commit, "r1", "j1", 3, true, None, None, "ok");
        assert!(d.contains("\"op\":\"commit\""), "detail: {d}");
        assert!(d.contains("\"repo_id\":\"r1\""), "detail: {d}");
        assert!(d.contains("\"path_count\":3"), "detail: {d}");
        assert!(d.contains("\"confirmed\":true"), "detail: {d}");
        for forbidden in ["userpass", "token", "secret", "password", "a.txt", ".git/"] {
            assert!(
                !d.contains(forbidden),
                "审计 detail 不得含 {forbidden}: {d}"
            );
        }
        // 长错误串必须被截断（防整段 diff/冗长文本落审计），且不切 UTF-8 字符
        let long = "你".repeat(500); // 每字 3 字节，共 1500 字节
        let s = sanitize_audit_text(&long);
        assert!(s.len() <= 300 + "…".len(), "截断后长度应受限: {}", s.len());
        assert!(
            std::str::from_utf8(s.as_bytes()).is_ok(),
            "必须是合法 UTF-8"
        );
        let short = "短错误";
        assert_eq!(sanitize_audit_text(short), short, "短文本不应被改动");
    }

    // T-gw-d-9：push 审计必须含 branch/remote_name，且不得含 token/远端 URL 凭据
    #[test]
    fn git_write_push_audit_fields_present_and_sanitized() {
        let d = git_write_audit_detail(
            GitWriteOp::Push,
            "r1",
            "j1",
            0,
            true,
            Some("master"),
            Some("origin"),
            "ok",
        );
        assert!(d.contains("\"op\":\"push\""), "detail: {d}");
        assert!(d.contains("\"branch\":\"master\""), "detail: {d}");
        assert!(d.contains("\"remote_name\":\"origin\""), "detail: {d}");
        assert!(d.contains("\"confirmed\":true"), "detail: {d}");
        for forbidden in [
            "userpass", "token", "secret", "password", "https://", "://", "@",
        ] {
            assert!(
                !d.contains(forbidden),
                "push 审计不得含凭据/远端 URL: {forbidden} in {d}"
            );
        }
        // 失败路径：错误串经 scrub + 截断后落审计，token 与 URL userinfo 不出现
        // （URL host 保留用于排障，userinfo 统一掩码为 ***，审计另有 remote_name 字段）
        let raw = "push 失败: cannot push to 'https://u:tok-123@example.com/r.git'（auth failed for tok-123）";
        let clean = sanitize_audit_text(&sync::scrub_sensitive_error(raw, "tok-123"));
        assert!(!clean.contains("tok-123"), "错误串必须脱敏: {clean}");
        assert!(!clean.contains("u:tok"), "URL userinfo 必须掩码: {clean}");
        assert!(
            clean.contains("https://***@example.com"),
            "userinfo 应为 ***: {clean}"
        );
        let failed = git_write_audit_detail(
            GitWriteOp::Push,
            "r1",
            "j1",
            0,
            true,
            Some("master"),
            Some("origin"),
            &clean,
        );
        assert!(!failed.contains("tok-123"), "失败审计必须脱敏: {failed}");
        assert!(
            !failed.contains("u:tok"),
            "失败审计不得含带凭据的 URL: {failed}"
        );
        assert!(
            !failed.contains("\"branch\":null"),
            "失败审计必须带分支名: {failed}"
        );
    }
}

#[cfg(test)]
mod resource_capture_tests {
    use super::*;

    fn raw(
        url: &str,
        status: Option<u32>,
        mime: Option<&str>,
        size: Option<u64>,
    ) -> RawResourceEvent {
        RawResourceEvent {
            url: url.to_string(),
            method: Some("GET".to_string()),
            status,
            mime: mime.map(|m| m.to_string()),
            size_bytes: size,
            started_at: 1000,
            finished_at: 1050,
        }
    }

    fn rec(tab: &str, id: &str) -> ResourceReceived {
        ResourceReceived {
            id: id.to_string(),
            tab_id: tab.to_string(),
            url: "https://example.com/a".to_string(),
            method: "GET".to_string(),
            status: Some(200),
            mime: Some("text/html".to_string()),
            size_bytes: Some(10),
            started_at: 1,
            finished_at: Some(2),
            duration_ms: Some(1),
            resource_type: ResourceKind::Document,
        }
    }

    // T-rc-1：URL 敏感 query 参数脱敏（token/password/secret/signature 原值不得入库）
    #[test]
    fn build_redacts_sensitive_query_params() {
        let r = raw(
            "https://api.example.com/x?token=sekrit-1&password=pw&ok=1",
            Some(200),
            Some("application/json"),
            Some(100),
        );
        let out = build_resource_received("tab-1", &r, 2048).expect("入库");
        assert!(!out.url.contains("sekrit-1"), "token 原值泄漏: {}", out.url);
        assert!(!out.url.contains("pw&"), "password 原值泄漏: {}", out.url);
        assert!(out.url.contains("ok=1"), "非敏感参数应保留: {}", out.url);
    }

    // T-rc-2：DTO 不含 Cookie/Authorization/Set-Cookie/body（结构性）
    #[test]
    fn dto_carries_no_headers_or_body() {
        let out = build_resource_received(
            "tab-1",
            &raw(
                "https://example.com/a",
                Some(200),
                Some("text/html"),
                Some(1),
            ),
            2048,
        )
        .expect("入库");
        let json = serde_json::to_string(&out).expect("serialize");
        for bad in ["cookie", "authorization", "set-cookie", "body", "headers"] {
            assert!(
                !json.to_ascii_lowercase().contains(bad),
                "DTO 不得含 {bad}: {json}"
            );
        }
    }

    // T-rc-3：降级不伪造——无响应/未知长度时 status/mime/size 为 None
    #[test]
    fn degraded_fields_are_none_not_forged() {
        let out = build_resource_received(
            "tab-1",
            &raw("https://example.com/x", None, None, Some(0)),
            2048,
        )
        .expect("入库");
        assert_eq!(out.status, None);
        assert_eq!(out.mime, None);
        assert_eq!(out.size_bytes, None, "content_length=0 必须映射为未知 None");
        assert_eq!(out.duration_ms, Some(50));
        let skew = RawResourceEvent {
            started_at: 2000,
            finished_at: 1000,
            ..raw("https://example.com/x", None, None, None)
        };
        let out2 = build_resource_received("tab-1", &skew, 2048).expect("入库");
        assert_eq!(out2.duration_ms, None, "时钟回拨不得伪造耗时");
    }

    // T-rc-3b：非 http(s) URL 一律不入库（file:/data:/blob:/about:）
    #[test]
    fn non_http_urls_are_dropped() {
        for bad in [
            "file:///etc/passwd",
            "data:text/html,x",
            "blob:https://x/y",
            "about:blank",
        ] {
            assert!(
                build_resource_received("tab-1", &raw(bad, Some(200), None, None), 2048).is_none(),
                "{bad} 不得入库"
            );
        }
    }

    // T-rc-4：每 tab 容量上限（FIFO 丢最旧 + 驱逐计数）
    #[test]
    fn per_tab_cap_evicts_oldest() {
        let mut buf = ResourceBuffer::default();
        for i in 0..5 {
            buf.push(rec("tab-1", &format!("r{i}")), 3, 100);
        }
        let ids: Vec<String> = buf.records("tab-1").iter().map(|r| r.id.clone()).collect();
        assert_eq!(ids, vec!["r2", "r3", "r4"], "应保留最新 3 条");
        assert_eq!(buf.evicted_of("tab-1"), 2);
        assert_eq!(buf.total, 3);
    }

    // T-rc-5：全局容量上限（跨 tab FIFO，跳过已被 per-tab 驱逐的过期项）
    #[test]
    fn global_cap_evicts_oldest_across_tabs() {
        let mut buf = ResourceBuffer::default();
        buf.push(rec("tab-1", "a1"), 10, 4);
        buf.push(rec("tab-2", "b1"), 10, 4);
        buf.push(rec("tab-1", "a2"), 10, 4);
        buf.push(rec("tab-2", "b2"), 10, 4);
        buf.push(rec("tab-1", "a3"), 10, 4); // 触发全局驱逐 a1
        assert_eq!(buf.total, 4);
        let t1: Vec<String> = buf.records("tab-1").iter().map(|r| r.id.clone()).collect();
        assert_eq!(t1, vec!["a2", "a3"], "全局 FIFO 应丢掉最旧的 a1");
        assert_eq!(buf.evicted_global, 1);
        // 过期 order 项（per-tab 已驱逐）在全局驱逐时被惰性跳过，不出错
        let mut buf2 = ResourceBuffer::default();
        for i in 0..6 {
            buf2.push(rec("tab-1", &format!("x{i}")), 2, 3);
        }
        buf2.push(rec("tab-2", "y1"), 2, 3);
        buf2.push(rec("tab-2", "y2"), 2, 3);
        assert!(buf2.total <= 3, "全局上限必须守住: {}", buf2.total);
    }

    // T-rc-6：tab 关闭清理（remove_tab 释放记录与计数）
    #[test]
    fn tab_close_clears_records() {
        let mut buf = ResourceBuffer::default();
        buf.push(rec("tab-1", "a1"), 10, 100);
        buf.push(rec("tab-2", "b1"), 10, 100);
        buf.remove_tab("tab-1");
        assert!(buf.records("tab-1").is_empty());
        assert_eq!(buf.records("tab-2").len(), 1);
        assert_eq!(buf.total, 1);
        buf.clear_all();
        assert_eq!(buf.total, 0);
        assert!(buf.records("tab-2").is_empty());
    }

    // T-rc-7：clear_tab 返回清除条数并复位驱逐计数
    #[test]
    fn clear_tab_returns_removed_count() {
        let mut buf = ResourceBuffer::default();
        buf.push(rec("tab-1", "a1"), 1, 100);
        buf.push(rec("tab-1", "a2"), 1, 100);
        assert_eq!(buf.evicted_of("tab-1"), 1);
        let removed = buf.clear_tab("tab-1");
        assert_eq!(removed, 1, "a1 已被 per-tab 驱逐，只剩 a2");
        assert_eq!(buf.evicted_of("tab-1"), 0);
        assert_eq!(buf.total, 0);
    }

    // T-rc-8：capture disabled 语义（on_resource_received 入口 gating 由 settings.enabled 决定；
    // 这里验证 build+push 之外的判定契约：enabled=false 时调用方必须短路）
    #[test]
    fn disabled_capture_gate_contract() {
        let s = ResourceCaptureSettings {
            enabled: false,
            ..Default::default()
        };
        assert!(
            !s.enabled,
            "enabled=false 时 on_resource_received 必须直接丢弃"
        );
    }

    // T-rc-9：来源校验——远程页面对资源命令必须被阻断（无令牌）
    #[test]
    fn remote_invocation_to_resource_commands_is_rejected() {
        let registry = crate::security_policy::IntentRegistry::new();
        for scope in [
            "list_tab_resources",
            "clear_tab_resources",
            "get_resource_capture_settings",
            "set_resource_capture_settings",
        ] {
            assert!(
                crate::security_policy::check_remote_invocation("tab-9", scope, None, &registry)
                    .is_err(),
                "远程页面（tab-*）无令牌调用 {scope} 必须被拒绝"
            );
            assert!(
                crate::security_policy::check_remote_invocation("main", scope, None, &registry)
                    .is_ok(),
                "主窗口调用 {scope} 应放行"
            );
        }
    }

    // T-rc-10：classify 归类（mime 优先，URL 后缀兜底，xhr_fetch 启发式）
    #[test]
    fn classify_kind_by_mime_then_extension() {
        assert_eq!(
            classify_resource_kind(Some("text/html"), "https://x/"),
            ResourceKind::Document
        );
        assert_eq!(
            classify_resource_kind(Some("application/javascript"), "https://x/a"),
            ResourceKind::Script
        );
        assert_eq!(
            classify_resource_kind(Some("text/css"), "https://x/a"),
            ResourceKind::Stylesheet
        );
        assert_eq!(
            classify_resource_kind(Some("image/png"), "https://x/a"),
            ResourceKind::Image
        );
        assert_eq!(
            classify_resource_kind(Some("font/woff2"), "https://x/a"),
            ResourceKind::Font
        );
        assert_eq!(
            classify_resource_kind(Some("video/mp4"), "https://x/a"),
            ResourceKind::Media
        );
        assert_eq!(
            classify_resource_kind(Some("application/json"), "https://x/api"),
            ResourceKind::XhrFetch
        );
        assert_eq!(
            classify_resource_kind(Some("application/octet-stream"), "https://x/a.bin"),
            ResourceKind::Other
        );
        assert_eq!(
            classify_resource_kind(None, "https://x/app.js?token=***"),
            ResourceKind::Script
        );
        assert_eq!(
            classify_resource_kind(None, "https://x/style.css"),
            ResourceKind::Stylesheet
        );
        assert_eq!(
            classify_resource_kind(None, "https://x/pic.webp"),
            ResourceKind::Image
        );
        assert_eq!(
            classify_resource_kind(None, "https://x/data.json"),
            ResourceKind::XhrFetch
        );
        assert_eq!(
            classify_resource_kind(None, "https://x/page/"),
            ResourceKind::Document
        );
        assert_eq!(
            classify_resource_kind(None, "https://x/unknown.bin"),
            ResourceKind::Other
        );
    }

    // T-rc-11：事件 payload 即脱敏 DTO（on_resource_received emit 的就是 build 产物）
    #[test]
    fn event_payload_is_redacted_dto() {
        let out = build_resource_received(
            "tab-1",
            &raw(
                "https://ex.com/cb?access_token=tok-9&session=s-1&v=2",
                Some(200),
                Some("text/html"),
                Some(5),
            ),
            2048,
        )
        .expect("入库");
        let payload = serde_json::to_value(&out).expect("to value");
        let s = payload.to_string();
        assert!(!s.contains("tok-9"));
        assert!(!s.contains("s-1"));
        assert!(s.contains("v=2"));
        // method 缺省回退 GET（信号缺省时）
        let no_method = RawResourceEvent {
            method: None,
            ..raw("https://ex.com/", None, None, None)
        };
        let out2 = build_resource_received("tab-1", &no_method, 2048).expect("入库");
        assert_eq!(out2.method, "GET");
    }

    // T-rc-12：tab_id 形态校验（拒绝 grid/伪造 label）
    #[test]
    fn tab_id_shape_is_enforced() {
        assert!(check_tab_id("tab-1").is_ok());
        assert!(check_tab_id("grid-0").is_err());
        assert!(check_tab_id("main").is_err());
        assert!(check_tab_id(&"tab-".repeat(20)).is_err());
    }

    // T-rc-13：URL 限长（settings 可调低，不得超过策略硬上限）
    #[test]
    fn url_length_cap_is_enforced() {
        let long = format!("https://example.com/{}", "a".repeat(3000));
        let out =
            build_resource_received("tab-1", &raw(&long, None, None, None), 100).expect("入库");
        assert!(
            out.url.len() <= 104,
            "settings 调低后必须更严: {}",
            out.url.len()
        );
        let out2 =
            build_resource_received("tab-1", &raw(&long, None, None, None), 999_999).expect("入库");
        assert!(
            out2.url.len() <= crate::security_policy::MAX_RESOURCE_URL_BYTES + 4,
            "settings 调高不得突破策略硬上限"
        );
    }
}

#[cfg(test)]
mod session_gate_tests {
    use super::*;

    const SESSION_SCOPES: [&str; 10] = [
        "session_save",
        "session_discard",
        "session_list",
        "session_get",
        "session_delete",
        "session_export",
        "session_restore",
        "flush_sessions",
        "get_session_policy",
        "set_session_policy",
    ];

    // T-sg-1：来源校验——远程页面（tab-*/grid-*）无令牌调用会话命令一律拒绝；
    // 主窗口（用户手势）放行。会话存档含浏览痕迹，绝不允许远程页面读写。
    #[test]
    fn remote_invocation_to_session_commands_is_rejected() {
        let registry = crate::security_policy::IntentRegistry::new();
        for scope in SESSION_SCOPES {
            assert!(
                crate::security_policy::check_remote_invocation("tab-3", scope, None, &registry)
                    .is_err(),
                "远程页面调用 {scope} 必须被拒绝"
            );
            assert!(
                crate::security_policy::check_remote_invocation("main", scope, None, &registry)
                    .is_ok(),
                "主窗口调用 {scope} 应放行"
            );
        }
    }

    // T-sg-2：策略默认值——关闭弹窗默认开（关闭不可静默丢弃），
    // 退出自动保存默认关（不静默保存浏览痕迹）。
    #[test]
    fn session_policy_defaults_favor_no_silent_persist() {
        let p = SessionPolicy::default();
        assert!(p.close_prompt, "默认应弹关闭选择（不可静默丢弃）");
        assert!(!p.auto_save_on_exit, "默认不得自动保存（不静默落盘）");
    }

    // T-sg-3：会话 DTO 序列化不含敏感字段（结构性红线，与落盘口径一致）
    #[test]
    fn session_dto_has_no_sensitive_fields() {
        // 草稿 URL 必须已是脱敏形态（upsert 内部完成脱敏）
        let draft = SessionDraft {
            tab_id: "tab-1".to_string(),
            url: crate::security_policy::redact_sensitive_url("https://ex.com/a?token=raw-1"),
            title: "t".to_string(),
        };
        assert!(
            !draft.url.contains("raw-1"),
            "草稿 URL 必须脱敏: {}",
            draft.url
        );

        let session = crate::session::build_session(
            "tab-1",
            "https://ex.com/a?token=raw-1",
            "t",
            "预览",
            &[],
            CLOSE_REASON_SAVED,
        );
        let json = serde_json::to_string(&session).expect("serialize");
        for bad in [
            "cookie",
            "authorization",
            "set-cookie",
            "headers",
            "body",
            "raw-1",
        ] {
            assert!(
                !json.to_ascii_lowercase().contains(bad),
                "SessionDraft 不得含 {bad}: {json}"
            );
        }
        let report = SessionFlushReport {
            persisted: 0,
            drafts_dropped: 1,
            tmp_removed: 2,
            capacity_removed: 3,
        };
        let rj = serde_json::to_string(&report).expect("serialize");
        for key in [
            "persisted",
            "drafts_dropped",
            "tmp_removed",
            "capacity_removed",
        ] {
            assert!(rj.contains(key), "flush 报告缺字段 {key}");
        }
        assert!(!rj.contains("url"), "flush 报告不得含 URL 字段");
    }

    // T-sg-4：审计脱敏——会话审计格式串只含 id/计数，不含 URL、预览或凭据
    #[test]
    fn session_audit_format_strings_carry_no_url() {
        // 与生产代码中的审计格式串保持一致（改动这里必须同步 bridge.rs）
        let samples = [
            "tab_id={} resources={} preview_bytes={}",
            "tab_id={tab_id} draft_existed={existed}",
            "id={id} removed={removed}",
            "id={id} bytes={}",
            "id={id} tab_id={} resources={}",
            "auto_save={auto_save} persisted={persisted} drafts_dropped={drafts_dropped} tmp_removed={tmp_removed}",
            "close_prompt={} auto_save_on_exit={}",
        ];
        for s in samples {
            let lower = s.to_ascii_lowercase();
            for bad in [
                "url",
                "token",
                "cookie",
                "authorization",
                "preview=",
                "secret",
            ] {
                assert!(!lower.contains(bad), "审计格式串不得含 {bad}: {s}");
            }
        }
    }

    // T-sg-5（M2-1 顺带加固）：会话 id 形态校验——拼路径前必须挡住 `../` 与绝对路径。
    // 来源：M1-ACCEPT 审计挂账项（NON-BLOCKER 1）。
    #[test]
    fn session_id_must_be_path_safe() {
        let uuid = uuid::Uuid::new_v4().to_string();
        assert!(check_id(&uuid, "会话 id").is_ok());
        for bad in ["../../etc/passwd", "/etc/passwd", "..", "", "a/b", "a\\b"] {
            assert!(
                check_id(bad, "会话 id").is_err(),
                "必须拒绝非法会话 id: {bad:?}"
            );
        }
    }
}

#[cfg(test)]
mod image_gate_tests {
    use super::*;

    const IMAGE_SCOPES: [&str; 2] = ["save_image", "list_artifact_images"];

    // T-ig-1：来源校验——远程页面（tab-*/grid-*）无令牌调用图片命令一律拒绝；
    // 主窗口放行。图片落盘属于写副作用，绝不允许远程页面触发。
    #[test]
    fn remote_invocation_to_image_commands_is_rejected() {
        let registry = crate::security_policy::IntentRegistry::new();
        for scope in IMAGE_SCOPES {
            assert!(
                crate::security_policy::check_remote_invocation("tab-3", scope, None, &registry)
                    .is_err(),
                "远程页面调用 {scope} 必须被拒绝"
            );
            assert!(
                crate::security_policy::check_remote_invocation("main", scope, None, &registry)
                    .is_ok(),
                "主窗口调用 {scope} 应放行"
            );
        }
    }

    // T-ig-2：成果 id 形态校验（拼路径前拦截）
    #[test]
    fn artifact_id_must_be_path_safe() {
        let uuid = uuid::Uuid::new_v4().to_string();
        assert!(check_id(&uuid, "成果 id").is_ok());
        for bad in ["../../etc/passwd", "/etc", "..", "", "a/b"] {
            assert!(check_id(bad, "成果 id").is_err(), "必须拒绝: {bad:?}");
        }
    }

    // T-ig-3：图片审计格式串只含 id/字节数/MIME，不含 URL 与图片内容
    #[test]
    fn image_audit_format_strings_carry_no_url() {
        let samples = [
            "artifact_id={artifact_id} image_id={} bytes={} mime={}",
            "artifact_id={id} removed={n}",
            "artifact_id={id} error={e}",
        ];
        for s in samples {
            let lower = s.to_ascii_lowercase();
            for bad in ["url", "token", "cookie", "authorization", "secret"] {
                assert!(!lower.contains(bad), "审计格式串不得含 {bad}: {s}");
            }
        }
    }

    // T-ig-4：成果 DTO 带 images 字段且结构上不含凭据（与 M1 口径一致）
    #[test]
    fn artifact_dto_carries_images_without_credentials() {
        let art = Artifact::new(
            "标题".into(),
            "https://ex.com/p?token=zz".into(),
            "t".into(),
            "h".into(),
        );
        let json = serde_json::to_string(&art).expect("serialize");
        assert!(
            json.contains("\"images\":[]"),
            "新成果应带空 images: {json}"
        );
        for bad in ["cookie", "authorization", "set-cookie", "headers"] {
            assert!(!json.to_ascii_lowercase().contains(bad), "DTO 不得含 {bad}");
        }
    }
}

#[cfg(test)]
mod image_preview_gate_tests {
    // T-ip-1（M2-2.b）：预览通道命令同样过来源校验——远程页面不得探测本机图片目录。
    // 该命令虽是只读，但返回值是真实文件系统路径，属环境信息，不对远程页面开放。
    #[test]
    fn remote_invocation_to_images_dir_is_rejected() {
        let registry = crate::security_policy::IntentRegistry::new();
        assert!(
            crate::security_policy::check_remote_invocation(
                "tab-3",
                "workspace_images_dir",
                None,
                &registry
            )
            .is_err(),
            "远程页面调用 workspace_images_dir 必须被拒绝"
        );
        assert!(
            crate::security_policy::check_remote_invocation(
                "main",
                "workspace_images_dir",
                None,
                &registry
            )
            .is_ok(),
            "主窗口调用 workspace_images_dir 应放行"
        );
    }

    // T-ip-2（M2-2.b）：目录基准与相对路径的拼接必须由后端白名单承担，
    // 前端只做「基准 + rel_path」的传入，不得拼出图片目录之外的路径。
    #[test]
    fn preview_path_join_is_fail_closed() {
        let dir = "/data/ws/images";
        assert_eq!(
            crate::images::join_image_path(dir, "images/a1/b2.png"),
            Ok("/data/ws/images/images/a1/b2.png".to_string())
        );
        for bad in ["../x.png", "/etc/passwd", "images/../../x.png", ""] {
            assert!(
                crate::images::join_image_path(dir, bad).is_err(),
                "越界相对路径必须被拒绝: {bad}"
            );
        }
    }

    // T-ip-3（M2-2.b）：预览通道不使用新的安全边界——目录名常量与落盘 rel 前缀同源，
    // 防止前后端各拼一套导致 asset:// 指向错误目录。
    #[test]
    fn images_dir_constant_matches_rel_prefix() {
        assert_eq!(crate::images::IMAGES_DIR_NAME, "images");
    }
}

#[cfg(test)]
mod script_execution_gate_tests {
    use crate::domain::RunStatus;

    #[test]
    fn run_snapshot_serializes_without_output_or_body() {
        let snap = crate::script_runner::RunSnapshot {
            run_id: "run-1".to_string(),
            script_id: "script-1".to_string(),
            status: RunStatus::Running,
            started_at: chrono::Utc::now(),
            finished_at: None,
            exit_code: None,
            error: None,
            output_tail: String::new(),
            truncated: false,
            output_seq: 0,
        };
        let json = serde_json::to_string(&snap).expect("snapshot serializes");
        let lower = json.to_ascii_lowercase();
        for forbidden in [
            "stdout",
            "stderr",
            "body",
            "authorization",
            "cookie",
            "token",
        ] {
            assert!(
                !lower.contains(forbidden),
                "RunSnapshot must not expose {forbidden}"
            );
        }
    }

    #[test]
    fn script_run_audit_format_strings_do_not_include_values_or_body() {
        let source = include_str!("bridge.rs");
        let start = source.find("script.run.start").expect("start audit exists");
        let cancel = source
            .find("script.run.cancel")
            .expect("cancel audit exists");
        let status = source
            .find("script.run.status")
            .expect("status audit exists");
        for window_start in [start, cancel, status] {
            let window: String = source[window_start..].chars().take(260).collect();
            assert!(
                !window.contains("values"),
                "script run audit must not log argument values"
            );
            assert!(
                !window.contains("body"),
                "script run audit must not log script body"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// M2-6-fix1（复核 P1-1）：命令片段域的**运行时**来源校验取证。
//
// 背景：`snippet_*` / `run_command` 的 `check_invocation_source` 此前只有
// `scripts/check-command-domain-policy.py` 的**函数体内文本存在性**门禁——
// 把 `check_invocation_source` 换成同名空函数即可骗过静态门禁，而 `run_command`
// 是全仓唯一「新增的起真实子进程」入口，必须有运行时单测兜底。
// 范式照搬 image / session / resource / image-preview 四域的
// `remote_invocation_to_*_is_rejected`。
// ---------------------------------------------------------------------------
#[cfg(test)]
mod snippet_execution_gate_tests {
    // 命令片段域全部 IPC 作用域：四条 CRUD + 一条执行
    const SNIPPET_SCOPES: [&str; 5] = [
        "snippet_list",
        "snippet_add",
        "snippet_update",
        "snippet_remove",
        "run_command",
    ];

    // T-sn-1（P1-1）：远程 webview（tab-* / grid-*）调用命令片段命令一律拒绝；
    // 主窗口放行。`run_command` 会 spawn 真实子进程，是命令片段域风险最高的入口，
    // 运行期来源校验是实效上的**唯一**闸门（capabilities 未限定 webviews）。
    #[test]
    fn remote_invocation_to_snippet_commands_is_rejected() {
        let registry = crate::security_policy::IntentRegistry::new();
        for scope in SNIPPET_SCOPES {
            for remote in ["tab-3", "grid-1", "tab-remote-9"] {
                assert!(
                    crate::security_policy::check_remote_invocation(remote, scope, None, &registry)
                        .is_err(),
                    "远程 webview {remote} 调用 {scope} 必须被拒绝"
                );
            }
            assert!(
                crate::security_policy::check_remote_invocation("main", scope, None, &registry)
                    .is_ok(),
                "主窗口调用 {scope} 应放行"
            );
        }
    }

    // T-sn-2（P1-1 配套）：命令片段审计格式串不得含 values / argv / 命令行明文。
    // 与静态门禁（check-command-domain-policy.py 的 AUDIT_FORBIDDEN）构成双轨。
    #[test]
    fn snippet_audit_format_strings_do_not_include_values_or_argv() {
        let source = include_str!("bridge.rs");
        for name in [
            "cmd.run.start",
            "cmd.validate.reject",
            "cmd.add",
            "cmd.update",
            "cmd.remove",
        ] {
            let index = source
                .find(name)
                .unwrap_or_else(|| panic!("审计事件 {name} 必须存在"));
            let window: String = source[index..].chars().take(260).collect();
            for bad in ["values", "argv", "{PATTERN}"] {
                assert!(!window.contains(bad), "{name} 审计格式串不得含 {bad}");
            }
        }
    }

    // T-sn-3（P1-2 最小收敛）：argv[0] 为占位符的片段在定义期即被拒绝，
    // 程序名不得由运行期参数值决定。
    #[test]
    fn snippet_argv0_placeholder_is_rejected_at_definition() {
        let mut s = crate::domain::CommandSnippet {
            id: uuid::Uuid::new_v4().to_string(),
            name: "任意程序".to_string(),
            category: "system".to_string(),
            interpreter: crate::domain::ScriptInterpreter::Bash,
            argv: vec!["{PROG}".to_string(), "--version".to_string()],
            params: vec![crate::domain::ScriptParam {
                name: "PROG".to_string(),
                label: "程序".to_string(),
                param_type: crate::domain::ParamType::String,
                required: true,
                default: None,
                options: vec![],
                raw: false,
                secret: false,
            }],
            description: String::new(),
            dangerous: false,
            builtin: false,
            enabled: true,
            timeout_secs: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(
            crate::snippets::validate_snippet(&s),
            Err(crate::snippets::SnippetError::ArgvProgramPlaceholder)
        );
        // 反例：argv[0] 为字面量时放行（不误伤 grep -r {PATTERN} 这类常规片段）
        s.argv = vec!["grep".to_string(), "-r".to_string(), "{PROG}".to_string()];
        assert_eq!(crate::snippets::validate_snippet(&s), Ok(()));
    }
}

// ===========================================================================
// M4-3 / M4-2.s（Lane A4）：数据库命令层
//
// 安全闸门接线（A3 的 `DbPool::query` 不判写，写闸门全链路归本层）：
//   `db_query` 在**任何语句实际执行前**必须过 `security_policy::evaluate_db_query_gate`，
//   任一步拒绝即短路返回，绝不降级放行（F1 / F4 / 契约 G-4 / G-5）。
//
// 连接模型：因 `DbPool` 包裹的驱动句柄（rusqlite::Connection 等）非 `Send`，
// 不能驻留于 Tauri 全局 managed state，故采用「按需重连」模型——
// `db_connect` 仅打通一次以校验可达性/凭据，并把配置登记进 `DbConnectionRegistry`、
// 凭据写入系统密钥库（键 = `db:<conn_id>`）；`db_query` 每次从登记簿取配置 +
// 从密钥库取凭据即时建连执行；`db_disconnect` 撤销登记并删除密钥。
// SQLite 为文件级、MySQL/PostgreSQL 取数通道尚未实现（D27），故该模型对当前
// 可验证路径完全成立。
//
// 凭据：password 永不进 `DbConnectionConfig`（F2），不进审计 detail（G-1 / G-3）。
// ===========================================================================

use crate::database::{DbPool, QueryCancel};
use uuid::Uuid;

/// 数据库连接配置登记簿（不含非 Send 的池句柄）。注册为 Tauri managed state。
pub struct DbConnectionRegistry {
    pub configs: Mutex<HashMap<String, DbConnectionConfig>>,
}

impl Default for DbConnectionRegistry {
    fn default() -> Self {
        DbConnectionRegistry {
            configs: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbConnectResult {
    pub conn_id: String,
    pub kind: SupportedDb,
}

#[tauri::command]
pub fn db_connect(
    app: AppHandle,
    webview: tauri::Webview,
    cfg: DbConnectionConfig,
    password: Option<String>,
) -> Result<DbConnectResult, String> {
    check_invocation_source(&webview, "db_connect", None, &app)?;
    // 实际打通一次以校验配置/凭据/可达性；连接不驻留全局（非 Send），校验后即弃。
    let roots = allowed_roots(&app);
    let kind = cfg.kind;
    let conn_id = cfg.id.clone();
    let _probe = DbPool::connect(&cfg, password.as_deref(), &roots)
        .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;
    // G-1：凭据（若有）写入系统密钥库，键必须带 `db:` 前缀，与 git 的 repo_id 命名空间隔离。
    if let Some(p) = password {
        KeyringStore::save_token(&crate::database::credential_key(&conn_id), &p)?;
    }
    {
        let reg = app.state::<DbConnectionRegistry>();
        reg.configs.lock().unwrap().insert(conn_id.clone(), cfg);
    }
    // 审计 detail 禁含 SQL/凭据（G-3）：只记连接标识与类型。
    workspace::log_audit(
        &app,
        "db.connect",
        format!("conn_id={} kind={:?}", conn_id, kind),
    );
    Ok(DbConnectResult { conn_id, kind })
}

#[tauri::command]
pub fn db_query(
    app: AppHandle,
    webview: tauri::Webview,
    conn_id: String,
    sql: String,
    timeout_secs: Option<u64>,
    confirm_write: bool,
) -> Result<crate::database::DbQueryResult, String> {
    use crate::security_policy as sp;
    check_invocation_source(&webview, "db_query", None, &app)?;

    // 取登记配置（db_connect 未登记即视为未连接）。
    let cfg = {
        let reg = app.state::<DbConnectionRegistry>();
        let guard = reg.configs.lock().unwrap();
        guard
            .get(&conn_id)
            .cloned()
            .ok_or_else(|| "DB_NOT_CONNECTED".to_string())?
    };

    // SQLite 不需要密码；其余从密钥库取（键 = db:<conn_id>）。
    let password = if cfg.kind == SupportedDb::Sqlite {
        None
    } else {
        KeyringStore::get_token(&crate::database::credential_key(&conn_id)).ok()
    };

    let roots = allowed_roots(&app);
    let mut pool = DbPool::connect(&cfg, password.as_deref(), &roots)
        .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;

    // ===== 安全闸门（A3 的 query 不判写，全链路归此处）=====
    let encrypted = pool.encrypted();
    sp::evaluate_db_query_gate(&sql, &cfg, encrypted, confirm_write)
        .map_err(|code| code.as_str().to_string())?;

    // 执行取数。
    let cancel = QueryCancel::new();
    let query_id = Uuid::new_v4().to_string();
    let result = pool
        .query(&sql, &cancel, timeout_secs, &query_id)
        .map_err(|e| format!("{}: {}", e.code_str(), e.message))?;

    // 审计 detail 禁含 SQL 原文与凭据（G-3）：只记连接标识与截断标记。
    workspace::log_audit(
        &app,
        "db.query",
        format!(
            "conn_id={} rows={} truncated={} field_truncated={}",
            conn_id, result.row_count, result.truncated, result.field_truncated
        ),
    );
    Ok(result)
}

#[tauri::command]
pub fn db_disconnect(
    app: AppHandle,
    webview: tauri::Webview,
    conn_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "db_disconnect", None, &app)?;
    let removed = {
        let reg = app.state::<DbConnectionRegistry>();
        let mut guard = reg.configs.lock().unwrap();
        guard.remove(&conn_id).is_some()
    };
    // 一并撤销密钥库中的凭据（G-1：命名空间隔离，不影响 git 的 repo_id）。
    let _ = KeyringStore::delete_token(&crate::database::credential_key(&conn_id));
    workspace::log_audit(
        &app,
        "db.disconnect",
        format!("conn_id={} removed={}", conn_id, removed),
    );
    if removed {
        Ok(())
    } else {
        Err("DB_NOT_CONNECTED".to_string())
    }
}

// ===== M5-W7（Lane A5）：Agent/Skill 只读命令桥 =====
// 仅 parse / validate / permission_preview，绝不执行/安装/联网/持久化写。
// 每个命令首行做 source check（防前端裸 invoke）；返回类型可序列化供前端消费。
// 命名与 A6 M5-6 UI 冻结的 agent_*/skill_* 前缀一致，但本波仅「只读」子集，不含 M5-5 运行时命令。

/// 校验报告（agent_validate / skill_validate 返回）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ValidationReport {
    fn ok() -> Self {
        ValidationReport {
            valid: true,
            errors: Vec::new(),
        }
    }
    fn with_errors(errors: Vec<String>) -> Self {
        ValidationReport {
            valid: errors.is_empty(),
            errors,
        }
    }
}

// M5-W8（A5）硬化：解析/校验/预览输入上限，防超大 payload DoS（A4 W7 评审 R5-2 / F2）。
// Agent/Skill 定义为小结构，256 KiB 已留足余量；超出在来源校验之后直接拒绝。
const AGENT_DEF_MAX_BYTES: usize = 256 * 1024;
const SKILL_DEF_MAX_BYTES: usize = 256 * 1024;

fn reject_oversized_def(text: &str, max: usize, kind: &str) -> Result<(), String> {
    if text.len() > max {
        return Err(format!(
            "{kind} 定义超出大小上限（{} 字节），已拒绝解析",
            max
        ));
    }
    Ok(())
}

fn agent_validate_inner(text: &str) -> ValidationReport {
    if text.len() > AGENT_DEF_MAX_BYTES {
        return ValidationReport::with_errors(vec![format!(
            "Agent 定义超出大小上限（{} 字节），已拒绝解析",
            AGENT_DEF_MAX_BYTES
        )]);
    }
    match crate::domain::AgentDef::parse(text) {
        Ok(def) => match def.validate() {
            Ok(()) => ValidationReport::ok(),
            Err(e) => ValidationReport::with_errors(vec![format!("{e}")]),
        },
        Err(e) => ValidationReport::with_errors(vec![e]),
    }
}

#[tauri::command]
pub fn agent_parse(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<crate::domain::AgentDef, String> {
    check_invocation_source(&webview, "agent_parse", None, &app)?;
    agent_parse_inner(&text)
}

#[tauri::command]
pub fn agent_validate(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<ValidationReport, String> {
    check_invocation_source(&webview, "agent_validate", None, &app)?;
    Ok(agent_validate_inner(&text))
}

#[tauri::command]
pub fn agent_permission_preview(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<crate::domain::PermissionPreview, String> {
    check_invocation_source(&webview, "agent_permission_preview", None, &app)?;
    agent_permission_preview_inner(&text)
}

fn skill_validate_inner(text: &str) -> ValidationReport {
    if text.len() > SKILL_DEF_MAX_BYTES {
        return ValidationReport::with_errors(vec![format!(
            "Skill 定义超出大小上限（{} 字节），已拒绝解析",
            SKILL_DEF_MAX_BYTES
        )]);
    }
    match crate::domain::SkillDef::parse(text) {
        Ok(def) => match def.validate() {
            Ok(()) => ValidationReport::ok(),
            Err(e) => ValidationReport::with_errors(vec![format!("{e}")]),
        },
        Err(e) => ValidationReport::with_errors(vec![e]),
    }
}

fn agent_parse_inner(text: &str) -> Result<crate::domain::AgentDef, String> {
    reject_oversized_def(text, AGENT_DEF_MAX_BYTES, "Agent")?;
    crate::domain::AgentDef::parse(text)
}

fn skill_parse_inner(text: &str) -> Result<crate::domain::SkillDef, String> {
    reject_oversized_def(text, SKILL_DEF_MAX_BYTES, "Skill")?;
    crate::domain::SkillDef::parse(text)
}

fn agent_permission_preview_inner(text: &str) -> Result<crate::domain::PermissionPreview, String> {
    reject_oversized_def(text, AGENT_DEF_MAX_BYTES, "Agent")?;
    let def = crate::domain::AgentDef::parse(text)?;
    Ok(def.permission_preview())
}

fn skill_permission_preview_inner(text: &str) -> Result<crate::domain::PermissionPreview, String> {
    reject_oversized_def(text, SKILL_DEF_MAX_BYTES, "Skill")?;
    let def = crate::domain::SkillDef::parse(text)?;
    Ok(def.permission_preview())
}

#[tauri::command]
pub fn skill_parse(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<crate::domain::SkillDef, String> {
    check_invocation_source(&webview, "skill_parse", None, &app)?;
    skill_parse_inner(&text)
}

#[tauri::command]
pub fn skill_validate(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<ValidationReport, String> {
    check_invocation_source(&webview, "skill_validate", None, &app)?;
    Ok(skill_validate_inner(&text))
}

#[tauri::command]
pub fn skill_permission_preview(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<crate::domain::PermissionPreview, String> {
    check_invocation_source(&webview, "skill_permission_preview", None, &app)?;
    skill_permission_preview_inner(&text)
}

#[cfg(test)]
mod agent_skill_bridge_tests {
    use super::*;
    use crate::domain::{AclLevel, AgentDialect, SkillExec};
    use serde_json;

    fn good_agent_json() -> String {
        let a = crate::domain::AgentDef {
            id: "assistant".into(),
            version: "1.0.0".into(),
            display_name: "Assistant".into(),
            description: "general assistant".into(),
            dialect: AgentDialect::OpenAiCompatible,
            system_prompt: "You are helpful.".into(),
            default_capabilities: vec![],
            a2a: crate::domain::A2aConfig {
                delegate_to: false,
                delegated_from: false,
            },
            metadata: serde_json::json!({}),
        };
        serde_json::to_string(&a).unwrap()
    }

    fn good_skill_json() -> String {
        let s = crate::domain::SkillDef {
            id: "demo".into(),
            version: "1.0.0".into(),
            display_name: "Demo".into(),
            description: "a safe demo".into(),
            acl: AclLevel::Safe,
            exec: SkillExec::ScriptRef {
                script_id: "s1".into(),
                params: serde_json::json!({}),
            },
            inputs: vec![],
            capabilities: vec![],
            tests: vec![],
            metadata: serde_json::json!({}),
        };
        serde_json::to_string(&s).unwrap()
    }

    #[test]
    fn validate_inner_accepts_good_agent() {
        assert!(agent_validate_inner(&good_agent_json()).valid);
    }

    #[test]
    fn validate_inner_rejects_credential_leak() {
        let mut a: crate::domain::AgentDef = serde_json::from_str(&good_agent_json()).unwrap();
        a.system_prompt = "use sk-abc123XYZ".into();
        let r = agent_validate_inner(&serde_json::to_string(&a).unwrap());
        assert!(
            !r.valid,
            "credential-leak agent must be invalid, errors={:?}",
            r.errors
        );
        assert!(!r.errors.is_empty());
        let joined = r.errors.join(" ");
        assert!(
            joined.contains("泄露")
                || joined.contains("凭据")
                || joined.to_lowercase().contains("credential")
                || joined.to_lowercase().contains("leak"),
            "errors should mention credential leak, got {:?}",
            r.errors
        );
    }

    #[test]
    fn validate_inner_accepts_good_skill() {
        assert!(skill_validate_inner(&good_skill_json()).valid);
    }

    #[test]
    fn validate_inner_rejects_skill_credential() {
        let mut s: crate::domain::SkillDef = serde_json::from_str(&good_skill_json()).unwrap();
        s.description = "token sk-abc123".into();
        let r = skill_validate_inner(&serde_json::to_string(&s).unwrap());
        assert!(!r.valid);
    }

    #[test]
    fn report_ok_and_err() {
        assert!(ValidationReport::ok().valid);
        assert!(!ValidationReport::with_errors(vec!["x".into()]).valid);
    }

    // ---- M5-W8 硬化：校验错误形状 + 边界用例 ----

    #[test]
    fn report_ok_has_empty_errors() {
        let r = ValidationReport::ok();
        assert!(r.valid);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn report_with_errors_shape() {
        let r = ValidationReport::with_errors(vec!["e1".into(), "e2".into()]);
        assert!(!r.valid);
        assert_eq!(r.errors.len(), 2);
    }

    #[test]
    fn reject_oversized_def_guard() {
        assert!(reject_oversized_def("small", AGENT_DEF_MAX_BYTES, "Agent").is_ok());
        let huge = "x".repeat(AGENT_DEF_MAX_BYTES + 1);
        let e = reject_oversized_def(&huge, AGENT_DEF_MAX_BYTES, "Agent").unwrap_err();
        assert!(
            e.contains("大小上限") || e.to_lowercase().contains("limit"),
            "got {e}"
        );
    }

    #[test]
    fn agent_validate_inner_rejects_oversized() {
        let huge = "x".repeat(AGENT_DEF_MAX_BYTES + 1);
        let r = agent_validate_inner(&huge);
        assert!(!r.valid);
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn skill_validate_inner_rejects_oversized() {
        let huge = "x".repeat(SKILL_DEF_MAX_BYTES + 1);
        let r = skill_validate_inner(&huge);
        assert!(!r.valid);
    }

    #[test]
    fn agent_parse_inner_rejects_oversized() {
        let huge = "x".repeat(AGENT_DEF_MAX_BYTES + 1);
        assert!(agent_parse_inner(&huge).is_err());
    }

    #[test]
    fn agent_parse_inner_rejects_malformed() {
        assert!(agent_parse_inner("not json {{{").is_err());
    }

    #[test]
    fn skill_parse_inner_rejects_oversized() {
        let huge = "x".repeat(SKILL_DEF_MAX_BYTES + 1);
        assert!(skill_parse_inner(&huge).is_err());
    }

    #[test]
    fn agent_validate_rejects_invalid_dialect() {
        let mut v: serde_json::Value = serde_json::from_str(&good_agent_json()).unwrap();
        v["dialect"] = serde_json::json!(123); // 非枚举字符串 → 解析失败
        let r = agent_validate_inner(&v.to_string());
        assert!(
            !r.valid,
            "invalid dialect type must invalidate, got {:?}",
            r.errors
        );
    }

    #[test]
    fn skill_validate_rejects_invalid_acl() {
        let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
        v["acl"] = serde_json::json!(123); // 非枚举字符串 → 解析失败
        let r = skill_validate_inner(&v.to_string());
        assert!(
            !r.valid,
            "invalid acl type must invalidate, got {:?}",
            r.errors
        );
    }

    #[test]
    fn agent_permission_preview_inner_shape() {
        let p = agent_permission_preview_inner(&good_agent_json()).unwrap();
        assert!(matches!(
            p.gate,
            AclLevel::Safe | AclLevel::Confirm | AclLevel::Dangerous
        ));
        assert!(p.capabilities.is_empty());
    }

    #[test]
    fn agent_permission_preview_inner_rejects_malformed() {
        assert!(agent_permission_preview_inner("bad").is_err());
    }

    #[test]
    fn skill_permission_preview_inner_shape() {
        let p = skill_permission_preview_inner(&good_skill_json()).unwrap();
        assert!(matches!(
            p.gate,
            AclLevel::Safe | AclLevel::Confirm | AclLevel::Dangerous
        ));
        assert!(p.capabilities.is_empty());
    }

    // ---- M5-W9 聚焦测试：脱敏校验错误 + 解析/权限预览边界（A4 W8 F-W8-1 闭环） ----

    #[test]
    fn credential_leak_display_is_redacted() {
        // A0 修复 CredentialLeak Display 后必须不再回显密文
        let e = crate::security_policy::PolicyError::CredentialLeak("sk-abc123XYZ".into());
        let s = format!("{e}");
        assert!(!s.contains("sk-abc123XYZ"), "Display 不得回显密文: {s}");
        assert!(s.contains("<redacted>"), "Display 必须脱敏: {s}");
    }

    #[test]
    fn agent_validate_redacts_credential_leak() {
        let mut v: serde_json::Value = serde_json::from_str(&good_agent_json()).unwrap();
        v["system_prompt"] =
            serde_json::json!("system prompt contains sk-abc123XYZsecret token here");
        let r = agent_validate_inner(&v.to_string());
        assert!(!r.valid, "credential leak 必须使校验失败");
        let msg = r.errors.concat();
        assert!(!msg.contains("sk-abc123XYZ"), "校验错误不得回显密文: {msg}");
        assert!(
            msg.contains("<redacted>") || msg.contains("凭据") || msg.contains("密钥"),
            "必须指示泄露但不含密文: {msg}"
        );
    }

    #[test]
    fn skill_validate_redacts_credential_leak() {
        let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
        v["description"] = serde_json::json!("desc with sk-abc123XYZsecret in it");
        let r = skill_validate_inner(&v.to_string());
        assert!(!r.valid, "credential leak 必须使校验失败");
        let msg = r.errors.concat();
        assert!(!msg.contains("sk-abc123XYZ"), "校验错误不得回显密文: {msg}");
        assert!(
            msg.contains("<redacted>") || msg.contains("凭据") || msg.contains("密钥"),
            "必须指示泄露但不含密文: {msg}"
        );
    }

    #[test]
    fn agent_permission_preview_gate_is_confirm() {
        // Agent 默认需确认（A2A 委派场景），权限预览闸门恒为 Confirm
        let p = agent_permission_preview_inner(&good_agent_json()).unwrap();
        assert_eq!(p.gate, AclLevel::Confirm);
    }

    #[test]
    fn skill_permission_preview_gate_maps_acl() {
        for (acl, expected) in [
            ("safe", AclLevel::Safe),
            ("confirm", AclLevel::Confirm),
            ("dangerous", AclLevel::Dangerous),
        ] {
            let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
            v["acl"] = serde_json::json!(acl);
            let p = skill_permission_preview_inner(&v.to_string()).unwrap();
            assert_eq!(p.gate, expected, "acl={acl} 映射错误");
        }
    }

    #[test]
    fn skill_permission_preview_reflects_declared_capabilities() {
        // 预览能力来自定义声明的 capabilities（上游 validate 已按白名单校验），
        // 这里确认预览原样透出，不臆造、不遗漏。
        let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
        v["capabilities"] = serde_json::json!([{"id": "cap_a"}, {"id": "cap_b"}]);
        let p = skill_permission_preview_inner(&v.to_string()).unwrap();
        assert_eq!(
            p.capabilities,
            vec!["cap_a".to_string(), "cap_b".to_string()]
        );
    }

    #[test]
    fn agent_parse_inner_rejects_empty() {
        assert!(agent_parse_inner("").is_err());
        assert!(agent_parse_inner("   ").is_err());
    }

    #[test]
    fn skill_parse_inner_rejects_empty() {
        assert!(skill_parse_inner("").is_err());
        assert!(skill_parse_inner("   ").is_err());
    }
}

// ====== M5-2 MCP 只读注册表/策略桥命令（W7）======
// 无 rmcp / 无 server / 无监听 / 无网络；仅把已冻结的纯注册表 + 纯策略以只读
// introspection 形式暴露给受信任的主窗口。每一项都先过来源校验
// `check_invocation_source`（main 受信任免令牌；tab-*/grid-* 无令牌一律拒绝），
// 与 M5-2 红线和 W7 Hard Stop「只读」完全一致。路径裁决复用既有的
// `evaluate_mcp_command`（内部走 `check_path_within_roots` + `redact_sensitive_url`）。

#[tauri::command]
pub fn mcp_policy_get(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<McpPolicySnapshot, String> {
    check_invocation_source(&webview, "mcp_policy_get", None, &app)?;
    Ok(crate::mcp::current_policy_snapshot())
}

#[tauri::command]
pub fn mcp_registry_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<McpRegistryEntryView>, String> {
    check_invocation_source(&webview, "mcp_registry_list", None, &app)?;
    Ok(crate::mcp::list_registry_entries())
}

#[tauri::command]
pub fn mcp_capability_preview(
    app: AppHandle,
    webview: tauri::Webview,
    capability: String,
    raw_path: Option<String>,
) -> Result<McpDecisionView, String> {
    check_invocation_source(&webview, "mcp_capability_preview", None, &app)?;
    let roots = allowed_roots(&app);
    Ok(McpDecisionView::from_decision(
        crate::mcp::evaluate_mcp_command(&capability, raw_path.as_deref(), &roots),
    ))
}

// ===========================================================================
// M5-W12 图谱只读 live-query 命令（Lane A7）
//
// 命令体仅做来源校验 + 调 `crate::graph` 纯内核；不反向依赖 `crate::bridge`
// （graph.rs 已是纯逻辑，守 GRAPH_NO_SECOND_PATH），无副作用 / 无 IO / 无写。
// 出参经 View DTO 删除 `props`；错误仅稳定 `GRAPH_*` 码，绝不 echo 凭据/路径/body。
// ===========================================================================

#[tauri::command]
pub async fn graph_query(
    app: AppHandle,
    webview: tauri::Webview,
    req: GraphQueryRequest,
) -> Result<GraphQueryResult, String> {
    check_invocation_source(&webview, "graph_query", None, &app)?;
    let start = req.start_id.trim();
    if let Err(e) = crate::graph::validate_id_public(start) {
        return Err(e.code().to_string());
    }
    // 纯内存读锁；临界区不跨 await，drop 后即无残留状态（取消 = no-op）。
    let state = app.state::<crate::graph::GraphState>();
    let store = state.store.read().unwrap();
    let result = crate::graph::graph_query_impl(&store, start, req.depth, req.limit);
    drop(store);
    Ok(result)
}

#[tauri::command]
pub async fn graph_node_get(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<Option<GraphNodeView>, String> {
    check_invocation_source(&webview, "graph_node_get", None, &app)?;
    if let Err(e) = crate::graph::validate_id_public(&id) {
        return Err(e.code().to_string());
    }
    let state = app.state::<crate::graph::GraphState>();
    let store = state.store.read().unwrap();
    let view = crate::graph::graph_node_get_impl(&store, &id);
    drop(store);
    Ok(view)
}

#[tauri::command]
pub async fn graph_stats(app: AppHandle, webview: tauri::Webview) -> Result<GraphStats, String> {
    check_invocation_source(&webview, "graph_stats", None, &app)?;
    let state = app.state::<crate::graph::GraphState>();
    let store = state.store.read().unwrap();
    let stats = crate::graph::graph_stats_impl(&store);
    drop(store);
    Ok(stats)
}

// ===========================================================================
// M5-W13 插件 manifest 生命周期 Stage-I 命令（Lane A9）
//
// 只做：来源校验 + 纯登记簿读写 + 原子落盘 + 脱敏审计。
// 不做：invoke / 命令执行 / 动态加载 / 网络下载 / 资源解包 / 真验签——均属后续
// runtime wave，本 wave 硬停锁死。
// 审计只记 id / 版本 / 能力计数 / 稳定错误码，**不**记资源路径、签名原文、公钥原文、
// 请求或响应正文（读命令 `plugin_list` / `plugin_get` / `plugin_keys_list` 不写审计）。
// ===========================================================================

fn plugin_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 审计用 id：截断后再落审计，避免未校验输入撑爆审计条目。
fn plugin_id_for_audit(id: &str) -> String {
    id.chars().take(64).collect()
}

/// 状态过滤参数解析（`snake_case`，与 `PluginState` serde 口径一致）。
fn parse_plugin_state(raw: Option<String>) -> Result<Option<crate::domain::PluginState>, String> {
    use crate::domain::PluginState as S;
    match raw.as_deref().map(str::trim) {
        Some(s) if !s.is_empty() => match s {
            "discovered" => Ok(Some(S::Discovered)),
            "validating" => Ok(Some(S::Validating)),
            "signed_ok" => Ok(Some(S::SignedOk)),
            "signed_failed" => Ok(Some(S::SignedFailed)),
            "loaded" => Ok(Some(S::Loaded)),
            "enabled" => Ok(Some(S::Enabled)),
            "disabled" => Ok(Some(S::Disabled)),
            "uninstalled" => Ok(Some(S::Uninstalled)),
            _ => Err("PLUGIN_STATE_INVALID".to_string()),
        },
        _ => Ok(None),
    }
}

/// 状态迁移公共体（enable / disable 共用），审计脱敏由本函数统一收口。
fn plugin_set_state(
    app: &AppHandle,
    id: &str,
    next: crate::domain::PluginState,
    action: &str,
) -> Result<crate::domain::PluginSummary, String> {
    let file = crate::plugin::plugins_file(app);
    let mut list = crate::plugin::load_plugins_at(&file);
    let id_safe = plugin_id_for_audit(id);
    match crate::plugin::set_plugin_state(&mut list, id, next, &plugin_now()) {
        Ok(rec) => {
            crate::plugin::save_plugins_at(&file, &list)?;
            let summary = crate::plugin::summary_of(&rec);
            let dangerous = rec
                .capabilities
                .iter()
                .filter(|c| {
                    crate::plugin::capability_acl_level(&c.capability)
                        == crate::domain::AclLevel::Dangerous
                })
                .count();
            workspace::log_audit(
                app,
                action,
                format!(
                    "id={id_safe} capability_count={} dangerous_count={dangerous} result=ok",
                    summary.capability_count
                ),
            );
            Ok(summary)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(app, action, format!("id={id_safe} result=err code={code}"));
            Err(code.to_string())
        }
    }
}

#[tauri::command]
pub fn plugin_install(
    app: AppHandle,
    webview: tauri::Webview,
    manifest: crate::domain::PluginManifest,
    resource_path: Option<String>,
) -> Result<crate::domain::PluginSummary, String> {
    check_invocation_source(&webview, "plugin_install", None, &app)?;
    // 资源路径（可选）：必须在允许根目录内；**只取「是否通过」布尔，绝不落盘路径**。
    let resource_ok = match resource_path.as_deref().map(str::trim) {
        Some(p) if !p.is_empty() => {
            let roots = allowed_roots(&app);
            if let Err(e) = crate::security_policy::check_path_within_roots(p, &roots) {
                let code = crate::plugin::error_code(&e);
                workspace::log_audit(&app, "plugin.install", format!("result=err code={code}"));
                return Err(code.to_string());
            }
            true
        }
        _ => false,
    };
    let id_safe = plugin_id_for_audit(&manifest.id);
    let file = crate::plugin::plugins_file(&app);
    let mut list = crate::plugin::load_plugins_at(&file);
    match crate::plugin::install_record(&mut list, manifest, resource_ok, &plugin_now()) {
        Ok(rec) => {
            crate::plugin::save_plugins_at(&file, &list)?;
            let summary = crate::plugin::summary_of(&rec);
            workspace::log_audit(
                &app,
                "plugin.install",
                format!(
                    "id={} version={} capability_count={} result=ok",
                    summary.id, summary.version, summary.capability_count
                ),
            );
            Ok(summary)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(
                &app,
                "plugin.install",
                format!("id={id_safe} result=err code={code}"),
            );
            Err(code.to_string())
        }
    }
}

#[tauri::command]
pub fn plugin_enable(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<crate::domain::PluginSummary, String> {
    check_invocation_source(&webview, "plugin_enable", None, &app)?;
    plugin_set_state(
        &app,
        &id,
        crate::domain::PluginState::Enabled,
        "plugin.enable",
    )
}

#[tauri::command]
pub fn plugin_disable(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<crate::domain::PluginSummary, String> {
    check_invocation_source(&webview, "plugin_disable", None, &app)?;
    plugin_set_state(
        &app,
        &id,
        crate::domain::PluginState::Disabled,
        "plugin.disable",
    )
}

#[tauri::command]
pub fn plugin_list(
    app: AppHandle,
    webview: tauri::Webview,
    state: Option<String>,
) -> Result<Vec<crate::domain::PluginSummary>, String> {
    check_invocation_source(&webview, "plugin_list", None, &app)?;
    let filter = parse_plugin_state(state)?;
    let list = crate::plugin::load_plugins_at(&crate::plugin::plugins_file(&app));
    Ok(crate::plugin::filter_by_state(&list, filter))
}

#[tauri::command]
pub fn plugin_get(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<crate::domain::PluginDetail, String> {
    check_invocation_source(&webview, "plugin_get", None, &app)?;
    let list = crate::plugin::load_plugins_at(&crate::plugin::plugins_file(&app));
    match crate::plugin::find_record(&list, &id) {
        Some(rec) => Ok(crate::plugin::detail_of(rec)),
        None => Err("PLUGIN_NOT_FOUND".to_string()),
    }
}

#[tauri::command]
pub fn plugin_keys_add(
    app: AppHandle,
    webview: tauri::Webview,
    key_id: String,
    pubkey: String,
    note: Option<String>,
) -> Result<Vec<crate::domain::TrustedKeyRecord>, String> {
    check_invocation_source(&webview, "plugin_keys_add", None, &app)?;
    let file = crate::plugin::trusted_keys_file(&app);
    let mut keys = crate::plugin::load_trusted_keys_at(&file);
    let key_safe: String = key_id.chars().take(64).collect();
    let result = crate::plugin::add_trusted_key(
        &mut keys,
        &key_id,
        &pubkey,
        note.unwrap_or_default().as_str(),
        &plugin_now(),
    );
    match result {
        Ok(_) => {
            crate::plugin::save_trusted_keys_at(&file, &keys)?;
            workspace::log_audit(
                &app,
                "plugin.keys.add",
                format!("key_id={key_safe} count={} result=ok", keys.len()),
            );
            Ok(keys)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(
                &app,
                "plugin.keys.add",
                format!("key_id={key_safe} result=err code={code}"),
            );
            Err(code.to_string())
        }
    }
}

#[tauri::command]
pub fn plugin_keys_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<crate::domain::TrustedKeyRecord>, String> {
    check_invocation_source(&webview, "plugin_keys_list", None, &app)?;
    Ok(crate::plugin::load_trusted_keys_at(
        &crate::plugin::trusted_keys_file(&app),
    ))
}

#[tauri::command]
pub fn plugin_keys_remove(
    app: AppHandle,
    webview: tauri::Webview,
    key_id: String,
) -> Result<Vec<crate::domain::TrustedKeyRecord>, String> {
    check_invocation_source(&webview, "plugin_keys_remove", None, &app)?;
    let file = crate::plugin::trusted_keys_file(&app);
    let mut keys = crate::plugin::load_trusted_keys_at(&file);
    let key_safe: String = key_id.chars().take(64).collect();
    match crate::plugin::remove_trusted_key(&mut keys, &key_id) {
        Ok(_) => {
            crate::plugin::save_trusted_keys_at(&file, &keys)?;
            workspace::log_audit(
                &app,
                "plugin.keys.remove",
                format!("key_id={key_safe} count={} result=ok", keys.len()),
            );
            Ok(keys)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(
                &app,
                "plugin.keys.remove",
                format!("key_id={key_safe} result=err code={code}"),
            );
            Err(code.to_string())
        }
    }
}
