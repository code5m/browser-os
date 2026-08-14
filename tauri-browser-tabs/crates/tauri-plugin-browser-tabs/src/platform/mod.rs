//! Platform-specific implementations for child webview resizing.

use crate::models::{LogicalRect, Result};
use tauri::{LogicalPosition, LogicalSize, Runtime, Webview, WebviewBuilder, Window};

#[cfg(target_os = "linux")]
pub mod linux;

/// Ensure a webview's native (GTK) allocation matches its wry-level size.
///
/// This is primarily useful for webviews created outside the plugin — for
/// example the main UI webview added via `Window::add_child`, which can also
/// suffer from WebKitGTK's cached initial allocation (e.g. stuck at 400px).
/// No-op on non-Linux platforms.
pub fn ensure_native_layout<R: Runtime>(webview: &Webview<R>) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let size = webview.size()?;
        linux::ensure_physical_size(webview, size.width, size.height)?;
    }
    #[cfg(not(target_os = "linux"))]
    let _ = webview;
    Ok(())
}

/// Apply a logical rect to a child webview.
///
/// We use logical coordinates (CSS pixels) exclusively to avoid the
/// Physical/Logical mixing that causes subtle platform-specific bugs.
pub fn apply_rect<R: Runtime>(webview: &Webview<R>, rect: LogicalRect) -> Result<()> {
    if rect.is_empty() {
        return Err(crate::models::BrowserTabError::InvalidRect(rect));
    }

    let pos = LogicalPosition::new(rect.x, rect.y);
    let size = LogicalSize::new(rect.width, rect.height);

    webview.set_position(pos)?;
    webview.set_size(size)?;

    #[cfg(target_os = "linux")]
    linux::ensure_size_allocated(webview, rect)?;

    Ok(())
}

/// Create a child webview with the given rect.
///
/// The child is created directly at the target rect rather than at 1x1,
/// because WebKitGTK on Linux may cache the initial size allocation and
/// ignore later resize requests.
pub fn create_child_webview<R: Runtime>(
    window: &Window<R>,
    builder: WebviewBuilder<R>,
    rect: LogicalRect,
) -> Result<Webview<R>> {
    if rect.is_empty() {
        return Err(crate::models::BrowserTabError::InvalidRect(rect));
    }

    let pos = LogicalPosition::new(rect.x, rect.y);
    let size = LogicalSize::new(rect.width, rect.height);

    let webview = window.add_child(builder, pos, size)?;

    #[cfg(target_os = "linux")]
    linux::ensure_size_allocated(&webview, rect)?;

    Ok(webview)
}
