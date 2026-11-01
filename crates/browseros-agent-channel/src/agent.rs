//! Agent task lifecycle and explicit context references.
//!
//! State transitions model UI/task semantics; they do not grant tool access.
//! The tools::ToolCatalog approval gate is authoritative for tool execution.

use serde::{Deserialize, Serialize};

use crate::context::ContextRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Created,
    Running,
    WaitingApproval,
    Paused,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskAction {
    Start,
    RequestApproval,
    Approve,
    Pause,
    Resume,
    Succeed,
    Fail,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskError {
    EmptyIdentity,
    InvalidTransition { state: TaskState, action: TaskAction },
    RevisionOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    state: TaskState,
    revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRequest {
    pub task_id: String,
    pub session_id: String,
    pub instruction: String,
    /// Only explicitly selected / authorized references belong here.
    pub contexts: Vec<ContextRef>,
}

impl AgentTask {
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        agent_id: impl Into<String>,
    ) -> Result<Self, TaskError> {
        let (id, session_id, agent_id) = (id.into(), session_id.into(), agent_id.into());
        if [id.as_str(), session_id.as_str(), agent_id.as_str()]
            .iter()
            .any(|part| part.trim().is_empty())
        {
            return Err(TaskError::EmptyIdentity);
        }
        Ok(Self {
            id,
            session_id,
            agent_id,
            state: TaskState::Created,
            revision: 0,
        })
    }

    pub fn state(&self) -> TaskState {
        self.state
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn apply(&mut self, action: TaskAction) -> Result<TaskState, TaskError> {
        use TaskAction as A;
        use TaskState as S;
        let next = match (self.state, action) {
            (S::Created, A::Start) => S::Running,
            (S::Running, A::RequestApproval) => S::WaitingApproval,
            (S::WaitingApproval, A::Approve) => S::Running,
            (S::Running, A::Pause) => S::Paused,
            (S::Paused, A::Resume) => S::Running,
            (S::Running, A::Succeed) => S::Succeeded,
            (S::Running, A::Fail) => S::Failed,
            (S::Created | S::Running | S::WaitingApproval | S::Paused, A::Cancel) => S::Cancelled,
            _ => return Err(TaskError::InvalidTransition {
                state: self.state,
                action,
            }),
        };
        self.revision = self.revision.checked_add(1).ok_or(TaskError::RevisionOverflow)?;
        self.state = next;
        Ok(next)
    }
}

impl AgentRequest {
    pub fn validate(&self) -> Result<(), TaskError> {
        if self.task_id.trim().is_empty()
            || self.session_id.trim().is_empty()
            || self.instruction.trim().is_empty()
            || self.contexts.iter().any(|c| c.resource_id.trim().is_empty())
        {
            return Err(TaskError::EmptyIdentity);
        }
        Ok(())
    }
}
