#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod domain;
mod bridge;
mod workspace;
mod keyring_store;
mod sync;

use bridge::AppState;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn main() {
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
            let main_url = if cfg!(debug_assertions) {
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
            // 启动布局守护线程：持续纠正 GTK 布局循环导致的子 webview 位置漂移
            bridge::start_layout_enforcer(app.handle().clone());
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
