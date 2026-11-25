use crate::models::{
    BrowserTabError, BrowserTabEvent, CreateTabOptions, LogicalRect, Result, TabId,
};
use crate::platform;
use parking_lot::RwLock;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(not(target_os = "linux"))]
use std::sync::Mutex;
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(not(target_os = "linux"))]
use tauri::webview::PageLoadEvent;
use tauri::{
    webview::NewWindowResponse, AppHandle, Emitter, Manager, State, Webview, WebviewBuilder,
    WebviewUrl, Window, Wry,
};

/// M1-8：资源事件时间戳（epoch 毫秒）。
#[cfg(target_os = "linux")]
fn epoch_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

// External browser tabs share one dedicated persistent WebKit context.
// A WebContext without an explicit data_directory does not configure WebKit's
// persistent session storage; a Bilibili login can disappear on restart.
// Keep the browser profile separate from the trusted local app UI WebView and
// never expose its contents through native commands or the BFC readback.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn browser_profile_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("browser-webview-profile")
}

#[cfg(target_os = "linux")]
fn ensure_private_browser_profile(app: &AppHandle<Wry>) -> Result<PathBuf> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|_| BrowserTabError::Platform("browser profile directory unavailable".into()))?;
    let profile = browser_profile_path(&base);
    if let Ok(meta) = std::fs::symlink_metadata(&profile) {
        if !meta.file_type().is_dir() {
            return Err(BrowserTabError::Platform(
                "browser profile is not a directory".into(),
            ));
        }
    }
    std::fs::create_dir_all(&profile)
        .map_err(|_| BrowserTabError::Platform("cannot create browser profile".into()))?;
    let canonical_base = std::fs::canonicalize(&base)
        .map_err(|_| BrowserTabError::Platform("browser profile base unavailable".into()))?;
    let canonical_profile = std::fs::canonicalize(&profile)
        .map_err(|_| BrowserTabError::Platform("browser profile unavailable".into()))?;
    if canonical_profile != canonical_base.join("browser-webview-profile") {
        return Err(BrowserTabError::Platform(
            "browser profile path invalid".into(),
        ));
    }
    std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o700))
        .map_err(|_| BrowserTabError::Platform("cannot secure browser profile".into()))?;
    Ok(profile)
}

// Browser sub-WebViews use a single app-owned WebView2 directory. Native
// owner/SYSTEM checks are shared with app-data storage; display names, locales
// and command output never participate in an authorization decision.
#[cfg(target_os = "windows")]
fn browser_profile_security_options() -> browseros_platform_security::windows::PrivateTreeOptions {
    browseros_platform_security::windows::PrivateTreeOptions {
        max_objects: 200_000,
        max_depth: 128,
        max_repairs: 200_000,
        max_duration: Duration::from_secs(30),
    }
}

#[cfg(all(test, target_os = "windows"))]
fn secure_windows_browser_profile(
    profile: &Path,
) -> Result<browseros_platform_security::windows::PrivateTreeReport> {
    browseros_platform_security::windows::ensure_private_directory(
        profile,
        &browser_profile_security_options(),
    )
    .map_err(|error| BrowserTabError::Platform(format!("BROWSER_PROFILE_ACL_MIGRATION: {error}")))
}

#[cfg(all(test, target_os = "windows"))]
fn audit_windows_profile_acl(
    profile: &Path,
    deep: bool,
) -> Result<browseros_platform_security::windows::PrivateTreeReport> {
    browseros_platform_security::windows::audit_private_directory(
        profile,
        deep,
        &browser_profile_security_options(),
    )
    .map_err(|error| BrowserTabError::Platform(format!("BROWSER_PROFILE_ACL_AUDIT: {error}")))
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod browser_profile_tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_profile_acl_contains_only_current_user_and_system() {
        let dir = std::env::temp_dir().join(format!(
            "browseros-webview-acl-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("profile.dat"), b"fixture").unwrap();
        let secured = secure_windows_browser_profile(&dir).expect("native owner-only profile ACL");
        let audited = audit_windows_profile_acl(&dir, true).expect("native ACE identity audit");
        assert_eq!(audited.checked_objects, 2);
        assert_eq!(secured.root_identity, audited.root_identity);
        assert_eq!(
            secure_windows_browser_profile(&dir)
                .unwrap()
                .repaired_objects,
            0
        );
        assert_eq!(std::fs::read(dir.join("profile.dat")).unwrap(), b"fixture");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_browser_tabs_share_one_app_scoped_profile_directory() {
        let base = Path::new("/some-app-user-data");
        assert_eq!(
            browser_profile_path(base),
            base.join("browser-webview-profile")
        );
        assert!(!browser_profile_path(base)
            .to_string_lossy()
            .contains("tab-1"));
        assert!(!browser_profile_path(base).to_string_lossy().contains("bfc"));
    }
}

// Admission protects the native identity until a synchronous dispatcher
// call returns. No registry or admission lock is held inside that call.
// Ordinary calls may overlap; layout mutations are serialized separately.
// A lifecycle change fails fast while any call for the same identity is active.
#[derive(Default)]
struct TabAccess {
    lifecycle: bool,
    operations: usize,
    layout: bool,
}

#[derive(Clone, Copy)]
enum TabAccessKind {
    Lifecycle,
    Operation,
    Layout,
}

struct TabReservation<'a> {
    busy: &'a RwLock<HashMap<TabId, TabAccess>>,
    id: TabId,
    kind: TabAccessKind,
}

impl Drop for TabReservation<'_> {
    fn drop(&mut self) {
        let mut busy = self.busy.write();
        if let Some(access) = busy.get_mut(&self.id) {
            match self.kind {
                TabAccessKind::Lifecycle => access.lifecycle = false,
                TabAccessKind::Operation => access.operations -= 1,
                TabAccessKind::Layout => {
                    access.operations -= 1;
                    access.layout = false;
                }
            }
            if !access.lifecycle && access.operations == 0 {
                busy.remove(&self.id);
            }
        }
    }
}

fn reserve_tab_access<'a>(
    busy: &'a RwLock<HashMap<TabId, TabAccess>>,
    id: &TabId,
    kind: TabAccessKind,
) -> Result<TabReservation<'a>> {
    let mut entries = busy.write();
    let access = entries.entry(id.clone()).or_default();
    match kind {
        TabAccessKind::Lifecycle if access.lifecycle || access.operations > 0 => {
            return Err(BrowserTabError::Platform("TAB_LIFECYCLE_BUSY".into()));
        }
        TabAccessKind::Operation | TabAccessKind::Layout if access.lifecycle => {
            return Err(BrowserTabError::Platform("TAB_LIFECYCLE_BUSY".into()));
        }
        TabAccessKind::Layout if access.layout => {
            return Err(BrowserTabError::Platform("TAB_LAYOUT_BUSY".into()));
        }
        TabAccessKind::Lifecycle => access.lifecycle = true,
        TabAccessKind::Operation => access.operations += 1,
        TabAccessKind::Layout => {
            access.operations += 1;
            access.layout = true;
        }
    }
    Ok(TabReservation {
        busy,
        id: id.clone(),
        kind,
    })
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
struct TabLayout {
    rect: LogicalRect,
    // Only the explicit plugin visibility API restores this rectangle.
    // Ordinary BrowserOS offscreen positioning stays authoritative.
    restore_rect: Option<LogicalRect>,
}

#[cfg(target_os = "linux")]
fn offscreen_rect(rect: LogicalRect) -> LogicalRect {
    LogicalRect::new(-30000.0, rect.y, rect.width, rect.height)
}

