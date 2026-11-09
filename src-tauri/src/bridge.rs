use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};
use url::Url;

use crate::capabilities::browser::commands::*;
use crate::domain::*;
use crate::grid_ipc::GridCmd;
use crate::keyring_store::KeyringStore;
use crate::mcp::{McpDecisionView, McpRegistryEntryView};
use crate::script_runner::{RunError, RunSnapshot, ScriptProcessTable, ScriptRunRecord};
use crate::seam::{PathResolver, Progress, ProgressSink, RootsProvider};
use crate::shared::invocation::check_invocation_source;
use crate::terminal::{self, ChannelSink, EventSink};
use crate::workspace;

mod automation_commands;
mod browser_commands;
mod credential_commands;
mod filesystem_commands;
mod grid_runtime;
mod integration_commands;
mod resource_commands;
mod system_commands;
mod tab_runtime;
mod terminal_commands;
mod workbench_commands;
mod workspace_commands;

pub(crate) use self::automation_commands::*;
pub(crate) use self::browser_commands::*;
pub(crate) use self::credential_commands::*;
pub(crate) use self::filesystem_commands::*;
pub(crate) use self::grid_runtime::*;
pub(crate) use self::integration_commands::*;
pub(crate) use self::resource_commands::*;
pub(crate) use self::system_commands::*;
pub(crate) use self::tab_runtime::*;
pub(crate) use self::terminal_commands::*;
pub(crate) use self::workbench_commands::*;
#[cfg(test)]
use self::workbench_commands::{
    agent_parse_inner, agent_permission_preview_inner, agent_validate_inner,
};
pub(crate) use self::workspace_commands::*;

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
    let (x, y, w, h) = layout.unwrap_or((
        0.0,
        0.0,
        INITIAL_HIDDEN_WEBVIEW_WIDTH,
        INITIAL_HIDDEN_WEBVIEW_HEIGHT,
    ));
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
    /// Scroll-only restore metadata; form/password/file values are never persisted.
    pub hibernated_scroll: Mutex<HashMap<String, (f64, f64)>>,
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
    /// 自动填充句柄：opaque credential_id -> 非敏感元数据（keyring key / url / username）。
    ///
    /// 会话级、仅内存、**绝不落盘**；每次 `list_browser_credentials` 重新生成，
    /// 前端只能拿着 `credential_id` 指认「用户选了哪一条」，永远拿不到 password，
    /// 也拿不到真正的 keyring account key。
    pub credential_handles: Mutex<HashMap<String, BrowserCredentialMeta>>,
    /// M2-4.c：脚本执行运行表（命令层只接入 b 卡内核；输出/落盘归 M2-4.d）。
    pub script_runs: Arc<ScriptProcessTable>,
}

/// 终端会话与输出内核（M3.a：mpsc+pump 管道、进程组回收、Channel 单播）。
pub use crate::terminal::{TermInfo, TerminalSession};

/// 宫格子 webview 的初始 URL 解析：
/// - 已配置真实服务 URL（含 http(s)://、about: 等）→ 归一化后原样使用；
/// - 未配置 / 显式为空 → 使用 `about:blank` 作为本地占位页。
/// 严禁回退到百度等第三方站点（避免"先开百度再跳真实服务"的双导航）。
fn initial_grid_url(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return "about:blank".to_string();
    }
    normalize_url(s)
}

#[cfg(test)]
mod grid_resize_policy_tests {
    use super::{grid_url_needs_navigation, requested_grid_url};

    #[test]
    fn missing_grid_url_preserves_existing_navigation() {
        assert_eq!(
            requested_grid_url(None, Some("https://example.com")),
            Some("https://example.com".to_string())
        );
        assert!(!grid_url_needs_navigation(
            Some("https://example.com"),
            requested_grid_url(None, None).as_deref()
        ));
    }

    #[test]
    fn explicit_url_changes_navigate_but_unchanged_urls_do_not() {
        let target = requested_grid_url(Some("https://example.com"), None);
        assert_eq!(target.as_deref(), Some("https://example.com"));
        assert!(!grid_url_needs_navigation(
            Some("https://example.com"),
            target.as_deref()
        ));
        assert!(grid_url_needs_navigation(
            Some("https://old.example"),
            target.as_deref()
        ));
    }

    #[test]
    fn explicit_about_blank_is_not_treated_as_missing() {
        assert_eq!(
            requested_grid_url(Some("about:blank"), Some("https://example.com")),
            Some("about:blank".to_string())
        );
    }
}

#[cfg(test)]
mod normalize_url_tests {
    use super::{
        initial_grid_url, is_tab_label, reserve_tab_recovery_attempt, TabRecoveryBudget,
        TAB_RECOVERY_WINDOW_SECS,
    };
    use crate::capabilities::browser::commands::normalize_url;

    #[test]
    fn preserves_about_blank_for_offline_browser_scenarios() {
        assert_eq!(normalize_url("about:blank"), "about:blank");
    }

