//! Small, versioned internal message contract; NOT wire-compatible ACP or A2A.
//! The trusted host has a separate approval entry point; model-origin messages
//! cannot represent approval or tool execution authorization.
use crate::{context::ContextRef, FoundationError, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub version: u16,
    pub request_id: Uuid,
    pub payload: T,
}

impl<T> Envelope<T> {
    pub fn new(payload: T) -> Self {
        Self { version: SCHEMA_VERSION, request_id: Uuid::new_v4(), payload }
    }
    pub fn validate(&self) -> Result<()> {
        if self.version != SCHEMA_VERSION {
            return Err(FoundationError::InvalidInput("protocol version"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AgentCommand {
    Create { session_id: Uuid, goal: String, context: Vec<ContextRef> },
    Start { task_id: Uuid },
    Pause { task_id: Uuid },
    Cancel { task_id: Uuid },
    Get { task_id: Uuid },
}

/// Approval is purposefully absent from AgentCommand.
/// The user-facing host must authenticate a principal and call a trusted API.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_version_is_rejected() {
        let mut msg = Envelope::new(AgentCommand::Get { task_id: Uuid::new_v4() });
        msg.version = 55;
        assert_eq!(msg.validate(), Err(FoundationError::InvalidInput("protocol version")));
    }
    #[test]
    fn approval_cannot_be_deserialized_as_agent_command() {
        let raw = r#"{"command":"approve","task_id":"00000000-0000-0000-0000-000000000000"}"#;
        assert!(serde_json::from_str::<AgentCommand>(raw).is_err());
    }
}
