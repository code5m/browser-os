use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Listener, Manager, Webview};
use url::Url;

use crate::domain::*;
use crate::keyring_store::KeyringStore;
use crate::sync;
use crate::workspace;

// 方案 B：每个浏览器页签/宫格都是一个独立的 Tauri WebviewWindow（无装饰、透明、置顶、
// 作为主窗口子窗口），靠屏幕坐标定位到主窗内容区的指定矩形。相比方案 C3 的
// gtk::Overlay + gtk::Fixed 内嵌，子窗口是 WebKit 原生顶级窗口，渲染稳定、不再白屏。
// WebviewWindow 是 Send+Sync，可直接放在 AppState 的 Mutex 里，无需线程本地存储。
thread_local! {
    // 防止重复注册主窗 move/resize 监听
    static MOVE_LISTENED: RefCell<bool> = RefCell::new(false);
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

/// 注册主窗 move/resize 监听（仅一次），触发时把内容区坐标重新换算并应用到所有子窗口。
/// 子窗口的定位信息存在 AppState.child_layouts（id -> 内容区 CSS 矩形）。
/// 注意：改用 add_child 后，子 webview 自动跟随主窗口移动，这里仅用于窗口 Resized 时
/// 重新计算子 webview 尺寸（前端也会在 resize 时重新调用 tab_position，这里是兜底）。
#[allow(dead_code)]
fn ensure_reposition_listener(app: &AppHandle) {
    if MOVE_LISTENED.with(|m| *m.borrow()) {
        return;
    }
    MOVE_LISTENED.with(|m| *m.borrow_mut() = true);
    let app_clone = app.clone();
    // 用 main 窗口的 on_window_event 捕获 Moved/Resized，重定位全部子窗口
    if let Some(main) = app.get_window("main") {
        let app_ev = app_clone.clone();
        main.on_window_event(move |event| {
            use tauri::WindowEvent;
            if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
                // on_window_event 本身在主线程事件循环中，直接调用即可，
                // 不要再 run_on_main_thread，避免嵌套调度死锁。
                reposition_all(&app_ev);
            }
        });
    }
    // 主窗失焦时隐藏子窗口（避免"幽灵浮层"），重新聚焦时恢复定位
    let app_blur = app.clone();
    let _ = {
        let captured = app_blur.clone();
        app_blur.listen("tauri://blur", move |_| {
            // listen 回调也在主线程，直接执行
            hide_all_children(&captured);
        })
    };
    let app_focus = app.clone();
    let _ = {
        let captured = app_focus.clone();
        app_focus.listen("tauri://focus", move |_| {
            // listen 回调也在主线程，直接执行
            reposition_all(&captured);
        })
    };
}

/// 按当前主窗内容区坐标重定位所有子 webview（add_child 的子 webview 自动跟随父窗口移动，
/// 这里只在主窗 Resized 时更新尺寸；position 用相对主窗口内容区的逻辑坐标）。
#[allow(dead_code)]
fn reposition_all(app: &AppHandle) {
    use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
    let layouts = app.state::<AppState>().child_layouts.lock().unwrap().clone();
    let manager = app.state::<TabManagerState>();
    for (id, r) in layouts {
        // child_layouts 存的是相对主窗内容区的 CSS 逻辑坐标
        let _ = manager.update_rect(&id, LogicalRect::new(r.0, r.1, r.2.max(1.0), r.3.max(1.0)));
    }
}

/// 隐藏全部子窗口（主窗失焦时调用），避免浮在主窗之外的幽灵窗口。
#[allow(dead_code)]
fn hide_all_children(app: &AppHandle) {
    use tauri_plugin_browser_tabs::TabManagerState;
    let ids: Vec<String> = app
        .state::<AppState>()
        .child_layouts
        .lock()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let manager = app.state::<TabManagerState>();
    for id in ids {
        let _ = manager.set_visible(&id, false);
    }
}

