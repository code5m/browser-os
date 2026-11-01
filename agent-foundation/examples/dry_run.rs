//! A safe, deterministic demonstration: mock model -> approval -> mock tool.
use browseros_agent_foundation::{
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

    let task = tasks.create(Uuid::new_v4(), "Inspect demo document", vec![])?;
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
    // In a real app, only an authenticated and informed host user can approve.
    let permit = tasks
        .resolve_approval(task.id, call_id, true)?
        .expect("demo user explicitly approved the one pending tool");
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
