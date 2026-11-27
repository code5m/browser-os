#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// (capabilities/agent + capabilities/skill declared under `mod capabilities` below)
mod bridge;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod browser_profile_owner;
#[cfg(test)]
#[path = "../build_support/output_layout.rs"]
mod build_output_layout;
#[cfg(target_os = "linux")]
mod crashlog;
#[cfg(not(target_os = "linux"))]
#[path = "crashlog_windows.rs"]
mod crashlog;
mod domain;
mod grid_ipc;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod native_visibility;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod renderer_services;
mod gui_regression_receipt;
#[cfg(target_os = "windows")]
mod windows_apps;
#[cfg(target_os = "windows")]
mod windows_pipe;
#[cfg(target_os = "windows")]
mod windows_process;
#[cfg(target_os = "linux")]
type GridStream = std::os::unix::net::UnixStream;
#[cfg(target_os = "windows")]
type GridStream = windows_pipe::WindowsPipeStream;

#[cfg(target_os = "linux")]
mod grid_process;
#[cfg(target_os = "windows")]
#[path = "grid_process_windows.rs"]
mod grid_process;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[path = "grid_process_unsupported.rs"]
mod grid_process;
#[cfg(target_os = "linux")]
mod hang_diagnostics;
#[cfg(not(target_os = "linux"))]
#[path = "hang_diagnostics_windows.rs"]
mod hang_diagnostics;
mod shared {
    pub mod blocking;
    pub mod clipboard;
    pub(crate) mod credential_versions;
    pub(crate) mod dispatch_receipt;
    pub mod images;
    pub mod invocation;
    pub mod native_legacy_policy;
    #[cfg(target_os = "linux")]
    pub(crate) mod process_scope;
    pub mod validation;
}

mod framework {
    pub mod agent_channel;
    pub mod ai;
    pub mod commands;
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod isolated_capture;
mod mcp;
#[cfg(not(test))]
mod native_58_gui_review;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod native_supervisor;
mod security_policy;
mod session;
mod shutdown;
mod startup;
mod capabilities {
    pub mod agent;
    pub mod bfc;
    pub mod browser;
    pub mod database;
    pub mod git;
    pub mod graph;
    pub mod plugin;
    pub mod script;
    pub mod session;
    pub mod skill;
    pub mod task;
    pub mod terminal;
    pub mod tools;
    pub mod workspace;
}
mod tasks;
mod workbench;
#[cfg(debug_assertions)]
mod workbench_smoke;

// M5-W10 A3：MCP stdio-prep 骨架（feature-gated，默认构建不编译）。
// 仅 `--features mcp` 才纳入；以 `--mcp-stdio` 启动时不进入 Tauri 主流程，
// 直接跑只读 stdio JSON-RPC 环（复用 crate::mcp 注册表/策略接线，无网络监听、
// 无文件/数据库/脚本/插件执行副作用）。具体实现见 src/mcp_server.rs。
#[cfg(feature = "mcp")]
#[path = "mcp_server.rs"]
mod mcp_server;

// M5-1 re-export shim：已搬入 `mvp_core` 的模块在此再导出，
// 于是二进制侧既有的 `crate::<mod>::X` 路径**无需任何改写**即可继续解析。
// 每搬入一个模块，就在下方补一行 `pub use mvp_core::<mod>;`，
// 并保证 `mod <mod>;` 已从上面删除（重复定义由 check-core-boundary.py 的
// `CORE_SHIM_CONFLICT` 守门）。
pub use mvp_core::core::seam;
pub use mvp_core::keyring_store;

// Native Physical Boundary Pilot 3 (script): re-export shim so existing
// `crate::scripts::` / `crate::snippets::` call sites keep resolving without
// per-call-site edits (mirrors M5-1 mvp_core shim).
pub use crate::capabilities::script::scripts;
pub use crate::capabilities::script::snippets;

// Native Physical Boundary Pilot 4 (images): re-export shim so existing
// `crate::images::` call sites keep resolving without per-call-site edits.
pub use crate::shared::images;

// Native Physical Boundary Pilot 15 (clipboard): 与 images 同属 SHARED_NATIVE_INFRASTRUCTURE。
// 命令体从 bridge.rs 迁入 shared/clipboard.rs，crate 根再导出 `clipboard` 模块，
// 于是 generate_handler! 可直接以 `clipboard::clipboard_read` / `clipboard::clipboard_write` 注册。
pub use crate::shared::clipboard;
// Native Physical Boundary (invocation): 共享的来源校验 helper，crate 根再导出，
// 供各 capability 命令体以 `crate::invocation::check_invocation_source` 直接引用（原在 bridge.rs）。
pub use crate::shared::invocation;
// Native Physical Boundary (validation): 跨能力共享的输入校验原语（体积上限守卫 +
// 校验报告），crate 根再导出，供 agent/skill 命令体以 `crate::validation::` 直接引用。
pub use crate::shared::validation;

// Native Physical Boundary Pilot 5 (plugin): re-export shim so existing
// `crate::plugin::` call sites keep resolving without per-call-site edits.
pub use crate::capabilities::plugin::plugin;

// Native Physical Boundary Pilot 6 (graph): re-export shim so existing
// `crate::graph::` call sites (GraphState / load_snapshot / *_impl helpers,
// referenced by bridge.rs + main.rs) keep resolving without per-call-site edits.
pub use crate::capabilities::graph::graph;

// Native Physical Boundary Pilot 7 (workspace): re-export shim so existing
// `crate::workspace::` call sites (32 across scheduler/tasks/plugin/scripts/
// tools/bridge/workbench_smoke + `use crate::workspace;` in sync/bridge/tools)
// keep resolving without per-call-site edits.
pub use crate::capabilities::workspace::workspace;

// Native Physical Boundary Pilot 8 (database): re-export shim so existing
// `crate::database::` call sites (DbPool / QueryCancel / credential_key /
// DbQueryResult, all in bridge.rs) keep resolving without per-call-site edits.
pub use crate::capabilities::database::database;

// Native Physical Boundary Pilot 9 (scheduler): re-export shim so existing
// `crate::scheduler::` call sites (request_stop / cancel_in_flight / fire_now in
// bridge.rs; start in main.rs) keep resolving without per-call-site edits.
pub use crate::capabilities::task::scheduler;

// Native Physical Boundary Pilot 10 (script_runner): re-export shim so existing
// `crate::script_runner::` call sites (ScriptProcessTable / start_run /
// start_command / kill_all_running / process_group_alive; bridge 6 / tasks 2 /
// terminal 1 / scheduler 3) keep resolving without per-call-site edits.
pub use crate::capabilities::script::script_runner;

// Native Physical Boundary Pilot 11 (terminal): re-export shim so existing
// `crate::terminal::` call sites (ChannelSink / EventSink / TermInfo /
// TerminalSession, in bridge.rs) keep resolving without per-call-site edits.
pub use crate::capabilities::terminal::terminal;

// Native Physical Boundary Pilot 12 (sync): re-export shim so existing
// `crate::sync::` call sites (repo_dir; workbench_smoke.rs ×1) keep resolving
// without per-call-site edits.
pub use crate::capabilities::git::sync;

// Native Physical Boundary Pilot 13 (fs_cmds): re-export shim so existing
// `fs_cmds::` call sites (reveal_path / move_path in generate_handler!) keep
// resolving without per-call-site edits.
pub use crate::capabilities::workspace::fs_cmds;

// Native Physical Boundary Pilot 14 (tools): re-export shim so existing
// `crate::tools::` call sites (tool_html in main.rs) + `tools::list_tools` /
// `tools::open_tool` in generate_handler! keep resolving without per-call-site
// edits.
pub use crate::capabilities::tools::tools;

use bridge::AppState;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

// Native-only regression and startup paths use the exact same main-window
// authorization as the Tauri IPC commands. There is no privileged bypass.
fn native_main_webview(app: &tauri::AppHandle) -> Result<tauri::Webview, String> {
    app.get_webview("main")
        .ok_or_else(|| "NATIVE_MAIN_WEBVIEW_REQUIRED".into())
}
fn native_eval_in_tab(app: tauri::AppHandle, id: String, js: String) -> Result<String, String> {
    bridge::eval_in_tab_sync(native_main_webview(&app)?, app, id, js)
}
fn native_tab_activate(app: tauri::AppHandle, id: String) -> Result<(), String> {
    bridge::tab_activate_impl(native_main_webview(&app)?, app, id)
}
fn native_tab_position(
    app: tauri::AppHandle,
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    bridge::tab_position_sync(native_main_webview(&app)?, app, id, x, y, width, height)
}
fn native_hide_all_webviews(app: tauri::AppHandle) -> Result<(), String> {
    bridge::hide_all_webviews_sync(native_main_webview(&app)?, app)
}

/// The GTK thread must never run the terminal/grid/WebView cleanup chain.
#[derive(Default)]
struct ExitLifecycleState {
    started: std::sync::atomic::AtomicBool,
    completed: std::sync::atomic::AtomicBool,
    worker: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl ExitLifecycleState {
    fn reap_finished_worker(&self) {
        let finished = {
            let mut worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
            if worker.as_ref().is_some_and(|worker| worker.is_finished()) {
                worker.take()
            } else {
                None
            }
        };
        if let Some(worker) = finished {
            if worker.join().is_err() {
                eprintln!("[shutdown] SHUTDOWN_LAUNCHER_PANICKED");
            }
        }
    }

    fn claim(&self) -> bool {
        // A rejected save can reopen admission before its worker has returned.
        // Never replace that still-owned JoinHandle on a rapid second close.
        let worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        if worker.is_some() || self.completed() {
            return false;
        }
        !self.started.swap(true, std::sync::atomic::Ordering::AcqRel)
    }

    fn completed(&self) -> bool {
        self.completed.load(std::sync::atomic::Ordering::Acquire)
    }
}

fn publish_exit_status(app: &tauri::AppHandle, phase: &str, code: &str) {
    use tauri::Emitter;
    // Fixed phase/code/counts only: never send URLs, page text or filesystem
    // error strings into a global event or diagnostic log.
    if app
        .emit_to(
            "main",
            "app://exit-status",
            serde_json::json!({
                "phase": phase, "code": code
            }),
        )
        .is_err()
    {
        eprintln!("[shutdown] EXIT_STATUS_DELIVERY_FAILED");
    }
}

fn cancel_graceful_shutdown(app: &tauri::AppHandle, code: &str) {
    // Called only before the coordinator/native shutdown latches are entered.
    app.state::<ExitLifecycleState>()
        .started
        .store(false, std::sync::atomic::Ordering::Release);
    eprintln!("[shutdown] exit_cancelled {code}");
    publish_exit_status(app, "blocked", code);
}

fn begin_graceful_shutdown(app: tauri::AppHandle, trigger: &'static str) {
    let state = app.state::<ExitLifecycleState>();
    state.reap_finished_worker();
    if !state.claim() {
        return;
    }
    publish_exit_status(&app, "saving", "");
    let worker_app = app.clone();
    // Keep publication in the same slot gate checked by claim(). A fast save
    // failure may reopen started before spawn returns its JoinHandle.
    let mut worker_slot = state.worker.lock().unwrap_or_else(|e| e.into_inner());
    let spawn = std::thread::Builder::new()
        .name("browseros-shutdown".into())
        .spawn(move || {
            if let Err(code) = crate::capabilities::browser::public_api::prepare_capture_exit(&worker_app) {
                cancel_graceful_shutdown(&worker_app, &code);
                return;
            }
            // Saving is a reversible exit preflight, not an ordinary cleanup
            // task. A failed/panicked save must leave tabs and all native owners
            // alive so the user can correct the problem and retry closing.
            let saved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::capabilities::session::commands::prepare_exit_sessions(&worker_app)
            }));
            let saved = match saved {
                Ok(saved) => saved,
                Err(_) => {
                    cancel_graceful_shutdown(&worker_app, "SESSION_EXIT_SAVE_PANICKED");
                    return;
                }
            };
            if !saved.can_exit() {
                cancel_graceful_shutdown(&worker_app, &saved.failure_summary());
                return;
            }
            eprintln!(
                "[shutdown] sessions_saved persisted={} drafts_dropped={} tmp_removed={} capacity_removed={}",
                saved.summary.persisted, saved.summary.drafts_dropped,
                saved.summary.tmp_removed, saved.summary.capacity_removed
            );
            // The Session owner atomically accepted the authoritative snapshot
            // and closed bridge admission under its final locks. No fallible
            // session flush is repeated after this point.
            publish_exit_status(&worker_app, "closing", "");
            let started = std::time::Instant::now();
            let deadline = started + shutdown::APPLICATION_SHUTDOWN_BUDGET;
            eprintln!("[shutdown] begin trigger={trigger}");
            // Independent owners run off the UI loop and share one absolute
            // cleanup deadline. Saving cannot be timed out into data loss.
            let report = worker_app
                .state::<shutdown::ShutdownCoordinator>()
                .shutdown_until(deadline);
            eprintln!(
                "[shutdown] cleanup_report trigger={trigger} elapsed_ms={}",
                started.elapsed().as_millis()
            );
            log_shutdown_report(&report);
            let exit_code = if report.ok() { 0 } else { 1 };
            worker_app
                .state::<ExitLifecycleState>()
                .completed
                .store(true, std::sync::atomic::Ordering::Release);
            let exit_handle = worker_app.clone();
            if let Err(error) = worker_app.run_on_main_thread(move || exit_handle.exit(exit_code)) {
                eprintln!("[shutdown] scheduling final exit failed: {error}");
                worker_app.exit(1);
            }
        });
    match spawn {
        Ok(worker) => {
            *worker_slot = Some(worker);
            drop(worker_slot);
            state.reap_finished_worker();
        }
        Err(_) => {
            drop(worker_slot);
            cancel_graceful_shutdown(&app, "SESSION_EXIT_WORKER_UNAVAILABLE");
        }
    }
}

