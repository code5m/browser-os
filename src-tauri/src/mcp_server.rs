#![cfg(feature = "mcp")]
//! M5-W10 A3：MCP stdio-prep 骨架（feature-gated，`--features mcp` 才编译）。
//! M5-W11 A3：dry-run 硬化——确定性 JSON-RPC 错误、有界输入/响应、稳定 `tools/list`
//! 模式、所有未绑定/未知能力 `tools/call` 显式 fail-closed。
//!
//! 形态：编译隔离的 stdio JSON-RPC 骨架，复用 `crate::mcp` 的注册表 / 策略接线
//! （`list_registry_entries` / `current_policy_snapshot`），**无 TCP 监听 / 无网络 /
//! 无 rmcp / 无 tokio / 无文件·数据库·脚本·插件执行副作用**。
//!
//! 设计约束（对齐 W7/W10/W11 Hard Stop 与 `scripts/check-mcp-policy.py` 的
//! MCP_SERVER_GATED 守门）：
//!   - 仅用 `std::io` 做 stdio 传输（`stdin` 逐行读 JSON-RPC，`stdout` 写响应），不碰 `std::net`；
//!   - 不 `spawn` 进程、不调用 `script_runner`、不碰 DB / 文件系统写 / 插件运行时；
//!   - 能力清单单一真源来自 `crate::mcp` 的注册表视图 `list_registry_entries`，只读 introspection 工具
//!     （`mcp_policy_get` / `mcp_registry_list`）直接复用 `crate::mcp` 视图函数零副作用返回；
//!   - 其余 registry 能力（file_read 等）仍是「prep」态——`tools/call` 时 fail-closed 返回
//!     not-yet-bound，不落地任何执行路径（M5-2.b 后续再绑定 core API）；
//!   - W11 硬化：单行输入 / 单条响应均有字节上限，越界回确定性错误而非无限分配或静默截断；
//!     所有错误码遵循 JSON-RPC 2.0，且**绝不回显请求参数 / 凭据 / token**。
//!
//! 真正的 rmcp server / 工具执行留待 M5-2.b，由 A0 显式解锁；届时本骨架的 transport 与
//! 守门逻辑可平滑升级（见 logs/assist/A3-M5-W10-*.md）。

use crate::mcp::{current_policy_snapshot, list_registry_entries};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

/// 单条 stdio 请求最大字节数（W11 硬化：防超大行撑爆内存 / 请求 smuggling）。
const MAX_INPUT_BYTES: usize = 1_048_576; // 1 MiB
/// 单条响应最大序列化字节数（W11 硬化：响应越界则回确定性错误，绝不截断业务数据）。
const MAX_RESPONSE_BYTES: usize = 4_194_304; // 4 MiB

/// 两个已绑定只读 introspection 工具（能力单一真源来自 `crate::mcp` 注册表）。
const BOUND_READONLY_TOOLS: &[&str] = &["mcp_policy_get", "mcp_registry_list"];

/// 单行读取结果：EOF，或一行数据（可能已被截断）。
enum ReadLine {
    Eof,
    Line { data: Vec<u8>, truncated: bool },
}

/// 从 `BufRead` 逐字节读一行，但累计字节超过 `max` 后继续读到换行 / EOF 并标记 `truncated`，
/// 保证内存有上限，且越界请求不致让读取循环卡死或无限分配。
fn read_line_bounded<R: BufRead>(reader: &mut R, max: usize) -> io::Result<ReadLine> {
    let mut buf: Vec<u8> = Vec::with_capacity(max.min(1024));
    let mut truncated = false;
    let mut started = false;
    let mut byte = [0u8; 1];
    loop {
        let n = reader.read(&mut byte)?;
        if n == 0 {
            if !started {
                return Ok(ReadLine::Eof);
            }
            if truncated {
                return Ok(ReadLine::Line {
                    data: buf,
                    truncated: true,
                });
            }
            if buf.last() == Some(&b'\r') {
                buf.pop();
            }
            return Ok(ReadLine::Line {
                data: buf,
                truncated: false,
            });
        }
        started = true;
        if byte[0] == b'\n' {
            if truncated {
                return Ok(ReadLine::Line {
                    data: buf,
                    truncated: true,
                });
            }
            if buf.last() == Some(&b'\r') {
                buf.pop();
            }
            return Ok(ReadLine::Line {
                data: buf,
                truncated: false,
            });
        }
        if buf.len() < max {
            buf.push(byte[0]);
        } else {
            truncated = true;
        }
    }
}

