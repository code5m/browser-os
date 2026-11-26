//! Shared Foundation execution boundary for streaming model turns and
//! bounded complete-message model/tool loops. Native adapters retain their
//! existing capability ownership, durable receipts and approval authority.
//! This module is not an autonomous planner, ACP server or MCP transport.
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

/// Shared model transport gate for Foundation streaming and experimental Rig
/// adapters. One revision/lease and one absolute deadline per model turn.
/// Dropping a provider future is only a cooperative transport cancellation;
/// this gate does not prove a remote provider or external effect stopped.
pub(crate) struct ModelTurnGate<'a> {
    deadline: tokio::time::Instant,
    cancellation: tokio::sync::watch::Receiver<bool>,
    _lease: crate::task::ExecutionLease<'a>,
}
impl<'a> ModelTurnGate<'a> {
    pub(crate) fn acquire(
        tasks: &'a TaskRegistry,
        id: Uuid,
        revision: u64,
        limit: Duration,
    ) -> Result<Self> {
        let lease = tasks.acquire_execution(id, revision)?;
        let cancellation = tasks.cancellation(id)?;
        let gate = Self {
            deadline: tokio::time::Instant::now() + limit,
            cancellation,
            _lease: lease,
        };
        gate.active()?;
        Ok(gate)
    }
    pub(crate) fn active(&self) -> Result<()> {
        if *self.cancellation.borrow() || !self._lease.is_active() {
            return Err(FoundationError::Cancelled);
        }
        Ok(())
    }
    pub(crate) async fn receive<T>(
        &mut self,
        future: impl std::future::Future<Output = Result<T>>,
    ) -> Result<T> {
        self.active()?;
        let deadline = self.deadline;
        if tokio::time::Instant::now() >= deadline {
            return Err(FoundationError::Timeout);
        }
        let result = tokio::select! {
            biased;
            _ = self.cancellation.changed() => Err(FoundationError::Cancelled),
            _ = tokio::time::sleep_until(deadline) => Err(FoundationError::Timeout),
            response = future => response,
        };
        self.active()?;
        result
    }
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
        let mut gate = ModelTurnGate::acquire(self.tasks, task_id, revision, self.limits.deadline)?;
        let request = ModelRequest {
            session_id: task.session_id,
            model_id: model_id.into(),
            prompt: prompt.into(),
            context: task.context,
        };
        request.validate()?;
        let run = async {
            let mut stream = gate.receive(self.provider.stream(request)).await?;
            let mut text = String::new();
            let mut usage = None;
            let mut count = 0usize;

            loop {
                let event = gate.receive(async { Ok(stream.recv().await) }).await?;
                count += 1;
                if count > self.limits.max_model_events {
                    return Err(FoundationError::Provider("MODEL_EVENT_LIMIT".to_owned()));
                }
                match event {
                    Some(ModelEvent::TextDelta { text: delta }) => {
                        if delta.len() > self.limits.max_text_bytes.saturating_sub(text.len()) {
                            return Err(FoundationError::Provider("MODEL_TEXT_LIMIT".to_owned()));
                        }
                        self.tasks.record_event_at_revision(
                            task_id,
                            revision,
                            EventKind::ModelProgress {
                                bytes: delta.len() as u64,
                            },
                        )?;
                        text.push_str(&delta);
                    }
                    Some(ModelEvent::ToolSuggestion {
                        tool_name,
                        arguments_json,
                    }) => {
                        let arguments = crate::model::parse_tool_arguments(&arguments_json)?;
                        let call_id = self
                            .tools
                            .propose_at_revision(
                                self.tasks, task_id, &tool_name, arguments, revision,
                            )
                            .map_err(|error| {
                                if error == FoundationError::InvalidTransition {
                                    FoundationError::Cancelled
                                } else {
                                    error
                                }
                            })?;
                        return Ok(TurnResult {
                            text,
                            completion: TurnCompletion::NeedsApproval { call_id },
                            usage,
                        });
                    }
                    Some(ModelEvent::Usage {
                        input_tokens,
                        output_tokens,
                    }) => {
                        usage = Some((input_tokens, output_tokens));
                    }
                    Some(ModelEvent::Finished) => {
                        self.tasks
                            .finish_at_revision(task_id, true, revision)
                            .map_err(|error| {
                                if error == FoundationError::InvalidTransition {
                                    FoundationError::Cancelled
                                } else {
                                    error
                                }
                            })?;
                        return Ok(TurnResult {
                            text,
                            completion: TurnCompletion::Completed,
                            usage,
                        });
                    }
                    Some(ModelEvent::Failed { .. }) => {
                        return Err(FoundationError::Provider(
                            "MODEL_REPORTED_FAILURE".to_owned(),
                        ));
                    }
                    None => {
                        return Err(FoundationError::Provider("MODEL_STREAM_ENDED".to_owned()));
                    }
                }
            }
        };
        let result = run.await;
        if result.is_err() && self.tasks.active_at_revision(task_id, revision) {
            // Only the same active revision can be failed; never override Pause or Cancel.
            let _ = self.tasks.finish_at_revision(task_id, false, revision);
        }
        result
    }
}

