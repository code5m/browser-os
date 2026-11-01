//! Fail-closed tool dispatch with exact-effect binding, one-shot approval,
//! timeout, and cooperative cancellation. No shell/MCP/FS tool is registered.
use crate::{
    approval::ApprovalBinding,
    event::EventKind,
    task::{ApprovedOperation, TaskRegistry},
    FoundationError, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolEffect {
    ReadOnly,
    WriteWorkspace,
    ExecuteProcess,
    ExternalSideEffect,
}

pub type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>>;

/// Passed ONLY after signed host approval and resource-owner validation.
pub struct AuthorizedCall {
    pub arguments: Value,
    pub binding: ApprovalBinding,
    /// Must be sent to the external provider as a durable idempotency key.
    pub idempotency_key: String,
}

pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn effect(&self) -> ToolEffect;
    fn call<'a>(&'a self, arguments: Value) -> ToolFuture<'a>;

    /// The owning capability must re-check actual resource identity, revision,
    /// tenant/project and effect against the authenticated principal at EXECUTE.
    /// Default DENY, including for read-only registered tools.
    fn validate_binding(&self, _binding: &ApprovalBinding, _args: &Value) -> Result<()> {
        Err(FoundationError::PermissionDenied)
    }

    /// Effectful tools opt into end-to-end provider-side idempotency explicitly.
    fn supports_idempotency(&self) -> bool {
        false
    }

    /// Host-backed tools require passing the signed proof into the SAME call,
    /// not merely validating before calling a detached unbound function.
    fn requires_authorized_call(&self) -> bool {
        false
    }

    /// Non-read tools MUST implement this, and forward idempotency_key to the
    /// external operation. A legacy call method may never perform side effects.
    fn call_authorized<'a>(&'a self, _call: AuthorizedCall) -> ToolFuture<'a> {
        Box::pin(async { Err(FoundationError::PermissionDenied) })
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn AgentTool>>>,
}

impl ToolRegistry {
    pub fn register(&self, tool: Arc<dyn AgentTool>) -> Result<()> {
        let name = tool.name();
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        {
            return Err(FoundationError::InvalidInput("tool name"));
        }
        let mut tools = self.tools.write().unwrap_or_else(|e| e.into_inner());
        if tools.contains_key(name) {
            return Err(FoundationError::InvalidInput("duplicate tool"));
        }
        tools.insert(name.to_owned(), tool);
        Ok(())
    }

    /// Only a trusted host may advertise an already registered read-only
    /// tool to an experimental model runtime.
    pub fn is_registered_read_only(&self, name: &str) -> bool {
        self.tools
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .is_some_and(|tool| tool.effect() == ToolEffect::ReadOnly)
    }

    /// Proposals are created using the registered tool effect, never a
    /// model-supplied effect classification. The operation still requires
    /// a separate trusted-host user approval before execution.
    pub fn propose(
        &self,
        tasks: &TaskRegistry,
        task_id: uuid::Uuid,
        name: &str,
        args: Value,
    ) -> Result<uuid::Uuid> {
        let effect = {
            let registry = self.tools.read().unwrap_or_else(|e| e.into_inner());
            registry
                .get(name)
                .map(|tool| tool.effect())
                .ok_or(FoundationError::PermissionDenied)?
        };
        tasks.propose_tool(task_id, name, args, effect)
    }

    /// Calls are only possible with a permit produced by trusted UI approval.
    /// Tool implementations must independently validate root/scope/arguments and
    /// provide an idempotency key for irreversible external side effects.
    pub async fn execute(
        &self,
        permit: ApprovedOperation,
        tasks: &TaskRegistry,
        timeout: Duration,
    ) -> Result<Value> {
        if timeout.is_zero() || timeout > Duration::from_secs(120) {
            return Err(FoundationError::InvalidInput("tool timeout"));
        }
        if !tasks.active_at_revision(permit.task_id, permit.revision) {
            return Err(FoundationError::PermissionDenied);
        }
        // Model and tool execution share one non-reentrant task slot.
        let _lease = tasks.acquire_execution(permit.task_id, permit.revision)?;
        let tool = {
            let tools = self.tools.read().unwrap_or_else(|e| e.into_inner());
            tools
                .get(&permit.operation.tool_name)
                .cloned()
                .ok_or(FoundationError::PermissionDenied)?
        };
        if tool.effect() != permit.operation.effect {
            return Err(FoundationError::PermissionDenied);
        }

        let effectful = permit.operation.effect != ToolEffect::ReadOnly;
        let binding = permit.binding.as_ref();
        if let Some(binding) = binding {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| FoundationError::PermissionDenied)?;
            if u64::try_from(now.as_millis()).map_err(|_| FoundationError::PermissionDenied)?
                >= binding.expires_at_unix_ms
            {
                return Err(FoundationError::PermissionDenied);
            }
            tool.validate_binding(binding, &permit.operation.arguments)?;
        } else if effectful {
            return Err(FoundationError::PermissionDenied);
        }

