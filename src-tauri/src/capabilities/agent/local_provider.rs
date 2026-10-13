//! Limited local Ollama provider adapter. Fixed loopback endpoint; no credentials,
//! tools, shell, or remote-network control. HTTP/1.1 close + bounded response.
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    time::Duration,
};
const ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 11434);
const MAX_RESPONSE: usize = 256 * 1024;
#[derive(Deserialize)]
pub struct LocalChatRequest {
    pub model: String,
    pub message: String,
    pub system: String,
}
#[derive(Serialize)]
pub struct LocalChatAnswer {
    pub provider: String,
    pub model: String,
    pub content: String,
}
fn check(req: &LocalChatRequest) -> Result<(), String> {
    if req.model.trim().is_empty()
        || req.model.len() > 100
        || !req
            .model
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.:-/".contains(&c))
        || req.message.trim().is_empty()
        || req.message.len() > 12_000
        || req.system.len() > 8_000
    {
        return Err("AGENT_CHAT_INVALID_INPUT".into());
    }
    if crate::security_policy::contains_credential_leak(&req.message)
        || crate::security_policy::contains_credential_leak(&req.system)
    {
        return Err("AGENT_CHAT_CREDENTIAL_DENIED".into());
    }
    Ok(())
}
fn send_local(body: &str) -> Result<String, String> {
    let mut socket = TcpStream::connect_timeout(&ADDRESS.into(), Duration::from_secs(2))
        .map_err(|_| "AGENT_PROVIDER_OFFLINE")?;
    socket
        .set_read_timeout(Some(Duration::from_secs(25)))
        .map_err(|_| "AGENT_PROVIDER_IO")?;
    socket
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|_| "AGENT_PROVIDER_IO")?;
    let header=format!("POST /api/generate HTTP/1.1\r\nHost: 127.0.0.1:11434\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",body.len());
    socket
        .write_all(header.as_bytes())
        .and_then(|_| socket.write_all(body.as_bytes()))
        .map_err(|_| "AGENT_PROVIDER_IO")?;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = socket
            .read(&mut buf)
            .map_err(|_| "AGENT_PROVIDER_TIMEOUT")?;
        if n == 0 {
            break;
        }
        if response.len() + n > MAX_RESPONSE {
            return Err("AGENT_RESPONSE_TOO_LARGE".into());
        }
        response.extend_from_slice(&buf[..n]);
    }
    let text = std::str::from_utf8(&response).map_err(|_| "AGENT_PROVIDER_INVALID_RESPONSE")?;
    let (headers, payload) = text
        .split_once("\r\n\r\n")
        .ok_or("AGENT_PROVIDER_INVALID_RESPONSE")?;
    if !headers.starts_with("HTTP/1.1 200 ") && !headers.starts_with("HTTP/1.0 200 ") {
        return Err("AGENT_PROVIDER_HTTP_ERROR".into());
    }
    // Fail closed on unsupported chunked transfer or streaming responses.
    if headers
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        return Err("AGENT_PROVIDER_TRANSFER_UNSUPPORTED".into());
    }
    Ok(payload.to_owned())
}
pub fn local_chat(req: LocalChatRequest) -> Result<LocalChatAnswer, String> {
    check(&req)?;
    let body = serde_json::json!({
       "model":req.model,"prompt":req.message,"system":req.system,
       "stream":false,"options":{"num_predict":1024}
    })
    .to_string();
    let text = send_local(&body)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "AGENT_PROVIDER_INVALID_RESPONSE")?;
    let answer = parsed
        .get("response")
        .and_then(|x| x.as_str())
        .ok_or("AGENT_PROVIDER_INVALID_RESPONSE")?;
    if answer.is_empty() {
        return Err("AGENT_PROVIDER_EMPTY_RESPONSE".into());
    }
    Ok(LocalChatAnswer {
        provider: "ollama_local".into(),
        model: req.model,
        content: answer.into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn denies_invalid_input_without_network() {
        assert!(check(&LocalChatRequest {
            model: "".into(),
            message: "hi".into(),
            system: "".into()
        })
        .is_err());
        assert!(check(&LocalChatRequest {
            model: "test\r\n".into(),
            message: "hi".into(),
            system: "".into()
        })
        .is_err());
        assert!(check(&LocalChatRequest {
            model: "m".into(),
            message: "".into(),
            system: "".into()
        })
        .is_err());
    }
}

#[tauri::command]
pub async fn agent_local_chat(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    request: LocalChatRequest,
) -> Result<LocalChatAnswer, String> {
    crate::shared::invocation::check_invocation_source(&webview, "agent_local_chat", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || local_chat(request))
        .await
        .map_err(|_| "AGENT_PROVIDER_WORKER_FAILED".to_string())?
}