/// Production-compatible complete-message adapter for the existing engine.
/// Streaming and complete-message transports have different cancellation and
/// transcript semantics. Host adapters retain their real owner/lease and
/// capability authorization; this driver owns bounded round progression,
/// cancellation boundaries and complete function-batch admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchFlow {
    Continue,
    Stop,
}

pub trait FunctionBatchHost {
    type Error;
    fn cancelled(&self) -> bool;
    fn on_cancelled(&mut self) -> std::result::Result<(), Self::Error>;
    fn request(&mut self) -> std::result::Result<serde_json::Value, Self::Error>;
    fn provider_failure(&mut self, error: Self::Error) -> Self::Error;
    fn validate_reply(&mut self, reply: &serde_json::Value)
        -> std::result::Result<(), Self::Error>;
    fn invalid_batch(&mut self, error: FoundationError) -> Self::Error;
    /// An accepted batch must be paired completely before committing its
    /// transcript. Model-selected calls are not authorization for effects.
    fn accept_batch(
        &mut self,
        round: usize,
        last: bool,
        reply: serde_json::Value,
        calls: Vec<crate::model::FunctionCall>,
    ) -> std::result::Result<BatchFlow, Self::Error>;
}

pub fn drive_function_batches<H: FunctionBatchHost>(
    host: &mut H,
    rounds: usize,
    max_calls: usize,
) -> std::result::Result<(), H::Error> {
    drive_function_batches_checked(host, rounds, max_calls, || true)
}

/// Production complete-message hosts retain their actual native execution
/// lease across the synchronous provider and tool batch. Do not acquire a
/// second competing lease or admit another batch on a stale generation.
pub fn drive_owned_function_batches<H: FunctionBatchHost>(
    host: &mut H,
    lease: &crate::task::ExecutionLease<'_>,
    rounds: usize,
    max_calls: usize,
) -> std::result::Result<(), H::Error> {
    drive_function_batches_checked(host, rounds, max_calls, || lease.is_active())
}

fn drive_function_batches_checked<H: FunctionBatchHost>(
    host: &mut H,
    rounds: usize,
    max_calls: usize,
    active: impl Fn() -> bool,
) -> std::result::Result<(), H::Error> {
    if !(1..=8).contains(&rounds) || max_calls > 8 {
        return Err(host.invalid_batch(FoundationError::InvalidInput("function turn limits")));
    }
    // IDs must remain unique throughout this bounded model/tool loop,
    // not only within a single assistant response.
    let mut admitted_ids = std::collections::HashSet::new();
    for round in 0..rounds {
        if !active() || host.cancelled() {
            return host.on_cancelled();
        }
        let reply = match host.request() {
            Ok(value) => value,
            Err(error) => {
                if !active() || host.cancelled() {
                    return host.on_cancelled();
                }
                return Err(host.provider_failure(error));
            }
        };
        if !active() || host.cancelled() {
            return host.on_cancelled();
        }
        host.validate_reply(&reply)?;
        let calls = crate::model::parse_function_batch(&reply, max_calls)
            .map_err(|error| host.invalid_batch(error))?;
        if !active() || host.cancelled() {
            return host.on_cancelled();
        }
        if calls
            .iter()
            .any(|call| !admitted_ids.insert(call.id.clone()))
        {
            return Err(host.invalid_batch(FoundationError::InvalidInput(
                "duplicate tool call id across rounds",
            )));
        }
        let flow = host.accept_batch(round, round + 1 == rounds, reply, calls)?;
        // No successful completion or next round may be accepted after a
        // host callback has returned on a superseded task generation.
        if !active() || host.cancelled() {
            return host.on_cancelled();
        }
        match flow {
            BatchFlow::Stop => return Ok(()),
            BatchFlow::Continue if round + 1 < rounds => {}
            BatchFlow::Continue => {
                return Err(host.invalid_batch(FoundationError::InvalidTransition))
            }
        }
    }
    Ok(())
}