fn parse_error() -> Value {
    json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32700,"message":"Parse error"}})
}

fn invalid_request() -> Value {
    json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32600,"message":"Invalid Request"}})
}

fn request_too_large() -> Value {
    json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32600,"message":"Request too large"}})
}

/// 把响应序列化为有界字符串：超界 / 序列化失败都回确定性错误，绝不泄露业务 / 参数内容。
fn serialize_bounded(resp: &Value) -> String {
    match serde_json::to_string(resp) {
        Ok(s) if s.len() <= MAX_RESPONSE_BYTES => s,
        Ok(_) => {
            let id = resp.get("id").cloned().unwrap_or(Value::Null);
            serde_json::to_string(
                &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"Response exceeded size limit"}}),
            )
            .unwrap_or_default()
        }
        Err(_) => serde_json::to_string(
            &json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32603,"message":"Internal error"}}),
        )
        .unwrap_or_default(),
    }
}

fn write_response<W: Write>(out: &mut W, resp: &Value) {
    let _ = writeln!(out, "{}", serialize_bounded(resp));
    let _ = out.flush();
}

/// 处理单行输入：空行跳过（None）；越界回确定性错误；解析失败回 Parse error；否则分发。
fn handle_line(line: &str) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() > MAX_INPUT_BYTES {
        return Some(request_too_large());
    }
    let req = match serde_json::from_str::<Value>(trimmed) {
        Ok(r) => r,
        Err(_) => return Some(parse_error()),
    };
    Some(handle_request(&req))
}

/// stdio 服务环（可注入读写端，便于流级冒烟测试；生产入口见 `run_stdio`）。
/// 逐行读 JSON-RPC 请求 → 逐行写响应；EOF 或 IO 错误即返回，永不 panic 上抛。
fn serve<R: BufRead, W: Write>(reader: &mut R, out: &mut W) {
    loop {
        match read_line_bounded(reader, MAX_INPUT_BYTES) {
            Ok(ReadLine::Eof) => break,
            Ok(ReadLine::Line { data, truncated }) => {
                if truncated {
                    write_response(out, &request_too_large());
                    continue;
                }
                let line = String::from_utf8_lossy(&data);
                if let Some(resp) = handle_line(&line) {
                    // 通知（notifications/*）返回 Value::Null，按 JSON-RPC 规范不写响应。
                    if !resp.is_null() {
                        write_response(out, &resp);
                    }
                }
            }
            Err(_) => break,
        }
    }
}

/// 启动 stdio 循环：从 stdin 逐行读 JSON-RPC 请求，向 stdout 写 JSON-RPC 响应。
/// 仅用标准库 IO（stdio transport），无网络监听。读到 EOF 即返回。
pub fn run_stdio() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout().lock();
    serve(&mut reader, &mut stdout);
}

/// 单条 JSON-RPC 请求分发。
fn handle_request(req: &Value) -> Value {
    // 非对象请求（数组 / 标量）一律按 Invalid Request，确定性错误码。
    if !req.is_object() {
        return invalid_request();
    }
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

/// 稳定 `tools/list` 模式：每个工具都含 `name` / `description` / `inputSchema` 三段，
/// `inputSchema` 为合法 JSON Schema（object、无额外属性），不随运行时变化。
fn stable_input_schema() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

/// `tools/list`：复用 crate::mcp 注册表接线（能力单一真源），列出已声明能力。
/// 视图中标注每个能力是「已绑定只读」还是「prep 未绑定」（fail-closed 提示）。
fn tools_list(id: Value) -> Value {
    let entries = list_registry_entries();
    let tools: Vec<Value> = entries
        .iter()
        .map(|e| {
            let bound = BOUND_READONLY_TOOLS.contains(&e.capability.as_str());
            json!({
                "name": e.capability,
                "description": format!(
                    "[{}/{}] core_api={} (M5-2 prep: {})",
                    if e.touches_fs { "fs" } else { "ro" },
                    if e.returns_url { "url" } else { "no-url" },
                    e.core_api,
                    if bound { "bound-readonly" } else { "not-yet-bound" }
                ),
                "inputSchema": stable_input_schema()
            })
        })
        .collect();
    json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tools } })
}

/// 构造 fail-closed 文本响应（isError=true），用于未绑定 / 未知能力。
/// 仅回显能力名（协议标识符，非机密），**绝不**回显请求参数。
fn fail_closed_text(id: Value, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [ { "type": "text", "text": message.to_string() } ],
            "isError": true
        }
    })
}

