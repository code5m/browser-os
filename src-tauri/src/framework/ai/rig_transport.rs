//! Rig 0.43 generates the OpenAI-compatible request and decodes the reply.
//! The existing native profile/keyring remains the credential authority.
//! This reviewed unary transport binds the SDK to one HTTPS endpoint/model,
//! caps the body before allocation, and never performs a hidden retry.
use super::{
    acquire_provider_permit, provider_value_contains_credential, validate, ProviderProfile,
};
use bytes::Bytes;
use rig_core::{
    completion::{CompletionRequest, CompletionResponse},
    http_client::{
        self as http, DynHttpClient, HttpClientExt, LazyBody, MultipartForm, Request, Response,
        StreamingResponse,
    },
    providers::openai::OpenAIConfig,
    wasm_compat::WasmCompatSend,
};
use serde_json::{json, Value};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

const REQUEST_LIMIT: usize = 512 * 1024;
const RESPONSE_LIMIT: usize = 256 * 1024;

fn denied(code: &'static str) -> http::Error {
    http::Error::instance(std::io::Error::other(code))
}

#[derive(Clone)]
struct BoundHttp {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    credential: String,
    model: String,
    max_calls: usize,
    dispatched: Arc<AtomicBool>,
    // Only local, fixed error codes cross the SDK boundary. Never parse or
    // expose provider-controlled error strings to classify the failure.
    failure: Arc<Mutex<Option<&'static str>>>,
}

fn status_code(status: u16) -> &'static str {
    match status {
        401 | 403 => "AI_PROVIDER_AUTH_FAILED",
        404 => "AI_PROVIDER_ENDPOINT_NOT_FOUND",
        408 | 504 => "AI_PROVIDER_TIMEOUT",
        429 => "AI_PROVIDER_RATE_LIMITED",
        500..=599 => "AI_PROVIDER_SERVER_ERROR",
        _ => "AI_PROVIDER_HTTP_ERROR",
    }
}

