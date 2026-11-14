//! Windows LSP sandbox is unavailable until XP-017 restricted token/AppContainer
//! isolation is proven. Explicitly deny every native source/code execution.
use std::path::Path;
use serde_json::{json, Value};
pub fn execute(_root: &Path, request: Value) -> Result<Value, String> {
    if request["action"] == "lsp_status" {
        Ok(json!({"profiles":[],"sandbox_available":false,"platform":"windows"}))
    } else if request["action"] == "lsp_close" {
        Ok(json!({"closed_sessions":0}))
    } else {
        Err("LSP_PLATFORM_UNSUPPORTED".into())
    }
}
