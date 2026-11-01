//! Rig 0.43 official SDK typed completion adapter.
//! Uses an injected, already-configured SDK DynModel. No provider credentials
//! are read here; no tools are executed by Rig or the model.
use crate::{
    model::{ModelEvent, ModelFuture, ModelProvider, ModelRequest},
    FoundationError, Result,
};
use futures::StreamExt;
use rig_core::{
    completion::{message::AssistantContent, CompletionRequest, CompletionResponse, Message},
    driver::DynModel,
    operation::Completion,
    streaming::{Item, StreamEvent},
};
use tokio::sync::mpsc;

pub struct RigCompletionProvider {
    pub model: DynModel<Completion>,
}

pub fn to_rig_request(request: &ModelRequest) -> Result<CompletionRequest> {
    request.validate()?;
    // Context IDs intentionally are NOT interpolated into prompts or used as
    // remote fetch targets. BrowserOS context owner resolves them separately.
    let candidate = CompletionRequest::new(Message::user(&request.prompt))
        .model::<String>(Some(request.model_id.clone()));
    candidate
        .validate_message_content()
        .map_err(|_| FoundationError::InvalidInput("rig request"))?;
    Ok(candidate)
}

/// Non-streaming SDK completion response -> bounded, typed single-turn events.
/// Multiple tool calls are rejected instead of partly executing a response.
pub fn from_rig_response(response: CompletionResponse) -> Result<Vec<ModelEvent>> {
    let mut events = Vec::new();
    let mut proposal = None;
    for content in response.choice {
        match content {
            AssistantContent::Text(text) => {
                if text.text.len() > 65_536 {
                    return Err(FoundationError::Provider("RIG_TEXT_LIMIT".into()));
                }
                events.push(ModelEvent::TextDelta { text: text.text });
            }
            AssistantContent::ToolCall(tool) => {
                if proposal.is_some() {
                    return Err(FoundationError::Provider(
                        "RIG_MULTIPLE_TOOL_CALLS_UNSUPPORTED".into(),
                    ));
                }
                let arguments_json = serde_json::to_string(&tool.function.arguments)
                    .map_err(|e| FoundationError::Serialization(e.to_string()))?;
                if arguments_json.len() > 16_384 {
                    return Err(FoundationError::InvalidInput("rig tool arguments"));
                }
                proposal = Some(ModelEvent::ToolSuggestion {
                    tool_name: tool.function.name.to_string(),
                    arguments_json,
                });
            }
            AssistantContent::Reasoning(_) | AssistantContent::Image(_) => {
                // No reasoning signatures, secrets or images are ever silently
                // converted to plain model text, tools or audit events.
                return Err(FoundationError::Provider("RIG_UNSUPPORTED_CONTENT".into()));
            }
        }
    }
    if let (Some(input_tokens), Some(output_tokens)) =
        (response.usage.input_tokens, response.usage.output_tokens)
    {
        events.push(ModelEvent::Usage {
            input_tokens,
            output_tokens,
        });
    }
    if let Some(proposal) = proposal {
        events.push(proposal);
    } else {
        events.push(ModelEvent::Finished);
    }
    Ok(events)
}