impl BoundHttp {
    fn reject(&self, code: &'static str) -> http::Error {
        let mut failure = self.failure.lock().unwrap_or_else(|p| p.into_inner());
        if failure.is_none() {
            *failure = Some(code);
        }
        denied(code)
    }
    async fn unary<U>(&self, request: Request<Bytes>) -> http::Result<Response<LazyBody<U>>>
    where
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        let (parts, body) = request.into_parts();
        let endpoint = reqwest::Url::parse(&parts.uri.to_string())
            .map_err(|_| self.reject("AI_ENDPOINT_INVALID"))?;
        let auth = format!("Bearer {}", self.credential);
        if parts.method != http::Method::POST
            || endpoint != self.endpoint
            || parts
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                != Some(auth.as_str())
        {
            return Err(self.reject("AI_RIG_TRANSPORT_SCOPE_DENIED"));
        }
        if body.len() > REQUEST_LIMIT {
            return Err(self.reject("AI_CONTEXT_LIMIT"));
        }
        let payload: Value =
            serde_json::from_slice(&body).map_err(|_| self.reject("AI_CONTEXT_INVALID"))?;
        checked_wire_request(&payload, &self.model, &self.credential)
            .map_err(|code| self.reject(code))?;
        if self.dispatched.swap(true, Ordering::AcqRel) {
            return Err(self.reject("AI_RIG_DUPLICATE_DISPATCH"));
        }
        // Only the reviewed endpoint and auth are forwarded; SDK-provided
        // incidental headers cannot redirect or expand native authority.
        let mut response = self
            .client
            .post(endpoint)
            .bearer_auth(&self.credential)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|error| {
                self.reject(if error.is_timeout() {
                    "AI_PROVIDER_TIMEOUT"
                } else {
                    "AI_PROVIDER_REQUEST_FAILED"
                })
            })?;
        let status = response.status();
        if !status.is_success() {
            // Never pass provider-controlled error bodies/headers to logs.
            return Err(self.reject(status_code(status.as_u16())));
        }
        if response
            .content_length()
            .is_some_and(|length| length > RESPONSE_LIMIT as u64)
        {
            return Err(self.reject("AI_RESPONSE_LIMIT"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            self.reject(if error.is_timeout() {
                "AI_PROVIDER_TIMEOUT"
            } else {
                "AI_HTTP_IO"
            })
        })? {
            if chunk.len() > RESPONSE_LIMIT.saturating_sub(bytes.len()) {
                return Err(self.reject("AI_RESPONSE_LIMIT"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let safe = checked_wire_reply(&bytes, &self.credential, self.max_calls)
            .map_err(|code| self.reject(code))?;
        let body: LazyBody<U> = Box::pin(async move { Ok(U::from(Bytes::from(safe))) });
        Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(body)
            .map_err(http::Error::Protocol)
    }
}

impl HttpClientExt for BoundHttp {
    fn send<T, U>(
        &self,
        request: Request<T>,
    ) -> impl Future<Output = http::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        T: Into<Bytes> + WasmCompatSend,
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        let transport = self.clone();
        let (parts, body) = request.into_parts();
        let request = Request::from_parts(parts, body.into());
        async move { transport.unary(request).await }
    }

    fn send_multipart<U>(
        &self,
        _: Request<MultipartForm>,
    ) -> impl Future<Output = http::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        async { Err(denied("AI_RIG_MULTIPART_DENIED")) }
    }

    fn send_streaming<T>(
        &self,
        _: Request<T>,
    ) -> impl Future<Output = http::Result<StreamingResponse>> + WasmCompatSend
    where
        T: Into<Bytes> + WasmCompatSend,
    {
        async { Err(denied("AI_RIG_STREAMING_NOT_ENABLED")) }
    }
}

// Function arguments are JSON encoded inside a JSON string. Inspect the
// decoded value as well as the outer envelope before opening the transport.
// The current tool inventory does not limit historical, already paired calls.
fn checked_wire_request(
    payload: &Value,
    model: &str,
    credential: &str,
) -> Result<(), &'static str> {
    if payload["model"].as_str() != Some(model)
        || payload
            .get("stream")
            .is_some_and(|value| value.as_bool() != Some(false))
        || !payload["messages"].is_array()
    {
        return Err("AI_CONTEXT_INVALID");
    }
    if credential.is_empty() || provider_value_contains_credential(payload, credential) {
        return Err("AI_CONTEXT_CREDENTIAL_DENIED");
    }
    for message in payload["messages"]
        .as_array()
        .ok_or_else(|| "AI_CONTEXT_INVALID")?
    {
        if message
            .get("tool_calls")
            .is_some_and(|value| !value.is_null())
        {
            let calls = browseros_agent_foundation::model::parse_function_batch(message, 8)
                .map_err(|_| "AI_CONTEXT_INVALID")?;
            if calls
                .iter()
                .any(|call| provider_value_contains_credential(&call.arguments, credential))
            {
                return Err("AI_CONTEXT_CREDENTIAL_DENIED");
            }
        }
    }
    Ok(())
}

