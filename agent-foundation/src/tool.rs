//! Fail-closed tool dispatch with exact-effect binding, one-shot approval,
//! timeout, and cooperative cancellation. No shell/MCP/FS tool is registered.
use crate::{event::EventKind, task::{ApprovedOperation, TaskRegistry}, FoundationError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, future::Future, pin::Pin, sync::{Arc, RwLock}, time::Duration};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolEffect {
    ReadOnly,
    WriteWorkspace,
    ExecuteProcess,
    ExternalSideEffect,
}

pub type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>>;

pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn effect(&self) -> ToolEffect;
    fn call<'a>(&'a self, arguments: Value) -> ToolFuture<'a>;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn AgentTool>>>,
}

impl ToolRegistry {
    pub fn register(&self, tool: Arc<dyn AgentTool>) -> Result<()> {
        let name = tool.name();
        if name.is_empty() || name.len() > 128
            || !name.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
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


    /// Proposals are created using the registered tool effect, never a
    /// model-supplied effect classification. The operation still requires
    /// a separate trusted-host user approval before execution.
    pub fn propose(&self, tasks: &TaskRegistry, task_id: uuid::Uuid, name: &str, args: Value) -> Result<uuid::Uuid> {
        let effect = {
            let registry = self.tools.read().unwrap_or_else(|e| e.into_inner());
            registry.get(name).map(|tool| tool.effect()).ok_or(FoundationError::PermissionDenied)?
        };
        tasks.propose_tool(task_id, name, args, effect)
    }

    /// Calls are only possible with a permit produced by trusted UI approval.
    /// Tool implementations must independently validate root/scope/arguments and
    /// provide an idempotency key for irreversible external side effects.
    pub async fn execute(&self, permit: ApprovedOperation, tasks: &TaskRegistry, timeout: Duration) -> Result<Value> {
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
            tools.get(&permit.operation.tool_name).cloned().ok_or(FoundationError::PermissionDenied)?
        };
        if tool.effect() != permit.operation.effect {
            return Err(FoundationError::PermissionDenied);
        }
        let mut cancellation = tasks.cancellation(permit.task_id)?;
        if *cancellation.borrow() {
            return Err(FoundationError::Cancelled);
        }
        let future = tool.call(permit.operation.arguments);
        let result = tokio::select! {
            response = future => response,
            _ = cancellation.changed() => Err(FoundationError::Cancelled),
            _ = tokio::time::sleep(timeout) => Err(FoundationError::Timeout),
        };
        // Reject stale results from a paused/cancelled/completed/restarted task.
        if !tasks.active_at_revision(permit.task_id, permit.revision) {
            return Err(FoundationError::Cancelled);
        }
        tasks.journal().append(permit.task_id, EventKind::ToolFinished {
            call_id: permit.operation.call_id, succeeded: result.is_ok(),
        });
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
        fn name(&self) -> &str { "example.echo" }
        fn effect(&self) -> ToolEffect { ToolEffect::ReadOnly }
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
        let call = tasks.propose_tool(task.id, "example.echo", serde_json::json!({"ok":true}), ToolEffect::ReadOnly).unwrap();
        let permit = tasks.resolve_approval(task.id, call, true).unwrap().unwrap();
        assert_eq!(tools.execute(permit, &tasks, Duration::from_secs(1)).await.unwrap()["ok"], true);
        tasks.finish(task.id, true).unwrap();
    }

    #[tokio::test]
    async fn cancellation_blocks_a_previously_approved_call() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let task = tasks.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let call = tasks.propose_tool(task.id, "example.echo", Value::Null, ToolEffect::ReadOnly).unwrap();
        let permit = tasks.resolve_approval(task.id, call, true).unwrap().unwrap();
        tasks.cancel(task.id).unwrap();
        let tools = ToolRegistry::default();
        tools.register(Arc::new(Echo)).unwrap();
        assert_eq!(tools.execute(permit, &tasks, Duration::from_secs(1)).await.err(), Some(FoundationError::PermissionDenied));
    }
}
