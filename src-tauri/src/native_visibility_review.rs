//! Explicit, disposable-profile installed GUI fixture. No public IPC command,
//! no production screenshot API, no page-supplied proof. A separate desktop
//! observer must acknowledge each physical screen sample before the next step.
use crate::{
    bridge,
    grid_ipc::{IpcRect, NativeWindowReceipt, RendererBinding},
    AppState,
};
use serde::Serialize;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::{AppHandle, Manager};

const LEFT: (f64, f64, f64, f64) = (80.0, 180.0, 320.0, 240.0);
const RIGHT: (f64, f64, f64, f64) = (450.0, 180.0, 320.0, 240.0);

pub(crate) struct Review {
    base: url::Url,
    nonce: String,
}

fn canonical_target(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .ok_or("VISIBILITY_REVIEW_DATA_ROOT_INVALID")?;
    let parent = path.parent().ok_or("VISIBILITY_REVIEW_DATA_ROOT_INVALID")?;
    Ok(parent
        .canonicalize()
        .map_err(|_| "VISIBILITY_REVIEW_DATA_ROOT_INVALID")?
        .join(name))
}

impl Review {
    /// Before workspace/profile preparation. A caller cannot opt an existing
    /// account's BrowserOS data into the fixture. Windows uses a disposable
    /// OS account, not an APPDATA environment override as isolation proof.
    pub(crate) fn preflight(app: &AppHandle) -> Result<Option<Self>, String> {
        if std::env::var_os("BROWSEROS_VISIBILITY_REVIEW").is_none() {
            return Ok(None);
        }
        if cfg!(debug_assertions)
            || std::env::var("BROWSEROS_VISIBILITY_REVIEW").as_deref() != Ok("1")
            || std::env::var_os("BROWSEROS_EXPERIMENTAL_ISOLATED_TABS").is_some()
            || bridge::isolated_tabs::mode_reason() != "default_isolated_kernel_ready"
            || [
                "GRID_SELFTEST",
                "GRID_GUI_REGRESSION",
                "NATIVE_58_GUI_REVIEW",
                "M0_RUN_ID",
                "MVP_WORKBENCH_SMOKE",
            ]
            .iter()
            .any(|key| std::env::var_os(key).is_some())
        {
            return Err("VISIBILITY_REVIEW_RELEASE_DEFAULT_REQUIRED".into());
        }
        let base = url::Url::parse(
            &std::env::var("BROWSEROS_VISIBILITY_REVIEW_BASE")
                .map_err(|_| "VISIBILITY_REVIEW_BASE_REQUIRED")?,
        )
        .map_err(|_| "VISIBILITY_REVIEW_BASE_INVALID")?;
        if base.scheme() != "http"
            || base.host_str() != Some("127.0.0.1")
            || base.port().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err("VISIBILITY_REVIEW_LOOPBACK_REQUIRED".into());
        }
        let nonce = std::env::var("BROWSEROS_VISIBILITY_REVIEW_NONCE")
            .map_err(|_| "VISIBILITY_REVIEW_NONCE_REQUIRED")?;
        if nonce.len() != 32
            || !nonce
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err("VISIBILITY_REVIEW_NONCE_INVALID".into());
        }
        let requested = PathBuf::from(
            std::env::var_os("BROWSEROS_VISIBILITY_REVIEW_DATA_ROOT")
                .ok_or("VISIBILITY_REVIEW_DATA_ROOT_REQUIRED")?,
        );
        let actual = app
            .path()
            .app_data_dir()
            .map_err(|_| "VISIBILITY_REVIEW_DATA_ROOT_INVALID")?;
        if !requested.is_absolute()
            || canonical_target(&requested)? != canonical_target(&actual)?
            || actual.file_name().and_then(|name| name.to_str()) != Some("com.jizhijiandan.mvp")
        {
            return Err("VISIBILITY_REVIEW_DATA_ROOT_MISMATCH".into());
        }
        // A fresh path only: even an existing empty profile may belong to
        // another process. No deletion, migration or adoption by the runner.
        match std::fs::symlink_metadata(&actual) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("VISIBILITY_REVIEW_EXISTING_PROFILE_DENIED".into()),
        }
        #[cfg(target_os = "linux")]
        {
            let data = PathBuf::from(
                std::env::var_os("XDG_DATA_HOME").ok_or("VISIBILITY_REVIEW_XDG_REQUIRED")?,
            );
            let root = data
                .parent()
                .ok_or("VISIBILITY_REVIEW_XDG_REQUIRED")?
                .canonicalize()
                .map_err(|_| "VISIBILITY_REVIEW_XDG_REQUIRED")?;
            if !root.starts_with(
                std::env::temp_dir()
                    .canonicalize()
                    .map_err(|_| "VISIBILITY_REVIEW_XDG_REQUIRED")?,
            ) || !root
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("browseros-visibility-"))
                || canonical_target(&actual)?.parent()
                    != Some(
                        data.canonicalize()
                            .map_err(|_| "VISIBILITY_REVIEW_XDG_REQUIRED")?
                            .as_path(),
                    )
            {
                return Err("VISIBILITY_REVIEW_XDG_REQUIRED".into());
            }
            for key in ["HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
                let path =
                    PathBuf::from(std::env::var_os(key).ok_or("VISIBILITY_REVIEW_XDG_REQUIRED")?)
                        .canonicalize()
                        .map_err(|_| "VISIBILITY_REVIEW_XDG_REQUIRED")?;
                if !path.starts_with(&root) || path == root {
                    return Err("VISIBILITY_REVIEW_XDG_REQUIRED".into());
                }
            }
        }
        Ok(Some(Self { base, nonce }))
    }

    fn fixture(&self, name: &str) -> String {
        self.base
            .join(&format!("fixture/{name}"))
            .expect("fixed fixture route")
            .to_string()
    }

    pub(crate) fn start(self, app: AppHandle) -> Result<(), String> {
        std::thread::Builder::new()
            .name("native-visibility-review".into())
            .spawn(move || {
                // All real window work uses the same native worker APIs as the
                // existing GUI regression. Neither the desktop observer nor a
                // fixture page can issue BrowserOS commands or change test steps.
                let client = reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .retry(reqwest::retry::never())
                    .timeout(Duration::from_secs(35))
                    .build();
                let result = match client {
                    Ok(client) => {
                        let result = self.run(&app, &client);
                        let acknowledged = self.post(
                            &client,
                            "finished",
                            json!({"ok":result.is_ok(),
                        "error":result.as_ref().err(), "pid":std::process::id()}),
                        );
                        result.and(acknowledged)
                    }
                    Err(_) => Err("VISIBILITY_REVIEW_HTTP_UNAVAILABLE".into()),
                };
                if let Err(error) = result {
                    eprintln!("[visibility-review] incomplete: {error}");
                }
                // Same graceful owner cleanup on success and failure; the
                // observer also requires launcher exit and all sampled PIDs gone.
                crate::begin_graceful_shutdown(app, "native_visibility_review");
            })
            .map(|_| ())
            .map_err(|_| "VISIBILITY_REVIEW_WORKER_UNAVAILABLE".into())
    }

    fn post(
        &self,
        client: &reqwest::Client,
        route: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let endpoint = self
            .base
            .join(route)
            .map_err(|_| "VISIBILITY_REVIEW_BASE_INVALID")?;
        tauri::async_runtime::block_on(async {
            let reply = client
                .post(endpoint)
                .bearer_auth(&self.nonce)
                .json(&value)
                .send()
                .await
                .map_err(|_| "VISIBILITY_REVIEW_OBSERVER_UNAVAILABLE")?;
            if reply.status() != reqwest::StatusCode::NO_CONTENT {
                return Err("VISIBILITY_REVIEW_DESKTOP_EVIDENCE_REJECTED".into());
            }
            Ok(())
        })
    }

    fn run(&self, app: &AppHandle, client: &reqwest::Client) -> Result<(), String> {
        let state = app.state::<AppState>();
        let mgr = &state.grid_manager;
        // Wait for the normal main window, not an invisible or extracted
        // WebView. A subsequent unexpected frontend mutation fails the test.
        std::thread::sleep(Duration::from_secs(3));
        if !state.tabs.lock().unwrap().is_empty() || !mgr.renderer_processes().is_empty() {
            return Err("VISIBILITY_REVIEW_FRESH_HOST_REQUIRED".into());
        }
        let window = app
            .get_window("main")
            .ok_or("VISIBILITY_REVIEW_MAIN_MISSING")?;
        let size = window
            .inner_size()
            .map_err(|_| "VISIBILITY_REVIEW_MAIN_BOUNDS")?;
        let scale = window
            .scale_factor()
            .map_err(|_| "VISIBILITY_REVIEW_MAIN_BOUNDS")?;
        if !scale.is_finite()
            || scale <= 0.0
            || f64::from(size.width) / scale < 800.0
            || f64::from(size.height) / scale < 460.0
            || !window.is_visible().unwrap_or(false)
        {
            return Err("VISIBILITY_REVIEW_MAIN_BOUNDS".into());
        }
        let left = mgr.abs_rect(LEFT).ok_or("VISIBILITY_REVIEW_MAIN_BOUNDS")?;
        let right = mgr.abs_rect(RIGHT).ok_or("VISIBILITY_REVIEW_MAIN_BOUNDS")?;
        left.validate()?;
        right.validate()?;
        let regions = [left, right];
        let mut sequence = 0u32;
        let mut stage = |phase: &str,
                         markers: [Option<&str>; 2],
                         targets: &[(u32, Option<IpcRect>)]|
         -> Result<(), String> {
            sequence += 1;
            if mgr.abs_rect(LEFT) != Some(left) || mgr.abs_rect(RIGHT) != Some(right) {
                return Err("VISIBILITY_REVIEW_MAIN_GEOMETRY_CHANGED".into());
            }
            let before = observe(app, targets)?;
            let actual =
                std::env::current_exe().map_err(|_| "VISIBILITY_REVIEW_EXECUTABLE_UNKNOWN")?;
            self.post(
                client,
                "stage",
                json!({
                    "sequence":sequence,"phase":phase,"pid":std::process::id(),
                    "executable":actual,"backend":bridge::isolated_tabs::mode_reason(),
                    "regions":regions,"markers":markers,"receipts":before,
                }),
            )?;
            if observe(app, targets)? != before
                || mgr.abs_rect(LEFT) != Some(left)
                || mgr.abs_rect(RIGHT) != Some(right)
            {
                return Err("VISIBILITY_REVIEW_OWNER_OR_GEOMETRY_CHANGED".into());
            }
            Ok(())
        };
        stage("baseline", [None, None], &[])?;

        let a = bridge::tab_new_native(app.clone(), self.fixture("a"))?;
        let ai = crate::grid_ipc::tab_renderer_index(&a.id).ok_or("VISIBILITY_REVIEW_TAB_ROUTE")?;
        crate::native_tab_position(app.clone(), a.id.clone(), LEFT.0, LEFT.1, LEFT.2, LEFT.3)?;
        stage("tab_a_show", [Some("a"), None], &[(ai, Some(left))])?;

        // Creation selects B and hides A through the real Browser owner.
        let b = bridge::tab_new_native(app.clone(), self.fixture("b"))?;
        let bi = crate::grid_ipc::tab_renderer_index(&b.id).ok_or("VISIBILITY_REVIEW_TAB_ROUTE")?;
        crate::native_tab_position(
            app.clone(),
            b.id.clone(),
            RIGHT.0,
            RIGHT.1,
            RIGHT.2,
            RIGHT.3,
        )?;
        stage(
            "tab_b_switch",
            [None, Some("b")],
            &[(ai, None), (bi, Some(right))],
        )?;

        // Move the same native owner. The old screen region must no longer
        // contain B; a successful layout receipt alone is insufficient.
        crate::native_tab_position(app.clone(), b.id.clone(), LEFT.0, LEFT.1, LEFT.2, LEFT.3)?;
        stage(
            "tab_b_move",
            [Some("b"), None],
            &[(ai, None), (bi, Some(left))],
        )?;

        crate::native_tab_activate(app.clone(), a.id.clone())?;
        crate::native_tab_position(
            app.clone(),
            a.id.clone(),
            RIGHT.0,
            RIGHT.1,
            RIGHT.2,
            RIGHT.3,
        )?;
        stage(
            "tab_a_return",
            [None, Some("a")],
            &[(ai, Some(right)), (bi, None)],
        )?;

        crate::native_hide_all_webviews(app.clone())?;
        stage("tabs_hidden", [None, None], &[(ai, None), (bi, None)])?;

        if bridge::create_grid_blocking(app.clone(), 2, vec![self.fixture("c"), self.fixture("d")])?
            != 2
        {
            return Err("VISIBILITY_REVIEW_GRID_COUNT".into());
        }
        bridge::grid_position_blocking(app.clone(), 0, LEFT.0, LEFT.1, LEFT.2, LEFT.3)?;
        bridge::grid_position_blocking(app.clone(), 1, RIGHT.0, RIGHT.1, RIGHT.2, RIGHT.3)?;
        stage(
            "grid_show",
            [Some("c"), Some("d")],
            &[(ai, None), (bi, None), (0, Some(left)), (1, Some(right))],
        )?;

        crate::native_hide_all_webviews(app.clone())?;
        stage(
            "grid_hidden",
            [None, None],
            &[(ai, None), (bi, None), (0, None), (1, None)],
        )?;

        bridge::close_grid_blocking(app.clone())?;
        bridge::tab_close_native(app.clone(), a.id)?;
        bridge::tab_close_native(app.clone(), b.id)?;
        if !state.tabs.lock().unwrap().is_empty()
            || !mgr.renderer_processes().is_empty()
            || [ai, bi, 0, 1]
                .into_iter()
                .any(|index| !matches!(mgr.capture_window_route(index), Ok(None)))
        {
            return Err("VISIBILITY_REVIEW_CLEANUP_UNCONFIRMED".into());
        }
        stage("closed", [None, None], &[])?;
        Ok(())
    }
}

