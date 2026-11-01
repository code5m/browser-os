use browseros_agent_channel::agent::{
    AgentRequest, AgentTask, TaskAction, TaskError, TaskState,
};
use browseros_agent_channel::context::{ContextAccess, ContextKind, ContextRef};
use browseros_agent_channel::events::{EventError, EventKind, EventLog};
use browseros_agent_channel::interop::AgentTransport;
use browseros_agent_channel::model::{
    ModelError, ModelMessage, ModelRequest, ModelRole, ModelTool,
};
use browseros_agent_channel::tools::{
    Approval, ToolCall, ToolCatalog, ToolError, ToolRisk, ToolSpec,
};
use serde_json::json;

fn sample_request() -> ModelRequest {
    ModelRequest {
        model: "example-model".into(),
        messages: vec![ModelMessage {
            role: ModelRole::User,
            content: "Summarize this selected page".into(),
            tool_call_id: None,
        }],
        tools: vec![],
        max_output_tokens: Some(128),
    }
}

fn sample_tools() -> ToolCatalog {
    let mut catalog = ToolCatalog::default();
    catalog.register(ToolSpec {
        name: "editor.read".into(),
        description: "Read an explicitly scoped file".into(),
        input_schema: json!({"type": "object"}),
        risk: ToolRisk::Read,
    }).unwrap();
    catalog
}

fn read_call() -> ToolCall {
    ToolCall {
        request_id: "call-1".into(),
        name: "editor.read".into(),
        scope: "project-1".into(),
        arguments: json!({"path": "README.md"}),
    }
}

fn allow(call: &ToolCall) -> Approval {
    Approval::AllowOnce {
        request_id: call.request_id.clone(),
        name: call.name.clone(),
        scope: call.scope.clone(),
    }
}

#[test]
fn model_payload_is_text_only_and_streaming() {
    let body = sample_request().to_chat_completions_body().unwrap();
    assert_eq!(body["model"], "example-model");
    assert_eq!(body["stream"], true);
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["max_completion_tokens"], 128);
    assert!(body.get("tools").is_none());
}

#[test]
fn model_tools_are_explicit_and_validated() {
    let mut request = sample_request();
    request.tools.push(ModelTool {
        name: "browser.read".into(),
        description: "Read a selected tab".into(),
        input_schema: json!({"type": "object"}),
    });
    let body = request.to_chat_completions_body().unwrap();
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["function"]["name"], "browser.read");
    request.tools[0].input_schema = json!(true);
    assert!(matches!(request.validate(), Err(ModelError::InvalidRequest(_))));
}

#[test]
fn model_requires_a_tool_call_id_for_tool_messages() {
    let mut request = sample_request();
    request.messages[0].role = ModelRole::Tool;
    assert!(request.validate().is_err());
    request.messages[0].tool_call_id = Some("call-1".into());
    assert!(request.validate().is_ok());
}

#[test]
fn task_has_a_guarded_state_machine() {
    let mut task = AgentTask::new("t", "s", "a").unwrap();
    assert_eq!(task.state(), TaskState::Created);
    assert!(matches!(
        task.apply(TaskAction::Succeed),
        Err(TaskError::InvalidTransition { .. })
    ));
    assert_eq!(task.revision(), 0);
    assert_eq!(task.apply(TaskAction::Start).unwrap(), TaskState::Running);
    assert_eq!(task.apply(TaskAction::Pause).unwrap(), TaskState::Paused);
    assert_eq!(task.apply(TaskAction::Resume).unwrap(), TaskState::Running);
    assert_eq!(task.apply(TaskAction::Succeed).unwrap(), TaskState::Succeeded);
    assert!(task.apply(TaskAction::Start).is_err());
    assert_eq!(task.revision(), 4);
}

#[test]
fn approval_is_a_distinct_task_state() {
    let mut task = AgentTask::new("t", "s", "a").unwrap();
    task.apply(TaskAction::Start).unwrap();
    task.apply(TaskAction::RequestApproval).unwrap();
    assert_eq!(task.state(), TaskState::WaitingApproval);
    assert!(task.apply(TaskAction::Succeed).is_err());
    task.apply(TaskAction::Approve).unwrap();
    assert_eq!(task.state(), TaskState::Running);
}

#[test]
fn task_can_cancel_while_waiting() {
    let mut task = AgentTask::new("t", "s", "a").unwrap();
    task.apply(TaskAction::Start).unwrap();
    task.apply(TaskAction::RequestApproval).unwrap();
    assert_eq!(task.apply(TaskAction::Cancel).unwrap(), TaskState::Cancelled);
    assert!(task.apply(TaskAction::Approve).is_err());
}

