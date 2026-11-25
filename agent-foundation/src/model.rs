//! Vendor-neutral model stream boundary inspired by Rig, not a Rig integration.
use crate::{
    context::{validate_context_list, ContextRef},
    FoundationError, Result,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelRequest {
    pub session_id: Uuid,
    pub model_id: String,
    pub prompt: String,
    pub context: Vec<ContextRef>,
}

impl ModelRequest {
    pub fn validate(&self) -> Result<()> {
        if self.model_id.trim().is_empty()
            || self.model_id.len() > 160
            || self.prompt.trim().is_empty()
            || self.prompt.len() > 32_768
        {
            return Err(FoundationError::InvalidInput("model request"));
        }
        validate_context_list(&self.context)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ModelEvent {
    TextDelta {
        text: String,
    },
    ToolSuggestion {
        tool_name: String,
        arguments_json: String,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    Finished,
    Failed {
        code: String,
    },
}

pub type ModelStream = mpsc::Receiver<ModelEvent>;
pub type ModelFuture<'a> = Pin<Box<dyn Future<Output = Result<ModelStream>> + Send + 'a>>;

/// Providers must bound their channel, enforce model capabilities, and honor cancellation.
/// No credentials, connections, or provider SDK are owned by this interface.
pub trait ModelProvider: Send + Sync {
    fn stream<'a>(&'a self, request: ModelRequest) -> ModelFuture<'a>;
}

/// Shared bounded argument parsing for streaming and batched model adapters.
/// This is syntax/size validation, not tool authorization. The capability
/// owner must still validate the tool's arguments and effects.
pub fn parse_tool_arguments(raw: &str) -> Result<serde_json::Value> {
    if raw.len() > 16_384 {
        return Err(FoundationError::InvalidInput("tool arguments too large"));
    }
    serde_json::from_str(raw).map_err(|_| FoundationError::InvalidInput("tool argument JSON"))
}

/// A fully parsed function call from an OpenAI-compatible response adapter.
/// Kept in the existing model boundary rather than in a second tool runtime.
pub struct FunctionCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Validate an entire assistant message before any tool in its batch runs.
/// Reject duplicated/missing IDs and malformed arguments as a batch so a
/// partially applied proposal cannot leave an unpaired durable transcript.
/// The legacy protocol permits an omitted function type; a supplied type
/// must be "function". It also permits null/absent tool_calls.
pub fn parse_function_batch(reply: &serde_json::Value, max_calls: usize) -> Result<Vec<FunctionCall>> {
    if reply["role"].as_str() != Some("assistant") {
        return Err(FoundationError::InvalidInput("assistant message role"));
    }
    let Some(calls) = reply.get("tool_calls").filter(|v| !v.is_null()) else {
        return Ok(Vec::new());
    };
    let calls = calls.as_array().ok_or(FoundationError::InvalidInput("function batch"))?;
    if calls.len() > max_calls {
        return Err(FoundationError::InvalidInput("function batch limit"));
    }
    let mut ids = std::collections::HashSet::new();
    let mut parsed = Vec::with_capacity(calls.len());
    for call in calls {
        let id = call["id"].as_str().filter(|id| !id.is_empty() && id.len() <= 256
            && !id.chars().any(char::is_whitespace) && !id.chars().any(char::is_control))
            .ok_or(FoundationError::InvalidInput("tool call id"))?;
        if !ids.insert(id) {
            return Err(FoundationError::InvalidInput("duplicate tool call id"));
        }
        if call.get("type").is_some_and(|kind| kind.as_str() != Some("function")) {
            return Err(FoundationError::InvalidInput("tool call type"));
        }
        let name = call["function"]["name"].as_str().filter(|name| !name.is_empty()
            && name.len() <= 128
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)))
            .ok_or(FoundationError::InvalidInput("tool name"))?;
        let raw = call["function"]["arguments"].as_str()
            .ok_or(FoundationError::InvalidInput("tool argument JSON"))?;
        let arguments = parse_tool_arguments(raw)?;
        if !arguments.is_object() {
            return Err(FoundationError::InvalidInput("function arguments object"));
        }
        parsed.push(FunctionCall { id: id.to_owned(), name: name.to_owned(), arguments });
    }
    Ok(parsed)
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn function_batch_rejects_invalid_or_duplicate_calls_before_dispatch() {
        let first = json!({"id":"one","type":"function","function":{"name":"read_file","arguments":"{}"}});
        let second = json!({"id":"two","function":{"name":"write_file","arguments":"{\"path\":\"a.rs\"}"}});
        let valid = json!({"role":"assistant","tool_calls":[first.clone(),second.clone()]});
        assert_eq!(parse_function_batch(&valid, 8).unwrap().len(), 2);
        for bad in [
            json!({"role":"assistant","tool_calls":[first.clone(),first.clone()]}),
            json!({"role":"assistant","tool_calls":[first.clone(),{"id":"two","function":{"name":"write_file","arguments":"{"}}]}),
            json!({"role":"assistant","tool_calls":[first.clone(),{"id":"two","function":{"name":"write_file","arguments":"null"}}]}),
            json!({"role":"assistant","tool_calls":"not an array"}),
            json!({"role":"system","tool_calls":[second]}),
        ] {
            assert!(parse_function_batch(&bad, 8).is_err());
        }
    }
}
