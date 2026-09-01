use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};
use url::Url;

use crate::domain::*;
use crate::grid_ipc::GridCmd;
use crate::keyring_store::KeyringStore;
use crate::sync;
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
}

/// 终端会话：持有 PTY 写入端与子进程，读取在后台线程进行。
pub struct TerminalSession {
    pub writer: Box<dyn std::io::Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

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
                if let Err(error) = session.child.kill() {
                    errors.push(format!("{id} kill: {error}"));
                }
                if let Err(error) = session.child.wait() {
                    errors.push(format!("{id} wait: {error}"));
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

    Ok(())
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

/// M1-2: 新增或更新一条收藏。同 URL 视为更新。
#[tauri::command]
pub fn add_bookmark(
    app: AppHandle,
    url: String,
    title: String,
    category: String,
) -> Result<Bookmark, String> {
    let bm = workspace::add_bookmark(&app, url.clone(), title.clone(), category.clone())?;
    workspace::log_audit(
        &app,
        "bookmark.add",
        format!("{} -> {}", url, bm.title),
    );
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
        t.url = target;
    }
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

#[derive(serde::Serialize)]
pub struct TermInfo {
    pub id: String,
}

/// 创建并启动一个 PTY 终端（bash），后台线程读取输出并通过 `term-data` 事件推给前端。
#[tauri::command]
pub fn term_spawn(app: AppHandle) -> Result<TermInfo, String> {
    use portable_pty::{native_pty_system, PtySize};

    let id = format!("term-{}", uuid::Uuid::new_v4());
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("无法创建 PTY: {e}"))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = portable_pty::CommandBuilder::new(shell);
    // xterm.js 是完整终端模拟器，需要正常 TERM 与 ANSI 序列，不能过滤
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("无法启动 shell: {e}"))?;
    drop(pair.slave);

    let mut writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("writer: {error}"));
        }
    };
    // 触发初始提示符
    let _ = writer.write_all(b"\n");

    let reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("reader: {error}"));
        }
    };
    let app2 = app.clone();
    let tid = id.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 4096];
        let mut reader = reader;
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    // xterm.js 自己解析 ANSI 序列，不再过滤
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    if !data.is_empty() {
                        let _ =
                            app2.emit("term-data", serde_json::json!({ "id": tid, "data": data }));
                    }
                }
                Err(_) => break,
            }
        }
        let _ = app2.emit(
            "term-data",
            serde_json::json!({ "id": tid, "data": "\r\n[终端已退出]\r\n" }),
        );
    });

    let session = TerminalSession {
        writer: Box::new(writer),
        child: child,
    };
    app.state::<AppState>()
        .terminals
        .lock()
        .unwrap()
        .insert(id.clone(), session);
    Ok(TermInfo { id })
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

/// 调整终端大小（列/行）。
#[tauri::command]
pub fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let _ = (app, id, cols, rows);
    Ok(())
}

/// 关闭终端。
#[tauri::command]
pub fn term_kill(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    if let Some(mut s) = terms.remove(&id) {
        s.child.kill().map_err(|e| format!("终端关闭失败: {e}"))?;
        s.child.wait().map_err(|e| format!("终端等待失败: {e}"))?;
    }
    Ok(())
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

/// ready 信号（契约 §6.1）：主前端完成 mount + 2×rAF 后调用本命令，作为
/// 「一次轻量 IPC 往返」的终点；后端校验 run_id 后将带 run_id 的 ready 信号
/// 原子写入 ready_file。返回 run_id（非测量运行返回空串）。
#[tauri::command]
pub fn m0_ready(app: AppHandle) -> Result<String, String> {
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
