//! Explicit boundary for platforms without an approved native Context/MCP adapter.
//! A third platform must not accidentally select Windows-only code or execute
//! the Linux sandbox/toolchain path. Add a native adapter before enabling it.
use serde_json::{json, Value};
use std::path::Path;

pub fn execute(_root: &Path, _storage: &Path, request: Value) -> Result<Value, String> {
    if request["action"] == "context_status" {
        return Ok(json!({
            "platform": std::env::consts::OS,
            "supported": false,
            "sandbox_available": false,
            "reason": "CONTEXT_PLATFORM_UNSUPPORTED"
        }));
    }
    Err(format!(
        "CONTEXT_PLATFORM_UNSUPPORTED:{}",
        std::env::consts::OS
    ))
}

pub(crate) fn execute_cancelable(
    root: &Path,
    storage: &Path,
    request: Value,
    _cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<Value, String> {
    execute(root, storage, request)
}

/// Unsupported platforms cannot admit any external execution.
pub(super) fn execute_cancelable_with_admission(
    root: &Path,
    storage: &Path,
    request: Value,
    cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
    _before_spawn: impl FnOnce(&Value) -> Result<(), String>,
) -> Result<Value, String> {
    execute_cancelable(root, storage, request, cancellation)
}
