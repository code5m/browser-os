//! Native browser WebView positioning, visibility and layout commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

fn publish_layout_if_current(
    layouts: &mut HashMap<String, (f64, f64, f64, f64)>,
    revisions: &mut HashMap<String, u64>,
    id: &str,
    prior_target: Option<(f64, f64, f64, f64)>,
    prior_revision: u64,
    next_target: (f64, f64, f64, f64),
) -> bool {
    if layouts.get(id).copied() != prior_target
        || revisions.get(id).copied().unwrap_or(0) != prior_revision
    {
        return false;
    }
    layouts.insert(id.to_string(), next_target);
    revisions.insert(id.to_string(), prior_revision.wrapping_add(1));
    true
}

#[cfg(test)]
mod layout_revision_tests {
    use super::*;

    #[test]
    fn a_late_visible_position_cannot_undo_a_hide_even_after_aba() {
        let mut layouts = HashMap::from([("tab".to_string(), (4.0, 5.0, 600.0, 400.0))]);
        let mut revisions = HashMap::from([("tab".to_string(), 1)]);
        let original = layouts["tab"];
        layouts.insert("tab".into(), (-30000.0, 5.0, 600.0, 400.0));
        revisions.insert("tab".into(), 2);
        layouts.insert("tab".into(), original);
        revisions.insert("tab".into(), 3);
        assert!(!publish_layout_if_current(
            &mut layouts,
            &mut revisions,
            "tab",
            Some(original),
            1,
            (10.0, 5.0, 600.0, 400.0),
        ));
        assert_eq!(layouts["tab"], original);
        assert_eq!(revisions["tab"], 3);
    }
}

/// 打开浏览器（内嵌为 main 窗口的子 webview）。
/// 内部等价于「新建一个页签」并设为激活页签。前端优先使用 tab_new 多开页签。
#[tauri::command]
pub async fn open_browser(webview: tauri::Webview, app: AppHandle, url: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "open_browser")?;
    crate::shared::native_legacy_policy::bound_text("open_browser", "url", &url)?;

    // Compatibility alias uses the same Browser Owner and receipt path.
    super::tab_mutation::legacy_action::<TabInfo>(webview,app,
        super::tab_mutation::Action::New{url}).await.map(|_|())
}

/// 关闭内嵌浏览器（默认关闭当前激活页签）。
#[tauri::command]
pub async fn close_browser(webview: tauri::Webview, app: AppHandle) -> Result<(), String> {
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
        return super::tab_mutation::legacy_action::<()>(webview,app,
            super::tab_mutation::Action::Close{id}).await;
    }
    Ok(())
}

