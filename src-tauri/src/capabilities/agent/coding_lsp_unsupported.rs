//! Explicit boundary for platforms without an approved native LSP adapter.
//! Compilation of shared code does not imply sandbox or language-server support.
use serde_json::{json, Value};
use std::path::Path;

pub fn execute(_root: &Path, request: Value) -> Result<Value, String> {
    if request["action"] == "lsp_status" {
        Ok(json!({
            "profiles": [],
            "sandbox_available": false,
            "supported": false,
            "platform": std::env::consts::OS,
            "reason": "LSP_PLATFORM_UNSUPPORTED"
        }))
    } else if request["action"] == "lsp_close" {
        // No native session can have been created by this adapter.
        Ok(json!({"closed_sessions": 0}))
    } else {
        Err("LSP_PLATFORM_UNSUPPORTED".into())
    }
}
