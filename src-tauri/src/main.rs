#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod domain;
mod bridge;
mod workspace;
mod keyring_store;
mod sync;
mod crashlog;
mod grid_ipc;
mod grid_process;

use bridge::AppState;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

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
    eprintln!("[grid-child-{}] starting (pid={})", index, std::process::id());
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
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_browser_tabs::init_with_host(&label_for_plugin))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            bridge::collect_selection,
            bridge::save_note,
            bridge::request_open_terminal,
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
            start_grid_child_ipc(app.handle().clone(), index, label.clone(), sock_path.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("[grid-child-{}] RUN_ERROR: {e}", index);
            std::process::exit(1);
        });
}

/// 子进程全局 UDS 写端（connect 成功后设置；窗口事件回调里取用）。
static CHILD_WRITER: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<std::os::unix::net::UnixStream>>> =
    std::sync::OnceLock::new();

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
fn start_grid_child_ipc(
    app: tauri::AppHandle,
    index: u32,
    host_label: String,
    sock_path: String,
) {
    use tauri::Listener;
    std::thread::spawn(move || {
        use std::os::unix::net::UnixStream;
        let start = std::time::Instant::now();
        let stream = loop {
            match UnixStream::connect(&sock_path) {
                Ok(s) => break s,
                Err(e) => {
                    if start.elapsed().as_secs() > 15 {
                        eprintln!("[grid-child-{}] connect {} 失败: {e}，退出", index, sock_path);
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
                    _ => {}
                }
            });
        }
        // 桥命令在子进程内 emit 的事件（采集完成/笔记保存/打开终端）→ 主进程 → 前端。
        for name in ["open-terminal", "note-saved", "artifact-collected"] {
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
        GridCmd::UpdateRect { rect, .. } => {
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
            manager.eval(&id, &js).map_err(|e| format!("eval {id} 失败: {e}"))
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
        let mut log = |m: &str| {
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
        let step = |name: &str, r: Result<(), String>| {
            match r {
                Ok(()) => log(&format!("PASS {name}")),
                Err(e) => {
                    fails.borrow_mut().push(format!("{name}: {e}"));
                    log(&format!("FAIL {name}: {e}"));
                }
            }
        };
        // 1) 创建 2 宫格（spawn 2 子进程 + UDS CreateTab）
        step("create_grid(2)", bridge::create_grid(app.clone(), 2).map(|_| ()));
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
            (Some(old), Some(new)) if new != old => {
                log(&format!("PASS crash-restart grid-0 重启 pid {old} -> {new}"))
            }
            _ => {
                fails.borrow_mut().push(format!("grid-0 未重启: old={:?} new={:?}", pid0, pid0_new));
                log(&format!("FAIL crash-restart old={:?} new={:?}", pid0, pid0_new));
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
            fails.borrow_mut().push(format!("shutdown 后 count={count}"));
            log(&format!("FAIL shutdown count={count}"));
        }
        if fails.borrow().is_empty() {
            log("SELFTEST_RESULT=ALL_PASS");
        } else {
            log(&format!("SELFTEST_RESULT=FAIL ({})", fails.borrow().join(" | ")));
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        std::process::exit(if fails.borrow().is_empty() { 0 } else { 1 });
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
        .plugin(tauri_plugin_shell::init())
        // 浏览器页签/宫格子 webview 统一由 browser-tabs 插件创建与定位
        .plugin(tauri_plugin_browser_tabs::init())
        .setup(|app| {
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
                            let _ = forward.emit(
                                "new-tab-request",
                                serde_json::json!({ "url": url }),
                            );
                        }
                    }
                    // 子 webview 内导航完成（点链接/前进/后退/刷新），转发给前端同步地址栏
                    Some("navigationFinished") => {
                        let id = payload.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let url = payload.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        eprintln!("[main] emit tab-navigated id={} url={}", id, url);
                        let _ = forward.emit(
                            "tab-navigated",
                            serde_json::json!({ "id": id, "url": url }),
                        );
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
                            state.grid_manager.shutdown_all();
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
            // GRID_SELFTEST=1：宫格多进程端到端自检（Phase 2 UDS 转发 + Phase 3 崩溃自愈）。
            // 跑完写 /tmp/grid-selftest-result.txt 并退出。日常运行不设该变量即可。
            if std::env::var("GRID_SELFTEST").is_ok() {
                run_grid_selftest(app.handle().clone());
            }
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            bridge::open_browser,
            bridge::close_browser,
            bridge::position_browser,
            bridge::report_resources,
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
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            let _ = std::fs::write("/tmp/mvp-life.log", format!("RUN_ERROR: {e}\n"));
            std::process::exit(1);
        });
}
