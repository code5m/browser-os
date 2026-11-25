//! Production Agent task lifecycle boundary.
//! Plain chat and project-scoped Coding share the official Foundation TaskRegistry
//! state machine and a single process-level task registry. Model transport and
//! business tools remain in their separately approved owner adapters.
//! Terminal tasks are retired; paused Coding tasks are detached after
//! producing a host checkpoint. The Coding session stores that checkpoint.
use browseros_agent_foundation::{
    context::ContextRef,
    event::EventJournal,
    task::{TaskRecord, TaskRegistry, TaskStatus},
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

/// The native Run receipt owns only encrypted task/approval metadata. Text
/// history is still provided by the authenticated chat UI; no credentials,
/// provider responses or user prompts are put in this journal.
#[derive(serde::Serialize, serde::Deserialize)]
struct NativeRunReceipt {
    id: Uuid,
    phase: String,
    task: TaskRecord,
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
struct DurableRun {
    id: Uuid,
    checkpoint: super::coding_checkpoint::RunCheckpoint,
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
impl DurableRun {
    fn persist(&self, phase: &str, task: TaskRecord) -> Result<(), String> {
        let receipt = NativeRunReceipt { id: self.id, phase: phase.into(), task };
        self.checkpoint.write(&serde_json::to_vec(&receipt).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?)
    }
    fn reject_existing(&self) -> Result<(), String> {
        let Some(bytes) = self.checkpoint.read()? else { return Ok(()); };
        let mut record: NativeRunReceipt = serde_json::from_slice(&bytes).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
        if record.id != self.id { return Err("AGENT_RUN_RECEIPT_INVALID".into()); }
        // Obtaining this OS lock excludes the previous cooperating host owner.
        // It does NOT prove that an external effect failed or may be replayed.
        if matches!(record.task.status, TaskStatus::Running | TaskStatus::AwaitingApproval) {
            record.task.revision = record.task.revision.checked_add(1).ok_or("AGENT_TASK_REVISION_EXHAUSTED")?;
            record.task.status = TaskStatus::Paused;
            record.task.pending = None;
            self.persist("interrupted", record.task)?;
        }
        Err("AGENT_RUN_ALREADY_RECORDED".into())
    }
}

pub(crate) struct ProductionExecution {
    tasks: &'static TaskRegistry,
    id: Uuid,
    finished: bool,
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    durable_run: Option<DurableRun>,
}

impl ProductionExecution {
    pub(crate) fn begin(kind: &'static str) -> Result<Self, String> {
        Self::begin_scoped(kind, Uuid::nil(), Uuid::new_v4(), vec![])
    }

    /// One-shot production entrypoints persist a Run before provider/tool
    /// dispatch. A repeated UUID cannot dispatch a second attempt, even
    /// after restart. A crash leaves an authenticated interrupted record.
    pub(crate) fn begin_durable(storage: &Path, kind: &'static str, run: Uuid,
        workspace: Uuid, session: Uuid, context: Vec<ContextRef>) -> Result<Self, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let checkpoint = super::coding_checkpoint::RunCheckpoint::acquire(storage, run)?;
            let durable = DurableRun { id: run, checkpoint };
            durable.reject_existing()?;
            let mut execution = Self::begin_scoped(kind, workspace, session, context)?;
            durable.persist("running", execution.record()?)?;
            execution.durable_run = Some(durable);
            Ok(execution)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, kind, run, workspace, session, context); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    pub(crate) fn checkpoint(&self, phase: &str) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if let Some(run) = &self.durable_run {
            run.persist(phase, self.record()?)?;
        }
        let _ = phase;
        Ok(())
    }

    /// Resume one Coding Task across separate Runs under its native session
    /// owner. The encrypted checkpoint is the authority across restarts.
    pub(crate) fn resume_coding(root: &Path, session_id: &str, previous: Option<&TaskRecord>) -> Result<Self, String> {
        let (workspace, session) = coding_scope(root, Some(session_id))?.ok_or("AGENT_SESSION_REQUIRED")?;
        if let Some(record) = previous {
            if record.workspace_id != workspace || record.session_id != session
                || record.goal != "coding" || !record.context.is_empty()
            { return Err("AGENT_TASK_SCOPE_MISMATCH".into()); }
            match record.status {
                TaskStatus::Paused => {
                    let tasks = task_owner();
                    tasks.restore_paused_record(record.clone()).map_err(|_| "AGENT_TASK_RESTORE_FAILED")?;
                    if tasks.start(record.id).is_err() {
                        let _ = tasks.detach_paused_record(record.id);
                        return Err("AGENT_TASK_START_FAILED".into());
                    }
                    return Ok(Self { tasks, id: record.id, finished: false,
                        #[cfg(any(target_os = "linux", target_os = "windows"))]
                        durable_run: None });
                }
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {}
                _ => return Err("AGENT_TASK_RECOVERY_REQUIRED".into()),
            }
        }
        Self::begin_scoped("coding", workspace, session, vec![])
    }

    /// Model and external tool adapters share the Foundation task's
    /// execution lease, in addition to durable session ownership.
    pub(crate) fn acquire_lease(&self) -> Result<browseros_agent_foundation::task::ExecutionLease<'static>, String> {
        let record = self.record()?;
        let tasks: &'static TaskRegistry = self.tasks;
        tasks.acquire_execution(self.id, record.revision).map_err(|_| "AGENT_TASK_EXECUTION_BUSY".into())
    }

    pub(crate) fn record(&self) -> Result<TaskRecord, String> {
        self.tasks.get(self.id).map_err(|_| "AGENT_TASK_LOOKUP_FAILED".into())
    }

    /// A completed provider response can still mean a paused Task.
    pub(crate) fn settle_coding(mut self, status: &str) -> Result<TaskRecord, String> {
        match status {
            "review" | "approval_required" | "step_limit" | "interrupted"
                | "operation_unknown" | "operation_executing" | "operation_review" | "ready" =>
                self.tasks.pause(self.id),
            "complete" | "applied" => self.tasks.finish(self.id, true),
            "cancelled" => self.tasks.cancel(self.id),
            _ => self.tasks.finish(self.id, false),
        }.map_err(|_| "AGENT_TASK_SETTLE_FAILED")?;
        let checkpoint = self.record()?;
        if checkpoint.status == TaskStatus::Paused {
            self.tasks.detach_paused_record(self.id).map_err(|_| "AGENT_TASK_DETACH_FAILED")?;
        } else {
            self.tasks.retire_terminal(self.id).map_err(|_| "AGENT_TASK_RETIRE_FAILED")?;
        }
        self.finished = true;
        Ok(checkpoint)
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
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            durable_run: None,
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
        self.checkpoint("cancelled")?;
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
        self.checkpoint(if success { "completed" } else { "failed" })?;
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

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn durable_one_shot_run_cannot_be_replayed_after_drop() {
        let storage = std::env::temp_dir().join(format!("browseros-agent-run-{}", Uuid::new_v4()));
        let id = Uuid::new_v4();
        let execution = ProductionExecution::begin_durable(&storage, "chat", id, Uuid::nil(), id, vec![]).unwrap();
        let task = execution.id();
        drop(execution); // simulate lost terminal publication; disk remains running.
        assert!(ProductionExecution::begin_durable(&storage, "chat", id, Uuid::nil(), id, vec![]).is_err());
        let checkpoint = super::super::coding_checkpoint::RunCheckpoint::acquire(&storage, id).unwrap();
        let bytes = checkpoint.read().unwrap().unwrap();
        let recovered: NativeRunReceipt = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(recovered.task.id, task);
        assert_eq!(recovered.task.status, TaskStatus::Paused);
        assert_eq!(recovered.phase, "interrupted");
        drop(checkpoint);
        let _ = std::fs::remove_dir_all(storage);
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
