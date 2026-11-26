//! Explicit independent capture process, optionally with an interactive
//! login session. No session migration, normal app/remote command handlers,
//! shared browser profile, or live-tab reload.
//! The parent owns a memory-limited native process domain and publishes output
//! only after recursive-empty accounting and the original child exit.
//! This is process lifecycle containment, NOT a hostile same-UID sandbox.
use crate::capabilities::workspace::safe_paths;
use crate::native_supervisor::OwnedHost;
use std::{
    ffi::OsStr,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;
use tauri_plugin_browser_tabs::{CaptureCancellation, CreateTabOptions, LogicalRect, TabManager};

const PARENT_FLAG: &str = "--isolated-capture";
const CHILD_FLAG: &str = "--isolated-capture-worker";
const INTERACTIVE_CHILD_FLAG: &str = "--isolated-interactive-capture-worker";
const READY_FILE: &str = "interactive-ready";
const READY_MAGIC: &[u8] = b"browseros-render-ready-v1\n";
const CAPTURE_BYTE: u8 = b'C';
const INTERACTIVE_WAIT: Duration = Duration::from_secs(600);
const URL_ENV: &str = "BROWSEROS_CAPTURE_JOB_URL";
const ROOT_ENV: &str = "BROWSEROS_CAPTURE_JOB_ROOT";
const START_MAGIC: &[u8] = b"browseros-render-admission-v1\n";
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MEMORY_LIMIT: usize = 2 * 1024 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);
const HOST: &str = "isolated-capture-host";
const PAGE: &str = "isolated-capture-page";

pub(crate) fn checked_url(raw: &str) -> Result<String, String> {
    if raw.len() > 8192 {
        return Err("CAPTURE_URL_INVALID".into());
    }
    let url = url::Url::parse(raw).map_err(|_| "CAPTURE_URL_INVALID")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("CAPTURE_HTTPS_URL_REQUIRED".into());
    }
    Ok(url.into())
}
fn checked_png_header(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 24
        || bytes.len() > MAX_BYTES
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || &bytes[12..16] != b"IHDR"
    {
        return Err("CAPTURE_INVALID_PNG".into());
    }
    let width = u32::from_be_bytes(
        bytes[16..20]
            .try_into()
            .map_err(|_| "CAPTURE_INVALID_PNG")?,
    );
    let height = u32::from_be_bytes(
        bytes[20..24]
            .try_into()
            .map_err(|_| "CAPTURE_INVALID_PNG")?,
    );
    if width == 0
        || height == 0
        || width > 16_384
        || height > 16_384
        || u64::from(width) * u64::from(height) > 16_000_000
    {
        return Err("CAPTURE_DIMENSION_LIMIT".into());
    }
    Ok(())
}
// Validate the complete worker artifact before publishing it. Decode one
// row/pass at a time instead of allocating a second full raster in the host.
// The existing encoded-byte and dimension limits still apply. PNG decoder
// limits are best-effort intermediate allocation limits, not renderer limits.
pub(crate) fn checked_png(bytes: &[u8]) -> Result<(), String> {
    checked_png_header(bytes)?;
    if !bytes.ends_with(b"\0\0\0\0IEND\xaeB\x60\x82") {
        return Err("CAPTURE_INVALID_PNG".into());
    }
    let mut limits = png::Limits::default();
    limits.bytes = MAX_BYTES;
    let mut decoder = png::Decoder::new_with_limits(std::io::Cursor::new(bytes), limits);
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    let mut reader = decoder.read_info().map_err(|_| "CAPTURE_INVALID_PNG")?;
    if reader.info().animation_control.is_some() {
        return Err("CAPTURE_ANIMATED_PNG_DENIED".into());
    }
    while reader
        .next_interlaced_row()
        .map_err(|_| "CAPTURE_INVALID_PNG")?
        .is_some()
    {}
    reader.finish().map_err(|_| "CAPTURE_INVALID_PNG")?;
    Ok(())
}
fn scratch_parent() -> Result<PathBuf, String> {
    std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "CAPTURE_TEMP_UNAVAILABLE".into())
}
fn fresh_scratch() -> Result<(PathBuf, PathBuf, String), String> {
    let parent = scratch_parent()?;
    let root = parent.join(format!(
        "browseros-isolated-capture-{}",
        uuid::Uuid::new_v4()
    ));
    // No sensitive content until the OS private-directory policy is applied.
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&root)
        .map_err(|_| "CAPTURE_TEMP_CREATE_FAILED")?;
    safe_paths::ensure_private_directory(&root).map_err(|_| "CAPTURE_TEMP_PRIVATE_FAILED")?;
    let identity =
        safe_paths::directory_identity(&root).map_err(|_| "CAPTURE_TEMP_IDENTITY_FAILED")?;
    Ok((parent, root, identity))
}
fn cleanup(parent: &Path, root: &Path, identity: &str) -> Result<(), String> {
    if safe_paths::directory_identity(root).map_err(|_| "CAPTURE_TEMP_IDENTITY_FAILED")? != identity
    {
        return Err("CAPTURE_TEMP_IDENTITY_CHANGED".into());
    }
    safe_paths::remove_tree(root, &[parent.to_path_buf()])
        .map_err(|_| "CAPTURE_TEMP_CLEANUP_FAILED".into())
}
/// Same execution path for the explicit CLI and Browser Owner. Cancellation
/// does not return until the owned process domain has really been retired.
/// The pending callback is metadata-only; it never releases admission.
pub(crate) struct InteractiveControl<'a> {
    pub ready: &'a dyn Fn() -> Result<(), String>,
    pub capture_requested: &'a dyn Fn() -> Result<bool, String>,
}

