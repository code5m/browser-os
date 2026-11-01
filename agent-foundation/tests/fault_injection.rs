#![cfg(feature = "durable-sqlite")]
//! Failure injection at the trusted host -> external tool and durable commit
//! boundary. Never executes real shell, filesystem or network operations.
use browseros_agent_foundation::{
    approval::{
        ApprovalBinding, ApprovalBroker, HostApprovalDecision, HostApprovalIssuer, ResourceBinding,
    },
    context::{ContextGrant, ContextKind, ContextRef},
    durable::{EffectState, SqliteTaskStore},
    event::EventJournal,
    task::{TaskRegistry, TaskStatus},
    tool::{AgentTool, AuthorizedCall, ToolEffect, ToolFuture, ToolRegistry},
    FoundationError, Result,
};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use uuid::Uuid;

struct SideEffectMock {
    attempted: Arc<AtomicUsize>,
    stale_resource: Arc<AtomicBool>,
}
impl AgentTool for SideEffectMock {
    fn name(&self) -> &str {
        "demo.external"
    }
    fn effect(&self) -> ToolEffect {
        ToolEffect::ExternalSideEffect
    }
    fn supports_idempotency(&self) -> bool {
        true
    }
    fn validate_binding(&self, binding: &ApprovalBinding, _args: &Value) -> Result<()> {
        if binding.principal_id.is_nil()
            || binding.resource.resource_revision != "current-v1"
            || self.stale_resource.load(Ordering::SeqCst)
        {
            return Err(FoundationError::PermissionDenied);
        }
        Ok(())
    }
    fn call<'a>(&'a self, _args: Value) -> ToolFuture<'a> {
        Box::pin(async { panic!("unsafe legacy call must never be used for an effectful tool") })
    }
    fn call_authorized<'a>(&'a self, call: AuthorizedCall) -> ToolFuture<'a> {
        Box::pin(async move {
            assert!(call.idempotency_key.starts_with("browseros:"));
            self.attempted.fetch_add(1, Ordering::SeqCst);
            std::future::pending::<Result<Value>>().await
        })
    }
}

struct Fixture {
    tasks: Arc<TaskRegistry>,
    db: Arc<SqliteTaskStore>,
    tools: ToolRegistry,
    task_id: Uuid,
    call_id: Uuid,
    attempted: Arc<AtomicUsize>,
    stale: Arc<AtomicBool>,
}

fn fixture() -> Fixture {
    let db = Arc::new(SqliteTaskStore::in_memory().unwrap());
    let tasks =
        Arc::new(TaskRegistry::new_durable(db.clone(), Arc::new(EventJournal::new(128))).unwrap());
    let context = vec![ContextRef::checked(
        ContextKind::Resource,
        "opaque:resource-r1",
        ContextGrant::ExplicitSelection,
    )
    .unwrap()];
    let task = tasks
        .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "External mock", context)
        .unwrap();
    tasks.start(task.id).unwrap();
    let attempted = Arc::new(AtomicUsize::new(0));
    let stale = Arc::new(AtomicBool::new(false));
    let tools = ToolRegistry::default();
    tools
        .register(Arc::new(SideEffectMock {
            attempted: attempted.clone(),
            stale_resource: stale.clone(),
        }))
        .unwrap();
    let call_id = tools
        .propose(
            &tasks,
            task.id,
            "demo.external",
            json!({"destination":"mock"}),
        )
        .unwrap();
    Fixture {
        tasks,
        db,
        tools,
        task_id: task.id,
        call_id,
        attempted,
        stale,
    }
}

fn approve(f: &Fixture) -> browseros_agent_foundation::task::ApprovedOperation {
    let issuer = HostApprovalIssuer::new([91; 32]).unwrap();
    let broker = ApprovalBroker::new([91; 32]).unwrap();
    let ticket = issuer
        .issue(
            &f.tasks,
            f.task_id,
            f.call_id,
            HostApprovalDecision {
                principal_id: Uuid::new_v4(),
                resource: ResourceBinding {
                    resource_id: "opaque:resource-r1".into(),
                    resource_revision: "current-v1".into(),
                },
                accepted: true,
                ttl: Duration::from_secs(30),
            },
        )
        .unwrap();
    broker.resolve(&f.tasks, &ticket).unwrap().unwrap()
}

