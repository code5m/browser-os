//! Native clipboard adapter — `SHARED_NATIVE_INFRASTRUCTURE`.
//!
//! 系统剪贴板读写。无 `AppState` 依赖、无资源生命周期、无跨能力副作用，
//! 与 `images.rs` 同属 `shared/` 共享原生基座（见 Native Physical Boundary Matrix §2.3）。
//! 命令体从 `bridge.rs` 迁出，经 `main.rs` 的 `pub use crate::shared::clipboard;`
//! 在 crate 根再导出，`generate_handler!` 直接以 `clipboard::clipboard_read` /
//! `clipboard::clipboard_write` 注册（不再经 `bridge::` facade）。

use std::sync::{Mutex, OnceLock};

// arboard releases X11 CLIPBOARD ownership when its last Clipboard handle is
// dropped. An IPC-local handle disappears after clipboard_write() returns,
// leaving subsequent reads empty on systems without a clipboard manager
// (including isolated Xvfb). Keep one owner alive for the app lifetime.
// A single mutex also serializes simultaneous native clipboard requests.
static NATIVE_CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();

fn with_clipboard<T>(
    operation: impl FnOnce(&mut arboard::Clipboard) -> Result<T, String>,
) -> Result<T, String> {
    let mut owner = NATIVE_CLIPBOARD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "剪贴板状态锁不可用".to_string())?;
    if owner.is_none() {
        *owner = Some(arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?);
    }
    operation(owner.as_mut().expect("clipboard just initialized"))
}

/// 读取系统剪贴板文本。
#[tauri::command]
pub fn clipboard_read(webview: tauri::Webview) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "clipboard_read")?;

    with_clipboard(|cb| cb.get_text().map_err(|e| format!("读取失败: {e}")))
}

/// 写入系统剪贴板文本。
#[tauri::command]
pub fn clipboard_write(webview: tauri::Webview, text: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "clipboard_write")?;
    crate::shared::native_legacy_policy::bound_text("clipboard_write", "text", &text)?;

    with_clipboard(|cb| cb.set_text(text).map_err(|e| format!("写入失败: {e}")))
}
