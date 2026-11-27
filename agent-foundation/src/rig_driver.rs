//! Actual Rig AgentRun multi-turn driver. The SDK owns ONLY the prompt/turn
//! algorithm: TaskRegistry, signed host approval, ToolRegistry and native
//! capability owners remain the sole authority for execution.
use crate::{
    rig_run::RigRunPilot,
    task::{TaskRegistry, TaskStatus},
    tool::{ToolExecutionReceipt, ToolRegistry},
    FoundationError, Result,
};
use rig_agent::run::{AgentRunStep, ModelTurn, ModelTurnOutcome};
use rig_core::{
    completion::{CompletionRequest, CompletionResponse, ToolDefinition},
    driver::DynModel,
    message::{AssistantContent, ToolCall, ToolResult, ToolResultContent, UserContent},
    operation::Completion,
};
#[cfg(test)]
use serde_json::Value;
use std::{collections::BTreeSet, future::Future, pin::Pin, sync::Arc, time::Duration};
use uuid::Uuid;

pub type RigCall<'a> = Pin<Box<dyn Future<Output = Result<CompletionResponse>> + Send + 'a>>;
pub trait RigModelBackend: Send + Sync {
    fn complete<'a>(&'a self, request: CompletionRequest) -> RigCall<'a>;

    /// The trusted host may supply a backend without a fixed SDK model ID.
    /// A configured SDK model must not accept a different requested label.
    fn accepts_model(&self, _model_id: &str) -> bool { true }
}

/// A genuine official Rig SDK transport for the multi-turn Driver.
pub struct RigSdkBackend(pub DynModel<Completion>);
impl RigModelBackend for RigSdkBackend {
    fn accepts_model(&self, model_id: &str) -> bool {
        self.0.id().is_none_or(|id| id == model_id)
    }

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

struct DriverOwner {
    registry: Arc<()>,
    tools: Arc<()>,
    definitions_digest: [u8;32],
    task: Uuid,
    revision: u64,
    model: String,
    backend: Arc<dyn RigModelBackend>,
}

// A dropped SDK future cannot leave a still-current Running task with no
// driver. This is local settlement, never proof of remote cancellation.
struct AbandonedRigDrive<'a> {
    tasks: &'a TaskRegistry,
    task: Uuid,
    revision: u64,
    armed: bool,
}
impl Drop for AbandonedRigDrive<'_> {
    fn drop(&mut self) {
        if self.armed && self.tasks.active_at_revision(self.task, self.revision) {
            let _ = self.tasks.finish_at_revision(self.task, false, self.revision);
        }
    }
}

pub struct RigMultiTurnDriver {
    pilot: RigRunPilot,
    pending: Option<(Uuid, Uuid, u64, ToolCall)>,
    owner: Option<DriverOwner>,
    budget: crate::engine::ModelAdmissionBudget,
    call_ids: BTreeSet<String>,
    tool_result_bytes: usize,
    in_flight: bool,
    finished: bool,
}

impl RigMultiTurnDriver {
    pub fn new(prompt: &str, max_turns: u8) -> Result<Self> {
        Ok(Self {
            pilot: RigRunPilot::new(prompt, max_turns)?,
            pending: None,
            owner: None,
            budget: crate::engine::ModelAdmissionBudget::new(crate::engine::TurnLimits::complete_message())?,
            call_ids: BTreeSet::new(),
            tool_result_bytes: 0,
            in_flight: false,
            finished: false,
        })
    }

