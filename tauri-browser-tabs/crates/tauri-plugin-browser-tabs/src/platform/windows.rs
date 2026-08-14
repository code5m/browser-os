//! Windows-specific implementation.
//!
//! On Windows, WebView2 handles child webview resizing through its own
//! composition system. Tauri's standard API is sufficient.

use crate::models::{LogicalRect, Result};
use tauri::Webview;

/// Windows doesn't require special handling.
#[allow(dead_code)]
pub fn ensure_size_allocated(_webview: &Webview, _rect: LogicalRect) -> Result<()> {
    Ok(())
}