/// Shared text-only adapter for remote and local complete-message providers.
/// The transport supplies a normalized assistant message, never a tool grant.
struct TextOnlyHost<P, C, M> {
    request: P,
    cancelled: C,
    error: M,
    result: Option<String>,
}
impl<E, P, C, M> FunctionBatchHost for TextOnlyHost<P, C, M>
where
    P: FnMut() -> std::result::Result<serde_json::Value, E>,
    C: Fn() -> bool,
    M: Fn(FoundationError) -> E,
{
    type Error = E;
    fn cancelled(&self) -> bool {
        (self.cancelled)()
    }
    fn on_cancelled(&mut self) -> std::result::Result<(), E> {
        Err((self.error)(FoundationError::Cancelled))
    }
    fn request(&mut self) -> std::result::Result<serde_json::Value, E> {
        (self.request)()
    }
    fn provider_failure(&mut self, error: E) -> E {
        error
    }
    fn validate_reply(&mut self, _: &serde_json::Value) -> std::result::Result<(), E> {
        Ok(())
    }
    fn invalid_batch(&mut self, error: FoundationError) -> E {
        (self.error)(error)
    }
    fn accept_batch(
        &mut self,
        _: usize,
        _: bool,
        reply: serde_json::Value,
        calls: Vec<crate::model::FunctionCall>,
    ) -> std::result::Result<BatchFlow, E> {
        if !calls.is_empty() {
            return Err((self.error)(FoundationError::PermissionDenied));
        }
        self.result = Some(
            reply["content"]
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| (self.error)(FoundationError::Provider("MODEL_TEXT_EMPTY".into())))?
                .to_owned(),
        );
        Ok(BatchFlow::Stop)
    }
}
/// Non-production convenience adapter. Production text callers must pass
/// the existing task lease to drive_owned_text_turn instead.
pub fn drive_text_turn<E>(
    request: impl FnMut() -> std::result::Result<serde_json::Value, E>,
    cancelled: impl Fn() -> bool,
    error: impl Fn(FoundationError) -> E,
) -> std::result::Result<String, E> {
    drive_text_turn_checked(request, cancelled, error, || true)
}

pub fn drive_owned_text_turn<E>(
    lease: &crate::task::ExecutionLease<'_>,
    request: impl FnMut() -> std::result::Result<serde_json::Value, E>,
    cancelled: impl Fn() -> bool,
    error: impl Fn(FoundationError) -> E,
) -> std::result::Result<String, E> {
    drive_text_turn_checked(request, cancelled, error, || lease.is_active())
}

