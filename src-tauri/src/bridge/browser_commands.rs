//! Native browser WebView positioning, visibility and layout commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

/// 打开浏览器（内嵌为 main 窗口的子 webview）。
/// 内部等价于「新建一个页签」并设为激活页签。前端优先使用 tab_new 多开页签。
#[tauri::command]
pub fn open_browser(webview: tauri::Webview, app: AppHandle, url: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "open_browser")?;
    crate::shared::native_legacy_policy::bound_text("open_browser", "url", &url)?;

    let _tab = create_tab(app.clone(), &url)?;
    Ok(())
}

/// 关闭内嵌浏览器（默认关闭当前激活页签）。
#[tauri::command]
pub fn close_browser(webview: tauri::Webview, app: AppHandle) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "close_browser")?;

    // Copy the active id then DROP its mutex guard before close_tab().
    // An if-let scrutinee temporary can keep the lock alive across the body;
    // close_tab() also locks active_tab, causing a self-deadlock in real GTK.
    let active_id = {
        let state = app.state::<AppState>();
        let active = state.active_tab.lock().unwrap();
        active.clone()
    };
    if let Some(id) = active_id {
        close_tab(&app, &id)?;
    }
    Ok(())
}

/// 把当前激活页签的子窗口定位到主窗内容区指定矩形（CSS 坐标）。
/// 坐标由前端传入：相对主窗内容区左上角的 CSS 像素（getBoundingClientRect 的 left/top/width/height）。
#[tauri::command]
pub fn position_browser(
    webview: tauri::Webview,
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "position_browser")?;
    crate::shared::native_legacy_policy::bound_dimension("position_browser", "x", x)?;
    crate::shared::native_legacy_policy::bound_dimension("position_browser", "y", y)?;
    crate::shared::native_legacy_policy::bound_dimension("position_browser", "width", width)?;
    crate::shared::native_legacy_policy::bound_dimension("position_browser", "height", height)?;

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
pub(crate) fn apply_bounds(
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
pub(crate) fn apply_bounds_inner(
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

/// 强制隐藏所有子 webview（页签 + 宫格）。
/// 关键：【不能走 grid_position/apply_bounds】——它有 50ms 去重，宫格刚定位后
/// 50ms 内的"移出屏幕"请求会被去重丢弃，导致宫格仍留在屏幕上（实测切主页仍看到
/// 宫格内容）。hide_bounds 无去重、保持尺寸只移位置，是隐藏的正确入口。
/// 对没记录在 child_layouts 的宫格（极端情况），也强制移出（直接给 1x1 临时尺寸）。
/// 隐藏单个子 webview（页签/宫格），用无去重的 hide_bounds。
/// 宫格显示时移出激活页签、页签切换时移出非激活页签都走这里，
/// 避免 tab_position/apply_bounds 的 50ms 去重把"移出屏幕"请求丢弃。
#[tauri::command]
pub fn hide_webview(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "hide_webview")?;
    crate::shared::native_legacy_policy::bound_text("hide_webview", "id", &id)?;

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
pub fn hide_all_webviews(webview: tauri::Webview, app: AppHandle) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "hide_all_webviews")?;

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
        // GTK can redistribute the main widget when a native tab is added or
        // removed. Keep the shell viewport full-sized as well as the tabs.
        if let Some(main) = app.get_webview("main") {
            let _ = tauri_plugin_browser_tabs::ensure_native_layout(&main);
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
