//! Explicit opt-in ordinary-tab process backend. Browser metadata remains
//! in AppState.tabs; the shared native process manager owns the actual child.
//! No runtime migration of an existing WebView, cookie copy, or fallback.
use super::*;
use crate::grid_ipc::{tab_renderer_index, GridCmd};

pub(crate) fn mode_enabled() -> bool {
    cfg!(any(target_os = "linux", target_os = "windows"))
        && std::env::var("BROWSEROS_EXPERIMENTAL_ISOLATED_TABS").as_deref() == Ok("1")
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
    // The tab is already published. Report the partial outcome explicitly
    // so the UI reconciles its logical list instead of creating a duplicate.
    let selection = state.grid_manager.renderer_binding(index)
        .and_then(|target| select_bound(app, &id, target));
    if let Err(error) = selection {
        return Err(format!("TAB_CREATED_ACTIVATION_PARTIAL:{id}:{error}"));
    }
    Ok(info)
}

fn select_bound(app: &AppHandle, id: &str, target: crate::grid_ipc::RendererBinding) -> Result<(), String> {
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
    if target.index != index { return Err("TAB_RENDERER_ACTIVATION_STALE".into()); }
    state.grid_manager.select_tab_renderer_bound(target, &others)?;
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

/// Explicit suspension publishes sleeping state only after native retirement.
pub(super) fn park_admitted(app: &AppHandle, id: &str, address: &str,
    binding: crate::grid_ipc::RendererBinding) -> Result<(), String> {
    let index = exists(app, id)?;
    if index != binding.index { return Err("TAB_RENDERER_PARK_STALE".into()); }
    let state = app.state::<AppState>();
    if state.hibernated_tabs.lock().unwrap().contains(id)
        || state.active_tab.lock().unwrap().as_deref() == Some(id) {
        return Err("TAB_RENDERER_PARK_STALE".into());
    }
    let viewport = state.grid_manager.park_tab_renderer(binding, address)?;
    state.child_layouts.lock().unwrap().remove(id);
    state.layout_revisions.lock().unwrap().remove(id);
    state.last_position_at.lock().unwrap().remove(id);
    state.tab_idle_since.lock().unwrap().remove(id);
    state.hibernated_scroll.lock().unwrap().insert(id.to_owned(), (address.to_owned(), viewport));
    state.tab_recovery.lock().unwrap().remove(id);
    state.hibernated_tabs.lock().unwrap().insert(id.to_owned());
    Ok(())
}

/// An explicit activation may restore a crashed tab from its last URL after
/// original native retirement. It never silently reloads a healthy document.
pub(super) fn activate_admitted(app: &AppHandle, id: &str, admitted: Option<Option<(u32, u64)>>) -> Result<(), String> {
    let index = exists(app, id)?;
    let state = app.state::<AppState>();
    let owner = state.grid_manager.tab_renderer_owner(index)?;
    if admitted.is_some_and(|expected| expected != owner) {
        return Err("TAB_RENDERER_ACTIVATION_STALE".into());
    }
    let mut viewport_restored = true;
    let binding = if owner.is_none() {
        let url = state.tabs.lock().unwrap().get(id).map(|tab| tab.url.clone())
            .ok_or("TAB_NOT_FOUND")?;
        // A crash is not authorization to re-execute a transient document
        // or reopen an embedded-credential URL. Keep the logical tab.
        if !crate::grid_ipc::restorable_address(&url) {
            return Err("TAB_RESTORE_URL_UNSUPPORTED".into());
        }
        state.grid_manager.start_tab_renderer(index, &url)?;
        // A recreated host is no longer a hibernated native page, even if
        // binding or selection fails. Retain a partial activation receipt.
        state.hibernated_tabs.lock().unwrap().remove(id);
        let initial = state.grid_manager.renderer_binding(index)
            .map_err(|error| format!("TAB_RENDERER_ACTIVATION_PARTIAL:{id}:{error}"))?;
        // Consume a snapshot only on recreation, before selection. Never
        // defer it to an unrelated later activation or override existing scroll.
        let snapshot = state.hibernated_scroll.lock().unwrap().remove(id);
        if let Some((address,(x,y))) = snapshot {
            if address == url && crate::grid_ipc::valid_viewport(x,y) {
                viewport_restored = false;
                for _ in 0..4 {
                    if state.shutdown_requested.load(Ordering::Acquire) { break; }
                    let Ok(current) = state.grid_manager.renderer_binding(index) else { break; };
                    if current.owner != initial.owner || current.generation != initial.generation { break; }
                    if !state.tabs.lock().unwrap().get(id).is_some_and(|tab| tab.url == address) { break; }
                    let reply = state.grid_manager.renderer_request_data(current, GridCmd::ViewportRestore {
                        id: id.to_owned(), address: address.clone(), x, y,
                    }, 1000);
                    match reply {
                        Ok(Some(value)) if value == "applied" || value == "skip_nonzero" => {
                            viewport_restored = true;
                            break;
                        }
                        Ok(Some(value)) if value == "deferred" => {
                            std::thread::sleep(std::time::Duration::from_millis(250));
                        }
                        // No resend after an unknown outcome or failed identity.
                        _ => break,
                    }
                }
            }
        }
        let current = state.grid_manager.renderer_binding(index)
            .map_err(|error| format!("TAB_RENDERER_ACTIVATION_PARTIAL:{id}:{error}"))?;
        if current.owner != initial.owner || current.generation != initial.generation {
            return Err(format!("TAB_RENDERER_ACTIVATION_PARTIAL:{id}:TAB_VIEWPORT_STALE"));
        }
        current
    } else {
        let binding = state.grid_manager.renderer_binding(index)?;
        if Some(binding.owner) != owner {
            return Err("TAB_RENDERER_ACTIVATION_STALE".into());
        }
        state.grid_manager.refresh_tab_downloads(binding, 3000)?;
        // Activation and Ping are process-scoped; normal navigation must
        // not invalidate selection of the same authenticated native host.
        let current = state.grid_manager.renderer_binding(index)?;
        if current.owner != binding.owner || current.generation != binding.generation {
            return Err("TAB_RENDERER_ACTIVATION_STALE".into());
        }
        state.hibernated_tabs.lock().unwrap().remove(id);
        current
    };
    // The normal frontend position command supplies the new active rectangle.
    let selected = select_bound(app, id, binding);
    if owner.is_none() {
        selected.map_err(|error| format!("TAB_RENDERER_ACTIVATION_PARTIAL:{id}:{error}"))?;
        if !viewport_restored {
            return Err(format!("TAB_RENDERER_ACTIVATION_PARTIAL:{id}:TAB_VIEWPORT_RESTORE_UNCONFIRMED"));
        }
        Ok(())
    } else {
        selected
    }
}

/// Explicit user recovery at a NEW address, not automatic replay of the
/// previous document. Caller owns Browser admission and tab transition.
/// Keep the logical tab and do not implicitly select a background tab.
pub(super) fn reopen_absent(app: &AppHandle, id: &str, url: &str, absence_epoch: u64) -> Result<TabInfo, String> {
    let index = exists(app, id)?;
    if !crate::grid_ipc::explicit_reopen_address(url) {
        return Err("TAB_RENDERER_REOPEN_INVALID".into());
    }
    let state = app.state::<AppState>();
    let (owner, status, _) = state.grid_manager.tab_renderer_inspection(index)?;
    if owner.is_some() || status != "absent"
        || state.hibernated_tabs.lock().unwrap().contains(id) {
        return Err("TAB_RENDERER_REOPEN_STALE".into());
    }
    state.grid_manager.start_tab_renderer_at_absence(index, url, absence_epoch)?;
    state.hibernated_tabs.lock().unwrap().remove(id);
    state.hibernated_scroll.lock().unwrap().remove(id);
    let binding = state.grid_manager.renderer_binding(index)
        .map_err(|error| format!("TAB_RENDERER_REOPEN_PARTIAL:{id}:{error}"))?;
    state.grid_manager.publish_reopened_tab(binding)
        .map_err(|error| format!("TAB_RENDERER_REOPEN_PARTIAL:{id}:{error}"))
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

pub(super) type LayoutAdmission = (u32, Option<crate::grid_ipc::RendererBinding>);

pub(super) fn capture_layout(app: &AppHandle, id: &str) -> Result<LayoutAdmission, String> {
    let index = exists(app, id)?;
    let target = app.state::<AppState>().grid_manager.capture_window_route(index)?;
    Ok((index, target))
}

pub(super) fn position_admitted(app: &AppHandle, id: &str, css: (f64, f64, f64, f64),
    (index, target): LayoutAdmission,
) -> Result<(), String> {
    if exists(app, id)? != index { return Err("TAB_RENDERER_LAYOUT_SUPERSEDED".into()); }
    app.state::<AppState>().grid_manager.position_tab_renderer_bound(index, target, css)
}

pub(super) fn hide_admitted(app: &AppHandle, id: &str, (index, target): LayoutAdmission) -> Result<(), String> {
    if exists(app, id)? != index { return Err("TAB_RENDERER_LAYOUT_SUPERSEDED".into()); }
    app.state::<AppState>().grid_manager.hide_tab_renderer_bound(index, target)
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
    // Capture all logical peers before the first native wait.
    let targets: Vec<_> = ids.into_iter().map(|id| {
        let target = capture_layout(app, &id);
        (id, target)
    }).collect();
    let mut errors = Vec::new();
    for (id, target) in targets {
        if let Err(error) = target.and_then(|admitted| hide_admitted(app, &id, admitted)) {
            errors.push(format!("{id}:{error}"));
        }
    }
    if let Err(error) = state.grid_manager.hide_grid_renderers() {
        errors.push(error);
    }
    if errors.is_empty() { Ok(()) }
    else { Err(format!("TAB_RENDERER_HIDE_PARTIAL:{}", errors.join("; "))) }
}