fn drive_text_turn_checked<E>(
    request: impl FnMut() -> std::result::Result<serde_json::Value, E>,
    cancelled: impl Fn() -> bool,
    error: impl Fn(FoundationError) -> E,
    active: impl Fn() -> bool,
) -> std::result::Result<String, E> {
    let mut host = TextOnlyHost {
        request,
        cancelled,
        error,
        result: None,
    };
    drive_function_batches_checked(&mut host, 1, 0, &active)?;
    if !active() || (host.cancelled)() {
        return Err((host.error)(FoundationError::Cancelled));
    }
    host.result
        .ok_or_else(|| (host.error)(FoundationError::Provider("MODEL_TEXT_EMPTY".into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::EventJournal,
        model::{ModelFuture, ModelStream},
        tool::{AgentTool, ToolEffect, ToolFuture},
    };
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    struct BatchFixture {
        cancelled: bool,
        replies: Vec<Value>,
        admitted: usize,
    }
    impl FunctionBatchHost for BatchFixture {
        type Error = FoundationError;
        fn cancelled(&self) -> bool {
            self.cancelled
        }
        fn on_cancelled(&mut self) -> Result<()> {
            Err(FoundationError::Cancelled)
        }
        fn request(&mut self) -> Result<Value> {
            self.replies.pop().ok_or(FoundationError::InvalidTransition)
        }
        fn provider_failure(&mut self, error: FoundationError) -> FoundationError {
            error
        }
        fn validate_reply(&mut self, _: &Value) -> Result<()> {
            Ok(())
        }
        fn invalid_batch(&mut self, error: FoundationError) -> FoundationError {
            error
        }
        fn accept_batch(
            &mut self,
            _: usize,
            last: bool,
            _: Value,
            calls: Vec<crate::model::FunctionCall>,
        ) -> Result<BatchFlow> {
            self.admitted += calls.len();
            Ok(if last {
                BatchFlow::Stop
            } else {
                BatchFlow::Continue
            })
        }
    }
    #[test]
    fn complete_batch_driver_refuses_invalid_suffix_before_host_admission() {
        let mut host = BatchFixture {
            cancelled: false,
            admitted: 0,
            replies: vec![json!({
                "role":"assistant", "tool_calls":[
                    {"id":"one","function":{"name":"read_file","arguments":"{}"}},
                    {"id":"one","function":{"name":"write_file","arguments":"{}"}}
                ]
            })],
        };
        assert!(drive_function_batches(&mut host, 1, 8).is_err());
        assert_eq!(host.admitted, 0);
        host.cancelled = true;
        assert_eq!(
            drive_function_batches(&mut host, 1, 8),
            Err(FoundationError::Cancelled)
        );
    }

    #[test]
    fn complete_batch_driver_rejects_reused_call_id_before_second_admission() {
        let reply = json!({"role":"assistant","tool_calls":[
            {"id":"one","function":{"name":"read_file","arguments":"{}"}}
        ]});
        let mut host = BatchFixture {
            cancelled: false,
            admitted: 0,
            replies: vec![reply.clone(), reply],
        };
        assert!(drive_function_batches(&mut host, 2, 8).is_err());
        assert_eq!(host.admitted, 1);
    }

    #[test]
    fn owned_text_driver_rejects_a_return_after_task_generation_changes() {
        let tasks = TaskRegistry::new(std::sync::Arc::new(crate::event::EventJournal::new(32)));
        let task = tasks.create(Uuid::new_v4(), "text", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let lease = tasks
            .acquire_execution(task.id, tasks.get(task.id).unwrap().revision)
            .unwrap();
        let result = drive_owned_text_turn(
            &lease,
            || {
                tasks.pause(task.id).unwrap();
                Ok(serde_json::json!({"role":"assistant","content":"stale"}))
            },
            || false,
            |error| error,
        );
        assert_eq!(result, Err(FoundationError::Cancelled));
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Paused);
        let result = drive_owned_text_turn(
            &lease,
            || panic!("stale lease must never contact a provider"),
            || false,
            |error| error,
        );
        assert_eq!(result, Err(FoundationError::Cancelled));
    }

    #[test]
    fn owned_batch_driver_rejects_stale_task_before_provider_or_host() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(16)));
        let task = tasks.create(Uuid::new_v4(), "fixture", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let lease = tasks
            .acquire_execution(task.id, tasks.get(task.id).unwrap().revision)
            .unwrap();
        tasks.pause(task.id).unwrap();
        assert!(!lease.is_active());
        let mut host = BatchFixture {
            cancelled: false,
            replies: vec![json!({"role":"assistant","content":"unused"})],
            admitted: 0,
        };
        assert_eq!(
            drive_owned_function_batches(&mut host, &lease, 1, 0),
            Err(FoundationError::Cancelled)
        );
        assert_eq!(host.replies.len(), 1);
        assert_eq!(host.admitted, 0);
    }

    struct ScriptedProvider(Vec<ModelEvent>);
    impl ModelProvider for ScriptedProvider {
        fn stream<'a>(&'a self, request: ModelRequest) -> ModelFuture<'a> {
            let events = self.0.clone();
            Box::pin(async move {
                request.validate()?;
                let (sender, receiver): (_, ModelStream) = mpsc::channel(8);
                for event in events {
                    sender
                        .send(event)
                        .await
                        .map_err(|_| FoundationError::Cancelled)?;
                }
                Ok(receiver)
            })
        }
    }
    struct ReadOnlyTool;
    impl AgentTool for ReadOnlyTool {
        fn name(&self) -> &str {
            "editor.read"
        }
        fn effect(&self) -> ToolEffect {
            ToolEffect::ReadOnly
        }
        fn call<'a>(&'a self, arguments: Value) -> ToolFuture<'a> {
            Box::pin(async move { Ok(arguments) })
        }
    }

    #[tokio::test]
    async fn a_tool_suggestion_always_waits_for_host_approval() {
        let provider = ScriptedProvider(vec![
            ModelEvent::TextDelta {
                text: "Inspecting".into(),
            },
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
        let engine = AgentEngine {
            provider: &provider,
            tools: &tools,
            tasks: &tasks,
            limits: TurnLimits::default(),
        };
        let result = engine.drive_turn(task.id, "mock", "inspect").await.unwrap();
        assert_eq!(result.text, "Inspecting");
        assert_eq!(
            tasks.get(task.id).unwrap().status,
            TaskStatus::AwaitingApproval
        );
        let TurnCompletion::NeedsApproval { call_id } = result.completion else {
            panic!("model must not auto-run a proposed tool");
        };
        assert!(journal
            .replay(task.id, 0)
            .unwrap()
            .iter()
            .any(|event| { matches!(&event.kind, EventKind::ApprovalRequested { .. }) }));
        assert!(tasks
            .resolve_approval(task.id, call_id, false)
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn completed_stream_finishes_task() {
        let provider = ScriptedProvider(vec![
            ModelEvent::TextDelta {
                text: "Done".into(),
            },
            ModelEvent::Finished,
        ]);
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let tools = ToolRegistry::default();
        let task = tasks.create(Uuid::new_v4(), "Plan", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let engine = AgentEngine {
            provider: &provider,
            tools: &tools,
            tasks: &tasks,
            limits: TurnLimits::default(),
        };
        let response = engine.drive_turn(task.id, "mock", "plan").await.unwrap();
        assert_eq!(response.completion, TurnCompletion::Completed);
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn unregistered_tools_fail_closed() {
        let provider = ScriptedProvider(vec![ModelEvent::ToolSuggestion {
            tool_name: "shell.exec".into(),
            arguments_json: "{}".into(),
        }]);
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let tools = ToolRegistry::default();
        let task = tasks.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let engine = AgentEngine {
            provider: &provider,
            tools: &tools,
            tasks: &tasks,
            limits: TurnLimits::default(),
        };
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
    struct ConnectingProvider(Arc<tokio::sync::Notify>);
    impl ModelProvider for ConnectingProvider {
        fn stream<'a>(&'a self, _request: ModelRequest) -> ModelFuture<'a> {
            Box::pin(async move {
                self.0.notify_one();
                std::future::pending::<Result<ModelStream>>().await
            })
        }
    }
    #[tokio::test]
    async fn cancellation_interrupts_provider_connection() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let tools = ToolRegistry::default();
        let task = tasks.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let connected = Arc::new(tokio::sync::Notify::new());
        let provider = ConnectingProvider(connected.clone());
        let engine = AgentEngine {
            provider: &provider,
            tools: &tools,
            tasks: &tasks,
            limits: TurnLimits::default(),
        };
        let cancel = async {
            connected.notified().await;
            tasks.cancel(task.id).unwrap();
        };
        let (result, ()) = tokio::join!(
            tokio::time::timeout(
                Duration::from_millis(500),
                engine.drive_turn(task.id, "mock", "work")
            ),
            cancel
        );
        assert_eq!(
            result
                .expect("cancellation must not wait for provider timeout")
                .err(),
            Some(FoundationError::Cancelled)
        );
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Cancelled);
    }
    #[tokio::test]
    async fn timeout_is_a_terminal_failure_not_a_stuck_task() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let tools = ToolRegistry::default();
        let task = tasks.create(Uuid::new_v4(), "Task", vec![]).unwrap();
        tasks.start(task.id).unwrap();
        let engine = AgentEngine {
            provider: &HangingProvider,
            tools: &tools,
            tasks: &tasks,
            limits: TurnLimits {
                deadline: Duration::from_millis(1),
                ..TurnLimits::default()
            },
        };
        assert_eq!(
            engine.drive_turn(task.id, "mock", "work").await.err(),
            Some(FoundationError::Timeout)
        );
        assert_eq!(tasks.get(task.id).unwrap().status, TaskStatus::Failed);
    }
}
