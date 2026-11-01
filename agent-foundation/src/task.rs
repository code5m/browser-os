//! Independent task state machine. No Tauri commands, networking or file writes.
//! Trust boundary: resolve_approval MUST only be called by an authenticated host UI.
//! Never expose this method through a model-callable tool or untrusted RPC.
use crate::{
    context::{validate_context_list, ContextRef},
    event::{EventJournal, EventKind},
    tool::ToolEffect,
    FoundationError, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Ready,
    Running,
    AwaitingApproval,
    Paused,
    Completed,
    Failed,
    Cancelled,
}
impl TaskStatus {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingOperation {
    pub call_id: Uuid,
    pub tool_name: String,
    pub arguments: Value,
    pub effect: ToolEffect,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: Uuid,
    pub session_id: Uuid,
    pub goal: String,
    pub context: Vec<ContextRef>,
    pub status: TaskStatus,
    pub revision: u64,
    pub pending: Option<PendingOperation>,
}

/// One-shot permit: only TaskRegistry::resolve_approval can construct it;
/// not serializable, not cloneable, cannot be forged through model JSON.
pub struct ApprovedOperation {
    pub(crate) task_id: Uuid,
    pub(crate) revision: u64,
    pub(crate) operation: PendingOperation,
}

pub struct TaskRegistry {
    tasks: Mutex<HashMap<Uuid, TaskRecord>>,
    signals: Mutex<HashMap<Uuid, watch::Sender<bool>>>,
    journal: Arc<EventJournal>,
    execution_leases: Mutex<HashSet<Uuid>>,
}

/// Prevents concurrent model/tool execution for one task across all adapters.
/// Dropping the lease releases it (including early errors and cancellation).
pub(crate) struct ExecutionLease<'a> {
    registry: &'a TaskRegistry,
    task_id: Uuid,
}

impl Drop for ExecutionLease<'_> {
    fn drop(&mut self) {
        self.registry
            .execution_leases
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.task_id);
    }
}

