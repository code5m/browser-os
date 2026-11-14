//! Windows fail-closed MCP context until XP-012/017 supplies a verified
//! Windows Job Object and ACL-scoped pipe sandbox. No untrusted MCP process
//! may run through this API with only a path-canonicalization check.
use serde_json::Value;
use std::path::Path;
pub fn execute(_root: &Path, _storage: &Path, _request: Value) -> Result<Value, String> {
    Err("MCP_CONTEXT_PLATFORM_UNSUPPORTED".into())
}