pub(crate) fn render_job(
    raw: &str,
    cancelled: impl Fn() -> bool,
    on_retire_pending: impl Fn(),
) -> Result<Vec<u8>, String> {
    render_owned_job(raw, cancelled, on_retire_pending, None)
}

pub(crate) fn render_interactive_job(
    raw: &str,
    cancelled: impl Fn() -> bool,
    on_retire_pending: impl Fn(),
    control: InteractiveControl<'_>,
) -> Result<Vec<u8>, String> {
    render_owned_job(raw, cancelled, on_retire_pending, Some(control))
}

fn render_owned_job(
    raw: &str,
    cancelled: impl Fn() -> bool,
    on_retire_pending: impl Fn(),
    interactive: Option<InteractiveControl<'_>>,
) -> Result<Vec<u8>, String> {
    let url = checked_url(raw)?;
    if cancelled() {
        return Err("CAPTURE_ISOLATED_CANCELLED".into());
    }
    let (parent, root, identity) = fresh_scratch()?;
    let result = (|| {
        if cancelled() {
            return Err("CAPTURE_ISOLATED_CANCELLED".into());
        }
        let exe = std::env::current_exe().map_err(|_| "CAPTURE_EXECUTABLE_UNAVAILABLE")?;
        let mut command = Command::new(exe);
        command
            .arg(if interactive.is_some() { INTERACTIVE_CHILD_FLAG } else { CHILD_FLAG })
            .env(URL_ENV, url)
            .env(ROOT_ENV, &root)
            .stdin(Stdio::piped())
            .env_remove("BROWSEROS_SUPERVISOR_ADDRESS")
            .env_remove("BROWSEROS_SUPERVISOR_TOKEN");
        // Overrides inherited from the GUI/shell must not silently redirect
        // a fresh worker to a persistent profile or a remote debugger.
        #[cfg(target_os = "windows")]
        for name in [
            "WEBVIEW2_USER_DATA_FOLDER",
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
            "WEBVIEW2_RELEASE_CHANNEL_PREFERENCE",
            "WEBVIEW2_PIPE_FOR_SCRIPT_DEBUGGER",
            "WEBVIEW2_WAIT_FOR_SCRIPT_DEBUGGER",
        ] {
            command.env_remove(name);
        }
        #[cfg(target_os = "linux")]
        for name in ["WEBKIT_INSPECTOR_SERVER", "WEBKIT_INSPECTOR_HTTP_SERVER"] {
            command.env_remove(name);
        }
        let mut deadline = Instant::now() + DEADLINE;
        let mut owner = OwnedHost::spawn_command(command, Some(MEMORY_LIMIT), &on_retire_pending)?;
        // Child startup is gated on the existing kernel domain assignment.
        // A failed handoff cannot drop an assigned Child or permit publication.
        if cancelled() || Instant::now() >= deadline {
            owner.retire_confirmed(&on_retire_pending);
            return Err(if cancelled() {
                "CAPTURE_ISOLATED_CANCELLED"
            } else {
                "CAPTURE_DEADLINE_EXCEEDED"
            }.into());
        }
        // All fallible work after spawn stays inside this boundary.
        // In particular, readiness or pipe errors cannot return past retirement.
        let render = (|| -> Result<(), String> {
            let mut pipe = if interactive.is_some() {
                Some(owner.open_start_pipe(START_MAGIC)?)
            } else {
                owner.release_start_pipe(START_MAGIC)?;
                None
            };
            let mut ready = false;
            let mut triggered = false;
            loop {
                if cancelled() {
                    return Err("CAPTURE_ISOLATED_CANCELLED".into());
                }
                if Instant::now() >= deadline {
                    return Err("CAPTURE_DEADLINE_EXCEEDED".into());
                }
                if let Some(status) = owner.exited()? {
                    return if status.success() && (interactive.is_none() || triggered) {
                        Ok(())
                    } else {
                        Err("CAPTURE_WORKER_FAILED".into())
                    };
                }
                if let Some(control) = &interactive {
                    if !ready {
                        let marker = safe_paths::read_versioned(
                            &root.join(READY_FILE),
                            &[root.clone()],
                            READY_MAGIC.len(),
                        )?;
                        if let Some(marker) = marker {
                            if marker.bytes != READY_MAGIC
                                || safe_paths::directory_identity(&root)
                                    .map_err(|_| "CAPTURE_TEMP_IDENTITY_FAILED")? != identity
                            {
                                return Err("CAPTURE_WORKER_READY_INVALID".into());
                            }
                            (control.ready)()?;
                            ready = true;
                            deadline = Instant::now() + INTERACTIVE_WAIT;
                        }
                    }
                    if ready && !triggered && (control.capture_requested)()? {
                        if cancelled() {
                            return Err("CAPTURE_ISOLATED_CANCELLED".into());
                        }
                        let mut endpoint = pipe.take().ok_or("CAPTURE_WORKER_ADMISSION_FAILED")?;
                        endpoint.write_all(&[CAPTURE_BYTE]).map_err(|_| "CAPTURE_WORKER_TRIGGER_FAILED")?;
                        // EOF proves the one-shot command is complete.
                        drop(endpoint);
                        triggered = true;
                        deadline = Instant::now() + DEADLINE;
                    }
                }
                thread::sleep(POLL.min(deadline.saturating_duration_since(Instant::now())));
            }
        })();
        // An exit request or a deadline does not release the GUI's capture
        // slot. Even if the first cleanup budget expires, retain the exact
        // owner and keep trying until the kernel and Child agree on exit.
        owner.retire_confirmed(&on_retire_pending);
        if cancelled() {
            return Err("CAPTURE_ISOLATED_CANCELLED".into());
        }
        render?;
        if safe_paths::directory_identity(&root).map_err(|_| "CAPTURE_TEMP_IDENTITY_FAILED")?
            != identity
        {
            return Err("CAPTURE_TEMP_IDENTITY_CHANGED".into());
        }
        let bytes = safe_paths::read_regular(&root.join("capture.png"), &[root.clone()], MAX_BYTES)
            .map_err(|_| "CAPTURE_WORKER_RESULT_UNAVAILABLE")?;
        checked_png(&bytes)?;
        Ok(bytes)
    })();
    // spawn_command returns only pre-dispatch failure or a fully retired
    // failed assignment. Unknown termination retains the owner above.
    if let Err(error) = cleanup(&parent, &root, &identity) {
        eprintln!(
            "[isolated-capture] {error}; scratch retained: {}",
            root.display()
        );
    }
    result
}