    pub async fn drive(
        &mut self,
        task_id: Uuid,
        model_id: &str,
        model: Arc<dyn RigModelBackend>,
        tasks: &TaskRegistry,
        registry: &ToolRegistry,
        definitions: &[ToolDefinition],
    ) -> Result<RigDriverStep> {
        if self.pending.is_some() || self.finished || self.in_flight {
            return Err(FoundationError::InvalidTransition);
        }
        if model_id.trim().is_empty() || model_id.len() > 160 || definitions.len() > 32 {
            return Err(FoundationError::InvalidInput(
                "rig driver model or tool limit",
            ));
        }
        if !model.accepts_model(model_id) {
            return Err(FoundationError::PermissionDenied);
        }
        // Bound schema and description material before SDK copies. Exact
        // ordered definitions are part of this run, not a replaceable grant.
        crate::model::serialized_size(definitions,64*1024)?;
        use sha2::{Digest,Sha256};
        let definitions_digest:[u8;32]=Sha256::digest(serde_json::to_vec(definitions)
            .map_err(|_|FoundationError::InvalidInput("rig definitions"))?).into();
        let tools_identity=registry.execution_identity();
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
        let revision = start.revision;
        let identity = tasks.execution_identity();
        if self.owner.as_ref().is_some_and(|owner|
            !Arc::ptr_eq(&owner.registry,&identity)
                || !Arc::ptr_eq(&owner.tools,&tools_identity)
                || !Arc::ptr_eq(&owner.backend, &model)
                || owner.definitions_digest!=definitions_digest || owner.task!=task_id
                || owner.revision!=revision || owner.model!=model_id) {
            return Err(FoundationError::ApprovalMismatch);
        }
        // Keep the same owner across SDK state advancement, provider
        // transport, proposal admission and terminal settlement.
        let limits = crate::engine::TurnLimits {
            deadline: Duration::from_secs(45),
            ..crate::engine::TurnLimits::complete_message()
        };
        let mut gate =
            crate::engine::ModelTurnGate::acquire(tasks, task_id, revision, limits.deadline)?;
        if self.owner.is_none() {
            self.owner=Some(DriverOwner{registry:identity,tools:tools_identity,definitions_digest,
                task:task_id,revision,model:model_id.to_owned(),backend:Arc::clone(&model)});
        }
        // If the awaiting future is dropped, this flag remains set. The
        // partially advanced SDK state is not a resumable host receipt.
        self.in_flight=true;
        let mut abandoned = AbandonedRigDrive { tasks, task: task_id, revision, armed: true };
        let run = async {
            loop {
                gate.active()?;
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
                        let request = CompletionRequest::new(prompt)
                            .messages(history)
                            .model::<String>(Some(model_id.to_owned()))
                            .tools(definitions.to_vec());
                        self.pilot
                            .state()
                            .advertise_tools(turn, definitions.to_vec());
                        let response = gate.receive(model.complete(request)).await?;
                        if response.choice.len() > 64 {
                            return Err(FoundationError::Provider("RIG_CONTENT_LIMIT".into()));
                        }
                        // Budget belongs to the complete SDK run, not a
                        // single invocation between user approval pauses.
                        self.budget.admit_events(response.choice.len())?;
                        let mut suggested=false;
                        for content in response.choice.iter() {
                            match content {
                                AssistantContent::Text(text) => self.budget.admit_text(text.text.len())
                                    .map_err(|_| FoundationError::Provider("RIG_OUTPUT_LIMIT".into()))?,
                                AssistantContent::ToolCall(call) => {
                                    if suggested || !names.contains(call.function.name.as_ref()) {
                                        return Err(FoundationError::PermissionDenied);
                                    }
                                    suggested=true;
                                    let id=call.id.to_string();
                                    if id.is_empty() || id.len()>256 || id.chars().any(|c| c.is_control() || c.is_whitespace())
                                        || !self.call_ids.insert(id) {
                                        return Err(FoundationError::InvalidInput("rig tool call id"));
                                    }
                                    if !call.function.arguments.is_object() {
                                        return Err(FoundationError::InvalidInput("rig tool arguments"));
                                    }
                                    crate::model::validate_serialized_arguments(&call.function.arguments)?;
                                }
                                AssistantContent::Reasoning(_) | AssistantContent::Image(_) =>
                                    return Err(FoundationError::Provider("RIG_UNSUPPORTED_CONTENT".into())),
                            }
                        }
                        gate.active()?;
                        let turn =
                            ModelTurn::from_response_parts(&response, names.clone(), names.clone());
                        let outcome =
                            self.pilot.state().model_response(turn).map_err(|_| {
                                FoundationError::Provider("RIG_MODEL_TURN_REJECTED".into())
                            })?;
                        self.pilot.ensure_snapshot_budget()?;
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
                        crate::model::validate_serialized_arguments(&call.function.arguments)?;
                        let tool_name = call.function.name.to_string();
                        let approved_revision = revision
                            .checked_add(2)
                            .ok_or(FoundationError::InvalidTransition)?;
                        let proposed = registry
                            .propose_at_revision(
                                tasks,
                                task_id,
                                &tool_name,
                                call.function.arguments.clone(),
                                revision,
                            )
                            .map_err(stale_transition)?;
                        self.pending = Some((task_id, proposed, approved_revision, call));
                        return Ok(RigDriverStep::NeedsApproval {
                            call_id: proposed,
                            tool_name,
                        });
                    }
                    AgentRunStep::Done(response) => {
                        // Validation must precede committing a successful terminal
                        // state; the previous order permanently marked an oversized
                        // error as Completed. Fail closed and retain a Failed event.
                        // Raw admitted text is already counted cumulatively.
                        // Independently bound the SDK's assembled final text.
                        if response.output.len()>limits.max_text_bytes {
                            tasks
                                .finish_at_revision(task_id, false, revision)
                                .map_err(stale_transition)?;
                            return Err(FoundationError::Provider("RIG_OUTPUT_LIMIT".into()));
                        }
                        tasks
                            .finish_at_revision(task_id, true, revision)
                            .map_err(stale_transition)?;
                        return Ok(RigDriverStep::Completed {
                            text: response.output,
                            input_tokens: response.usage.input_tokens,
                            output_tokens: response.usage.output_tokens,
                        });
                    }
                }
            }
        };
        let result = run.await;
        abandoned.armed = false;
        self.in_flight=false;
        if result.is_err() {
            self.finished=true;
            if tasks.active_at_revision(task_id, revision) {
                let _ = tasks.finish_at_revision(task_id, false, revision);
            }
        } else if matches!(&result, Ok(RigDriverStep::Completed { .. })) {
            self.finished=true;
        }
        result
    }

    /// Only the opaque execution receipt returned by ToolRegistry can
    /// advance a pending Rig turn. Approval alone is not execution.
    pub fn accept_host_tool_result(
        &mut self,
        receipt: &ToolExecutionReceipt<'_>,
        tasks: &TaskRegistry,
    ) -> Result<()> {
        let Some((task_id, call_id, approved_revision, call)) = self.pending.as_ref() else {
            return Err(FoundationError::ApprovalMismatch);
        };
        let (task_id, call_id, approved_revision) = (*task_id, *call_id, *approved_revision);
        if self.finished || self.in_flight || !self.owner.as_ref().is_some_and(|owner|
            Arc::ptr_eq(&owner.registry,&tasks.execution_identity()) && owner.task==task_id
                && owner.revision.checked_add(2)==Some(approved_revision)
                && receipt.belongs_to_tool_registry(&owner.tools))
            || !receipt.matches_approval(tasks, task_id, call_id, approved_revision) {
            return Err(FoundationError::ApprovalMismatch);
        }
        // A matching receipt proves execution already occurred. An oversized
        // result cannot be re-executed to repair the SDK continuation.
        // Acquire the same task revision before settling a terminal rejection.
        let _lease = tasks.acquire_execution(task_id, approved_revision)?;
        let bytes = match crate::model::serialized_size(receipt.value(),
            (64 * 1024).min((256 * 1024usize).saturating_sub(self.tool_result_bytes))) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.finished = true;
                self.pending = None;
                let _ = tasks.finish_at_revision(task_id, false, approved_revision);
                return Err(FoundationError::InvalidInput("rig tool result"));
            }
        };
        let mut entered=false;
        let accepted=tasks.with_active_revision(task_id, approved_revision, || {
            entered=true;
            self.pilot
                .state()
                .tool_results(vec![UserContent::ToolResult(ToolResult {
                    call: call.id.clone(),
                    name: call.function.name.clone(),
                    content: vec![ToolResultContent::json(receipt.value().clone())],
                })])
                .map_err(|_| FoundationError::Provider("RIG_TOOL_RESULTS".into()))?;
            self.pilot.ensure_snapshot_budget()
        });
        if let Err(error)=accepted {
            // A rejected SDK transition may already have changed SDK state.
            // A failure before entry still leaves the pending call untouched.
            if entered {
                self.finished=true;
                let _=tasks.finish_at_revision(task_id,false,approved_revision);
            }
            return Err(error);
        }
        self.tool_result_bytes+=bytes;
        // Only a consumed, matching tool receipt may advance the driver's
        // expected task revision. A pause/resume or restored task cannot.
        self.owner.as_mut().ok_or(FoundationError::ApprovalMismatch)?.revision=approved_revision;
        self.pending = None;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<String> {
        // Never checkpoint a run with a pending host approval: run state and
        // approval must commit atomically in the hosting durable store.
        if self.pending.is_some() || self.in_flight
            || (self.finished && self.owner.is_none()) {
            return Err(FoundationError::InvalidTransition);
        }
        self.pilot.snapshot()
    }
}

