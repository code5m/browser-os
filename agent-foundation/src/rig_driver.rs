//! Actual Rig AgentRun multi-turn driver. The SDK owns ONLY the prompt/turn
//! algorithm: TaskRegistry, signed host approval, ToolRegistry and native
//! capability owners remain the sole authority for execution.
use crate::{
    rig_run::RigRunPilot,
    task::{TaskRegistry, TaskStatus},
    tool::ToolRegistry,
    FoundationError, Result,
};
use rig_agent::run::{AgentRunStep, ModelTurn, ModelTurnOutcome};
use rig_core::{
    completion::{CompletionRequest, CompletionResponse, ToolDefinition},
    driver::DynModel,
    message::{ToolCall, ToolResult, ToolResultContent, UserContent},
    operation::Completion,
};
use serde_json::Value;
use std::{collections::BTreeSet, future::Future, pin::Pin, time::Duration};
use uuid::Uuid;

pub type RigCall<'a> = Pin<Box<dyn Future<Output = Result<CompletionResponse>> + Send + 'a>>;
pub trait RigModelBackend: Send + Sync {
    fn complete<'a>(&'a self, request: CompletionRequest) -> RigCall<'a>;
}

/// A genuine official Rig SDK transport for the multi-turn Driver.
pub struct RigSdkBackend(pub DynModel<Completion>);
impl RigModelBackend for RigSdkBackend {
    fn complete<'a>(&'a self, request: CompletionRequest) -> RigCall<'a> {
        Box::pin(async move {
            self.0
                .call(request)
                .await
                .map_err(|_| FoundationError::Provider("RIG_MODEL_CALL_FAILED".into()))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigDriverStep {
    NeedsApproval {
        call_id: Uuid,
        tool_name: String,
    },
    Completed {
        text: String,
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
    },
}

pub struct RigMultiTurnDriver {
    pilot: RigRunPilot,
    pending: Option<(Uuid, ToolCall)>,
}

impl RigMultiTurnDriver {
    pub fn new(prompt: &str, max_turns: u8) -> Result<Self> {
        Ok(Self {
            pilot: RigRunPilot::new(prompt, max_turns)?,
            pending: None,
        })
    }

    pub async fn drive(
        &mut self,
        task_id: Uuid,
        model_id: &str,
        model: &dyn RigModelBackend,
        tasks: &TaskRegistry,
        registry: &ToolRegistry,
        definitions: &[ToolDefinition],
    ) -> Result<RigDriverStep> {
        if self.pending.is_some() {
            return Err(FoundationError::InvalidTransition);
        }
        if model_id.is_empty() || model_id.len() > 160 || definitions.len() > 32 {
            return Err(FoundationError::InvalidInput(
                "rig driver model or tool limit",
            ));
        }
        let mut names = BTreeSet::new();
        for def in definitions {
            if !registry.is_registered_read_only(&def.name) || !names.insert(def.name.clone()) {
                return Err(FoundationError::PermissionDenied);
            }
        }
        let start = tasks.get(task_id)?;
        if start.status != TaskStatus::Running {
            return Err(FoundationError::InvalidTransition);
        }

        loop {
            let step = self
                .pilot
                .state()
                .next_step()
                .map_err(|_| FoundationError::Provider("RIG_NEXT_STEP".into()))?;
            match step {
                AgentRunStep::CallModel {
                    prompt,
                    history,
                    turn,
                } => {
                    let task = tasks.get(task_id)?;
                    if task.status != TaskStatus::Running {
                        return Err(FoundationError::Cancelled);
                    }
                    let _lease = tasks.acquire_execution(task_id, task.revision)?;
                    let request = CompletionRequest::new(prompt)
                        .messages(history)
                        .model::<String>(Some(model_id.to_owned()))
                        .tools(definitions.to_vec());
                    self.pilot
                        .state()
                        .advertise_tools(turn, definitions.to_vec());
                    let mut cancellation = tasks.cancellation(task_id)?;
                    if *cancellation.borrow_and_update() {
                        return Err(FoundationError::Cancelled);
                    }
                    let response = tokio::select! {
                        reply=tokio::time::timeout(Duration::from_secs(45),model.complete(request)) =>
                            reply.map_err(|_|FoundationError::Timeout)??,
                        _=cancellation.changed()=>return Err(FoundationError::Cancelled),
                    };
                    if !tasks.active_at_revision(task_id, task.revision) {
                        return Err(FoundationError::Cancelled);
                    }
                    if response.choice.len() > 64 {
                        return Err(FoundationError::Provider("RIG_CONTENT_LIMIT".into()));
                    }
                    let turn =
                        ModelTurn::from_response_parts(&response, names.clone(), names.clone());
                    let outcome =
                        self.pilot.state().model_response(turn).map_err(|_| {
                            FoundationError::Provider("RIG_MODEL_TURN_REJECTED".into())
                        })?;
                    if !matches!(outcome, ModelTurnOutcome::Continue { .. }) {
                        return Err(FoundationError::Provider(
                            "RIG_TOOL_REQUIRES_RESOLUTION".into(),
                        ));
                    }
                }
                AgentRunStep::CallTools { calls } => {
                    // Rig may suggest parallel calls. Our single-owner TaskRegistry
                    // explicitly has ONE pending grant; reject all ambiguous batches.
                    if calls.len() != 1 || calls[0].preresolved_result.is_some() {
                        return Err(FoundationError::PermissionDenied);
                    }
                    let call = calls[0].tool_call.clone();
                    if !names.contains(call.function.name.as_ref()) {
                        return Err(FoundationError::PermissionDenied);
                    }
                    let json = serde_json::to_vec(&call.function.arguments)
                        .map_err(|e| FoundationError::Serialization(e.to_string()))?;
                    if json.len() > 16_384 {
                        return Err(FoundationError::InvalidInput("rig tool arguments"));
                    }
                    let tool_name = call.function.name.to_string();
                    let proposed = registry.propose(
                        tasks,
                        task_id,
                        &tool_name,
                        call.function.arguments.clone(),
                    )?;
                    self.pending = Some((proposed, call));
                    return Ok(RigDriverStep::NeedsApproval {
                        call_id: proposed,
                        tool_name,
                    });
                }
                AgentRunStep::Done(response) => {
                    // Validation must precede committing a successful terminal
                    // state; the previous order permanently marked an oversized
                    // error as Completed. Fail closed and retain a Failed event.
                    if response.output.len() > 256 * 1024 {
                        tasks.finish(task_id, false)?;
                        return Err(FoundationError::Provider("RIG_OUTPUT_LIMIT".into()));
                    }
                    tasks.finish(task_id, true)?;
                    return Ok(RigDriverStep::Completed {
                        text: response.output,
                        input_tokens: response.usage.input_tokens,
                        output_tokens: response.usage.output_tokens,
                    });
                }
            }
        }
    }

    /// This accepts ONLY the result returned by the trusted host's
    /// ToolRegistry::execute, AFTER HMAC approval and Owner revalidation.
    /// Caller must NOT use this API with a model-generated fabricated result.
    pub fn accept_host_tool_result(
        &mut self,
        task_id: Uuid,
        call_id: Uuid,
        result: Value,
        tasks: &TaskRegistry,
    ) -> Result<()> {
        let task = tasks.get(task_id)?;
        if task.status != TaskStatus::Running {
            return Err(FoundationError::InvalidTransition);
        }
        let Some((pending, call)) = self.pending.take() else {
            return Err(FoundationError::ApprovalMismatch);
        };
        if pending != call_id {
            self.pending = Some((pending, call));
            return Err(FoundationError::ApprovalMismatch);
        }
        if serde_json::to_vec(&result)
            .map_err(|e| FoundationError::Serialization(e.to_string()))?
            .len()
            > 64 * 1024
        {
            self.pending = Some((pending, call));
            return Err(FoundationError::InvalidInput("rig tool result"));
        }
        self.pilot
            .state()
            .tool_results(vec![UserContent::ToolResult(ToolResult {
                call: call.id,
                name: call.function.name,
                content: vec![ToolResultContent::json(result)],
            })])
            .map_err(|_| FoundationError::Provider("RIG_TOOL_RESULTS".into()))
    }

    pub fn snapshot(&self) -> Result<String> {
        // Never checkpoint a run with a pending host approval: run state and
        // approval must commit atomically in the hosting durable store.
        if self.pending.is_some() {
            return Err(FoundationError::InvalidTransition);
        }
        self.pilot.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        approval::{ApprovalBroker, HostApprovalDecision, HostApprovalIssuer, ResourceBinding},
        event::EventJournal,
        tool::{AgentTool, ToolEffect, ToolFuture},
    };
    use rig_core::{
        completion::Usage,
        message::{AssistantContent, ToolFunction, ToolName},
    };
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    struct FakeBackend(Mutex<Vec<CompletionResponse>>);
    impl RigModelBackend for FakeBackend {
        fn complete<'a>(&'a self, request: CompletionRequest) -> RigCall<'a> {
            Box::pin(async move {
                if request.tools.len() != 1 {
                    return Err(FoundationError::PermissionDenied);
                }
                let mut items = self.0.lock().unwrap();
                if items.is_empty() {
                    return Err(FoundationError::Provider("NO_MORE_RESPONSES".into()));
                }
                Ok(items.remove(0))
            })
        }
    }
    struct ReadTool;
    impl AgentTool for ReadTool {
        fn name(&self) -> &str {
            "demo.inspect"
        }
        fn effect(&self) -> ToolEffect {
            ToolEffect::ReadOnly
        }
        fn call<'a>(&'a self, args: Value) -> ToolFuture<'a> {
            Box::pin(async move { Ok(args) })
        }
        fn validate_binding(&self, b: &crate::approval::ApprovalBinding, _: &Value) -> Result<()> {
            if b.resource.resource_revision == "head-1" {
                Ok(())
            } else {
                Err(FoundationError::PermissionDenied)
            }
        }
    }

    #[tokio::test]
    async fn oversized_completion_cannot_commit_success() {
        let backend = FakeBackend(Mutex::new(vec![CompletionResponse::new(
            vec![AssistantContent::text("X".repeat(256 * 1024 + 2))],
            Usage::default(),
            "mock",
            json!({}),
        )]));
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(16)));
        let task = tasks.create(Uuid::new_v4(), "oversized", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let tools = ToolRegistry::default();
        tools.register(Arc::new(ReadTool)).unwrap();
        let defs = vec![ToolDefinition::new(
            ToolName::new("demo.inspect").unwrap(),
            "Inspect repo",
            json!({"type":"object"}),
        )];
        let mut driver = RigMultiTurnDriver::new("Review", 2).unwrap();
        let outcome = driver
            .drive(task.id, "model-mock", &backend, &tasks, &tools, &defs)
            .await;
        assert_eq!(
            outcome.err(),
            Some(FoundationError::Provider("RIG_OUTPUT_LIMIT".into()))
        );
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Failed);
    }

    #[tokio::test]
    async fn two_model_turns_with_real_rig_state_and_signed_host_approval() {
        let call = ToolCall::from_wire(
            "provider-call-1",
            ToolFunction::new(
                ToolName::new("demo.inspect").unwrap(),
                json!({"repo_id":"test"}),
            ),
        );
        let backend = FakeBackend(Mutex::new(vec![
            CompletionResponse::new(
                vec![AssistantContent::ToolCall(call)],
                Usage::default(),
                "mock",
                json!({}),
            ),
            CompletionResponse::new(
                vec![AssistantContent::text("All done")],
                Usage::default(),
                "mock",
                json!({}),
            ),
        ]));
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(128)));
        let workspace = Uuid::new_v4();
        let task = tasks
            .create_scoped(workspace, Uuid::new_v4(), "Run two turns", vec![])
            .unwrap();
        tasks.start(task.id).unwrap();
        let tools = ToolRegistry::default();
        tools.register(Arc::new(ReadTool)).unwrap();
        let defs = vec![ToolDefinition::new(
            ToolName::new("demo.inspect").unwrap(),
            "Inspect repo",
            json!({"type":"object"}),
        )];
        let mut driver = RigMultiTurnDriver::new("Review", 3).unwrap();
        let first = driver
            .drive(task.id, "model-mock", &backend, &tasks, &tools, &defs)
            .await
            .unwrap();
        let RigDriverStep::NeedsApproval { call_id, .. } = first else {
            panic!("expected approval");
        };
        assert!(driver.snapshot().is_err());
        let receipt = HostApprovalIssuer::new([23; 32])
            .unwrap()
            .issue(
                &tasks,
                task.id,
                call_id,
                HostApprovalDecision {
                    principal_id: Uuid::new_v4(),
                    resource: ResourceBinding {
                        resource_id: format!("workspace:{workspace}"),
                        resource_revision: "head-1".into(),
                    },
                    accepted: true,
                    ttl: Duration::from_secs(30),
                },
            )
            .unwrap();
        let permit = ApprovalBroker::new([23; 32])
            .unwrap()
            .resolve(&tasks, &receipt)
            .unwrap()
            .unwrap();
        let output = tools
            .execute(permit, &tasks, Duration::from_secs(2))
            .await
            .unwrap();
        driver
            .accept_host_tool_result(task.id, call_id, output, &tasks)
            .unwrap();
        let next = driver
            .drive(task.id, "model-mock", &backend, &tasks, &tools, &defs)
            .await
            .unwrap();
        assert!(matches!(next,RigDriverStep::Completed{text,..} if text=="All done"));
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Completed);
        assert!(driver.snapshot().is_ok());
    }
}
