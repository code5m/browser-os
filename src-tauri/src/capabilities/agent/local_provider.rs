//! Limited local Ollama provider adapter. Fixed loopback endpoint; no credentials,
//! tools, shell, or remote-network control. HTTP/1.1 close + bounded response.
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    time::{Duration, Instant},
};
const ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 11434);
const MAX_RESPONSE: usize = 256 * 1024;
#[derive(Deserialize)]
pub struct LocalChatRequest {
    pub model: String,
    pub message: String,
    pub system: String,
}
#[derive(Serialize, Deserialize)]
pub struct LocalChatAnswer {
    pub provider: String,
    pub model: String,
    pub content: String,
}
pub(crate) fn check(req: &LocalChatRequest) -> Result<(), String> {
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
fn send_local(body: &str, cancelled: &dyn Fn() -> bool) -> Result<String, String> {
    if cancelled() {
        return Err("AI_PROVIDER_CANCELLED".into());
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut socket = TcpStream::connect_timeout(&ADDRESS.into(), Duration::from_secs(2))
        .map_err(|_| "AGENT_PROVIDER_OFFLINE")?;
    if cancelled() {
        return Err("AI_PROVIDER_CANCELLED".into());
    }
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
        if cancelled() {
            return Err("AI_PROVIDER_CANCELLED".into());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("AGENT_PROVIDER_TIMEOUT".into());
        }
        socket
            .set_read_timeout(Some(remaining.min(Duration::from_millis(250))))
            .map_err(|_| "AGENT_PROVIDER_IO")?;
        let n = match socket.read(&mut buf) {
            Ok(n) => n,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(_) => return Err("AGENT_PROVIDER_IO".into()),
        };
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
pub(crate) fn local_chat(
    req: LocalChatRequest,
    lease: &browseros_agent_foundation::task::ExecutionLease<'_>,
    cancelled: &dyn Fn() -> bool,
) -> Result<LocalChatAnswer, String> {
    check(&req)?;
    let body = serde_json::json!({
       "model":req.model,"prompt":req.message,"system":req.system,
       "stream":false,"options":{"num_predict":1024}
    })
    .to_string();
    use browseros_agent_foundation::{engine::drive_owned_text_turn, FoundationError};
    let inactive = || cancelled() || !lease.is_active();
    let answer = drive_owned_text_turn(
        lease,
        || -> Result<serde_json::Value, String> {
            let text = send_local(&body, &inactive)?;
            let parsed: serde_json::Value =
                serde_json::from_str(&text).map_err(|_| "AGENT_PROVIDER_INVALID_RESPONSE")?;
            let response = parsed
                .get("response")
                .and_then(|x| x.as_str())
                .ok_or("AGENT_PROVIDER_INVALID_RESPONSE")?;
            Ok(serde_json::json!({"role":"assistant","content":response}))
        },
        &inactive,
        |error| match error {
            FoundationError::Cancelled => "AI_PROVIDER_CANCELLED".into(),
            FoundationError::Timeout => "AI_PROVIDER_TIMEOUT".into(),
            FoundationError::Provider(code)
                if matches!(code.as_str(), "MODEL_TEXT_LIMIT" | "MODEL_EVENT_LIMIT") =>
            {
                "AI_RESPONSE_LIMIT".into()
            }
            FoundationError::Provider(code) if code == "MODEL_TEXT_EMPTY" => {
                "AGENT_PROVIDER_EMPTY_RESPONSE".into()
            }
            _ => "AGENT_PROVIDER_INVALID_RESPONSE".into(),
        },
    )?;
    Ok(LocalChatAnswer {
        provider: "ollama_local".into(),
        model: req.model,
        content: answer,
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
    let storage = crate::workspace::data_dir(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let run = uuid::Uuid::new_v4();
        check(&request)?;
        let input = serde_json::json!({"provider":"ollama_local","model":request.model,
            "message":request.message,"system":request.system});
        let result = super::execution_owner::ProductionExecution::run_checkpointed_text(
            &storage,
            "local_chat",
            run,
            input,
            || false,
            |lease| {
                serde_json::to_value(local_chat(request, lease, &|| false)?)
                    .map_err(|_| "AGENT_RUN_RECEIPT_INVALID".into())
            },
        )?;
        serde_json::from_value(result).map_err(|_| "AGENT_RUN_RECEIPT_INVALID".into())
    })
    .await
    .map_err(|_| "AGENT_PROVIDER_WORKER_FAILED".to_string())?
}