struct ManagedTab {
    webview: Webview<Wry>,
    document_revision: Arc<std::sync::atomic::AtomicU64>,
    #[cfg(target_os = "linux")]
    layout: RwLock<TabLayout>,
    #[cfg(not(target_os = "linux"))]
    last_rect: RwLock<LogicalRect>,
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct BoundedPngWriter(Vec<u8>);

#[cfg(target_os = "linux")]
impl std::io::Write for BoundedPngWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > 32 * 1024 * 1024 - self.0.len() {
            return Err(std::io::Error::other("CAPTURE_SIZE_LIMIT"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
#[test]
fn png_writer_rejects_a_chunk_before_exceeding_its_limit() {
    use std::io::Write;
    let mut writer = BoundedPngWriter(vec![0; 32 * 1024 * 1024 - 2]);
    assert_eq!(writer.write(&[1, 2]).unwrap(), 2);
    assert!(writer.write(&[3]).is_err());
    assert_eq!(writer.0.len(), 32 * 1024 * 1024);
}

/// One request owns its cancellation signal. A request to cancel is NOT a
/// terminal receipt: the host must retain admission until native completion.
/// WebKit accepts GCancellable, but its renderer may still finish a snapshot.
/// WebView2 has no per-CDP-call cancellation API here.
#[derive(Clone)]
pub struct CaptureCancellation {
    requested: Arc<std::sync::atomic::AtomicBool>,
    #[cfg(target_os = "linux")]
    native: webkit2gtk::gio::Cancellable,
}

impl CaptureCancellation {
    pub fn new() -> Self {
        Self {
            requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            #[cfg(target_os = "linux")]
            native: webkit2gtk::gio::Cancellable::new(),
        }
    }

    pub fn cancel(&self) {
        use std::sync::atomic::Ordering;
        if !self.requested.swap(true, Ordering::AcqRel) {
            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::gio::prelude::CancellableExt;
                self.native.cancel();
            }
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.requested.load(std::sync::atomic::Ordering::Acquire)
    }
}

impl Default for CaptureCancellation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(target_os = "linux", target_os = "windows", test))]
fn complete_capture<T>(pending: &parking_lot::Mutex<Option<impl FnOnce(T)>>, result: T) {
    let callback = pending.lock().take();
    if let Some(callback) = callback {
        callback(result);
    }
}

const MAX_CAPTURE_EDGE: f64 = 16_384.0;
const MAX_CAPTURE_PIXELS: f64 = 16_000_000.0;

#[cfg(any(target_os = "linux", target_os = "windows", test))]
fn checked_capture_pixels(width: f64, height: f64, scale: f64) -> std::result::Result<(), String> {
    if !width.is_finite() || !height.is_finite() || !scale.is_finite()
        || width <= 0.0 || height <= 0.0 || scale <= 0.0
    {
        return Err("CAPTURE_METRICS_INVALID".into());
    }
    let width = (width * scale).ceil();
    let height = (height * scale).ceil();
    if width > MAX_CAPTURE_EDGE || height > MAX_CAPTURE_EDGE
        || width * height > MAX_CAPTURE_PIXELS
    {
        return Err("CAPTURE_DIMENSION_LIMIT".into());
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn validate_linux_capture_metrics(json: &str, native_scale: f64) -> std::result::Result<(), String> {
    if json.len() > 1024 || !native_scale.is_finite() || native_scale <= 0.0 {
        return Err("CAPTURE_METRICS_INVALID".into());
    }
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| "CAPTURE_METRICS_INVALID".to_string())?;
    let number = |key: &str| value.get(key).and_then(|v| v.as_f64())
        .ok_or_else(|| "CAPTURE_METRICS_INVALID".to_string());
    let scale = number("scale")?;
    if !scale.is_finite() || scale <= 0.0 {
        return Err("CAPTURE_METRICS_INVALID".into());
    }
    // Use the larger of the native and isolated-world scale estimates.
    checked_capture_pixels(number("width")?, number("height")?, scale.max(native_scale).max(1.0))
}

#[cfg(target_os = "linux")]
fn copy_capture_pixels(surface: gtk::cairo::Surface)
    -> std::result::Result<(Vec<u8>, gtk::cairo::Format, i32, i32, i32), String>
{
    let mut surface = gtk::cairo::ImageSurface::try_from(surface)
        .map_err(|_| "CAPTURE_SURFACE_INVALID".to_string())?;
    let (width, height, stride, format) =
        (surface.width(), surface.height(), surface.stride(), surface.format());
    checked_capture_pixels(f64::from(width), f64::from(height), 1.0)?;
    if !matches!(format, gtk::cairo::Format::ARgb32 | gtk::cairo::Format::Rgb24)
        || stride <= 0 || i64::from(stride) < i64::from(width) * 4
    {
        return Err("CAPTURE_SURFACE_INVALID".into());
    }
    let len = usize::try_from(stride).ok().and_then(|s|
        usize::try_from(height).ok().and_then(|h| s.checked_mul(h)))
        .filter(|n| *n <= 64 * 1024 * 1024)
        .ok_or_else(|| "CAPTURE_DIMENSION_LIMIT".to_string())?;
    surface.flush();
    let data = surface.data().map_err(|_| "CAPTURE_SURFACE_DATA_UNAVAILABLE".to_string())?;
    let bytes = data.get(..len).ok_or("CAPTURE_SURFACE_INVALID")?.to_vec();
    Ok((bytes, format, width, height, stride))
}

/// Request an explicitly bounded CSS clip; never fall back to an unbounded
/// screenshot if layout metrics are absent or malformed. This bounds the
/// requested region, not all renderer allocations or device-scale overhead.
#[cfg(any(target_os = "windows", test))]
fn capture_clip_from_metrics(json: &str) -> std::result::Result<serde_json::Value, String> {
    if json.len() > 64 * 1024 {
        return Err("CAPTURE_METRICS_LIMIT".into());
    }
    let metrics: serde_json::Value =
        serde_json::from_str(json).map_err(|_| "CAPTURE_METRICS_INVALID".to_string())?;
    let rect = metrics.get("cssContentSize").ok_or("CAPTURE_METRICS_UNAVAILABLE")?;
    let field = |key: &str| {
        rect.get(key)
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite())
            .ok_or_else(|| "CAPTURE_METRICS_INVALID".to_string())
    };
    let x = field("x")?;
    let y = field("y")?;
    let width = field("width")?;
    let height = field("height")?;
    if x.abs() > MAX_CAPTURE_EDGE || y.abs() > MAX_CAPTURE_EDGE {
        return Err("CAPTURE_DIMENSION_LIMIT".into());
    }
    checked_capture_pixels(width, height, 1.0)?;
    // CDP also reports the legacy layout coordinate space. Refuse an
    // oversize or unavailable second measurement instead of relying solely
    // on CSS dimensions. Neither metric proves the engine's total allocations.
    let legacy = metrics.get("contentSize").ok_or("CAPTURE_METRICS_UNAVAILABLE")?;
    let legacy_dimension = |key: &str| {
        legacy.get(key).and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v > 0.0 && v.ceil() <= 16_384.0)
            .ok_or_else(|| "CAPTURE_DIMENSION_LIMIT".to_string())
    };
    let legacy_width = legacy_dimension("width")?;
    let legacy_height = legacy_dimension("height")?;
    checked_capture_pixels(legacy_width, legacy_height, 1.0)?;
    Ok(serde_json::json!({
        "format": "png", "captureBeyondViewport": true, "fromSurface": true,
        "clip": { "x": x, "y": y, "width": width, "height": height, "scale": 1 }
    }))
}

#[cfg(test)]
mod capture_request_tests {
    use super::*;

