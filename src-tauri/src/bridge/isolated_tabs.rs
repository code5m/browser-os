//! Explicit opt-in ordinary-tab process backend. Browser metadata remains
//! in AppState.tabs; the shared native process manager owns the actual child.
//! No runtime migration of an existing WebView, cookie copy, or fallback.
use super::*;
use crate::grid_ipc::{tab_renderer_index, GridCmd};

pub(crate) fn mode_enabled() -> bool {
    std::env::var("BROWSEROS_EXPERIMENTAL_ISOLATED_TABS").as_deref() == Ok("1")
        && std::env::var_os("GRID_CHILD_INDEX").is_none()
}

pub(crate) fn route(id: &str) -> Option<u32> {
    mode_enabled().then(|| tab_renderer_index(id)).flatten()
}

static RENDERER_LAYOUT: crate::shared::blocking::BlockingLane =
    crate::shared::blocking::BlockingLane::new(8, "TAB_RENDERER_LAYOUT_BUSY", "TAB_RENDERER_LAYOUT_WORKER_FAILED");

pub(super) async fn on_worker<T, F>(work: F) -> Result<T, String>
where T: Send + 'static, F: FnOnce() -> Result<T, String> + Send + 'static {
    RENDERER_LAYOUT.run(work).await
}

fn check_running(app: &AppHandle) -> Result<(), String> {
    if app.state::<AppState>().shutdown_requested.load(Ordering::Acquire) {
        Err("TAB_APPLICATION_SHUTTING_DOWN".into())
    } else {
        Ok(())
    }
}

fn exists(app: &AppHandle, id: &str) -> Result<u32, String> {
    check_running(app)?;
    let index = route(id).ok_or("TAB_RENDERER_ROUTE_INVALID")?;
    if !app.state::<AppState>().tabs.lock().unwrap().contains_key(id) {
        return Err("TAB_NOT_FOUND".into());
    }
    Ok(index)
}

/// Called only by an admitted Browser tab mutation worker.
pub(super) fn create(app: &AppHandle, url: &str) -> Result<TabInfo, String> {
    check_running(app)?;
    if !cfg!(any(target_os = "linux", target_os = "windows")) {
        return Err("TAB_RENDERER_PLATFORM_UNSUPPORTED".into());
    }
    let url = normalize_url(url);
    let parsed = Url::parse(&url).map_err(|error| format!("TAB_URL_INVALID:{error}"))?;
    let state = app.state::<AppState>();
    let id = {
        let mut counter = state.tab_counter.lock().unwrap();
        *counter = counter.checked_add(1).ok_or("TAB_RENDERER_ID_EXHAUSTED")?;
        format!("tab-{}", *counter)
    };
    let index = tab_renderer_index(&id).ok_or("TAB_RENDERER_ID_EXHAUSTED")?;
    if let Err(error) = state.grid_manager.start_tab_renderer(index, &url) {
        // Even an unpublished failed spawn may own a listener or retirement.
        return match state.grid_manager.close_tab_renderer(index, true) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!("{error}; {cleanup}")),
        };
    }
    let info = TabInfo {
        id: id.clone(),
        url: state.grid_manager.last_url(index).unwrap_or_else(|| url.clone()),
        title: parsed.host_str().unwrap_or(url.as_str()).to_owned(),
    };
    let published = {
        let mut tabs = state.tabs.lock().unwrap();
        if state.shutdown_requested.load(Ordering::Acquire) {
            false
        } else {
            tabs.insert(id.clone(), info.clone());
            true
        }
    };
    if !published {
        state.grid_manager.close_tab_renderer(index, true)?;
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }
    // A failure here is a partial activation, not a claim creation never ran.
    select(app, &id)?;
    Ok(info)
}

fn select(app: &AppHandle, id: &str) -> Result<(), String> {
    check_running(app)?;
    let state = app.state::<AppState>();
    let ids: Vec<String> = state.tabs.lock().unwrap().keys().cloned().collect();
    if !ids.iter().any(|other| other == id) {
        return Err("TAB_NOT_FOUND".into());
    }
    // Fence late positions of the previous active tab before issuing hides.
    *state.active_tab.lock().unwrap() = Some(id.to_owned());
    for other in ids.iter().filter(|other| other.as_str() != id) {
        let index = route(other).ok_or("TAB_RENDERER_MIXED_BACKEND_UNSUPPORTED")?;
        state.grid_manager.hide_tab_renderer(index)
            .map_err(|error| format!("TAB_RENDERER_ACTIVATION_PARTIAL:{error}"))?;
    }
    let mut idle = state.tab_idle_since.lock().unwrap();
    idle.remove(id);
    let now = std::time::Instant::now();
    for other in ids {
        if other != id {
            idle.entry(other).or_insert(now);
        }
    }
    Ok(())
}

/// An explicit activation may restore a crashed tab from its last URL after
/// original native retirement. It never silently reloads a healthy document.
pub(super) fn activate(app: &AppHandle, id: &str) -> Result<(), String> {
    let index = exists(app, id)?;
    let state = app.state::<AppState>();
    if state.grid_manager.tab_renderer_owner(index)?.is_none() {
        let url = state.tabs.lock().unwrap().get(id).map(|tab| tab.url.clone())
            .ok_or("TAB_NOT_FOUND")?;
        state.grid_manager.start_tab_renderer(index, &url)?;
    } else {
        state.grid_manager.request(index, GridCmd::Ping, 3000)?;
    }
    // The normal frontend position command supplies the new active rectangle.
    select(app, id)
}

pub(super) fn navigate(app: &AppHandle, id: &str, url: &str) -> Result<(), String> {
    let index = exists(app, id)?;
    let manager = &app.state::<AppState>().grid_manager;
    manager.request(index, GridCmd::Navigate { id: id.to_owned(), url: url.to_owned() }, 10000)?;
    manager.record_url(index, url);
    Ok(())
}

pub(super) fn eval(app: &AppHandle, id: &str, js: &str) -> Result<(), String> {
    let index = exists(app, id)?;
    app.state::<AppState>().grid_manager.request(index, GridCmd::Eval {
        id: id.to_owned(), js: js.to_owned(),
    }, 10000)
}

pub(super) fn position(app: &AppHandle, id: &str, css: (f64, f64, f64, f64)) -> Result<(), String> {
    let index = exists(app, id)?;
    app.state::<AppState>().grid_manager.position_tab_renderer(index, css)
}

pub(super) fn hide(app: &AppHandle, id: &str) -> Result<(), String> {
    let index = exists(app, id)?;
    app.state::<AppState>().grid_manager.hide_tab_renderer(index)
}

pub(super) fn hide_all(app: &AppHandle) -> Result<(), String> {
    check_running(app)?;
    let state = app.state::<AppState>();
    let ids: Vec<_> = state.tabs.lock().unwrap().keys().cloned().collect();
    for id in ids {
        hide(app, &id)?;
    }
    for index in state.grid_manager.indices() {
        state.grid_manager.record_hidden(index);
        state.grid_manager.request(index, GridCmd::HideWindow {
            id: crate::grid_ipc::renderer_label(index),
        }, 3000)?;
    }
    Ok(())
}
