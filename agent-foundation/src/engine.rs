//! Minimal single-turn Agent runtime. It can stream a model reply and prepare one
//! tool proposal, but CANNOT execute a tool without a separate trusted host approval.
//! This is NOT a complete planner, autonomous loop, ACP server or MCP client.
use crate::{
    event::EventKind,
    model::{ModelEvent, ModelProvider, ModelRequest},
    task::{TaskRegistry, TaskStatus},
    tool::ToolRegistry,
    FoundationError, Result,
};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct TurnLimits {
    pub deadline: Duration,
    pub max_model_events: usize,
    pub max_text_bytes: usize,
}
impl Default for TurnLimits {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(45),
            max_model_events: 256,
            max_text_bytes: 64 * 1024,
        }
    }
}
impl TurnLimits {
    fn validate(self) -> Result<()> {
        if self.deadline.is_zero()
            || self.deadline > Duration::from_secs(120)
            || !(1..=4096).contains(&self.max_model_events)
            || !(1..=256 * 1024).contains(&self.max_text_bytes)
        {
            return Err(FoundationError::InvalidInput("agent turn limits"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnCompletion {
    Completed,
    NeedsApproval { call_id: Uuid },
}

#[derive(Debug)]
pub struct TurnResult {
    /// In-memory content only. Never put model text in metadata-only EventJournal.
    pub text: String,
    pub completion: TurnCompletion,
    pub usage: Option<(u64, u64)>,
}

pub struct AgentEngine<'a> {
    pub provider: &'a dyn ModelProvider,
    pub tools: &'a ToolRegistry,
    pub tasks: &'a TaskRegistry,
    pub limits: TurnLimits,
}

impl AgentEngine<'_> {
    /// Drive exactly one model turn. A suggested tool is only proposed, never
    /// executed. Calling code must use an authenticated UI to resolve approval.
    pub async fn drive_turn(
        &self,
        task_id: Uuid,
        model_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<TurnResult> {
        self.limits.validate()?;
        let task = self.tasks.get(task_id)?;
        if task.status != TaskStatus::Running {
            return Err(FoundationError::InvalidTransition);
        }
        let revision = task.revision;
        // Acquire one shared task execution slot before opening model transport.
        let _lease = self.tasks.acquire_execution(task_id, revision)?;
        let request = ModelRequest {
            session_id: task.session_id,
            model_id: model_id.into(),
            prompt: prompt.into(),
            context: task.context,
        };
        request.validate()?;
        let mut cancellation = self.tasks.cancellation(task_id)?;
        if *cancellation.borrow_and_update() {
            return Err(FoundationError::Cancelled);
        }

        let run = async {
            let mut stream = self.provider.stream(request).await?;
            let mut text = String::new();
            let mut usage = None;
            let mut count = 0usize;

            loop {
                let event = tokio::select! {
                    change = cancellation.changed() => {
                        let _ = change; // closed sender is also cancellation
                        return Err(FoundationError::Cancelled);
                    },
                    item = stream.recv() => item,
                };
                if !self.tasks.active_at_revision(task_id, revision) {
                    return Err(FoundationError::Cancelled);
                }
                count += 1;
                if count > self.limits.max_model_events {
                    return Err(FoundationError::Provider("MODEL_EVENT_LIMIT".to_owned()));
                }
                match event {
                    Some(ModelEvent::TextDelta { text: delta }) => {
                        if delta.len() > self.limits.max_text_bytes.saturating_sub(text.len()) {
                            return Err(FoundationError::Provider("MODEL_TEXT_LIMIT".to_owned()));
                        }
                        self.tasks.journal().append(
                            task_id,
                            EventKind::ModelProgress { bytes: delta.len() as u64 },
                        );
                        text.push_str(&delta);
                    }
                    Some(ModelEvent::ToolSuggestion { tool_name, arguments_json }) => {
                        if arguments_json.len() > 16_384 {
                            return Err(FoundationError::InvalidInput("tool arguments too large"));
                        }
                        let arguments = serde_json::from_str(&arguments_json)
                            .map_err(|_| FoundationError::InvalidInput("tool argument JSON"))?;
                        let call_id = self.tools.propose(self.tasks, task_id, &tool_name, arguments)?;
                        return Ok(TurnResult {
                            text,
                            completion: TurnCompletion::NeedsApproval { call_id },
                            usage,
                        });
                    }
                    Some(ModelEvent::Usage { input_tokens, output_tokens }) => {
                        usage = Some((input_tokens, output_tokens));
                    }
                    Some(ModelEvent::Finished) => {
                        self.tasks.finish(task_id, true)?;
                        return Ok(TurnResult { text, completion: TurnCompletion::Completed, usage });
                    }
                    Some(ModelEvent::Failed { .. }) => {
                        return Err(FoundationError::Provider("MODEL_REPORTED_FAILURE".to_owned()));
                    }
                    None => {
                        return Err(FoundationError::Provider("MODEL_STREAM_ENDED".to_owned()));
                    }
                }
            }
        };
        let result = match tokio::time::timeout(self.limits.deadline, run).await {
            Ok(result) => result,
            Err(_) => Err(FoundationError::Timeout),
        };
        if result.is_err() && self.tasks.active_at_revision(task_id, revision) {
            // Only the same active revision can be failed; never override Pause or Cancel.
            let _ = self.tasks.finish(task_id, false);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event::EventJournal, model::{ModelFuture, ModelStream}, tool::{AgentTool, ToolEffect, ToolFuture}};
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    struct ScriptedProvider(Vec<ModelEvent>);
    impl ModelProvider for ScriptedProvider {
        fn stream<'a>(&'a self, request: ModelRequest) -> ModelFuture<'a> {
            let events = self.0.clone();
            Box::pin(async move {
                request.validate()?;
                let (sender, receiver): (_, ModelStream) = mpsc::channel(8);
                for event in events {
                    sender.send(event).await.map_err(|_| FoundationError::Cancelled)?;
                }
                Ok(receiver)
            })
        }
    }
    struct ReadOnlyTool;
    impl AgentTool for ReadOnlyTool {
        fn name(&self) -> &str { "editor.read" }
        fn effect(&self) -> ToolEffect { ToolEffect::ReadOnly }
        fn call<'a>(&'a self, arguments: Value) -> ToolFuture<'a> {
            Box::pin(async move { Ok(arguments) })
        }
    }

    #[tokio::test]
    async fn a_tool_suggestion_always_waits_for_host_approval() {
        let provider = ScriptedProvider(vec![
            ModelEvent::TextDelta { text: "Inspecting".into() },
            ModelEvent::ToolSuggestion {
                tool_name: "editor.read".into(),
                arguments_json: json!({"file":"sample.rs"}).to_string(),
            },
        ]);
        let journal = Arc::new(EventJournal::new(32));
        let tasks = TaskRegistry::new(journal.clone());
        let tools = ToolRegistry::default();
        tools.register(Arc::new(ReadOnlyTool)).unwrap();
        let task = tasks.create(Uuid::new_v4(), "Review code", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let engine = AgentEngine { provider: &provider, tools: &tools, tasks: &tasks, limits: TurnLimits::default() };
        let result = engine.drive_turn(task.id, "mock", "inspect").await.unwrap();
        assert_eq!(result.text, "Inspecting");
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::AwaitingApproval);
        let TurnCompletion::NeedsApproval { call_id } = result.completion else {
            panic!("model must not auto-run a proposed tool");
        };
        assert!(journal.replay(task.id, 0).unwrap().iter().any(|event| {
            matches!(&event.kind, EventKind::ApprovalRequested { .. })
        }));
        assert!(tasks.resolve_approval(task.id, call_id, false).unwrap().is_none());
    }

    #[tokio::test]
    async fn completed_stream_finishes_task() {
        let provider = ScriptedProvider(vec![
            ModelEvent::TextDelta { text: "Done".into() },
            ModelEvent::Finished,
        ]);
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let tools = ToolRegistry::default();
        let task = tasks.create(Uuid::new_v4(), "Plan", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let engine = AgentEngine { provider: &provider, tools: &tools, tasks: &tasks, limits: TurnLimits::default() };
        let response = engine.drive_turn(task.id, "mock", "plan").await.unwrap();
        assert_eq!(response.completion, TurnCompletion::Completed);
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn unregistered_tools_fail_closed() {
        let provider = ScriptedProvider(vec![ModelEvent::ToolSuggestion {
            tool_name: "shell.exec".into(), arguments_json: "{}".into(),
        }]);
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let tools = ToolRegistry::default();
        let task = tasks.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let engine = AgentEngine { provider: &provider, tools: &tools, tasks: &tasks, limits: TurnLimits::default() };
        let result = engine.drive_turn(task.id, "mock", "work").await;
        assert_eq!(result.err(), Some(FoundationError::PermissionDenied));
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Failed);
    }

    struct HangingProvider;
    impl ModelProvider for HangingProvider {
        fn stream<'a>(&'a self, _request: ModelRequest) -> ModelFuture<'a> {
            Box::pin(async move { std::future::pending::<Result<ModelStream>>().await })
        }
    }
    #[tokio::test]
    async fn timeout_is_a_terminal_failure_not_a_stuck_task() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let tools = ToolRegistry::default();
        let task = tasks.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let engine = AgentEngine {
            provider: &HangingProvider, tools: &tools, tasks: &tasks,
            limits: TurnLimits { deadline: Duration::from_millis(1), ..TurnLimits::default() },
        };
        assert_eq!(engine.drive_turn(task.id, "mock", "work").await.err(), Some(FoundationError::Timeout));
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Failed);
    }
}