/// 记住某个子窗口的内容区布局矩形（CSS 坐标），供 move/resize 时重定位。
fn remember_layout(app: &AppHandle, id: &str, x: f64, y: f64, w: f64, h: f64) {
    app.state::<AppState>()
        .child_layouts
        .lock()
        .unwrap()
        .insert(id.to_string(), (x, y, w, h));
}

/// 桥的进程级状态：仅保存待确认任务（凭据/审计都在磁盘，避免内存泄漏）
#[derive(Default)]
pub struct AppState {
    pub pending_jobs: Mutex<HashMap<String, SyncJob>>,
    /// 控制资源扫描后台线程的生命周期
    pub browser_scanning: Arc<AtomicBool>,
    /// 当前所有浏览器页签（id -> 信息）
    pub tabs: Mutex<HashMap<String, TabInfo>>,
    /// 当前激活的页签 id
    pub active_tab: Mutex<Option<String>>,
    /// 页签自增计数器
    pub tab_counter: Arc<Mutex<u32>>,
    /// 子窗口布局：id -> 内容区 CSS 矩形 (x, y, w, h)，供主窗 move/resize 重定位
    pub child_layouts: Mutex<HashMap<String, (f64, f64, f64, f64)>>,
    /// 上次定位时间：id -> Instant，用于后端去重防抖
    pub last_position_at: Mutex<HashMap<String, std::time::Instant>>,
    /// 终端会话（PTY）：id -> 会话
    pub terminals: Mutex<HashMap<String, TerminalSession>>,
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
    // 含协议分隔但非 http（ftp 等）原样
    if s.contains("://") {
        return s.to_string();
    }
    // 像搜索词（含空格、中文、或不是 域名.后缀 形态）-> 搜索引擎
    let looks_like_domain =
        s.contains('.') && !s.contains(' ') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    if !looks_like_domain {
        // 百度搜索
        return format!("https://www.baidu.com/s?wd={}", urlencoding::encode(s));
    }
    // 裸域名：补 https://
    format!("https://{}", s)
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
        close_tab(&app, &id);
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
    apply_bounds(&app, &id, x, y, width, height)
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
    // 后端去重：50ms 内同一 label 不重复提交定位，避免 IPC 洪泛
    {
        let state = app.state::<AppState>();
        let mut last = state.last_position_at.lock().unwrap();
        let now = std::time::Instant::now();
        if let Some(t) = last.get(id) {
            if now.duration_since(*t).as_millis() < 50 {
                return Ok(());
            }
        }
        last.insert(id.to_string(), now);
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
    manager
        .update_rect(
            &id.to_string(),
            LogicalRect::new(x.round().max(0.0), y.round().max(0.0), width.round().max(1.0), height.round().max(1.0)),
        )
        .map_err(|e| format!("定位失败: {e}"))?;
    let _ = manager.set_visible(&id.to_string(), true);
    eprintln!(
        "[apply_bounds] label={} logical=({},{},{},{})",
        id, x, y, width, height
    );
    remember_layout(app, id, x, y, width, height);
    Ok(())
}

/// apply_bounds 的同步版：仅在已经处于主线程上下文（如 run_on_main_thread 闭包内）
/// 时调用，避免重复排队。直接执行窗口操作。
fn apply_bounds_sync(
    app: &AppHandle,
    id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // 后端去重：50ms 内同一 label 不重复定位
    {
        let state = app.state::<AppState>();
        let mut last = state.last_position_at.lock().unwrap();
        let now = std::time::Instant::now();
        if let Some(t) = last.get(id) {
            if now.duration_since(*t).as_millis() < 50 {
                return Ok(());
            }
        }
        last.insert(id.to_string(), now);
    }
    apply_bounds_inner(app, id, x, y, width, height)
}

/// 把子窗口移出可视区（隐藏态），用于非激活页签/宫格。
fn hide_bounds(app: &AppHandle, id: &str) {
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    let _ = manager.set_visible(&id.to_string(), false);
    // 仍然记住布局，便于再次激活时快速恢复
    if let Some(r) = app.state::<AppState>().child_layouts.lock().unwrap().get(id).copied() {
        remember_layout(app, id, r.0, r.1, r.2, r.3);
    }
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

    // 停止旧的资源扫描线程
    app.state::<AppState>().browser_scanning.store(false, Ordering::Relaxed);

    let title0 = match Url::parse(&target) {
        Ok(u) => u.host_str().unwrap_or(&target).to_string(),
        Err(_) => target.clone(),
    };

    // 方案 B：异步创建独立子窗口（提交到主线程事件循环，避开同步创建第二个
    // webview 卡死主线程）。窗口真正建好后由前端 tab_position 放大显示。
    spawn_child_window(&app, &id, &target, 0.0, 0.0, 1.0, 1.0)?;
    eprintln!("[create_tab] 子窗口已提交创建 label={} init=(1x1) visible=false", id);

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

    start_resource_scanner(app.clone());
    eprintln!("[create_tab] 页签已创建 label={} url={}", id, target);
    Ok(info)
}

/// 关闭指定页签，并清理状态。
fn close_tab(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    state.browser_scanning.store(false, Ordering::Relaxed);
    // 销毁对应子 webview（插件内部走 dispatcher，跨线程安全）
    {
        use tauri_plugin_browser_tabs::TabManagerState;
        let manager = app.state::<TabManagerState>();
        let _ = manager.close_tab(&id.to_string());
    }
    app.state::<AppState>().child_layouts.lock().unwrap().remove(id);
    state.tabs.lock().unwrap().remove(id);
    let mut active = state.active_tab.lock().unwrap();
    if active.as_deref() == Some(id) {
        *active = state.tabs.lock().unwrap().keys().next().cloned();
    }
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

#[tauri::command]
pub fn report_resources(app: AppHandle, page_url: String, items: Vec<ResourceItem>) {
    let _ = app.emit(
        "browser-resources",
        serde_json::json!({ "page_url": page_url, "items": items }),
    );
}

/// 由子窗口内 JS 经 invoke 回传的真实页面标题，转成 tab-title 事件推给前端。
#[tauri::command]
pub fn report_title(app: AppHandle, title: String) {
  let t = title.trim().to_string();
  if t.is_empty() {
    return;
  }
  // 找到当前激活页签（标题回传只针对激活页）
  let id = app
    .state::<AppState>()
    .active_tab
    .lock()
    .unwrap()
    .clone();
  if let Some(id) = id {
    let _ = app.emit(
      "tab-title",
      serde_json::json!({ "id": id, "url": "", "title": t }),
    );
  }
}

/// 对指定子窗口执行 JS 资源扫描，并回传真实标题，通过事件推送给前端。
/// 注意：WebviewWindow 的 eval 在 Tauri 2 主线程执行即可；资源/标题的回传目前通过
/// webview 内已注入的 `window.ipc.postMessage` 通道（见 resources_eval.js 改造）由前端监听，
/// 这里仅负责触发执行。扫描线程通过 run_on_main_thread 调度到此，保证在主线程。
fn scan_resources(_app: AppHandle, _id: &str, win: &Webview) {
    // 执行资源扫描脚本（脚本内部会把结果经 ipc 回传，由前端 onBrowserResources 接收）
    let js = include_str!("../injected/resources_eval.js");
    if let Err(e) = win.eval(js) {
        eprintln!("[scan_resources] eval 失败: {e}");
    }
    // 标题回传：执行一段脚本把 document.title 经 invoke 回传（见 report_title command）
    if let Err(e) = win.eval(
        "window.__TAURI__ && window.__TAURI__.core.invoke('report_title', { title: document.title })",
    ) {
        eprintln!("[scan_resources] 标题回传失败: {e}");
    }
}

/// 启动后台轮询线程：每 2 秒扫描一次当前激活页签的资源，直到浏览器关闭。
/// 线程仅作定时器，真正的 webview 操作通过 run_on_main_thread 调度到主线程执行。
fn start_resource_scanner(app: AppHandle) {
    let state = app.state::<AppState>();
    state.browser_scanning.store(true, Ordering::Relaxed);
    let app_thread = app.clone();
    std::thread::spawn(move || {
        loop {
            if !app_thread
                .state::<AppState>()
                .browser_scanning
                .load(Ordering::Relaxed)
            {
                break;
            }
            let active = app_thread
                .state::<AppState>()
                .active_tab
                .lock()
                .unwrap()
                .clone();
            if let Some(id) = active {
                let a = app_thread.clone();
                let a2 = a.clone();
                let _ = a.run_on_main_thread(move || {
                    if let Some(win) = a2.get_webview(&id) {
                        scan_resources(a2.clone(), &id, &win);
                    } else {
                        a2.state::<AppState>()
                            .browser_scanning
                            .store(false, Ordering::Relaxed);
                    }
                });
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

/// 采集：网页选区 → 本地成果（带溯源）。渲染进程零特权，只能 invoke 本命令。
#[tauri::command]
pub fn collect_selection(
    app: AppHandle,
    url: String,
    title: String,
    text: String,
    html: String,
) -> Result<Artifact, String> {
    let art = Artifact::new(title, url.clone(), text, html);
    workspace::save_artifact(&app, &art)?;
    workspace::log_audit(&app, "collect", format!("{} <- {}", art.title, url));
    let _ = app.emit("artifact-collected", art.clone());
    Ok(art)
}

#[tauri::command]
pub fn list_artifacts(app: AppHandle) -> Vec<Artifact> {
    workspace::load_artifacts(&app)
}

/// 配置仓库：token 仅写入系统密钥库，绝不回传前端。
#[tauri::command]
pub fn configure_repo(
    app: AppHandle,
    config: RepoConfig,
    token: String,
) -> Result<(), String> {
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
        format!("{} ({:?})", repos.last().unwrap().name, repos.last().unwrap().provider),
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

/// 工作区目录树：按来源域名聚合成果，便于在本地成果浏览器里浏览
#[tauri::command]
pub fn browse_workspace(app: AppHandle) -> WorkspaceTree {
    let arts = workspace::load_artifacts(&app);
    let mut domains: std::collections::BTreeMap<String, Vec<Artifact>> = std::collections::BTreeMap::new();
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
    if !dir.exists() { return Err("路径不存在".into()); }
    if !dir.is_dir() { return Err("不是目录".into()); }
    let mut entries = vec![];
    for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let meta = e.metadata().map_err(|e| e.to_string())?;
        entries.push(DirEntry {
            name: e.file_name().to_string_lossy().to_string(),
            path: e.path().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            size: meta.len(),
        });
    }
    // 目录在前、按名字排序
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });
    Ok(entries)
}

/// 读取文本文件内容
#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// 写入文本文件（创建或覆盖）
#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())
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
    dirs
}

// ====== 文件管理：新建 / 删除 / 重命名 ======

/// 新建文件（content 为空则创建空文件）。已存在则报错。
#[tauri::command]
pub fn create_file(path: String, content: Option<String>) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if p.exists() {
        return Err("已存在同名文件/目录".into());
    }
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, content.unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(())
}

/// 新建目录（递归创建父目录）。
#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if p.exists() {
        return Err("已存在同名文件/目录".into());
    }
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除文件或目录（递归删除）。
#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("路径不存在".into());
    }
    if p.is_dir() {
        std::fs::remove_dir_all(&p).map_err(|e| e.to_string())?;
    } else {
        std::fs::remove_file(&p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 重命名 / 移动文件或目录。
#[tauri::command]
pub fn rename_path(path: String, new_name: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("路径不存在".into());
    }
    let parent = p.parent().ok_or("无法取得父目录")?;
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

// ====== 宫格浏览器：多网页并排对比 ======
//
// 思路：创建一组独立子窗口（label = "grid-{i}"），由前端按网格计算每个格子的
// 主窗内容区坐标后调用 grid_position 定位。create_grid 仅负责创建窗口并打开默认页。

const MAX_GRID: usize = 12;

/// 创建 n 个宫格独立子窗口（2..=12），默认都打开引导页，位置由前端 grid_position 告知。
#[tauri::command]
pub fn create_grid(app: AppHandle, n: usize) -> Result<(), String> {
    let n = n.clamp(2, MAX_GRID);
    // 先清理旧的宫格
    close_grid(app.clone())?;
    // 初始用 1x1 隐藏尺寸，前端 grid_position 会精确放大定位
    for i in 0..n {
        let label = format!("grid-{i}");
        // 已存在则跳过（spawn_child_window 内部也会销毁重建，这里直接 continue 更轻）
        if app.get_webview(&label).is_some() {
            continue;
        }
        spawn_child_window(
            &app,
            &label,
            "https://www.baidu.com",
            0.0,
            0.0,
            1.0,
            1.0,
        )?;
        // 窗口已在 builder 中 visible(false)，异步建好后由前端 grid_position 放大
    }
    Ok(())
}

/// 关闭所有宫格（销毁对应子 webview）。
#[tauri::command]
pub fn close_grid(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    for i in 0..MAX_GRID {
        let label = format!("grid-{i}");
        let _ = manager.close_tab(&label);
        app.state::<AppState>().child_layouts.lock().unwrap().remove(&label);
    }
    Ok(())
}

/// 在指定宫格(index)中打开网址。
#[tauri::command]
pub fn grid_open(app: AppHandle, index: usize, url: String) -> Result<(), String> {
    let label = format!("grid-{index}");
    let target = normalize_url(&url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview(&label) {
            let _ = win.eval(&format!(
                "window.location.href = {url_js}",
                url_js = serde_json::to_string(&target).unwrap()
            ));
        }
    });
    Ok(())
}

/// 批量定位所有宫格（前端按网格计算好每个格子的 x/y/w/h 后调用）。
/// 坐标为主窗口内容区相对坐标（CSS），换算成屏幕物理坐标后 set_position/set_size。
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
    apply_bounds(&app, &label, x, y, width, height)
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
        if !in_entry || !t.contains('=') { continue; }
        let (k, v) = t.split_once('=')?;
        match k.trim() {
            "Name" => name = v.to_string(),
            "Exec" => exec = v.to_string(),
            "Icon" => icon = v.to_string(),
            "NoDisplay" => no_display = v.trim() == "true",
            _ => {}
        }
    }
    if name.is_empty() || exec.is_empty() || no_display { return None; }
    // 去掉 Exec 中的字段码 (%U/%F 等)
    let exec_clean = exec.split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");
    let icon_path = resolve_icon(&icon);
    Some(AppEntry { name, exec: exec_clean, icon, icon_path })
}

