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
use serde_json::Value;
use std::{
    path::Path,
    sync::{Arc, OnceLock},
};
use uuid::Uuid;
mod channel;
pub(crate) use channel::{ChannelAdmission, ChannelInput, ChannelStart};

/// Revalidate the original manual approval binding without recreating a
/// live approval or calling an external service.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn manual_approval_matches_request(claim: &ApprovalClaims, run: Uuid, digest: &str) -> bool {
    let args = serde_json::json!({"run_id":run,"review_digest":digest});
    browseros_agent_foundation::approval::approval_argument_digest(&claim.tool_name, &args, &claim.effect)
        .ok().as_deref() == Some(claim.arguments_sha256.as_str())
}

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

/// Bounded input and eligible result under the same encrypted owner as
/// task/approval metadata. The historical field name is "text"; consumers
/// must classify the Task goal before interpreting the typed payload.
/// Never serialize a ProviderProfile credential or reusable permit.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct TextRunPayload {
    input: serde_json::Value,
    result: Option<serde_json::Value>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct TextSuccessor {
    id: Uuid,
    input: Option<serde_json::Value>,
    resume: bool,
}
/// New manual MCP records bind the native workspace BEFORE any approval
/// or adapter dispatch. Absence is legacy history, not new proof.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ManualAdmission {
    workspace_identity: String,
}

// Transport completion is not proof of an external effect. Require the
// protocol result for the actual method and refuse malformed error flags.
pub(super) fn mcp_transport_reported_success(action: &str, response: &Value) -> bool {
    let Some(result) = response.get("result").and_then(Value::as_object) else { return false; };
    if !matches!(result.get("isError"), None | Some(Value::Bool(false))) {
        return false;
    }
    let expected = match action {
        "mcp_tools" => "tools",
        "mcp_call" => "content",
        _ => return false,
    };
    result.get(expected).is_some_and(Value::is_array)
}

