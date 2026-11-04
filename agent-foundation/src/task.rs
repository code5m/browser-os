//! Independent task state machine. No Tauri commands, networking or file writes.
//! Trust boundary: resolve_approval MUST only be called by an authenticated host UI.
//! Never expose this method through a model-callable tool or untrusted RPC.
#[cfg(feature = "durable-sqlite")]
use crate::durable::SqliteTaskStore;
use crate::{
    context::{validate_context_list, ContextRef},
    event::{EventJournal, EventKind, EventRecord},
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
    #[serde(default)]
    pub workspace_id: Uuid,
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
    pub(crate) binding: Option<crate::approval::ApprovalBinding>,
}

pub struct TaskRegistry {
    tasks: Mutex<HashMap<Uuid, TaskRecord>>,
    signals: Mutex<HashMap<Uuid, watch::Sender<bool>>>,
    journal: Arc<EventJournal>,
    execution_leases: Mutex<HashSet<Uuid>>,
    #[cfg(feature = "durable-sqlite")]
    durable: Option<Arc<SqliteTaskStore>>,
}

/// Prevents concurrent model/tool execution for one task across all adapters.
/// Dropping the lease releases it (including early errors and cancellation).
pub(crate) struct ExecutionLease<'a> {
    registry: &'a TaskRegistry,
    task_id: Uuid,
    #[cfg(feature = "durable-sqlite")]
    db_lease: Option<Uuid>,
}