#[tokio::test]
async fn timeout_leaves_unknown_receipt_and_no_automatic_retry() {
    let f = fixture();
    let permit = approve(&f);
    let result = f
        .tools
        .execute(permit, &f.tasks, Duration::from_millis(15))
        .await;
    assert_eq!(result.err(), Some(FoundationError::Timeout));
    assert_eq!(f.attempted.load(Ordering::SeqCst), 1);
    let unknown = f.db.unknown_effects().unwrap();
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].call_id, f.call_id);
    assert_eq!(unknown[0].state, EffectState::Unknown);
    assert!(f
        .db
        .begin_effect(
            f.task_id,
            f.call_id,
            &format!("browseros:{}", f.call_id),
            ToolEffect::ExternalSideEffect,
            "demo.external"
        )
        .is_err());
    assert_eq!(f.attempted.load(Ordering::SeqCst), 1);
    f.db.reconcile_effect(
        f.call_id,
        true,
        "mock provider confirms original operation was applied",
    )
    .unwrap();
    assert!(f.db.unknown_effects().unwrap().is_empty());
}

#[tokio::test]
async fn revision_drift_denies_execution_without_effect_intent() {
    let f = fixture();
    let permit = approve(&f);
    f.stale.store(true, Ordering::SeqCst);
    assert_eq!(
        f.tools
            .execute(permit, &f.tasks, Duration::from_millis(15))
            .await
            .err(),
        Some(FoundationError::PermissionDenied)
    );
    assert_eq!(f.attempted.load(Ordering::SeqCst), 0);
    assert!(f.db.unknown_effects().unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_keeps_unknown_outcome_for_reconciliation() {
    let f = fixture();
    let permit = approve(&f);
    let tasks = f.tasks.clone();
    let task_id = f.task_id;
    let future = f.tools.execute(permit, &f.tasks, Duration::from_secs(2));
    let cancelled = async move {
        while tasks.get(task_id).unwrap().status != TaskStatus::Running {
            tokio::task::yield_now().await;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        tasks.cancel(task_id).unwrap();
    };
    let (result, ()) = tokio::join!(future, cancelled);
    assert_eq!(result.err(), Some(FoundationError::Cancelled));
    assert_eq!(f.attempted.load(Ordering::SeqCst), 1);
    assert_eq!(f.db.unknown_effects().unwrap().len(), 1);
}

#[test]
fn disk_restart_persists_atomic_outbox_and_pauses_running_tasks() {
    let path =
        std::env::temp_dir().join(format!("agent-foundation-fault-{}.sqlite", Uuid::new_v4()));
    let db = Arc::new(SqliteTaskStore::open(&path).unwrap());
    let tasks = TaskRegistry::new_durable(db.clone(), Arc::new(EventJournal::new(64))).unwrap();
    let task = tasks
        .create_scoped(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Safe simulated crash",
            vec![],
        )
        .unwrap();
    tasks.start(task.id).unwrap();
    let created = tasks.get(task.id).unwrap();
    assert_eq!(created.status, TaskStatus::Running);
    drop(tasks);
    drop(db);

    let reopened = Arc::new(SqliteTaskStore::open(&path).unwrap());
    let recovered =
        TaskRegistry::new_durable(reopened.clone(), Arc::new(EventJournal::new(64))).unwrap();
    assert_eq!(recovered.get(task.id).unwrap().status, TaskStatus::Paused);
    let events = reopened.events_since(task.id, 0, 100).unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(reopened.pending_outbox(100).unwrap().len(), 4);
    let first = events[0].event_id;
    reopened.mark_outbox_delivered(first).unwrap();
    assert!(reopened.mark_outbox_delivered(first).is_err());
    assert_eq!(reopened.pending_outbox(100).unwrap().len(), 3);
    drop(recovered);
    drop(reopened);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}