    #[test]
    fn callback_is_consumed_once_across_dispatch_and_completion() {
        let observed = std::cell::RefCell::new(Vec::new());
        let pending = parking_lot::Mutex::new(Some(|value| observed.borrow_mut().push(value)));
        complete_capture(&pending, "dispatch_failed");
        complete_capture(&pending, "late_completion");
        assert_eq!(*observed.borrow(), vec!["dispatch_failed"]);
    }

    #[test]
    fn metrics_require_finite_bounded_css_clip() {
        let clip = capture_clip_from_metrics(
            r#"{"cssContentSize":{"x":0,"y":0,"width":1200,"height":4000},"contentSize":{"width":1200,"height":4000}}"#,
        ).unwrap();
        assert_eq!(clip["clip"]["height"].as_f64(), Some(4000.0));
        assert!(capture_clip_from_metrics(
            r#"{"cssContentSize":{"x":0,"y":0,"width":1200,"height":4000},"contentSize":{"width":4800,"height":16000}}"#,
        ).is_err());
        assert!(capture_clip_from_metrics(r#"{"contentSize":{"width":100,"height":100}}"#).is_err());
        assert!(capture_clip_from_metrics(
            r#"{"cssContentSize":{"x":0,"y":0,"width":16000,"height":16000}}"#,
        ).is_err());
        assert!(capture_clip_from_metrics(
            r#"{"cssContentSize":{"x":0,"y":0,"width":0,"height":100}}"#,
        ).is_err());
    }

    #[test]
    fn linux_preflight_counts_physical_pixels_and_rejects_invalid_measurements() {
        assert!(validate_linux_capture_metrics(r#"{"width":1200,"height":4000,"scale":1}"#, 1.0).is_ok());
        assert!(validate_linux_capture_metrics(r#"{"width":1200,"height":4000,"scale":2}"#, 1.0).is_err());
        assert!(validate_linux_capture_metrics(r#"{"width":1200,"height":4000,"scale":1}"#, 2.0).is_err());
        assert!(validate_linux_capture_metrics(r#"{"width":0,"height":100,"scale":1}"#, 1.0).is_err());
        assert!(validate_linux_capture_metrics("null", 1.0).is_err());
        assert!(validate_linux_capture_metrics(r#"{"width":100,"height":100,"scale":1}"#, f64::NAN).is_err());
    }

    #[test]
    fn exhausted_document_generation_does_not_wrap() {
        let generation = std::sync::atomic::AtomicU64::new(u64::MAX - 1);
        invalidate_capture_document(&generation);
        invalidate_capture_document(&generation);
        assert_eq!(generation.load(std::sync::atomic::Ordering::Acquire), u64::MAX);
    }

    #[test]
    fn cancellation_is_shared_and_idempotent() {
        let request = CaptureCancellation::new();
        let native = request.clone();
        assert!(!native.is_cancelled());
        request.cancel();
        request.cancel();
        assert!(native.is_cancelled());
        #[cfg(target_os = "linux")]
        {
            use webkit2gtk::gio::prelude::CancellableExt;
            assert!(native.native.is_cancelled());
        }
    }
}

// An exhausted revision never becomes a reusable generation.
fn invalidate_capture_document(revision: &std::sync::atomic::AtomicU64) {
    use std::sync::atomic::Ordering;
    let _ = revision.fetch_update(Ordering::AcqRel, Ordering::Acquire,
        |old| old.checked_add(1));
}

/// A capture is bound to both the native object and its observed document
/// generation. This is not a promise that the DOM cannot change in-place.
#[derive(Clone)]
pub struct CaptureDocument {
    instance: TabInstance,
    revision: u64,
}

impl CaptureDocument {
    fn from_tab(tab: &Arc<ManagedTab>) -> Self {
        Self {
            instance: TabInstance(Arc::downgrade(tab)),
            revision: tab.document_revision.load(std::sync::atomic::Ordering::Acquire),
        }
    }

    fn observed_generation_is_current(&self) -> bool {
        self.revision != u64::MAX && self.instance.0.upgrade().is_some_and(|tab| {
            tab.document_revision.load(std::sync::atomic::Ordering::Acquire) == self.revision
        })
    }
}

/// Opaque native instance identity. A weak reference cannot retain a closed
/// WebView and cannot match a replacement reusing the same logical tab ID.
#[derive(Clone)]
pub struct TabInstance(std::sync::Weak<ManagedTab>);

/// Internal state for managing tabs.
///
/// The host window is resolved lazily on every operation, so the plugin works
/// even when the host window is created after the plugin's `setup` hook runs
/// (e.g. apps that build their main window manually inside `.setup()`).
pub struct TabManager {
    tabs: RwLock<HashMap<TabId, Arc<ManagedTab>>>,
    lifecycle_busy: RwLock<HashMap<TabId, TabAccess>>,
    app: AppHandle<Wry>,
    host_label: String,
    #[cfg(target_os = "windows")]
    profile: RwLock<Option<Arc<browseros_platform_security::windows::PreparedPrivateDirectory>>>,
}

impl TabManager {
    pub fn new(app: AppHandle<Wry>, host_label: impl Into<String>) -> Self {
        Self {
            tabs: RwLock::new(HashMap::new()),
            lifecycle_busy: RwLock::new(HashMap::new()),
            app,
            host_label: host_label.into(),
            #[cfg(target_os = "windows")]
            profile: RwLock::new(None),
        }
    }

    /// Call in app setup before creating any WebView or accepting tab commands.
    /// Full preparation never runs under the tab collection lock. Windows Grid
    /// children join the same native preparation epoch as the main process.
    pub fn prepare_storage(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let mut slot = self.profile.write();
            if slot.is_some() {
                return Ok(());
            }
            let base = self.app.path().app_data_dir().map_err(|_| {
                BrowserTabError::Platform("BROWSER_PROFILE_LOCATION_UNAVAILABLE".into())
            })?;
            browseros_platform_security::windows::ensure_private_directory_root(
                &base,
                &browser_profile_security_options(),
            )
            .map_err(|error| {
                BrowserTabError::Platform(format!("BROWSER_PROFILE_PARENT_ACL: {error}"))
            })?;
            let prepared = browseros_platform_security::windows::PreparedPrivateDirectory::prepare(
                &browser_profile_path(&base),
                &browser_profile_security_options(),
            )
            .map_err(|error| {
                BrowserTabError::Platform(format!("BROWSER_PROFILE_PREPARATION: {error}"))
            })?;
            let report = prepared.report();
            eprintln!(
                "BROWSER_PROFILE_ACL_READY reused={} checked={} repaired={} elapsed_ms={}",
                prepared.reused_preparation(),
                report.checked_objects,
                report.repaired_objects,
                report.elapsed_ms
            );
            *slot = Some(Arc::new(prepared));
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn prepared_browser_profile(&self) -> Result<PathBuf> {
        let prepared = self
            .profile
            .read()
            .as_ref()
            .cloned()
            .ok_or_else(|| BrowserTabError::Platform("BROWSER_PROFILE_NOT_PREPARED".into()))?;
        prepared
            .check_root()
            .map_err(|error| BrowserTabError::Platform(format!("BROWSER_PROFILE_ROOT: {error}")))?;
        Ok(prepared.path().to_path_buf())
    }

    fn host_window(&self) -> Result<Window<Wry>> {
        self.app
            .get_window(&self.host_label)
            .ok_or(BrowserTabError::WindowNotFound)
    }

    fn reserve_lifecycle(&self, id: &TabId) -> Result<TabReservation<'_>> {
        reserve_tab_access(&self.lifecycle_busy, id, TabAccessKind::Lifecycle)
    }

    fn get_tab(
        &self,
        id: &TabId,
        kind: TabAccessKind,
    ) -> Result<(TabReservation<'_>, Arc<ManagedTab>)> {
        let reservation = reserve_tab_access(&self.lifecycle_busy, id, kind)?;
        let tab = self
            .tabs
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;
        Ok((reservation, tab))
    }

    pub fn create_tab(&self, options: CreateTabOptions) -> Result<()> {
        if options.rect.is_empty() {
            return Err(BrowserTabError::InvalidRect(options.rect));
        }

        let url: tauri::Url = options
            .url
            .parse()
            .map_err(|_| BrowserTabError::InvalidUrl(options.url.clone()))?;

        #[cfg(target_os = "windows")]
        let profile = self.prepared_browser_profile()?;
        #[cfg(target_os = "linux")]
        let profile = ensure_private_browser_profile(&self.app)?;

        let _reservation = self.reserve_lifecycle(&options.id)?;
        if self.tabs.read().contains_key(&options.id) {
            return Err(BrowserTabError::TabAlreadyExists(options.id));
        }

        // Navigation permission callbacks are NOT completion notifications.
        // On Windows, use Tauri's page-load lifecycle rather than emitting
        // NavigationFinished before the document actually finishes loading.
        // Linux continues to use native WebKit main-frame URI observations.
        let document_revision = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let mut builder = WebviewBuilder::new(&options.id, WebviewUrl::External(url))
            .on_navigation(|_| true)
            .transparent(options.transparent);

        #[cfg(not(target_os = "linux"))]
        {
            let nav_id = options.id.clone();
            let nav_app = self.app.clone();
            let revision = document_revision.clone();
            builder = builder.on_page_load(move |_webview, payload| {
                invalidate_capture_document(&revision);
                let url = payload.url().to_string();
                let event = match payload.event() {
                    PageLoadEvent::Started => BrowserTabEvent::NavigationStarted {
                        id: nav_id.clone(),
                        url,
                    },
                    PageLoadEvent::Finished => BrowserTabEvent::NavigationFinished {
                        id: nav_id.clone(),
                        url,
                    },
                };
                let _ = nav_app.emit("browser-tabs://event", event);
            });
        }

        // Linux and Windows browser tabs share one app-scoped persistent
        // external-page profile. The trusted main UI uses a separate profile;
        // BFC never reads or exports browser session credentials.
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            builder = builder.data_directory(profile);
        }

        // Intercept window.open / target="_blank": deny the popup and notify
        // the host app so it can open a new tab instead.
        let id_nw = options.id.clone();
        let app_nw = self.app.clone();
        builder = builder.on_new_window(move |url, _features| {
            eprintln!(
                "[browser-tabs] newWindowRequested id={} url_bytes={}",
                id_nw,
                url.as_str().len()
            );
            let _ = app_nw.emit(
                "browser-tabs://event",
                BrowserTabEvent::NewWindowRequested {
                    id: id_nw.clone(),
                    url: url.to_string(),
                },
            );
            NewWindowResponse::Deny
        });

        // auto_resize 让子 webview 跟随主窗 resize 自动调整，配合 force_allocation
        // 的 set_size_request + queue_resize 保证撑满。此为 v0.3.0 确立行为，
        // 【禁止改动】（曾因误以为与前端定位冲突而关闭，导致网页撑不满回退）。
        if options.auto_resize {
            builder = builder.auto_resize();
        }

        if let Some(ua) = &options.user_agent {
            builder = builder.user_agent(ua);
        }

        if let Some(script) = &options.initialization_script {
            builder = builder.initialization_script(script);
        }

        let host = self.host_window()?;
        // Linux hidden children start offscreen at their real viewport.
        // Never hide a newly rendering WebKit view or resize it to 1x1.
        #[cfg(target_os = "linux")]
        let native_rect = if options.visible {
            options.rect
        } else {
            offscreen_rect(options.rect)
        };
        #[cfg(not(target_os = "linux"))]
        let native_rect = options.rect;
        let webview = platform::create_child_webview(&host, builder, native_rect)?;

        // Main-frame navigation truth on Linux. Gtk/WebKit's WebView.uri
        // property changes for the top-level document, not its iframe
        // documents. notify::uri also catches redirects and history changes.
        // This listener ONLY observes URL state; it never changes GTK sizing,
        // visibility or the browser lifecycle.
        #[cfg(target_os = "linux")]
        {
            let uri_id = options.id.clone();
            let uri_app = self.app.clone();
            let uri_revision = document_revision.clone();
            let load_revision = document_revision.clone();
            let _ = webview.with_webview(move |native| {
                use webkit2gtk::WebViewExt;
                let gtk_wv = native.inner();
                // Observation only: do not change layout, visibility or load
                // behavior. Same-URL reloads also invalidate the capture.
                gtk_wv.connect_load_changed(move |_, _| {
                    invalidate_capture_document(&load_revision);
                });
                let last_uri = std::cell::RefCell::new(String::new());
                gtk_wv.connect_uri_notify(move |wv| {
                    invalidate_capture_document(&uri_revision);
                    let Some(uri) = wv.uri() else { return };
                    let value = uri.as_str();
                    let mut previous = last_uri.borrow_mut();
                    if value.is_empty() || value == previous.as_str() {
                        return;
                    }
                    *previous = value.to_owned();
                    // Keep URL in the trusted IPC payload only. Never write
                    // a URL/query containing credentials into stderr.
                    let _ = uri_app.emit(
                        "browser-tabs://event",
                        BrowserTabEvent::NavigationFinished {
                            id: uri_id.clone(),
                            url: value.to_owned(),
                        },
                    );
                });
            });
        }

        // Shell shortcuts belong to the trusted GTK host, not remote-page JS/IPC.
        // Capture only three app-owned shortcuts from a focused child WebView.
        #[cfg(target_os = "linux")]
        {
            let id_key = options.id.clone();
            let app_key = self.app.clone();
            let _ = webview.with_webview(move |native| {
                use gtk::prelude::*;
                let child = native.inner();
                child.connect_key_press_event(move |_wv, event| {
                    use gtk::gdk::ModifierType;
                    let state = event.state();
                    let ctrl = state.contains(ModifierType::CONTROL_MASK);
                    let shift = state.contains(ModifierType::SHIFT_MASK);
                    let alt = state.contains(ModifierType::MOD1_MASK);
                    let meta = state.contains(ModifierType::META_MASK)
                        || state.contains(ModifierType::SUPER_MASK);
                    if alt || meta {
                        return gtk::glib::Propagation::Proceed;
                    }
                    let key = event
                        .keyval()
                        .name()
                        .map(|n| n.to_string())
                        .unwrap_or_default();
                    let action = if key == "F11" && !ctrl {
                        if shift {
                            None
                        } else {
                            Some("immersive")
                        }
                    } else if key == "F11" && ctrl && shift {
                        Some("compact")
                    } else if (key == "l" || key == "L") && ctrl && !shift {
                        Some("address")
                    } else {
                        None
                    };
                    if let Some(action) = action {
                        let _ = app_key.emit(
                            "browser-tabs://shell-shortcut",
                            serde_json::json!({
                                "id": id_key, "action": action
                            }),
                        );
                        gtk::glib::Propagation::Stop
                    } else {
                        gtk::glib::Propagation::Proceed
                    }
                });
            });
        }

        // 加载失败上报（Linux）：WebKit 内部错误页不注入用户脚本，前端无法感知
        // TLS/网络失败，必须连原生 load-failed / load-failed-with-tls-errors 信号。
        #[cfg(target_os = "linux")]
        {
            let id_lf = options.id.clone();
            let app_lf = self.app.clone();
            let _ = webview.with_webview(move |pwv| {
                use webkit2gtk::WebViewExt;
                let gtk_wv = pwv.inner();
                let id1 = id_lf.clone();
                let app1 = app_lf.clone();
                gtk_wv.connect_load_failed(move |_wv, _event, uri, error| {
                    eprintln!(
                        "[browser-tabs] loadFailed id={} uri_bytes={}",
                        id1,
                        uri.len()
                    );
                    let _ = app1.emit(
                        "browser-tabs://event",
                        BrowserTabEvent::LoadFailed {
                            id: id1.clone(),
                            url: uri.to_string(),
                            error: error.to_string(),
                        },
                    );
                    false // 保留 WebKit 默认错误页（用户可见错误原因）
                });
                let id2 = id_lf.clone();
                let app2 = app_lf.clone();
                gtk_wv.connect_load_failed_with_tls_errors(move |_wv, uri, _cert, _errors| {
                    eprintln!(
                        "[browser-tabs] loadFailedTLS id={} uri_bytes={}",
                        id2,
                        uri.len()
                    );
                    let _ = app2.emit(
                        "browser-tabs://event",
                        BrowserTabEvent::LoadFailed {
                            id: id2.clone(),
                            url: uri.to_string(),
                            error: "tls-handshake".to_string(),
                        },
                    );
                    false
                });
            });
        }

        // M1-8 资源瀑布采集（Linux WebKitGTK 原生信号）：
        // resource-load-started 记录方法与开始时刻，finished/failed 时取真实
        // URIResponse（status/mime/content_length）后上报。字段全部来自原生信号，
        // 无响应时 status/mime/size_bytes 为 None，绝不伪造；错误信息不上报
        //（GError 文本可能含原始 URL，隐私红线）。
        // 隐私注意：此处**不打印、不落盘**任何资源 URL（含 query 的原始 URL
        // 可能带 token）；脱敏与容量上限由主进程 bridge 统一负责。
        #[cfg(target_os = "linux")]
        {
            let id_rs = options.id.clone();
            let app_rs = self.app.clone();
            let _ = webview.with_webview(move |pwv| {
                use webkit2gtk::{URIRequestExt, URIResponseExt, WebResourceExt, WebViewExt};
                let gtk_wv = pwv.inner();
                gtk_wv.connect_resource_load_started(move |_wv, resource, request| {
                    let started = std::time::Instant::now();
                    let started_at = epoch_millis();
                    let url = request.uri().map(|u| u.to_string()).unwrap_or_default();
                    if url.is_empty() {
                        return;
                    }
                    let method = request.http_method().map(|m| m.to_string());
                    let emit = {
                        let id = id_rs.clone();
                        let app = app_rs.clone();
                        let url = url.clone();
                        let method = method.clone();
                        move |status: Option<u32>, mime: Option<String>, size: Option<u64>| {
                            let _ = app.emit(
                                "browser-tabs://event",
                                BrowserTabEvent::ResourceReceived {
                                    id: id.clone(),
                                    url: url.clone(),
                                    method: method.clone(),
                                    status,
                                    mime,
                                    size_bytes: size,
                                    started_at,
                                    finished_at: epoch_millis(),
                                },
                            );
                        }
                    };
                    let emit_finished = emit.clone();
                    resource.connect_finished(move |r| {
                        let _elapsed = started.elapsed();
                        match r.response() {
                            Some(resp) => {
                                let size = match resp.content_length() {
                                    0 => None, // WebKit 未知长度返回 0，映射为 None（不伪造）
                                    n => Some(n),
                                };
                                emit_finished(
                                    Some(resp.status_code()),
                                    resp.mime_type().map(|m| m.to_string()),
                                    size,
                                );
                            }
                            None => emit_finished(None, None, None),
                        }
                    });
                    resource.connect_failed(move |_r, _error| {
                        // 加载失败：只上报能确定的真实字段，status/mime/size 为 None
                        emit(None, None, None);
                    });
                });
            });
        }

        #[cfg(not(target_os = "linux"))]
        if !options.visible {
            webview.hide()?;
        }

        let entry = Arc::new(ManagedTab {
            webview,
            document_revision,
            #[cfg(target_os = "linux")]
            layout: RwLock::new(TabLayout {
                rect: native_rect,
                restore_rect: (!options.visible).then_some(options.rect),
            }),
            #[cfg(not(target_os = "linux"))]
            last_rect: RwLock::new(native_rect),
        });
        self.tabs.write().insert(options.id, entry);
        Ok(())
    }

    pub fn update_rect(&self, id: &TabId, rect: LogicalRect) -> Result<()> {
        if rect.is_empty() {
            return Err(BrowserTabError::InvalidRect(rect));
        }

        let (_operation, tab) = self.get_tab(id, TabAccessKind::Layout)?;
        platform::apply_rect(&tab.webview, rect)?;
        #[cfg(not(target_os = "linux"))]
        {
            *tab.last_rect.write() = rect;
        }
        #[cfg(target_os = "linux")]
        {
            *tab.layout.write() = TabLayout {
                rect,
                restore_rect: None,
            };
        }
        Ok(())
    }

    /// Bridge positioning is one admission transaction: geometry, show
    /// and the host's short metadata publication cannot interleave with a
    /// periodic sweep. The callback must not call back into native code.
    pub fn position_visible(
        &self,
        id: &TabId,
        rect: LogicalRect,
        publish: impl FnOnce() -> bool,
    ) -> Result<()> {
        if rect.is_empty() {
            return Err(BrowserTabError::InvalidRect(rect));
        }
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Layout)?;
        platform::apply_rect(&tab.webview, rect)?;
        #[cfg(not(target_os = "linux"))]
        {
            *tab.last_rect.write() = rect;
        }
        #[cfg(target_os = "linux")]
        {
            *tab.layout.write() = TabLayout {
                rect,
                restore_rect: None,
            };
        }
        tab.webview.show()?;
        if !publish() {
            #[cfg(target_os = "linux")]
            Self::park_tab(&tab)?;
            #[cfg(not(target_os = "linux"))]
            {
                let parked = LogicalRect::new(-30000.0, rect.y, rect.width, rect.height);
                platform::apply_rect(&tab.webview, parked)?;
                *tab.last_rect.write() = parked;
            }
            return Err(BrowserTabError::Platform("TAB_LAYOUT_SUPERSEDED".into()));
        }
        Ok(())
    }

    /// Read the latest host target *after* acquiring the layout reservation,
    /// so a periodic sweep cannot overwrite a completed position command
    /// with geometry captured before that command.
    pub fn reconcile_rect(
        &self,
        id: &TabId,
        target: impl FnOnce() -> Option<LogicalRect>,
    ) -> Result<()> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Layout)?;
        let Some(rect) = target() else {
            return Ok(());
        };
        if rect.is_empty() {
            return Err(BrowserTabError::InvalidRect(rect));
        }
        #[cfg(target_os = "linux")]
        {
            let current = *tab.layout.read();
            if current.rect == rect {
                return platform::linux::ensure_size_allocated(&tab.webview, rect);
            }
        }
        #[cfg(not(target_os = "linux"))]
        if *tab.last_rect.read() == rect {
            return Ok(());
        }
        platform::apply_rect(&tab.webview, rect)?;
        #[cfg(not(target_os = "linux"))]
        {
            *tab.last_rect.write() = rect;
        }
        #[cfg(target_os = "linux")]
        {
            *tab.layout.write() = TabLayout {
                rect,
                restore_rect: None,
            };
        }
        Ok(())
    }

    /// A missing bridge layout must not cause a guessed 800x600 resize of a
    /// live Linux viewport. Return the real parked rectangle to the caller.
    #[cfg(target_os = "linux")]
    pub fn hide_offscreen(&self, id: &TabId, publish: impl FnOnce(LogicalRect)) -> Result<()> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Layout)?;
        let rect = Self::park_tab(&tab)?;
        publish(rect);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn park_tab(tab: &ManagedTab) -> Result<LogicalRect> {
        let layout = *tab.layout.read();
        if layout.rect.x < -1000.0 {
            return Ok(layout.rect);
        }
        let rect = offscreen_rect(layout.rect);
        platform::apply_rect(&tab.webview, rect)?;
        *tab.layout.write() = TabLayout {
            rect,
            restore_rect: Some(layout.rect),
        };
        Ok(rect)
    }

    pub fn set_visible(&self, id: &TabId, visible: bool) -> Result<()> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Layout)?;
        #[cfg(target_os = "linux")]
        {
            let layout = *tab.layout.read();
            if visible {
                if let Some(rect) = layout.restore_rect {
                    platform::apply_rect(&tab.webview, rect)?;
                    *tab.layout.write() = TabLayout {
                        rect,
                        restore_rect: None,
                    };
                }
                tab.webview.show()?;
            } else {
                Self::park_tab(&tab)?;
            }
        }
        #[cfg(not(target_os = "linux"))]
        if visible {
            tab.webview.show()?;
        } else {
            tab.webview.hide()?;
        }
        Ok(())
    }

    /// 设置子 webview 内容缩放（WebKitGTK set_zoom_level）。
    /// 用于宫格小窗按比例缩小网页内容，实现"自适应"展示。
    pub fn set_zoom(&self, id: &TabId, scale_factor: f64) -> Result<()> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Operation)?;
        invalidate_capture_document(&tab.document_revision);
        tab.webview.set_zoom(scale_factor)?;
        Ok(())
    }

    pub fn close_tab(&self, id: &TabId) -> Result<()> {
        let _reservation = self.reserve_lifecycle(id)?;
        // A successful native close may outlive its IPC reply. Retrying an
        // already-closed or hibernated tab is successful and creates no resource.
        let tab = self.tabs.read().get(id).cloned();
        let Some(tab) = tab else {
            return Ok(());
        };
        // Keep ownership on failure. Admission excludes other synchronous
        // dispatches and duplicate lifecycle changes for this identity.
        invalidate_capture_document(&tab.document_revision);
        tab.webview.close()?;
        self.tabs.write().remove(id);
        platform::forget_native_layout(id);
        Ok(())
    }

    pub fn navigate(&self, id: &TabId, url: &str) -> Result<()> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Operation)?;

        let url: tauri::Url = url
            .parse()
            .map_err(|_| BrowserTabError::InvalidUrl(url.to_string()))?;
        invalidate_capture_document(&tab.document_revision);
        tab.webview.navigate(url)?;
        Ok(())
    }

    pub fn get_tab_ids(&self) -> Vec<TabId> {
        self.tabs.read().keys().cloned().collect()
    }

    pub fn instance(&self, id: &TabId) -> Result<TabInstance> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Operation)?;
        Ok(TabInstance(Arc::downgrade(&tab)))
    }

