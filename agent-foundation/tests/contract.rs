//! External/public API smoke tests (must run with cargo test).
use browseros_agent_foundation::{
    context::{ContextGrant, ContextKind, ContextRef},
    event::{EventJournal, EventKind},
    protocol::{AgentCommand, Envelope, SCHEMA_VERSION},
    task::{TaskRegistry, TaskStatus},
    tool::{AgentTool, ToolEffect, ToolFuture, ToolRegistry},
    FoundationError, Result,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

struct ReadTool;
impl AgentTool for ReadTool {
    fn name(&self) -> &str {
        "test.inspect"
    }
    fn effect(&self) -> ToolEffect {
        ToolEffect::ReadOnly
    }
    fn call<'a>(&'a self, input: Value) -> ToolFuture<'a> {
        Box::pin(async move { Ok(json!({"inspected":input})) })
    }
}

#[test]
fn versioned_command_is_stable_and_model_cannot_approve() {
    let envelope = Envelope::new(AgentCommand::Get {
        task_id: Uuid::new_v4(),
    });
    assert_eq!(envelope.version, SCHEMA_VERSION);
    let roundtrip: Envelope<AgentCommand> =
        serde_json::from_str(&serde_json::to_string(&envelope).unwrap()).unwrap();
    assert!(roundtrip.validate().is_ok());
    assert!(serde_json::from_str::<AgentCommand>(r#"{"command":"approve"}"#).is_err());
}

#[test]
fn context_requires_a_user_grant_and_does_not_auto_load_resources() {
    let resource = ContextRef::checked(
        ContextKind::EditorSelection,
        "editor:selection:opaque",
        ContextGrant::ExplicitSelection,
    )
    .unwrap();
    assert_eq!(resource.resource_id, "editor:selection:opaque");
    assert!(
        ContextRef::checked(ContextKind::WorkspaceFile, "", ContextGrant::ApprovedScope,).is_err()
    );
}

#[tokio::test]
async fn public_tool_path_requires_explicit_approval_and_is_replay_safe() -> Result<()> {
    let journal = Arc::new(EventJournal::new(32));
    let tasks = TaskRegistry::new(journal.clone());
    let tools = ToolRegistry::default();
    tools.register(Arc::new(ReadTool))?;
    let task = tasks.create(Uuid::new_v4(), "Inspect", vec![])?;
    tasks.start(task.id)?;
    let call_id = tools.propose(&tasks, task.id, "test.inspect", json!({"path":"mock"}))?;
    assert_eq!(tasks.get(task.id)?.status, TaskStatus::AwaitingApproval);
    assert_eq!(
        tasks.resolve_approval(task.id, Uuid::new_v4(), true).err(),
        Some(FoundationError::ApprovalMismatch)
    );
    let permit = tasks.resolve_approval(task.id, call_id, true)?.unwrap();
    let result = tools
        .execute(permit, &tasks, std::time::Duration::from_secs(2))
        .await?;
    assert_eq!(result["inspected"]["path"], "mock");
    assert_eq!(
        tasks.resolve_approval(task.id, call_id, true).err(),
        Some(FoundationError::InvalidTransition)
    );
    assert!(journal.replay(task.id, 0)?.iter().any(|record| matches!(
        &record.kind,
        EventKind::ApprovalResolved { accepted: true, .. }
    )));
    Ok(())
}

#[test]
fn snapshot_restoration_discards_pending_approvals_and_does_not_execute() -> Result<()> {
    let journal = Arc::new(EventJournal::new(32));
    let tasks = TaskRegistry::new(journal);
    let tools = ToolRegistry::default();
    tools.register(Arc::new(ReadTool))?;
    let task = tasks.create(Uuid::new_v4(), "Task", vec![])?;
    tasks.start(task.id)?;
    tools.propose(&tasks, task.id, "test.inspect", json!({"input":"safe"}))?;
    let restored =
        TaskRegistry::restore_snapshot(&tasks.export_snapshot()?, Arc::new(EventJournal::new(32)))?;
    let loaded = restored.get(task.id)?;
    assert_eq!(loaded.status, TaskStatus::Paused);
    assert!(loaded.pending.is_none());
    Ok(())
}
