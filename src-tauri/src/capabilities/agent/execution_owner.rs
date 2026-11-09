//! Production Agent task lifecycle boundary.
//! Plain chat and project-scoped Coding share the official Foundation TaskRegistry
//! state machine and a single process-level task registry. Model transport and
//! business tools remain in their separately approved owner adapters.
//! Terminal tasks are retired after completion to avoid an unbounded cache.
//! This is not a durable session store.
use browseros_agent_foundation::{
    context::ContextRef,
    event::EventJournal,
    task::{TaskRegistry, TaskStatus},
};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Arc, OnceLock},
};
use uuid::Uuid;

/// Stable task identity for a project with an existing Coding session.
/// The canonical workspace path is never written to the TaskRegistry: only
/// its domain-separated 128-bit digest is retained.
fn coding_scope(root: &Path, session_id: Option<&str>) -> Result<Option<(Uuid, Uuid)>, String> {
    let Some(session_id) = session_id.filter(|value| !value.is_empty()) else {
        // Bootstrap reads can create a new persisted session inside coding.rs;
        // do not pretend to know its ID before that operation finishes.
        return Ok(None);
    };
    let session = Uuid::parse_str(session_id).map_err(|_| "AGENT_SESSION_INVALID")?;
    let mut digest = Sha256::new();
    digest.update(b"browseros:coding:workspace:v1:");
    digest.update(root.to_string_lossy().as_bytes());
    let bytes = digest.finalize();
    let mut id = [0u8; 16];
    id.copy_from_slice(&bytes[..16]);
    // RFC 9562 UUIDv8: application-specific SHA-256 identity, not UUIDv5 SHA-1.
    id[6] = (id[6] & 0x0f) | 0x80;
    id[8] = (id[8] & 0x3f) | 0x80;
    Ok(Some((Uuid::from_bytes(id), session)))
}

static PRODUCTION_TASKS: OnceLock<TaskRegistry> = OnceLock::new();
fn task_owner() -> &'static TaskRegistry {
    PRODUCTION_TASKS.get_or_init(|| TaskRegistry::new(Arc::new(EventJournal::new(128))))
}

pub(crate) struct ProductionExecution {
    tasks: &'static TaskRegistry,
    id: Uuid,
    finished: bool,
}

impl ProductionExecution {
    pub(crate) fn begin(kind: &'static str) -> Result<Self, String> {
        Self::begin_scoped(kind, Uuid::nil(), Uuid::new_v4(), vec![])
    }

    /// Session-bearing Coding calls use exactly the existing persisted session
    /// UUID and stable canonical workspace identity for cancellation, approval
    /// and task trace correlation. No invented UUID replaces a real session.
    pub(crate) fn begin_coding(root: &Path, session_id: Option<&str>) -> Result<Self, String> {
        match coding_scope(root, session_id)? {
            Some((workspace, session)) => Self::begin_scoped("coding", workspace, session, vec![]),
            None => Self::begin("coding"),
        }
    }

    pub(crate) fn begin_scoped(
        kind: &'static str,
        workspace: Uuid,
        session: Uuid,
        context: Vec<ContextRef>,
    ) -> Result<Self, String> {
        if !matches!(kind, "chat" | "coding" | "legacy_agent" | "agent_channel") {
            return Err("AGENT_TASK_KIND_DENIED".into());
        }
        let tasks = task_owner();
        // Never write user text or secrets into task metadata; session/workspace
        // identification is not invented for project-free chat.
        let task = tasks
            .create_scoped(workspace, session, kind, context)
            .map_err(|_| "AGENT_TASK_CREATE_FAILED")?;
        tasks
            .start(task.id)
            .map_err(|_| "AGENT_TASK_START_FAILED")?;
        Ok(Self {
            tasks,
            id: task.id,
            finished: false,
        })
    }

    pub(crate) fn id(&self) -> Uuid {
        self.id
    }
    pub(crate) fn tasks(&self) -> &'static TaskRegistry {
        self.tasks
    }
    pub(crate) fn cancel(mut self) -> Result<(), String> {
        self.tasks
            .cancel(self.id)
            .map_err(|_| "AGENT_TASK_CANCEL_FAILED")?;
        self.finished = true;
        self.tasks
            .retire_terminal(self.id)
            .map_err(|_| "AGENT_TASK_RETIRE_FAILED".to_owned())
    }

    pub(crate) fn finish(mut self, success: bool) -> Result<(), String> {
        // Engines that use this same registry may have committed the terminal
        // transition themselves. Never finish an already-terminal task twice.
        let status = self
            .tasks
            .get(self.id)
            .map_err(|_| "AGENT_TASK_LOOKUP_FAILED")?
            .status;
        match (status, success) {
            (TaskStatus::Completed, true) | (TaskStatus::Failed, false) => {}
            (TaskStatus::Running, _) => self
                .tasks
                .finish(self.id, success)
                .map_err(|_| "AGENT_TASK_FINISH_FAILED")?,
            _ => return Err("AGENT_TASK_TERMINAL_MISMATCH".into()),
        }
        self.finished = true;
        self.tasks
            .retire_terminal(self.id)
            .map_err(|_| "AGENT_TASK_RETIRE_FAILED".to_owned())
    }
}

impl Drop for ProductionExecution {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.tasks.cancel(self.id);
            let _ = self.tasks.retire_terminal(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coding_identity_is_stable_and_uses_real_session() {
        let session = Uuid::new_v4();
        let id = session.to_string();
        let one = coding_scope(Path::new("/workspace/a"), Some(&id))
            .unwrap()
            .unwrap();
        let same = coding_scope(Path::new("/workspace/a"), Some(&id))
            .unwrap()
            .unwrap();
        let other = coding_scope(Path::new("/workspace/b"), Some(&id))
            .unwrap()
            .unwrap();
        assert_eq!(one, same);
        assert_eq!(one.1, session);
        assert_ne!(one.0, other.0);
        assert!(!one.0.is_nil());
        assert_eq!(coding_scope(Path::new("/workspace/a"), None).unwrap(), None);
        assert!(coding_scope(Path::new("/workspace/a"), Some("../../bad")).is_err());
    }

    #[test]
    fn production_agent_task_owner_lifecycle() {
        assert!(ProductionExecution::begin("unknown").is_err());
        ProductionExecution::begin("chat")
            .unwrap()
            .finish(true)
            .unwrap();
        ProductionExecution::begin("legacy_agent")
            .unwrap()
            .finish(true)
            .unwrap();
        ProductionExecution::begin("agent_channel")
            .unwrap()
            .finish(true)
            .unwrap();
        ProductionExecution::begin("coding")
            .unwrap()
            .finish(false)
            .unwrap();
        ProductionExecution::begin("legacy_agent")
            .unwrap()
            .cancel()
            .unwrap();
        ProductionExecution::begin("agent_channel")
            .unwrap()
            .cancel()
            .unwrap();
        let aborted = ProductionExecution::begin("chat").unwrap();
        drop(aborted);
        assert_eq!(task_owner().export_snapshot().unwrap(), "[]");
    }
}
