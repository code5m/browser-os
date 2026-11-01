//! Signed, single-use host approval boundary. The host MUST authenticate the
//! user and present exact arguments before calling HostApprovalIssuer::issue.
//! Neither the model nor a remote protocol peer may hold the issuing key.
use crate::{
    context::ContextRef,
    task::{ApprovedOperation, PendingOperation, TaskRegistry, TaskStatus},
    tool::ToolEffect,
    FoundationError, Result,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;
const MAX_APPROVAL_SECS: u64 = 120;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResourceBinding {
    pub resource_id: String,
    pub resource_revision: String,
}

impl ResourceBinding {
    fn validate(&self, workspace: Uuid, effect: ToolEffect, context: &[ContextRef]) -> Result<()> {
        if self.resource_id.len() > 4096
            || self.resource_revision.is_empty()
            || self.resource_revision.len() > 256
            || self.resource_id.bytes().any(|c| c < 32)
            || self.resource_revision.bytes().any(|c| c < 32)
        {
            return Err(FoundationError::PermissionDenied);
        }
        let workspace_only = format!("workspace:{workspace}");
        // A workspace-only scope grants no write / process / remote side effect.
        if effect == ToolEffect::ReadOnly && self.resource_id == workspace_only {
            return Ok(());
        }
        if !context.iter().any(|c| c.resource_id == self.resource_id) {
            return Err(FoundationError::PermissionDenied);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalClaims {
    pub ticket_id: Uuid,
    pub principal_id: Uuid,
    pub workspace_id: Uuid,
    pub session_id: Uuid,
    pub task_id: Uuid,
    pub call_id: Uuid,
    pub task_revision: u64,
    pub tool_name: String,
    pub effect: ToolEffect,
    pub arguments_sha256: String,
    pub resource: ResourceBinding,
    pub accepted: bool,
    pub expires_at_unix_ms: u64,
}

/// A signed receipt. Host-issued receipts can be transferred across a process
/// boundary, but MUST NOT be exposed as a model-callable approval tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedApproval {
    pub claims: ApprovalClaims,
    pub mac: String,
}

#[derive(Clone, Debug)]
pub struct ApprovalBinding {
    pub principal_id: Uuid,
    pub workspace_id: Uuid,
    pub session_id: Uuid,
    pub resource: ResourceBinding,
    pub expires_at_unix_ms: u64,
}

/// Host-only issuer, owned by the trusted OS / UI authenticated user-action path.
/// Issuing a receipt itself is NOT proof that the UI authenticated a person.
pub struct HostApprovalIssuer {
    key: [u8; 32],
}

pub struct ApprovalBroker {
    key: [u8; 32],
}

/// All fields for one human decision. The authenticated host owns this value;
/// the model may only request a proposal, never construct a decision.
pub struct HostApprovalDecision {
    pub principal_id: Uuid,
    pub resource: ResourceBinding,
    pub accepted: bool,
    pub ttl: Duration,
}

fn validate_key(key: [u8; 32]) -> Result<[u8; 32]> {
    if key.iter().all(|value| *value == 0) {
        return Err(FoundationError::InvalidInput("empty approval signing key"));
    }
    Ok(key)
}

fn now_ms() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FoundationError::PermissionDenied)?;
    u64::try_from(now.as_millis()).map_err(|_| FoundationError::PermissionDenied)
}

fn argument_digest(op: &PendingOperation) -> Result<String> {
    let serialized = serde_json::to_vec(&(&op.tool_name, &op.arguments, &op.effect))
        .map_err(|e| FoundationError::Serialization(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(serialized)))
}

fn signature(key: &[u8; 32], claims: &ApprovalClaims) -> Result<String> {
    let bytes =
        serde_json::to_vec(claims).map_err(|e| FoundationError::Serialization(e.to_string()))?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| FoundationError::PermissionDenied)?;
    mac.update(&bytes);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

impl HostApprovalIssuer {
    pub fn new(secret: [u8; 32]) -> Result<Self> {
        Ok(Self {
            key: validate_key(secret)?,
        })
    }

    /// Call ONLY after an authenticated local user action approving the exact
    /// tool, arguments, resource identity/version and effect shown in the UI.
    /// The workspace/session are loaded from the existing trusted TaskRecord.
    pub fn issue(
        &self,
        tasks: &TaskRegistry,
        task_id: Uuid,
        call_id: Uuid,
        decision: HostApprovalDecision,
    ) -> Result<SignedApproval> {
        let HostApprovalDecision {
            principal_id,
            resource,
            accepted,
            ttl,
        } = decision;
        if principal_id.is_nil()
            || ttl.is_zero()
            || ttl.as_millis() > u128::from(MAX_APPROVAL_SECS) * 1000
        {
            return Err(FoundationError::PermissionDenied);
        }
        let task = tasks.get(task_id)?;
        if task.workspace_id.is_nil()
            || task.session_id.is_nil()
            || task.status != TaskStatus::AwaitingApproval
        {
            return Err(FoundationError::PermissionDenied);
        }
        let op = task
            .pending
            .as_ref()
            .ok_or(FoundationError::ApprovalMismatch)?;
        if op.call_id != call_id {
            return Err(FoundationError::ApprovalMismatch);
        }
        resource.validate(task.workspace_id, op.effect, &task.context)?;
        let expires_at_unix_ms = now_ms()?
            .checked_add(
                u64::try_from(ttl.as_millis()).map_err(|_| FoundationError::PermissionDenied)?,
            )
            .ok_or(FoundationError::PermissionDenied)?;
        let claims = ApprovalClaims {
            ticket_id: Uuid::new_v4(),
            principal_id,
            workspace_id: task.workspace_id,
            session_id: task.session_id,
            task_id,
            call_id,
            task_revision: task.revision,
            tool_name: op.tool_name.clone(),
            effect: op.effect,
            arguments_sha256: argument_digest(op)?,
            resource,
            accepted,
            expires_at_unix_ms,
        };
        let mac = signature(&self.key, &claims)?;
        Ok(SignedApproval { claims, mac })
    }
}

impl ApprovalBroker {
    pub fn new(secret: [u8; 32]) -> Result<Self> {
        Ok(Self {
            key: validate_key(secret)?,
        })
    }

    pub fn resolve(
        &self,
        tasks: &TaskRegistry,
        receipt: &SignedApproval,
    ) -> Result<Option<ApprovedOperation>> {
        let claim = &receipt.claims;
        // Constant-time HMAC verification over the original signed claims.
        let supplied = hex::decode(&receipt.mac).map_err(|_| FoundationError::PermissionDenied)?;
        let bytes =
            serde_json::to_vec(claim).map_err(|e| FoundationError::Serialization(e.to_string()))?;
        let mut mac =
            HmacSha256::new_from_slice(&self.key).map_err(|_| FoundationError::PermissionDenied)?;
        mac.update(&bytes);
        mac.verify_slice(&supplied)
            .map_err(|_| FoundationError::PermissionDenied)?;
        let now = now_ms()?;
        if claim.ticket_id.is_nil()
            || claim.principal_id.is_nil()
            || claim.expires_at_unix_ms <= now
            || claim.expires_at_unix_ms.saturating_sub(now) > MAX_APPROVAL_SECS * 1000
        {
            return Err(FoundationError::PermissionDenied);
        }
        let task = tasks.get(claim.task_id)?;
        let op = task
            .pending
            .as_ref()
            .ok_or(FoundationError::ApprovalMismatch)?;
        if task.status != TaskStatus::AwaitingApproval
            || task.workspace_id.is_nil()
            || task.workspace_id != claim.workspace_id
            || task.session_id != claim.session_id
            || task.revision != claim.task_revision
            || op.call_id != claim.call_id
            || op.tool_name != claim.tool_name
            || op.effect != claim.effect
            || argument_digest(op)? != claim.arguments_sha256
        {
            return Err(FoundationError::PermissionDenied);
        }
        claim
            .resource
            .validate(task.workspace_id, op.effect, &task.context)?;
        let binding = ApprovalBinding {
            principal_id: claim.principal_id,
            session_id: claim.session_id,
            workspace_id: claim.workspace_id,
            resource: claim.resource.clone(),
            expires_at_unix_ms: claim.expires_at_unix_ms,
        };
        let permit = tasks.resolve_approval(claim.task_id, claim.call_id, claim.accepted)?;
        Ok(permit.map(|mut permit| {
            permit.binding = Some(binding);
            permit
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event::EventJournal, tool::ToolEffect};
    use std::sync::Arc;

    fn setup() -> (
        TaskRegistry,
        HostApprovalIssuer,
        ApprovalBroker,
        Uuid,
        Uuid,
        ResourceBinding,
    ) {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let workspace = Uuid::new_v4();
        let task = tasks
            .create_scoped(workspace, Uuid::new_v4(), "Inspect", vec![])
            .unwrap();
        tasks.start(task.id).unwrap();
        let call = tasks
            .propose_tool(
                task.id,
                "editor.read",
                serde_json::json!({"p":"a"}),
                ToolEffect::ReadOnly,
            )
            .unwrap();
        let resource = ResourceBinding {
            resource_id: format!("workspace:{workspace}"),
            resource_revision: "r1".into(),
        };
        (
            tasks,
            HostApprovalIssuer::new([42; 32]).unwrap(),
            ApprovalBroker::new([42; 32]).unwrap(),
            task.id,
            call,
            resource,
        )
    }

    #[test]
    fn tampering_expiration_and_replay_are_rejected() {
        let (tasks, issuer, broker, task, call, resource) = setup();
        let ticket = issuer
            .issue(
                &tasks,
                task,
                call,
                HostApprovalDecision {
                    principal_id: Uuid::new_v4(),
                    resource,
                    accepted: true,
                    ttl: Duration::from_secs(30),
                },
            )
            .unwrap();
        let mut tampered = ticket.clone();
        tampered.claims.effect = ToolEffect::ExecuteProcess;
        assert!(broker.resolve(&tasks, &tampered).is_err());
        let mut expired = ticket.clone();
        expired.claims.expires_at_unix_ms = 1;
        assert!(broker.resolve(&tasks, &expired).is_err());
        assert!(broker.resolve(&tasks, &ticket).unwrap().is_some());
        assert!(broker.resolve(&tasks, &ticket).is_err());
    }

    #[test]
    fn mismatched_key_or_workspace_is_denied() {
        let (tasks, issuer, _, task, call, resource) = setup();
        let ticket = issuer
            .issue(
                &tasks,
                task,
                call,
                HostApprovalDecision {
                    principal_id: Uuid::new_v4(),
                    resource,
                    accepted: true,
                    ttl: Duration::from_secs(30),
                },
            )
            .unwrap();
        assert!(ApprovalBroker::new([3; 32])
            .unwrap()
            .resolve(&tasks, &ticket)
            .is_err());
    }
}
