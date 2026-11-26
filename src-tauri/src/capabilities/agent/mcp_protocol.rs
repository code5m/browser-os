//! Shared MCP protocol and output boundary. Native process ownership,
//! pipe I/O, termination, and Windows sandbox enforcement stay in their
//! platform adapters. No provider-side effect is inferred from a frame.
use serde_json::{json, Value};

pub(super) const VERSION: &str = "2024-11-05";

pub(super) fn initialize_params() -> Value {
    json!({"protocolVersion":VERSION,"capabilities":{},"clientInfo":{"name":"BrowserOS","version":"0.1.0"}})
}
pub(super) fn validate_initialize(value: &Value) -> Result<(), String> {
    if value["protocolVersion"].as_str() != Some(VERSION) {
        return Err("MCP_PROTOCOL_VERSION_UNSUPPORTED".into());
    }
    if !value["capabilities"]["tools"].is_object()
        || !value["serverInfo"]["name"].as_str().is_some_and(|v| !v.is_empty())
        || !value["serverInfo"]["version"].as_str().is_some_and(|v| !v.is_empty()) {
        return Err("MCP_INITIALIZE_INVALID".into());
    }
    Ok(())
}

pub(super) fn tool_request(action: &str, request: &Value) -> Result<(&'static str, Value), String> {
    match action {
        "mcp_tools" => Ok(("tools/list", json!({}))),
        "mcp_call" => {
            let tool = request["tool"].as_str().filter(|v| !v.is_empty() && v.len() <= 200)
                .ok_or("MCP_TOOL_REQUIRED")?;
            let arguments = request.get("arguments").cloned().unwrap_or_else(|| json!({}));
            if !arguments.is_object() { return Err("MCP_ARGUMENTS_INVALID".into()); }
            Ok(("tools/call", json!({"name":tool,"arguments":arguments})))
        }
        _ => Err("MCP_ACTION_UNKNOWN".into()),
    }
}

pub(super) enum RpcFrame { Reply(Value), ServerRequest(Value), Ignore }

/// JSON-RPC uses independent ID spaces in each direction. A server
/// request whose ID equals the pending client request is NOT its reply.
pub(super) fn classify_frame(value: Value, pending: u64) -> Result<RpcFrame, String> {
    let object = value.as_object().ok_or("MCP_PROTOCOL_FRAME_INVALID")?;
    if value["jsonrpc"] != "2.0" { return Err("MCP_PROTOCOL_FRAME_INVALID".into()); }
    if let Some(method) = object.get("method") {
        if !method.as_str().is_some_and(|v| !v.is_empty())
            || object.contains_key("result") || object.contains_key("error") {
            return Err("MCP_PROTOCOL_FRAME_INVALID".into());
        }
        return match object.get("id") {
            None => Ok(RpcFrame::Ignore),
            Some(id) if id.is_string() || id.as_i64().is_some() || id.as_u64().is_some() => {
                Ok(RpcFrame::ServerRequest(json!({"jsonrpc":"2.0","id":id,
                    "error":{"code":-32601,"message":"Client requests unsupported"}})))
            }
            _ => Err("MCP_PROTOCOL_FRAME_INVALID".into()),
        };
    }
    if !object.contains_key("id") { return Err("MCP_PROTOCOL_FRAME_INVALID".into()); }
    if value["id"] != json!(pending) { return Ok(RpcFrame::Ignore); }
    match (object.get("result"), object.get("error")) {
        (Some(result), None) => Ok(RpcFrame::Reply(result.clone())),
        (None, Some(error)) if error.is_object() => Err("MCP_REMOTE_ERROR".into()),
        (None, None) => Err("MCP_RESULT_MISSING".into()),
        _ => Err("MCP_PROTOCOL_FRAME_INVALID".into()),
    }
}

