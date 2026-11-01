//! Ordered task events and optional CloudEvents JSON projection.
//!
//! CloudEvents projection uses the standard envelope fields and a BrowserOS
//! data payload. This in-memory log is not durable storage or a network stream.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::agent::TaskState;
use crate::CONTRACT_VERSION;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    TaskState {
        state: TaskState,
    },
    ModelDelta {
        text: String,
    },
    ApprovalRequested {
        request_id: String,
        tool: String,
        scope: String,
    },
    ToolFinished {
        request_id: String,
        succeeded: bool,
    },
    ArtifactPublished {
        artifact_id: String,
        mime_type: String,
    },
    Error {
        message: String,
    },
}

impl EventKind {
    fn type_name(&self) -> &'static str {
        match self {
            Self::TaskState { .. } => "task_state",
            Self::ModelDelta { .. } => "model_delta",
            Self::ApprovalRequested { .. } => "approval_requested",
            Self::ToolFinished { .. } => "tool_finished",
            Self::ArtifactPublished { .. } => "artifact_published",
            Self::Error { .. } => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub schema_version: u16,
    pub event_id: String,
    pub task_id: String,
    pub session_id: String,
    pub sequence: u64,
    /// Caller-provided time; no OS clock is accessed by this crate.
    pub occurred_at_ms: u64,
    pub event: EventKind,
}

impl EventEnvelope {
    /// A valid CloudEvents 1.0 structured JSON envelope (without optional
    /// RFC3339 time); it does not imply an event broker or persistence.
    pub fn as_cloud_event(&self) -> Value {
        json!({
            "specversion": "1.0",
            "id": self.event_id,
            "source": "/browseros/agent-channel",
            "type": format!("dev.browseros.agent.{}", self.event.type_name()),
            "subject": self.task_id,
            "datacontenttype": "application/json",
            "data": {
                "schema_version": self.schema_version,
                "task_id": self.task_id,
                "session_id": self.session_id,
                "sequence": self.sequence,
                "occurred_at_ms": self.occurred_at_ms,
                "event": self.event,
            },
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventError {
    EmptyIdentity,
    DuplicateId,
    SequenceOverflow,
}

pub struct EventLog {
    task_id: String,
    session_id: String,
    seen_ids: BTreeSet<String>,
    events: Vec<EventEnvelope>,
}

impl EventLog {
    pub fn new(
        task_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Result<Self, EventError> {
        let (task_id, session_id) = (task_id.into(), session_id.into());
        if task_id.trim().is_empty() || session_id.trim().is_empty() {
            return Err(EventError::EmptyIdentity);
        }
        Ok(Self {
            task_id,
            session_id,
            seen_ids: BTreeSet::new(),
            events: Vec::new(),
        })
    }

    pub fn append(
        &mut self,
        event_id: impl Into<String>,
        occurred_at_ms: u64,
        event: EventKind,
    ) -> Result<EventEnvelope, EventError> {
        let event_id = event_id.into();
        if event_id.trim().is_empty() {
            return Err(EventError::EmptyIdentity);
        }
        if self.seen_ids.contains(&event_id) {
            return Err(EventError::DuplicateId);
        }
        let sequence = u64::try_from(self.events.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(EventError::SequenceOverflow)?;
        let entry = EventEnvelope {
            schema_version: CONTRACT_VERSION,
            event_id: event_id.clone(),
            task_id: self.task_id.clone(),
            session_id: self.session_id.clone(),
            sequence,
            occurred_at_ms,
            event,
        };
        self.seen_ids.insert(event_id);
        self.events.push(entry.clone());
        Ok(entry)
    }

    /// Replay only events after a cursor. Persistence is deliberately external.
    pub fn after(&self, sequence: u64) -> impl Iterator<Item = &EventEnvelope> {
        self.events
            .iter()
            .filter(move |entry| entry.sequence > sequence)
    }
}