// Inspect the exact wire envelope BEFORE the SDK normalizes it (it may
// otherwise select only choice zero or synthesize missing provider call IDs).
// Redact text only; never change an executable argument into another value.
fn checked_wire_reply(
    bytes: &[u8],
    credential: &str,
    max_calls: usize,
) -> Result<Vec<u8>, &'static str> {
    if bytes.len() > RESPONSE_LIMIT || credential.is_empty() {
        return Err("AI_RESPONSE_LIMIT");
    }
    let mut body: Value = serde_json::from_slice(bytes).map_err(|_| "AI_RESPONSE_INVALID")?;
    let choices = body["choices"]
        .as_array_mut()
        .ok_or_else(|| "AI_RESPONSE_INVALID")?;
    if choices.len() != 1 {
        return Err("AI_RESPONSE_INVALID");
    }
    let choice = &mut choices[0];
    if choice
        .get("index")
        .is_some_and(|index| index.as_u64() != Some(0))
    {
        return Err("AI_RESPONSE_INVALID");
    }
    let finish = choice["finish_reason"].as_str().map(str::to_owned);
    let message = choice
        .get_mut("message")
        .filter(|value| value.is_object())
        .ok_or_else(|| "AI_RESPONSE_INVALID")?;
    if message["role"] != "assistant"
        || message
            .get("function_call")
            .is_some_and(|value| !value.is_null())
    {
        return Err("AI_RESPONSE_INVALID");
    }
    if let Some(content) = message.get("content").filter(|value| !value.is_null()) {
        let text = content.as_str().ok_or_else(|| "AI_RESPONSE_INVALID")?;
        message["content"] = json!(text.replace(credential, "[REDACTED]"));
    }
    let calls = browseros_agent_foundation::model::parse_function_batch(message, max_calls)
        .map_err(|_| "AI_RESPONSE_INVALID")?;
    if !matches!(
        (finish.as_deref(), calls.is_empty()),
        (Some("stop"), true) | (Some("tool_calls"), false)
    ) {
        return Err("AI_RESPONSE_INVALID");
    }
    if calls.is_empty()
        && !message["content"]
            .as_str()
            .is_some_and(|text| !text.trim().is_empty())
    {
        return Err("AI_RESPONSE_EMPTY");
    }
    if calls
        .iter()
        .any(|call| provider_value_contains_credential(&call.arguments, credential))
        || provider_value_contains_credential(&body, credential)
    {
        return Err("AI_CONTEXT_CREDENTIAL_DENIED");
    }
    let safe = serde_json::to_vec(&body).map_err(|_| "AI_RESPONSE_INVALID")?;
    if safe.len() > RESPONSE_LIMIT {
        return Err("AI_RESPONSE_LIMIT");
    }
    Ok(safe)
}

