//! Model-independent request and streaming event contracts.
//!
//! Rig can later implement ModelPort. No HTTP client or model dependency is
//! linked here. The OpenAI-compatible chat payload is only a conversion helper,
//! not a network client or a claim of compatibility with every provider.

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ModelRole {
    fn as_wire_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMessage {
    pub role: ModelRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<ModelMessage>,
    #[serde(default)]
    pub tools: Vec<ModelTool>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModelEvent {
    TextDelta { text: String },
    ToolCall { call_id: String, name: String, arguments: Value },
    Usage { input_tokens: u64, output_tokens: u64 },
    Finished { reason: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    InvalidRequest(&'static str),
    Provider(String),
}

impl ModelRequest {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.model.trim().is_empty() {
            return Err(ModelError::InvalidRequest("model is required"));
        }
        if self.messages.is_empty() {
            return Err(ModelError::InvalidRequest("at least one message is required"));
        }
        for message in &self.messages {
            if message.role == ModelRole::Tool && message.tool_call_id.is_none() {
                return Err(ModelError::InvalidRequest("tool message requires tool_call_id"));
            }
        }
        for tool in &self.tools {
            if tool.name.trim().is_empty() || !tool.input_schema.is_object() {
                return Err(ModelError::InvalidRequest("tool requires name and object schema"));
            }
        }
        Ok(())
    }

    /// Convert the text-only subset to a Chat Completions-style request body.
    /// A provider adapter is responsible for API version and feature negotiation.
    pub fn to_chat_completions_body(&self) -> Result<Value, ModelError> {
        self.validate()?;

        let messages: Vec<Value> = self.messages.iter().map(|message| {
            let mut item = json!({
                "role": message.role.as_wire_str(),
                "content": message.content,
            });
            if let Some(call_id) = &message.tool_call_id {
                item["tool_call_id"] = json!(call_id);
            }
            item
        }).collect();

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });
        if let Some(limit) = self.max_output_tokens {
            body["max_completion_tokens"] = json!(limit);
        }
        if !self.tools.is_empty() {
            let functions: Vec<Value> = self.tools.iter().map(|tool| json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                },
            })).collect();
            body["tools"] = json!(functions);
        }
        Ok(body)
    }
}

/// Host-owned model adapter. The callback is for incremental output; the
/// future may be cancelled by dropping it. This interface owns no executor.
pub trait ModelPort: Send + Sync {
    fn provider(&self) -> &str;
    fn stream<'a>(
        &'a self,
        request: ModelRequest,
        on_event: &'a mut (dyn FnMut(ModelEvent) + Send + 'a),
    ) -> BoxFuture<'a, Result<(), ModelError>>;
}