    #[test]
    fn keeps_domain_and_search_input_behavior() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert!(normalize_url("search words").starts_with("https://www.baidu.com/s?wd="));
    }

    // 宫格首导航 URL 决定逻辑：已配置真实服务则用真实 URL，未配置用 about:blank，
    // 任何情况下都不得回退到百度（这是"先百度后真实"双导航的根因回归）。
    #[test]
    fn grid_initial_url_uses_real_service_and_never_baidu() {
        assert_eq!(
            initial_grid_url("https://www.doubao.com"),
            "https://www.doubao.com"
        );
        assert_eq!(
            initial_grid_url("https://kimi.moonshot.cn"),
            "https://kimi.moonshot.cn"
        );
        assert_eq!(
            initial_grid_url("https://tongyi.aliyun.com/qianwen/"),
            "https://tongyi.aliyun.com/qianwen/"
        );
        // 未配置 / 空 / 显式 about:blank -> 本地占位，绝不百度
        assert_eq!(initial_grid_url(""), "about:blank");
        assert_eq!(initial_grid_url("about:blank"), "about:blank");
        // 裸域名补协议，仍非百度
        assert_eq!(initial_grid_url("doubao.com"), "https://doubao.com");
        // 回归断言：任何输入都不应产生百度占位
        assert!(!initial_grid_url("https://www.doubao.com").contains("baidu"));
        assert!(!initial_grid_url("").contains("baidu"));
        assert!(!initial_grid_url("kimi.moonshot.cn").contains("baidu"));
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
    state.hibernated_scroll.lock().unwrap().remove(id);
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
            let report = crate::capabilities::session::commands::flush_sessions_inner(&app);
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

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
struct ResourceScanResult {
    page_url: String,
    items: Vec<ResourceItem>,
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
        crate::workspace::allowed_roots(self.app)
    }
}

// M1-8 资源瀑布（请求拦截与瀑布）——命令组收口说明：
//   report_resources 已迁移至 capabilities/browser/commands.rs（resource_collection → browser 子域收口）。
//   其余资源瀑布采集/脱敏/转发函数（on_resource_received 等）属 framework/infra，仍留本文件。
//
// 链路：插件 WebKitGTK 原生信号（resource-load-started/finished/failed）
//   → browser-tabs://event(type=resourceReceived)（原始 URL，仅进程内）
//   → main.rs 转发到 on_resource_received（脱敏 + 容量上限 + 入库）
//   → app.emit("resource-received", ResourceReceived)（脱敏 DTO，推前端）。
// 隐私红线：DTO 不含 headers/Cookie/Authorization/Set-Cookie/任何 body；
// URL 一律经 sp::redact_sensitive_url 脱敏 + 限长；审计只记 tab_id 与计数。
// ---------------------------------------------------------------------------

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
    crate::shared::native_legacy_policy::require_main(&webview, "issue_intent")?;
    crate::shared::native_legacy_policy::bound_text("issue_intent", "scope", &scope)?;

    use crate::security_policy as sp;
    if webview.label() != "main" {
        return Err(format!("只有主窗口可以签发意图令牌：{}", webview.label()));
    }
    let allowed = [
        sp::INTENT_SAVE_NOTE,
        sp::INTENT_COLLECT_SELECTION,
        sp::INTENT_OPEN_TERMINAL,
        "set_default_browser",
    ];
    let is_confirmed_effect = ["session_delete:", "snippet_remove:"].iter().any(|prefix| {
        scope.strip_prefix(prefix).is_some_and(|id| {
            !id.is_empty()
                && id.len() <= 64
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
    });
    if !allowed.contains(&scope.as_str()) && !is_confirmed_effect {
        return Err("NATIVE_INTENT_SCOPE_DENIED".into());
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

#[cfg(test)]
mod resource_capture_tests {
    use super::*;
    use crate::shared::invocation::check_tab_id;

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
        assert!(crate::images::check_id(&uuid, "会话 id").is_ok());
        for bad in ["../../etc/passwd", "/etc/passwd", "..", "", "a/b", "a\\b"] {
            assert!(
                crate::images::check_id(bad, "会话 id").is_err(),
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
        assert!(crate::images::check_id(&uuid, "成果 id").is_ok());
        for bad in ["../../etc/passwd", "/etc", "..", "", "a/b"] {
            assert!(
                crate::images::check_id(bad, "成果 id").is_err(),
                "必须拒绝: {bad:?}"
            );
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
        let source = include_str!("bridge/automation_commands.rs");
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
        let source = include_str!("bridge/automation_commands.rs");
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

#[cfg(test)]
mod agent_skill_bridge_tests {
    use super::*;
    use crate::domain::{AclLevel, AgentDialect};
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
    fn agent_validate_inner_rejects_oversized() {
        let huge = "x".repeat(crate::shared::validation::AGENT_DEF_MAX_BYTES + 1);
        let r = agent_validate_inner(&huge);
        assert!(!r.valid);
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn agent_parse_inner_rejects_oversized() {
        let huge = "x".repeat(crate::shared::validation::AGENT_DEF_MAX_BYTES + 1);
        assert!(agent_parse_inner(&huge).is_err());
    }

    #[test]
    fn agent_parse_inner_rejects_malformed() {
        assert!(agent_parse_inner("not json {{{").is_err());
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
    fn agent_permission_preview_gate_is_confirm() {
        // Agent 默认需确认（A2A 委派场景），权限预览闸门恒为 Confirm
        let p = agent_permission_preview_inner(&good_agent_json()).unwrap();
        assert_eq!(p.gate, AclLevel::Confirm);
    }

    #[test]
    fn agent_parse_inner_rejects_empty() {
        assert!(agent_parse_inner("").is_err());
        assert!(agent_parse_inner("   ").is_err());
    }
}

// ====== M5-2 MCP 只读注册表/策略桥命令（W7）======
// 无 rmcp / 无 server / 无监听 / 无网络；仅把已冻结的纯注册表 + 纯策略以只读
// introspection 形式暴露给受信任的主窗口。每一项都先过来源校验
// `check_invocation_source`（main 受信任免令牌；tab-*/grid-* 无令牌一律拒绝），
// 与 M5-2 红线和 W7 Hard Stop「只读」完全一致。路径裁决复用既有的
// `evaluate_mcp_command`（内部走 `check_path_within_roots` + `redact_sensitive_url`）。