#[derive(Serialize, PartialEq)]
struct ObservedWindow {
    index: u32,
    pid: u32,
    spawn_epoch: u64,
    generation: u64,
    navigation_revision: u64,
    receipt: NativeWindowReceipt,
}

fn observe(
    app: &AppHandle,
    targets: &[(u32, Option<IpcRect>)],
) -> Result<Vec<ObservedWindow>, String> {
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    let mut observations = Vec::with_capacity(targets.len());
    for &(index, rect) in targets {
        let binding = mgr
            .capture_window_route(index)?
            .ok_or("VISIBILITY_REVIEW_WINDOW_MISSING")?;
        let receipt = mgr
            .last_window_receipt(index, binding.owner)?
            .ok_or("VISIBILITY_REVIEW_RECEIPT_MISSING")?;
        receipt.validate_for(rect)?;
        let after = mgr
            .capture_window_route(index)?
            .ok_or("VISIBILITY_REVIEW_WINDOW_MISSING")?;
        let same = |a: RendererBinding, b: RendererBinding| a == b;
        if !same(binding, after) {
            return Err("VISIBILITY_REVIEW_WINDOW_REPLACED".into());
        }
        observations.push(ObservedWindow {
            index,
            pid: binding.owner.0,
            spawn_epoch: binding.owner.1,
            generation: binding.generation,
            navigation_revision: binding.navigation_revision,
            receipt,
        });
    }
    // No extra published native renderer may be ignored by the observer.
    let owners = mgr.renderer_processes();
    if owners.len() != observations.len()
        || owners.iter().any(|owner| {
            !observations.iter().any(|window| {
                window.index == owner.index
                    && window.pid == owner.pid
                    && window.spawn_epoch == owner.spawn_epoch
            })
        })
    {
        return Err("VISIBILITY_REVIEW_UNEXPECTED_RENDERER".into());
    }
    Ok(observations)
}
