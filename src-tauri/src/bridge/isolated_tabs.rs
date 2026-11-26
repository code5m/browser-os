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
        url: url.clone(),
        title: parsed.host_str().unwrap_or(url.as_str()).to_owned(),
    };
    let info = match state.grid_manager.publish_tab_renderer_info(index, info) {
        Ok(info) => info,
        Err(error) => {
            return match state.grid_manager.close_tab_renderer(index, true) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!("{error}; {cleanup}")),
            };
        }
    };
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
    let index = route(id).ok_or("TAB_RENDERER_ROUTE_INVALID")?;
    let others = ids.iter().filter(|other| other.as_str() != id)
        .map(|other| route(other).ok_or("TAB_RENDERER_MIXED_BACKEND_UNSUPPORTED"))
        .collect::<Result<Vec<_>, _>>()?;
    state.grid_manager.select_tab_renderer(index, &others)?;
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

/// Only explicitly enabled, idle pristine blank pages can be reclaimed.
/// Retain the logical tab and same listener generation across confirmed native
/// retirement. A failed retirement is not a successful hibernation receipt.
pub(super) fn hibernate_blank(app: &AppHandle, id: &str, idle_window: std::time::Duration) -> bool {
    let Some(index) = route(id) else { return false; };
    let Ok(_owner) = super::tab_mutation::reserve_internal() else { return false; };
    let Ok(_transition) = super::tab_runtime::reserve_tab_transition(id) else { return false; };
    let state = app.state::<AppState>();
    if !state.hibernation_enabled.load(Ordering::Acquire)
        || state.shutdown_requested.load(Ordering::Acquire)
        || state.active_tab.lock().unwrap().as_deref() == Some(id)
        || !state.tab_idle_since.lock().unwrap().get(id).is_some_and(|since| since.elapsed() >= idle_window)
        || state.hibernated_tabs.lock().unwrap().contains(id)
        || !state.tabs.lock().unwrap().get(id).is_some_and(|tab| tab.url == "about:blank")
    {
        return false;
    }
    if !matches!(state.grid_manager.hibernate_blank_tab_renderer(index, idle_window), Ok(true)) {
        return false;
    }
    state.child_layouts.lock().unwrap().remove(id);
    state.layout_revisions.lock().unwrap().remove(id);
    state.last_position_at.lock().unwrap().remove(id);
    state.tab_idle_since.lock().unwrap().remove(id);
    state.hibernated_scroll.lock().unwrap().remove(id);
    state.hibernated_tabs.lock().unwrap().insert(id.to_owned());
    true
}

/// An explicit activation may restore a crashed tab from its last URL after
/// original native retirement. It never silently reloads a healthy document.
pub(super) fn activate(app: &AppHandle, id: &str) -> Result<(), String> {
    let index = exists(app, id)?;
    let state = app.state::<AppState>();
    if state.grid_manager.tab_renderer_owner(index)?.is_none() {
        let url = state.tabs.lock().unwrap().get(id).map(|tab| tab.url.clone())
            .ok_or("TAB_NOT_FOUND")?;
        // A crash is not authorization to re-execute a transient document
        // or reopen an embedded-credential URL. Keep the logical tab.
        if !crate::grid_ipc::restorable_address(&url) {
            return Err("TAB_RESTORE_URL_UNSUPPORTED".into());
        }
        state.grid_manager.start_tab_renderer(index, &url)?;
        state.hibernated_tabs.lock().unwrap().remove(id);
    } else {
        state.grid_manager.request(index, GridCmd::Ping, 3000)?;
        state.hibernated_tabs.lock().unwrap().remove(id);
    }
    // The normal frontend position command supplies the new active rectangle.
    select(app, id)
}

pub(super) fn navigate(app: &AppHandle, id: &str, url: &str) -> Result<(), String> {
    let index = exists(app, id)?;
    app.state::<AppState>().grid_manager.navigate_tab_renderer(index, url)
}

pub(super) fn eval(app: &AppHandle, id: &str, js: &str) -> Result<(), String> {
    let index = exists(app, id)?;
    let state = app.state::<AppState>();
    if state.hibernated_tabs.lock().unwrap().contains(id) {
        return Err("TAB_RENDERER_HIBERNATED_ACTIVATE_FIRST".into());
    }
    // Preserve the existing trusted-main Eval permission and dispatch-only
    // receipt, but bind the native route before any IPC wait. An Agent/UI
    // caller cannot follow a replacement renderer or wake a sleeping page.
    let binding = state.grid_manager.renderer_binding(index)?;
    eval_bound(app, id, js, binding)
}

/// Reuse the authenticated request path without taking a fresh owner after
/// admission. This is a dispatch acknowledgement, not a JS-completion proof.
pub(super) fn eval_bound(
    app: &AppHandle, id: &str, js: &str, binding: crate::grid_ipc::RendererBinding,
) -> Result<(), String> {
    let index = exists(app, id)?;
    if index != binding.index {
        return Err("TAB_RENDERER_DOCUMENT_STALE".into());
    }
    let state = app.state::<AppState>();
    if state.hibernated_tabs.lock().unwrap().contains(id) {
        return Err("TAB_RENDERER_HIBERNATED_ACTIVATE_FIRST".into());
    }
    state.grid_manager.renderer_request_data(binding, GridCmd::Eval {
        id: id.to_owned(), js: js.to_owned(),
    }, 10000).map(|_| ())
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
    let mut ids: Vec<_> = state.tabs.lock().unwrap().keys().cloned().collect();
    ids.sort();
    let mut errors = Vec::new();
    for id in ids {
        if let Err(error) = hide(app, &id) {
            errors.push(format!("{id}:{error}"));
        }
    }
    let mut indices: Vec<_> = state.grid_manager.indices().into_iter().collect();
    indices.sort_unstable();
    for index in indices {
        state.grid_manager.record_hidden(index);
        if let Err(error) = state.grid_manager.request(index, GridCmd::HideWindow {
            id: crate::grid_ipc::renderer_label(index),
        }, 3000) {
            errors.push(format!("grid-{index}:{error}"));
        }
    }
    if errors.is_empty() { Ok(()) }
    else { Err(format!("TAB_RENDERER_HIDE_PARTIAL:{}", errors.join("; "))) }
}
