//! Production Agent task lifecycle boundary.
//! Plain chat and project-scoped Coding share the official Foundation TaskRegistry
//! state machine and a single process-level task registry. Model transport and
//! business tools remain in their separately approved owner adapters.
//! Terminal tasks are retired; paused Coding tasks are detached after
//! producing a host checkpoint. The Coding session stores that checkpoint.
use browseros_agent_foundation::{
    approval::{ApprovalClaims, HostApprovalDecision, HostApprovalIssuer, ResourceBinding},
    context::{ContextGrant, ContextKind, ContextRef},
    event::EventJournal,
    task::{ApprovedOperation, TaskRecord, TaskRegistry, TaskStatus},
    tool::ToolEffect,
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

/// Text-only runs keep their exact normalized provider input and eligible
/// result under the same encrypted owner as task/approval metadata. Never
/// serialize the ProviderProfile credential snapshot into this record.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct TextRunPayload {
    input: serde_json::Value,
    result: Option<serde_json::Value>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct EffectEvidence {
    request_digest: String,
    target: Option<String>,
    tool: Option<String>,
    disposition: String,
    response_digest: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NativeRunReceipt {
    id: Uuid,
    phase: String,
    task: TaskRecord,
    #[serde(default)]
    approval: Option<ApprovalClaims>,
    #[serde(default)]
    text: Option<TextRunPayload>,
    #[serde(default)]
    effect: Option<EffectEvidence>,
    #[serde(default)]
    created_at_ms: u64,
    #[serde(default)]
    updated_at_ms: u64,
}
fn unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_millis().min(u64::MAX as u128) as u64
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
struct DurableRun {
    id: Uuid,
    checkpoint: super::coding_checkpoint::RunCheckpoint,
    approval: Option<ApprovalClaims>,
    text: Option<TextRunPayload>,
    effect: Option<EffectEvidence>,
    created_at_ms: u64,
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
impl DurableRun {
    fn new(storage: &Path, id: Uuid) -> Result<Self, String> {
        Ok(Self { id, checkpoint: super::coding_checkpoint::RunCheckpoint::acquire(storage, id)?,
            approval: None, text: None, effect: None, created_at_ms: unix_ms() })
    }
    fn persist(&self, phase: &str, task: TaskRecord) -> Result<(), String> {
        let receipt = NativeRunReceipt {
            id: self.id, phase: phase.into(), task, approval: self.approval.clone(),
            text: self.text.clone(), effect: self.effect.clone(),
            created_at_ms: self.created_at_ms, updated_at_ms: unix_ms(),
        };
        self.checkpoint.write(&serde_json::to_vec(&receipt).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?)
    }
    fn recover_existing(&self) -> Result<Option<NativeRunReceipt>, String> {
        let Some(bytes) = self.checkpoint.read()? else { return Ok(None); };
        let mut record: NativeRunReceipt = serde_json::from_slice(&bytes).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
        if record.id != self.id { return Err("AGENT_RUN_RECEIPT_INVALID".into()); }
        // The OS lock excludes the old cooperating owner, not an already
        // dispatched remote side effect. Approval is evidence, never a
        // reusable execution permit.
        if matches!(record.task.status, TaskStatus::Running | TaskStatus::AwaitingApproval) {
            record.task.revision = record.task.revision.checked_add(1).ok_or("AGENT_TASK_REVISION_EXHAUSTED")?;
            record.task.status = TaskStatus::Paused;
            record.task.pending = None;
            let effect_possible = record.approval.as_ref().is_some_and(|claim|
                claim.accepted && matches!(claim.effect, ToolEffect::ExecuteProcess | ToolEffect::ExternalSideEffect));
            record.phase = if effect_possible { "operation_unknown" } else { "interrupted" }.into();
            if let Some(effect) = &mut record.effect { effect.disposition = "unknown".into(); }
            record.updated_at_ms = unix_ms();
            self.checkpoint.write(&serde_json::to_vec(&record).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?)?;
        }
        Ok(Some(record))
    }
    fn reject_existing(&self) -> Result<(), String> {
        if self.recover_existing()?.is_some() { Err("AGENT_RUN_ALREADY_RECORDED".into()) }
        else { Ok(()) }
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
            let durable = DurableRun::new(storage, run)?;
            durable.reject_existing()?;
            let mut execution = Self::begin_scoped(kind, workspace, session, context)?;
            durable.persist("running", execution.record()?)?;
            execution.durable_run = Some(durable);
            Ok(execution)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, kind, run, workspace, session, context); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    /// A retry with the same request UUID and exact normalized input can
    /// recover a committed text-only result. It never reruns an interrupted
    /// or ambiguous provider dispatch. A different input under an existing
    /// UUID is rejected even when the old result is available.
    pub(crate) fn run_checkpointed_text(
        storage: &Path, kind: &'static str, run: Uuid, input: serde_json::Value,
        cancelled: impl Fn() -> bool,
        provider: impl FnOnce() -> Result<serde_json::Value, String>,
    ) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if !matches!(kind, "chat" | "legacy_agent" | "local_chat") || !input.is_object()
                || serde_json::to_vec(&input).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?.len() > 128 * 1024
            {
                return Err("AGENT_RUN_INPUT_LIMIT".into());
            }
            if cancelled() { return Err("AI_PROVIDER_CANCELLED".into()); }
            let mut durable = DurableRun::new(storage, run)?;
            if let Some(previous) = durable.recover_existing()? {
                if previous.task.goal != kind || previous.task.session_id != run || !previous.task.workspace_id.is_nil() {
                    return Err("AGENT_RUN_SCOPE_MISMATCH".into());
                }
                let text = previous.text.ok_or("AGENT_RUN_RECOVERY_REQUIRED")?;
                if text.input != input { return Err("AGENT_RUN_INPUT_MISMATCH".into()); }
                if previous.phase == "completed" && previous.task.status == TaskStatus::Completed {
                    return text.result.ok_or_else(|| "AGENT_RUN_RESULT_UNAVAILABLE".into());
                }
                return Err("AGENT_RUN_RECOVERY_REQUIRED".into());
            }
            durable.text = Some(TextRunPayload { input, result: None });
            let mut execution = Self::begin_scoped(kind, Uuid::nil(), run, vec![])?;
            durable.persist("prepared", execution.record()?)?;
            execution.durable_run = Some(durable);
            if cancelled() {
                execution.cancel()?;
                return Err("AI_PROVIDER_CANCELLED".into());
            }
            let lease = execution.acquire_lease()?;
            execution.checkpoint("running")?;
            let response = provider();
            drop(lease);
            if cancelled() {
                execution.cancel()?;
                return Err("AI_PROVIDER_CANCELLED".into());
            }
            if let Ok(value) = &response {
                let encoded = serde_json::to_string(value).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
                // The provider adapter has already redacted its known
                // credential. Do not retain additional suspected secrets
                // or oversized output in a recovery record.
                if encoded.len() <= 512 * 1024 && !crate::security_policy::contains_credential_leak(&encoded) {
                    if let Some(payload) = execution.durable_run.as_mut().and_then(|run| run.text.as_mut()) {
                        payload.result = Some(value.clone());
                    }
                }
            }
            // Complete the authenticated publication before handing the
            // successful answer back to the UI. A lost IPC response can
            // recover this result without a second provider invocation.
            execution.finish(response.is_ok())?;
            response
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, kind, run, input, cancelled, provider); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    /// Keep bounded, encrypted evidence for manual MCP. The receipt records
    /// what the transport reported, not a claim of exactly-once remote effects.
    pub(crate) fn checkpoint_manual_mcp(&mut self, digest: String, request: &serde_json::Value) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let run = self.durable_run.as_mut().ok_or("AGENT_RUN_RECEIPT_REQUIRED")?;
            if !run.approval.as_ref().is_some_and(|claim| claim.accepted) {
                return Err("AGENT_APPROVAL_REQUIRED".into());
            }
            let safe_label = |field: &str| request[field].as_str().filter(|value|
                !value.is_empty() && value.len() <= 256 && !crate::security_policy::contains_credential_leak(value))
                .map(str::to_owned);
            run.effect = Some(EffectEvidence { request_digest: digest.clone(), target: safe_label("server_id"),
                tool: safe_label("tool"), disposition: "dispatching".into(), response_digest: None });
        }
        let _ = (digest, request);
        self.checkpoint("dispatching")
    }

    pub(crate) fn record_manual_mcp_return(&mut self, result: &Result<serde_json::Value, String>, reported_success: bool) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let effect = self.durable_run.as_mut().and_then(|run| run.effect.as_mut())
                .ok_or("AGENT_RUN_RECEIPT_REQUIRED")?;
            effect.disposition = if reported_success { "transport_returned" } else { "unknown" }.into();
            effect.response_digest = result.as_ref().ok().and_then(|value| serde_json::to_vec(value).ok())
                .map(|bytes| format!("{:x}", Sha256::digest(&bytes)));
        }
        let _ = (result, reported_success);
        self.checkpoint(if reported_success { "transport_returned" } else { "operation_unknown" })
    }

    pub(crate) fn checkpoint(&self, phase: &str) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if let Some(run) = &self.durable_run {
            run.persist(phase, self.record()?)?;
        }
        let _ = phase;
        Ok(())
    }

    pub(crate) fn begin_workspace_run(storage: &Path, kind: &'static str, root: &Path, run: Uuid) -> Result<Self, String> {
        let canonical = root.canonicalize().map_err(|_| "AGENT_ROOT_INVALID")?;
        if !canonical.is_dir() { return Err("AGENT_ROOT_INVALID".into()); }
        let (workspace, session) = coding_scope(&canonical, Some(&run.to_string()))?.ok_or("AGENT_SESSION_REQUIRED")?;
        let context = ContextRef::checked(ContextKind::Project, format!("workspace:{workspace}"), ContextGrant::ApprovedScope)
            .map_err(|_| "AGENT_TASK_SCOPE_MISMATCH")?;
        Self::begin_durable(storage, kind, run, workspace, session, vec![context])
    }

    pub(crate) fn finish_unknown(mut self) -> Result<(), String> {
        self.tasks.pause(self.id).map_err(|_| "AGENT_TASK_SETTLE_FAILED")?;
        self.checkpoint("operation_unknown")?;
        self.tasks.detach_paused_record(self.id).map_err(|_| "AGENT_TASK_DETACH_FAILED")?;
        self.finished = true;
        Ok(())
    }

    pub(crate) fn checkpoint_approval(&mut self, claim: ApprovalClaims) -> Result<(), String> {
        let task = self.record()?;
        if !claim.accepted || claim.task_id != task.id
            || claim.workspace_id != task.workspace_id || claim.session_id != task.session_id
            || claim.task_revision.checked_add(1) != Some(task.revision)
            || task.status != TaskStatus::Running
        { return Err("AGENT_APPROVAL_STALE".into()); }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if let Some(run) = &mut self.durable_run { run.approval = Some(claim); }
        self.checkpoint("approved")
    }

    /// Only callable by an already-authorized main-window native owner.
    /// Native workspace owners still validate the actual resource and
    /// persist their own effect intent before dispatch.
    pub(crate) fn approve_native(&self, tool: &str, arguments: serde_json::Value,
        effect: ToolEffect, revision: String) -> Result<(ApprovalClaims, ApprovedOperation), String> {
        use ring::rand::{SecureRandom, SystemRandom};
        let record = self.record()?;
        let resource = ResourceBinding { resource_id: format!("workspace:{}", record.workspace_id),
            resource_revision: revision };
        let mut secret = [0u8; 32];
        SystemRandom::new().fill(&mut secret).map_err(|_| "AGENT_APPROVAL_RANDOM_FAILED")?;
        let issuer = HostApprovalIssuer::new(secret).map_err(|_| "AGENT_APPROVAL_FAILED")?;
        let call = issuer.propose_reviewed_native(self.tasks, self.id, record.revision,
            tool, arguments, effect).map_err(|_| "AGENT_APPROVAL_FAILED")?;
        let (receipt, permit) = issuer.resolve_local_decision(self.tasks, self.id, call,
            HostApprovalDecision { principal_id: Uuid::new_v4(), resource, accepted: true,
                ttl: std::time::Duration::from_secs(120) })
            .map_err(|_| "AGENT_APPROVAL_FAILED")?;
        Ok((receipt.claims, permit.ok_or("AGENT_APPROVAL_DENIED")?))
    }
    pub(crate) fn acquire_approved_native(&self, permit: ApprovedOperation,
        claim: &ApprovalClaims, arguments: &serde_json::Value)
        -> Result<browseros_agent_foundation::task::ExecutionLease<'static>, String> {
        if claim.task_id != self.id { return Err("AGENT_APPROVAL_STALE".into()); }
        permit.claim_native_owner(self.tasks, claim, arguments)
            .map_err(|_| "AGENT_APPROVAL_STALE".into())
    }

    /// Resume one Coding Task across separate Runs under its native session
    /// owner. The encrypted checkpoint is the authority across restarts.
    pub(crate) fn resume_coding(root: &Path, session_id: &str, previous: Option<&TaskRecord>) -> Result<Self, String> {
        let (workspace, session) = coding_scope(root, Some(session_id))?.ok_or("AGENT_SESSION_REQUIRED")?;
        let context = vec![ContextRef::checked(ContextKind::Project,
            format!("workspace:{workspace}"), ContextGrant::ApprovedScope)
            .map_err(|_| "AGENT_TASK_SCOPE_MISMATCH")?];
        if let Some(record) = previous {
            if record.workspace_id != workspace || record.session_id != session
                || record.goal != "coding" || (!record.context.is_empty() && record.context != context)
            { return Err("AGENT_TASK_SCOPE_MISMATCH".into()); }
            match record.status {
                TaskStatus::Paused => {
                    let tasks = task_owner();
                    let mut restored = record.clone();
                    // Old authenticated paused checkpoints have no workspace
                    // reference. Add only the deterministic current owner scope.
                    restored.context = context.clone();
                    tasks.restore_paused_record(restored).map_err(|_| "AGENT_TASK_RESTORE_FAILED")?;
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
        Self::begin_scoped("coding", workspace, session, context)
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
        if !matches!(kind, "chat" | "coding" | "legacy_agent" | "agent_channel" | "manual_mcp" | "local_chat") {
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

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn text_run_recovers_exact_result_without_provider_replay() {
        let storage = std::env::temp_dir().join(format!("browseros-text-receipt-{}", Uuid::new_v4()));
        let id = Uuid::new_v4();
        let input = serde_json::json!({"model":"fixture","messages":[{"role":"user","content":"hello"}]});
        let expected = serde_json::json!({"content":"answer","model":"fixture"});
        assert_eq!(ProductionExecution::run_checkpointed_text(&storage, "chat", id, input.clone(),
            || false, || Ok(expected.clone())).unwrap(), expected);
        assert_eq!(ProductionExecution::run_checkpointed_text(&storage, "chat", id, input,
            || false, || panic!("a committed response must not contact the provider")).unwrap(), expected);
        assert!(ProductionExecution::run_checkpointed_text(&storage, "chat", id,
            serde_json::json!({"model":"other"}), || false, || panic!("mismatched input must not dispatch")).is_err());
        let raw = std::fs::read(storage.join("agent-run-receipts").join(format!("{id}.run"))).unwrap();
        assert!(!raw.windows(b"hello".len()).any(|bytes| bytes == b"hello"));
        std::fs::remove_dir_all(storage).unwrap();
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn interrupted_text_run_retains_context_but_never_replays() {
        let storage = std::env::temp_dir().join(format!("browseros-text-interrupted-{}", Uuid::new_v4()));
        let id = Uuid::new_v4();
        let input = serde_json::json!({"model":"fixture"});
        let mut durable = DurableRun::new(&storage, id).unwrap();
        durable.text = Some(TextRunPayload { input: input.clone(), result: None });
        let execution = ProductionExecution::begin_scoped("chat", Uuid::nil(), id, vec![]).unwrap();
        durable.persist("running", execution.record().unwrap()).unwrap();
        drop(execution);
        drop(durable);
        assert_eq!(ProductionExecution::run_checkpointed_text(&storage, "chat", id, input,
            || false, || panic!("unknown dispatch may not replay")).unwrap_err(), "AGENT_RUN_RECOVERY_REQUIRED");
        let receipt = DurableRun::new(&storage, id).unwrap().recover_existing().unwrap().unwrap();
        assert_eq!(receipt.task.status, TaskStatus::Paused);
        assert_eq!(receipt.phase, "interrupted");
        assert!(receipt.text.is_some());
        std::fs::remove_dir_all(storage).unwrap();
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
