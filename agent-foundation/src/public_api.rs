//! BrowserOS Public API boundary: no direct DB/Git/Browser/Terminal internals.
//! The application host supplies the existing native command/Studio owner
//! implementations. An allowlisted read operation never bypasses Host policy.
use crate::{
    approval::ApprovalBinding,
    tool::{AgentTool, ToolEffect, ToolFuture},
    FoundationError, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityOwner {
    Browser,
    Coding,
    Database,
    Git,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicApiSpec {
    pub owner: CapabilityOwner,
    pub command: String,
    pub effect: ToolEffect,
    /// A resource owner must independently verify exact principal, workspace,
    /// capability scope and revision at the time the operation executes.
    pub requires_host_authorization: bool,
}
/// Only actual existing commands found in src-tauri, not guessed names.
/// This is a CONTRACT CATALOG, not an adapter connected to the running app.
pub fn current_public_api_catalog() -> Vec<PublicApiSpec> {
    use CapabilityOwner::*;
    [
        (Browser, "tab_list", ToolEffect::ReadOnly),
        (Coding, "coding_lsp_status", ToolEffect::ReadOnly),
        (Database, "db_list_connections", ToolEffect::ReadOnly),
        (Database, "db_query", ToolEffect::ExternalSideEffect),
        (Git, "git_status", ToolEffect::ReadOnly),
        (Git, "git_diff", ToolEffect::ReadOnly),
        (Git, "git_studio_confirm", ToolEffect::ExternalSideEffect),
    ]
    .into_iter()
    .map(|(owner, command, effect)| PublicApiSpec {
        owner,
        command: command.into(),
        effect,
        requires_host_authorization: true,
    })
    .collect()
}

/// Required host adapter. BrowserOS owns the actual Tauri command and its
/// credential storage, permission check, IO scheduling and cancellation.
pub trait PublicApiHost: Send + Sync {
    fn validate<'a>(
        &'a self,
        spec: &'a PublicApiSpec,
        proof: &'a ApprovalBinding,
        args: &'a Value,
    ) -> Result<()>;
    fn call<'a>(
        &'a self,
        spec: &'a PublicApiSpec,
        proof: ApprovalBinding,
        args: Value,
    ) -> ToolFuture<'a>;
}

/// A read-only, native PUBLIC-API-backed tool. This adapter cannot call a
/// bridge or invent an auth context. No host installed => no execution.
/// Side-effectful operations must separately satisfy durable UNKNOWN/outbox.
pub struct PublicReadTool {
    spec: PublicApiSpec,
    host: Arc<dyn PublicApiHost>,
}
impl PublicReadTool {
    pub fn new(spec: PublicApiSpec, host: Arc<dyn PublicApiHost>) -> Result<Self> {
        if spec.effect != ToolEffect::ReadOnly || !spec.requires_host_authorization {
            return Err(FoundationError::PermissionDenied);
        }
        Ok(Self { spec, host })
    }
}
impl AgentTool for PublicReadTool {
    fn name(&self) -> &str {
        &self.spec.command
    }
    fn effect(&self) -> ToolEffect {
        ToolEffect::ReadOnly
    }
    fn validate_binding(&self, binding: &ApprovalBinding, args: &Value) -> Result<()> {
        self.host.validate(&self.spec, binding, args)
    }
    fn requires_authorized_call(&self) -> bool {
        true
    }
    fn call<'a>(&'a self, _arguments: Value) -> ToolFuture<'a> {
        Box::pin(async { Err(FoundationError::PermissionDenied) })
    }
    fn call_authorized<'a>(&'a self, call: crate::tool::AuthorizedCall) -> ToolFuture<'a> {
        Box::pin(async move {
            self.host
                .validate(&self.spec, &call.binding, &call.arguments)?;
            self.host
                .call(&self.spec, call.binding, call.arguments)
                .await
        })
    }
}

/// The host must explicitly install real PublicApiHost implementations from
/// the existing capability modules. Runtime has no implicit native access.
#[derive(Default)]
pub struct PublicApiRegistry {
    owners: RwLock<HashMap<CapabilityOwner, Arc<dyn PublicApiHost>>>,
}
impl PublicApiRegistry {
    pub fn register(&self, owner: CapabilityOwner, host: Arc<dyn PublicApiHost>) -> Result<()> {
        let mut map = self.owners.write().unwrap_or_else(|e| e.into_inner());
        if map.contains_key(&owner) {
            return Err(FoundationError::InvalidTransition);
        }
        map.insert(owner, host);
        Ok(())
    }
    pub fn read_tool(&self, spec: PublicApiSpec) -> Result<PublicReadTool> {
        if !current_public_api_catalog().contains(&spec) {
            return Err(FoundationError::PermissionDenied);
        }
        let map = self.owners.read().unwrap_or_else(|e| e.into_inner());
        let host = map
            .get(&spec.owner)
            .cloned()
            .ok_or(FoundationError::PermissionDenied)?;
        PublicReadTool::new(spec, host)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    struct FakeHost;
    impl PublicApiHost for FakeHost {
        fn validate<'a>(
            &'a self,
            _: &'a PublicApiSpec,
            proof: &'a ApprovalBinding,
            _: &'a Value,
        ) -> Result<()> {
            if proof.principal_id.is_nil() {
                Err(FoundationError::PermissionDenied)
            } else {
                Ok(())
            }
        }
        fn call<'a>(
            &'a self,
            _: &'a PublicApiSpec,
            proof: ApprovalBinding,
            _: Value,
        ) -> ToolFuture<'a> {
            Box::pin(async move {
                if proof.principal_id.is_nil() {
                    return Err(FoundationError::PermissionDenied);
                }
                Ok(serde_json::json!({"status":"mock public api"}))
            })
        }
    }
    #[tokio::test]
    async fn host_required_and_no_write_capability_bypass() {
        let registry = PublicApiRegistry::default();
        let specs = current_public_api_catalog();
        let read = specs
            .iter()
            .find(|s| s.command == "git_status")
            .unwrap()
            .clone();
        let write = specs
            .iter()
            .find(|s| s.command == "git_studio_confirm")
            .unwrap()
            .clone();
        assert!(registry.read_tool(read.clone()).is_err());
        registry
            .register(CapabilityOwner::Git, Arc::new(FakeHost))
            .unwrap();
        assert!(registry.read_tool(write).is_err());
        let tool = registry.read_tool(read).unwrap();
        let proof = ApprovalBinding {
            principal_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            resource: crate::approval::ResourceBinding {
                resource_id: "git:demo".into(),
                resource_revision: "1".into(),
            },
            expires_at_unix_ms: u64::MAX,
        };
        tool.validate_binding(&proof, &serde_json::json!({}))
            .unwrap();
        assert!(tool.call(serde_json::json!({})).await.is_err());
        assert_eq!(
            tool.call_authorized(crate::tool::AuthorizedCall {
                arguments: serde_json::json!({}),
                binding: proof,
                idempotency_key: "test-local".into(),
            })
            .await
            .unwrap()["status"],
            "mock public api"
        );
    }
}
