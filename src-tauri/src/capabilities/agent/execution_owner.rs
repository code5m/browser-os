//! Production Agent task lifecycle boundary.
//! Plain chat and project-scoped Coding share the official Foundation TaskRegistry
//! state machine, while their model transport and business tools stay owned by
//! the existing respective adapters. Each short-lived request owns its TaskRegistry
//! to avoid an unbounded global task cache; this is not a durable session store.
use browseros_agent_foundation::{event::EventJournal, task::TaskRegistry};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) struct ProductionExecution {
    tasks: TaskRegistry,
    id: Uuid,
    finished: bool,
}

impl ProductionExecution {
    pub(crate) fn begin(kind: &'static str) -> Result<Self, String> {
        if !matches!(kind, "chat" | "coding") {
            return Err("AGENT_TASK_KIND_DENIED".into());
        }
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(16)));
        // Never write user text or secrets into task metadata; session/workspace
        // identification is not invented for project-free chat.
        let task = tasks
            .create_scoped(Uuid::nil(), Uuid::new_v4(), kind, vec![])
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

    pub(crate) fn finish(mut self, success: bool) -> Result<(), String> {
        self.tasks
            .finish(self.id, success)
            .map_err(|_| "AGENT_TASK_FINISH_FAILED")?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for ProductionExecution {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.tasks.cancel(self.id);
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
        ProductionExecution::begin("coding")
            .unwrap()
            .finish(false)
            .unwrap();
        let aborted = ProductionExecution::begin("chat").unwrap();
        drop(aborted);
    }
}