        if effectful && !tool.supports_idempotency() {
            return Err(FoundationError::PermissionDenied);
        }
        #[cfg(not(feature = "durable-sqlite"))]
        if effectful {
            return Err(FoundationError::PermissionDenied);
        }
        #[cfg(feature = "durable-sqlite")]
        let effect_store = if effectful {
            Some(
                tasks
                    .durable_store()
                    .ok_or(FoundationError::PermissionDenied)?,
            )
        } else {
            None
        };
        let idempotency_key = format!("browseros:{}", permit.operation.call_id);
        let mut cancellation = tasks.cancellation(permit.task_id)?;
        if *cancellation.borrow_and_update() {
            return Err(FoundationError::Cancelled);
        }
        // The UNKNOWN receipt must be DURABLE before any external execution.
        // A process crash or a timeout leaves it unknown; never auto-retry.
        #[cfg(feature = "durable-sqlite")]
        if let Some(store) = effect_store {
            store.begin_effect(
                permit.task_id,
                permit.operation.call_id,
                &idempotency_key,
                permit.operation.effect,
                &permit.operation.tool_name,
            )?;
        }
        let future = if effectful || tool.requires_authorized_call() {
            tool.call_authorized(AuthorizedCall {
                arguments: permit.operation.arguments,
                binding: binding.cloned().ok_or(FoundationError::PermissionDenied)?,
                idempotency_key,
            })
        } else {
            tool.call(permit.operation.arguments)
        };
        let result = tokio::select! {
            response = future => response,
            _ = cancellation.changed() => Err(FoundationError::Cancelled),
            _ = tokio::time::sleep(timeout) => Err(FoundationError::Timeout),
        };
        // Reject stale results from paused/cancelled/completed/restarted work.
        // For side effects the durable UNKNOWN receipt survives this exit.
        if !tasks.active_at_revision(permit.task_id, permit.revision) {
            return Err(FoundationError::Cancelled);
        }
        #[cfg(feature = "durable-sqlite")]
        if let Some(store) = effect_store {
            if result.is_ok() {
                store.confirm_effect(permit.operation.call_id)?;
            }
        }
        if effectful && result.is_err() {
            tasks.record_event(
                permit.task_id,
                EventKind::EffectUnknown {
                    call_id: permit.operation.call_id,
                },
            )?;
        }
        tasks.record_event(
            permit.task_id,
            EventKind::ToolFinished {
                call_id: permit.operation.call_id,
                succeeded: result.is_ok(),
            },
        )?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event::EventJournal, task::TaskRegistry};
    use uuid::Uuid;

    struct Echo;
    impl AgentTool for Echo {
        fn name(&self) -> &str {
            "example.echo"
        }
        fn effect(&self) -> ToolEffect {
            ToolEffect::ReadOnly
        }
        fn call<'a>(&'a self, args: Value) -> ToolFuture<'a> {
            Box::pin(async move { Ok(args) })
        }
    }

    #[tokio::test]
    async fn approved_call_succeeds_and_unapproved_cannot_run() {
        let events = Arc::new(EventJournal::new(32));
        let tasks = TaskRegistry::new(events);
        let task = tasks.create(Uuid::new_v4(), "Summarize", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let tools = ToolRegistry::default();
        tools.register(Arc::new(Echo)).unwrap();
        let call = tasks
            .propose_tool(
                task.id,
                "example.echo",
                serde_json::json!({"ok":true}),
                ToolEffect::ReadOnly,
            )
            .unwrap();
        let permit = tasks
            .resolve_approval(task.id, call, true)
            .unwrap()
            .unwrap();
        assert_eq!(
            tools
                .execute(permit, &tasks, Duration::from_secs(1))
                .await
                .unwrap()["ok"],
            true
        );
        tasks.finish(task.id, true).unwrap();
    }

    #[tokio::test]
    async fn cancellation_blocks_a_previously_approved_call() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let task = tasks.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let call = tasks
            .propose_tool(task.id, "example.echo", Value::Null, ToolEffect::ReadOnly)
            .unwrap();
        let permit = tasks
            .resolve_approval(task.id, call, true)
            .unwrap()
            .unwrap();
        tasks.cancel(task.id).unwrap();
        let tools = ToolRegistry::default();
        tools.register(Arc::new(Echo)).unwrap();
        assert_eq!(
            tools
                .execute(permit, &tasks, Duration::from_secs(1))
                .await
                .err(),
            Some(FoundationError::PermissionDenied)
        );
    }
}
