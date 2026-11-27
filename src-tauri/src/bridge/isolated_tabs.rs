//! Process-lifetime ordinary-tab backend selection. Browser metadata remains
//! in AppState.tabs; the shared native process manager owns the actual child.
//! No runtime migration of an existing WebView, cookie copy, or fallback.
use super::*;
use crate::grid_ipc::{tab_renderer_index, GridCmd};

// One backend for the entire process lifetime. Routing an existing logical
// tab must not depend on a later environment read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BackendSelection {
    isolated: bool,
    reason: &'static str,
}
static ISOLATED_MODE: std::sync::OnceLock<BackendSelection> = std::sync::OnceLock::new();

fn select_backend(supported: bool, child: bool, release: bool,
    setting: Option<&str>, available: impl FnOnce() -> bool) -> BackendSelection
{
    let embedded = |reason| BackendSelection { isolated: false, reason };
    if child { return embedded("renderer_child"); }
    if !supported { return embedded("unsupported_platform"); }
    match setting {
        Some("0") => return embedded("explicit_embedded"),
        Some("1") => {
            // Preserve the existing explicit strict opt-in. A failed native
            // spawn is reported, never substituted with an embedded page.
            return BackendSelection { isolated: true, reason: "explicit_isolated" };
        }
        Some(_) => return embedded("invalid_backend_setting"),
        None if !release => return embedded("development_embedded"),
        None => {}
    }
    if available() {
        BackendSelection { isolated: true, reason: "default_isolated_kernel_ready" }
    } else {
        // Only the initial empty-host selection may choose compatibility
        // mode. No renderer has been started and no document is migrated.
        embedded("kernel_containment_unavailable")
    }
}

fn kernel_ready() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Probe only a fresh, empty owned group in the delegated domain.
        // No renderer or application payload is spawned by this selection.
        let Ok(Some(scope)) = crate::shared::process_scope::ProcessScope::prepare() else { return false; };
        scope.configure_memory_limit(crate::grid_ipc::TAB_RENDERER_MEMORY_LIMIT).is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        crate::windows_process::JobObject::create_with_memory_limit(crate::grid_ipc::TAB_RENDERER_MEMORY_LIMIT).is_ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    false
}

fn selection() -> &'static BackendSelection {
    ISOLATED_MODE.get_or_init(|| {
        let setting = std::env::var_os("BROWSEROS_EXPERIMENTAL_ISOLATED_TABS");
        let value = setting.as_ref().map(|value| value.to_str().unwrap_or("<invalid>"));
        select_backend(cfg!(any(target_os = "linux", target_os = "windows")),
            std::env::var_os("GRID_CHILD_INDEX").is_some(), !cfg!(debug_assertions),
            value, kernel_ready)
    })
}

/// Called once after supervisor/delegation startup and before any GUI tab.
pub(crate) fn initialize() {
    let chosen = selection();
    eprintln!("[browser-backend] isolated={} reason={}", chosen.isolated, chosen.reason);
}

pub(crate) fn mode_enabled() -> bool { selection().isolated }
pub(crate) fn mode_reason() -> &'static str { selection().reason }

#[cfg(test)]
mod backend_selection_tests {
    use super::*;
    #[test]
    fn release_defaults_to_isolated_only_with_kernel_preflight() {
        assert!(select_backend(true, false, true, None, || true).isolated);
        assert_eq!(select_backend(true, false, true, None, || false).reason, "kernel_containment_unavailable");
        assert!(!select_backend(true, false, true, Some("0"), || panic!("opt out must not probe")).isolated);
        assert!(select_backend(true, false, false, Some("1"), || panic!("explicit strict route must not fall back")).isolated);
        assert!(!select_backend(true, true, true, Some("1"), || panic!("child must not probe")).isolated);
        assert!(!select_backend(false, false, true, None, || panic!("unsupported must not probe")).isolated);
        assert!(!select_backend(true, false, false, None, || panic!("development must not probe")).isolated);
    }
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
    let created = match state.grid_manager.start_tab_renderer(index, &url) {
        Ok(binding) => binding,
        Err(error) => {
            // This logical ID has never been published and cannot be reused.
            // Even a failed spawn may own a listener or retirement.
            return match state.grid_manager.close_tab_renderer(index, true) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!("{error}; {cleanup}")),
            };
        }
    };
    let info = TabInfo {
        id: id.clone(),
        url: url.clone(),
        title: parsed.host_str().unwrap_or(url.as_str()).to_owned(),
    };
    let info = match state.grid_manager.publish_tab_renderer_info(created, info) {
        Ok(info) => info,
        Err(error) => {
            // The create receipt cannot authorize closing another owner.
            return match state.grid_manager.close_tab_renderer_bound(index, true, Some(created.owner)) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!("{error}; {cleanup}")),
            };
        }
    };
    // The tab is already published. Report the partial outcome explicitly
    // so the UI reconciles its logical list instead of creating a duplicate.
    let selection = select_bound(app, &id, created);
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

/// Window activation is not document execution. Bind live process/connection
/// or the precise native absence and logical sleeping state before admission.
#[derive(Clone, Copy)]
pub(super) enum ActivationAdmission {
    Live(crate::grid_ipc::RendererBinding),
    Absent { epoch:u64, hibernated:bool },
}