fn log_shutdown_report(report: &shutdown::ShutdownReport) {
    eprintln!(
        "[shutdown] report ok={} already_shutdown={} tasks={} waiting_tasks={} owner_pending={}",
        report.ok(),
        report.already_shutdown,
        report.executed(),
        report.pending(),
        !report.owners_complete
    );
    for failure in report.failures() {
        eprintln!(
            "[shutdown] task {} failed: {:?}",
            failure.name, failure.outcome
        );
    }
}

/// Phase 0 最小验证原型：解析 `--grid-child N`，有则以"宫格子进程"身份启动。
/// 返回 Some(N) 表示应走子进程分支；None 表示主进程。
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn parse_grid_child_arg() -> Option<u32> {
    let args: Vec<String> = std::env::args().collect();
    for (i, a) in args.iter().enumerate() {
        if a == "--grid-child" {
            if let Some(v) = args.get(i + 1) {
                if let Ok(n) = v.parse::<u32>() {
                    return Some(n);
                }
            }
        } else if let Some(rest) = a.strip_prefix("--grid-child=") {
            if let Ok(n) = rest.parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

/// 宫格子进程入口（Phase 2 完整版）：
/// - 极简壳窗口（无装饰/置顶/跳任务栏/初始隐藏），加载 about:blank —— 宫格 webview
///   add_child 全覆盖其上，壳页面本身无需任何前端内容（取代设计文档 4.6 的
///   /grid-renderer 前端路由方案，前端零改动）。
/// - browser-tabs 插件用 init_with_host 绑定本进程窗口 label（默认 "main" 会找不到宿主）。
/// - 注册宫格 webview 内 collect.js 需要的桥命令（采集/笔记/开终端）。
/// - UDS client：connect 主进程 → 命令循环（GridCmd → 本地 TabManagerState）；
///   插件事件与桥命令事件经 UDS Event 回传主进程。主进程断开 → 自行退出（防孤儿窗口）。
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn run_grid_child(index: u32) {
    // A parent flag is not evidence of kernel containment. Refuse ordinary
    // renderer startup before Tauri, workspace preparation or any WebView
    // if this process cannot observe the exact enforced domain limit.
    if grid_ipc::is_tab_renderer(index) {
        #[cfg(target_os = "linux")]
        let verified = shared::process_scope::verify_current_render_budget(grid_ipc::TAB_RENDERER_MEMORY_LIMIT).is_ok();
        #[cfg(target_os = "windows")]
        let verified = windows_process::verify_current_render_budget(grid_ipc::TAB_RENDERER_MEMORY_LIMIT).is_ok();
        if !verified {
            eprintln!("[tab-renderer] TAB_RENDERER_KERNEL_MEMORY_OWNER_UNCONFIRMED");
            std::process::exit(1);
        }
    }
    eprintln!(
        "[grid-child-{}] starting (pid={})",
        index,
        std::process::id()
    );
    crashlog::init();
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        std::env::set_var("GDK_BACKEND", "x11");
        if std::env::var("GTK_USE_PORTAL").is_err() {
            std::env::set_var("GTK_USE_PORTAL", "0");
        }
    }

    let label = format!("grid-child-{}", index);
    let sock_path = std::env::var("GRID_SOCK_PATH").unwrap_or_default();
    let label_for_plugin = label.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_browser_tabs::init_with_host(&label_for_plugin))
        .manage(AppState::default())
        .manage(security_policy::IntentRegistry::new())
        .manage(startup::PendingOpenUrls::default())
        .invoke_handler(tauri::generate_handler![
            bridge::report_title,
            crate::capabilities::browser::commands::report_resources,
            bridge::collect_selection,
            bridge::save_note,
            bridge::request_open_terminal,
            bridge::report_grid_load_failed,
            crate::framework::commands::debug_log,
        ])
        .setup(move |app| {
            if let Err(error) = startup::prepare_workspace(app.handle()) {
                // Tauri turns a returned setup error into a panic at RunEvent::Ready.
                // Report this failure here, before any WebView can be created.
                startup::report_failure(&error);
                std::process::exit(1);
            }
            app.state::<tauri_plugin_browser_tabs::TabManagerState>()
                .set_native_document_sink(move |id, instance, revision| {
                    let report = serde_json::json!({
                        "id": id, "instance": instance.to_string(), "revision": revision.to_string(),
                    });
                    if grid_ipc::valid_renderer_event(index, "native-document-revision", &report) {
                        grid_child_send_event("native-document-revision", report);
                    }
                });
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            if grid_ipc::is_tab_renderer(index) {
                app.state::<tauri_plugin_browser_tabs::TabManagerState>()
                    .set_native_shell_shortcut_sink(move |id, action| {
                        let payload = serde_json::json!({ "id": id, "action": action });
                        if grid_ipc::valid_renderer_event(index, "browser-tabs://shell-shortcut", &payload) {
                            enqueue_renderer_report("browser-tabs://shell-shortcut", payload);
                        }
                    });
                app.state::<tauri_plugin_browser_tabs::TabManagerState>()
                    .set_native_download_sink(move |id, phase| {
                        // Serialize callback observations and include cumulative
                        // counts: the bounded best-effort report queue may drop
                        // intermediate events. No URL/path leaves the child.
                        let mut counts = CHILD_DOWNLOAD_COUNTS.lock().unwrap_or_else(|p| p.into_inner());
                        let count = match phase {
                            "requested" => &mut counts.requested,
                            "succeeded" => &mut counts.succeeded,
                            "failed" => &mut counts.failed,
                            _ => return,
                        };
                        let Some(next) = count.checked_add(1) else { return; };
                        *count = next;
                        let payload = serde_json::json!({
                            "id": id, "phase": phase, "requested": counts.requested,
                            "succeeded": counts.succeeded, "failed": counts.failed
                        });
                        if grid_ipc::valid_renderer_event(index, "renderer-download", &payload) {
                            enqueue_renderer_report("renderer-download", payload);
                        }
                    });
            }
            let window = WebviewWindowBuilder::new(
                app,
                &label,
                WebviewUrl::External("about:blank".parse().unwrap()),
            )
            .title(format!("grid-child-{}", index))
            .inner_size(400.0, 300.0)
            .resizable(false)
            .decorations(false)
            // 宫格子窗口必须始终压在主窗之上（点击主窗不沉底），幽灵浮层靠
            // 主进程 blur/focus 转发 HideWindow/UpdateRect 抑制。
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .build()?;
            eprintln!("[grid-child-{}] shell window created", index);
            // 子窗口获焦/失焦上报主进程（主进程据此区分"点宫格"与"切走应用"）
            window.on_window_event(move |event| {
                use tauri::WindowEvent;
                let name = match event {
                    WindowEvent::Focused(true) => Some("grid-child-focus"),
                    WindowEvent::Focused(false) => Some("grid-child-blur"),
                    _ => None,
                };
                if let Some(name) = name {
                    grid_child_send_event(name, serde_json::json!({ "index": index }));
                }
            });
            start_grid_child_ipc(
                app.handle().clone(),
                index,
                label.clone(),
                sock_path.clone(),
            );
            // 子进程 layout enforcer：每 400ms 按 child_layouts 重放 update_rect，
            // 纠正 GTK 布局循环导致的宫格 webview 漂移（与主进程页签同款机制）
            bridge::start_layout_enforcer(app.handle().clone());
            // PHASE 2：资源扫描器是 Browser 全局基础设施，于启动期统一启动（幂等），
            // 覆盖 open_browser / tab_new / session_restore 全部建 tab 路径，避免 create_tab
            // 反向写 Session（保持 Browser→Session 零依赖）。
            bridge::start_resource_scanner(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("[grid-child-{}] RUN_ERROR", index);
            startup::report_failure(&e);
            std::process::exit(1);
        });
}

/// Critical native events and receipts share one bounded FIFO. A WebView
/// callback must never wait for the transport writer.
#[cfg(any(target_os = "linux", target_os = "windows"))]
static CHILD_CRITICAL: std::sync::OnceLock<std::sync::mpsc::SyncSender<grid_ipc::Wire>> =
    std::sync::OnceLock::new();

#[cfg(any(target_os = "linux", target_os = "windows"))]
static RENDERER_REPORTS: std::sync::OnceLock<std::sync::mpsc::SyncSender<grid_ipc::Wire>> =
    std::sync::OnceLock::new();

#[cfg(any(target_os = "linux", target_os = "windows"))]
static LATEST_DOWNLOAD_REPORT: std::sync::Mutex<Option<grid_ipc::Wire>> = std::sync::Mutex::new(None);

// Native callback owner; Ping can observe this even when an advisory event
// was coalesced or has not yet reached the parent.
#[cfg(any(target_os = "linux", target_os = "windows"))]
static CHILD_DOWNLOAD_COUNTS: std::sync::Mutex<grid_ipc::DownloadObservation> =
    std::sync::Mutex::new(grid_ipc::DownloadObservation { requested: 0, succeeded: 0, failed: 0 });

/// Resource and title notifications never synchronously write on the native
/// WebView event thread. Download observations have a separate one-item
/// coalescing slot so the final cumulative count does not compete for space.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn enqueue_renderer_report(name: &str, payload: serde_json::Value) {
    if name == "renderer-download" {
        *LATEST_DOWNLOAD_REPORT.lock().unwrap_or_else(|p| p.into_inner()) =
            Some(grid_ipc::Wire::Event { name: name.to_owned(), payload });
        return;
    }
    if let Some(sender) = RENDERER_REPORTS.get() {
        let _ = sender.try_send(grid_ipc::Wire::Event { name: name.to_owned(), payload });
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn enqueue_child_critical(message: grid_ipc::Wire) {
    let Some(sender) = CHILD_CRITICAL.get() else { return; };
    if sender.try_send(message).is_err() {
        // Losing an ordered navigation event or response would leave a
        // seemingly healthy parent with stale authority. Retire this child
        // fault domain instead; the original callers observe UNKNOWN.
        eprintln!("[grid-child] critical IPC lane unavailable");
        std::process::exit(1);
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn grid_child_send_event(name: &str, payload: serde_json::Value) {
    enqueue_child_critical(grid_ipc::Wire::Event { name: name.to_owned(), payload });
}

/// Use the same remaining-budget writer for child reports and receipts.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn write_child_frame(writer: &std::sync::Mutex<GridStream>, message: &grid_ipc::Wire)
    -> Result<(), String>
{
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let frame = grid_ipc::encode_wire(message).map_err(|_| "GRID_CHILD_FRAME_REJECTED")?;
    let mut stream = grid_ipc::lock_renderer_writer(writer, deadline, || Ok(()))?;
    write_child_stream(&mut *stream, &frame, deadline)
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn write_child_stream(stream: &mut GridStream, frame: &[u8], deadline: std::time::Instant)
    -> Result<(), String>
{
    grid_ipc::write_frame_until(frame, deadline, |bytes, remaining| {
        #[cfg(target_os = "linux")]
        {
            stream.set_write_timeout(Some(remaining))?;
            std::io::Write::write(stream, bytes)
        }
        #[cfg(target_os = "windows")]
        {
            stream.write_with_timeout(bytes, remaining)
        }
    }).map_err(|_| "GRID_CHILD_WRITE_FAILED".into())
}

/// 子进程 UDS client：连接主进程 socket（重试至 15s）→ 注册事件转发 → 命令循环。
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn start_grid_child_ipc(app: tauri::AppHandle, index: u32, host_label: String, sock_path: String) {
    use tauri::Listener;
    std::thread::spawn(move || {
        use crate::GridStream as UnixStream;
        let start = std::time::Instant::now();
        let stream = loop {
            match UnixStream::connect(&sock_path) {
                Ok(s) => break s,
                Err(e) => {
                    if start.elapsed().as_secs() > 15 {
                        eprintln!(
                            "[grid-child-{}] connect {} 失败: {e}，退出",
                            index, sock_path
                        );
                        std::process::exit(1);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        };
        let expected_parent = std::env::var("GRID_PARENT_PID")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|pid| *pid != 0);
        #[cfg(target_os = "linux")]
        let actual_server = grid_ipc::linux_peer_pid(&stream).ok();
        #[cfg(windows)]
        let actual_server = stream.server_pid().ok();
        if expected_parent.is_none() || actual_server != expected_parent {
            eprintln!("[grid-child-{index}] refused unauthenticated renderer parent");
            std::process::exit(1);
        }
        eprintln!("[grid-child-{}] UDS 已连接 {}", index, sock_path);
        let writer = match stream.try_clone() {
            Ok(w) => std::sync::Arc::new(std::sync::Mutex::new(w)),
            Err(e) => {
                eprintln!("[grid-child-{}] try_clone 失败: {e}", index);
                std::process::exit(1);
            }
        };
        let mut initial_writer = writer.lock().unwrap();
        let (critical_send, critical_receive) = std::sync::mpsc::sync_channel::<grid_ipc::Wire>(64);
        let critical_writer = writer.clone();
        if std::thread::Builder::new().name("renderer-critical-ipc".into()).spawn(move || {
            while let Ok(message) = critical_receive.recv() {
                if let Err(error) = write_child_frame(&critical_writer, &message) {
                    if error == "GRID_CHILD_FRAME_REJECTED" {
                        if let grid_ipc::Wire::Response { seq, .. } = &message {
                            // The operation may already have run. A local
                            // response-size rejection is not a definite failure.
                            let fallback = grid_ipc::Wire::err(*seq,
                                "GRID_REQUEST_OUTCOME_UNKNOWN: renderer response encoding rejected");
                            if write_child_frame(&critical_writer, &fallback).is_ok() { continue; }
                        }
                    }
                    std::process::exit(1);
                }
            }
        }).is_err() {
            std::process::exit(1);
        }
        let _ = CHILD_CRITICAL.set(critical_send);
        {
            // Grid and ordinary renderers both emit advisory bridge events.
            let (send, recv) = std::sync::mpsc::sync_channel::<grid_ipc::Wire>(32);
            let report_writer = writer.clone();
            if std::thread::Builder::new().name("renderer-advisory-reports".into()).spawn(move || {
                loop {
                    // Never hold the callback's coalescing lock during IPC.
                    let latest = LATEST_DOWNLOAD_REPORT.lock().unwrap_or_else(|p| p.into_inner()).take();
                    if let Some(report) = latest {
                        if let Err(error) = write_child_frame(&report_writer, &report) {
                            if error != "GRID_CHILD_FRAME_REJECTED" { std::process::exit(1); }
                        }
                    }
                    match recv.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok(report) => {
                            if let Err(error) = write_child_frame(&report_writer, &report) {
                                if error != "GRID_CHILD_FRAME_REJECTED" { std::process::exit(1); }
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            }).is_err() {
                std::process::exit(1);
            }
            let _ = RENDERER_REPORTS.set(send);
            // The native plugin callback installed during setup feeds this
            // lane directly. Never subscribe to a remotely emit-able event
            // as the source of an independent host keyboard shortcut.
        }

        // 插件事件（宫格 webview 内导航完成 / window.open）→ 主进程。
        {
            app.listen("browser-tabs://event", move |event| {
                let payload: serde_json::Value = match serde_json::from_str(event.payload()) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                let label = grid_ipc::renderer_label(index);
                if payload.get("id").and_then(|id| id.as_str()) != Some(label.as_str()) {
                    return;
                }
                match payload.get("type").and_then(|t| t.as_str()) {
                    Some("newWindowRequested") => {
                        if let Some(url) = payload.get("url").and_then(|u| u.as_str()) {
                            let report = serde_json::json!({ "id": label, "url": url });
                            // Oversized/invalid popup observations are denied,
                            // not sent as malformed frames that disconnect a host.
                            if grid_ipc::valid_renderer_event(index, "new-tab-request", &report) {
                                grid_child_send_event("new-tab-request", report);
                            }
                        }
                    }
                    // Native document_sink already publishes the authoritative
                    // invalidation. Do not duplicate it through the UI event
                    // lane, which can otherwise amplify navigation bursts.
                    Some("navigationStarted" | "documentRevision") => {}
                    Some("navigationFinished") => {
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        let report = serde_json::json!({ "id": label, "url": url });
                        if grid_ipc::valid_renderer_event(index, "tab-navigated", &report) {
                            grid_child_send_event("tab-navigated", report);
                        } else {
                            // Even an unrepresentable URL must invalidate the
                            // original document's authorization.
                            grid_child_send_event("tab-document-changed", serde_json::json!({ "id": label }));
                        }
                    }
                    Some("resourceReceived") if grid_ipc::is_tab_renderer(index) => {
                        if let Ok(mut raw) = serde_json::from_value::<bridge::RawResourceEvent>(payload.clone()) {
                            raw.url = security_policy::redact_sensitive_url(&raw.url);
                            if raw.method.as_ref().is_some_and(|v| v.len() > 32)
                                || raw.mime.as_ref().is_some_and(|v| v.len() > 256)
                            {
                                return;
                            }
                            enqueue_renderer_report("renderer-resource", serde_json::json!({
                                "id": label, "resource": raw
                            }));
                        }
                    }
                    // 原生 load-failed（TLS/网络错误页，JS 无法感知）→ 主进程自动重试
                    Some("loadFailed") => {
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        let err = payload.get("error").and_then(|e| e.as_str()).unwrap_or("");
                        eprintln!(
                            "[grid-child-{}] loadFailed url_bytes={}",
                            index, url.len()
                        );
                        let report = serde_json::json!({
                            "index": index.to_string(), "url": url,
                            "error": if err.is_empty() { "renderer load failed" } else { "native renderer load failed" },
                        });
                        if grid_ipc::valid_renderer_event(index, "grid-load-failed", &report) {
                            grid_child_send_event("grid-load-failed", report);
                        }
                    }
                    _ => {}
                }
            });
        }
        // 桥命令在子进程内 emit 的事件（采集完成/笔记保存/打开终端/加载失败上报）→ 主进程 → 前端。
        for name in [
            "open-terminal",
            "note-saved",
            "artifact-collected",
            "grid-load-failed",
        ] {
            let name_owned = name.to_string();
            app.listen(name, move |event| {
                // Reject oversized/invalid advisory payloads locally; they
                // cannot become an authenticated renderer lifecycle event.
                if event.payload().len() >= grid_ipc::MAX_GRID_WIRE_BYTES { return; }
                let payload: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or(serde_json::Value::Null);
                if grid_ipc::valid_renderer_event(index, &name_owned, &payload) {
                    if name_owned == "grid-load-failed" {
                        // A valid load-failure observation participates in
                        // document-bound retry ordering. Keep it on the
                        // critical lane, with a bounded normalized payload.
                        let report = serde_json::json!({
                            "index": index.to_string(), "url": payload["url"],
                            "error": "renderer load failed",
                        });
                        grid_child_send_event(&name_owned, report);
                    } else {
                        // Optional bridge notifications are best effort. The
                        // report worker enforces the encoded frame bound.
                        enqueue_renderer_report(&name_owned, payload);
                    }
                }
            });
        }

        let ready_deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let ready = grid_ipc::encode_wire(&grid_ipc::renderer_ready(index))
            .unwrap_or_else(|_| std::process::exit(1));
        if write_child_stream(&mut *initial_writer, &ready, ready_deadline).is_err() {
            std::process::exit(1);
        }
        drop(initial_writer);

        // 命令循环：Request → 本地执行 → Response。主进程断开（EOF/错误）→ 退出，
        // 避免主进程退出后残留置顶孤儿窗口。
        let mut reader = std::io::BufReader::new(stream);
        let mut request_fence = grid_ipc::GridRequestFence::default();
        let mut renderer_capture = crate::renderer_services::CaptureBuffer::default();
        loop {
            match grid_ipc::read_wire(&mut reader) {
                Ok(Some(grid_ipc::Wire::Request { seq, cmd, native_document })) => {
                    if let Err(code) = request_fence.admit(index, seq, &cmd) {
                        eprintln!("[grid-child-{index}] {code}");
                        // Do not acknowledge a duplicate sequence or execute a
                        // foreign renderer command. The parent observes disconnect.
                        break;
                    }
                    let document = if let Some(id) = cmd.document_id() {
                        let manager = app.state::<tauri_plugin_browser_tabs::TabManagerState>();
                        let admitted = native_document.filter(|stamp| stamp.usable())
                            .and_then(|stamp| manager.capture_document(&id.to_owned()).ok()
                                .filter(|document| document.native_identity() == (stamp.instance, stamp.revision)));
                        let Some(document) = admitted else {
                            enqueue_child_critical(grid_ipc::Wire::err(seq, "GRID_NATIVE_DOCUMENT_STALE"));
                            continue;
                        };
                        Some(document)
                    } else {
                        if native_document.is_some() {
                            enqueue_child_critical(grid_ipc::Wire::err(seq, "GRID_NATIVE_DOCUMENT_UNEXPECTED"));
                            continue;
                        }
                        None
                    };
                    if matches!(&cmd, grid_ipc::GridCmd::Ping) && grid_ipc::is_tab_renderer(index) {
                        let counts = *CHILD_DOWNLOAD_COUNTS.lock().unwrap_or_else(|p| p.into_inner());
                        let data = serde_json::json!({"requested":counts.requested,
                            "succeeded":counts.succeeded,"failed":counts.failed}).to_string();
                        enqueue_child_critical(grid_ipc::Wire::Response { seq, ok: true, err: None, data: Some(data) });
                        continue;
                    }
                    if let Some(result) = crate::renderer_services::dispatch(&app, &mut renderer_capture, &cmd, document.as_ref()) {
                        let response = match result {
                            Ok(data) => grid_ipc::Wire::Response { seq, ok: true, err: None, data },
                            Err(error) => grid_ipc::Wire::err(seq, error),
                        };
                        enqueue_child_critical(response);
                        continue;
                    }
                    if let grid_ipc::GridCmd::ReadReplies { ref id } = cmd {
                        let manager = app.state::<tauri_plugin_browser_tabs::TabManagerState>();
                        let Some(document) = document.as_ref() else {
                            enqueue_child_critical(grid_ipc::Wire::err(seq, "GRID_DOCUMENT_STALE"));
                            continue;
                        };
                        let result = manager.eval_result_document(
                                id, document,
                                include_str!("../injected/extract-replies.js").to_string(),
                                move |value| {
                                    let response = match value {
                                        Ok(data) => grid_ipc::Wire::Response {
                                            seq,
                                            ok: true,
                                            err: None,
                                            data: Some(data),
                                        },
                                        Err(e) => grid_ipc::Wire::err(seq, e),
                                    };
                                    enqueue_child_critical(response);
                                },
                            );
                        if result.is_err() {
                            enqueue_child_critical(grid_ipc::Wire::err(seq, "GRID_NOT_OPEN"));
                        }
                        continue;
                    }
                    if let grid_ipc::GridCmd::HibernationProbe { ref id } = cmd {
                        let manager = app.state::<tauri_plugin_browser_tabs::TabManagerState>();
                        if !manager.download_history_clear(id) {
                            enqueue_child_critical(grid_ipc::Wire::err(seq, "GRID_HIBERNATION_DOWNLOAD_HISTORY"));
                            continue;
                        }
                        let Some(document) = document.as_ref() else {
                            enqueue_child_critical(grid_ipc::Wire::err(seq, "GRID_DOCUMENT_STALE"));
                            continue;
                        };
                        let probe_manager = std::sync::Arc::clone(manager.inner());
                        let probe_id = id.clone();
                        let result = manager.eval_result_document(
                                id, document,
                                if grid_ipc::is_tab_renderer(index) {
                                    include_str!("../injected/tab-hibernation-probe.js").to_string()
                                } else {
                                    include_str!("../injected/grid-hibernation-probe.js").to_string()
                                },
                                move |value| {
                                    let value = if probe_manager.download_history_clear(&probe_id) { value }
                                        else { Err("GRID_HIBERNATION_DOWNLOAD_HISTORY".into()) };
                                    let response = match value {
                                        Ok(data) => grid_ipc::Wire::Response {
                                            seq,
                                            ok: true,
                                            err: None,
                                            data: Some(data),
                                        },
                                        Err(e) => grid_ipc::Wire::err(seq, e),
                                    };
                                    enqueue_child_critical(response);
                                },
                            );
                        if result.is_err() {
                            enqueue_child_critical(grid_ipc::Wire::err(seq, "GRID_HIBERNATION_PROBE_FAILED"));
                        }
                        continue;
                    }
                    let resp = match dispatch_grid_cmd(&app, &host_label, cmd, document.as_ref()) {
                        Ok(None) => grid_ipc::Wire::ok(seq),
                        Ok(Some(receipt)) => match serde_json::to_string(&receipt) {
                            Ok(data) => grid_ipc::Wire::Response { seq, ok: true, err: None, data: Some(data) },
                            Err(_) => grid_ipc::Wire::err(seq, "GRID_NATIVE_VISIBILITY_RECEIPT_INVALID"),
                        },
                        Err(e) => grid_ipc::Wire::err(seq, e),
                    };
                    enqueue_child_critical(resp);
                }
                Ok(Some(_)) => {}
                Ok(None) => {
                    eprintln!("[grid-child-{}] 主进程 UDS 断开，退出", index);
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("[grid-child-{}] UDS 读错误: {e}，退出", index);
                    std::process::exit(1);
                }
            }
        }
        std::process::exit(0);
    });
}

/// 子进程本地执行 GridCmd（插件命令原样复用，插件代码零改动）。
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn dispatch_grid_cmd(
    app: &tauri::AppHandle,
    host_label: &str,
    cmd: grid_ipc::GridCmd,
    document: Option<&tauri_plugin_browser_tabs::CaptureDocument>,
) -> Result<Option<grid_ipc::NativeWindowReceipt>, String> {
    use grid_ipc::GridCmd;
    use tauri_plugin_browser_tabs::{CreateTabOptions, LogicalRect, TabManagerState};
    let result = match cmd {
        GridCmd::CreateTab { id, url } => {
            let init_script = include_str!("../injected/collect.js");
            let manager = app.state::<TabManagerState>();
            match manager.create_tab(CreateTabOptions {
                id: id.clone(),
                url,
                // Start at the hidden host's real 400x300 viewport, never
                // force a rendering WebKitGTK WebView through a 1x1 resize.
                // The main process later supplies the final UpdateRect.
                rect: LogicalRect::new(0.0, 0.0, 400.0, 300.0),
                visible: true,
                auto_resize: true,
                user_agent: None,
                transparent: false,
                initialization_script: Some(init_script.to_string()),
            }) {
                Ok(()) => {
                    // The ordinary/Grid host starts hidden; creation must not
                    // accidentally reveal it before the owner's layout intent.
                    let win = app.get_window(host_label).ok_or("GRID_NATIVE_HOST_UNAVAILABLE")?;
                    if win.is_visible().map_err(|e| format!("GRID_NATIVE_VISIBILITY_UNCONFIRMED:{e}"))? {
                        win.hide().map_err(|e| format!("GRID_NATIVE_CREATE_HIDE_FAILED:{e}"))?;
                    }
                    if win.is_visible().map_err(|e| format!("GRID_NATIVE_VISIBILITY_UNCONFIRMED:{e}"))? {
                        return Err("GRID_NATIVE_CREATE_HIDE_UNCONFIRMED".into());
                    }
                    // Creation is not a window-system visibility receipt:
                    // a hidden shell may not have materialized its native
                    // surface yet. Explicit show/hide obtains that evidence.
                    eprintln!("[grid-child] CreateTab {} 完成", id);
                    Ok(())
                }
                // A different command identity must not turn an existing
                // renderer/document into a successful fresh creation.
                Err(e) => Err(format!("CreateTab {id} 失败: {e}")),
            }
        }
        GridCmd::UpdateRect { id, rect } => {
            rect.validate()?;
            // 注意用 get_window 而非 get_webview_window：后者在本场景返回 None
            // （实测），插件 host_window() 同样走 get_window。窗口操作 Window 都有。
            let win = app
                .get_window(host_label)
                .ok_or_else(|| "子进程壳窗口不存在".to_string())?;
            // A successful native receipt requires every layout stage.
            let result = (|| -> Result<grid_ipc::NativeWindowReceipt, String> {
                win.set_position(tauri::PhysicalPosition::new(rect.x as i32, rect.y as i32))
                    .map_err(|e| format!("set_position 失败: {e}"))?;
                win.set_size(tauri::PhysicalSize::new(rect.w as u32, rect.h as u32))
                    .map_err(|e| format!("set_size 失败: {e}"))?;
                let scale = win.scale_factor().map_err(|e| format!("scale_factor 失败: {e}"))?;
                if !scale.is_finite() || scale <= 0.0 {
                    return Err("GRID_NATIVE_SCALE_INVALID".into());
                }
                let lw = (rect.w / scale).max(1.0);
                let lh = (rect.h / scale).max(1.0);
                let manager = app.state::<TabManagerState>();
                manager.update_rect(&id, LogicalRect::new(0.0, 0.0, lw, lh))
                    .map_err(|e| format!("GRID_NATIVE_LAYOUT_FAILED:{e}"))?;
                win.show().map_err(|e| format!("show 失败: {e}"))?;
                // Native toolkit visibility observation is stronger than a
                // submitted show, but does not prove compositor paint.
                if !win.is_visible().map_err(|e| format!("GRID_NATIVE_VISIBILITY_UNCONFIRMED:{e}"))? {
                    return Err("GRID_NATIVE_SHOW_UNCONFIRMED".into());
                }
                let receipt = native_visibility::confirm(&win, Some(rect))?;
                app.state::<AppState>().child_layouts.lock().unwrap()
                    .insert(id.clone(), (0.0, 0.0, lw, lh));
                Ok(receipt)
            })();
            if result.is_err() {
                app.state::<AppState>().child_layouts.lock().unwrap().remove(&id);
                if let Err(error) = win.hide() {
                    return Err(format!("GRID_NATIVE_LAYOUT_HIDE_FAILED:{error}"));
                }
                if win.is_visible().map_err(|e| format!("GRID_NATIVE_VISIBILITY_UNCONFIRMED:{e}"))? {
                    return Err("GRID_NATIVE_LAYOUT_HIDE_UNCONFIRMED".into());
                }
                native_visibility::confirm(&win, None)
                    .map_err(|error| format!("GRID_NATIVE_LAYOUT_HIDE_UNCONFIRMED:{error}"))?;
            }
            return result.map(Some);
        }
        GridCmd::HideWindow { .. } => {
            let win = app
                .get_window(host_label)
                .ok_or_else(|| "子进程壳窗口不存在".to_string())?;
            win.hide().map_err(|e| format!("hide 失败: {e}"))?;
            if win.is_visible().map_err(|e| format!("GRID_NATIVE_VISIBILITY_UNCONFIRMED:{e}"))? {
                return Err("GRID_NATIVE_HIDE_UNCONFIRMED".into());
            }
            return native_visibility::confirm(&win, None).map(Some);
        }
        GridCmd::Eval { id, js } => {
            let manager = app.state::<TabManagerState>();
            manager
                .eval_document(&id, document.ok_or("GRID_NATIVE_DOCUMENT_STALE")?, &js)
                .map_err(|e| format!("eval {id} 失败: {e}"))
        }
        GridCmd::Navigate { id, url } => {
            let manager = app.state::<TabManagerState>();
            manager
                .navigate_document(&id, document.ok_or("GRID_NATIVE_DOCUMENT_STALE")?, &url)
                .map_err(|e| format!("navigate {id} 失败: {e}"))
        }
        GridCmd::SetZoom { id, zoom } => {
            let manager = app.state::<TabManagerState>();
            manager
                .set_zoom(&id, zoom)
                .map_err(|e| format!("set_zoom {id} 失败: {e}"))
        }
        GridCmd::CloseTab { id } => {
            let manager = app.state::<TabManagerState>();
            match manager.close_tab(&id) {
                Ok(()) | Err(tauri_plugin_browser_tabs::BrowserTabError::TabNotFound(_)) => {
                    app.state::<AppState>().child_layouts.lock().unwrap().remove(&id);
                    Ok(())
                }
                Err(e) => Err(format!("close_tab {id} 失败: {e}")),
            }
        }
        GridCmd::Ping => Ok(()),
        GridCmd::FillCredential { .. } | GridCmd::CaptureStart { .. }
        | GridCmd::CaptureRead { .. } | GridCmd::CaptureRelease { .. }
        | GridCmd::ReadReplies { .. } | GridCmd::DownloadTasks { .. } | GridCmd::HibernationProbe { .. }
        | GridCmd::ViewportSnapshot { .. } | GridCmd::ViewportRestore { .. } => {
            Err("GRID_ASYNC_ONLY".into())
        }
    };
    result.map(|_| None)
}

/// Test-only force-exit for a Grid child owned by this BrowserOS process.
/// The PID is verified against the active Grid manager before any OS call.
fn force_exit_owned_grid_child(app: &tauri::AppHandle, index: u32, pid: u32) -> Result<(), String> {
    if app.state::<AppState>().grid_manager.pid_of(index) != Some(pid) {
        return Err("GRID_SELFTEST_CHILD_PID_MISMATCH".into());
    }
    #[cfg(unix)]
    {
        if unsafe { libc::kill(pid as i32, libc::SIGSEGV) } != 0 {
            return Err("GRID_SELFTEST_SIGSEGV_FAILED".into());
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };
        let process = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
        if process.is_null() {
            return Err(format!(
                "GRID_SELFTEST_OPEN_PROCESS: {}",
                std::io::Error::last_os_error()
            ));
        }
        let ok = unsafe { TerminateProcess(process, 139) };
        let error = if ok == 0 {
            Some(format!(
                "GRID_SELFTEST_TERMINATE: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            None
        };
        unsafe {
            CloseHandle(process);
        }
        if let Some(error) = error {
            return Err(error);
        }
    }
    Ok(())
}

/// GRID_SELFTEST=1 端到端自检（Phase 2 + Phase 3 验收，无需手工点 UI）：
/// 1) create_grid(2) → 两子进程 UDS 建 webview
/// 2) grid_open / grid_position / eval_in_tab 经 UDS 正常
/// 3) kill -11 杀 grid-0 → 监控检测到 139 → 自动重启 + 状态重放 → eval 恢复
/// 4) close_grid 清理
/// 结果写 /tmp/grid-selftest-result.txt（PASS/FAIL + 各步明细），随后退出进程。
fn run_grid_selftest(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let log = |m: &str| {
            eprintln!("[grid-selftest] {}", m);
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(std::env::temp_dir().join("grid-selftest-result.txt"))
                .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{m}\n").as_bytes()));
        };
        let _ = std::fs::remove_file(std::env::temp_dir().join("grid-selftest-result.txt"));
        // 等主窗起来
        std::thread::sleep(std::time::Duration::from_secs(3));
        let fails = std::cell::RefCell::new(Vec::<String>::new());
        let step = |name: &str, r: Result<(), String>| match r {
            Ok(()) => log(&format!("PASS {name}")),
            Err(e) => {
                fails.borrow_mut().push(format!("{name}: {e}"));
                log(&format!("FAIL {name}: {e}"));
            }
        };
        // 1) 创建 2 宫格（spawn 2 子进程 + UDS CreateTab）
        step(
            "create_grid(2)",
            bridge::create_grid_blocking(app.clone(), 2, vec!["about:blank".to_string(); 2])
                .map(|_| ()),
        );
        // 2) 导航 + 定位 + eval
        step(
            "grid_open(0)",
            bridge::grid_open_blocking(app.clone(), 0, "https://www.baidu.com".into()),
        );
        step(
            "grid_open(1)",
            bridge::grid_open_blocking(app.clone(), 1, "https://www.bing.com".into()),
        );
        step(
            "grid_position(0)",
            bridge::grid_position_blocking(app.clone(), 0, 40.0, 120.0, 560.0, 420.0),
        );
        step(
            "grid_position(1)",
            bridge::grid_position_blocking(app.clone(), 1, 640.0, 120.0, 560.0, 420.0),
        );
        step(
            "eval_in_tab(grid-0)",
            native_eval_in_tab(app.clone(), "grid-0".into(), "document.title".into()).map(|_| ()),
        );
        // 3) kill -11 崩溃 grid-0 → 自动重启 + 重放
        let pid0 = app.state::<AppState>().grid_manager.pid_of(0);
        match pid0 {
            Some(pid) => {
                log(&format!("force-exit grid-child-0 pid={pid}"));
                if let Err(e) = force_exit_owned_grid_child(&app, 0, pid) {
                    fails.borrow_mut().push(e);
                }
            }
            None => fails.borrow_mut().push("pid_of(0) 为空".to_string()),
        }
        // 等监控检测(0.5s 轮询) + 子进程冷启动 + 重放
        std::thread::sleep(std::time::Duration::from_secs(12));
        let pid0_new = app.state::<AppState>().grid_manager.pid_of(0);
        match (pid0, pid0_new) {
            (Some(old), Some(new)) if new != old => log(&format!(
                "PASS crash-restart grid-0 重启 pid {old} -> {new}"
            )),
            _ => {
                fails
                    .borrow_mut()
                    .push(format!("grid-0 未重启: old={:?} new={:?}", pid0, pid0_new));
                log(&format!(
                    "FAIL crash-restart old={:?} new={:?}",
                    pid0, pid0_new
                ));
            }
        }
        step(
            "eval_in_tab(grid-0-after-restart)",
            native_eval_in_tab(app.clone(), "grid-0".into(), "1+1".into()).map(|_| ()),
        );
        // grid-1 全程存活
        let pid1_alive = app.state::<AppState>().grid_manager.pid_of(1).is_some();
        if pid1_alive {
            log("PASS grid-1-survived");
        } else {
            fails.borrow_mut().push("grid-1 未存活".to_string());
            log("FAIL grid-1-survived");
        }
        // 4) 清理
        step("close_grid", bridge::close_grid_blocking(app.clone()));
        let count = app.state::<AppState>().grid_manager.count();
        if count == 0 {
            log("PASS shutdown count=0");
        } else {
            fails
                .borrow_mut()
                .push(format!("shutdown 后 count={count}"));
            log(&format!("FAIL shutdown count={count}"));
        }
        if fails.borrow().is_empty() {
            log("SELFTEST_RESULT=ALL_PASS");
        } else {
            log(&format!(
                "SELFTEST_RESULT=FAIL ({})",
                fails.borrow().join(" | ")
            ));
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        std::process::exit(if fails.borrow().is_empty() { 0 } else { 1 });
    });
}

#[derive(serde::Serialize)]
struct GuiRegressionStep {
    id: &'static str,
    status: &'static str,
    detail: String,
}

#[derive(serde::Serialize)]
struct GuiRegressionReport {
    status: &'static str,
    steps: Vec<GuiRegressionStep>,
}

fn gui_regression_eval(case_id: &str, grid: usize, phase: &str, nonce: &str) -> String {
    format!(
        r#"(function(){{
  var params = new URLSearchParams({{
    case: "{case_id}",
    grid: "{grid}",
    phase: "{phase}",
    nonce: "{nonce}",
    title: document.title || "",
    cookie: document.cookie || "",
    bodyText: (document.body && document.body.innerText || "").slice(0, 80)
  }});
  return fetch("/event?" + params.toString(), {{ credentials: "include" }}).then(function(){{ return "ok"; }}).catch(function(e){{ return String(e); }});
}})();"#
    )
}

fn gui_regression_menu_eval(case_id: &str, grid: usize, phase: &str, nonce: &str) -> String {
    format!(
        r#"(function(){{
  document.dispatchEvent(new MouseEvent("contextmenu", {{
    bubbles: true,
    cancelable: true,
    clientX: 32,
    clientY: 32
  }}));
  setTimeout(function(){{
    var params = new URLSearchParams({{
      case: "{case_id}",
      grid: "{grid}",
      phase: "{phase}",
      nonce: "{nonce}",
      menu: String(!!document.getElementById("jzjd-menu")),
      cookie: document.cookie || ""
    }});
    fetch("/event?" + params.toString(), {{ credentials: "include" }}).catch(function(){{}});
  }}, 300);
  return "scheduled";
}})();"#
    )
}

fn gui_mock_url(base: &str, case_id: &str, grid: usize) -> String {
    format!(
        "{}/login?case={}&grid={}",
        base.trim_end_matches('/'),
        case_id,
        grid
    )
}

fn run_grid_gui_regression(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let report_path = std::env::var("M0_GUI_REGRESSION_REPORT").unwrap_or_else(|_| {
            std::env::temp_dir()
                .join("m0-6c-gui-regression.json")
                .to_string_lossy()
                .to_string()
        });
        let mock_base = std::env::var("M0_GUI_MOCK_BASE").unwrap_or_default();
        let receipt_path = std::env::var_os("M0_GUI_EVENT_FILE").map(std::path::PathBuf::from);
        let mut steps: Vec<GuiRegressionStep> = Vec::new();
        let mut fails: Vec<String> = Vec::new();
        let mut step = |id: &'static str, result: Result<String, String>| match result {
            Ok(detail) => {
                eprintln!("[m0-6c-gui] PASS {id}: {detail}");
                steps.push(GuiRegressionStep {
                    id,
                    status: "PASS",
                    detail,
                });
            }
            Err(error) => {
                eprintln!("[m0-6c-gui] FAIL {id}: {error}");
                fails.push(format!("{id}: {error}"));
                steps.push(GuiRegressionStep {
                    id,
                    status: "FAIL",
                    detail: error,
                });
            }
        };
        let sleep_ms = |ms| std::thread::sleep(std::time::Duration::from_millis(ms));
        let position_grid = |index: usize, x: f64, y: f64, w: f64, h: f64| {
            bridge::grid_position_blocking(app.clone(), index, x, y, w, h)
                .map(|_| format!("grid-{index} positioned"))
        };
        let open_grid = |case_id: &str, index: usize| {
            bridge::grid_open_blocking(app.clone(), index, gui_mock_url(&mock_base, case_id, index))
                .map(|_| format!("grid-{index} opened case={case_id}"))
        };
        let send_grid = |case_id: &str, index: usize, phase: &str| {
            gui_regression_receipt::ReceiptProbe::send(case_id, index, phase, false, |nonce| {
                native_eval_in_tab(
                    app.clone(),
                    format!("grid-{index}"),
                    gui_regression_eval(case_id, index, phase, nonce),
                )
                .map(|_| ())
            })
        };
        let wait_receipt = |probe: gui_regression_receipt::ReceiptProbe| {
            let path = receipt_path
                .as_deref()
                .filter(|path| path.is_absolute())
                .ok_or_else(|| "GUI_RECEIPT_PATH_MISSING".to_string())?;
            probe.wait(path)
        };
        let eval_grid = |case_id: &str, index: usize, phase: &str| {
            wait_receipt(send_grid(case_id, index, phase)?)
        };

        sleep_ms(3000);
        if mock_base.is_empty() {
            step("preflight", Err("M0_GUI_MOCK_BASE is empty".to_string()));
        } else {
            step("preflight", Ok(format!("mock_base={mock_base}")));
        }

        step(
            "scenario-1-single-ai-broadcast",
            bridge::create_grid_blocking(app.clone(), 4, vec!["about:blank".to_string(); 4])
                .and_then(|created| {
                    if created < 4 {
                        return Err(format!("created {created} grids, need 4"));
                    }
                    open_grid("1", 0)?;
                    position_grid(0, 40.0, 110.0, 620.0, 450.0)?;
                    sleep_ms(1200);
                    eval_grid("1", 0, "single_broadcast")?;
                    Ok("single grid command path exercised with local AI mock".to_string())
                }),
        );

        step(
            "scenario-2-four-grid-concurrent-ai",
            (|| {
                for i in 0..4 {
                    open_grid("2", i)?;
                    position_grid(
                        i,
                        if i % 2 == 0 { 40.0 } else { 680.0 },
                        if i < 2 { 110.0 } else { 590.0 },
                        600.0,
                        420.0,
                    )?;
                }
                sleep_ms(1800);
                let mut probes = Vec::with_capacity(4);
                for i in 0..4 {
                    probes.push(send_grid("2", i, "concurrent_ai")?);
                }
                for probe in probes {
                    wait_receipt(probe)?;
                }
                Ok("four grid children opened, positioned, and evaluated".to_string())
            })(),
        );

        step(
            "scenario-3-main-window-move-resize",
            (|| {
                open_grid("3", 0)?;
                sleep_ms(1000);
                let window = app
                    .get_webview_window("main")
                    .ok_or_else(|| "main window not found".to_string())?;
                window
                    .set_position(tauri::PhysicalPosition::new(80, 80))
                    .map_err(|e| e.to_string())?;
                window
                    .set_size(tauri::PhysicalSize::new(1280, 860))
                    .map_err(|e| e.to_string())?;
                sleep_ms(1200);
                app.state::<AppState>().grid_manager.reposition_visible();
                sleep_ms(800);
                eval_grid("3", 0, "after_move_resize")?;
                Ok("main window moved/resized and grid layout replayed".to_string())
            })(),
        );

        step(
            "scenario-4-blur-focus-grid-visibility",
            (|| {
                open_grid("4", 0)?;
                sleep_ms(1000);
                app.state::<AppState>().grid_manager.hide_for_blur();
                sleep_ms(700);
                app.state::<AppState>().grid_manager.show_for_focus();
                sleep_ms(900);
                eval_grid("4", 0, "after_focus_restore")?;
                Ok("grid windows hidden for blur and restored for focus".to_string())
            })(),
        );

        step(
            "scenario-5-switch-views",
            (|| {
                open_grid("5", 0)?;
                sleep_ms(1000);
                let tab = bridge::tab_new_native(app.clone(), gui_mock_url(&mock_base, "5", 0))?;
                native_tab_activate(app.clone(), tab.id.clone())?;
                native_tab_position(app.clone(), tab.id.clone(), 60.0, 120.0, 800.0, 520.0)?;
                native_hide_all_webviews(app.clone())?;
                sleep_ms(800);
                position_grid(0, 40.0, 110.0, 620.0, 450.0)?;
                sleep_ms(800);
                eval_grid("5", 0, "after_view_switch")?;
                bridge::tab_close_native(app.clone(), tab.id)?;
                Ok("tab view and grid view switched without orphaned child windows".to_string())
            })(),
        );

        step(
            "scenario-6-normal-tabs-multi-open",
            (|| {
                open_grid("6", 0)?;
                sleep_ms(1000);
                let first = bridge::tab_new_native(app.clone(), gui_mock_url(&mock_base, "6", 0))?;
                let second = bridge::tab_new_native(app.clone(), gui_mock_url(&mock_base, "6", 1))?;
                native_tab_activate(app.clone(), first.id.clone())?;
                native_tab_position(app.clone(), first.id.clone(), 80.0, 120.0, 760.0, 500.0)?;
                native_tab_activate(app.clone(), second.id.clone())?;
                native_tab_position(app.clone(), second.id.clone(), 90.0, 130.0, 760.0, 500.0)?;
                bridge::tab_close_native(app.clone(), first.id)?;
                bridge::tab_close_native(app.clone(), second.id)?;
                eval_grid("6", 0, "tabs_closed_grid_survives")?;
                Ok("normal tabs opened, switched, closed, and grid survived".to_string())
            })(),
        );

        step(
            "scenario-7-grid-child-crash-recovery",
            (|| {
                open_grid("7", 0)?;
                sleep_ms(1000);
                let old = app
                    .state::<AppState>()
                    .grid_manager
                    .pid_of(0)
                    .ok_or_else(|| "grid-0 pid missing before crash".to_string())?;
                force_exit_owned_grid_child(&app, 0, old)?;
                sleep_ms(12_000);
                let new = app
                    .state::<AppState>()
                    .grid_manager
                    .pid_of(0)
                    .ok_or_else(|| "grid-0 pid missing after crash".to_string())?;
                if old == new {
                    return Err(format!("grid-0 pid did not change: {old}"));
                }
                eval_grid("7", 0, "after_crash_recovery")?;
                if app.state::<AppState>().grid_manager.pid_of(1).is_none() {
                    return Err("grid-1 pid missing after grid-0 crash".to_string());
                }
                Ok(format!(
                    "grid-0 recovered pid {old}->{new}; grid-1 survived"
                ))
            })(),
        );

        step(
            "scenario-8-terminal-resource-context-menu",
            (|| {
                open_grid("8", 0)?;
                sleep_ms(1000);
                let term = bridge::term_spawn_sync(app.clone())?;
                bridge::term_write(
                    app.clone(),
                    term.id.clone(),
                    "printf 'M0_6C_TERMINAL_OK\\n'\n".to_string(),
                )?;
                sleep_ms(800);
                bridge::term_kill_sync(app.clone(), term.id)?;
                let stats = bridge::resource_stats_blocking(app.clone());
                let probe = gui_regression_receipt::ReceiptProbe::send(
                    "8",
                    0,
                    "context_menu",
                    true,
                    |nonce| {
                        native_eval_in_tab(
                            app.clone(),
                            "grid-0".to_string(),
                            gui_regression_menu_eval("8", 0, "context_menu", nonce),
                        )
                        .map(|_| ())
                    },
                )?;
                wait_receipt(probe)?;
                Ok(format!(
                    "terminal spawn/write/kill ok; resource app_total={:.1}MB",
                    stats.app_total_mb
                ))
            })(),
        );

        step(
            "scenario-9-login-state-after-restart",
            (|| {
                open_grid("9", 0)?;
                sleep_ms(1800);
                eval_grid("9", 0, "before_restart")?;
                let old = app
                    .state::<AppState>()
                    .grid_manager
                    .pid_of(0)
                    .ok_or_else(|| "grid-0 pid missing before login-state restart".to_string())?;
                force_exit_owned_grid_child(&app, 0, old)?;
                sleep_ms(12_000);
                let new = app
                    .state::<AppState>()
                    .grid_manager
                    .pid_of(0)
                    .ok_or_else(|| "grid-0 pid missing after login-state restart".to_string())?;
                if old == new {
                    return Err(format!("grid-0 pid did not change: {old}"));
                }
                eval_grid("9", 0, "after_restart")?;
                Ok(format!(
                    "login mock replayed across grid restart pid {old}->{new}"
                ))
            })(),
        );

        step(
            "cleanup-close-grid",
            bridge::close_grid_blocking(app.clone()).map(|_| {
                let count = app.state::<AppState>().grid_manager.count();
                format!("close_grid completed count={count}")
            }),
        );

        let status = if fails.is_empty() { "PASS" } else { "FAIL" };
        let report = GuiRegressionReport { status, steps };
        if let Some(parent) = std::path::Path::new(&report_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let write_result = serde_json::to_string_pretty(&report)
            .map_err(|e| e.to_string())
            .and_then(|content| std::fs::write(&report_path, content).map_err(|e| e.to_string()));
        if let Err(error) = write_result {
            eprintln!("[m0-6c-gui] failed to write report {report_path}: {error}");
            std::process::exit(1);
        }
        eprintln!("[m0-6c-gui] RESULT={status} report={report_path}");
        sleep_ms(1000);
        std::process::exit(if fails.is_empty() { 0 } else { 1 });
    });
}

/// M0-0.b 资源循环/终端吞吐驱动（契约 logs/m0-baseline-contract-v1.md §6.2/§6.3）。
/// 由 M0 采集脚本经 `M0_DRIVER` 环境变量启动，仿 GRID_SELFTEST 范式在应用内驱动真实
/// 场景，与脚本侧轮询同步（标记文件协议）：
/// - 循环类（tab|grid|terminal）：每次循环「创建场景 -> 稳定 2s -> 关闭 -> 稳定 2s」
///   -> 写 `<report_dir>/<kind>.cycle-<NN>.done`；全部结束后写 `<kind>.driver.result`。
/// - `term-throughput`：spawn PTY -> 发固定 10 MiB 负载命令 -> 前端检测 begin/end 标记
///   并上报 `m0_term_report` -> 驱动轮询报告文件后写 done。
/// 每类场景独立启动应用、独立统计（契约 §6.2 第 1 条），循环总数由 `M0_CYCLES` 传入。
fn run_m0_driver(app: tauri::AppHandle, cfg: bridge::M0Config) {
    std::thread::spawn(move || {
        let log = |m: &str| eprintln!("[m0-driver {}] {}", cfg.driver, m);
        let report_dir = cfg.report_dir.clone().unwrap_or_else(|| {
            std::env::temp_dir()
                .join("mvp-browser-os-m0")
                .to_string_lossy()
                .to_string()
        });
        let _ = std::fs::create_dir_all(&report_dir);
        let result_file = format!("{report_dir}/{}.driver.result", cfg.driver);
        let start_file = format!("{report_dir}/{}.driver.start", cfg.driver);
        let _ = std::fs::remove_file(&result_file);
        let _ = std::fs::write(&start_file, format!("{}\n", cfg.run_id));

        // 1) 等 ready 信号（前端 mount + 2×rAF + IPC 往返写出），最多 90 秒
        let ready_ok = match &cfg.ready_file {
            Some(p) => {
                let mut ok = false;
                for _ in 0..180 {
                    if let Ok(content) = std::fs::read_to_string(p) {
                        if content
                            .lines()
                            .next()
                            .map(|l| l.trim() == cfg.run_id)
                            .unwrap_or(false)
                        {
                            ok = true;
                            break;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                ok
            }
            None => true,
        };
        if !ready_ok {
            let _ = std::fs::write(&result_file, "FAIL ready-signal-timeout\n");
            log("FAIL ready signal timeout");
            std::process::exit(1);
        }
        log("ready ok");

        let finish = |code: i32, result: String| {
            let _ = std::fs::write(&result_file, format!("{result}\n"));
            log(&format!("driver result={result}"));
            std::thread::sleep(std::time::Duration::from_millis(500));
            std::process::exit(code);
        };

        // 2) 终端吞吐（契约 §6.3）：与循环类不同，独立启动、独立统计。
        //    终端由前端自动挂载并驱动 10 MiB 负载（前端持有 termId），
        //    本驱动只轮询前端上报的 term-throughput-report.json 并写完成标记。
        if cfg.driver == "term-throughput" {
            let cycles: usize = std::env::var("M0_CYCLES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            let mut fails: Vec<String> = Vec::new();
            for n in 1..=cycles {
                let report_file = format!("{report_dir}/term-throughput-report.json");
                let _ = std::fs::remove_file(&report_file);
                let mut got = false;
                for _ in 0..600 {
                    if std::fs::metadata(&report_file).is_ok() {
                        got = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                if got {
                    let _ = std::fs::write(
                        format!("{report_dir}/term-throughput.cycle-{n:02}.done"),
                        "ok\n",
                    );
                    log(&format!("cycle {n} report ok"));
                } else {
                    fails.push(format!("cycle {n}: report timeout"));
                    log(&format!("cycle {n} FAIL: report timeout"));
                }
            }
            if fails.is_empty() {
                finish(0, "PASS".to_string());
            } else {
                finish(1, format!("FAIL {}", fails.join(" | ")));
            }
        }

        // 3) 资源循环类（tab|grid|terminal，契约 §6.2）。每轮通过
        // prepare/opened/done/sampled 四阶段握手，保证脚本能分别取得创建前、关闭前、
        // 关闭后的进程树，且最后一轮采样完成前应用不会退出。
        enum OpenedResource {
            Tab(String),
            Grid,
            Terminal(String),
        }
        let wait_marker = |path: &str, timeout_s: u64| {
            for _ in 0..timeout_s * 2 {
                if std::fs::metadata(path).is_ok() {
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            false
        };
        let cycles: usize = std::env::var("M0_CYCLES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let mut fails: Vec<String> = Vec::new();
        for n in 1..=cycles {
            let prefix = format!("{report_dir}/{}.cycle-{n:02}", cfg.driver);
            let prepare = format!("{prefix}.prepare");
            let prepare_ack = format!("{prefix}.prepare.ack");
            let opened = format!("{prefix}.opened");
            let opened_ack = format!("{prefix}.opened.ack");
            let done = format!("{prefix}.done");
            let sampled = format!("{prefix}.sampled");
            for path in [&prepare_ack, &opened_ack, &done, &sampled] {
                let _ = std::fs::remove_file(path);
            }
            let _ = std::fs::write(&prepare, "ready\n");
            if !wait_marker(&prepare_ack, 90) {
                fails.push(format!("cycle {n}: prepare ack timeout"));
                break;
            }

            let opened_resource = match cfg.driver.as_str() {
                "tab" => bridge::tab_new_native(app.clone(), "about:blank".into())
                    .map(|tab| OpenedResource::Tab(tab.id)),
                "grid" => {
                    match bridge::create_grid_blocking(
                        app.clone(),
                        4,
                        vec!["about:blank".to_string(); 4],
                    ) {
                        Ok(4) => Ok(OpenedResource::Grid),
                        Ok(created) => {
                            let _ = bridge::close_grid_blocking(app.clone());
                            Err(format!("grid degraded to {created} (need 4)"))
                        }
                        Err(error) => Err(error),
                    }
                }
                "terminal" => bridge::term_spawn_sync(app.clone()).map(|term| {
                    log(&format!("cycle {n}: term_spawn ok id={}", term.id));
                    OpenedResource::Terminal(term.id)
                }),
                other => Err(format!("unknown M0_DRIVER: {other}")),
            };
            let settle_s = if cfg.driver == "grid" { 4 } else { 2 };
            if opened_resource.is_ok() {
                std::thread::sleep(std::time::Duration::from_secs(settle_s));
                let _ = std::fs::write(&opened, "ok\n");
            } else {
                let error = opened_resource.as_ref().err().unwrap();
                let _ = std::fs::write(&opened, format!("FAIL {error}\n"));
            }
            if !wait_marker(&opened_ack, 90) {
                fails.push(format!("cycle {n}: opened ack timeout"));
                break;
            }

            let close_result = match opened_resource {
                Ok(OpenedResource::Tab(id)) => bridge::tab_close_native(app.clone(), id),
                Ok(OpenedResource::Grid) => bridge::close_grid_blocking(app.clone()),
                Ok(OpenedResource::Terminal(id)) => bridge::term_kill_sync(app.clone(), id),
                Err(error) => Err(error),
            };
            if cfg.driver == "tab" && close_result.is_ok() {
                if let Err(error) = bridge::close_grid_blocking(app.clone()) {
                    fails.push(format!("cycle {n}: tab isolation close_grid: {error}"));
                    log(&format!(
                        "cycle {n} FAIL: tab isolation close_grid: {error}"
                    ));
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
            match close_result {
                Ok(()) => {
                    let _ = std::fs::write(&done, "ok\n");
                    log(&format!("cycle {n} done"));
                }
                Err(error) => {
                    fails.push(format!("cycle {n}: {error}"));
                    let _ = std::fs::write(&done, format!("FAIL {error}\n"));
                    log(&format!("cycle {n} FAIL: {error}"));
                }
            }
            if !wait_marker(&sampled, 90) {
                fails.push(format!("cycle {n}: sampled ack timeout"));
                break;
            }
        }
        if fails.is_empty() {
            finish(0, "PASS".to_string());
        } else {
            finish(1, format!("FAIL {}", fails.join(" | ")));
        }
    });
}

fn main() {
    // The one-shot Linux domain guardian must never enter Tauri, GTK,
    // application plugins, profile migration or the single-instance path.
    #[cfg(target_os = "linux")]
    if let Some(code) = shared::process_scope::launch_guard_if_requested() {
        std::process::exit(code);
    }
    // Explicit separate render job: no single-instance forwarding, workspace
    // loading, remote bridge commands or access to the live browser profile.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if let Some(code) = isolated_capture::launch_if_requested() {
        std::process::exit(code);
    }
    // Phase 0：宫格子进程分支（在最前判断，避免初始化主进程逻辑）
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if let Some(index) = parse_grid_child_arg() {
        run_grid_child(index);
        return;
    }

    // M5-W10 A3：MCP stdio-prep 骨架入口（仅 --features mcp 编译；默认构建不含）。
    // 以 `--mcp-stdio` 启动时不进入 Tauri 主流程，直接跑只读 stdio JSON-RPC 环，
    // 复用 crate::mcp 的注册表/策略接线，无网络监听、无文件/数据库/脚本/插件执行。
    #[cfg(feature = "mcp")]
    if std::env::args().any(|a| a == "--mcp-stdio") {
        crate::mcp_server::run_stdio();
        return;
    }

    // Whole-application recovery launcher: release Windows, explicitly
    // delegated Linux, or an eligible transient user scope select the
    // native owner. Special entries never create a second supervisor.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if let Some(code) = native_supervisor::launch_if_requested() {
        std::process::exit(code);
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let supervisor_endpoint = native_supervisor::take_child_endpoint();

    // Freeze the ordinary-tab route after supervisor/delegation selection
    // and before Tauri can publish any browser tab or restore a session.
    bridge::isolated_tabs::initialize();

    // 日志/崩溃捕获必须最先初始化（stderr 镜像 + panic 钩子 + 致命信号捕获）
    crashlog::init();
    // Workaround：WebKitGTK 在 Wayland 下默认启用 DMA-BUF 渲染器会静默崩溃
    // （进程启动数秒后退出、窗口白屏）。禁用后可稳定运行。
    #[cfg(target_os = "linux")]
    if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    // 用 X11 后端规避 Tauri v2 在 Wayland 下子 webview（浏览器/终端）定位错位的已知 bug。
    #[cfg(target_os = "linux")]
    std::env::set_var("GDK_BACKEND", "x11");
    // 禁用 GTK portal，避免初始化时等待 org.freedesktop.portal.* 超时导致窗口卡住/不显示。
    #[cfg(target_os = "linux")]
    if std::env::var("GTK_USE_PORTAL").is_err() {
        std::env::set_var("GTK_USE_PORTAL", "0");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // 浏览器页签/宫格子 webview 统一由 browser-tabs 插件创建与定位
        .plugin(tauri_plugin_browser_tabs::init())
        // M1-4：单实例——应用已运行时再次 xdg-open（第二实例）把 argv 中的
        // http(s) URL 路由给首个实例的内嵌页签，而不是另起一个主窗口。
        // grid 子进程走 run_grid_child 的独立 Builder（无本插件），不受影响。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            for u in bridge::extract_open_urls(&argv) {
                startup::dispatch_open_url(app, &u);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        // M2-8：注册自定义 `tool://` 协议，供工具箱打开离线 HTML 小工具。
        // 在 Builder 上注册（App 版 register_uri_scheme_protocol 为 mut self，无法在
        // setup 的 &mut App 上调用）。协议为 app 级全局注册；工具窗口不授予任何 bridge
        // 能力（见 F4 隔离）。handler 内经 UriSchemeContext.app_handle() 读取 workspace。
        .register_uri_scheme_protocol("tool", |ctx, request| {
            let handle = ctx.app_handle().clone();
            let id = request.uri().path().trim_start_matches('/');
            let html = crate::tools::tool_html(&handle, id);
            tauri::http::Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(html.into_bytes())
                .unwrap()
        })
        .setup(move |app| {
            // Prepare workspace storage before business state, schedulers or WebViews.
            // Early single-instance/Opened requests stay in bounded memory.
            if let Err(error) = startup::prepare_workspace(app.handle()) {
                // Tauri turns a returned setup error into a panic at RunEvent::Ready.
                // Report this failure here, before any WebView can be created.
                startup::report_failure(&error);
                std::process::exit(1);
            }
            // debug 主界面由 Vite 的 HTTP 地址提供，只在 debug 进程动态授予精确
            // localhost:1421 来源。文件故意放在 capabilities/ 之外，避免自动打进
            // release；默认 capability 也不向任何远程来源扩权。
            #[cfg(debug_assertions)]
            app.add_capability(include_str!("../dev-capabilities/main.json"))?;

            // M1-4 冷启动：应用未运行时 xdg-open 经 desktop 文件 %u 把 URL 放进
            // 本进程 argv。此时前端未就绪，统一进 pending 队列（handle_open_url
            // 内部就绪前不发提示），待 m0_ready 后由前端拉取打开，保证不丢。
            let argv: Vec<String> = std::env::args().collect();
            for u in bridge::extract_open_urls(&argv) {
                startup::dispatch_open_url(app.handle(), &u);
            }
            // 插件把 window.open / target="_blank" 统一拦截为 browser-tabs://event
            // (newWindowRequested)。这里转发为前端既有的 new-tab-request 事件，
            // 保持前端零改动。
            use tauri::{Emitter, Listener};
            let forward = app.handle().clone();
            app.listen("browser-tabs://event", move |event| {
                let payload: serde_json::Value = match serde_json::from_str(event.payload()) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                match payload.get("type").and_then(|t| t.as_str()) {
                    Some("newWindowRequested") => {
                        if let Some(url) = payload.get("url").and_then(|u| u.as_str()) {
                            eprintln!("[main] forward new-tab-request url_bytes={}", url.len());
                            let _ =
                                forward.emit("new-tab-request", serde_json::json!({ "url": url }));
                        }
                    }
                    // 子 webview 内导航完成（点链接/前进/后退/刷新），转发给前端同步地址栏
                    Some("navigationFinished") => {
                        let id = payload.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        // Navigation URLs may contain query credentials. Only the
                        // trusted frontend event carries the full URL; disk logs
                        // record tab identity and URL length, never its contents.
                        static NAV_SEQUENCE: std::sync::atomic::AtomicU64 =
                            std::sync::atomic::AtomicU64::new(1);
                        let seq = NAV_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        bridge::refresh_embedded_navigation(&forward, id);
                        eprintln!("[navigation] main_forward seq={} tab={} url_bytes={}", seq, id, url.len());
                        let _ = forward
                            .emit("tab-navigated", serde_json::json!({ "id": id, "url": url, "seq": seq }));
                    }
                    // 插件只暴露 loadFailed（网络/TLS/站点加载失败），不等价于渲染进程崩溃。
                    // 这里仅转为可观察事件，不自动重建，避免坏 URL 进入恢复循环。
                    Some("loadFailed") => {
                        let id = payload.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        let error = payload.get("error").and_then(|e| e.as_str()).unwrap_or("");
                        bridge::report_tab_load_failed(&forward, id, url, error);
                    }
                    // M1-8：插件原生资源事件（含原始 URL，仅进程内）→ 脱敏 + 容量上限
                    // + 入库 + resource-received 推前端。绝不直接转发原始 payload。
                    Some("resourceReceived") => {
                        let id = payload.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        if !id.is_empty() {
                            match serde_json::from_value::<bridge::RawResourceEvent>(
                                payload.clone(),
                            ) {
                                Ok(raw) => bridge::on_resource_received(&forward, id, raw),
                                Err(e) => {
                                    eprintln!("[main] resourceReceived payload 解析失败: {e}")
                                }
                            }
                        }
                    }
                    _ => {}
                }
            });
            // 标准 WebviewWindow：主 webview 作为窗口主内容，由 tao/wry 正常管理尺寸。
            // （此前用裸 Window + add_child 承载主 UI，wry 的 GtkFixed 路径只做
            //   set_size_request 而不保证 allocation，导致主 webview CSS 视口卡在 ~400px。）
            // 浏览器页签/宫格仍通过 browser-tabs 插件 add_child 到该窗口——这不是
            // 多顶层 WebviewWindow，不会触发 X11 多窗口死锁。
            // dev 下强制指向 vite 开发服务器：本项目窗口是 Rust 代码里 programmatic
            // 创建的，曾出现 App URL 未切到 Vite、持续加载旧 dist 的问题。该 HTTP
            // 来源由 dev-capabilities/main.json 在 debug 时动态精确授权；不能在全局
            // 配置恢复 devUrl，否则无 custom-protocol 的 release 构建也会加载开发
            // 地址。正式打包仍走 App("index.html") 的包内资源。
            // MVP_FORCE_DIST=1：强制加载内嵌 dist（白屏二分诊断用，绕开 vite）
            let main_url = if cfg!(debug_assertions) && std::env::var("MVP_FORCE_DIST").is_err() {
                WebviewUrl::External("http://localhost:1421".parse().unwrap())
            } else {
                WebviewUrl::App("index.html".into())
            };
            let window = WebviewWindowBuilder::new(app, "main", main_url)
                .title("浏览器OS融合")
                .decorations(false)
                .inner_size(1200.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .maximized(true)
                .build()?;
            eprintln!("[main] main window url={:?}", window.url());
            // Out-of-band GTK health sampler. The timer posts at most one
            // callback; the dedicated worker can still snapshot thread wait
            // channels and memory pressure when the window loop is blocked.
            hang_diagnostics::start(app.handle().clone());
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            if let Some(endpoint) = supervisor_endpoint {
                native_supervisor::start_child_probe(app.handle().clone(), endpoint);
            }
            let _ = window.show();
            let _ = window.set_focus();
            // 修正主 UI webview 的 GTK allocation / CSS 视口（WebKitGTK 偶发卡在小尺寸，
            // 导致顶部 TitleBar/ActivityBar 等被挤出可视区）。scale=1 时等效无操作。
            if let Some(main_wv) = app.get_webview("main") {
                let _ = tauri_plugin_browser_tabs::ensure_native_layout(&main_wv);
            }
            workspace::record_lifecycle(app.handle(), "main-window-created-and-shown");
            // 宫格子进程管理器：注入 AppHandle（事件转发/定位换算/崩溃自愈）
            app.state::<AppState>()
                .grid_manager
                .set_app(app.handle().clone());
            bridge::register_shutdown_tasks(app.handle()).map_err(std::io::Error::other)?;
            // M4-7：启动调度线程（全进程仅一次，`SchedulerHandle` 由 OnceLock 保证）。
            // 退出收口走 `stop-scheduler` 关机任务（已注册在 `kill-running-scripts`
            // 之前，见 F7）；本线程只做触发，执行唯一入口是 `script_runner`（F6）。
            crate::scheduler::start(app.handle().clone());
            // M1-9：会话策略默认值（关闭弹窗默认开 = 关闭不可静默丢弃；
            // 退出自动保存默认关 = 不静默保存浏览痕迹）。
            app.state::<AppState>()
                .session_close_prompt
                .store(true, std::sync::atomic::Ordering::SeqCst);
            // M1-9：异常退出后的恢复边界——原子写保证没有半截 JSON，这里只清理
            // 残留 .tmp 并按容量裁剪；会话本身保持「用户已保存才存在」的语义。
            {
                let dir = workspace::sessions_dir(app.handle());
                let tmp = crate::session::prune_tmp_files(&dir);
                let pruned = crate::session::prune_sessions(&dir, domain::SESSION_MAX_COUNT);
                let total = crate::session::count_sessions(&dir);
                eprintln!(
                    "[main] sessions restored total={total} tmp_cleaned={tmp} capacity_pruned={pruned}"
                );
            }
            // M0-3.a：打印安全策略指纹，确认运行中的二进制对应哪版策略契约。
            // 本检查点只定义契约，不收口任何调用方（M0-3.b/c/d）。
            eprintln!(
                "[main] security policy: {}",
                security_policy::policy_fingerprint()
            );
            // 主窗 Moved/Resized → 宫格子进程窗口跟随（子窗口是独立顶层窗口，
            // 不像 add_child 自动跟随）；Focused → 失焦隐藏/聚焦恢复（防幽灵浮层）；
            // CloseRequested → 杀掉全部子进程（防孤儿置顶窗口）。
            {
                let handle = app.handle().clone();
                let main_win = window.clone();
                main_win.on_window_event(move |event| {
                    use tauri::WindowEvent;
                    let state = handle.state::<AppState>();
                    match event {
                        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                            // Bounded latest-value work: never acquire a grid
                            // UDS writer lock on the GTK window event thread.
                            state.grid_manager.request_reposition_async();
                        }
                        WindowEvent::Focused(false) => {
                            // 延迟判定：点击宫格子窗口同样触发主窗 blur，
                            // 150ms 内等子进程焦点上报到达后再决定（睡必须离开事件回调）
                            handle.state::<AppState>()
                                .grid_manager
                                .request_hide_for_blur_async();
                        }
                        WindowEvent::Focused(true) => {
                            // show_for_focus sends UDS messages; avoid blocking GTK
                            // behind an unresponsive grid child.
                            handle.state::<AppState>()
                                .grid_manager
                                .request_show_for_focus_async();
                        }
                        WindowEvent::CloseRequested { api, .. } => {
                            // Prevent the main window from disappearing while
                            // resource cleanup is still in progress. GTK only
                            // schedules work and never waits for it.
                            api.prevent_close();
                            begin_graceful_shutdown(handle.clone(), "window_close");
                        }
                        _ => {}
                    }
                });
            }
            // 启动布局守护线程：持续纠正 GTK 布局循环导致的子 webview 位置漂移
            // （页签 tab-N 仍在主进程 add_child；宫格已迁子进程不在 child_layouts）
            bridge::start_layout_enforcer(app.handle().clone());
            // 页签休眠清扫线程（开关默认关，设置面板开启后生效）
            bridge::start_hibernation_sweeper(app.handle().clone());
            // Grid load-failure retries are admitted directly by the authenticated
            // child reader; no globally emitted event grants retry authority.
            // GRID_SELFTEST=1：宫格多进程端到端自检（Phase 2 UDS 转发 + Phase 3 崩溃自愈）。
            // 跑完写 /tmp/grid-selftest-result.txt 并退出。日常运行不设该变量即可。
            if std::env::var("GRID_SELFTEST").is_ok() {
                run_grid_selftest(app.handle().clone());
            }
            #[cfg(debug_assertions)]
            if std::env::var("MVP_WORKBENCH_SMOKE").is_ok() {
                workbench_smoke::start(app.handle().clone()).map_err(std::io::Error::other)?;
            }
            // GRID_GUI_REGRESSION=1：M0-6.c 九项 GUI 回归驱动。由外层脚本提供本地
            // AI mock 与证据目录，驱动真实 Tauri 主窗、宫格子进程、页签与 PTY。
            if std::env::var("GRID_GUI_REGRESSION").is_ok() {
                run_grid_gui_regression(app.handle().clone());
            }
            // M0-0.b 测量钩子：M0_RUN_ID 非空时注入测量配置（日常运行全空，零影响），
            // 并按 M0_DRIVER 启动资源循环/终端吞吐驱动（契约 §6.2/§6.3）。
            let m0_run_id = std::env::var("M0_RUN_ID").unwrap_or_default();
            if !m0_run_id.is_empty() {
                let m0_cfg = bridge::M0Config {
                    run_id: m0_run_id,
                    ready_file: std::env::var("M0_READY_FILE").ok(),
                    report_dir: std::env::var("M0_REPORT_DIR").ok(),
                    driver: std::env::var("M0_DRIVER").unwrap_or_default(),
                };
                *app.state::<AppState>().m0_config.lock().unwrap() = m0_cfg.clone();
                if !m0_cfg.driver.is_empty() {
                    run_m0_driver(app.handle().clone(), m0_cfg);
                }
            }
            // M5-W12（Lane A7）：图谱只读快照载入（缺文件/损坏/双扫命中均回退空 store，绝不 panic）。
            {
                let path = workspace::data_dir(app.handle()).join("graph.json");
                let store = crate::graph::load_snapshot(&path);
                if let Ok(mut g) = app.state::<crate::graph::GraphState>().store.write() {
                    *g = store;
                }
            }
            // Opt-in installed-client audit: all 58 legacy commands are invoked
            // through the real main WebView IPC; remote-only telemetry is tested
            // by a separate loopback browser tab. Never enabled in normal runs.
            #[cfg(not(test))]
            if std::env::var("NATIVE_58_GUI_REVIEW").is_ok() {
                native_58_gui_review::start(app.handle().clone()).map_err(std::io::Error::other)?;
            }
            Ok(())
        })
        .manage(AppState::default())
        .manage(startup::PendingOpenUrls::default())
        .manage(capabilities::bfc::runtime::BfcRuntime::default())
        .manage(capabilities::bfc::managed_service::ManagedBfcService::default())
        // M0-2.b：注入统一生命周期核心。本检查点只提供核心，不迁移任何退出路径
        // （M0-2.c 负责把窗口关闭/菜单退出/系统退出与 grid/PTY/tab/线程调用方切到它）。
        .manage(shutdown::ShutdownCoordinator::new())
        .manage(ExitLifecycleState::default())
        // M0-3.b：一次性用户意图令牌登记表（外部页面发起副作用调用时校验）。
        .manage(security_policy::IntentRegistry::new())
        // M4-3（Lane A4）：数据库连接配置登记簿（不含非 Send 的池句柄，按需重连）。
        .manage(crate::capabilities::database::commands::DbConnectionRegistry::default())
        .manage(crate::capabilities::git::local_workbench::LocalGitWriteQueue::default())
        // M5-W12（Lane A7）：图谱只读托管状态（载入期写一次，运行期只读）。
        .manage(crate::graph::GraphState::default())
        .manage(capabilities::git::studio::GitStudioRuntime::default())
        .manage(capabilities::database::studio::DatabaseStudioRuntime::default())
        .invoke_handler(tauri::generate_handler![
            capabilities::database::studio::db_studio_cancel,
            capabilities::database::studio::db_studio_close,
            capabilities::database::studio::db_studio_call,
            capabilities::database::studio::db_studio_open,
            capabilities::bfc::commands::bfc_readback_begin,
            capabilities::bfc::commands::bfc_readback_submit,
            capabilities::bfc::commands::bfc_readback_status,
            capabilities::bfc::commands::bfc_readback_cancel,
            capabilities::bfc::commands::bfc_readback_finish,
            capabilities::bfc::commands::bfc_service_status,
            capabilities::bfc::commands::bfc_service_start,
            capabilities::bfc::commands::bfc_service_stop,
            capabilities::git::studio::git_studio_cancel,
            capabilities::git::studio::git_studio_confirm,
            capabilities::git::studio::git_studio_preview,
            capabilities::git::studio::git_studio_read,
            bridge::open_browser,
            bridge::close_browser,
            bridge::position_browser,
            crate::capabilities::browser::commands::report_resources,
            bridge::report_grid_load_failed,
            bridge::report_title,
            bridge::collect_selection,
            bridge::request_open_terminal,
            bridge::save_note,
            bridge::list_artifacts,
            bridge::configure_repo,
            bridge::list_repos,
            crate::capabilities::git::commands::request_sync,
            crate::capabilities::git::commands::confirm_sync,
            crate::framework::commands::audit_log,
            bridge::read_artifact,
            bridge::update_artifact,
            bridge::delete_artifact,
            bridge::browse_workspace,
            bridge::add_bookmark,
            bridge::list_bookmarks,
            bridge::remove_bookmark,
            bridge::list_dir,
            bridge::read_file,
            bridge::read_image_data_url,
            bridge::write_file,
            capabilities::workspace::cas::write_file_cas,
            capabilities::workspace::draft_journal::editor_draft_save,
            capabilities::workspace::draft_journal::editor_draft_list,
            capabilities::workspace::draft_journal::editor_draft_clear,
            capabilities::workspace::draft_journal::editor_draft_catalog,
            capabilities::workspace::draft_journal::editor_draft_read,
            capabilities::workspace::draft_journal::editor_draft_update_recovered,
            capabilities::workspace::draft_journal::editor_draft_remove,
            capabilities::workspace::draft_journal::editor_draft_save_as,
            capabilities::workspace::weixin_drafts::wechat_account_save,
            capabilities::workspace::weixin_drafts::wechat_account_status,
            capabilities::workspace::weixin_drafts::wechat_draft_permission_check,
            capabilities::workspace::weixin_drafts::wechat_upload_body_image,
            capabilities::workspace::weixin_drafts::wechat_upload_cover,
            capabilities::workspace::weixin_drafts::wechat_create_draft,
            capabilities::workspace::weixin_drafts::wechat_get_draft_status,
            capabilities::workspace::weixin_drafts::wechat_working_draft_load,
            capabilities::workspace::weixin_drafts::wechat_working_draft_save,
            capabilities::workspace::weixin_drafts::wechat_working_draft_clear,
            bridge::get_start_dirs,
            bridge::reveal_artifact,
            fs_cmds::reveal_path,
            fs_cmds::move_path,
            bridge::open_source,
            bridge::create_file,
            bridge::create_dir,
            bridge::delete_path,
            bridge::rename_path,
            clipboard::clipboard_read,
            clipboard::clipboard_write,
            bridge::create_grid,
            bridge::close_grid,
            bridge::grid_open,
            bridge::grid_position,
            bridge::grid_set_zoom,
            bridge::grid_close_one,
            bridge::hide_all_webviews,
            bridge::hide_webview,
            crate::framework::commands::debug_log,
            bridge::list_apps,
            crate::framework::commands::launch_app,
            bridge::tab_new,
            bridge::tab_close,
            bridge::tab_open,
            bridge::tab_position,
            bridge::tab_list,
            bridge::tab_set_title,
            bridge::tab_activate,
            bridge::tab_mutation,
            bridge::tab_go_back,
            bridge::tab_go_forward,
            bridge::tab_reload,
            bridge::eval_in_tab,
            crate::capabilities::browser::commands::capture_full_page,
            bridge::set_tab_hibernation,
            bridge::resource_stats,
            bridge::term_spawn,
            bridge::term_spawn_channel,
            bridge::term_write,
            bridge::term_resize,
            bridge::term_kill,
            bridge::m0_ready,
            bridge::m0_term_report,
            bridge::m0_config,
            bridge::issue_intent,
            bridge::take_pending_open_urls,
            bridge::get_default_browser,
            bridge::set_default_browser,
            crate::capabilities::git::commands::git_status,
            crate::capabilities::git::commands::git_diff,
            crate::capabilities::git::commands::git_branch_list,
            crate::capabilities::git::commands::request_git_write,
            crate::capabilities::git::commands::confirm_git_write,
            crate::capabilities::browser::commands::list_tab_resources,
            crate::capabilities::browser::commands::clear_tab_resources,
            crate::capabilities::browser::commands::get_resource_capture_settings,
            crate::capabilities::browser::commands::set_resource_capture_settings,
            capabilities::session::commands::session_save,
            capabilities::session::commands::session_discard,
            capabilities::session::commands::session_list,
            capabilities::session::commands::session_get,
            capabilities::session::commands::session_delete,
            capabilities::session::commands::session_export,
            capabilities::session::commands::session_restore,
            capabilities::session::commands::flush_sessions,
            capabilities::session::commands::get_session_policy,
            capabilities::session::commands::set_session_policy,
            bridge::save_image,
            bridge::list_artifact_images,
            bridge::workspace_images_dir,
            bridge::script_list,
            bridge::script_add,
            bridge::script_update,
            bridge::script_remove,
            bridge::snippet_list,
            bridge::snippet_add,
            bridge::snippet_update,
            bridge::snippet_remove,
            bridge::run_command,
            tools::list_tools,
            tools::open_tool,
            tools::close_tools,
            bridge::run_script,
            bridge::cancel_script,
            bridge::script_status,
            bridge::script_runs_list,
            crate::capabilities::task::commands::task_list,
            crate::capabilities::task::commands::task_add,
            crate::capabilities::task::commands::task_update,
            crate::capabilities::task::commands::task_remove,
            crate::capabilities::task::commands::task_run_now,
            bridge::agent_parse,
            bridge::agent_validate,
            crate::capabilities::agent::local_provider::agent_local_chat,
            crate::framework::ai::ai_service,
            crate::framework::ai::coding_service,
            crate::framework::agent_channel::agent_runtime_get_config,
            crate::framework::agent_channel::agent_runtime_set_config,
            crate::framework::agent_channel::agent_runtime_read_tool,
            crate::framework::agent_channel::agent_runtime_benchmark,
            crate::framework::agent_channel::agent_runtime_run_fixture,
            bridge::agent_permission_preview,
            crate::capabilities::skill::commands::skill_parse,
            crate::capabilities::skill::commands::skill_validate,
            crate::capabilities::skill::commands::skill_permission_preview,
            bridge::import_browser_credentials,
            bridge::list_browser_credentials,
            bridge::fill_browser_credential,
            crate::capabilities::database::commands::db_connect,
            crate::capabilities::database::commands::db_list_connections,
            crate::capabilities::database::commands::db_import_profiles,
            crate::capabilities::database::commands::db_cancel,
            crate::capabilities::database::commands::db_query,
            crate::capabilities::database::commands::db_disconnect,
            bridge::vault_open,
            bridge::vault_save_note,
            bridge::grid_read_replies,
            bridge::archive_replies,
            crate::capabilities::git::commands::git_log,
            crate::capabilities::git::commands::git_commit_diff,
            crate::capabilities::git::commands::git_local_context,
            crate::capabilities::git::commands::git_local_log,
            crate::capabilities::git::commands::git_local_diff,
            crate::capabilities::git::commands::git_local_commit_diff,
            crate::capabilities::git::local_history::git_local_history,
            crate::capabilities::git::local_history::git_local_history_diff,
            crate::capabilities::git::local_workbench::git_local_inspect,
            crate::capabilities::git::local_workbench::git_local_write_request,
            crate::capabilities::git::local_workbench::git_local_write_confirm,
            crate::capabilities::git::local_workbench::git_local_conflict,
            crate::capabilities::git::local_workbench::git_local_github,
            crate::capabilities::git::local_advanced::git_local_blame,
            crate::capabilities::git::local_advanced::git_local_follow,
            bridge::mcp_capability_preview,
            bridge::mcp_policy_get,
            bridge::mcp_registry_list,
            // M5-W12（Lane A7）：图谱只读 live-query 命令（见 bridge.rs）。
            crate::capabilities::graph::commands::graph_query,
            crate::capabilities::graph::commands::graph_node_get,
            crate::capabilities::graph::commands::graph_stats,
            crate::capabilities::graph::commands::graph_import_json,
            crate::capabilities::graph::commands::graph_rebuild_documents,
            crate::capabilities::graph::commands::graph_sync_vault,
            // M5-W13（Lane A9）：插件 manifest 生命周期 Stage-I（仅本地状态，无执行）。
            bridge::plugin_install,
            bridge::plugin_enable,
            bridge::plugin_disable,
            bridge::plugin_list,
            bridge::plugin_get,
            bridge::plugin_keys_add,
            bridge::plugin_keys_list,
            bridge::plugin_keys_remove,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            startup::report_failure(&e);
            std::process::exit(1);
        })
        .run(|app, event| {
            match &event {
                // M1-4：macOS/iOS/Android 经 RunEvent::Opened 投递打开请求
                // （该变体仅在这些平台编译；Linux 主走 argv + 单实例插件）。
                #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
                tauri::RunEvent::Opened { urls } => {
                    for u in urls {
                        startup::dispatch_open_url(app, u.as_str());
                    }
                }
                tauri::RunEvent::ExitRequested { api, .. } => {
                    if !app.state::<ExitLifecycleState>().completed() {
                        api.prevent_exit();
                        begin_graceful_shutdown(app.clone(), "exit_requested");
                    }
                }
                _ => {}
            }
        });
}

#[cfg(test)]
mod graceful_exit_state_tests {
    use super::*;

    #[test]
    fn only_one_cleanup_worker_claims_close_and_exit_events() {
        let exit = ExitLifecycleState::default();
        assert!(exit.claim());
        assert!(!exit.claim());
        assert!(!exit.completed());
        exit.completed
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(exit.completed());
        assert!(!exit.claim());
    }

    #[test]
    fn concurrent_close_callbacks_do_not_spawn_duplicate_cleanup() {
        let exit = std::sync::Arc::new(ExitLifecycleState::default());
        let wins = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let handles: Vec<_> = (0..24)
            .map(|_| {
                let exit = exit.clone();
                let wins = wins.clone();
                std::thread::spawn(move || {
                    if exit.claim() {
                        wins.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(wins.load(std::sync::atomic::Ordering::Acquire), 1);
    }

    #[test]
    fn rejected_exit_waits_for_owned_worker_before_retry() {
        use std::sync::{mpsc, Arc};
        use std::time::{Duration, Instant};
        let exit = Arc::new(ExitLifecycleState::default());
        assert!(exit.claim());
        let (cancelled_tx, cancelled_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let owner = Arc::clone(&exit);
        let worker = std::thread::spawn(move || {
            owner
                .started
                .store(false, std::sync::atomic::Ordering::Release);
            cancelled_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        });
        *exit.worker.lock().unwrap() = Some(worker);
        cancelled_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            !exit.claim(),
            "a cancelled but live worker still owns the exit attempt"
        );
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            exit.reap_finished_worker();
            if exit.worker.lock().unwrap().is_none() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "worker must finish before a retry"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!exit.completed());
        assert!(
            exit.claim(),
            "failed saving must not permanently close the application"
        );
    }
}