#[test]
fn context_is_explicit_reference_only() {
    assert!(ContextRef::new(ContextKind::BrowserTab, "", ContextAccess::UserSelected).is_err());
    let context = ContextRef::new(
        ContextKind::EditorSelection,
        "editor-selection-1",
        ContextAccess::ExplicitlyApproved,
    ).unwrap();
    let request = AgentRequest {
        task_id: "task-1".into(),
        session_id: "session-1".into(),
        instruction: "Review the selected code".into(),
        contexts: vec![context],
    };
    assert!(request.validate().is_ok());
}

#[test]
fn tool_gate_defaults_to_deny_even_for_reads() {
    let mut catalog = sample_tools();
    let call = read_call();
    assert_eq!(catalog.authorize(call, Approval::Deny), Err(ToolError::Denied));
}

#[test]
fn tool_gate_rejects_unknown_tools_and_mismatched_scopes() {
    let mut catalog = sample_tools();
    let mut call = read_call();
    call.name = "terminal.exec".into();
    assert_eq!(
        catalog.authorize(call.clone(), allow(&call)),
        Err(ToolError::UnknownTool),
    );
    let read = read_call();
    let grant = Approval::AllowOnce {
        request_id: read.request_id.clone(),
        name: read.name.clone(),
        scope: "another-project".into(),
    };
    assert_eq!(catalog.authorize(read, grant), Err(ToolError::ApprovalMismatch));
}

#[test]
fn approvals_are_scoped_and_one_shot() {
    let mut catalog = sample_tools();
    let call = read_call();
    let authorized = catalog.authorize(call.clone(), allow(&call)).unwrap();
    assert_eq!(authorized.call().scope, "project-1");
    assert_eq!(
        catalog.authorize(call.clone(), allow(&call)),
        Err(ToolError::ApprovalUsed)
    );
    assert_eq!(catalog.specs().count(), 1);
}

#[test]
fn catalog_rejects_invalid_and_duplicate_tool_definitions() {
    let mut catalog = sample_tools();
    assert_eq!(catalog.register(ToolSpec {
        name: "editor.read".into(),
        description: "duplicate".into(),
        input_schema: json!({}),
        risk: ToolRisk::Read,
    }), Err(ToolError::DuplicateTool));
    assert_eq!(catalog.register(ToolSpec {
        name: "invalid".into(),
        description: "invalid".into(),
        input_schema: json!(3),
        risk: ToolRisk::Write,
    }), Err(ToolError::InvalidDefinition));
}

#[test]
fn event_log_orders_events_and_supports_cursor_replay() {
    let mut log = EventLog::new("task-1", "session-1").unwrap();
    let first = log.append("evt-1", 100, EventKind::TaskState {
        state: TaskState::Running,
    }).unwrap();
    let second = log.append("evt-2", 101, EventKind::ModelDelta {
        text: "Hello".into(),
    }).unwrap();
    assert_eq!(first.sequence, 1);
    assert_eq!(second.sequence, 2);
    assert_eq!(log.after(1).map(|e| e.event_id.as_str()).collect::<Vec<_>>(), vec!["evt-2"]);
    assert_eq!(log.after(2).count(), 0);
    assert_eq!(
        log.append("evt-2", 102, EventKind::Error { message: "again".into() }),
        Err(EventError::DuplicateId)
    );
}

#[test]
fn events_project_to_a_cloudevents_envelope() {
    let mut log = EventLog::new("t", "s").unwrap();
    let event = log.append("e", 1234, EventKind::ApprovalRequested {
        request_id: "call-1".into(),
        tool: "editor.read".into(),
        scope: "project-1".into(),
    }).unwrap();
    let wire = event.as_cloud_event();
    assert_eq!(wire["specversion"], "1.0");
    assert_eq!(wire["id"], "e");
    assert_eq!(wire["type"], "dev.browseros.agent.approval_requested");
    assert_eq!(wire["data"]["sequence"], 1);
    assert_eq!(wire["data"]["event"]["kind"], "approval_requested");
}

#[test]
fn external_agent_protocols_remain_separate_from_browseros_contract() {
    let acp = serde_json::to_string(&AgentTransport::Acp).unwrap();
    let a2a = serde_json::to_string(&AgentTransport::A2a).unwrap();
    assert_eq!(acp, "\"acp\"");
    assert_eq!(a2a, "\"a2a\"");
}
