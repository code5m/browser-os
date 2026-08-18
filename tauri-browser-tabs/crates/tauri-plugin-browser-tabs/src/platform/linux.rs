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
    webview
        .with_webview(move |platform_webview| {
            use gtk::prelude::*;

            let gtk_webview = platform_webview.inner();
            let scale = gtk_webview.scale_factor() as f64;
            let w = (rect.width * scale).round() as i32;
            let h = (rect.height * scale).round() as i32;
            let x = (rect.x * scale).round() as i32;
            let y = (rect.y * scale).round() as i32;

            // 仅在 allocation 与目标错配时才纠正（move + size_allocate）。
            // 关键：GTK 布局循环会在页面加载/容器重排后把子 webview 的 allocation
            // 拉回"自然位置"（实测漂移到 (0,400,1200,400) 下半屏），因此 bridge 侧有
            // 布局守护线程每 400ms 按记忆矩形重放本函数。错配才纠正让守护线程空转代价
            // 趋近于零（不触发无谓的 size-allocate 信号与 WebKit 重排）。
            let a = gtk_webview.allocation();
            if a.x() != x || a.y() != y || a.width() != w || a.height() != h {
                // 日志去重：守护线程每 400ms 重放，对"纠正后不回落"的隐藏 webview
                // 同一签名会无限刷屏。只在签名变化时输出（新的漂移仍会看到）。
                use std::sync::{Mutex, OnceLock};
                static LAST_SIG: OnceLock<Mutex<Option<(i32, i32, i32, i32, i32, i32, i32, i32)>>> =
                    OnceLock::new();
                let sig = (x, y, w, h, a.x(), a.y(), a.width(), a.height());
                let mut last = LAST_SIG.get_or_init(|| Mutex::new(None)).lock().unwrap();
                if *last != Some(sig) {
                    eprintln!(
                        "[browser-tabs] layout drift-correct target=({},{},{}x{}) alloc_before=({},{},{}x{})",
                        x, y, w, h, a.x(), a.y(), a.width(), a.height()
                    );
                    *last = Some(sig);
                }
                drop(last);
                // 血泪教训（详见 PROJECT-RULES.md）：
                // GtkFixed 子控件必须【位置 + 尺寸一起定死】，且【绝不 queue_resize】。
                // 1) gtk_fixed_move 固定位置 —— wry 的 set_bounds/set_position 对 GtkFixed
                //    子控件只 size_allocate 不 move，位置会漂。
                // 2) size_allocate 固定尺寸 —— 触发 size-allocate 信号让 WebKit 刷新视口。
                // 3) 不 set_size_request、不 queue_resize —— 二者都会触发 GtkFixed 重算，
                //    按"剩余空间"把子控件拉满全窗口。
                if let Some(parent) = gtk_webview.parent() {
                    if let Ok(fixed) = parent.downcast::<gtk::Fixed>() {
                        fixed.move_(&gtk_webview, x, y);
                    }
                }
                gtk_webview.size_allocate(&gtk::Allocation::new(x, y, w, h));
                gtk_webview.queue_draw();
            }
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
            use gtk::prelude::*;
            // 主 UI webview（非 GtkFixed 子控件）：仅当 allocation 错配时补一次
            // size_allocate 刷新 CSS 视口。不 queue_resize（避免无谓的布局重算）。
            let gtk_webview = platform_webview.inner();
            let a = gtk_webview.allocation();
            if a.width() != width as i32 || a.height() != height as i32 {
                gtk_webview.size_allocate(&gtk::Allocation::new(a.x(), a.y(), width as i32, height as i32));
                gtk_webview.queue_draw();
            }
        })
        .map_err(|e| crate::models::BrowserTabError::Platform(e.to_string()))?;

    Ok(())
}