    pub fn is_current_instance(&self, id: &TabId, instance: &TabInstance) -> bool {
        self.get_tab(id, TabAccessKind::Operation)
            .is_ok_and(|(_operation, tab)| {
                std::sync::Weak::ptr_eq(&instance.0, &Arc::downgrade(&tab))
            })
    }

    pub fn capture_document(&self, id: &TabId) -> Result<CaptureDocument> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Operation)?;
        let document = CaptureDocument::from_tab(&tab);
        if !document.observed_generation_is_current() {
            return Err(BrowserTabError::Platform("CAPTURE_DOCUMENT_STALE".into()));
        }
        Ok(document)
    }

    pub fn is_current_capture_document(&self, id: &TabId, document: &CaptureDocument) -> bool {
        self.is_current_instance(id, &document.instance)
            && document.observed_generation_is_current()
    }

    pub fn eval_instance(&self, id: &TabId, instance: &TabInstance, js: &str) -> Result<()> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Operation)?;
        if !std::sync::Weak::ptr_eq(&instance.0, &Arc::downgrade(&tab)) {
            return Err(BrowserTabError::Platform("TAB_INSTANCE_STALE".into()));
        }
        tab.webview.eval(js)?;
        Ok(())
    }

    /// 在指定子 webview 中执行 JavaScript（fire-and-forget）。
    /// 注意：子 webview 由本插件持有，app.get_webview 找不到它们，
    /// 宿主应用必须走这个方法（否则 eval 静默空转，AI 群发/前进后退全失效）。
    pub fn eval(&self, id: &TabId, js: &str) -> Result<()> {
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Operation)?;
        tab.webview.eval(js)?;
        Ok(())
    }

    /// Capture the entire document through its native engine. No page scroll,
    /// remote script, authentication state or image bytes cross the IPC bridge.
    /// The callback is bound to the requested, app-owned tab and is size limited.
    pub fn capture_full_page(
        &self,
        id: &TabId,
        callback: impl FnOnce(std::result::Result<Vec<u8>, String>) + Send + 'static,
    ) -> Result<()> {
        self.capture_full_page_cancellable(id, CaptureCancellation::new(), callback)
    }

    pub fn capture_full_page_cancellable(
        &self,
        id: &TabId,
        cancellation: CaptureCancellation,
        callback: impl FnOnce(std::result::Result<Vec<u8>, String>) + Send + 'static,
    ) -> Result<()> {
        let document = self.capture_document(id)?;
        self.capture_bound_document(id, document, cancellation, callback)
    }

    #[allow(deprecated)] // Keep support for the pinned WebKitGTK 2.22 API.
    pub fn capture_bound_document(
        &self,
        id: &TabId,
        capture_document: CaptureDocument,
        cancellation: CaptureCancellation,
        callback: impl FnOnce(std::result::Result<Vec<u8>, String>) + Send + 'static,
    ) -> Result<()> {
        // Bind the admitted document, not whichever native instance happens
        // to own the logical id when the worker starts.
        let Ok((_operation, tab)) = self.get_tab(id, TabAccessKind::Operation) else {
            callback(Err("CAPTURE_INSTANCE_STALE".into()));
            return Ok(());
        };
        if !std::sync::Weak::ptr_eq(&capture_document.instance.0, &Arc::downgrade(&tab))
            || !capture_document.observed_generation_is_current() {
            callback(Err("CAPTURE_DOCUMENT_STALE".into()));
            return Ok(());
        }
        let webview = tab.webview.clone();
        #[cfg(target_os = "linux")]
        {
            webview.with_webview(move |native| {
                use java_script_core::ValueExt;
                use webkit2gtk::WebViewExt;
                use gtk::prelude::WidgetExt;
                let gtk_wv = native.inner().clone();
                if cancellation.is_cancelled() || !capture_document.observed_generation_is_current() {
                    callback(Err("CAPTURE_CANCELLED_OR_STALE".into()));
                    return;
                }
                if gtk_wv.is_loading() {
                    callback(Err("CAPTURE_DOCUMENT_LOADING".into()));
                    return;
                }
                // A fixed isolated-world program reads layout metrics only.
                // No page-selected JS, DOM contents, URL or credentials cross IPC.
                let native_scale = f64::from(gtk_wv.scale_factor()) * gtk_wv.zoom_level();
                let metrics_view = gtk_wv.clone();
                let metrics_cancel = cancellation.native.clone();
                gtk_wv.run_javascript_in_world(
                    r#"(() => {
                        const root = document.documentElement;
                        if (!root) return "null";
                        const body = document.body;
                        return JSON.stringify({
                            width: Math.max(root.scrollWidth, root.offsetWidth, root.clientWidth,
                                body?.scrollWidth || 0, body?.offsetWidth || 0, innerWidth),
                            height: Math.max(root.scrollHeight, root.offsetHeight, root.clientHeight,
                                body?.scrollHeight || 0, body?.offsetHeight || 0, innerHeight),
                            scale: devicePixelRatio
                        });
                    })()"#,
                    "browseros.capture.metrics.v1",
                    Some(&metrics_cancel),
                    move |result| {
                        if cancellation.is_cancelled() || !capture_document.observed_generation_is_current()
                            || metrics_view.is_loading()
                        {
                            callback(Err("CAPTURE_CANCELLED_OR_STALE".into()));
                            return;
                        }
                        let measurement = result.map_err(|_| "CAPTURE_METRICS_FAILED".to_string())
                            .and_then(|value| value.js_value()
                                .map(|v| v.to_str().to_string())
                                .ok_or_else(|| "CAPTURE_METRICS_UNAVAILABLE".to_string()))
                            .and_then(|json| validate_linux_capture_metrics(&json, native_scale));
                        if let Err(error) = measurement {
                            callback(Err(error));
                            return;
                        }
                        let snapshot_cancel = cancellation.native.clone();
                        // FullDocument cannot accept a native allocation ceiling.
                        // This is a conservative preflight, not a proof against
                        // an in-place resize between measurement and allocation.
                        metrics_view.snapshot(
                            webkit2gtk::SnapshotRegion::FullDocument,
                            webkit2gtk::SnapshotOptions::NONE,
                            Some(&snapshot_cancel),
                            move |result| {
                                if cancellation.is_cancelled()
                                    || !capture_document.observed_generation_is_current()
                                {
                                    callback(Err("CAPTURE_CANCELLED_OR_STALE".into()));
                                    return;
                                }
                                let pixels = result.map_err(|_| "CAPTURE_ENGINE_FAILED".to_string())
                                    .and_then(copy_capture_pixels);
                                let (bytes, format, width, height, stride) = match pixels {
                                    Ok(pixels) => pixels,
                                    Err(error) => { callback(Err(error)); return; }
                                };
                                // Cairo surfaces stay on the GTK thread; only a
                                // bounded owned pixel buffer crosses to the worker.
                                // The existing host permit covers encoding, too.
                                let pending = Arc::new(parking_lot::Mutex::new(Some(callback)));
                                let worker_reply = pending.clone();
                                let worker = std::thread::Builder::new()
                                    .name("browseros-screenshot-png".into())
                                    .spawn(move || {
                                        let result = (|| {
                                            if cancellation.is_cancelled()
                                                || !capture_document.observed_generation_is_current()
                                            {
                                                return Err("CAPTURE_CANCELLED_OR_STALE".into());
                                            }
                                            let surface = gtk::cairo::ImageSurface::create_for_data(
                                                bytes, format, width, height, stride,
                                            ).map_err(|_| "CAPTURE_SURFACE_INVALID".to_string())?;
                                            let mut png = BoundedPngWriter::default();
                                            surface.write_to_png(&mut png)
                                                .map_err(|_| "CAPTURE_PNG_ENCODE_FAILED".to_string())?;
                                            if cancellation.is_cancelled()
                                                || !capture_document.observed_generation_is_current()
                                            {
                                                return Err("CAPTURE_CANCELLED_OR_STALE".into());
                                            }
                                            Ok(png.0)
                                        })();
                                        complete_capture(&worker_reply, result);
                                    });
                                if worker.is_err() {
                                    complete_capture(&pending, Err("CAPTURE_WORKER_FAILED".into()));
                                }
                            },
                        );
                    },
                );
            })?;
        }
        #[cfg(target_os = "windows")]
        {
            let capture_app = self.app.clone();
            let capture_id = id.clone();
            webview.with_webview(move |native| {
                use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
                use windows::core::PCWSTR;
                let core = match unsafe { native.controller().CoreWebView2() } {
                    Ok(core) => core,
                    Err(_) => {
                        callback(Err("CAPTURE_ENGINE_UNAVAILABLE".into()));
                        return;
                    }
                };
                let pending = Arc::new(parking_lot::Mutex::new(Some(callback)));
                if cancellation.is_cancelled() {
                    complete_capture(&pending, Err("CAPTURE_CANCELLED_BEFORE_DISPATCH".into()));
                    return;
                }
                let metrics_reply = pending.clone();
                let capture_core = core.clone();
                let metrics_method: Vec<u16> = "Page.getLayoutMetrics".encode_utf16().chain(std::iter::once(0)).collect();
                let empty_params: Vec<u16> = "{}".encode_utf16().chain(std::iter::once(0)).collect();
                let metrics_completion = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                    move |status, json| {
                        if cancellation.is_cancelled() {
                            complete_capture(&metrics_reply, Err("CAPTURE_CANCELLED_AT_COMPLETION".into()));
                            return Ok(());
                        }
                        let params = match status
                            .map_err(|_| "CAPTURE_METRICS_FAILED".to_string())
                            .and_then(|_| capture_clip_from_metrics(&json))
                        {
                            Ok(params) => params,
                            Err(error) => {
                                complete_capture(&metrics_reply, Err(error));
                                return Ok(());
                            }
                        };
                        // Layout metrics may arrive after a close/replacement.
                        // Never dispatch the second native operation in that case.
                        if !capture_app.state::<TabManagerState>()
                            .is_current_capture_document(&capture_id, &capture_document)
                        {
                            complete_capture(&metrics_reply, Err("CAPTURE_INSTANCE_STALE".into()));
                            return Ok(());
                        }
                        let method: Vec<u16> = "Page.captureScreenshot".encode_utf16().chain(std::iter::once(0)).collect();
                        let params: Vec<u16> = params.to_string().encode_utf16().chain(std::iter::once(0)).collect();
                        let image_reply = metrics_reply.clone();
                        let image_cancel = cancellation.clone();
                        let image_document = capture_document.clone();
                        let completion = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                            move |status, json| {
                                if image_cancel.is_cancelled()
                                    || !image_document.observed_generation_is_current()
                                {
                                    complete_capture(&image_reply, Err("CAPTURE_CANCELLED_OR_STALE".into()));
                                    return Ok(());
                                }
                                let data = status
                                    .map_err(|_| "CAPTURE_ENGINE_FAILED".to_string())
                                    .and_then(|_| {
                                        if json.len() > 45 * 1024 * 1024 {
                                            return Err("CAPTURE_SIZE_LIMIT".into());
                                        }
                                        let response: serde_json::Value = serde_json::from_str(&json)
                                            .map_err(|_| "CAPTURE_RESPONSE_INVALID".to_string())?;
                                        let image = response.get("data").and_then(|v| v.as_str())
                                            .ok_or_else(|| "CAPTURE_NO_IMAGE".to_string())?;
                                        if image.len() > 44 * 1024 * 1024 {
                                            return Err("CAPTURE_SIZE_LIMIT".into());
                                        }
                                        use base64::Engine;
                                        base64::engine::general_purpose::STANDARD.decode(image)
                                            .map_err(|_| "CAPTURE_PNG_DECODE_FAILED".into())
                                    });
                                complete_capture(&image_reply, data);
                                Ok(())
                            },
                        ));
                        // Recheck before dispatch. Once admitted, only the CDP
                        // completion or a synchronous dispatch rejection ends it.
                        if cancellation.is_cancelled() {
                            complete_capture(&metrics_reply, Err("CAPTURE_CANCELLED_BEFORE_DISPATCH".into()));
                        } else if unsafe {
                            capture_core.CallDevToolsProtocolMethod(
                                PCWSTR(method.as_ptr()), PCWSTR(params.as_ptr()), &completion,
                            )
                        }.is_err() {
                            complete_capture(&metrics_reply, Err("CAPTURE_DISPATCH_FAILED".into()));
                        }
                        Ok(())
                    },
                ));
                if unsafe {
                    core.CallDevToolsProtocolMethod(
                        PCWSTR(metrics_method.as_ptr()), PCWSTR(empty_params.as_ptr()), &metrics_completion,
                    )
                }.is_err() {
                    complete_capture(&pending, Err("CAPTURE_METRICS_DISPATCH_FAILED".into()));
                }
            })?;
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        callback(Err("CAPTURE_PLATFORM_UNSUPPORTED".into()));
        Ok(())
    }

    /// Native return channel; never grants the remote document an IPC permission.
    #[allow(deprecated)] // Supports WebKitGTK 2.22 as well as 2.40+.
    pub fn eval_result(
        &self,
        id: &TabId,
        js: String,
        callback: impl FnOnce(std::result::Result<String, String>) + Send + 'static,
    ) -> Result<()> {
        // Protect dispatch only, never keep a reservation across an
        // asynchronous WebKit/WebView2 completion callback.
        let (_operation, tab) = self.get_tab(id, TabAccessKind::Operation)?;
        let webview = tab.webview.clone();
        #[cfg(target_os = "linux")]
        webview.with_webview(move |native| {
            use java_script_core::ValueExt;
            use webkit2gtk::WebViewExt;
            native.inner().run_javascript(
                &js,
                None::<&webkit2gtk::gio::Cancellable>,
                move |result| {
                    let value = result
                        .map_err(|_| "GRID_EXTRACT_FAILED".to_string())
                        .and_then(|r| {
                            r.js_value()
                                .map(|v| v.to_str().to_string())
                                .ok_or_else(|| "GRID_NO_RESULT".into())
                        })
                        .and_then(|s| {
                            if s.len() <= 512 * 1024 {
                                Ok(s)
                            } else {
                                Err("GRID_RESULT_LIMIT".into())
                            }
                        });
                    callback(value);
                },
            );
        })?;
        #[cfg(not(target_os = "linux"))]
        {
            // Tauri delegates to WebView2's native script evaluation callback.
            // It returns a JSON-serialized value; normalize a JS string result
            // to match Linux WebKit's existing raw-string callback contract.
            // The mutex enforces the original FnOnce callback even if a
            // platform runtime unexpectedly invokes its Fn callback twice.
            let pending = Mutex::new(Some(callback));
            webview.eval_with_callback(js, move |raw_json| {
                if let Ok(mut guard) = pending.lock() {
                    if let Some(complete) = guard.take() {
                        complete(decode_native_eval_result(&raw_json));
                    }
                }
            })?;
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod offscreen_layout_tests {
    use super::*;

    #[test]
    fn linux_hide_preserves_real_viewport() {
        let original = LogicalRect::new(25.0, 80.0, 1200.0, 640.0);
        let hidden = offscreen_rect(original);
        assert_eq!(hidden.x, -30000.0);
        assert_eq!(hidden.y, original.y);
        assert_eq!(hidden.width, original.width);
        assert_eq!(hidden.height, original.height);
    }

    #[test]
    fn lifecycle_reservation_releases_its_id_on_drop() {
        let busy = RwLock::new(HashMap::new());
        let id = "tab-test".to_string();
        {
            let _reservation = reserve_tab_access(&busy, &id, TabAccessKind::Lifecycle).unwrap();
            assert!(reserve_tab_access(&busy, &id, TabAccessKind::Operation).is_err());
            assert!(busy.read().contains_key(&id));
        }
        assert!(!busy.read().contains_key(&id));
    }

    #[test]
    fn active_dispatch_excludes_close_but_allows_non_layout_operations() {
        let busy = RwLock::new(HashMap::new());
        let id = "tab-test".to_string();
        let operation = reserve_tab_access(&busy, &id, TabAccessKind::Operation).unwrap();
        let layout = reserve_tab_access(&busy, &id, TabAccessKind::Layout).unwrap();
        assert!(reserve_tab_access(&busy, &id, TabAccessKind::Layout).is_err());
        assert!(reserve_tab_access(&busy, &id, TabAccessKind::Lifecycle).is_err());
        drop(layout);
        assert!(reserve_tab_access(&busy, &id, TabAccessKind::Lifecycle).is_err());
        drop(operation);
        assert!(busy.read().is_empty());
        assert!(reserve_tab_access(&busy, &id, TabAccessKind::Lifecycle).is_ok());
    }
}

// Tauri returns a JSON value on Windows; Linux WebKit already returns the
// string payload directly. Avoid double-encoding JSON.stringify(...) results.
#[cfg(any(not(target_os = "linux"), test))]
fn decode_native_eval_result(raw: &str) -> std::result::Result<String, String> {
    const MAX_BYTES: usize = 512 * 1024;
    // JSON escaping can inflate the serialized WebView2 response. Bound it
    // before parsing so an untrusted remote page cannot force huge allocations.
    if raw.len() > MAX_BYTES * 6 {
        return Err("GRID_RESULT_LIMIT".into());
    }
    if raw == "null" {
        return Err("GRID_NO_RESULT".into());
    }
    let value = serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.to_string());
    if value.len() > MAX_BYTES {
        Err("GRID_RESULT_LIMIT".into())
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod native_eval_result_tests {
    use super::decode_native_eval_result;

    #[test]
    fn decodes_windows_json_string_into_the_linux_callback_contract() {
        assert_eq!(
            decode_native_eval_result(r#""{\"safe\":true,\"url\":\"about:blank\"}""#),
            Ok(r#"{"safe":true,"url":"about:blank"}"#.to_string())
        );
        assert_eq!(decode_native_eval_result("42"), Ok("42".into()));
        assert_eq!(
            decode_native_eval_result("null"),
            Err("GRID_NO_RESULT".into())
        );
    }

    #[test]
    fn script_result_is_bounded_before_decoding() {
        let long = format!("\"{}\"", "x".repeat(512 * 1024 + 1));
        assert_eq!(
            decode_native_eval_result(&long),
            Err("GRID_RESULT_LIMIT".into())
        );
    }
}

/// Shared state type for Tauri commands and host-app integration.
pub type TabManagerState = Arc<TabManager>;

#[tauri::command]
pub async fn create_tab(
    state: State<'_, TabManagerState>,
    options: CreateTabOptions,
) -> Result<()> {
    state.create_tab(options)
}

#[tauri::command]
pub async fn update_rect(
    state: State<'_, TabManagerState>,
    id: TabId,
    rect: LogicalRect,
) -> Result<()> {
    state.update_rect(&id, rect)
}

#[tauri::command]
pub async fn set_visible(
    state: State<'_, TabManagerState>,
    id: TabId,
    visible: bool,
) -> Result<()> {
    state.set_visible(&id, visible)
}

#[tauri::command]
pub async fn close_tab(state: State<'_, TabManagerState>, id: TabId) -> Result<()> {
    state.close_tab(&id)
}

#[tauri::command]
pub async fn navigate(state: State<'_, TabManagerState>, id: TabId, url: String) -> Result<()> {
    state.navigate(&id, &url)
}

#[tauri::command]
pub async fn list_tabs(state: State<'_, TabManagerState>) -> Result<Vec<TabId>> {
    Ok(state.get_tab_ids())
}

#[tauri::command]
pub async fn set_zoom(
    state: State<'_, TabManagerState>,
    id: TabId,
    scale_factor: f64,
) -> Result<()> {
    state.set_zoom(&id, scale_factor)
}