/// 解析 .desktop 的 Icon 字段为真实图标文件路径。
/// Icon 可能是：绝对路径 / 主题图标名（如 firefox）/ 空。
fn resolve_icon(icon: &str) -> Option<String> {
    if icon.is_empty() { return None; }
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
            if std::path::Path::new(&p).exists() { return Some(p); }
        }
        if let Some(found) = find_icon_recursive(std::path::Path::new(root), icon, &exts, 0) {
            return Some(found);
        }
    }
    None
}

fn find_icon_recursive(dir: &std::path::Path, icon: &str, exts: &[&str], depth: u32) -> Option<String> {
    if depth > 4 { return None; }
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            if let Some(f) = find_icon_recursive(&path, icon, exts, depth + 1) {
                return Some(f);
            }
        } else if path.extension().map(|x| x == "png" || x == "svg" || x == "xpm").unwrap_or(false) {
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
        if !dir.exists() { return; }
        for e in std::fs::read_dir(dir).ok().into_iter().flatten() {
            let e = e.ok(); let path = e.as_ref().map(|e| e.path());
            let Some(path) = path else { continue };
            if !path.to_string_lossy().ends_with(".desktop") { continue; }
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
        scan(std::path::Path::new(&home).join(".local/share/applications").as_path());
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

#[tauri::command]
pub fn launch_app(exec: String) -> Result<(), String> {
    let cmd = exec.trim();
    if cmd.is_empty() {
        return Err("没有可执行命令".into());
    }
    // 使用 sh -c 启动，并把子进程 stdio 重定向到 null，
    // 避免外部应用占用当前终端/stdout 导致主窗口渲染异常。
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
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
    close_tab(&app, &id);
    Ok(())
}

/// 在指定页签中打开网址（导航）。
#[tauri::command]
pub fn tab_open(app: AppHandle, id: String, url: String) -> Result<(), String> {
    let target = normalize_url(&url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    let app2 = app.clone();
    let id_for_closure = id.clone();
    let target_for_closure = target.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview(&id_for_closure) {
            let _ = win.eval(&format!(
                "window.location.href = {url_js}",
                url_js = serde_json::to_string(&target_for_closure).unwrap()
            ));
        }
    });
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
    apply_bounds(&app, &id, x, y, width, height)
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
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview(&id) {
            let _ = win.eval("if (history.length > 1) { history.back(); }");
        }
    });
    Ok(())
}

