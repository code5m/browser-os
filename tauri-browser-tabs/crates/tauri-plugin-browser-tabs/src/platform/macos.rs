//! macOS-specific implementation.
//!
//! On macOS, Tauri's standard `set_position`/`set_size` work reliably
//! for child webviews. No additional fixes are needed.

use crate::models::{LogicalRect, Result};
use tauri::Webview;

/// macOS doesn't require special handling.
#[allow(dead_code)]
pub fn ensure_size_allocated(_webview: &Webview, _rect: LogicalRect) -> Result<()> {
    Ok(())
}
