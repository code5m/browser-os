#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod crashlog;
mod domain;
mod grid_ipc;
mod grid_process;
mod images;
mod keyring_store;
mod script_runner;
mod scripts;
mod security_policy;
mod session;
mod shutdown;
mod sync;
mod workspace;

use bridge::AppState;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn log_shutdown_report(report: &shutdown::ShutdownReport) {
    eprintln!(
        "[shutdown] completed ok={} already_shutdown={} executed={}",
        report.ok(),
        report.already_shutdown,
        report.executed()
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
fn run_grid_child(index: u32) {
    eprintln!(
        "[grid-child-{}] starting (pid={})",
        index,
        std::process::id()
    );
    crashlog::init();
    if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    std::env::set_var("GDK_BACKEND", "x11");
    if std::env::var("GTK_USE_PORTAL").is_err() {
        std::env::set_var("GTK_USE_PORTAL", "0");
    }

    let label = format!("grid-child-{}", index);
    let sock_path = std::env::var("GRID_SOCK_PATH").unwrap_or_default();
    let label_for_plugin = label.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_browser_tabs::init_with_host(&label_for_plugin))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            bridge::collect_selection,
            bridge::save_note,
            bridge::request_open_terminal,
            bridge::report_grid_load_failed,
            bridge::debug_log,
        ])
        .setup(move |app| {
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("[grid-child-{}] RUN_ERROR: {e}", index);
            std::process::exit(1);
        });
}

/// 子进程全局 UDS 写端（connect 成功后设置；窗口事件回调里取用）。
static CHILD_WRITER: std::sync::OnceLock<
    std::sync::Arc<std::sync::Mutex<std::os::unix::net::UnixStream>>,
> = std::sync::OnceLock::new();

/// 子进程 → 主进程 发异步事件（焦点上报 / 导航 / 新窗口 / 桥命令事件转发）。
fn grid_child_send_event(name: &str, payload: serde_json::Value) {
    if let Some(w) = CHILD_WRITER.get() {
        let mut guard = w.lock().unwrap();
        let _ = grid_ipc::write_wire(
            &mut *guard,
            &grid_ipc::Wire::Event {
                name: name.to_string(),
                payload,
            },
        );
    }
}

