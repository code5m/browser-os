#![cfg(feature = "mcp")]
//! M5-W10 A3：MCP stdio-prep 骨架（feature-gated，`--features mcp` 才编译）。
//!
//! 形态：编译隔离的 stdio JSON-RPC 骨架，复用 `crate::mcp` 的注册表 / 策略接线
//! （`list_registry_entries` / `current_policy_snapshot`），**无 TCP 监听 / 无网络 /
//! 无 rmcp / 无 tokio / 无文件·数据库·脚本·插件执行副作用**。
//!
//! 设计约束（对齐 W7/W10 Hard Stop 与 `scripts/check-mcp-policy.py` 的 MCP_SERVER_GATED 守门）：
//!   - 仅用 `std::io` 做 stdio 传输（`stdin` 逐行读 JSON-RPC，`stdout` 写响应），不碰 `std::net`；
//!   - 不 `spawn` 进程、不调用 `script_runner`、不碰 DB / 文件系统写 / 插件运行时；
//!   - 能力清单单一真源来自 `crate::mcp` 的注册表视图 `list_registry_entries`，只读 introspection 工具
//!     （`mcp_policy_get` / `mcp_registry_list`）直接复用 `crate::mcp` 视图函数零副作用返回；
//!   - 其余 registry 能力（file_read 等）仍是「prep」态——`tools/call` 时 fail-closed 返回
//!     not-yet-bound，不落地任何执行路径（M5-2.b 后续再绑定 core API）。
//!
//! 真正的 rmcp server / 工具执行留待 M5-2.b，由 A0 显式解锁；届时本骨架的 transport 与
//! 守门逻辑可平滑升级（见 logs/assist/A3-M5-W10-*.md）。

use crate::mcp::{current_policy_snapshot, list_registry_entries};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

/// 启动 stdio 循环：从 stdin 逐行读 JSON-RPC 请求，向 stdout 写 JSON-RPC 响应。
/// 仅用标准库 IO（stdio transport），无网络监听。读到 EOF 即返回。
pub fn run_stdio() {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let resp: Value = match serde_json::from_str::<Value>(trimmed) {
            Ok(req) => handle_request(&req),
            Err(_) => json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": "Parse error" }
            }),
        };
        if let Ok(s) = serde_json::to_string(&resp) {
            let _ = writeln!(stdout, "{s}");
            let _ = stdout.flush();
        }
    }
}

/// 单条 JSON-RPC 请求分发。
fn handle_request(req: &Value) -> Value {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "mvp-browser-os-mcp", "version": env!("CARGO_PKG_VERSION") }
            }
        }),
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        "tools/list" => tools_list(id),
        "tools/call" => tools_call(id, &params),
        // 通知（notifications/*）无响应。
        m if m.starts_with("notifications/") => Value::Null,
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": "Method not found" }
        }),
    }
}

/// `tools/list`：复用 crate::mcp 注册表接线（能力单一真源），列出已声明能力。
/// 视图中标注每个能力是「已绑定只读」还是「prep 未绑定」（fail-closed 提示）。
fn tools_list(id: Value) -> Value {
    let entries = list_registry_entries();
    let tools: Vec<Value> = entries
        .iter()
        .map(|e| {
            let bound = matches!(
                e.capability.as_str(),
                "mcp_policy_get" | "mcp_registry_list"
            );
            json!({
                "name": e.capability,
                "description": format!(
                    "[{}/{}] core_api={} (W10 prep: {})",
                    if e.touches_fs { "fs" } else { "ro" },
                    if e.returns_url { "url" } else { "no-url" },
                    e.core_api,
                    if bound { "bound-readonly" } else { "not-yet-bound" }
                ),
                "inputSchema": { "type": "object", "properties": {} }
            })
        })
        .collect();
    json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tools } })
}

/// `tools/call`：仅只读 introspection 工具复用 crate::mcp 视图函数返回（零副作用）；
/// 其余 registry 能力 fail-closed 返回 not-yet-bound，绝不落地任何执行路径。
fn tools_call(id: Value, params: &Value) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    match name {
        "mcp_policy_get" => {
            let snap = current_policy_snapshot();
            json!({ "jsonrpc": "2.0", "id": id, "result": {
                "content": [ { "type": "text", "text": serde_json::to_string(&snap).unwrap_or_default() } ],
                "isError": false
            }})
        }
        "mcp_registry_list" => {
            let entries = list_registry_entries();
            json!({ "jsonrpc": "2.0", "id": id, "result": {
                "content": [ { "type": "text", "text": serde_json::to_string(&entries).unwrap_or_default() } ],
                "isError": false
            }})
        }
        other => json!({ "jsonrpc": "2.0", "id": id, "result": {
            "content": [ { "type": "text", "text": format!(
                "capability '{other}' is declared in the MCP capability registry but not yet bound (M5-2.b pending); fail-closed, no execution" ) } ],
            "isError": true
        }}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_server_info() {
        let req = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
        let resp = handle_request(&req);
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["serverInfo"]["name"], "mvp-browser-os-mcp");
        assert_eq!(resp["result"]["capabilities"]["tools"], json!({}));
    }

    #[test]
    fn tools_list_uses_registry_source_of_truth() {
        let req = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}});
        let resp = handle_request(&req);
        let tools = resp["result"]["tools"].as_array().unwrap();
        // 与 crate::mcp::list_registry_entries() 数量一致（能力单一真源，无漂移）。
        assert_eq!(tools.len(), list_registry_entries().len());
        assert!(!tools.is_empty());
    }

    #[test]
    fn policy_get_is_readonly_no_error() {
        let req = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"mcp_policy_get","arguments":{}}});
        let resp = handle_request(&req);
        assert_eq!(resp["result"]["isError"], false);
        assert!(resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("MCP_CAPABILITY_V1"));
    }

    #[test]
    fn registry_list_is_readonly_no_error() {
        let req = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"mcp_registry_list","arguments":{}}});
        let resp = handle_request(&req);
        assert_eq!(resp["result"]["isError"], false);
    }

    #[test]
    fn unbound_capability_is_fail_closed() {
        // 即便传入越权路径，prep 态也 fail-closed：不执行、不调用 fs/db、返回 isError=true。
        let req = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"file_read","arguments":{"path":"/etc/passwd"}}});
        let resp = handle_request(&req);
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("not yet bound"));
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let req = serde_json::json!({"jsonrpc":"2.0","id":5,"method":"foobar","params":{}});
        let resp = handle_request(&req);
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn notifications_have_no_response() {
        let req =
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}});
        let resp = handle_request(&req);
        assert!(resp.is_null());
    }
}
