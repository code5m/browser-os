//! Explicit fresh-URL capture process. No session migration, no normal
//! app/remote command handlers, no shared browser profile, no live-tab reload.
//! The parent owns a memory-limited native process domain and publishes output
//! only after recursive-empty accounting and the original child exit.
//! This is process lifecycle containment, NOT a hostile same-UID sandbox.
use crate::capabilities::workspace::safe_paths;
use crate::native_supervisor::OwnedHost;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;
use tauri_plugin_browser_tabs::{CaptureCancellation, CreateTabOptions, LogicalRect, TabManager};

const PARENT_FLAG: &str = "--isolated-capture";
const CHILD_FLAG: &str = "--isolated-capture-worker";
const URL_ENV: &str = "BROWSEROS_CAPTURE_JOB_URL";
const ROOT_ENV: &str = "BROWSEROS_CAPTURE_JOB_ROOT";
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MEMORY_LIMIT: usize = 2 * 1024 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);
const HOST: &str = "isolated-capture-host";
const PAGE: &str = "isolated-capture-page";

pub(crate) fn checked_url(raw: &str) -> Result<String, String> {
    if raw.len() > 8192 { return Err("CAPTURE_URL_INVALID".into()); }
    let url = url::Url::parse(raw).map_err(|_| "CAPTURE_URL_INVALID")?;
    if url.scheme() != "https" || url.host_str().is_none()
        || !url.username().is_empty() || url.password().is_some() {
        return Err("CAPTURE_HTTPS_URL_REQUIRED".into());
    }
    Ok(url.into())
}
fn checked_png_header(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 24 || bytes.len() > MAX_BYTES
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || &bytes[12..16] != b"IHDR" {
        return Err("CAPTURE_INVALID_PNG".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| "CAPTURE_INVALID_PNG")?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| "CAPTURE_INVALID_PNG")?);
    if width == 0 || height == 0 || width > 16_384 || height > 16_384
        || u64::from(width) * u64::from(height) > 16_000_000 {
        return Err("CAPTURE_DIMENSION_LIMIT".into());
    }
    Ok(())
}
// Validate the complete worker artifact before publishing it. Decode one
// row/pass at a time instead of allocating a second full raster in the host.
// The existing encoded-byte and dimension limits still apply. PNG decoder
// limits are best-effort intermediate allocation limits, not renderer limits.
fn checked_png(bytes: &[u8]) -> Result<(), String> {
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
    while reader.next_interlaced_row().map_err(|_| "CAPTURE_INVALID_PNG")?.is_some() {}
    reader.finish().map_err(|_| "CAPTURE_INVALID_PNG")?;
    Ok(())
}
fn scratch_parent() -> Result<PathBuf, String> {
    std::env::temp_dir().canonicalize().map_err(|_| "CAPTURE_TEMP_UNAVAILABLE".into())
}
fn fresh_scratch() -> Result<(PathBuf, PathBuf, String), String> {
    let parent = scratch_parent()?;
    let root = parent.join(format!("browseros-isolated-capture-{}", uuid::Uuid::new_v4()));
    // No sensitive content until the OS private-directory policy is applied.
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&root).map_err(|_| "CAPTURE_TEMP_CREATE_FAILED")?;
    safe_paths::ensure_private_directory(&root).map_err(|_| "CAPTURE_TEMP_PRIVATE_FAILED")?;
    let identity = safe_paths::directory_identity(&root).map_err(|_| "CAPTURE_TEMP_IDENTITY_FAILED")?;
    Ok((parent, root, identity))
}
fn cleanup(parent: &Path, root: &Path, identity: &str) -> Result<(), String> {
    if safe_paths::directory_identity(root).map_err(|_| "CAPTURE_TEMP_IDENTITY_FAILED")? != identity {
        return Err("CAPTURE_TEMP_IDENTITY_CHANGED".into());
    }
    safe_paths::remove_tree(root, &[parent.to_path_buf()]).map_err(|_| "CAPTURE_TEMP_CLEANUP_FAILED".into())
}
/// Same execution path for the explicit CLI and Browser Owner. Cancellation
/// does not return until the owned process domain has really been retired.
/// The pending callback is metadata-only; it never releases admission.
pub(crate) fn render_job(
    raw: &str, cancelled: impl Fn() -> bool, on_retire_pending: impl Fn(),
) -> Result<Vec<u8>, String> {
    let url = checked_url(raw)?;
    if cancelled() { return Err("CAPTURE_ISOLATED_CANCELLED".into()); }
    let (parent, root, identity) = fresh_scratch()?;
    let result = (|| {
        if cancelled() { return Err("CAPTURE_ISOLATED_CANCELLED".into()); }
        let exe = std::env::current_exe().map_err(|_| "CAPTURE_EXECUTABLE_UNAVAILABLE")?;
        let mut command = Command::new(exe);
        command.arg(CHILD_FLAG).env(URL_ENV, url).env(ROOT_ENV, &root);
        let mut owner = OwnedHost::spawn_command(command, Some(MEMORY_LIMIT), &on_retire_pending)?;
        let deadline = Instant::now() + DEADLINE;
        let render = loop {
            if cancelled() { break Err("CAPTURE_ISOLATED_CANCELLED".to_owned()); }
            match owner.exited() {
                Ok(Some(status)) => break if status.success() { Ok(()) } else { Err("CAPTURE_WORKER_FAILED".to_owned()) },
                Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
                Ok(None) => break Err("CAPTURE_DEADLINE_EXCEEDED".to_owned()),
                Err(error) => break Err(error),
            }
        };
        // An exit request or a deadline does not release the GUI's capture
        // slot. Even if the first cleanup budget expires, retain the exact
        // owner and keep trying until the kernel and Child agree on exit.
        owner.retire_confirmed(&on_retire_pending);
        if cancelled() { return Err("CAPTURE_ISOLATED_CANCELLED".into()); }
        render?;
        if safe_paths::directory_identity(&root).map_err(|_| "CAPTURE_TEMP_IDENTITY_FAILED")? != identity {
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
        eprintln!("[isolated-capture] {error}; scratch retained: {}", root.display());
    }
    result
}

fn run_parent(raw: &str, output: PathBuf) -> Result<(), String> {
    if !output.is_absolute() || output.extension() != Some(OsStr::new("png")) {
        return Err("CAPTURE_ABSOLUTE_NEW_PNG_PATH_REQUIRED".into());
    }
    let output_parent = output.parent().ok_or("CAPTURE_OUTPUT_PATH_INVALID")?.to_path_buf();
    match std::fs::symlink_metadata(&output) {
        Ok(_) => return Err("CAPTURE_OUTPUT_ALREADY_EXISTS".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
        Err(_) => return Err("CAPTURE_OUTPUT_PATH_INVALID".into()),
    }
    let bytes = render_job(raw, || false, || {
        eprintln!("[isolated-capture] renderer exit unconfirmed; retaining owner");
    })?;
    safe_paths::create_new_file(&output, &[output_parent], &bytes)
        .map_err(|_| "CAPTURE_OUTPUT_NO_REPLACE_FAILED".into())
}
fn worker_root() -> Result<PathBuf, String> {
    let root = std::env::var_os(ROOT_ENV).map(PathBuf::from).ok_or("CAPTURE_WORKER_ROOT_REQUIRED")?;
    let parent = scratch_parent()?;
    let name = root.file_name().and_then(OsStr::to_str).ok_or("CAPTURE_WORKER_ROOT_INVALID")?;
    let id = name.strip_prefix("browseros-isolated-capture-").ok_or("CAPTURE_WORKER_ROOT_INVALID")?;
    if uuid::Uuid::parse_str(id).map_or(true, |id| id.get_version_num() != 4)
        || root.parent() != Some(parent.as_path())
        || root.canonicalize().ok().as_ref() != Some(&root) {
        return Err("CAPTURE_WORKER_ROOT_INVALID".into());
    }
    safe_paths::ensure_private_directory(&root).map_err(|_| "CAPTURE_TEMP_PRIVATE_FAILED")?;
    Ok(root)
}
fn run_render(manager: Arc<TabManager>, root: PathBuf, url: String) -> Result<(), String> {
    manager.prepare_storage().map_err(|_| "CAPTURE_PROFILE_PREPARE_FAILED")?;
    let id = PAGE.to_string();
    manager.create_tab(CreateTabOptions {
        id: id.clone(), url, rect: LogicalRect::new(0.0, 0.0, 1280.0, 720.0),
        visible: true, auto_resize: false, user_agent: None, transparent: false,
        initialization_script: None,
    }).map_err(|_| "CAPTURE_RENDER_CREATE_FAILED")?;
    let deadline = Instant::now() + DEADLINE;
    // Actual page-load Finished notification, not a fixed sleep or URI change.
    while !manager.isolated_document_ready() {
        if Instant::now() >= deadline { return Err("CAPTURE_DOCUMENT_LOADING".into()); }
        thread::sleep(POLL);
    }
    let document = manager.capture_document(&id).map_err(|_| "CAPTURE_DOCUMENT_UNAVAILABLE")?;
    if !manager.isolated_document_ready() { return Err("CAPTURE_DOCUMENT_STALE".into()); }
    let cancellation = CaptureCancellation::new();
    let (tx, rx) = mpsc::sync_channel(1);
    manager.capture_bound_document(&id, document.clone(), cancellation.clone(), move |result| {
        let _ = tx.try_send(result);
    }).map_err(|_| "CAPTURE_RENDER_DISPATCH_FAILED")?;
    let bytes = match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(result) => result?,
        Err(_) => {
            cancellation.cancel();
            return Err("CAPTURE_RENDER_OUTCOME_UNKNOWN".into());
        }
    };
    if !manager.isolated_document_ready() || !manager.is_current_capture_document(&id, &document) {
        return Err("CAPTURE_DOCUMENT_STALE".into());
    }
    checked_png_header(&bytes)?;
    safe_paths::create_new_file(&root.join("capture.png"), &[root], &bytes)
        .map_err(|_| "CAPTURE_WORKER_OUTPUT_FAILED".into())
}
fn run_child() -> Result<(), String> {
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
                .title("BrowserOS independent capture (fresh profile)")
                .inner_size(1280.0, 720.0)
                .build()?;
            // Register the exact private manager before dispatch. Windows' CDP
            // metrics callback checks this managed state before taking the image.
            // Managing native state does not install the tab IPC plugin.
            let manager = Arc::new(TabManager::new_isolated_capture(app.handle().clone(), HOST, root.clone()));
            if !app.manage(manager.clone()) {
                return Err(std::io::Error::other("CAPTURE_MANAGER_ALREADY_REGISTERED").into());
            }
            let root = root.clone();
            let url = url.clone();
            thread::Builder::new().name("browseros-isolated-render".into()).spawn(move || {
                match run_render(manager, root, url) {
                    Ok(()) => std::process::exit(0),
                    Err(_) => std::process::exit(2),
                }
            })?;
            Ok(())
        })
        .run(context).map_err(|_| "CAPTURE_RENDER_RUNTIME_FAILED".to_owned())?;
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
    } else if first == OsStr::new(CHILD_FLAG) {
        if args.len() != 1 { Err("CAPTURE_WORKER_ARGUMENTS_INVALID".into()) } else { run_child() }
    } else { return None; };
    Some(match result {
        Ok(()) => 0,
        Err(code) => { eprintln!("[isolated-capture] {code}"); 1 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn output_requires_bounded_png_dimensions() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&1280u32.to_be_bytes());
        png.extend_from_slice(&720u32.to_be_bytes());
        assert!(checked_png_header(&png).is_ok());
        assert!(checked_png(&png).is_err(), "IHDR alone is not a complete image");
        png[16..20].copy_from_slice(&20_000u32.to_be_bytes());
        assert!(checked_png_header(&png).is_err());
    }
}
