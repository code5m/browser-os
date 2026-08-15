//! Linux-specific fixes for WebKitGTK child webview resizing.
//!
//! WebKitGTK on Linux has a known behavior where `set_size` on a child
//! webview may not trigger a proper `size_allocate` on the underlying
//! `WebKitWebView` widget. This module forces the allocation.

use crate::models::{LogicalRect, Result};
use tauri::{Runtime, Webview};

/// Ensure the underlying WebKitGTK widget is properly size-allocated,
/// given a rect in logical (CSS) pixels.
///
/// Works around child webviews getting "stuck" at an initial size
/// (e.g. 400px) and ignoring subsequent `set_size` calls.
pub fn ensure_size_allocated<R: Runtime>(webview: &Webview<R>, rect: LogicalRect) -> Result<()> {
    // Convert logical pixels to physical using the GTK widget's own scale factor.
    webview
        .with_webview(move |platform_webview| {
            use gtk::prelude::*;

            // On Linux the inner handle is a webkit2gtk::WebView (a GTK widget).
            let gtk_webview = platform_webview.inner();
            let scale = gtk_webview.scale_factor() as f64;
            let expected_width = (rect.width * scale).round() as i32;
            let expected_height = (rect.height * scale).round() as i32;
            force_allocation(&gtk_webview, expected_width, expected_height);
        })
        .map_err(|e| crate::models::BrowserTabError::Platform(e.to_string()))?;

    Ok(())
}

/// Ensure the underlying WebKitGTK widget matches a physical pixel size.
///
/// Useful for webviews whose size is already known in physical pixels
/// (e.g. the main UI webview created from `window.inner_size()`).
pub fn ensure_physical_size<R: Runtime>(
    webview: &Webview<R>,
    width: u32,
    height: u32,
) -> Result<()> {
    webview
        .with_webview(move |platform_webview| {
            let gtk_webview = platform_webview.inner();
            force_allocation(&gtk_webview, width as i32, height as i32);
        })
        .map_err(|e| crate::models::BrowserTabError::Platform(e.to_string()))?;

    Ok(())
}

/// Force the GTK allocation if it doesn't match the expected physical size.
///
/// 关键约束（血泪教训）：
/// - **绝不调用 `set_size_request` / `queue_resize`**。GtkFixed 子控件的
///   `size_request` 一旦被我们设成固定值，会与父容器布局循环互相覆盖，
///   形成「(1200x800)->(1200x665)」的无限 force 死循环，拖死主线程事件循环，
///   表现为网页「点了没反应 / 前进后退失效」。
/// - wry 0.55 的 `set_size()` 已经正确同时更新物理 `size_allocate` 与
///   `size_request`，WebKitGTK 的 CSS 视口会随之刷新，无需我们额外干预。
/// - 这里只在「初始 allocation 仍是错的（如 1x1 / 400px）」这种极端情况下，
///   补一次直接的 `size_allocate` + `queue_draw`，且立刻返回，绝不重入布局。
fn force_allocation(gtk_webview: &webkit2gtk::WebView, expected_width: i32, expected_height: i32) {
    use gtk::prelude::*;

    let allocation = gtk_webview.allocation();

    // 只在明显错配（初始尺寸没生效）时补一次；之后由 wry.set_size 维持，不再打扰。
    let mismatched = allocation.width() != expected_width
        || allocation.height() != expected_height;
    let is_initial_stuck = allocation.width() <= 1 || allocation.height() <= 1;

    if mismatched && (is_initial_stuck || allocation.width() <= 400 || allocation.height() <= 400) {
        log::debug!(
            "browser-tabs: forcing size_allocate current=({}x{}) expected=({}x{})",
            allocation.width(),
            allocation.height(),
            expected_width,
            expected_height
        );
        eprintln!(
            "[browser-tabs] force size_allocate: ({:?}x{:?}) -> ({}x{})",
            allocation.width(),
            allocation.height(),
            expected_width,
            expected_height
        );

        let new_allocation =
            gtk::Allocation::new(allocation.x(), allocation.y(), expected_width, expected_height);
        gtk_webview.size_allocate(&new_allocation);
        gtk_webview.queue_draw();
    }
}
