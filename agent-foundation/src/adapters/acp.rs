//! ACP v1 SDK projections for editor-to-agent session updates.
//! ACP session ids are transport identities, never authorization principals.
use crate::{model::ModelEvent, FoundationError, Result};
use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, SessionId, SessionNotification, SessionUpdate, TextContent,
};
use uuid::Uuid;

/// Produce a real ACP v1 typed session/update notification for a streamed text
/// delta. A model-supplied tool proposal NEVER becomes an ACP approval.
pub fn text_update(session: Uuid, text: &str) -> Result<SessionNotification> {
    if session.is_nil() || text.len() > 65_536 {
        return Err(FoundationError::InvalidInput("acp session text"));
    }
    Ok(SessionNotification::new(
        SessionId::new(session.to_string()),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            text,
        )))),
    ))
}

/// ACP typed events projected from safe model output. The host is responsible
/// for v1 protocol lifecycle/session notifications and explicit human approval.
pub fn project_events(session: Uuid, events: &[ModelEvent]) -> Result<Vec<SessionNotification>> {
    let mut projected = Vec::new();
    for event in events {
        match event {
            ModelEvent::TextDelta { text } => projected.push(text_update(session, text)?),
            ModelEvent::Usage { .. } | ModelEvent::Finished => {}
            // A tool suggestion must remain in the BrowserOS PermissionBroker.
            ModelEvent::ToolSuggestion { .. } => {}
            ModelEvent::Failed { .. } => {
                return Err(FoundationError::Provider("ACP_MODEL_FAILURE".into()));
            }
        }
        if projected.len() > 256 {
            return Err(FoundationError::InvalidInput("acp update limit"));
        }
    }
    Ok(projected)
}

/// Encode a typed ACP v1 update in the official SDK transport frame format.
/// The caller (trusted editor host) owns handshake, transport credentials,
/// flow control and session mapping; this helper never approves tool execution.
pub fn frame_text_update(
    session: Uuid,
    text: &str,
) -> Result<agent_client_protocol::TransportFrame> {
    let notification = text_update(session, text)?;
    let raw = agent_client_protocol::RawJsonRpcMessage::notification(
        "session/update".to_owned(),
        serde_json::to_value(notification)
            .map_err(|e| FoundationError::Serialization(e.to_string()))?,
    )
    .map_err(|_| FoundationError::Serialization("ACP notification frame".into()))?;
    Ok(agent_client_protocol::TransportFrame::Single(raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::JsonRpcMessage;
    #[test]
    fn acp_v1_session_update_roundtrip_and_no_model_approval() {
        let session = Uuid::new_v4();
        let events = vec![
            ModelEvent::TextDelta {
                text: "hello".into(),
            },
            ModelEvent::ToolSuggestion {
                tool_name: "shell.exec".into(),
                arguments_json: "{}".into(),
            },
        ];
        let projected = project_events(session, &events).unwrap();
        assert_eq!(projected.len(), 1);
        let wire = projected[0].to_untyped_message().unwrap();
        assert_eq!(wire.method(), "session/update");
        let decoded = SessionNotification::parse_message(wire.method(), wire.params()).unwrap();
        assert_eq!(decoded.session_id, SessionId::new(session.to_string()));
        assert_eq!(decoded, projected[0]);
    }

    #[tokio::test]
    async fn actual_sdk_channel_delivers_v1_notification_to_mock_editor() {
        use agent_client_protocol::{Channel, RawJsonRpcMessage, TransportFrame};
        use futures::StreamExt;
        let (runtime, mut editor) = Channel::duplex();
        let session = Uuid::new_v4();
        runtime
            .tx
            .unbounded_send(frame_text_update(session, "streaming").unwrap())
            .unwrap();
        let received = editor.rx.next().await.unwrap();
        let TransportFrame::Single(RawJsonRpcMessage::Notification(notification)) = received else {
            panic!("ACP v1 session/update must be notification");
        };
        assert_eq!(notification.method.as_ref(), "session/update");
        let encoded = serde_json::to_value(notification.params).unwrap();
        let parsed: SessionNotification = serde_json::from_value(encoded).unwrap();
        assert_eq!(parsed.session_id, SessionId::new(session.to_string()));
    }

    #[test]
    fn invalid_acp_session_and_model_failures_are_rejected() {
        assert!(text_update(Uuid::nil(), "bad").is_err());
        assert!(project_events(
            Uuid::new_v4(),
            &[ModelEvent::Failed {
                code: "fault".into()
            }]
        )
        .is_err());
    }
}