/// 把当前激活页签的子窗口定位到主窗内容区指定矩形（CSS 坐标）。
/// 坐标由前端传入：相对主窗内容区左上角的 CSS 像素（getBoundingClientRect 的 left/top/width/height）。
#[tauri::command]
pub async fn position_browser(
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
    if super::isolated_tabs::route(&id).is_some() {
        let admitted = super::isolated_tabs::capture_layout(&app, &id)?;
        return super::isolated_tabs::on_worker(move || {
            super::isolated_tabs::position_admitted(&app, &id, (x, y, width, height), admitted)
        }).await;
    }
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
    // Dedup only successful requests. A pending or failed dispatch is
    // not a receipt. Also consult the current target: a subsequent hide
    // must invalidate an otherwise identical recent visible position.
    let now = std::time::Instant::now();
    let state = app.state::<AppState>();
    let recent_success = {
        let last = state.last_position_at.lock().unwrap();
        last.get(id).is_some_and(|(t, lx, ly, lw, lh)| {
            let same_rect = (lx - x).abs() < 0.5
                && (ly - y).abs() < 0.5
                && (lw - width).abs() < 0.5
                && (lh - height).abs() < 0.5;
            same_rect && now.saturating_duration_since(*t).as_millis() < 50
        })
    };
    let expected = (
        x.round().max(0.0),
        y.round().max(0.0),
        width.round().max(1.0),
        height.round().max(1.0),
    );
    if recent_success && state.child_layouts.lock().unwrap().get(id).copied() == Some(expected) {
        return Ok(());
    }
    let result = apply_bounds_inner(app, id, x, y, width, height);
    if result.is_ok() {
        let mut last = state.last_position_at.lock().unwrap();
        if last.get(id).map_or(true, |entry| entry.0 <= now) {
            last.insert(id.to_string(), (now, x, y, width, height));
        }
    }
    result
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
    let rect = LogicalRect::new(
        x.round().max(0.0),
        y.round().max(0.0),
        width.round().max(1.0),
        height.round().max(1.0),
    );
    let state = app.state::<AppState>();
    let (prior_target, prior_revision) = {
        let layouts = state.child_layouts.lock().unwrap();
        let revisions = state.layout_revisions.lock().unwrap();
        (
            layouts.get(id).copied(),
            revisions.get(id).copied().unwrap_or(0),
        )
    };
    if let Err(e) = manager.position_visible(&id.to_string(), rect, || {
        let mut layouts = state.child_layouts.lock().unwrap();
        // A hide request may have been recorded while the dispatcher was
        // busy. Do not replace that newer target with an older receipt.
        let mut revisions = state.layout_revisions.lock().unwrap();
        publish_layout_if_current(
            &mut layouts,
            &mut revisions,
            id,
            prior_target,
            prior_revision,
            (rect.x, rect.y, rect.width, rect.height),
        )
    }) {
        eprintln!("[apply_bounds] label={} position/show failed: {e}", id);
        return Err(format!("定位或显示失败: {e}"));
    }
    eprintln!(
        "[apply_bounds] label={} logical=({},{},{},{})",
        id, x, y, width, height
    );
    // The normalized target was published before releasing the plugin's
    // layout reservation; the sweep never replays a pre-command snapshot.
    Ok(())
}

/// 强制隐藏所有子 webview（页签 + 宫格）。
/// 关键：【不能走 grid_position/apply_bounds】——它有 50ms 去重，宫格刚定位后
/// 50ms 内的"移出屏幕"请求会被去重丢弃，导致宫格仍留在屏幕上（实测切主页仍看到
/// 宫格内容）。hide_bounds 无去重、保持尺寸只移位置，是隐藏的正确入口。
/// 对无 bridge 布局记录的 Linux 页签，从插件读取已有视口尺寸再移出。
/// 隐藏单个子 webview（页签/宫格），用无去重的 hide_bounds。
/// 宫格显示时移出激活页签、页签切换时移出非激活页签都走这里，
/// 避免 tab_position/apply_bounds 的 50ms 去重把"移出屏幕"请求丢弃。
#[tauri::command]
pub async fn hide_webview(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "hide_webview")?;
    crate::shared::native_legacy_policy::bound_text("hide_webview", "id", &id)?;
    if super::isolated_tabs::route(&id).is_some() {
        let admitted = super::isolated_tabs::capture_layout(&app, &id)?;
        return super::isolated_tabs::on_worker(move || super::isolated_tabs::hide_admitted(&app, &id, admitted)).await;
    }

    // 宫格已迁子进程：隐藏 = 子进程整个窗口 hide（等价旧方案的移出屏幕）
    if let Some(index) = grid_index_of(&id) {
        let stamp = app.state::<AppState>().grid_manager.grid_route_stamps(&[index])?
            .into_iter().next().ok_or("GRID_INDEX_INVALID")?;
        return super::isolated_tabs::on_worker(move || {
            app.state::<AppState>().grid_manager.hide_grid_route(stamp)
        }).await;
    }
    let state=app.state::<AppState>();
    if state.hibernated_tabs.lock().unwrap().contains(&id) {return Ok(());}
    hide_bounds(&app,&id)
}

#[tauri::command]
pub async fn hide_all_webviews(webview: tauri::Webview, app: AppHandle) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "hide_all_webviews")?;
    if cfg!(any(target_os = "linux", target_os = "windows")) {
        return super::isolated_tabs::on_worker(move || hide_all_webviews_sync(webview, app)).await;
    }
    hide_all_webviews_sync(webview, app)
}