impl Drop for ExecutionLease<'_> {
    fn drop(&mut self) {
        #[cfg(feature = "durable-sqlite")]
        if let (Some(store), Some(token)) = (&self.registry.durable, self.db_lease) {
            store.release_lease(self.task_id, token);
        }
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
            #[cfg(feature = "durable-sqlite")]
            durable: None,
        }
    }

    #[cfg(feature = "durable-sqlite")]
    pub fn new_durable(store: Arc<SqliteTaskStore>, journal: Arc<EventJournal>) -> Result<Self> {
        let records = store.recover_paused()?;
        let mut registry = Self::new(journal.clone());
        for record in records {
            let (sender, _) = watch::channel(false);
            registry
                .signals
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(record.id, sender);
            journal.publish_committed(&store.recent_events(record.id, 1024)?);
            registry
                .tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(record.id, record);
        }
        registry.durable = Some(store);
        Ok(registry)
    }

    fn persist(
        &self,
        next: &TaskRecord,
        expected: Option<u64>,
        kinds: &[EventKind],
    ) -> Result<Vec<EventRecord>> {
        #[cfg(feature = "durable-sqlite")]
        if let Some(store) = &self.durable {
            return store.commit(next, expected, kinds);
        }
        let _ = (next, expected, kinds);
        Ok(Vec::new())
    }

    fn publish(&self, task_id: Uuid, kinds: &[EventKind], committed: Vec<EventRecord>) {
        #[cfg(feature = "durable-sqlite")]
        if self.durable.is_some() {
            self.journal.publish_committed(&committed);
            return;
        }
        let _ = committed;
        for kind in kinds {
            self.journal.append(task_id, kind.clone());
        }
    }

    /// Legacy in-memory/testing task: no trusted workspace, so host-signed
    /// approval is deliberately unavailable. Prefer create_scoped in production.
    pub fn create(
        &self,
        session_id: Uuid,
        goal: impl Into<String>,
        context: Vec<ContextRef>,
    ) -> Result<TaskRecord> {
        self.create_scoped(Uuid::nil(), session_id, goal, context)
    }

    pub fn create_scoped(
        &self,
        workspace_id: Uuid,
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
            workspace_id,
            goal,
            context,
            status: TaskStatus::Ready,
            revision: 0,
            pending: None,
        };
        let events = [EventKind::TaskCreated];
        let persisted = self.persist(&record, None, &events)?;
        self.tasks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(record.id, record.clone());
        let (sender, _) = watch::channel(false);
        self.signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(record.id, sender);
        self.publish(record.id, &events, persisted);
        Ok(record)
    }

    /// Retire a completed/cancelled in-memory execution to avoid leaking task
    /// records in the global production host. Durable tasks must never be pruned
    /// from the local map without a matching committed store transaction.
    pub fn retire_terminal(&self, id: Uuid) -> Result<()> {
        #[cfg(feature = "durable-sqlite")]
        if self.durable.is_some() {
            return Err(FoundationError::InvalidTransition);
        }
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let task = tasks.get(&id).ok_or(FoundationError::NotFound)?;
        if !task.status.is_terminal() {
            return Err(FoundationError::InvalidTransition);
        }
        tasks.remove(&id);
        drop(tasks);
        self.signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        self.execution_leases
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        // The task has retired from the live registry; its terminal event is
        // kept only within a finite replay window, not for process lifetime.
        self.journal.retire_task(id);
        Ok(())
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
        if leases.contains(&id) {
            return Err(FoundationError::InvalidTransition);
        }
        #[cfg(feature = "durable-sqlite")]
        let db_lease = self
            .durable
            .as_ref()
            .map(|store| store.acquire_lease(id, revision))
            .transpose()?;
        leases.insert(id);
        Ok(ExecutionLease {
            registry: self,
            task_id: id,
            #[cfg(feature = "durable-sqlite")]
            db_lease,
        })
    }

    pub fn start(&self, id: Uuid) -> Result<()> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let previous = tasks.get(&id).ok_or(FoundationError::NotFound)?.clone();
        if !matches!(previous.status, TaskStatus::Ready | TaskStatus::Paused) {
            return Err(FoundationError::InvalidTransition);
        }
        let mut next = previous.clone();
        next.status = TaskStatus::Running;
        next.revision += 1;
        let events = [EventKind::StateChanged {
            from: previous.status,
            to: next.status,
        }];
        let committed = self.persist(&next, Some(previous.revision), &events)?;
        tasks.insert(id, next);
        // Publish the new generation's signal before exposing Running; pause
        // and cancel use the same task -> signals lock order.
        let (sender, _) = watch::channel(false);
        self.signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, sender);
        self.publish(id, &events, committed);
        drop(tasks);
        Ok(())
    }

    pub fn pause(&self, id: Uuid) -> Result<()> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let previous = tasks.get(&id).ok_or(FoundationError::NotFound)?.clone();
        if previous.status != TaskStatus::Running {
            return Err(FoundationError::InvalidTransition);
        }
        let mut next = previous.clone();
        next.status = TaskStatus::Paused;
        next.revision += 1;
        let events = [EventKind::StateChanged {
            from: previous.status,
            to: next.status,
        }];
        let committed = self.persist(&next, Some(previous.revision), &events)?;
        tasks.insert(id, next);
        self.signal_cancel(id);
        self.publish(id, &events, committed);
        drop(tasks);
        Ok(())
    }

    pub fn cancel(&self, id: Uuid) -> Result<()> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let previous = tasks.get(&id).ok_or(FoundationError::NotFound)?.clone();
        if previous.status.is_terminal() {
            return Err(FoundationError::InvalidTransition);
        }
        let mut next = previous.clone();
        next.status = TaskStatus::Cancelled;
        next.pending = None;
        next.revision += 1;
        let events = [EventKind::StateChanged {
            from: previous.status,
            to: next.status,
        }];
        let committed = self.persist(&next, Some(previous.revision), &events)?;
        tasks.insert(id, next);
        self.signal_cancel(id);
        self.publish(id, &events, committed);
        drop(tasks);
        Ok(())
    }

    pub fn finish(&self, id: Uuid, success: bool) -> Result<()> {
        self.finish_checked(id, success, None)
    }

    pub(crate) fn finish_at_revision(&self, id: Uuid, success: bool, revision: u64) -> Result<()> {
        self.finish_checked(id, success, Some(revision))
    }

    fn finish_checked(&self, id: Uuid, success: bool, revision: Option<u64>) -> Result<()> {
        let status = if success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let previous = tasks.get(&id).ok_or(FoundationError::NotFound)?.clone();
        if previous.status != TaskStatus::Running
            || previous.pending.is_some()
            || revision.is_some_and(|expected| previous.revision != expected)
        {
            return Err(FoundationError::InvalidTransition);
        }
        let mut next = previous.clone();
        next.status = status;
        next.revision += 1;
        let events = [EventKind::StateChanged {
            from: previous.status,
            to: next.status,
        }];
        let committed = self.persist(&next, Some(previous.revision), &events)?;
        tasks.insert(id, next);
        self.signal_cancel(id);
        self.publish(id, &events, committed);
        drop(tasks);
        Ok(())
    }

    pub(crate) fn propose_tool(
        &self,
        id: Uuid,
        tool_name: impl Into<String>,
        arguments: Value,
        effect: ToolEffect,
    ) -> Result<Uuid> {
        self.propose_tool_checked(id, tool_name, arguments, effect, None)
    }

    pub(crate) fn propose_tool_at_revision(
        &self,
        id: Uuid,
        tool_name: impl Into<String>,
        arguments: Value,
        effect: ToolEffect,
        revision: u64,
    ) -> Result<Uuid> {
        self.propose_tool_checked(id, tool_name, arguments, effect, Some(revision))
    }

    fn propose_tool_checked(
        &self,
        id: Uuid,
        tool_name: impl Into<String>,
        arguments: Value,
        effect: ToolEffect,
        revision: Option<u64>,
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
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let previous = tasks.get(&id).ok_or(FoundationError::NotFound)?.clone();
        if previous.status != TaskStatus::Running
            || previous.pending.is_some()
            || revision.is_some_and(|expected| previous.revision != expected)
        {
            return Err(FoundationError::InvalidTransition);
        }
        let mut next = previous.clone();
        next.pending = Some(pending);
        next.status = TaskStatus::AwaitingApproval;
        next.revision += 1;
        let events = [
            EventKind::StateChanged {
                from: previous.status,
                to: next.status,
            },
            EventKind::ApprovalRequested {
                call_id,
                tool_name: name,
            },
        ];
        let committed = self.persist(&next, Some(previous.revision), &events)?;
        tasks.insert(id, next);
        self.publish(id, &events, committed);
        drop(tasks);
        Ok(call_id)
    }

    /// This is a trusted-host function, NEVER a model/tool RPC.
    /// Approval is scoped to exact (task, call) and consumes the pending operation.
    pub(crate) fn resolve_approval(
        &self,
        id: Uuid,
        call_id: Uuid,
        accepted: bool,
    ) -> Result<Option<ApprovedOperation>> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let previous = tasks.get(&id).ok_or(FoundationError::NotFound)?.clone();
        if previous.status != TaskStatus::AwaitingApproval {
            return Err(FoundationError::InvalidTransition);
        }
        let pending = previous
            .pending
            .clone()
            .ok_or(FoundationError::ApprovalMismatch)?;
        if pending.call_id != call_id {
            return Err(FoundationError::ApprovalMismatch);
        }
        let mut next = previous.clone();
        next.pending = None;
        next.status = TaskStatus::Running;
        next.revision += 1;
        let events = [
            EventKind::ApprovalResolved { call_id, accepted },
            EventKind::StateChanged {
                from: previous.status,
                to: next.status,
            },
        ];
        let committed = self.persist(&next, Some(previous.revision), &events)?;
        let revision = next.revision;
        tasks.insert(id, next);
        self.publish(id, &events, committed);
        drop(tasks);
        if accepted {
            Ok(Some(ApprovedOperation {
                task_id: id,
                revision,
                operation: pending,
                binding: None,
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

    #[cfg(feature = "experimental-rig-run")]
    pub(crate) fn with_active_revision<T>(
        &self,
        id: Uuid,
        revision: u64,
        action: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let task = tasks.get(&id).ok_or(FoundationError::NotFound)?;
        if task.status != TaskStatus::Running || task.revision != revision {
            return Err(FoundationError::InvalidTransition);
        }
        action()
    }

    /// Store metadata-only progress for the still-active task generation.
    pub(crate) fn record_event_at_revision(
        &self,
        task_id: Uuid,
        revision: u64,
        kind: EventKind,
    ) -> Result<()> {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let task = tasks.get(&task_id).ok_or(FoundationError::NotFound)?;
        if task.status != TaskStatus::Running || task.revision != revision {
            return Err(FoundationError::Cancelled);
        }
        let kinds = [kind];
        let committed = self.persist(task, Some(task.revision), &kinds)?;
        self.publish(task_id, &kinds, committed);
        drop(tasks);
        Ok(())
    }

    #[cfg(feature = "durable-sqlite")]
    pub(crate) fn durable_store(&self) -> Option<&Arc<SqliteTaskStore>> {
        self.durable.as_ref()
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
    fn stale_generation_cannot_finish_or_record_progress_after_resume() {
        let journal = Arc::new(EventJournal::new(32));
        let registry = TaskRegistry::new(journal.clone());
        let task = registry.create(Uuid::new_v4(), "race", vec![]).unwrap();
        registry.start(task.id).unwrap();
        let old_revision = registry.get(task.id).unwrap().revision;
        registry.pause(task.id).unwrap();
        registry.start(task.id).unwrap();
        assert_eq!(
            registry
                .finish_at_revision(task.id, true, old_revision)
                .err(),
            Some(FoundationError::InvalidTransition)
        );
        assert_eq!(
            registry
                .record_event_at_revision(
                    task.id,
                    old_revision,
                    EventKind::ModelProgress { bytes: 1 }
                )
                .err(),
            Some(FoundationError::Cancelled)
        );
        assert_eq!(
            registry
                .record_event_at_revision(
                    task.id,
                    old_revision,
                    EventKind::ToolFinished {
                        call_id: Uuid::new_v4(),
                        succeeded: true
                    }
                )
                .err(),
            Some(FoundationError::Cancelled)
        );
        assert_eq!(
            registry
                .propose_tool_at_revision(
                    task.id,
                    "editor.read",
                    Value::Null,
                    ToolEffect::ReadOnly,
                    old_revision
                )
                .err(),
            Some(FoundationError::InvalidTransition)
        );
        assert_eq!(registry.get(task.id).unwrap().status, TaskStatus::Running);
        assert!(!journal
            .replay(task.id, 0)
            .unwrap()
            .iter()
            .any(|event| matches!(&event.kind, EventKind::ModelProgress { .. })));
    }

    #[test]
    fn pause_resume_race_keeps_current_cancellation_signal_consistent() {
        let journal = Arc::new(EventJournal::new(1024));
        let registry = Arc::new(TaskRegistry::new(journal.clone()));
        let task = registry.create(Uuid::new_v4(), "race", vec![]).unwrap();
        registry.start(task.id).unwrap();
        for _ in 0..500 {
            let prior = registry.cancellation(task.id).unwrap();
            std::thread::scope(|scope| {
                let pausing = registry.clone();
                scope.spawn(move || pausing.pause(task.id).unwrap());
                let resuming = registry.clone();
                scope.spawn(move || loop {
                    if resuming.get(task.id).unwrap().status == TaskStatus::Paused {
                        resuming.start(task.id).unwrap();
                        break;
                    }
                    std::thread::yield_now();
                });
            });
            assert!(*prior.borrow(), "the paused generation must be cancelled");
            assert_eq!(registry.get(task.id).unwrap().status, TaskStatus::Running);
            assert!(
                !*registry.cancellation(task.id).unwrap().borrow(),
                "the resumed generation must remain active"
            );
        }
        let current = registry.cancellation(task.id).unwrap();
        registry.cancel(task.id).unwrap();
        assert!(
            *current.borrow(),
            "the current generation must be cancelled"
        );
        let mut status = TaskStatus::Ready;
        for event in journal.replay(task.id, 0).unwrap() {
            if let EventKind::StateChanged { from, to } = event.kind {
                assert_eq!(
                    from, status,
                    "published task transitions must remain in commit order"
                );
                status = to;
            }
        }
        assert_eq!(status, TaskStatus::Cancelled);
    }

    #[test]
    fn retirement_keeps_recent_terminal_replay_but_bounds_history() {
        let journal = Arc::new(EventJournal::new(4));
        let registry = TaskRegistry::new(journal.clone());
        let mut ids = Vec::new();
        for _ in 0..280 {
            let task = registry
                .create(Uuid::new_v4(), "short execution", vec![])
                .unwrap();
            registry.start(task.id).unwrap();
            registry.finish(task.id, true).unwrap();
            registry.retire_terminal(task.id).unwrap();
            ids.push(task.id);
        }
        assert!(journal.replay(ids[0], 0).unwrap().is_empty());
        assert!(journal.replay(*ids.last().unwrap(), 0).is_ok());
        assert!(registry.get(ids[0]).is_err());
    }

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
