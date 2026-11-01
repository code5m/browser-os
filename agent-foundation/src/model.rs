//! Vendor-neutral model stream boundary inspired by Rig, not a Rig integration.
use crate::{context::{validate_context_list, ContextRef}, FoundationError, Result};
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
        if self.model_id.trim().is_empty() || self.model_id.len() > 160
            || self.prompt.trim().is_empty() || self.prompt.len() > 32_768
        {
            return Err(FoundationError::InvalidInput("model request"));
        }
        validate_context_list(&self.context)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ModelEvent {
    TextDelta { text: String },
    ToolSuggestion { tool_name: String, arguments_json: String },
    Usage { input_tokens: u64, output_tokens: u64 },
    Finished,
    Failed { code: String },
}

pub type ModelStream = mpsc::Receiver<ModelEvent>;
pub type ModelFuture<'a> = Pin<Box<dyn Future<Output = Result<ModelStream>> + Send + 'a>>;

/// Providers must bound their channel, enforce model capabilities, and honor cancellation.
/// No credentials, connections, or provider SDK are owned by this interface.
pub trait ModelProvider: Send + Sync {
    fn stream<'a>(&'a self, request: ModelRequest) -> ModelFuture<'a>;
}
