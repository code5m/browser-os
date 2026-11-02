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
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

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
