//! Rig 0.43 official SDK typed completion adapter.
//! Uses an injected, already-configured SDK DynModel. No provider credentials
//! are read here; no tools are executed by Rig or the model.
use crate::{
    engine::{ModelAdmissionBudget, TurnLimits},
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

// Bound the local serialization of SDK-decoded arguments before allocation.
fn bounded_arguments(value: &impl serde::Serialize) -> Result<String> {
    use std::io::{self, Write};
    struct Capped(Vec<u8>);
    impl Write for Capped {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > 16_384usize.saturating_sub(self.0.len()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "RIG_ARGUMENT_LIMIT",
                ));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Capped(Vec::new());
    serde_json::to_writer(&mut out, value)
        .map_err(|_| FoundationError::InvalidInput("rig tool arguments"))?;
    let text = String::from_utf8(out.0)
        .map_err(|_| FoundationError::InvalidInput("rig tool arguments"))?;
    if !crate::model::parse_tool_arguments(&text)?.is_object() {
        return Err(FoundationError::InvalidInput("rig tool arguments object"));
    }
    Ok(text)
}

/// Non-streaming SDK completion response -> bounded, typed single-turn events.
/// Multiple tool calls are rejected instead of partly executing a response.
pub fn from_rig_response(response: CompletionResponse) -> Result<Vec<ModelEvent>> {
    let mut events = Vec::new();
    let mut proposal = None;
    let mut budget = ModelAdmissionBudget::new(TurnLimits::default())?;
    for content in response.choice {
        budget.admit_events(1)?;
        match content {
            AssistantContent::Text(text) => {
                budget.admit_text(text.text.len())?;
                events.push(ModelEvent::TextDelta { text: text.text });
            }
            AssistantContent::ToolCall(tool) => {
                if proposal.is_some() {
                    return Err(FoundationError::Provider(
                        "RIG_MULTIPLE_TOOL_CALLS_UNSUPPORTED".into(),
                    ));
                }
                let id = tool.id.to_string();
                let name = tool.function.name.to_string();
                if id.is_empty()
                    || id.len() > 256
                    || id.chars().any(|c| c.is_control() || c.is_whitespace())
                    || name.is_empty()
                    || name.len() > 128
                    || !name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                {
                    return Err(FoundationError::InvalidInput("rig tool identity"));
                }
                let arguments_json = bounded_arguments(&tool.function.arguments)?;
                proposal = Some(ModelEvent::ToolSuggestion {
                    tool_name: name,
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
        budget.admit_events(1)?;
        events.push(ModelEvent::Usage {
            input_tokens,
            output_tokens,
        });
    }
    if let Some(proposal) = proposal {
        events.push(proposal);
    } else {
        budget.admit_events(1)?;
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
            let limits = TurnLimits::default();
            let deadline = tokio::time::Instant::now() + limits.deadline;
            // A real SDK model, injected by the trusted host, owns the
            // provider transport. The receiver closing cancels the stream.
            let mut streamed = self
                .model
                .stream(request)
                .map_err(|_| FoundationError::Provider("RIG_STREAM_START_FAILED".into()))?;
            let (sender, receiver) = mpsc::channel(8);
            tokio::spawn(async move {
                let mut streamed_text = String::new();
                let run = async {
                    let mut budget = ModelAdmissionBudget::new(limits)?;
                    loop {
                        let item = streamed.next().await;
                        // Ignored SDK control/argument/reasoning frames are
                        // still transport work and consume the event budget.
                        if item.is_some() {
                            budget.admit_events(1)?;
                        }
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
                                budget.admit_text(text.len())?;
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
                        budget.admit_events(1)?;
                        budget.admit_text(remainder.len())?;
                        sender
                            .send(ModelEvent::TextDelta {
                                text: remainder.to_owned(),
                            })
                            .await
                            .map_err(|_| FoundationError::Cancelled)?;
                    }
                    for event in events {
                        if !matches!(event, ModelEvent::TextDelta { .. }) {
                            // Final SDK projection shares the producer's raw
                            // event budget; it does not get a second allowance.
                            budget.admit_events(1)?;
                            sender
                                .send(event)
                                .await
                                .map_err(|_| FoundationError::Cancelled)?;
                        }
                    }
                    Ok(())
                };
                // Receiver close and the same absolute deadline cover every
                // read, send/backpressure wait and the SDK finish future.
                let result: Result<()> = tokio::select! {
                    biased;
                    _ = sender.closed() => Err(FoundationError::Cancelled),
                    _ = tokio::time::sleep_until(deadline) => Err(FoundationError::Timeout),
                    result = run => result,
                };
                if let Err(error) = result {
                    // Only a machine-readable code crosses the model event bus.
                    let code = match error {
                        FoundationError::Cancelled => "RIG_CANCELLED",
                        FoundationError::Timeout => "RIG_TIMEOUT",
                        _ => "RIG_STREAM_FAILURE",
                    };
                    // Do not wait indefinitely to report a fault. EOF without
                    // Finished or a proposal is itself a failed turn.
                    let _ = sender.try_send(ModelEvent::Failed { code: code.into() });
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
