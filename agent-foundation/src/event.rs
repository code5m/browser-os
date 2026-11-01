//! Bounded typed event journal + live broadcast; payload deliberately contains no
//! prompts, file contents, tool arguments, credentials, or raw model responses.
//! JSON projection follows CloudEvents v1 core shape, not an official SDK binding.
use crate::{FoundationError, Result};
use crate::task::TaskStatus;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::{HashMap, VecDeque}, sync::Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    TaskCreated,
    StateChanged { from: TaskStatus, to: TaskStatus },
    ApprovalRequested { call_id: Uuid, tool_name: String },
    ApprovalResolved { call_id: Uuid, accepted: bool },
    ToolFinished { call_id: Uuid, succeeded: bool },
    ModelProgress { bytes: u64 },
    Interrupted,
}

impl EventKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::TaskCreated => "task.created",
            Self::StateChanged { .. } => "task.state_changed",
            Self::ApprovalRequested { .. } => "tool.approval_requested",
            Self::ApprovalResolved { .. } => "tool.approval_resolved",
            Self::ToolFinished { .. } => "tool.finished",
            Self::ModelProgress { .. } => "model.progress",
            Self::Interrupted => "task.interrupted",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventRecord {
    pub event_id: Uuid,
    pub task_id: Uuid,
    pub sequence: u64,
    pub kind: EventKind,
}

impl EventRecord {
    /// CloudEvents-compatible JSON projection for future adapters.
    /// No timestamp is asserted: event-time ownership is left to the transport.
    pub fn cloud_events_json(&self) -> Value {
        json!({
            "specversion": "1.0",
            "id": self.event_id.to_string(),
            "source": "urn:browseros:agent-foundation",
            "type": format!("dev.browseros.agent.{}", self.kind.name()),
            "subject": self.task_id.to_string(),
            "datacontenttype": "application/json",
            "data": {
                "sequence": self.sequence,
                "kind": self.kind
            }
        })
    }
}

#[derive(Default)]
struct Inner {
    last_sequence: HashMap<Uuid, u64>,
    events: HashMap<Uuid, VecDeque<EventRecord>>,
}

/// The journal is an in-memory prototype. A production durable store must
/// atomically commit task state + event outbox before publishing events.
pub struct EventJournal {
    inner: Mutex<Inner>,
    retained_per_task: usize,
    sender: broadcast::Sender<EventRecord>,
}

impl EventJournal {
    pub fn new(retained_per_task: usize) -> Self {
        let retained_per_task = retained_per_task.clamp(1, 1024);
        let (sender, _) = broadcast::channel(retained_per_task.max(16));
        Self { inner: Mutex::new(Inner::default()), retained_per_task, sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventRecord> {
        self.sender.subscribe()
    }

    pub fn append(&self, task_id: Uuid, kind: EventKind) -> EventRecord {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let sequence = inner.last_sequence.entry(task_id).or_insert(0);
        *sequence += 1;
        let record = EventRecord { event_id: Uuid::new_v4(), task_id, sequence: *sequence, kind };
        let queue = inner.events.entry(task_id).or_default();
        queue.push_back(record.clone());
        if queue.len() > self.retained_per_task {
            queue.pop_front();
        }
        drop(inner);
        let _ = self.sender.send(record.clone()); // Zero subscribers is normal.
        record
    }

    /// Events with sequence > after. A missing history window is an explicit error;
    /// clients must obtain a state snapshot instead of silently skipping events.
    pub fn replay(&self, task_id: Uuid, after: u64) -> Result<Vec<EventRecord>> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(queue) = inner.events.get(&task_id) else { return Ok(Vec::new()); };
        if let Some(first) = queue.front() {
            if after < first.sequence.saturating_sub(1) {
                return Err(FoundationError::ReplayGap);
            }
        }
        Ok(queue.iter().filter(|x| x.sequence > after).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monotonic_and_gap_detected() {
        let journal = EventJournal::new(2);
        let id = Uuid::new_v4();
        for _ in 0..3 {
            journal.append(id, EventKind::TaskCreated);
        }
        assert_eq!(journal.replay(id, 1).unwrap().iter().map(|x| x.sequence).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(journal.replay(id, 0), Err(FoundationError::ReplayGap));
        let cloud = journal.replay(id, 2).unwrap().pop().unwrap().cloud_events_json();
        assert_eq!(cloud["specversion"], "1.0");
        assert_eq!(cloud["data"]["sequence"], 3);
    }
}