// Leave headroom for approval, identity and digest metadata under the
// existing 64 KiB immutable Run archive limit. Only the adapter's scrubbed
// protocol return is eligible, never raw server credentials or unknown
// results. A withheld result keeps its digest, not an empty success payload.
const MANUAL_RESPONSE_LIMIT: usize = 32 * 1024;
fn manual_response_eligible(approved_tool: &str, value: &Value, encoded: &[u8]) -> bool {
    let action = match approved_tool {
        "coding.manual_mcp_tools" => "mcp_tools",
        "coding.manual_mcp_call" => "mcp_call",
        _ => return false,
    };
    encoded.len() <= MANUAL_RESPONSE_LIMIT
        && value["untrusted"] == true
        && mcp_transport_reported_success(action, value)
        && std::str::from_utf8(encoded).is_ok_and(|text|
            !crate::security_policy::contains_credential_leak(text))
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct EffectEvidence {
    request_digest: String,
    target: Option<String>,
    tool: Option<String>,
    disposition: String,
    response_digest: Option<String>,
    // Legacy receipts may have a digest without retrievable content.
    // Content is kept inside the same authenticated private Run record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    response_payload: Option<Value>,
    #[serde(default)]
    workspace_identity: Option<String>,
    #[serde(default)]
    review_revision: Option<String>,
}
/// Shared native reconciliation evidence for Coding and manual MCP. A user
/// attestation is not a provider-side idempotency or termination receipt.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct EffectReview {
    observation: String,
    evidence: String,
    recorded_at_ms: u64,
    workspace_identity_bound: bool,
    #[serde(default)]
    execution_ended_attested: bool,
    #[serde(default)]
    reviewed_revision: Option<String>,
}
impl EffectReview {
    pub(super) fn checked(observation: &str, evidence: &str, bound: bool,
        ended: bool, revision: String, limit: usize) -> Result<Self, String> {
        if !matches!(observation, "observed_applied" | "observed_not_applied" | "unresolved")
            || evidence.trim().is_empty() || evidence.len() > limit
            || crate::security_policy::contains_credential_leak(evidence)
            || (ended && observation == "unresolved") {
            return Err("AGENT_EFFECT_EVIDENCE_INVALID".into());
        }
        Ok(Self { observation: observation.into(), evidence: evidence.trim().into(),
            recorded_at_ms: unix_ms(), workspace_identity_bound: bound,
            execution_ended_attested: ended, reviewed_revision: Some(revision) })
    }
}
pub(super) fn workspace_identity(root: &Path) -> Result<String, String> {
    let identity = crate::capabilities::workspace::safe_paths::directory_identity(root)
        .map_err(|_| "AGENT_WORKSPACE_IDENTITY_UNAVAILABLE")?;
    Ok(format!("{:x}", Sha256::digest(identity.as_bytes())))
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
    #[serde(default)]
    previous_run: Option<Uuid>,
    #[serde(default)]
    successor: Option<TextSuccessor>,
    #[serde(default)]
    payload_forgotten: bool,
    #[serde(default)]
    effect_reviews: Vec<EffectReview>,
    #[serde(default)]
    effect_resolution: Option<EffectReview>,
    #[serde(default)]
    manual_admission: Option<ManualAdmission>,
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
    manual_admission: Option<ManualAdmission>,
    created_at_ms: u64,
    previous_run: Option<Uuid>,
    _predecessor: Option<Box<DurableRun>>,
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
impl DurableRun {
    fn new(storage: &Path, id: Uuid) -> Result<Self, String> {
        Ok(Self { id, checkpoint: super::coding_checkpoint::RunCheckpoint::acquire(storage, id)?,
            approval: None, text: None, effect: None, manual_admission: None, created_at_ms: unix_ms(),
            previous_run: None, _predecessor: None })
    }
    fn persist(&self, phase: &str, task: TaskRecord) -> Result<(), String> {
        let receipt = NativeRunReceipt {
            id: self.id, phase: phase.into(), task, approval: self.approval.clone(),
            text: self.text.clone(), effect: self.effect.clone(),
            created_at_ms: self.created_at_ms, updated_at_ms: unix_ms(),
            previous_run: self.previous_run, successor: None, payload_forgotten: false,
            effect_reviews: Vec::new(), effect_resolution: None,
            manual_admission: self.manual_admission.clone(),
        };
        self.write_record(&receipt)
    }
    fn write_record(&self, receipt: &NativeRunReceipt) -> Result<(), String> {
        if receipt.id != self.id { return Err("AGENT_RUN_RECEIPT_INVALID".into()); }
        self.checkpoint.write(&serde_json::to_vec(receipt).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?)
    }
    fn lookup(storage: &Path, id: Uuid) -> Result<Self, String> {
        if !super::coding_checkpoint::RunCheckpoint::record_exists(storage, id)? {
            return Err("AGENT_RUN_NOT_FOUND".into());
        }
        Self::new(storage, id)
    }
    fn read_record(&self) -> Result<Option<NativeRunReceipt>, String> {
        let Some(bytes) = self.checkpoint.read()? else { return Ok(None); };
        let record: NativeRunReceipt = serde_json::from_slice(&bytes).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
        if record.id != self.id { return Err("AGENT_RUN_RECEIPT_INVALID".into()); }
        if let Some(effect) = &record.effect {
            if let Some(value) = &effect.response_payload {
                let bytes = serde_json::to_vec(value).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
                let digest = format!("{:x}", Sha256::digest(&bytes));
                if !Self::manual_admission_bound(&record)
                    || effect.disposition != "transport_returned"
                    || effect.response_digest.as_deref() != Some(digest.as_str())
                    || !record.approval.as_ref().is_some_and(|claim|
                        claim.accepted && claim.task_id == record.task.id
                            && claim.workspace_id == record.task.workspace_id
                            && claim.session_id == record.task.session_id
                            && claim.resource.resource_revision == effect.request_digest
                            && claim.effect == ToolEffect::ExternalSideEffect
                            && manual_response_eligible(&claim.tool_name, value, &bytes))
                {
                    return Err("AGENT_RUN_RECEIPT_INVALID".into());
                }
            }
        }
        Ok(Some(record))
    }
    // These checks describe the original encrypted native receipt, not
    // an untrusted caller-supplied assertion or proof of remote application.
    fn manual_admission_bound(record: &NativeRunReceipt) -> bool {
        record.task.goal == "manual_mcp" && record.task.session_id == record.id
            && record.text.is_none() && record.successor.is_none()
            && record.manual_admission.as_ref().is_some_and(|a| !a.workspace_identity.is_empty())
    }
    fn manual_transport_proven(record: &NativeRunReceipt) -> bool {
        Self::manual_admission_bound(record) && record.phase == "transport_returned"
            && record.approval.as_ref().zip(record.effect.as_ref()).is_some_and(|(claim, effect)| {
                claim.accepted && claim.task_id == record.task.id
                    && claim.workspace_id == record.task.workspace_id
                    && claim.session_id == record.task.session_id
                    && claim.task_revision.checked_add(1) == Some(record.task.revision)
                    && claim.effect == ToolEffect::ExternalSideEffect
                    && matches!(claim.tool_name.as_str(), "coding.manual_mcp_call" | "coding.manual_mcp_tools")
                    && claim.resource.resource_revision == effect.request_digest
                    && claim.resource.resource_id == format!("workspace:{}", record.task.workspace_id)
                    && manual_approval_matches_request(claim, record.id, &effect.request_digest)
                    && effect.workspace_identity.as_deref() == record.manual_admission.as_ref().map(|a| a.workspace_identity.as_str())
                    && effect.disposition == "transport_returned"
                    && effect.response_digest.as_ref().is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
            })
    }
    fn recover_record(&self, mut record: NativeRunReceipt) -> Result<NativeRunReceipt, String> {
        // The OS lock excludes the old cooperating owner, not an already
        // dispatched remote side effect. Approval is evidence, never a
        // reusable execution permit.
        if matches!(record.task.status, TaskStatus::Running | TaskStatus::AwaitingApproval) {
            // The adapter is reachable only after publishing dispatch intent.
            // An authenticated NEW admission with no intent is a positive
            // no-dispatch receipt. A persisted successful protocol return
            // also survives a crash before the final Task transition.
            let not_dispatched = Self::manual_admission_bound(&record)
                && record.effect.is_none()
                && matches!(record.phase.as_str(), "running" | "approved");
            let returned = Self::manual_transport_proven(&record);
            record.task.revision = record.task.revision.checked_add(1).ok_or("AGENT_TASK_REVISION_EXHAUSTED")?;
            record.task.pending = None;
            if not_dispatched {
                record.task.status = TaskStatus::Cancelled;
                record.phase = "not_dispatched".into();
            } else if returned {
                record.task.status = TaskStatus::Completed;
                record.phase = "completed".into();
                // Keep disposition transport_returned. It does not assert
                // that the remote business effect applied or has quiesced.
            } else {
                record.task.status = TaskStatus::Paused;
                let effect_possible = record.effect.is_some() || record.approval.as_ref().is_some_and(|claim|
                    claim.accepted && matches!(claim.effect, ToolEffect::ExecuteProcess | ToolEffect::ExternalSideEffect));
                record.phase = if effect_possible { "operation_unknown" } else { "interrupted" }.into();
                if let Some(effect) = &mut record.effect {
                    effect.disposition = "unknown".into();
                    // Never expose even a retained transport payload when
                    // the enclosing execution proof cannot be reconciled.
                    effect.response_payload = None;
                }
            }
            record.updated_at_ms = unix_ms();
            self.write_record(&record)?;
        }
        Ok(record)
    }
    fn recover_existing(&self) -> Result<Option<NativeRunReceipt>, String> {
        self.read_record()?.map(|record| self.recover_record(record)).transpose()
    }
    fn reject_existing(&self) -> Result<(), String> {
        if self.recover_existing()?.is_some() { Err("AGENT_RUN_ALREADY_RECORDED".into()) }
        else { Ok(()) }
    }

    /// A durable successor is the only proof that a paused historical Run
    /// has relinquished continuation. A reservation alone is not proof.
    /// Both records are authenticated and the caller holds their OS locks.
    fn verify_successor(record: &NativeRunReceipt, child: &NativeRunReceipt) -> Result<(), String> {
        let next = record.successor.as_ref().ok_or("AGENT_RUN_SUCCESSOR_PENDING")?;
        if next.id == record.id || child.id != next.id || child.previous_run != Some(record.id)
            || child.task.goal != record.task.goal || child.task.session_id != record.task.session_id
            || child.task.workspace_id != record.task.workspace_id || child.task.context != record.task.context
            || record.effect.is_some() || child.effect.is_some()
            || child.approval.as_ref().is_some_and(|claim| claim.effect != ToolEffect::ReadOnly)
            || (next.resume && (child.task.id != record.task.id || child.task.revision <= record.task.revision))
            || (!next.resume && (record.task.goal != "chat" || child.task.id == record.task.id))
        {
            return Err("AGENT_RUN_SCOPE_MISMATCH".into());
        }
        if let Some(reserved) = &next.input {
            match &child.text {
                Some(payload) if &payload.input == reserved => {},
                None if child.payload_forgotten => {}, // authenticated compacted identity
                _ => return Err("AGENT_RUN_INPUT_MISMATCH".into()),
            }
        }
        if child.text.is_none() && !child.payload_forgotten {
            return Err("AGENT_RUN_SUCCESSOR_PENDING".into());
        }
        Ok(())
    }

    fn can_compact_payload(record: &NativeRunReceipt) -> bool {
        matches!(record.task.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled)
            || (record.task.status == TaskStatus::Paused
                && matches!(record.phase.as_str(), "interrupted" | "superseded")
                && record.successor.as_ref().is_some_and(|next| next.resume))
    }

    /// Shared text/Channel retention policy. Retain the old Task snapshot,
    /// Run identity, approval metadata and successor link. Mark the old Run
    /// superseded only after authenticating its independently durable child.
    /// This is NOT a Task completion transition.
    fn compact_payload(&self, storage: &Path, mut record: NativeRunReceipt) -> Result<bool, String> {
        if !Self::can_compact_payload(&record) { return Err("AGENT_RUN_NOT_TERMINAL".into()); }
        if record.payload_forgotten && record.text.is_none()
            && record.successor.as_ref().map_or(true, |next| next.input.is_none())
        {
            return Ok(self.checkpoint.archive_terminal().is_ok());
        }
        if let Some(next) = &record.successor {
            if next.id == record.id { return Err("AGENT_RUN_SCOPE_MISMATCH".into()); }
            let child_owner = Self::lookup(storage, next.id).map_err(|error|
                if error == "AGENT_RUN_NOT_FOUND" { "AGENT_RUN_SUCCESSOR_PENDING".into() } else { error })?;
            let child = child_owner.read_record()?.ok_or("AGENT_RUN_SUCCESSOR_PENDING")?;
            Self::verify_successor(&record, &child)?;
            // Keep the child lock through predecessor publication. The
            // durable child identity is never deleted by this protocol.
            if let Some(next) = &mut record.successor { next.input = None; }
            if record.task.status == TaskStatus::Paused { record.phase = "superseded".into(); }
            record.text = None;
            record.payload_forgotten = true;
            record.updated_at_ms = unix_ms();
            self.write_record(&record)?;
        } else {
            record.text = None;
            record.payload_forgotten = true;
            record.updated_at_ms = unix_ms();
            self.write_record(&record)?;
        }
        Ok(self.checkpoint.archive_terminal().is_ok())
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
            let durable = DurableRun::new(storage, run)?;
            if let Some(previous) = durable.read_record()? {
                if previous.task.goal != kind || previous.task.session_id != run || !previous.task.workspace_id.is_nil() {
                    return Err("AGENT_RUN_SCOPE_MISMATCH".into());
                }
                let previous = durable.recover_record(previous)?;
                let text = previous.text.ok_or("AGENT_RUN_RECOVERY_REQUIRED")?;
                if text.input != input { return Err("AGENT_RUN_INPUT_MISMATCH".into()); }
                if previous.phase == "completed" && previous.task.status == TaskStatus::Completed {
                    return text.result.ok_or_else(|| "AGENT_RUN_RESULT_UNAVAILABLE".into());
                }
                return Err("AGENT_RUN_RECOVERY_REQUIRED".into());
            }
            let execution = Self::begin_scoped(kind, Uuid::nil(), run, vec![])?;
            Self::execute_text_attempt(durable, execution, input, cancelled, provider)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, kind, run, input, cancelled, provider); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn execute_text_attempt(mut durable: DurableRun, mut execution: Self, input: serde_json::Value,
        cancelled: impl Fn() -> bool, provider: impl FnOnce() -> Result<serde_json::Value, String>)
        -> Result<serde_json::Value, String>
    {
        durable.text = Some(TextRunPayload { input, result: None });
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

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn text_kind(record: &NativeRunReceipt) -> Result<&'static str, String> {
        if !record.task.workspace_id.is_nil() || !record.task.context.is_empty()
            || record.task.pending.is_some() || record.approval.is_some() || record.effect.is_some()
        { return Err("AGENT_RUN_SCOPE_MISMATCH".into()); }
        match record.task.goal.as_str() {
            "chat" => Ok("chat"), "legacy_agent" => Ok("legacy_agent"),
            "local_chat" => Ok("local_chat"), _ => Err("AGENT_RUN_SCOPE_MISMATCH".into()),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn text_summary(record: &NativeRunReceipt) -> serde_json::Value {
        let interrupted = record.task.status == TaskStatus::Paused && record.phase == "interrupted";
        let completed = record.task.status == TaskStatus::Completed && record.phase == "completed";
        let has_result = record.text.as_ref().and_then(|text| text.result.as_ref()).is_some();
        serde_json::json!({
            "id":record.id,"kind":record.task.goal,"phase":record.phase,
            "task_id":record.task.id,"session_id":record.task.session_id,
            "previous_run":record.previous_run,"successor":record.successor.as_ref().map(|next| next.id),
            "reserved_input_available":record.successor.as_ref().is_some_and(|next| next.input.is_some()),
            "created_at_ms":record.created_at_ms,"updated_at_ms":record.updated_at_ms,
            "has_payload":record.text.is_some(),"has_result":has_result,"forgotten":record.payload_forgotten,
            "can_resume":interrupted && record.text.is_some() && record.successor.is_none(),
            "can_continue":completed && record.task.goal == "chat" && has_result && record.successor.is_none(),
            "can_forget":DurableRun::can_compact_payload(record)
        })
    }

    pub(crate) fn text_run_catalog(storage: &Path, after: Option<Uuid>) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let (ids, next) = super::coding_checkpoint::RunCheckpoint::catalog_page(storage, after)?;
            let mut records = Vec::new();
            let mut unavailable = 0;
            for id in ids {
                // Authenticate and classify before any recovery write. The
                // project-free catalog never changes another owner's records.
                let result = (|| -> Result<Option<serde_json::Value>, String> {
                    let run = DurableRun::lookup(storage, id)?;
                    let record = run.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
                    if Self::text_kind(&record).is_err() { return Ok(None); }
                    Ok(Some(Self::text_summary(&run.recover_record(record)?)))
                })();
                match result {
                    Ok(Some(record)) => records.push(record),
                    Ok(None) => {},
                    Err(_) => unavailable += 1,
                }
            }
            Ok(serde_json::json!({"runs":records,"next_cursor":next,"unavailable":unavailable}))
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, after); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    pub(crate) fn read_text_run(storage: &Path, id: Uuid) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            Self::text_kind(&record)?;
            let record = owner.recover_record(record)?;
            let mut response = Self::text_summary(&record);
            let mut messages = Vec::new();
            if let Some(text) = &record.text {
                if let Some(turns) = text.input["messages"].as_array() {
                    for turn in turns {
                        if matches!(turn["role"].as_str(), Some("user" | "assistant")) {
                            if let Some(content) = turn["content"].as_str() {
                                messages.push(serde_json::json!({"role":turn["role"],"content":content}));
                            }
                        }
                    }
                } else if let Some(content) = text.input["message"].as_str() {
                    messages.push(serde_json::json!({"role":"user","content":content}));
                }
                if let Some(content) = text.result.as_ref().and_then(|value| value["content"].as_str()) {
                    messages.push(serde_json::json!({"role":"assistant","content":content}));
                }
                response["profile_id"] = text.input["profile_id"].clone();
                response["model"] = text.input["model"].clone();
            }
            response["messages"] = serde_json::json!(messages);
            response["archived"] = serde_json::json!(owner.checkpoint.archive_confirmed()?);
            Ok(response)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, id); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn next_chat_input(text: &TextRunPayload, message: &str) -> Result<serde_json::Value, String> {
        use serde_json::{json, Value};
        if message.trim().is_empty() || message.len() > 24_000
            || crate::security_policy::contains_credential_leak(message)
        { return Err("AI_MESSAGE_INVALID".into()); }
        let turns = text.input["messages"].as_array().ok_or("AGENT_RUN_CONTEXT_INVALID")?;
        let system = turns.first().filter(|turn| turn["role"] == "system" && turn["content"].is_string())
            .ok_or("AGENT_RUN_CONTEXT_INVALID")?;
        let answer = text.result.as_ref().and_then(|value| value["content"].as_str())
            .ok_or("AGENT_RUN_RESULT_UNAVAILABLE")?;
        let mut prior = turns.iter().skip(1).cloned().collect::<Vec<Value>>();
        prior.push(json!({"role":"assistant","content":answer}));
        let mut selected = Vec::new();
        let mut remaining = 16_000usize;
        for turn in prior.iter().rev().take(12) {
            let role = turn["role"].as_str().ok_or("AGENT_RUN_CONTEXT_INVALID")?;
            let content = turn["content"].as_str().ok_or("AGENT_RUN_CONTEXT_INVALID")?;
            if !matches!(role, "user" | "assistant") { return Err("AGENT_RUN_CONTEXT_INVALID".into()); }
            let mut end = content.len().min(remaining).min(3000);
            while !content.is_char_boundary(end) { end -= 1; }
            if end == 0 { break; }
            let truncated = &content[..end];
            if crate::security_policy::contains_credential_leak(truncated) { return Err("AI_HISTORY_INVALID".into()); }
            if !truncated.trim().is_empty() {
                selected.push(json!({"role":role,"content":truncated}));
                remaining -= end;
            }
        }
        selected.reverse();
        selected.insert(0, system.clone());
        selected.push(json!({"role":"user","content":message}));
        let mut input = text.input.clone();
        input["messages"] = json!(selected);
        Ok(input)
    }

    /// The predecessor holds a single durable successor reservation before
    /// dispatch. Retrying a reserved child never creates another Run.
    pub(crate) fn advance_text<P>(
        storage: &Path, previous_id: Uuid, run: Uuid, message: &str,
        cancelled: impl Fn() -> bool,
        prepare: impl FnOnce(&str, &serde_json::Value) -> Result<P, String>,
    ) -> Result<serde_json::Value, String>
    where P: FnOnce() -> Result<serde_json::Value, String>
    {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if previous_id == run || run.is_nil() || cancelled() {
                return Err("AGENT_RUN_ADVANCE_INVALID".into());
            }
            let predecessor = DurableRun::lookup(storage, previous_id)?;
            let record = predecessor.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            let kind = Self::text_kind(&record)?;
            let mut record = predecessor.recover_record(record)?;
            if record.successor.as_ref().is_some_and(|next| next.id != run) {
                return Err("AGENT_RUN_SUCCESSOR_EXISTS".into());
            }
            let mut child = DurableRun::new(storage, run)?;
            let existing_child = child.read_record()?;
            if let Some(next) = &record.successor {
                if next.id != run { return Err("AGENT_RUN_SUCCESSOR_EXISTS".into()); }
                if !message.is_empty() && (next.resume || next.input.as_ref()
                    .and_then(|input| input["messages"].as_array())
                    .and_then(|turns| turns.last())
                    .and_then(|turn| turn["content"].as_str()) != Some(message)) {
                    return Err("AGENT_RUN_INPUT_MISMATCH".into());
                }
                if let Some(existing) = existing_child {
                    if Self::text_kind(&existing)? != kind { return Err("AGENT_RUN_SCOPE_MISMATCH".into()); }
                    DurableRun::verify_successor(&record, &existing)?;
                    let existing = child.recover_record(existing)?;
                    if existing.phase == "completed" && existing.task.status == TaskStatus::Completed {
                        return existing.text.and_then(|text| text.result)
                            .ok_or_else(|| "AGENT_RUN_RESULT_UNAVAILABLE".into());
                    }
                    return Err("AGENT_RUN_RECOVERY_REQUIRED".into());
                }
            } else if existing_child.is_some() {
                return Err("AGENT_RUN_ALREADY_RECORDED".into());
            }
            let (input, resume) = if let Some(next) = &record.successor {
                (next.input.clone().ok_or("AGENT_RUN_RESULT_UNAVAILABLE")?, next.resume)
            } else {
                let text = record.text.as_ref().ok_or("AGENT_RUN_RESULT_UNAVAILABLE")?;
                if record.task.status == TaskStatus::Paused && record.phase == "interrupted" && message.is_empty() {
                    (text.input.clone(), true)
                } else if kind == "chat" && record.task.status == TaskStatus::Completed && record.phase == "completed" {
                    (Self::next_chat_input(text, message)?, false)
                } else { return Err("AGENT_RUN_NOT_RESUMABLE".into()); }
            };
            if serde_json::to_vec(&input).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?.len() > 128 * 1024 {
                return Err("AGENT_RUN_INPUT_LIMIT".into());
            }
            if (resume && (record.task.status != TaskStatus::Paused || record.phase != "interrupted"))
                || (!resume && (kind != "chat" || record.task.status != TaskStatus::Completed || record.phase != "completed"))
            { return Err("AGENT_RUN_NOT_RESUMABLE".into()); }
            let provider = prepare(kind, &input)?;
            if cancelled() { return Err("AI_PROVIDER_CANCELLED".into()); }
            if record.successor.is_none() {
                record.successor = Some(TextSuccessor { id: run, input: Some(input.clone()), resume });
                record.updated_at_ms = unix_ms();
                predecessor.write_record(&record)?;
            }
            child.previous_run = Some(previous_id);
            let execution = if resume {
                let tasks = task_owner();
                let task_id = record.task.id;
                tasks.restore_paused_record(record.task.clone()).map_err(|_| "AGENT_TASK_RESTORE_FAILED")?;
                if tasks.start(task_id).is_err() {
                    let _ = tasks.detach_paused_record(task_id);
                    return Err("AGENT_TASK_START_FAILED".into());
                }
                Self { tasks, id: task_id, finished: false, durable_run: None }
            } else {
                Self::begin_scoped(kind, Uuid::nil(), record.task.session_id, vec![])?
            };
            // The predecessor's OS lock spans the child attempt.
            Self::execute_text_attempt(child, execution, input, cancelled, provider)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, previous_id, run, message, cancelled, prepare); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    /// Explicit terminal payload compaction: preserve the replay identity
    /// and any unpublished reservation. Other records may retain copies.
    pub(crate) fn forget_text_payload(storage: &Path, id: Uuid) -> Result<bool, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            Self::text_kind(&record)?;
            let record = owner.recover_record(record)?;
            owner.compact_payload(storage, record)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, id); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    /// Keep bounded, encrypted evidence for manual MCP. The receipt records
    /// what the transport reported, not a claim of exactly-once remote effects.
    pub(crate) fn checkpoint_manual_mcp(&mut self, root: &Path, digest: String, request: &serde_json::Value) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let identity = workspace_identity(root)?;
            let actual_digest = format!("{:x}", Sha256::digest(serde_json::to_vec(request)
                .map_err(|_| "AGENT_OPERATION_INVALID")?));
            let tool = match request["action"].as_str() {
                Some("mcp_call") => "coding.manual_mcp_call",
                Some("mcp_tools") => "coding.manual_mcp_tools",
                _ => return Err("AGENT_OPERATION_INVALID".into()),
            };
            let task = self.record()?;
            let run = self.durable_run.as_mut().ok_or("AGENT_RUN_RECEIPT_REQUIRED")?;
            if run.effect.is_some() || run.manual_admission.as_ref().map(|a| a.workspace_identity.as_str()) != Some(identity.as_str())
                || actual_digest != digest
                || !run.approval.as_ref().is_some_and(|claim| claim.accepted
                    && claim.task_id == task.id && claim.workspace_id == task.workspace_id
                    && claim.session_id == task.session_id
                    && claim.task_revision.checked_add(1) == Some(task.revision)
                    && claim.effect == ToolEffect::ExternalSideEffect
                    && claim.tool_name == tool && claim.resource.resource_revision == digest
                    && claim.resource.resource_id == format!("workspace:{}", task.workspace_id)
                    && manual_approval_matches_request(claim, run.id, &digest)) {
                return Err("AGENT_APPROVAL_STALE".into());
            }
            let safe_label = |field: &str| request[field].as_str().filter(|value|
                !value.is_empty() && value.len() <= 256 && !crate::security_policy::contains_credential_leak(value))
                .map(str::to_owned);
            let target = safe_label("server_id").or_else(|| {
                request["server"]["command"].as_str()
                    .and_then(|command| Path::new(command).file_name())
                    .and_then(|name| name.to_str())
                    .filter(|name| name.len() <= 256 && !crate::security_policy::contains_credential_leak(name))
                    .map(str::to_owned)
            });
            run.effect = Some(EffectEvidence { request_digest: digest.clone(), target,
                tool: safe_label("tool"), disposition: "dispatching".into(), response_digest: None,
                response_payload: None, workspace_identity: Some(identity),
                review_revision: safe_label("review_revision") });
        }
        let _ = (root, digest, request);
        self.checkpoint("dispatching")
    }

    pub(crate) fn record_manual_mcp_return(&mut self, result: &Result<serde_json::Value, String>, reported_success: bool) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let run = self.durable_run.as_mut().ok_or("AGENT_RUN_RECEIPT_REQUIRED")?;
            let approved_tool = run.approval.as_ref().ok_or("AGENT_APPROVAL_REQUIRED")?.tool_name.as_str();
            let effect = run.effect.as_mut().ok_or("AGENT_RUN_RECEIPT_REQUIRED")?;
            if effect.disposition != "dispatching" { return Err("AGENT_RUN_ALREADY_RECORDED".into()); }
            let encoded = result.as_ref().ok().and_then(|value| serde_json::to_vec(value).ok());
            let response_digest = encoded.as_ref().map(|bytes| format!("{:x}", Sha256::digest(bytes)));
            if reported_success && response_digest.is_none() { return Err("AGENT_RUN_RECEIPT_INVALID".into()); }
            effect.disposition = if reported_success { "transport_returned" } else { "unknown" }.into();
            effect.response_digest = response_digest;
            effect.response_payload = if reported_success {
                result.as_ref().ok().zip(encoded.as_ref()).filter(|(value, bytes)|
                    manual_response_eligible(approved_tool, value, bytes))
                    .map(|(value, _)| value.clone())
            } else { None };
        }
        let _ = (result, reported_success);
        self.checkpoint(if reported_success { "transport_returned" } else { "operation_unknown" })
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn manual_scope(root: &Path) -> Result<(Uuid, String), String> {
        let canonical = root.canonicalize().map_err(|_| "AGENT_ROOT_INVALID")?;
        if !canonical.is_dir() { return Err("AGENT_ROOT_INVALID".into()); }
        let (workspace, _) = coding_scope(&canonical, Some(&Uuid::new_v4().to_string()))?
            .ok_or("AGENT_SESSION_REQUIRED")?;
        Ok((workspace, workspace_identity(&canonical)?))
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn check_manual_scope(record: &NativeRunReceipt, workspace: Uuid, identity: &str) -> Result<(), String> {
        if record.task.goal != "manual_mcp" || record.task.workspace_id != workspace
            || record.task.session_id != record.id || record.text.is_some() || record.successor.is_some()
            || record.manual_admission.as_ref().is_some_and(|a| a.workspace_identity != identity)
            || record.effect.as_ref().and_then(|effect| effect.workspace_identity.as_deref())
                .is_some_and(|bound| bound != identity) {
            return Err("AGENT_RUN_SCOPE_MISMATCH".into());
        }
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn can_resolve_manual(record: &NativeRunReceipt) -> bool {
        record.phase == "operation_unknown" && record.task.status == TaskStatus::Paused
            && record.task.pending.is_none() && record.effect_resolution.is_none()
            && record.effect.as_ref().and_then(|effect| effect.workspace_identity.as_ref()).is_some()
            && record.approval.as_ref().is_some_and(|claim| claim.accepted
                && claim.task_id == record.task.id && claim.workspace_id == record.task.workspace_id
                && claim.session_id == record.task.session_id && claim.effect == ToolEffect::ExternalSideEffect)
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn manual_summary(record: &NativeRunReceipt, detail: bool) -> serde_json::Value {
        let effect = record.effect.as_ref();
        let response = if record.phase == "completed" && record.task.status == TaskStatus::Completed {
            effect.filter(|value| value.disposition == "transport_returned")
                .and_then(|value| value.response_payload.as_ref())
        } else { None };
        let mut result = serde_json::json!({
            "id":record.id,"task_id":record.task.id,"phase":record.phase,
            "revision":record.task.revision.to_string(),"updated_at_ms":record.updated_at_ms,
            "tool":effect.and_then(|value| value.tool.as_deref()),
            "target":effect.and_then(|value| value.target.as_deref()),
            "disposition":effect.map(|value| value.disposition.as_str()),
            "identity_bound":record.manual_admission.is_some() || effect.and_then(|value| value.workspace_identity.as_ref()).is_some(),
            "observations":record.effect_reviews.len(),
            "has_response":response.is_some(),
            "can_reconcile":record.phase == "operation_unknown" && record.task.status == TaskStatus::Paused
                && record.effect_reviews.len() < 16,
            "can_resolve":Self::can_resolve_manual(record),
            "can_archive":matches!(record.task.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled),
        });
        if detail {
            result["request_digest"] = serde_json::json!(effect.map(|value| &value.request_digest));
            result["review_revision"] = serde_json::json!(effect.and_then(|value| value.review_revision.as_ref()));
            result["response_digest"] = serde_json::json!(effect.and_then(|value| value.response_digest.as_ref()));
            result["response_payload"] = serde_json::json!(response);
            result["reviews"] = serde_json::json!(&record.effect_reviews);
            result["resolution"] = serde_json::json!(&record.effect_resolution);
            result["approved_tool"] = serde_json::json!(record.approval.as_ref().map(|claim| &claim.tool_name));
        }
        result
    }

    pub(crate) fn manual_mcp_catalog(storage: &Path, root: &Path, after: Option<Uuid>) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let (workspace, identity) = Self::manual_scope(root)?;
            let (ids, next) = super::coding_checkpoint::RunCheckpoint::catalog_page(storage, after)?;
            let mut records = Vec::new();
            let mut unavailable = 0;
            for id in ids {
                let result = (|| -> Result<Option<serde_json::Value>, String> {
                    let owner = DurableRun::lookup(storage, id)?;
                    let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
                    if Self::check_manual_scope(&record, workspace, &identity).is_err() { return Ok(None); }
                    Ok(Some(Self::manual_summary(&owner.recover_record(record)?, false)))
                })();
                match result { Ok(Some(record)) => records.push(record), Ok(None) => {}, Err(_) => unavailable += 1 }
            }
            Ok(serde_json::json!({"runs":records,"next_cursor":next,"unavailable":unavailable}))
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, root, after); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    pub(crate) fn read_manual_mcp(storage: &Path, root: &Path, id: Uuid) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let (workspace, identity) = Self::manual_scope(root)?;
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            Self::check_manual_scope(&record, workspace, &identity)?;
            let mut summary = Self::manual_summary(&owner.recover_record(record)?, true);
            summary["archived"] = serde_json::json!(owner.checkpoint.archive_confirmed()?);
            Ok(summary)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, root, id); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    /// Manual evidence is an attestation, not an authoritative effect
    /// receipt or proof of termination of an escaped remote/child process.
    /// No result is sent to a model and no retry is authorized by this API.
    pub(crate) fn reconcile_manual_mcp(storage: &Path, root: &Path, id: Uuid, revision: u64,
        observation: &str, evidence: &str) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let (workspace, identity) = Self::manual_scope(root)?;
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            Self::check_manual_scope(&record, workspace, &identity)?;
            let mut record = owner.recover_record(record)?;
            if record.task.revision != revision || record.phase != "operation_unknown"
                || record.task.status != TaskStatus::Paused || record.effect_reviews.len() >= 16 {
                return Err("AGENT_EFFECT_REVIEW_STALE".into());
            }
            let bound = record.effect.as_ref().and_then(|effect| effect.workspace_identity.as_ref()).is_some();
            record.effect_reviews.push(EffectReview::checked(observation, evidence, bound,
                false, revision.to_string(), 2048)?);
            record.task.revision = record.task.revision.checked_add(1).ok_or("AGENT_TASK_REVISION_EXHAUSTED")?;
            record.updated_at_ms = unix_ms();
            owner.write_record(&record)?;
            Ok(Self::manual_summary(&record, true))
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, root, id, revision, observation, evidence); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    /// Explicitly close an unknown manual effect based on the user's checked
    /// termination and outcome evidence. The old Run remains permanently
    /// non-replayable. This is a terminal *attestation*, never automatic
    /// provider proof and never a fresh permission to dispatch.
    pub(crate) fn resolve_manual_mcp(storage: &Path, root: &Path, id: Uuid, revision: u64,
        observation: &str, evidence: &str, execution_ended: bool) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if !execution_ended || !matches!(observation, "observed_applied" | "observed_not_applied") {
                return Err("AGENT_RECONCILIATION_REQUIRED".into());
            }
            let canonical = root.canonicalize().map_err(|_| "AGENT_ROOT_INVALID")?;
            // Same lock order as the actual manual MCP dispatch.
            let _workspace = super::coding_checkpoint::workspace_owner(storage, &canonical)?;
            let (workspace, identity) = Self::manual_scope(&canonical)?;
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            Self::check_manual_scope(&record, workspace, &identity)?;
            let mut record = owner.recover_record(record)?;
            if record.task.revision != revision || !Self::can_resolve_manual(&record) {
                return Err("AGENT_EFFECT_REVIEW_STALE".into());
            }
            let resolution = EffectReview::checked(observation, evidence, true,
                true, revision.to_string(), 2048)?;
            let applied = observation == "observed_applied";
            // Paused Task has no live in-memory lease; the authenticated Run
            // is authoritative. Do not start a new Task/Run to reconcile it.
            record.task.revision = record.task.revision.checked_add(1).ok_or("AGENT_TASK_REVISION_EXHAUSTED")?;
            record.task.status = if applied { TaskStatus::Completed } else { TaskStatus::Failed };
            record.phase = if applied { "reconciled_applied" } else { "reconciled_not_applied" }.into();
            if let Some(effect) = &mut record.effect {
                effect.disposition = if applied { "user_attested_applied" } else { "user_attested_not_applied" }.into();
            }
            record.effect_resolution = Some(resolution);
            record.updated_at_ms = unix_ms();
            owner.write_record(&record)?;
            Ok(Self::manual_summary(&record, true))
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, root, id, revision, observation, evidence, execution_ended); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
    }

    /// Archive the entire authenticated terminal receipt without dropping
    /// approval, effect evidence, reviews or the original Run identity.
    pub(crate) fn archive_manual_mcp(storage: &Path, root: &Path, id: Uuid, revision: u64) -> Result<serde_json::Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let (workspace, identity) = Self::manual_scope(root)?;
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            Self::check_manual_scope(&record, workspace, &identity)?;
            let record = owner.recover_record(record)?;
            if record.task.revision != revision
                || !matches!(record.task.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled) {
                return Err("AGENT_RUN_NOT_TERMINAL".into());
            }
            owner.checkpoint.archive_terminal()?;
            let mut summary = Self::manual_summary(&record, true);
            summary["archived"] = serde_json::json!(true);
            Ok(summary)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, root, id, revision); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
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
        if kind != "manual_mcp" {
            return Self::begin_durable(storage, kind, run, workspace, session, vec![context]);
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let mut durable = DurableRun::new(storage, run)?;
            durable.reject_existing()?;
            durable.manual_admission = Some(ManualAdmission { workspace_identity: workspace_identity(&canonical)? });
            let mut execution = Self::begin_scoped(kind, workspace, session, vec![context])?;
            durable.persist("running", execution.record()?)?;
            execution.durable_run = Some(durable);
            Ok(execution)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        { let _ = (storage, run, workspace, session, context); Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into()) }
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
    /// A confirmed pre-dispatch cancellation is only legal while the new
    /// authenticated manual admission has no effect intent. Once armed,
    /// cancellation requests cannot turn uncertainty into non-application.
    pub(crate) fn cancel_manual_before_dispatch(self) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let run = self.durable_run.as_ref().ok_or("AGENT_RUN_RECEIPT_REQUIRED")?;
            let record = self.record()?;
            if record.goal != "manual_mcp" || record.session_id != run.id
                || run.manual_admission.is_none() || run.effect.is_some() {
                return Err("AGENT_RECONCILIATION_REQUIRED".into());
            }
            self.cancel_with_phase("not_dispatched")
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into())
    }
    pub(crate) fn cancel(self) -> Result<(), String> {
        self.cancel_with_phase("cancelled")
    }
    fn cancel_with_phase(mut self, phase: &str) -> Result<(), String> {
        self.tasks
            .cancel(self.id)
            .map_err(|_| "AGENT_TASK_CANCEL_FAILED")?;
        self.checkpoint(phase)?;
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

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn interrupted_text_successor_resumes_same_task_and_never_forks() {
        let storage = std::env::temp_dir().join(format!("browseros-text-resume-{}", Uuid::new_v4()));
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let input = serde_json::json!({"model":"fixture"});
        let mut durable = DurableRun::new(&storage, first).unwrap();
        durable.text = Some(TextRunPayload { input: input.clone(), result: None });
        let execution = ProductionExecution::begin_scoped("chat", Uuid::nil(), first, vec![]).unwrap();
        let task_id = execution.id();
        durable.persist("running", execution.record().unwrap()).unwrap();
        drop(execution);
        drop(durable);
        let answer = serde_json::json!({"content":"resumed"});
        assert!(ProductionExecution::forget_text_payload(&storage, first).is_err(),
            "an interrupted Run without a durable successor must retain its payload");
        // A reservation alone must not authorize compaction.
        {
            let predecessor = DurableRun::lookup(&storage, first).unwrap();
            let mut receipt = predecessor.recover_existing().unwrap().unwrap();
            receipt.successor = Some(TextSuccessor { id: second, input: Some(input.clone()), resume: true });
            predecessor.write_record(&receipt).unwrap();
        }
        assert_eq!(ProductionExecution::forget_text_payload(&storage, first).unwrap_err(), "AGENT_RUN_SUCCESSOR_PENDING");
        assert_eq!(ProductionExecution::read_text_run(&storage, first).unwrap()["has_payload"], true);
        assert_eq!(ProductionExecution::advance_text(&storage, first, second, "", || false, |kind, saved| {
            assert_eq!(kind, "chat"); assert_eq!(saved, &input);
            Ok(|| Ok(answer.clone()))
        }).unwrap(), answer);
        let recovered = ProductionExecution::read_text_run(&storage, second).unwrap();
        assert_eq!(recovered["task_id"], serde_json::json!(task_id));
        assert_eq!(recovered["session_id"], serde_json::json!(first));
        assert!(ProductionExecution::advance_text(&storage, first, Uuid::new_v4(), "", || false,
            |_,_| Ok(|| panic!("second successor must be rejected"))).is_err());
        assert_eq!(ProductionExecution::advance_text(&storage, first, second, "", || false,
            |_,_| Ok(|| panic!("committed successor must not replay"))).unwrap(), answer);
        assert!(ProductionExecution::forget_text_payload(&storage, first).unwrap());
        let archived = ProductionExecution::read_text_run(&storage, first).unwrap();
        assert_eq!(archived["phase"], "superseded");
        assert_eq!(archived["task_id"], serde_json::json!(task_id));
        assert_eq!(archived["successor"], serde_json::json!(second));
        assert_eq!(archived["archived"], true);
        assert_eq!(archived["has_payload"], false);
        assert_eq!(archived["can_resume"], false);
        assert_eq!(ProductionExecution::advance_text(&storage, first, second, "", || false,
            |_,_| Ok(|| panic!("compacted predecessor must only recover committed child"))).unwrap(), answer);
        ProductionExecution::forget_text_payload(&storage, second).unwrap();
        assert_eq!(ProductionExecution::read_text_run(&storage, second).unwrap()["forgotten"], true);
        assert!(ProductionExecution::advance_text(&storage, first, second, "", || false,
            |_,_| Ok(|| panic!("forgotten successor must not replay"))).is_err());
        std::fs::remove_dir_all(storage).unwrap();
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn manual_effect_observation_does_not_release_unknown_or_allow_replay() {
        let root = std::env::temp_dir().join(format!("browseros-mcp-review-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let storage = root.join("store");
        let run = Uuid::new_v4();
        let mut execution = ProductionExecution::begin_workspace_run(&storage, "manual_mcp", &root, run).unwrap();
        let request = serde_json::json!({"action":"mcp_call","tool":"fixture"});
        let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&request).unwrap()));
        let arguments = serde_json::json!({"run_id":run,"review_digest":digest});
        let (claim, permit) = execution.approve_native("coding.manual_mcp_call", arguments.clone(),
            ToolEffect::ExternalSideEffect, digest.clone()).unwrap();
        execution.checkpoint_approval(claim.clone()).unwrap();
        let lease = execution.acquire_approved_native(permit, &claim, &arguments).unwrap();
        assert!(manual_approval_matches_request(&claim, run, &digest));
        assert!(!manual_approval_matches_request(&claim, Uuid::new_v4(), &digest));
        assert!(!manual_approval_matches_request(&claim, run, "different"));
        execution.checkpoint_manual_mcp(&root, digest, &request).unwrap();
        drop(lease);
        execution.record_manual_mcp_return(&Err("MCP_TERMINATION_UNKNOWN".into()), false).unwrap();
        execution.finish_unknown().unwrap();
        let receipt = ProductionExecution::read_manual_mcp(&storage, &root, run).unwrap();
        let revision = receipt["revision"].as_str().unwrap().parse().unwrap();
        let reconciled = ProductionExecution::reconcile_manual_mcp(&storage, &root, run, revision,
            "observed_not_applied", "External record inspected").unwrap();
        assert_eq!(reconciled["phase"], "operation_unknown");
        assert_eq!(reconciled["reviews"][0]["observation"], "observed_not_applied");
        assert!(ProductionExecution::reconcile_manual_mcp(&storage, &root, run, revision,
            "observed_applied", "Stale observation").is_err());
        assert!(ProductionExecution::begin_workspace_run(&storage, "manual_mcp", &root, run).is_err());
        let current = reconciled["revision"].as_str().unwrap().parse().unwrap();
        assert!(ProductionExecution::archive_manual_mcp(&storage, &root, run, current).is_err());
        assert!(ProductionExecution::resolve_manual_mcp(&storage, &root, run, current,
            "observed_not_applied", "Termination unconfirmed", false).is_err());
        assert!(ProductionExecution::resolve_manual_mcp(&storage, &root, run, revision,
            "observed_not_applied", "Stale revision", true).is_err());
        let settled = ProductionExecution::resolve_manual_mcp(&storage, &root, run, current,
            "observed_not_applied", "Operator verified process ended and external record absent", true).unwrap();
        assert_eq!(settled["phase"], "reconciled_not_applied");
        assert_eq!(settled["resolution"]["execution_ended_attested"], true);
        assert_eq!(settled["can_resolve"], false);
        let terminal_revision = settled["revision"].as_str().unwrap().parse().unwrap();
        let archived = ProductionExecution::archive_manual_mcp(&storage, &root, run, terminal_revision).unwrap();
        assert_eq!(archived["archived"], true);
        assert_eq!(archived["reviews"], settled["reviews"]);
        assert_eq!(archived["resolution"], settled["resolution"]);
        assert!(ProductionExecution::begin_workspace_run(&storage, "manual_mcp", &root, run).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn manual_mcp_crash_boundaries_recover_only_authenticated_local_evidence() {
        let root = std::env::temp_dir().join(format!("browseros-mcp-crash-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let storage = root.join("state");
        for stage in 0..4 {
            let run = Uuid::new_v4();
            let mut execution = ProductionExecution::begin_workspace_run(&storage, "manual_mcp", &root, run).unwrap();
            let request = serde_json::json!({"action":"mcp_call","run_id":run,"tool":"fixture"});
            let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&request).unwrap()));
            if stage > 0 {
                let args = serde_json::json!({"run_id":run,"review_digest":digest});
                let (claim, permit) = execution.approve_native("coding.manual_mcp_call", args.clone(),
                    ToolEffect::ExternalSideEffect, digest.clone()).unwrap();
                execution.checkpoint_approval(claim.clone()).unwrap();
                if stage > 1 {
                    let lease = execution.acquire_approved_native(permit, &claim, &args).unwrap();
                    assert!(execution.checkpoint_manual_mcp(&root, "wrong".into(), &request).is_err());
                    execution.checkpoint_manual_mcp(&root, digest.clone(), &request).unwrap();
                    assert!(execution.checkpoint_manual_mcp(&root, digest, &request).is_err());
                    drop(lease);
                    if stage == 3 {
                        execution.record_manual_mcp_return(&Ok(serde_json::json!({
                            "result":{"content":[{"type":"text","text":"fixture return"}]},"untrusted":true
                        })), true).unwrap();
                    }
                }
            }
            drop(execution);
            let receipt = ProductionExecution::read_manual_mcp(&storage, &root, run).unwrap();
            assert_eq!(receipt["phase"], match stage { 0 | 1 => "not_dispatched", 2 => "operation_unknown", _ => "completed" });
            assert_eq!(receipt["identity_bound"], true);
            assert_eq!(receipt["can_archive"], stage != 2);
            if stage == 3 {
                assert_eq!(receipt["disposition"], "transport_returned");
                assert!(receipt["resolution"].is_null());
                assert_eq!(receipt["has_response"], true);
                assert_eq!(receipt["response_payload"]["result"]["content"][0]["text"], "fixture return");
                let revision = receipt["revision"].as_str().unwrap().parse().unwrap();
                let archived = ProductionExecution::archive_manual_mcp(&storage, &root, run, revision).unwrap();
                assert_eq!(archived["archived"], true);
                assert_eq!(archived["response_payload"], receipt["response_payload"]);
                let restored = ProductionExecution::read_manual_mcp(&storage, &root, run).unwrap();
                assert_eq!(restored["response_payload"], receipt["response_payload"]);
                let catalog = ProductionExecution::manual_mcp_catalog(&storage, &root, None).unwrap();
                let row = catalog["runs"].as_array().unwrap().iter().find(|row| row["id"] == serde_json::json!(run)).unwrap();
                assert_eq!(row["has_response"], true);
                assert!(row.get("response_payload").is_none());
            } else {
                assert_eq!(receipt["has_response"], false);
                assert!(receipt["response_payload"].is_null());
            }
            assert!(ProductionExecution::begin_workspace_run(&storage, "manual_mcp", &root, run).is_err());
        }
        // An older record without this admission marker is not silently
        // promoted to a no-dispatch/transport-return proof.
        let old = Uuid::new_v4();
        let (workspace, session) = coding_scope(&root, Some(&old.to_string())).unwrap().unwrap();
        let execution = ProductionExecution::begin_durable(&storage, "manual_mcp", old, workspace, session, vec![]).unwrap();
        drop(execution);
        assert_eq!(ProductionExecution::read_manual_mcp(&storage, &root, old).unwrap()["phase"], "interrupted");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_manual_response_requires_bounded_scrubbed_method_result() {
        let eligible = |tool: &str, value: Value| {
            manual_response_eligible(tool, &value, &serde_json::to_vec(&value).unwrap())
        };
        let normal = serde_json::json!({"result":{"content":[{"type":"text","text":"fixture return"}]},"untrusted":true});
        assert!(eligible("coding.manual_mcp_call", normal.clone()));
        assert!(!eligible("coding.manual_mcp_tools", normal));
        assert!(eligible("coding.manual_mcp_tools", serde_json::json!({"result":{"tools":[]},"untrusted":true})));
        for invalid in [
            serde_json::json!({"result":{"content":[]}}),
            serde_json::json!({"result":{"content":[],"isError":true},"untrusted":true}),
            serde_json::json!({"result":{"content":[{"type":"text","text":"password=fixture"}]},"untrusted":true}),
            serde_json::json!({"result":{"content":[{"type":"text","text":"x".repeat(MANUAL_RESPONSE_LIMIT)}]},"untrusted":true}),
        ] {
            assert!(!eligible("coding.manual_mcp_call", invalid));
        }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn registered_manual_mcp_cancel_is_bound_to_one_run_and_retains_receipt() {
        let root = std::env::temp_dir().join(format!("browseros-mcp-cancel-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let storage = root.join("state");
        let run = Uuid::new_v4();
        let unrelated = Uuid::new_v4();
        // Cancel may arrive before the blocking worker is registered.
        super::super::coding::cancel_run_request(&run.to_string()).unwrap();
        let guard = super::super::coding::register_run_request(&run.to_string()).unwrap();
        let other = super::super::coding::register_run_request(&unrelated.to_string()).unwrap();
        let request = serde_json::json!({"run_id":run,"action":"mcp_call"});
        let signal = super::super::coding::request_signal(&request).unwrap();
        assert!(signal.load(std::sync::atomic::Ordering::Acquire));
        assert!(guard.cancelled());
        assert!(!other.cancelled());
        let execution = ProductionExecution::begin_workspace_run(&storage, "manual_mcp", &root, run).unwrap();
        execution.cancel_manual_before_dispatch().unwrap();
        let receipt = ProductionExecution::read_manual_mcp(&storage, &root, run).unwrap();
        assert_eq!(receipt["phase"], "not_dispatched");
        assert_eq!(receipt["can_archive"], true);
        assert_eq!(receipt["can_resolve"], false);
        assert!(ProductionExecution::begin_workspace_run(&storage, "manual_mcp", &root, run).is_err());
        drop(guard);
        drop(other);
        std::fs::remove_dir_all(root).unwrap();
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