pub(super) fn capture_activation(app:&AppHandle,id:&str) -> Result<ActivationAdmission,String> {
    let index=exists(app,id)?;
    let state=app.state::<AppState>();
    if let Some(binding)=state.grid_manager.capture_window_route(index)? {
        return Ok(ActivationAdmission::Live(binding));
    }
    let epoch=state.grid_manager.tab_renderer_absence_epoch(index)?;
    let hibernated=state.hibernated_tabs.try_lock()
        .map_err(|_|"TAB_RENDERER_OBSERVATION_BUSY")?.contains(id);
    Ok(ActivationAdmission::Absent{epoch,hibernated})
}
pub(super) fn activation_current(app:&AppHandle,id:&str,expected:ActivationAdmission) -> bool {
    match (expected,capture_activation(app,id)) {
        (ActivationAdmission::Live(old),Ok(ActivationAdmission::Live(current))) =>
            old.index==current.index && old.owner==current.owner && old.generation==current.generation,
        (ActivationAdmission::Absent{epoch,hibernated},Ok(ActivationAdmission::Absent{epoch:now,hibernated:sleeping})) =>
            epoch==now && hibernated==sleeping,
        _=>false,
    }
}

/// Explicit activation may recreate a missing host at a supported address.
/// No fresh absence or replacement connection can inherit old admission.
pub(super) fn activate_admitted(app: &AppHandle, id: &str, admitted: Option<ActivationAdmission>) -> Result<(), String> {
    let index = exists(app, id)?;
    let state = app.state::<AppState>();
    let admission=match admitted {
        Some(binding)=>binding,
        None=>capture_activation(app,id)?,
    };
    if !activation_current(app,id,admission) {
        return Err("TAB_RENDERER_ACTIVATION_STALE".into());
    }
    match admission {
        ActivationAdmission::Live(binding)=>{
            // Selecting an already-created native window is independent of
            // page navigation and optional cumulative download telemetry.
            select_bound(app,id,binding)?;
            state.hibernated_tabs.lock().unwrap().remove(id);
            Ok(())
        }
        ActivationAdmission::Absent{epoch,..}=>{
            let url=state.tabs.lock().unwrap().get(id).map(|tab|tab.url.clone())
                .ok_or("TAB_NOT_FOUND")?;
            if !crate::grid_ipc::restorable_address(&url) {
                return Err("TAB_RESTORE_URL_UNSUPPORTED".into());
            }
            let initial=state.grid_manager.start_tab_renderer_at_absence(index,&url,epoch)?;
            state.hibernated_tabs.lock().unwrap().remove(id);
            // Once a replacement exists, consume the old checkpoint before
            // any fallible binding observation. Never carry it to a later host.
            let snapshot=state.hibernated_scroll.lock().unwrap().remove(id);
            let window=|| state.grid_manager.capture_window_route(index)?
                .ok_or_else(||"TAB_RENDERER_WINDOW_UNAVAILABLE".to_owned());
            let partial=|error:String|format!("TAB_RENDERER_ACTIVATION_PARTIAL:{id}:{error}");
            let mut restored=snapshot.is_none();
            if let Some((address,(x,y)))=snapshot {
                if address==url && crate::grid_ipc::valid_viewport(x,y) {
                    // A document may still be loading. Retry admission only
                    // while no viewport operation has entered the native host.
                    for _ in 0..4 {
                        if state.shutdown_requested.load(Ordering::Acquire) {break;}
                        let Ok(current)=window() else {break;};
                        if current.owner!=initial.owner || current.generation!=initial.generation {break;}
                        if !state.tabs.lock().unwrap().get(id).is_some_and(|tab|tab.url==address) {break;}
                        let document=match state.grid_manager.renderer_binding(index) {
                            Ok(document) if document.owner==initial.owner && document.generation==initial.generation=>document,
                            Err(error) if error=="TAB_RENDERER_DOCUMENT_UNAVAILABLE"=>{
                                std::thread::sleep(std::time::Duration::from_millis(250));continue;
                            }
                            _=>break,
                        };
                        match state.grid_manager.renderer_request_data(document,GridCmd::ViewportRestore{
                            id:id.to_owned(),address:address.clone(),x,y,
                        },1000) {
                            Ok(Some(value)) if value=="applied" || value=="skip_nonzero"=>{restored=true;break;}
                            Ok(Some(value)) if value=="deferred"=>{
                                std::thread::sleep(std::time::Duration::from_millis(250));
                            }
                            _=>break, // Never replay ambiguous or partially applied scroll.
                        }
                    }
                }
            }
            let current=window().map_err(partial)?;
            if current.owner!=initial.owner || current.generation!=initial.generation {
                return Err(partial("TAB_VIEWPORT_STALE".into()));
            }
            select_bound(app,id,current).map_err(partial)?;
            if !restored {
                return Err(partial("TAB_VIEWPORT_RESTORE_UNCONFIRMED".into()));
            }
            Ok(())
        }
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
    let binding = state.grid_manager.start_tab_renderer_at_absence(index, url, absence_epoch)?;
    state.hibernated_tabs.lock().unwrap().remove(id);
    state.hibernated_scroll.lock().unwrap().remove(id);
    // A recovered host can acknowledge creation before its first document
    // stamp. Publish the actual owner's navigation metadata immediately;
    // document operations still require a fresh document binding.
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
    // Native manager holds one visibility dispatch across both domains,
    // including unpublished, restarting and retiring renderer ownership.
    app.state::<AppState>().grid_manager.hide_all_renderer_surfaces()
}