fn stale_transition(error: FoundationError) -> FoundationError {
    if error == FoundationError::InvalidTransition {
        FoundationError::Cancelled
    } else {
        error
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

    async fn approved_read_receipt<'a>(
        tasks: &'a TaskRegistry,
        tools: &ToolRegistry,
        task_id: Uuid,
        call_id: Uuid,
    ) -> ToolExecutionReceipt<'a> {
        let task = tasks.get(task_id).unwrap();
        let issuer = HostApprovalIssuer::new([23; 32]).unwrap();
        let approval = issuer
            .issue(
                tasks,
                task_id,
                call_id,
                HostApprovalDecision {
                    principal_id: Uuid::new_v4(),
                    resource: ResourceBinding {
                        resource_id: format!("workspace:{}", task.workspace_id),
                        resource_revision: "head-1".into(),
                    },
                    accepted: true,
                    ttl: Duration::from_secs(30),
                },
            )
            .unwrap();
        let permit = ApprovalBroker::new([23; 32])
            .unwrap()
            .resolve(tasks, &approval)
            .unwrap()
            .unwrap();
        tools
            .execute_receipted(permit, tasks, Duration::from_secs(2))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn stale_tool_result_cannot_resume_a_new_task_generation() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let task = tasks
            .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "test", vec![])
            .unwrap();
        tasks.start(task.id).unwrap();
        let tools = ToolRegistry::default();
        tools.register(Arc::new(ReadTool)).unwrap();
        let call_id = tools
            .propose(&tasks, task.id, "demo.inspect", json!({}))
            .unwrap();
        let receipt = approved_read_receipt(&tasks, &tools, task.id, call_id).await;
        let revision = tasks.get(task.id).unwrap().revision;
        let call = ToolCall::from_wire(
            "test-call",
            ToolFunction::new(ToolName::new("demo.inspect").unwrap(), json!({})),
        );
        let mut driver = RigMultiTurnDriver::new("Review", 2).unwrap();
        driver.owner=Some(DriverOwner{registry:tasks.execution_identity(),tools:tools.execution_identity(),
            definitions_digest:[0;32],task:task.id,
            revision:revision.checked_sub(2).unwrap(),model:"model-mock".into(),
            backend:Arc::new(FakeBackend(Mutex::new(vec![])))});
        driver.pending = Some((task.id, call_id, revision, call));
        tasks.pause(task.id).unwrap();
        tasks.start(task.id).unwrap();
        assert_eq!(
            driver
                .accept_host_tool_result(&receipt, &tasks)
                .err(),
            Some(FoundationError::InvalidTransition)
        );
        assert!(
            driver.pending.is_some(),
            "stale result must not consume the pending operation"
        );
    }

    #[tokio::test]
    async fn oversized_completion_cannot_commit_success() {
        let backend: Arc<dyn RigModelBackend> = Arc::new(FakeBackend(Mutex::new(vec![CompletionResponse::new(
            vec![AssistantContent::text("X".repeat(256 * 1024 + 2))],
            Usage::default(),
            "mock",
            json!({}),
        )])));
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
            .drive(task.id, "model-mock", Arc::clone(&backend), &tasks, &tools, &defs)
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
        let backend: Arc<dyn RigModelBackend> = Arc::new(FakeBackend(Mutex::new(vec![
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
        ])));
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
            .drive(task.id, "model-mock", Arc::clone(&backend), &tasks, &tools, &defs)
            .await
            .unwrap();
        let RigDriverStep::NeedsApproval { call_id, .. } = first else {
            panic!("expected approval");
        };
        assert!(driver.snapshot().is_err());
        let other = tasks
            .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "Other task", vec![])
            .unwrap();
        tasks.start(other.id).unwrap();
        let other_call = tools
            .propose(&tasks, other.id, "demo.inspect", json!({}))
            .unwrap();
        let wrong = approved_read_receipt(&tasks, &tools, other.id, other_call).await;
        assert_eq!(
            driver.accept_host_tool_result(&wrong, &tasks).err(),
            Some(FoundationError::ApprovalMismatch)
        );
        assert!(
            driver.snapshot().is_err(),
            "wrong-task result must not consume the pending operation"
        );
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
            .execute_receipted(permit, &tasks, Duration::from_secs(2))
            .await
            .unwrap();
        let competing = tasks
            .acquire_execution(task.id, tasks.get(task.id).unwrap().revision)
            .unwrap();
        assert_eq!(
            driver.accept_host_tool_result(&output, &tasks).err(),
            Some(FoundationError::InvalidTransition)
        );
        drop(competing);
        driver.accept_host_tool_result(&output, &tasks).unwrap();
        assert_eq!(
            driver.accept_host_tool_result(&output, &tasks).err(),
            Some(FoundationError::ApprovalMismatch)
        );
        let next = driver
            .drive(task.id, "model-mock", Arc::clone(&backend), &tasks, &tools, &defs)
            .await
            .unwrap();
        assert!(matches!(next,RigDriverStep::Completed{text,..} if text=="All done"));
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Completed);
        assert!(driver.snapshot().is_ok());
    }
}