/// 子进程 UDS client：连接主进程 socket（重试至 15s）→ 注册事件转发 → 命令循环。
fn start_grid_child_ipc(app: tauri::AppHandle, index: u32, host_label: String, sock_path: String) {
    use tauri::Listener;
    std::thread::spawn(move || {
        use std::os::unix::net::UnixStream;
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
        eprintln!("[grid-child-{}] UDS 已连接 {}", index, sock_path);
        let writer = match stream.try_clone() {
            Ok(w) => std::sync::Arc::new(std::sync::Mutex::new(w)),
            Err(e) => {
                eprintln!("[grid-child-{}] try_clone 失败: {e}", index);
                std::process::exit(1);
            }
        };
        let _ = CHILD_WRITER.set(writer.clone());

        // 插件事件（宫格 webview 内导航完成 / window.open）→ 主进程。
        {
            let w = writer.clone();
            app.listen("browser-tabs://event", move |event| {
                let payload: serde_json::Value = match serde_json::from_str(event.payload()) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                match payload.get("type").and_then(|t| t.as_str()) {
                    Some("newWindowRequested") => {
                        if let Some(url) = payload.get("url").and_then(|u| u.as_str()) {
                            let _ = grid_ipc::write_wire(
                                &mut *w.lock().unwrap(),
                                &grid_ipc::Wire::Event {
                                    name: "new-tab-request".to_string(),
                                    payload: serde_json::json!({ "url": url }),
                                },
                            );
                        }
                    }
                    Some("navigationFinished") => {
                        let id = payload.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        let _ = grid_ipc::write_wire(
                            &mut *w.lock().unwrap(),
                            &grid_ipc::Wire::Event {
                                name: "tab-navigated".to_string(),
                                payload: serde_json::json!({ "id": id, "url": url }),
                            },
                        );
                    }
                    // 原生 load-failed（TLS/网络错误页，JS 无法感知）→ 主进程自动重试
                    Some("loadFailed") => {
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        let err = payload.get("error").and_then(|e| e.as_str()).unwrap_or("");
                        eprintln!(
                            "[grid-child-{}] loadFailed 转发 url={} err={}",
                            index, url, err
                        );
                        let _ = grid_ipc::write_wire(
                            &mut *w.lock().unwrap(),
                            &grid_ipc::Wire::Event {
                                name: "grid-load-failed".to_string(),
                                payload: serde_json::json!({
                                    "index": index.to_string(),
                                    "url": url,
                                    "error": err,
                                }),
                            },
                        );
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
            let w = writer.clone();
            let name_owned = name.to_string();
            app.listen(name, move |event| {
                let payload: serde_json::Value =
                    serde_json::from_str(event.payload()).unwrap_or(serde_json::Value::Null);
                let _ = grid_ipc::write_wire(
                    &mut *w.lock().unwrap(),
                    &grid_ipc::Wire::Event {
                        name: name_owned.clone(),
                        payload,
                    },
                );
            });
        }

        // 命令循环：Request → 本地执行 → Response。主进程断开（EOF/错误）→ 退出，
        // 避免主进程退出后残留置顶孤儿窗口。
        let mut reader = std::io::BufReader::new(stream);
        loop {
            match grid_ipc::read_wire(&mut reader) {
                Ok(Some(grid_ipc::Wire::Request { seq, cmd })) => {
                    let resp = match dispatch_grid_cmd(&app, &host_label, cmd) {
                        Ok(()) => grid_ipc::Wire::ok(seq),
                        Err(e) => grid_ipc::Wire::err(seq, e),
                    };
                    if grid_ipc::write_wire(&mut *writer.lock().unwrap(), &resp).is_err() {
                        break;
                    }
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
fn dispatch_grid_cmd(
    app: &tauri::AppHandle,
    host_label: &str,
    cmd: grid_ipc::GridCmd,
) -> Result<(), String> {
    use grid_ipc::GridCmd;
    use tauri_plugin_browser_tabs::{CreateTabOptions, LogicalRect, TabManagerState};
    match cmd {
        GridCmd::CreateTab { id, url } => {
            let init_script = include_str!("../injected/collect.js");
            let manager = app.state::<TabManagerState>();
            match manager.create_tab(CreateTabOptions {
                id: id.clone(),
                url,
                // 1x1 占位 + auto_resize：子窗口尺寸由主进程 UpdateRect 控制，
                // webview 自动跟随窗口铺满。
                rect: LogicalRect::new(0.0, 0.0, 1.0, 1.0),
                visible: true,
                auto_resize: true,
                user_agent: None,
                transparent: false,
                initialization_script: Some(init_script.to_string()),
            }) {
                Ok(()) => {
                    eprintln!("[grid-child] CreateTab {} 完成", id);
                    Ok(())
                }
                // 幂等：重试/重放导致的重复创建视为成功
                Err(tauri_plugin_browser_tabs::BrowserTabError::TabAlreadyExists(_)) => Ok(()),
                Err(e) => Err(format!("CreateTab {id} 失败: {e}")),
            }
        }
        GridCmd::UpdateRect { id, rect } => {
            // 注意用 get_window 而非 get_webview_window：后者在本场景返回 None
            // （实测），插件 host_window() 同样走 get_window。窗口操作 Window 都有。
            let win = app
                .get_window(host_label)
                .ok_or_else(|| "子进程壳窗口不存在".to_string())?;
            win.set_position(tauri::PhysicalPosition::new(rect.x as i32, rect.y as i32))
                .map_err(|e| format!("set_position 失败: {e}"))?;
            win.set_size(tauri::PhysicalSize::new(
                (rect.w as u32).max(1),
                (rect.h as u32).max(1),
            ))
            .map_err(|e| format!("set_size 失败: {e}"))?;
            // 关键：显式把宫格 webview 铺满壳窗口。创建时是 1x1 + auto_resize，
            // 但 GDK_BACKEND=x11 下 auto_resize 不可靠（WebKitGTK size_allocate
            // 老问题），webview 会保持小尺寸/漂移（实测"大白壳+小内容"）。
            // rect 是屏幕绝对物理坐标，webview 用相对壳窗口的逻辑坐标 (0,0,w,h)。
            let scale = win.scale_factor().unwrap_or(1.0);
            let lw = (rect.w / scale).max(1.0);
            let lh = (rect.h / scale).max(1.0);
            {
                use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
                let manager = app.state::<TabManagerState>();
                let _ = manager.update_rect(&id, LogicalRect::new(0.0, 0.0, lw, lh));
            }
            // 记录到 child_layouts：子进程的 layout enforcer 每 400ms 重放纠偏
            // （GTK 布局循环会把子 webview 拉回"自然位置"，与主进程页签同款问题）
            app.state::<AppState>()
                .child_layouts
                .lock()
                .unwrap()
                .insert(id, (0.0, 0.0, lw, lh));
            win.show().map_err(|e| format!("show 失败: {e}"))?;
            Ok(())
        }
        GridCmd::HideWindow { .. } => {
            let win = app
                .get_window(host_label)
                .ok_or_else(|| "子进程壳窗口不存在".to_string())?;
            win.hide().map_err(|e| format!("hide 失败: {e}"))?;
            Ok(())
        }
        GridCmd::Eval { id, js } => {
            let manager = app.state::<TabManagerState>();
            manager
                .eval(&id, &js)
                .map_err(|e| format!("eval {id} 失败: {e}"))
        }
        GridCmd::Navigate { id, url } => {
            let manager = app.state::<TabManagerState>();
            manager
                .navigate(&id, &url)
                .map_err(|e| format!("navigate {id} 失败: {e}"))
        }
        GridCmd::SetZoom { id, zoom } => {
            let manager = app.state::<TabManagerState>();
            manager
                .set_zoom(&id, zoom)
                .map_err(|e| format!("set_zoom {id} 失败: {e}"))
        }
        GridCmd::CloseTab { id } => {
            app.state::<AppState>()
                .child_layouts
                .lock()
                .unwrap()
                .remove(&id);
            let manager = app.state::<TabManagerState>();
            match manager.close_tab(&id) {
                Ok(()) => Ok(()),
                Err(tauri_plugin_browser_tabs::BrowserTabError::TabNotFound(_)) => Ok(()),
                Err(e) => Err(format!("close_tab {id} 失败: {e}")),
            }
        }
        GridCmd::Ping => Ok(()),
    }
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
                .open("/tmp/grid-selftest-result.txt")
                .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{m}\n").as_bytes()));
        };
        let _ = std::fs::remove_file("/tmp/grid-selftest-result.txt");
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
            bridge::create_grid(app.clone(), 2).map(|_| ()),
        );
        // 2) 导航 + 定位 + eval
        step(
            "grid_open(0)",
            bridge::grid_open(app.clone(), 0, "https://www.baidu.com".into()),
        );
        step(
            "grid_open(1)",
            bridge::grid_open(app.clone(), 1, "https://www.bing.com".into()),
        );
        step(
            "grid_position(0)",
            bridge::grid_position(app.clone(), 0, 40.0, 120.0, 560.0, 420.0),
        );
        step(
            "grid_position(1)",
            bridge::grid_position(app.clone(), 1, 640.0, 120.0, 560.0, 420.0),
        );
        step(
            "eval_in_tab(grid-0)",
            bridge::eval_in_tab(app.clone(), "grid-0".into(), "document.title".into()).map(|_| ()),
        );
        // 3) kill -11 崩溃 grid-0 → 自动重启 + 重放
        let pid0 = app.state::<AppState>().grid_manager.pid_of(0);
        match pid0 {
            Some(pid) => {
                log(&format!("kill -11 grid-child-0 pid={pid}"));
                unsafe { libc::kill(pid as i32, libc::SIGSEGV) };
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
            bridge::eval_in_tab(app.clone(), "grid-0".into(), "1+1".into()).map(|_| ()),
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
        step("close_grid", bridge::close_grid(app.clone()));
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

fn gui_regression_eval(case_id: &str, grid: usize, phase: &str) -> String {
    format!(
        r#"(function(){{
  var params = new URLSearchParams({{
    case: "{case_id}",
    grid: "{grid}",
    phase: "{phase}",
    title: document.title || "",
    cookie: document.cookie || "",
    bodyText: (document.body && document.body.innerText || "").slice(0, 80)
  }});
  return fetch("/event?" + params.toString(), {{ credentials: "include" }}).then(function(){{ return "ok"; }}).catch(function(e){{ return String(e); }});
}})();"#
    )
}

fn gui_regression_menu_eval(case_id: &str, grid: usize, phase: &str) -> String {
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
        let report_path = std::env::var("M0_GUI_REGRESSION_REPORT")
            .unwrap_or_else(|_| "/tmp/m0-6c-gui-regression.json".to_string());
        let mock_base = std::env::var("M0_GUI_MOCK_BASE").unwrap_or_default();
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
            bridge::grid_position(app.clone(), index, x, y, w, h)
                .map(|_| format!("grid-{index} positioned"))
        };
        let open_grid = |case_id: &str, index: usize| {
            bridge::grid_open(app.clone(), index, gui_mock_url(&mock_base, case_id, index))
                .map(|_| format!("grid-{index} opened case={case_id}"))
        };
        let eval_grid = |case_id: &str, index: usize, phase: &str| {
            bridge::eval_in_tab(
                app.clone(),
                format!("grid-{index}"),
                gui_regression_eval(case_id, index, phase),
            )
            .map(|_| format!("grid-{index} eval case={case_id} phase={phase}"))
        };

        sleep_ms(3000);
        if mock_base.is_empty() {
            step("preflight", Err("M0_GUI_MOCK_BASE is empty".to_string()));
        } else {
            step("preflight", Ok(format!("mock_base={mock_base}")));
        }

        step(
            "scenario-1-single-ai-broadcast",
            bridge::create_grid(app.clone(), 4).and_then(|created| {
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
                for i in 0..4 {
                    eval_grid("2", i, "concurrent_ai")?;
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
                let tab = bridge::tab_new(app.clone(), gui_mock_url(&mock_base, "5", 0))?;
                bridge::tab_activate(app.clone(), tab.id.clone())?;
                bridge::tab_position(app.clone(), tab.id.clone(), 60.0, 120.0, 800.0, 520.0)?;
                bridge::hide_all_webviews(app.clone())?;
                sleep_ms(800);
                position_grid(0, 40.0, 110.0, 620.0, 450.0)?;
                sleep_ms(800);
                eval_grid("5", 0, "after_view_switch")?;
                bridge::tab_close(app.clone(), tab.id)?;
                Ok("tab view and grid view switched without orphaned child windows".to_string())
            })(),
        );

        step(
            "scenario-6-normal-tabs-multi-open",
            (|| {
                open_grid("6", 0)?;
                sleep_ms(1000);
                let first = bridge::tab_new(app.clone(), gui_mock_url(&mock_base, "6", 0))?;
                let second = bridge::tab_new(app.clone(), gui_mock_url(&mock_base, "6", 1))?;
                bridge::tab_activate(app.clone(), first.id.clone())?;
                bridge::tab_position(app.clone(), first.id.clone(), 80.0, 120.0, 760.0, 500.0)?;
                bridge::tab_activate(app.clone(), second.id.clone())?;
                bridge::tab_position(app.clone(), second.id.clone(), 90.0, 130.0, 760.0, 500.0)?;
                bridge::tab_close(app.clone(), first.id)?;
                bridge::tab_close(app.clone(), second.id)?;
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
                unsafe { libc::kill(old as i32, libc::SIGSEGV) };
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
                let term = bridge::term_spawn(app.clone())?;
                bridge::term_write(
                    app.clone(),
                    term.id.clone(),
                    "printf 'M0_6C_TERMINAL_OK\\n'\n".to_string(),
                )?;
                sleep_ms(800);
                bridge::term_kill(app.clone(), term.id)?;
                let stats = bridge::resource_stats(app.clone());
                bridge::eval_in_tab(
                    app.clone(),
                    "grid-0".to_string(),
                    gui_regression_menu_eval("8", 0, "context_menu"),
                )?;
                sleep_ms(800);
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
                sleep_ms(800);
                let old = app
                    .state::<AppState>()
                    .grid_manager
                    .pid_of(0)
                    .ok_or_else(|| "grid-0 pid missing before login-state restart".to_string())?;
                unsafe { libc::kill(old as i32, libc::SIGSEGV) };
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
            bridge::close_grid(app.clone()).map(|_| {
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
        let report_dir = cfg
            .report_dir
            .clone()
            .unwrap_or_else(|| "/tmp/mvp-browser-os-m0".to_string());
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
                "tab" => bridge::tab_new(app.clone(), "about:blank".into())
                    .map(|tab| OpenedResource::Tab(tab.id)),
                "grid" => match bridge::create_grid(app.clone(), 4) {
                    Ok(4) => Ok(OpenedResource::Grid),
                    Ok(created) => {
                        let _ = bridge::close_grid(app.clone());
                        Err(format!("grid degraded to {created} (need 4)"))
                    }
                    Err(error) => Err(error),
                },
                "terminal" => bridge::term_spawn(app.clone()).map(|term| {
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
                Ok(OpenedResource::Tab(id)) => bridge::tab_close(app.clone(), id),
                Ok(OpenedResource::Grid) => bridge::close_grid(app.clone()),
                Ok(OpenedResource::Terminal(id)) => bridge::term_kill(app.clone(), id),
                Err(error) => Err(error),
            };
            if cfg.driver == "tab" && close_result.is_ok() {
                if let Err(error) = bridge::close_grid(app.clone()) {
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
    // Phase 0：宫格子进程分支（在最前判断，避免初始化主进程逻辑）
    if let Some(index) = parse_grid_child_arg() {
        run_grid_child(index);
        return;
    }

    // 日志/崩溃捕获必须最先初始化（stderr 镜像 + panic 钩子 + 致命信号捕获）
    crashlog::init();
    // Workaround：WebKitGTK 在 Wayland 下默认启用 DMA-BUF 渲染器会静默崩溃
    // （进程启动数秒后退出、窗口白屏）。禁用后可稳定运行。
    if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    // 用 X11 后端规避 Tauri v2 在 Wayland 下子 webview（浏览器/终端）定位错位的已知 bug。
    std::env::set_var("GDK_BACKEND", "x11");
    // 禁用 GTK portal，避免初始化时等待 org.freedesktop.portal.* 超时导致窗口卡住/不显示。
    if std::env::var("GTK_USE_PORTAL").is_err() {
        std::env::set_var("GTK_USE_PORTAL", "0");
    }

    tauri::Builder::default()
        // 浏览器页签/宫格子 webview 统一由 browser-tabs 插件创建与定位
        .plugin(tauri_plugin_browser_tabs::init())
        // M1-4：单实例——应用已运行时再次 xdg-open（第二实例）把 argv 中的
        // http(s) URL 路由给首个实例的内嵌页签，而不是另起一个主窗口。
        // grid 子进程走 run_grid_child 的独立 Builder（无本插件），不受影响。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            for u in bridge::extract_open_urls(&argv) {
                bridge::handle_open_url(app, &u);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            // M1-4 冷启动：应用未运行时 xdg-open 经 desktop 文件 %u 把 URL 放进
            // 本进程 argv。此时前端未就绪，统一进 pending 队列（handle_open_url
            // 内部就绪前不发提示），待 m0_ready 后由前端拉取打开，保证不丢。
            let argv: Vec<String> = std::env::args().collect();
            for u in bridge::extract_open_urls(&argv) {
                bridge::handle_open_url(app.handle(), &u);
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
                            eprintln!("[main] forward new-tab-request url={}", url);
                            let _ =
                                forward.emit("new-tab-request", serde_json::json!({ "url": url }));
                        }
                    }
                    // 子 webview 内导航完成（点链接/前进/后退/刷新），转发给前端同步地址栏
                    Some("navigationFinished") => {
                        let id = payload.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        eprintln!("[main] emit tab-navigated id={} url={}", id, url);
                        let _ = forward
                            .emit("tab-navigated", serde_json::json!({ "id": id, "url": url }));
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
            // 创建的，config 的 devUrl 解析未生效（webview 一直加载旧 dist，前端改动
            // 全部不生效）。release 仍走 App("index.html") 打包包内资源。
            // MVP_FORCE_DIST=1：强制加载内嵌 dist（白屏二分诊断用，绕开 vite）
            let main_url = if cfg!(debug_assertions) && std::env::var("MVP_FORCE_DIST").is_err() {
                WebviewUrl::External("http://localhost:1421".parse().unwrap())
            } else {
                WebviewUrl::App("index.html".into())
            };
            let window = WebviewWindowBuilder::new(app, "main", main_url)
                .title("浏览器OS融合")
                .inner_size(1200.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .maximized(true)
                .build()?;
            eprintln!("[main] main window url={:?}", window.url());
            let _ = window.show();
            let _ = window.set_focus();
            // 修正主 UI webview 的 GTK allocation / CSS 视口（WebKitGTK 偶发卡在小尺寸，
            // 导致顶部 TitleBar/ActivityBar 等被挤出可视区）。scale=1 时等效无操作。
            if let Some(main_wv) = app.get_webview("main") {
                let _ = tauri_plugin_browser_tabs::ensure_native_layout(&main_wv);
            }
            let _ = std::fs::write("/tmp/mvp-life.log", "main-window-created-and-shown\n");
            // 宫格子进程管理器：注入 AppHandle（事件转发/定位换算/崩溃自愈）
            app.state::<AppState>()
                .grid_manager
                .set_app(app.handle().clone());
            bridge::register_shutdown_tasks(app.handle()).map_err(std::io::Error::other)?;
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
                            state.grid_manager.reposition_visible();
                        }
                        WindowEvent::Focused(false) => {
                            // 延迟判定：点击宫格子窗口同样触发主窗 blur，
                            // 150ms 内等子进程焦点上报到达后再决定（睡必须离开事件回调）
                            let h = handle.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(150));
                                h.state::<AppState>()
                                    .grid_manager
                                    .hide_for_blur_if_no_child_focus();
                            });
                        }
                        WindowEvent::Focused(true) => {
                            state.grid_manager.show_for_focus();
                        }
                        WindowEvent::CloseRequested { .. } => {
                            let report = handle.state::<shutdown::ShutdownCoordinator>().shutdown();
                            log_shutdown_report(&report);
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
            // 宫格加载失败自动重试（collect.js 上报错误页 → 重新导航）
            bridge::start_grid_load_retry(app.handle().clone());
            // GRID_SELFTEST=1：宫格多进程端到端自检（Phase 2 UDS 转发 + Phase 3 崩溃自愈）。
            // 跑完写 /tmp/grid-selftest-result.txt 并退出。日常运行不设该变量即可。
            if std::env::var("GRID_SELFTEST").is_ok() {
                run_grid_selftest(app.handle().clone());
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
            Ok(())
        })
        .manage(AppState::default())
        // M0-2.b：注入统一生命周期核心。本检查点只提供核心，不迁移任何退出路径
        // （M0-2.c 负责把窗口关闭/菜单退出/系统退出与 grid/PTY/tab/线程调用方切到它）。
        .manage(shutdown::ShutdownCoordinator::new())
        // M0-3.b：一次性用户意图令牌登记表（外部页面发起副作用调用时校验）。
        .manage(security_policy::IntentRegistry::new())
        .invoke_handler(tauri::generate_handler![
            bridge::open_browser,
            bridge::close_browser,
            bridge::position_browser,
            bridge::report_resources,
            bridge::report_grid_load_failed,
            bridge::report_title,
            bridge::collect_selection,
            bridge::request_open_terminal,
            bridge::save_note,
            bridge::list_artifacts,
            bridge::configure_repo,
            bridge::list_repos,
            bridge::request_sync,
            bridge::confirm_sync,
            bridge::audit_log,
            bridge::read_artifact,
            bridge::update_artifact,
            bridge::delete_artifact,
            bridge::browse_workspace,
            bridge::add_bookmark,
            bridge::list_bookmarks,
            bridge::remove_bookmark,
            bridge::list_dir,
            bridge::read_file,
            bridge::write_file,
            bridge::get_start_dirs,
            bridge::reveal_artifact,
            bridge::open_source,
            bridge::create_file,
            bridge::create_dir,
            bridge::delete_path,
            bridge::rename_path,
            bridge::clipboard_read,
            bridge::clipboard_write,
            bridge::create_grid,
            bridge::close_grid,
            bridge::grid_open,
            bridge::grid_position,
            bridge::grid_set_zoom,
            bridge::grid_close_one,
            bridge::hide_all_webviews,
            bridge::hide_webview,
            bridge::debug_log,
            bridge::list_apps,
            bridge::launch_app,
            bridge::tab_new,
            bridge::tab_close,
            bridge::tab_open,
            bridge::tab_position,
            bridge::tab_list,
            bridge::tab_set_title,
            bridge::tab_activate,
            bridge::tab_go_back,
            bridge::tab_go_forward,
            bridge::tab_reload,
            bridge::eval_in_tab,
            bridge::set_tab_hibernation,
            bridge::resource_stats,
            bridge::term_spawn,
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
            bridge::git_status,
            bridge::git_diff,
            bridge::git_branch_list,
            bridge::request_git_write,
            bridge::confirm_git_write,
            bridge::list_tab_resources,
            bridge::clear_tab_resources,
            bridge::get_resource_capture_settings,
            bridge::set_resource_capture_settings,
            bridge::session_save,
            bridge::session_discard,
            bridge::session_list,
            bridge::session_get,
            bridge::session_delete,
            bridge::session_export,
            bridge::session_restore,
            bridge::flush_sessions,
            bridge::get_session_policy,
            bridge::set_session_policy,
            bridge::save_image,
            bridge::list_artifact_images,
            bridge::workspace_images_dir,
            bridge::script_list,
            bridge::script_add,
            bridge::script_update,
            bridge::script_remove,
            bridge::run_script,
            bridge::cancel_script,
            bridge::script_status,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            let _ = std::fs::write("/tmp/mvp-life.log", format!("RUN_ERROR: {e}\n"));
            std::process::exit(1);
        })
        .run(|app, event| {
            match &event {
                // M1-4：macOS/iOS/Android 经 RunEvent::Opened 投递打开请求
                // （该变体仅在这些平台编译；Linux 主走 argv + 单实例插件）。
                #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
                tauri::RunEvent::Opened { urls } => {
                    for u in urls {
                        bridge::handle_open_url(app, u.as_str());
                    }
                }
                tauri::RunEvent::ExitRequested { .. } => {
                    let report = app.state::<shutdown::ShutdownCoordinator>().shutdown();
                    log_shutdown_report(&report);
                }
                _ => {}
            }
        });
}