pub(crate) fn hide_all_webviews_sync(webview: tauri::Webview, app: AppHandle) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "hide_all_webviews")?;
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager=app.state::<TabManagerState>();
    let state=app.state::<AppState>();
    let mut failures=Vec::new();
    if cfg!(any(target_os="linux",target_os="windows")) {
        let result=if super::isolated_tabs::mode_enabled(){
            super::isolated_tabs::hide_all(&app)
        }else{state.grid_manager.hide_grid_renderers()};
        if let Err(error)=result{failures.push(error);}
    }
    // Include all existing ordinary plugin tabs, even if creation has
    // published native state but not yet published logical metadata.
    let mut ids:Vec<_>=manager.get_tab_ids().into_iter().filter(|id|super::is_tab_label(id)).collect();
    ids.sort();
    for id in ids {
        if let Err(error)=hide_bounds(&app,&id){failures.push(error);}
    }
    if failures.is_empty(){Ok(())}
    else {Err(format!("TAB_HIDE_ALL_PARTIAL:{}",failures.join("; ")))} 
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
        #[cfg(target_os = "linux")]
        {
            // A blocked GTK loop must never collect one correction per
            // timer tick. Snapshot the newest geometry only when the
            // queued sweep actually executes, not when it was requested.
            if LAYOUT_SWEEP_IN_FLIGHT.swap(true, Ordering::AcqRel) {
                continue;
            }
            let sweep_app = app.clone();
            if app
                .run_on_main_thread(move || {
                    let _sweep = LayoutSweepGuard;
                    LAYOUT_SWEEP_RUNNING.store(true, Ordering::Release);
                    if !sweep_app
                        .state::<AppState>()
                        .shutdown_requested
                        .load(Ordering::Acquire)
                    {
                        enforce_layout_once(&sweep_app);
                    }
                })
                .is_err()
            {
                LAYOUT_SWEEP_IN_FLIGHT.store(false, Ordering::Release);
                break;
            }
        }
        #[cfg(not(target_os = "linux"))]
        enforce_layout_once(&app);
    });
}

#[cfg(target_os = "linux")]
static LAYOUT_SWEEP_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "linux")]
static LAYOUT_SWEEP_RUNNING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
struct LayoutSweepGuard;

#[cfg(target_os = "linux")]
impl Drop for LayoutSweepGuard {
    fn drop(&mut self) {
        LAYOUT_SWEEP_RUNNING.store(false, Ordering::Release);
        LAYOUT_SWEEP_IN_FLIGHT.store(false, Ordering::Release);
    }
}

/// Fixed diagnostic state: no tab identifiers, URLs, or browser content.
#[cfg(target_os = "linux")]
pub(crate) fn layout_enforcer_status() -> &'static str {
    if LAYOUT_SWEEP_RUNNING.load(Ordering::Acquire) {
        "running"
    } else if LAYOUT_SWEEP_IN_FLIGHT.load(Ordering::Acquire) {
        "queued"
    } else {
        "idle"
    }
}

fn enforce_layout_once(app: &AppHandle) {
    // Preserve the tested GTK allocation/coordinate behavior. Only the
    // scheduling and lock boundaries change.
    if let Some(main) = app.get_webview("main") {
        let _ = tauri_plugin_browser_tabs::ensure_native_layout(&main);
    }
    let state = app.state::<AppState>();
    use tauri_plugin_browser_tabs::{LogicalRect, TabManagerState};
    let manager = app.state::<TabManagerState>();
    {
        // Capture identities, not stale rectangles. The plugin reads each
        // target after it has reserved that tab's layout operation.
        let mut ids: Vec<String> = {
            let Ok(layouts) = state.child_layouts.try_lock() else {
                return;
            };
            layouts.keys().cloned().collect()
        };
        if ids.is_empty() {
            return;
        }
        ids.sort_unstable();
        // A single GTK callback has bounded work even when many background
        // tabs exist. Rotate the starting point so no tab starves.
        static NEXT_LAYOUT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let start = NEXT_LAYOUT.fetch_add(8, Ordering::Relaxed) % ids.len();
        for offset in 0..ids.len().min(8) {
            let id = &ids[(start + offset) % ids.len()];
            let _ = manager.reconcile_rect(id, || {
                let layouts = state.child_layouts.try_lock().ok()?;
                layouts
                    .get(id)
                    .map(|&(x, y, w, h)| LogicalRect::new(x, y, w.max(1.0), h.max(1.0)))
            });
        }
    }
}