/// Actual SDK completion invocation. Called on the existing blocking owner
/// worker, never the GUI loop. The permit survives the SDK future and the
/// cancellation check. Dropping the local future does not prove remote stop.
pub(super) fn turn(
    profile: &ProviderProfile,
    request: CompletionRequest,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<CompletionResponse, String> {
    let _permit = acquire_provider_permit(&profile.id, cancelled)?;
    validate(profile)?;
    if profile.kind != "chat" {
        return Err("AI_MODEL_NOT_CHAT".into());
    }
    if profile.credential.is_empty() || profile.credential.contains(['\r', '\n', '\0']) {
        return Err("AI_CREDENTIAL_INVALID".into());
    }
    if cancelled() {
        return Err("AI_PROVIDER_CANCELLED".into());
    }
    let endpoint = reqwest::Url::parse(&format!(
        "{}/chat/completions",
        profile.base_url.trim_end_matches('/')
    ))
    .map_err(|_| "AI_ENDPOINT_INVALID")?;
    let max_calls = if request.tools.is_empty() { 0 } else { 8 };
    tauri::async_runtime::block_on(async {
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|_| "AI_HTTP_CLIENT_UNAVAILABLE")?;
        let failure = Arc::new(Mutex::new(None));
        let transport = BoundHttp {
            client,
            endpoint,
            credential: profile.credential.clone(),
            model: profile.model.clone(),
            max_calls,
            dispatched: Arc::new(AtomicBool::new(false)),
            failure: Arc::clone(&failure),
        };
        let sdk = OpenAIConfig::new(profile.credential.clone())
            .with_base_url(profile.base_url.trim_end_matches('/'))
            .with_system_instructions_as_messages()
            .connect(DynHttpClient::new(transport));
        let model = sdk.chat(profile.model.clone());
        // SDK produces the wire request from typed messages and tools; the
        // host cannot replace the selected model or override its token budget.
        let request = request
            .model::<String>(Some(profile.model.clone()))
            .max_tokens(4096u64);
        request
            .validate_message_content()
            .map_err(|_| "AI_CONTEXT_INVALID")?;
        let mut future = Box::pin(model.call(request));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
        loop {
            if cancelled() {
                return Err("AI_PROVIDER_CANCELLED".into());
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err("AI_PROVIDER_TIMEOUT".into());
            }
            match tokio::time::timeout(remaining.min(Duration::from_millis(50)), &mut future).await
            {
                Err(_) => continue,
                Ok(result) => {
                    if cancelled() {
                        return Err("AI_PROVIDER_CANCELLED".into());
                    }
                    // A completed poll can race the absolute deadline.
                    // Dropping this future releases only local transport
                    // resources; it does not prove provider-side cancellation.
                    if tokio::time::Instant::now() >= deadline {
                        return Err("AI_PROVIDER_TIMEOUT".into());
                    }
                    let mut result = result.map_err(|_| {
                        let code = *failure.lock().unwrap_or_else(|p| p.into_inner());
                        code.unwrap_or("AI_RIG_PROVIDER_FAILED")
                    })?;
                    // No raw provider diagnostics or headers in durable state.
                    result.raw = Value::Null;
                    // OpenAI Chat replay correlates only by function call ID.
                    // Keep the same projection as authenticated native history.
                    result.message_id = None;
                    result.response_id = None;
                    result.provider_request_id = None;
                    return Ok(result);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_status_classification_uses_only_fixed_codes() {
        assert_eq!(status_code(401), "AI_PROVIDER_AUTH_FAILED");
        assert_eq!(status_code(404), "AI_PROVIDER_ENDPOINT_NOT_FOUND");
        assert_eq!(status_code(429), "AI_PROVIDER_RATE_LIMITED");
        assert_eq!(status_code(504), "AI_PROVIDER_TIMEOUT");
        assert_eq!(status_code(503), "AI_PROVIDER_SERVER_ERROR");
        assert_eq!(status_code(302), "AI_PROVIDER_HTTP_ERROR");
    }
    #[test]
    fn wire_gate_denies_truncation_duplicate_choices_and_encoded_credentials() {
        let complete = json!({"choices":[{"index":0,"finish_reason":"stop",
            "message":{"role":"assistant","content":"secret"}}]});
        let raw = checked_wire_reply(&serde_json::to_vec(&complete).unwrap(), "secret", 0).unwrap();
        assert!(!String::from_utf8(raw).unwrap().contains("secret"));
        for content in [Value::Null, json!(""), json!("  ")] {
            let mut blank = complete.clone();
            blank["choices"][0]["message"]["content"] = content;
            assert!(checked_wire_reply(&serde_json::to_vec(&blank).unwrap(), "secret", 0).is_err());
        }
        let mut truncated = complete.clone();
        truncated["choices"][0]["finish_reason"] = json!("length");
        assert!(checked_wire_reply(&serde_json::to_vec(&truncated).unwrap(), "secret", 0).is_err());
        let duplicate =
            json!({"choices":[complete["choices"][0].clone(),complete["choices"][0].clone()]});
        assert!(checked_wire_reply(&serde_json::to_vec(&duplicate).unwrap(), "secret", 0).is_err());
        let tool = json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","content":null,
            "tool_calls":[{"id":"x","type":"function","function":{"name":"read_file","arguments":r#"{"path":"secr\u0065t"}"#}}]}}]});
        // Prove the inner JSON is valid and is rejected because of the decoded
        // credential, not because a malformed test string happens to fail.
        assert!(
            checked_wire_reply(&serde_json::to_vec(&tool).unwrap(), "different-key", 8).is_ok()
        );
        assert!(checked_wire_reply(&serde_json::to_vec(&tool).unwrap(), "secret", 8).is_err());
        let history = json!({"model":"fixture","messages":[tool["choices"][0]["message"].clone()]});
        assert!(checked_wire_request(&history, "fixture", "different-key").is_ok());
        assert!(checked_wire_request(&history, "fixture", "secret").is_err());
        let mut invalid_stream = history;
        invalid_stream["stream"] = json!("false");
        assert!(checked_wire_request(&invalid_stream, "fixture", "different-key").is_err());
    }
}
