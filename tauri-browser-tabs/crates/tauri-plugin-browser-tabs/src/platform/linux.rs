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
/// 【规范锁定，禁止改动，详见项目根 PROJET-RULES.md 与 COMPONENT.md】
/// 必须同时做两件事，缺一不可，违反即网页撑不满 / 交互失效：
/// 1) `set_size_request(w, h)`：GtkFixed 布局循环按 size_request 分配子控件
///    尺寸，wry 的 set_bounds 只直接 size_allocate 不更新 size_request，缺这行
///    会导致窗口 resize / 重布局时子 webview 被打回旧尺寸（1x1 / 400px）。
/// 2) `size_allocate` 后 `queue_resize()`：直接 size_allocate 只改 widget
///    allocation，WebKitGTK 的 CSS 视口刷新依赖完整 GTK 布局迭代。
/// 这两步共同保证：立即分配与布局循环两条路径结果一致 + CSS 视口正确刷新。
fn force_allocation(gtk_webview: &webkit2gtk::WebView, expected_width: i32, expected_height: i32) {
    use gtk::prelude::*;

    // 1) GtkFixed 的布局循环按子控件的 size_request 分配尺寸。wry 的
    //    set_bounds 只直接 size_allocate、从不更新 size_request，导致窗口
    //    resize / GTK 重跑布局时子 webview 被打回旧尺寸（如 1x1 或 400px）。
    //    这里补上 size_request，让两条分配路径（立即 + 布局循环）结果一致。
    if gtk_webview.width_request() != expected_width
        || gtk_webview.height_request() != expected_height
    {
        gtk_webview.set_size_request(expected_width, expected_height);
    }

    let allocation = gtk_webview.allocation();

    if allocation.width() != expected_width || allocation.height() != expected_height {
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

        // 2) 排队一轮完整布局：直接 size_allocate 只改 widget allocation，
        //    WebKitGTK 的 CSS 视口刷新依赖完整的 GTK 布局迭代
        //    （size-allocate 信号链 → WebKitWebViewBase 更新 viewportSize）。
        gtk_webview.queue_resize();
    }
}