pub(super) fn sensitive(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["api_key", "apikey", "password", "authorization:", "bearer ",
     "private key", "sk-", "access_token", "secret="]
        .iter().any(|s| lower.contains(s))
}
pub(super) fn safe_text(text: &str) -> String {
    text.lines().map(|line| if sensitive(line) {
        "[credential-bearing line omitted]"
    } else { line }).collect::<Vec<_>>().join("\n")
}
pub(super) fn scrub(value: &mut Value, secrets: &[String]) {
    match value {
        Value::String(text) => {
            // Redact before splitting lines: a multiline opaque credential
            // must not survive newline normalization.
            for secret in secrets.iter().filter(|s| !s.is_empty()) {
                *text = text.replace(secret, "[redacted]");
            }
            *text = safe_text(text);
        }
        Value::Array(items) => {
            for item in items { scrub(item, secrets); }
        }
        Value::Object(items) => {
            let previous = std::mem::take(items);
            let reserved: std::collections::BTreeSet<_> = previous.keys().cloned().collect();
            let mut next = 0usize;
            for (name, mut item) in previous {
                scrub(&mut item, secrets);
                let sensitive_name = sensitive(&name) || secrets.iter().any(|s| !s.is_empty() && name.contains(s));
                let safe_name = if sensitive_name {
                    loop {
                        let candidate = format!("[redacted-key-{next}]");
                        next += 1;
                        if !reserved.contains(&candidate) && !items.contains_key(&candidate)
                            && !secrets.iter().any(|s| !s.is_empty() && candidate.contains(s)) {
                            break Some(candidate);
                        }
                        // Never search without bound for a safe placeholder.
                        if next > reserved.len().saturating_mul(2).saturating_add(64) {
                            break None;
                        }
                    }
                } else { Some(name) };
                if let Some(key) = safe_name { items.insert(key, item); }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_rpc_id_spaces_do_not_turn_requests_into_replies() {
        let request = json!({"jsonrpc":"2.0","id":2,"method":"sampling/createMessage","params":{}});
        match classify_frame(request, 2).unwrap() {
            RpcFrame::ServerRequest(reply) => assert_eq!(reply["error"]["code"], -32601),
            _ => panic!("server request was confused with a reply"),
        }
        assert!(matches!(classify_frame(json!({"jsonrpc":"2.0","id":1,"result":{}}), 2).unwrap(), RpcFrame::Ignore));
        assert!(matches!(classify_frame(json!({"jsonrpc":"2.0","id":2,"result":{"content":[]}}), 2).unwrap(), RpcFrame::Reply(_)));
        for invalid in [json!({"id":2,"result":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","result":{}}),
            json!({"jsonrpc":"2.0","id":2,"result":{},"error":{"code":1}})] {
            assert!(classify_frame(invalid,2).is_err());
        }
    }

    #[test]
    fn handshake_and_request_require_negotiated_protocol_and_shape() {
        assert!(validate_initialize(&json!({"protocolVersion":VERSION,"capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}})).is_ok());
        assert!(validate_initialize(&json!({"protocolVersion":"future","capabilities":{"tools":{}}})).is_err());
        assert!(validate_initialize(&json!({"protocolVersion":VERSION,"capabilities":{},"serverInfo":{"name":"fixture","version":"1"}})).is_err());
        assert!(tool_request("mcp_call",&json!({"tool":"echo","arguments":[]})).is_err());
        assert_eq!(tool_request("mcp_tools",&json!({})).unwrap().0,"tools/list");
    }

    #[test]
    fn redaction_does_not_overwrite_ordinary_keys_or_leak_multiline_value() {
        let mut value = json!({"password":"v","[redacted-key-0]":"keep","text":"opaque\r\ncredential"});
        scrub(&mut value,&["opaque\r\ncredential".into()]);
        assert_eq!(value["[redacted-key-0]"],"keep");
        assert_eq!(value["text"],"[redacted]");
        assert!(!value.to_string().contains("password"));
    }
}