/// 在当前激活页签的 webview 内执行 history.forward()（页面内前进）。
#[tauri::command]
pub fn tab_go_forward(app: AppHandle, id: String) -> Result<(), String> {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview(&id) {
            let _ = win.eval("history.forward();");
        }
    });
    Ok(())
}

/// 刷新当前激活页签的 webview（location.reload）。
#[tauri::command]
pub fn tab_reload(app: AppHandle, id: String) -> Result<(), String> {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview(&id) {
            let _ = win.eval("location.reload();");
        }
    });
    Ok(())
}

/// 设置激活页签：只更新 active_tab，并把其它页签隐藏。
/// 目标页签的精确位置由前端随后调用 tab_position 给出，避免后端与前端争夺坐标。
#[tauri::command]
pub fn tab_activate(app: AppHandle, id: String) -> Result<(), String> {
    // 暂存激活页签，前端会随后调用 tab_position 精确布局
    *app.state::<AppState>().active_tab.lock().unwrap() = Some(id.clone());
    // 把其它页签隐藏（子窗口 hide），目标页签若有记住布局则立即重定位显示。
    // 这些 GTK 操作必须整体走 run_on_main_thread，避免 command 同步上下文死锁。
    let app_act = app.clone();
    let active_id = id.clone();
    let _ = app.run_on_main_thread(move || {
        let st = app_act.state::<AppState>();
        let ids: Vec<String> = st.tabs.lock().unwrap().keys().cloned().collect();
        for k in ids {
            if k != active_id {
                hide_bounds(&app_act, &k);
            }
        }
        let layout = st.child_layouts.lock().unwrap().get(&active_id).copied();
        drop(st);
        if let Some(r) = layout {
            let _ = apply_bounds_sync(&app_act, &active_id, r.0, r.1, r.2, r.3);
        }
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
    // TERM=dumb 让大多数程序不输出 ANSI 颜色；后端再过滤一次兜底
    cmd.env("TERM", "dumb");
    cmd.env("COLORTERM", "");
    cmd.env("NO_COLOR", "1");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("无法启动 shell: {e}"))?;
    drop(pair.slave);

    let mut writer = pair.master.take_writer().map_err(|e| format!("writer: {e}"))?;
    // 触发初始提示符
    let _ = writer.write_all(b"\n");

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("reader: {e}"))?;
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
                    // 过滤 ANSI 转义序列，避免前端纯文本 div 显示乱码
                    let stripped = strip_ansi_escapes::strip(&buf[..n]);
                    let data = String::from_utf8_lossy(&stripped).to_string();
                    if !data.is_empty() {
                        let _ = app2.emit(
                            "term-data",
                            serde_json::json!({ "id": tid, "data": data }),
                        );
                    }
                }
                Err(_) => break,
            }
        }
        let _ = app2.emit("term-data", serde_json::json!({ "id": tid, "data": "\r\n[终端已退出]\r\n" }));
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
        let _ = s.child.kill();
    }
    Ok(())
}