/// `tools/call`：仅只读 introspection 工具复用 crate::mcp 视图函数返回（零副作用）；
/// 其余 registry 能力 fail-closed 返回 not-yet-bound，绝不落地任何执行路径。
/// 即便传入越权路径 / 参数，也只回 fail-closed，不执行、不调用 fs/db、不回显参数。
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
        // 缺 name：Invalid params，确定性错误码，不进能力分发。
        "" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32602, "message": "Invalid params: missing tool name" }
        }),
        // 已知但未绑定 / 完全未知：均 fail-closed（isError=true）。仅回显能力名，不回显参数。
        other => {
            let known = list_registry_entries()
                .iter()
                .any(|e| e.capability == other);
            let msg = if known {
                format!(
                    "{other} is declared in the MCP capability registry but not yet bound (M5-2.b pending); fail-closed, no execution"
                )
            } else {
                format!("unknown tool '{other}'; fail-closed, no execution")
            };
            fail_closed_text(id, &msg)
        }
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
    fn tools_list_schema_is_stable() {
        // 每个工具都必须具备 name / description / inputSchema，且 inputSchema 是合法 JSON Schema。
        let req = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}});
        let resp = handle_request(&req);
        let tools = resp["result"]["tools"].as_array().unwrap();
        for t in tools {
            assert!(t.get("name").is_some());
            assert!(t.get("description").is_some());
            let schema = t.get("inputSchema").unwrap();
            assert_eq!(schema["type"], "object");
            assert!(schema.get("properties").is_some());
            assert_eq!(schema["additionalProperties"], false);
        }
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
    fn known_unbound_capability_is_fail_closed() {
        // 即便传入越权路径，prep 态也 fail-closed：不执行、不调用 fs/db、返回 isError=true。
        let req = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"file_read","arguments":{"path":"/etc/passwd"}}});
        let resp = handle_request(&req);
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("not yet bound"));
    }

    #[test]
    fn unknown_tool_is_fail_closed() {
        // 完全不在注册表的工具名也必须 fail-closed。
        let req = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"pwn_all_the_things","arguments":{}}});
        let resp = handle_request(&req);
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown tool"));
    }

    #[test]
    fn arguments_are_never_echoed_in_fail_closed() {
        // W11 Hard Stop：响应 / 日志 / 审计里不得回显原始参数（可能含 URL/token/query）。
        let req = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"file_read","arguments":{"path":"SECRET_TOKEN_DO_NOT_ECHO_xyz"}}});
        let resp = handle_request(&req);
        let text = serde_json::to_string(&resp).unwrap();
        assert!(!text.contains("SECRET_TOKEN_DO_NOT_ECHO_xyz"));
    }

    #[test]
    fn missing_tool_name_is_invalid_params() {
        let req = serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{}});
        let resp = handle_request(&req);
        assert_eq!(resp["error"]["code"], -32602);
    }

    #[test]
    fn non_object_request_is_invalid_request() {
        // 数组 / 标量等非对象请求 → Invalid Request（-32600），确定性。
        let arr = serde_json::json!([1, 2, 3]);
        let resp = handle_request(&arr);
        assert_eq!(resp["error"]["code"], -32600);
        let num = serde_json::json!(42);
        let resp2 = handle_request(&num);
        assert_eq!(resp2["error"]["code"], -32600);
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

    #[test]
    fn handle_line_invalid_json_returns_parse_error() {
        let resp = handle_line("not json at all {{{").unwrap();
        assert_eq!(resp["error"]["code"], -32700);
        assert_eq!(resp["id"], Value::Null);
    }

    #[test]
    fn handle_line_empty_returns_none() {
        assert!(handle_line("   ").is_none());
        assert!(handle_line("").is_none());
    }

    #[test]
    fn handle_line_too_large_returns_error() {
        let big = "x".repeat(MAX_INPUT_BYTES + 1);
        let resp = handle_line(&big).unwrap();
        assert_eq!(resp["error"]["code"], -32600);
        assert_eq!(resp["error"]["message"], "Request too large");
    }

    #[test]
    fn read_line_bounded_keeps_short_lines() {
        let data = b"hello\nworld\n".to_vec();
        let mut cur = std::io::Cursor::new(data);
        match read_line_bounded(&mut cur, 1024).unwrap() {
            ReadLine::Line {
                ref data,
                truncated,
            } => {
                assert!(!truncated);
                assert_eq!(data.as_slice(), b"hello");
            }
            _ => panic!("expected line"),
        }
        match read_line_bounded(&mut cur, 1024).unwrap() {
            ReadLine::Line {
                ref data,
                truncated,
            } => {
                assert!(!truncated);
                assert_eq!(data.as_slice(), b"world");
            }
            _ => panic!("expected line"),
        }
        assert!(matches!(
            read_line_bounded(&mut cur, 1024).unwrap(),
            ReadLine::Eof
        ));
    }

    #[test]
    fn read_line_bounded_marks_truncated_and_drains() {
        // 无换行的超长行：截断标记 + 只保留前 max 字节；后续读取到 EOF（drain 完毕）。
        let big = format!("{}x", "a".repeat(2000));
        let mut cur = std::io::Cursor::new(big.into_bytes());
        match read_line_bounded(&mut cur, 100).unwrap() {
            ReadLine::Line { data, truncated } => {
                assert!(truncated);
                assert_eq!(data.len(), 100);
            }
            _ => panic!("expected truncated line"),
        }
        assert!(matches!(
            read_line_bounded(&mut cur, 100).unwrap(),
            ReadLine::Eof
        ));
    }

    /// 流级冒烟辅助：把一整段 stdio 输入喂进 `serve`，返回全部 stdout 输出。
    fn serve_bytes(input: &[u8]) -> String {
        let mut reader = std::io::Cursor::new(input.to_vec());
        let mut out: Vec<u8> = Vec::new();
        serve(&mut reader, &mut out);
        String::from_utf8_lossy(&out).to_string()
    }

    #[test]
    fn stdio_smoke_invalid_json_unknown_tool_and_large_line() {
        // 端到端 dry-run 冒烟：坏 JSON / 未绑定能力 + 机密参数 / 超大行 / 通知 / 恢复。
        let secret = "SECRET_ARG_DO_NOT_ECHO_zzz";
        let unknown = format!(
            r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"file_read","arguments":{{"token":"{secret}"}}}}}}"#
        );
        let notification = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;
        let ping = r#"{"jsonrpc":"2.0","id":8,"method":"ping","params":{}}"#;
        let big = "a".repeat(MAX_INPUT_BYTES + 1);
        let input = format!("not json {{{{\n{unknown}\n{notification}\n{big}\n{ping}\n");
        let out = serve_bytes(input.as_bytes());
        let lines: Vec<&str> = out.lines().collect();
        // 通知不产生响应：5 条输入 → 4 条输出；超大行被 drain 后循环仍不脱位。
        assert_eq!(lines.len(), 4, "unexpected stdio output: {out}");
        assert!(lines[0].contains("-32700"), "invalid JSON → Parse error");
        assert!(
            lines[1].contains("not yet bound"),
            "unbound capability → fail-closed"
        );
        assert!(lines[2].contains("Request too large"), "oversized line");
        assert!(lines[3].contains("\"id\":8"), "loop recovers after drain");
        // W11 Hard Stop：整条 stdio 流不得回显原始参数 / 超大行 payload。
        assert!(!out.contains(secret));
        assert!(!out.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn stdio_smoke_oversized_line_never_echoes_payload() {
        // 超大请求行即便以合法 JSON 开头（内含机密），也只回确定性错误，绝不回显 payload。
        let secret = "TOKEN_ABCDEFGH_0123456789";
        let mut big = format!(
            r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"file_read","arguments":{{"token":"{secret}"}}}}}}"#
        );
        while big.len() <= MAX_INPUT_BYTES {
            big.push('p');
        }
        let out = serve_bytes(big.as_bytes());
        assert!(out.contains("Request too large"));
        assert!(!out.contains(secret));
    }

    #[test]
    fn stdio_smoke_empty_input_and_eof_only() {
        // 空流 / 仅空行：不产生任何输出，正常 EOF 退出。
        assert_eq!(serve_bytes(b""), "");
        assert_eq!(serve_bytes(b"\n\n   \n"), "");
    }

    #[test]
    fn response_size_guard_caps_oversized() {
        // 合成超界响应：serialize_bounded 必须回确定性错误而非返回超长串。
        let huge =
            json!({"jsonrpc":"2.0","id":1,"result":{"x":"a".repeat(MAX_RESPONSE_BYTES * 3)}});
        let s = serialize_bounded(&huge);
        assert!(s.contains("Response exceeded size limit"));
        assert!(s.len() <= MAX_RESPONSE_BYTES + 256);
    }
}