impl TaskRegistry {
    pub fn new(journal: Arc<EventJournal>) -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            signals: Mutex::new(HashMap::new()),
            journal,
            execution_leases: Mutex::new(HashSet::new()),
        }
    }

    pub fn create(
        &self,
        session_id: Uuid,
        goal: impl Into<String>,
        context: Vec<ContextRef>,
    ) -> Result<TaskRecord> {
        let goal = goal.into();
        if goal.trim().is_empty() || goal.len() > 32_768 {
            return Err(FoundationError::InvalidInput("task goal"));
        }
        validate_context_list(&context)?;
        let record = TaskRecord {
            id: Uuid::new_v4(),
            session_id,
            goal,
            context,
            status: TaskStatus::Ready,
            revision: 0,
            pending: None,
        };
        self.tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(record.id, record.clone());
        let (sender, _) = watch::channel(false);
        self.signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(record.id, sender);
        self.journal.append(record.id, EventKind::TaskCreated);
        Ok(record)
    }

    pub fn get(&self, id: Uuid) -> Result<TaskRecord> {
        self.tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned()
            .ok_or(FoundationError::NotFound)
    }

    /// Acquire the sole in-flight model/tool execution slot for a task.
    /// Callers must still check cancellation and revision after async work.
    pub(crate) fn acquire_execution(&self, id: Uuid, revision: u64) -> Result<ExecutionLease<'_>> {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let task = tasks.get(&id).ok_or(FoundationError::NotFound)?;
        if task.status != TaskStatus::Running || task.revision != revision {
            return Err(FoundationError::InvalidTransition);
        }
        let mut leases = self
            .execution_leases
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !leases.insert(id) {
            return Err(FoundationError::InvalidTransition);
        }
        Ok(ExecutionLease {
            registry: self,
            task_id: id,
        })
    }

    pub fn start(&self, id: Uuid) -> Result<()> {
        let previous = {
            let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
            let task = tasks.get_mut(&id).ok_or(FoundationError::NotFound)?;
            if !matches!(task.status, TaskStatus::Ready | TaskStatus::Paused) {
                return Err(FoundationError::InvalidTransition);
            }
            let previous = task.status;
            task.status = TaskStatus::Running;
            task.revision += 1;
            previous
        };
        // A previously paused operation has a cancelled receiver; use a fresh epoch.
        let (sender, _) = watch::channel(false);
        self.signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, sender);
        self.journal.append(
            id,
            EventKind::StateChanged {
                from: previous,
                to: TaskStatus::Running,
            },
        );
        Ok(())
    }

    pub fn pause(&self, id: Uuid) -> Result<()> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let task = tasks.get_mut(&id).ok_or(FoundationError::NotFound)?;
        if task.status != TaskStatus::Running {
            return Err(FoundationError::InvalidTransition);
        }
        task.status = TaskStatus::Paused;
        task.revision += 1;
        drop(tasks);
        self.signal_cancel(id);
        self.journal.append(
            id,
            EventKind::StateChanged {
                from: TaskStatus::Running,
                to: TaskStatus::Paused,
            },
        );
        Ok(())
    }

    pub fn cancel(&self, id: Uuid) -> Result<()> {
        let old = {
            let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
            let task = tasks.get_mut(&id).ok_or(FoundationError::NotFound)?;
            if task.status.is_terminal() {
                return Err(FoundationError::InvalidTransition);
            }
            let old = task.status;
            task.status = TaskStatus::Cancelled;
            task.pending = None;
            task.revision += 1;
            old
        };
        self.signal_cancel(id);
        self.journal.append(
            id,
            EventKind::StateChanged {
                from: old,
                to: TaskStatus::Cancelled,
            },
        );
        Ok(())
    }

    pub fn finish(&self, id: Uuid, success: bool) -> Result<()> {
        let status = if success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        {
            let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
            let task = tasks.get_mut(&id).ok_or(FoundationError::NotFound)?;
            if task.status != TaskStatus::Running || task.pending.is_some() {
                return Err(FoundationError::InvalidTransition);
            }
            task.status = status;
            task.revision += 1;
        }
        self.signal_cancel(id);
        self.journal.append(
            id,
            EventKind::StateChanged {
                from: TaskStatus::Running,
                to: status,
            },
        );
        Ok(())
    }

    pub(crate) fn propose_tool(
        &self,
        id: Uuid,
        tool_name: impl Into<String>,
        arguments: Value,
        effect: ToolEffect,
    ) -> Result<Uuid> {
        let tool_name = tool_name.into();
        if tool_name.is_empty()
            || tool_name.len() > 128
            || !tool_name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            || serde_json::to_vec(&arguments)
                .map_err(|e| FoundationError::Serialization(e.to_string()))?
                .len()
                > 16_384
        {
            return Err(FoundationError::InvalidInput("tool proposal"));
        }
        let pending = PendingOperation {
            call_id: Uuid::new_v4(),
            tool_name,
            arguments,
            effect,
        };
        let call_id = pending.call_id;
        let name = pending.tool_name.clone();
        {
            let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
            let task = tasks.get_mut(&id).ok_or(FoundationError::NotFound)?;
            if task.status != TaskStatus::Running {
                return Err(FoundationError::InvalidTransition);
            }
            task.pending = Some(pending);
            task.status = TaskStatus::AwaitingApproval;
            task.revision += 1;
        }
        self.journal.append(
            id,
            EventKind::StateChanged {
                from: TaskStatus::Running,
                to: TaskStatus::AwaitingApproval,
            },
        );
        self.journal.append(
            id,
            EventKind::ApprovalRequested {
                call_id,
                tool_name: name,
            },
        );
        Ok(call_id)
    }

    /// This is a trusted-host function, NEVER a model/tool RPC.
    /// Approval is scoped to exact (task, call) and consumes the pending operation.
    pub fn resolve_approval(
        &self,
        id: Uuid,
        call_id: Uuid,
        accepted: bool,
    ) -> Result<Option<ApprovedOperation>> {
        let (pending, revision) = {
            let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
            let task = tasks.get_mut(&id).ok_or(FoundationError::NotFound)?;
            if task.status != TaskStatus::AwaitingApproval {
                return Err(FoundationError::InvalidTransition);
            }
            if task
                .pending
                .as_ref()
                .is_none_or(|call| call.call_id != call_id)
            {
                return Err(FoundationError::ApprovalMismatch);
            }
            let pending = task
                .pending
                .take()
                .ok_or(FoundationError::ApprovalMismatch)?;
            task.status = TaskStatus::Running;
            task.revision += 1;
            (pending, task.revision)
        };
        self.journal
            .append(id, EventKind::ApprovalResolved { call_id, accepted });
        self.journal.append(
            id,
            EventKind::StateChanged {
                from: TaskStatus::AwaitingApproval,
                to: TaskStatus::Running,
            },
        );
        if accepted {
            Ok(Some(ApprovedOperation {
                task_id: id,
                revision,
                operation: pending,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn cancellation(&self, id: Uuid) -> Result<watch::Receiver<bool>> {
        self.signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .map(watch::Sender::subscribe)
            .ok_or(FoundationError::NotFound)
    }

    pub(crate) fn active_at_revision(&self, id: Uuid, revision: u64) -> bool {
        self.tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .is_some_and(|task| task.status == TaskStatus::Running && task.revision == revision)
    }

    pub(crate) fn journal(&self) -> &Arc<EventJournal> {
        &self.journal
    }

    fn signal_cancel(&self, id: Uuid) {
        if let Some(signal) = self
            .signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
        {
            signal.send_replace(true);
        }
    }

    /// Serialize only into a trusted, access-controlled, encrypted host store.
    /// No disk writes are performed by this crate.
    pub fn export_snapshot(&self) -> Result<String> {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let mut records: Vec<TaskRecord> = tasks.values().cloned().collect();
        records.sort_by_key(|task| task.id);
        serde_json::to_string(&records).map_err(|e| FoundationError::Serialization(e.to_string()))
    }

    /// In-flight work and approvals never resume automatically. Pending operations
    /// are discarded, and running tasks become Paused until explicit user action.
    pub fn restore_snapshot(json: &str, journal: Arc<EventJournal>) -> Result<Self> {
        if json.len() > 4 * 1024 * 1024 {
            return Err(FoundationError::InvalidInput("snapshot too large"));
        }
        let mut records: Vec<TaskRecord> = serde_json::from_str(json)
            .map_err(|e| FoundationError::Serialization(e.to_string()))?;
        if records.len() > 1024 {
            return Err(FoundationError::InvalidInput("too many tasks"));
        }
        let registry = Self::new(journal);
        for task in &mut records {
            if task.id.is_nil() || task.goal.trim().is_empty() || task.goal.len() > 32_768 {
                return Err(FoundationError::InvalidInput("snapshot task"));
            }
            validate_context_list(&task.context)?;
            let interrupted = matches!(
                task.status,
                TaskStatus::Running | TaskStatus::AwaitingApproval
            );
            if interrupted {
                task.status = TaskStatus::Paused;
                task.revision += 1;
            }
            task.pending = None;
            if registry
                .tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(task.id, task.clone())
                .is_some()
            {
                return Err(FoundationError::InvalidInput("duplicate task id"));
            }
            let (sender, _) = watch::channel(false);
            registry
                .signals
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(task.id, sender);
            if interrupted {
                registry.journal.append(task.id, EventKind::Interrupted);
            }
        }
        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_is_one_shot_and_task_scoped() {
        let registry = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let task = registry
            .create(Uuid::new_v4(), "Explain code", vec![])
            .unwrap();
        registry.start(task.id).unwrap();
        let call = registry
            .propose_tool(task.id, "editor.read", Value::Null, ToolEffect::ReadOnly)
            .unwrap();
        assert_eq!(
            registry
                .resolve_approval(task.id, Uuid::new_v4(), true)
                .err(),
            Some(FoundationError::ApprovalMismatch)
        );
        assert!(registry
            .resolve_approval(task.id, call, true)
            .unwrap()
            .is_some());
        assert_eq!(
            registry.resolve_approval(task.id, call, true).err(),
            Some(FoundationError::InvalidTransition)
        );
    }

    #[test]
    fn task_execution_lease_is_exclusive_and_raii() {
        let registry = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let task = registry.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        registry.start(task.id).unwrap();
        let rev = registry.get(task.id).unwrap().revision;
        let one = registry.acquire_execution(task.id, rev).unwrap();
        assert_eq!(
            registry.acquire_execution(task.id, rev).err(),
            Some(FoundationError::InvalidTransition)
        );
        drop(one);
        assert!(registry.acquire_execution(task.id, rev).is_ok());
    }

    #[test]
    fn recover_never_replays_pending_effects() {
        let registry = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let task = registry.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        registry.start(task.id).unwrap();
        registry
            .propose_tool(
                task.id,
                "external.send",
                Value::Null,
                ToolEffect::ExternalSideEffect,
            )
            .unwrap();
        let snapshot = registry.export_snapshot().unwrap();
        let restored =
            TaskRegistry::restore_snapshot(&snapshot, Arc::new(EventJournal::new(32))).unwrap();
        let record = restored.get(task.id).unwrap();
        assert_eq!(record.status, TaskStatus::Paused);
        assert!(record.pending.is_none());
        assert!(restored.start(task.id).is_ok());
    }
}