impl ModelProvider for RigCompletionProvider {
    fn stream<'a>(&'a self, request: ModelRequest) -> ModelFuture<'a> {
        Box::pin(async move {
            // Do not allow a model request to silently override the exact
            // host-configured SDK model identity.
            if self.model.id().is_some_and(|id| id != request.model_id) {
                return Err(FoundationError::PermissionDenied);
            }
            let request = to_rig_request(&request)?;
            // A real SDK model, injected by the trusted host, owns the
            // provider transport. The receiver closing cancels the stream.
            let mut streamed = self
                .model
                .stream(request)
                .map_err(|_| FoundationError::Provider("RIG_STREAM_START_FAILED".into()))?;
            let (sender, receiver) = mpsc::channel(8);
            tokio::spawn(async move {
                let mut streamed_text = String::new();
                let result: Result<()> = async {
                    loop {
                        let item = tokio::select! {
                            _ = sender.closed() => return Err(FoundationError::Cancelled),
                            item = streamed.next() => item,
                        };
                        match item {
                            None => break,
                            Some(Err(_)) => {
                                return Err(FoundationError::Provider("RIG_STREAM_ERROR".into()))
                            }
                            Some(Ok(Item::Unknown(_))) => {
                                return Err(FoundationError::Provider(
                                    "RIG_UNMODELED_STREAM_FRAME".into(),
                                ));
                            }
                            Some(Ok(Item::Event(StreamEvent::Text { text, .. }))) => {
                                if streamed_text.len().saturating_add(text.len()) > 64 * 1024 {
                                    return Err(FoundationError::Provider("RIG_TEXT_LIMIT".into()));
                                }
                                streamed_text.push_str(&text);
                                sender
                                    .send(ModelEvent::TextDelta { text })
                                    .await
                                    .map_err(|_| FoundationError::Cancelled)?;
                            }
                            Some(Ok(Item::Event(
                                StreamEvent::Start { .. }
                                | StreamEvent::Arguments { .. }
                                | StreamEvent::End { .. }
                                | StreamEvent::Reasoning { .. },
                            ))) => {}
                        }
                    }
                    let response = streamed.finish().await.map_err(|_| {
                        FoundationError::Provider("RIG_STREAM_FINISH_FAILED".into())
                    })?;
                    let events = from_rig_response(response)?;
                    let final_text: String = events
                        .iter()
                        .filter_map(|event| match event {
                            ModelEvent::TextDelta { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect();
                    if !final_text.starts_with(&streamed_text) {
                        return Err(FoundationError::Provider("RIG_STREAM_TEXT_MISMATCH".into()));
                    }
                    let remainder = &final_text[streamed_text.len()..];
                    if !remainder.is_empty() {
                        sender
                            .send(ModelEvent::TextDelta {
                                text: remainder.to_owned(),
                            })
                            .await
                            .map_err(|_| FoundationError::Cancelled)?;
                    }
                    for event in events {
                        if !matches!(event, ModelEvent::TextDelta { .. }) {
                            sender
                                .send(event)
                                .await
                                .map_err(|_| FoundationError::Cancelled)?;
                        }
                    }
                    Ok(())
                }
                .await;
                if let Err(error) = result {
                    // Only a machine-readable code crosses the model event bus.
                    let code = match error {
                        FoundationError::Cancelled => "RIG_CANCELLED",
                        _ => "RIG_STREAM_FAILURE",
                    };
                    let _ = sender.send(ModelEvent::Failed { code: code.into() }).await;
                }
            });
            Ok(receiver)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::completion::{
        message::{Text, ToolCall, ToolFunction, ToolName},
        Usage,
    };
    use uuid::Uuid;

    fn request() -> ModelRequest {
        ModelRequest {
            session_id: Uuid::new_v4(),
            model_id: "mock-rig".into(),
            prompt: "Review".into(),
            context: vec![],
        }
    }

    #[test]
    fn request_is_sdk_typed_and_context_not_injected() {
        let parsed = to_rig_request(&request()).unwrap();
        assert_eq!(parsed.model.as_deref(), Some("mock-rig"));
        assert_eq!(parsed.chat_history, vec![Message::user("Review")]);
    }

    #[test]
    fn official_sdk_completion_is_mapped_without_running_tool() {
        let tool = ToolCall::from_wire(
            "call42",
            ToolFunction::new(
                ToolName::new("editor.read").unwrap(),
                serde_json::json!({"file":"a.rs"}),
            ),
        );
        let usage = Usage {
            input_tokens: Some(7),
            output_tokens: Some(9),
            ..Usage::default()
        };
        let response = CompletionResponse::new(
            vec![
                AssistantContent::Text(Text::new("Reviewing")),
                AssistantContent::ToolCall(tool),
            ],
            usage,
            "mock",
            serde_json::Value::Null,
        );
        let events = from_rig_response(response).unwrap();
        assert_eq!(events.len(), 3);
        assert!(
            matches!(&events[2],ModelEvent::ToolSuggestion {tool_name,..} if tool_name=="editor.read")
        );
    }

    #[test]
    fn duplicate_tool_calls_are_fail_closed() {
        let one = ToolCall::from_wire(
            "a",
            ToolFunction::new(ToolName::new("editor.read").unwrap(), serde_json::json!({})),
        );
        let two = ToolCall::from_wire(
            "b",
            ToolFunction::new(ToolName::new("editor.read").unwrap(), serde_json::json!({})),
        );
        let response = CompletionResponse::new(
            vec![
                AssistantContent::ToolCall(one),
                AssistantContent::ToolCall(two),
            ],
            Usage::default(),
            "mock",
            serde_json::Value::Null,
        );
        assert!(from_rig_response(response).is_err());
    }
}
