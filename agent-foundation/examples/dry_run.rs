//! A safe, deterministic demonstration: mock model -> approval -> mock tool.
use browseros_agent_foundation::{
    approval::{ApprovalBroker, HostApprovalDecision, HostApprovalIssuer, ResourceBinding},
    engine::{AgentEngine, TurnCompletion, TurnLimits},
    event::EventJournal,
    model::{ModelEvent, ModelFuture, ModelProvider, ModelRequest},
    task::{TaskRegistry, TaskStatus},
    tool::{AgentTool, ToolEffect, ToolFuture, ToolRegistry},
    Result,
};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;
use uuid::Uuid;

struct FakeModel;
impl ModelProvider for FakeModel {
    fn stream<'a>(&'a self, request: ModelRequest) -> ModelFuture<'a> {
        Box::pin(async move {
            request.validate()?;
            let (sender, receiver) = mpsc::channel(4);
            sender
                .send(ModelEvent::TextDelta {
                    text: "I need to inspect a mock document.".into(),
                })
                .await
                .expect("the mock stream is open");
            sender
                .send(ModelEvent::ToolSuggestion {
                    tool_name: "demo.read".into(),
                    arguments_json: json!({"document":"example.txt"}).to_string(),
                })
                .await
                .expect("the mock stream is open");
            Ok(receiver)
        })
    }
}

struct FakeReadTool;
impl AgentTool for FakeReadTool {
    fn name(&self) -> &str {
        "demo.read"
    }
    fn effect(&self) -> ToolEffect {
        ToolEffect::ReadOnly
    }
    // Mock has no real resource; production tools must query the owning API
    // and compare the actual resource revision to binding.resource_revision.
    fn validate_binding(
        &self,
        binding: &browseros_agent_foundation::approval::ApprovalBinding,
        _args: &Value,
    ) -> Result<()> {
        if binding.resource.resource_id.starts_with("workspace:") {
            Ok(())
        } else {
            Err(browseros_agent_foundation::FoundationError::PermissionDenied)
        }
    }
    fn call<'a>(&'a self, arguments: Value) -> ToolFuture<'a> {
        Box::pin(async move { Ok(json!({"simulated":true,"requested":arguments})) })
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let journal = Arc::new(EventJournal::new(32));
    let tasks = TaskRegistry::new(journal.clone());
    let tools = ToolRegistry::default();
    tools.register(Arc::new(FakeReadTool))?;

    let workspace = Uuid::new_v4();
    let task = tasks.create_scoped(workspace, Uuid::new_v4(), "Inspect demo document", vec![])?;
    tasks.start(task.id)?;
    let engine = AgentEngine {
        provider: &FakeModel,
        tasks: &tasks,
        tools: &tools,
        limits: TurnLimits::default(),
    };
    let result = engine
        .drive_turn(task.id, "mock", "Inspect the document")
        .await?;
    println!("Model: {}", result.text);
    let TurnCompletion::NeedsApproval { call_id } = result.completion else {
        unreachable!("mock model should request a tool");
    };
    assert_eq!(tasks.get(task.id)?.status, TaskStatus::AwaitingApproval);
    // Only a trusted host UI path may hold the signing key / issue receipts.
    // This demo uses a fixed key and fake principal, NEVER use either in production.
    let host = HostApprovalIssuer::new([31; 32])?;
    let broker = ApprovalBroker::new([31; 32])?;
    let ticket = host.issue(
        &tasks,
        task.id,
        call_id,
        HostApprovalDecision {
            principal_id: Uuid::new_v4(),
            resource: ResourceBinding {
                resource_id: format!("workspace:{workspace}"),
                resource_revision: "demo-v1".into(),
            },
            accepted: true,
            ttl: Duration::from_secs(30),
        },
    )?;
    let permit = broker
        .resolve(&tasks, &ticket)?
        .expect("demo host approved the one pending mock tool");
    let answer = tools
        .execute(permit, &tasks, Duration::from_secs(2))
        .await?;
    assert_eq!(answer["simulated"], true);
    tasks.finish(task.id, true)?;
    println!("Mock tool returned: {}", answer);
    println!("Task status: {:?}", tasks.get(task.id)?.status);
    println!("Events retained: {}", journal.replay(task.id, 0)?.len());
    Ok(())
}