fn run_parent(raw: &str, output: PathBuf) -> Result<(), String> {
    if !output.is_absolute() || output.extension() != Some(OsStr::new("png")) {
        return Err("CAPTURE_ABSOLUTE_NEW_PNG_PATH_REQUIRED".into());
    }
    let output_parent = output
        .parent()
        .ok_or("CAPTURE_OUTPUT_PATH_INVALID")?
        .to_path_buf();
    match std::fs::symlink_metadata(&output) {
        Ok(_) => return Err("CAPTURE_OUTPUT_ALREADY_EXISTS".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("CAPTURE_OUTPUT_PATH_INVALID".into()),
    }
    let bytes = render_job(
        raw,
        || false,
        || {
            eprintln!("[isolated-capture] renderer exit unconfirmed; retaining owner");
        },
    )?;
    safe_paths::create_new_file(&output, &[output_parent], &bytes)
        .map_err(|_| "CAPTURE_OUTPUT_NO_REPLACE_FAILED".into())
}
fn worker_root() -> Result<PathBuf, String> {
    let root = std::env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .ok_or("CAPTURE_WORKER_ROOT_REQUIRED")?;
    let parent = scratch_parent()?;
    let name = root
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or("CAPTURE_WORKER_ROOT_INVALID")?;
    let id = name
        .strip_prefix("browseros-isolated-capture-")
        .ok_or("CAPTURE_WORKER_ROOT_INVALID")?;
    if uuid::Uuid::parse_str(id).map_or(true, |id| id.get_version_num() != 4)
        || root.parent() != Some(parent.as_path())
        || root.canonicalize().ok().as_ref() != Some(&root)
    {
        return Err("CAPTURE_WORKER_ROOT_INVALID".into());
    }
    safe_paths::ensure_private_directory(&root).map_err(|_| "CAPTURE_TEMP_PRIVATE_FAILED")?;
    Ok(root)
}
// Vertical bands cover every CSS pixel exactly once. The existing full
// document limits remain in force; a band has an independent 1M-pixel
// limit, and the final CPU assembly remains bounded inside the Worker.
#[cfg(any(target_os = "windows", test))]
#[derive(Debug, PartialEq, Eq)]
struct BandPlan {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    rows: i32,
}
#[cfg(any(target_os = "windows", test))]
impl BandPlan {
    fn from_metrics(metrics: &serde_json::Value) -> Result<Self, String> {
        let field = |name: &str| -> Result<i32, String> {
            let n = metrics["clip"][name]
                .as_f64()
                .ok_or("CAPTURE_METRICS_INVALID")?;
            // Reject fractional clips rather than create a rounded seam or
            // silently rescale a high-DPI band.
            if !n.is_finite() || n.fract() != 0.0 || n.abs() > 16_384.0 {
                return Err("CAPTURE_BAND_COORDINATES_UNSUPPORTED".into());
            }
            Ok(n as i32)
        };
        let (x, y, width, height) = (field("x")?, field("y")?, field("width")?, field("height")?);
        if width <= 0 || height <= 0 || i64::from(width) * i64::from(height) > 16_000_000 {
            return Err("CAPTURE_DIMENSION_LIMIT".into());
        }
        let rows = (1_000_000 / width).min(1024);
        Ok(Self {
            x,
            y,
            width,
            height,
            rows,
        })
    }
    fn bands(&self) -> impl Iterator<Item = [i32; 4]> + '_ {
        (0..self.height)
            .step_by(self.rows as usize)
            .map(move |offset| {
                [
                    self.x,
                    self.y + offset,
                    self.width,
                    self.rows.min(self.height - offset),
                ]
            })
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
fn rgba_band(bytes: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > 1_000_000
        || bytes.len() > 6 * 1024 * 1024
    {
        return Err("CAPTURE_BAND_LIMIT".into());
    }
    checked_png_header(bytes)?;
    if !bytes.ends_with(b"\0\0\0\0IEND\xaeB\x60\x82") {
        return Err("CAPTURE_INVALID_PNG".into());
    }
    let mut limits = png::Limits::default();
    limits.bytes = 16 * 1024 * 1024;
    let mut decoder = png::Decoder::new_with_limits(std::io::Cursor::new(bytes), limits);
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| "CAPTURE_INVALID_PNG")?;
    let info = reader.info();
    if info.width != width || info.height != height || info.animation_control.is_some() {
        return Err("CAPTURE_BAND_SCALE_OR_SIZE_CHANGED".into());
    }
    let pixels = width as usize * height as usize;
    let size = reader
        .output_buffer_size()
        .filter(|size| *size <= pixels * 4)
        .ok_or("CAPTURE_BAND_LIMIT")?;
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(size)
        .map_err(|_| "CAPTURE_ALLOCATION_FAILED")?;
    decoded.resize(size, 0);
    let frame = reader
        .next_frame(&mut decoded)
        .map_err(|_| "CAPTURE_INVALID_PNG")?;
    reader.finish().map_err(|_| "CAPTURE_INVALID_PNG")?;
    if frame.width != width || frame.height != height || frame.bit_depth != png::BitDepth::Eight {
        return Err("CAPTURE_BAND_SCALE_OR_SIZE_CHANGED".into());
    }
    decoded.truncate(frame.buffer_size());
    if frame.color_type == png::ColorType::Rgba && decoded.len() == pixels * 4 {
        return Ok(decoded);
    }
    if frame.color_type != png::ColorType::Rgb || decoded.len() != pixels * 3 {
        return Err("CAPTURE_BAND_COLOR_UNSUPPORTED".into());
    }
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(pixels * 4)
        .map_err(|_| "CAPTURE_ALLOCATION_FAILED")?;
    for rgb in decoded.chunks_exact(3) {
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    Ok(rgba)
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
fn verify_stable_pixels(first: &[u8], second: &[u8]) -> Result<(), String> {
    if first != second {
        return Err("CAPTURE_PAINT_CHANGED".into());
    }
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
#[derive(Default)]
struct BoundedOutput(Vec<u8>);
#[cfg(any(target_os = "windows", target_os = "linux", test))]
impl std::io::Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_BYTES - self.0.len() {
            return Err(std::io::Error::other("CAPTURE_SIZE_LIMIT"));
        }
        let required = self.0.len() + bytes.len();
        if required > self.0.capacity() {
            let capacity = required
                .max(self.0.capacity().saturating_mul(2))
                .min(MAX_BYTES);
            self.0
                .try_reserve_exact(capacity - self.0.len())
                .map_err(|_| std::io::Error::other("CAPTURE_ALLOCATION_FAILED"))?;
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn capture_windows_bands(
    manager: &TabManager,
    id: &String,
    document: &tauri_plugin_browser_tabs::CaptureDocument,
    deadline: Instant,
) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let current =
        || manager.isolated_document_ready() && manager.is_current_capture_document(id, document);
    let request = |band: Option<[i32; 4]>| -> Result<Vec<u8>, String> {
        if Instant::now() >= deadline {
            return Err("CAPTURE_DEADLINE_EXCEEDED".into());
        }
        if !current() {
            return Err("CAPTURE_DOCUMENT_STALE".into());
        }
        let cancellation = CaptureCancellation::new();
        let (tx, rx) = mpsc::sync_channel(1);
        manager
            .capture_isolated_band_request(
                id,
                document.clone(),
                cancellation.clone(),
                band,
                move |result| {
                    let _ = tx.try_send(result);
                },
            )
            .map_err(|_| "CAPTURE_RENDER_DISPATCH_FAILED")?;
        let bytes = match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(result) => result?,
            Err(_) => {
                cancellation.cancel();
                return Err("CAPTURE_RENDER_OUTCOME_UNKNOWN".into());
            }
        };
        if !current() {
            return Err("CAPTURE_DOCUMENT_STALE".into());
        }
        Ok(bytes)
    };
    let metrics = request(None)?;
    let layout: serde_json::Value =
        serde_json::from_slice(&metrics).map_err(|_| "CAPTURE_METRICS_INVALID")?;
    let plan = BandPlan::from_metrics(&layout)?;
    let mut output = BoundedOutput::default();
    {
        let mut encoder = png::Encoder::new(&mut output, plan.width as u32, plan.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
        {
            let mut stream = writer
                .stream_writer()
                .map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
            for band in plan.bands() {
                // No scrolling, DOM mutation or reloading. Reject observable
                // layout/viewport changes; same-size visual mutations remain
                // best effort and are not an atomic document snapshot.
                if request(None)? != metrics {
                    return Err("CAPTURE_LAYOUT_CHANGED".into());
                }
                let png = request(Some(band))?;
                let rows = rgba_band(&png, band[2] as u32, band[3] as u32)?;
                let check = request(Some(band))?;
                let confirmation = rgba_band(&check, band[2] as u32, band[3] as u32)?;
                verify_stable_pixels(&rows, &confirmation)?;
                stream
                    .write_all(&rows)
                    .map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
            }
            if request(None)? != metrics {
                return Err("CAPTURE_LAYOUT_CHANGED".into());
            }
            stream.finish().map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
        }
        writer.finish().map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
    }
    if !current() {
        return Err("CAPTURE_DOCUMENT_STALE".into());
    }
    Ok(output.0)
}

#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LinuxViewportPlan {
    width: u32,
    height: u32,
    viewport_width: u32,
    viewport_height: u32,
    revision: u64,
}
#[cfg(any(target_os = "linux", test))]
impl LinuxViewportPlan {
    fn parse(value: &serde_json::Value) -> Result<Option<Self>, String> {
        if value["eligible"] == false {
            return Ok(None);
        }
        if value["eligible"] != true {
            return Err("CAPTURE_METRICS_INVALID".into());
        }
        let field = |name: &str| -> Result<u32, String> {
            value[name]
                .as_u64()
                .filter(|n| (1..=16_384).contains(n))
                .map(|n| n as u32)
                .ok_or_else(|| "CAPTURE_METRICS_INVALID".into())
        };
        let width = field("width")?;
        let height = field("height")?;
        let viewport_width = field("viewport_width")?;
        let viewport_height = field("viewport_height")?;
        if u64::from(width) * u64::from(height) > 16_000_000 {
            return Err("CAPTURE_DIMENSION_LIMIT".into());
        }
        if value["scale"].as_f64() != Some(1.0)
            || value["visual_scale"].as_f64() != Some(1.0)
            || value["client_width"].as_u64() != Some(u64::from(width))
            || value["client_height"].as_u64() != Some(u64::from(viewport_height))
            || value["x"].as_f64() != Some(0.0)
            || width > viewport_width
            || height < viewport_height
            || u64::from(viewport_width) * u64::from(viewport_height) > 1_000_000
        {
            return Ok(None);
        }
        Ok(Some(Self {
            width,
            height,
            viewport_width,
            viewport_height,
            revision: value["revision"]
                .as_u64()
                .ok_or("CAPTURE_METRICS_INVALID")?,
        }))
    }
    fn bands(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        (0..self.height)
            .step_by(self.viewport_height as usize)
            .map(move |top| {
                let target = top.min(self.height - self.viewport_height);
                (
                    target,
                    top - target,
                    self.viewport_height.min(self.height - top),
                )
            })
    }
    fn check_position(&self, value: &serde_json::Value, y: u32) -> Result<(), String> {
        if Self::parse(value)?.as_ref() != Some(self) || value["y"].as_f64() != Some(f64::from(y)) {
            return Err("CAPTURE_LAYOUT_OR_SCROLL_CHANGED".into());
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn capture_linux_viewports(
    manager: &TabManager,
    id: &String,
    document: &tauri_plugin_browser_tabs::CaptureDocument,
    deadline: Instant,
) -> Result<Option<Vec<u8>>, String> {
    use std::io::Write;
    let current =
        || manager.isolated_document_ready() && manager.is_current_capture_document(id, document);
    let request = |target: Option<i32>, snapshot: bool| -> Result<Vec<u8>, String> {
        if Instant::now() >= deadline {
            return Err("CAPTURE_DEADLINE_EXCEEDED".into());
        }
        if !current() {
            return Err("CAPTURE_DOCUMENT_STALE".into());
        }
        let cancellation = CaptureCancellation::new();
        let (tx, rx) = mpsc::sync_channel(1);
        let callback = move |result| {
            let _ = tx.try_send(result);
        };
        let dispatched = if snapshot {
            manager.capture_isolated_visible(id, document.clone(), cancellation.clone(), callback)
        } else {
            manager.capture_isolated_linux_metrics(
                id,
                document.clone(),
                cancellation.clone(),
                target,
                callback,
            )
        };
        dispatched.map_err(|_| "CAPTURE_RENDER_DISPATCH_FAILED")?;
        let bytes = match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(result) => result?,
            Err(_) => {
                cancellation.cancel();
                return Err("CAPTURE_RENDER_OUTCOME_UNKNOWN".into());
            }
        };
        if !current() {
            return Err("CAPTURE_DOCUMENT_STALE".into());
        }
        Ok(bytes)
    };
    let metrics = |target| -> Result<serde_json::Value, String> {
        serde_json::from_slice(&request(target, false)?)
            .map_err(|_| "CAPTURE_METRICS_INVALID".into())
    };
    let initial = metrics(None)?;
    let Some(plan) = LinuxViewportPlan::parse(&initial)? else {
        return Ok(None);
    };
    if initial["y"].as_f64() != Some(0.0) {
        return Ok(None);
    }
    let mut output = BoundedOutput::default();
    {
        let mut encoder = png::Encoder::new(&mut output, plan.width, plan.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
        {
            let mut stream = writer
                .stream_writer()
                .map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
            for (target, skip, rows) in plan.bands() {
                plan.check_position(&metrics(Some(target as i32))?, target)?;
                // A bounded repaint opportunity, not proof of atomic paint.
                thread::sleep(Duration::from_millis(32));
                plan.check_position(&metrics(None)?, target)?;
                let bytes = request(None, true)?;
                let pixels = rgba_band(&bytes, plan.viewport_width, plan.viewport_height)?;
                plan.check_position(&metrics(None)?, target)?;
                let confirmation = rgba_band(
                    &request(None, true)?,
                    plan.viewport_width,
                    plan.viewport_height,
                )?;
                plan.check_position(&metrics(None)?, target)?;
                verify_stable_pixels(&pixels, &confirmation)?;
                let stride = plan.viewport_width as usize * 4;
                let row_width = plan.width as usize * 4;
                for row in pixels
                    .chunks_exact(stride)
                    .skip(skip as usize)
                    .take(rows as usize)
                {
                    stream
                        .write_all(&row[..row_width])
                        .map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
                }
            }
            stream.finish().map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
        }
        writer.finish().map_err(|_| "CAPTURE_PNG_ENCODE_FAILED")?;
    }
    if !current() {
        return Err("CAPTURE_DOCUMENT_STALE".into());
    }
    Ok(Some(output.0))
}

fn run_render(
    manager: Arc<TabManager>,
    root: PathBuf,
    url: String,
    interactive: bool,
) -> Result<(), String> {
    manager
        .prepare_storage()
        .map_err(|_| "CAPTURE_PROFILE_PREPARE_FAILED")?;
    let id = PAGE.to_string();
    manager
        .create_tab(CreateTabOptions {
            id: id.clone(),
            url,
            rect: LogicalRect::new(0.0, 0.0, 1280.0, 720.0),
            visible: true,
            auto_resize: false,
            user_agent: None,
            transparent: false,
            initialization_script: None,
        })
        .map_err(|_| "CAPTURE_RENDER_CREATE_FAILED")?;
    let mut deadline = Instant::now() + DEADLINE;
    // Actual page-load Finished notification, not a fixed sleep or URI change.
    while !manager.isolated_document_ready() {
        if Instant::now() >= deadline {
            return Err("CAPTURE_DOCUMENT_LOADING".into());
        }
        thread::sleep(POLL);
    }
    if interactive {
        // The same WebView remains available for login and navigation.
        // Never copy the ordinary browser profile or re-open the URL.
        safe_paths::create_new_file(&root.join(READY_FILE), &[root.clone()], READY_MAGIC)
            .map_err(|_| "CAPTURE_WORKER_READY_FAILED")?;
        // The parent owns the bounded wait and can retire the entire domain.
        // This read is on the dedicated render thread, not the GUI thread.
        validate_capture_command(std::io::stdin().lock())?;
        deadline = Instant::now() + DEADLINE;
        if !manager.isolated_document_ready() {
            return Err("CAPTURE_DOCUMENT_LOADING".into());
        }
    }
    let document = manager
        .capture_document(&id)
        .map_err(|_| "CAPTURE_DOCUMENT_UNAVAILABLE")?;
    if !manager.isolated_document_ready() {
        return Err("CAPTURE_DOCUMENT_STALE".into());
    }
    #[cfg(target_os = "windows")]
    let bytes = capture_windows_bands(&manager, &id, &document, deadline)?;
    // Fallback only before a scroll is attempted; never call a FullDocument
    // capture after a failed or stale viewport stitching attempt.
    #[cfg(target_os = "linux")]
    let bytes = if let Some(bytes) = if interactive {
        // Do not scroll an authenticated interactive document. FullDocument
        // is permitted here only inside the already budgeted process domain.
        None
    } else {
        capture_linux_viewports(&manager, &id, &document, deadline)?
    } {
        bytes
    } else {
        eprintln!("[isolated-capture] full-document mode");
        let cancellation = CaptureCancellation::new();
        let (tx, rx) = mpsc::sync_channel(1);
        manager
            .capture_bound_document(&id, document.clone(), cancellation.clone(), move |result| {
                let _ = tx.try_send(result);
            })
            .map_err(|_| "CAPTURE_RENDER_DISPATCH_FAILED")?;
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(result) => result?,
            Err(_) => {
                cancellation.cancel();
                return Err("CAPTURE_RENDER_OUTCOME_UNKNOWN".into());
            }
        }
    };
    if !manager.isolated_document_ready() || !manager.is_current_capture_document(&id, &document) {
        return Err("CAPTURE_DOCUMENT_STALE".into());
    }
    checked_png_header(&bytes)?;
    safe_paths::create_new_file(&root.join("capture.png"), &[root], &bytes)
        .map_err(|_| "CAPTURE_WORKER_OUTPUT_FAILED".into())
}
fn validate_worker_prefix(mut input: impl Read) -> Result<(), String> {
    let mut magic = [0u8; START_MAGIC.len()];
    input.read_exact(&mut magic).map_err(|_| "CAPTURE_WORKER_ADMISSION_FAILED")?;
    if magic.as_slice() != START_MAGIC {
        return Err("CAPTURE_WORKER_ADMISSION_FAILED".into());
    }
    Ok(())
}

fn validate_worker_admission(mut input: impl Read) -> Result<(), String> {
    validate_worker_prefix(&mut input)?;
    let mut extra = [0u8; 1];
    if input.read(&mut extra).map_err(|_| "CAPTURE_WORKER_ADMISSION_FAILED")? != 0 {
        return Err("CAPTURE_WORKER_ADMISSION_FAILED".into());
    }
    Ok(())
}

fn validate_capture_command(mut input: impl Read) -> Result<(), String> {
    let mut byte = [0u8; 1];
    input.read_exact(&mut byte).map_err(|_| "CAPTURE_WORKER_TRIGGER_FAILED")?;
    if byte[0] != CAPTURE_BYTE
        || input.read(&mut byte).map_err(|_| "CAPTURE_WORKER_TRIGGER_FAILED")? != 0
    {
        return Err("CAPTURE_WORKER_TRIGGER_FAILED".into());
    }
    Ok(())
}

fn run_child(interactive: bool) -> Result<(), String> {
    // The internal CLI role cannot start an unbudgeted renderer even when
    // invoked directly with a valid scratch path and URL. Verify kernel
    // membership independently, before creating Tauri or any WebView.
    #[cfg(target_os = "linux")]
    crate::shared::process_scope::verify_current_render_budget(MEMORY_LIMIT)
        .map_err(|_| "CAPTURE_WORKER_MEMORY_OWNER_UNCONFIRMED")?;
    #[cfg(target_os = "windows")]
    crate::windows_process::verify_current_render_budget(MEMORY_LIMIT)?;
    if interactive {
        validate_worker_prefix(std::io::stdin().lock())?;
    } else {
        validate_worker_admission(std::io::stdin().lock())?;
    }
    let root = worker_root()?;
    let url = checked_url(&std::env::var(URL_ENV).map_err(|_| "CAPTURE_WORKER_URL_REQUIRED")?)?;
    // Clear request-specific state before the native engine starts any
    // subprocess. The render job never invokes the ordinary app bootstrap.
    std::env::remove_var(URL_ENV);
    std::env::remove_var(ROOT_ENV);
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("GDK_BACKEND", "x11");
        std::env::set_var("GTK_USE_PORTAL", "0");
    }
    let mut context = tauri::generate_context!();
    context.config_mut().app.with_global_tauri = false;
    context.config_mut().app.security.asset_protocol.enable = false;
    tauri::Builder::default()
        // No application plugins or invoke_handler. Labels do not match
        // normal main/tab/grid capabilities. The worker is not a sandbox.
        .setup(move |app| {
            tauri::window::WindowBuilder::new(app, HOST)
                .title(if interactive {
                    "BrowserOS isolated login capture - temporary session"
                } else {
                    "BrowserOS independent capture (fresh profile)"
                })
                .inner_size(1280.0, 720.0)
                .resizable(!interactive)
                .build()?;
            // Register the exact private manager before dispatch. Windows' CDP
            // metrics callback checks this managed state before taking the image.
            // Managing native state does not install the tab IPC plugin.
            let manager = Arc::new(TabManager::new_isolated_capture(
                app.handle().clone(),
                HOST,
                root.clone(),
            ));
            if !app.manage(manager.clone()) {
                return Err(std::io::Error::other("CAPTURE_MANAGER_ALREADY_REGISTERED").into());
            }
            let root = root.clone();
            let url = url.clone();
            thread::Builder::new()
                .name("browseros-isolated-render".into())
                .spawn(move || match run_render(manager, root, url, interactive) {
                    Ok(()) => std::process::exit(0),
                    Err(_) => std::process::exit(2),
                })?;
            Ok(())
        })
        .run(context)
        .map_err(|_| "CAPTURE_RENDER_RUNTIME_FAILED".to_owned())?;
    // A window close is not a completed capture.
    Err("CAPTURE_WORKER_CLOSED".into())
}
pub(crate) fn launch_if_requested() -> Option<i32> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let first = args.first()?.as_os_str();
    let result = if first == OsStr::new(PARENT_FLAG) {
        if args.len() != 3 {
            Err("usage: --isolated-capture HTTPS_URL ABSOLUTE_NEW_PNG".into())
        } else {
            match args[1].to_str() {
                Some(url) => run_parent(url, PathBuf::from(&args[2])),
                None => Err("CAPTURE_URL_INVALID".into()),
            }
        }
    } else if first == OsStr::new(CHILD_FLAG) || first == OsStr::new(INTERACTIVE_CHILD_FLAG) {
        if args.len() != 1 {
            Err("CAPTURE_WORKER_ARGUMENTS_INVALID".into())
        } else {
            run_child(first == OsStr::new(INTERACTIVE_CHILD_FLAG))
        }
    } else {
        return None;
    };
    Some(match result {
        Ok(()) => 0,
        Err(code) => {
            eprintln!("[isolated-capture] {code}");
            1
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn internal_worker_requires_exact_closed_admission_pipe() {
        assert!(validate_worker_admission(START_MAGIC).is_ok());
        assert!(validate_worker_admission(&START_MAGIC[..START_MAGIC.len() - 1]).is_err());
        assert!(validate_worker_admission(b"wrong".as_slice()).is_err());
        let mut extra = START_MAGIC.to_vec();
        extra.push(0);
        assert!(validate_worker_admission(extra.as_slice()).is_err());
    }

    #[test]
    fn interactive_pipe_accepts_only_one_capture_command() {
        let mut admission = START_MAGIC.to_vec();
        admission.push(CAPTURE_BYTE);
        let mut input = std::io::Cursor::new(admission);
        assert!(validate_worker_prefix(&mut input).is_ok());
        assert!(validate_capture_command(&mut input).is_ok());
        assert!(validate_capture_command(b"".as_slice()).is_err());
        assert!(validate_capture_command(b"CC".as_slice()).is_err());
        assert!(validate_capture_command(b"X".as_slice()).is_err());
    }

    #[test]
    fn explicit_mode_rejects_non_https_and_embedded_credentials() {
        assert!(checked_url("https://example.com/").is_ok());
        assert!(checked_url("file:///tmp/page.html").is_err());
        assert!(checked_url("http://example.com/").is_err());
        assert!(checked_url("https://user:pass@example.com/").is_err());
    }
    #[test]
    fn complete_png_is_required_before_publication() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[127u8; 16]).unwrap();
            writer.finish().unwrap();
        }
        assert!(checked_png(&bytes).is_ok());
        assert!(checked_png(&bytes[..bytes.len() - 1]).is_err());
        let idat = bytes.windows(4).position(|chunk| chunk == b"IDAT").unwrap();
        bytes[idat + 4] ^= 1;
        assert!(checked_png(&bytes).is_err());
    }

    #[test]
    fn linux_viewports_cover_clamped_final_scroll_without_duplicate_rows() {
        let initial = serde_json::json!({"eligible":true,"width":1265,"height":1500,
            "client_width":1265,"client_height":720,"viewport_width":1280,"viewport_height":720,
            "scale":1,"visual_scale":1,"x":0,"y":0,"revision":0});
        let plan = LinuxViewportPlan::parse(&initial).unwrap().unwrap();
        assert_eq!(
            plan.bands().collect::<Vec<_>>(),
            vec![(0, 0, 720), (720, 0, 720), (780, 660, 60)]
        );
        let mut changed = initial.clone();
        changed["y"] = serde_json::json!(720);
        assert!(plan.check_position(&changed, 720).is_ok());
        changed["revision"] = serde_json::json!(1);
        assert!(plan.check_position(&changed, 720).is_err());
        changed["scale"] = serde_json::json!(2);
        assert!(LinuxViewportPlan::parse(&changed).unwrap().is_none());
        assert!(
            LinuxViewportPlan::parse(&serde_json::json!({"eligible":false}))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn bands_cover_exact_rows_without_gaps_or_overlaps() {
        let plan = BandPlan::from_metrics(&serde_json::json!({
            "clip":{"x":-5,"y":7,"width":1200,"height":4000}
        }))
        .unwrap();
        let mut y = 7;
        for [x, top, width, height] in plan.bands() {
            assert_eq!((x, top, width), (-5, y, 1200));
            assert!(width * height <= 1_000_000);
            y += height;
        }
        assert_eq!(y, 4007);
        assert!(BandPlan::from_metrics(&serde_json::json!({
            "clip":{"x":0,"y":0.5,"width":1200,"height":4000}
        }))
        .is_err());
        assert!(BandPlan::from_metrics(&serde_json::json!({
            "clip":{"x":0,"y":0,"width":16000,"height":16000}
        }))
        .is_err());
    }

    #[test]
    fn band_decode_rejects_scale_changes_and_preserves_rgb() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[1, 2, 3, 4, 5, 6]).unwrap();
            writer.finish().unwrap();
        }
        assert_eq!(
            rgba_band(&bytes, 2, 1).unwrap(),
            vec![1, 2, 3, 255, 4, 5, 6, 255]
        );
        assert!(rgba_band(&bytes, 1, 1).is_err());
        assert!(rgba_band(&bytes[..bytes.len() - 1], 2, 1).is_err());
        use std::io::Write;
        let mut output = BoundedOutput(vec![0; MAX_BYTES - 1]);
        assert!(output.write_all(&[1, 2]).is_err());
        assert_eq!(output.0.len(), MAX_BYTES - 1);
    }

    #[test]
    fn repeated_band_rejects_same_size_visual_changes() {
        assert!(verify_stable_pixels(&[1, 2, 3, 255], &[1, 2, 3, 255]).is_ok());
        assert!(verify_stable_pixels(&[1, 2, 3, 255], &[1, 2, 4, 255]).is_err());
        assert!(verify_stable_pixels(&[1, 2, 3, 255], &[1, 2]).is_err());
    }

    #[test]
    fn output_requires_bounded_png_dimensions() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&1280u32.to_be_bytes());
        png.extend_from_slice(&720u32.to_be_bytes());
        assert!(checked_png_header(&png).is_ok());
        assert!(
            checked_png(&png).is_err(),
            "IHDR alone is not a complete image"
        );
        png[16..20].copy_from_slice(&20_000u32.to_be_bytes());
        assert!(checked_png_header(&png).is_err());
    }
}
