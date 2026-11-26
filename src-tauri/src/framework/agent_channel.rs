//! BrowserOS main-window Agent Channel experiment host.
//! Reuses existing capability public commands and their invocation/policy checks.
//! No remote WebView, model message or MCP peer can reach this control plane.
use crate::capabilities::agent::public_api::{
    bounded_channel_json, ChannelAdmission, ChannelInput, ChannelStart, ProductionExecution,
};
use crate::shared::blocking::BlockingLane;
use crate::capabilities::workspace::safe_paths;
#[cfg(test)]
use std::fs;
use browseros_agent_foundation as foundation;
use foundation::{
    approval::{ApprovalBinding, HostApprovalDecision, HostApprovalIssuer, ResourceBinding},
    benchmark::{
        run_benchmark_suite, run_routed_fixture_owned, BenchmarkReport, RoutedFixtureResult,
    },
    experiment::{ExperimentConfig, ExperimentRouter},
    public_api::{channel_read_revision, current_public_api_catalog, PublicApiHost, PublicApiRegistry, PublicApiSpec},
    tool::{ToolEffect, ToolFuture, ToolRegistry},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Webview};

static CONFIG_LOCK: Mutex<()> = Mutex::new(());
const CONFIG_FILE: &str = "agent-runtime-experiment.json";
const MAX_CONFIG_BYTES: u64 = 16_384;
const MAX_RESULT_BYTES: usize = 64 * 1024;
static READ_LANE: BlockingLane = BlockingLane::new(
    1,
    "AGENT_CHANNEL_READ_BUSY",
    "AGENT_CHANNEL_READ_WORKER_FAILED",
);

fn check_main(app: &AppHandle, webview: &Webview, scope: &str) -> Result<(), String> {
    if webview.label() != "main" {
        return Err("AGENT_EXPERIMENT_SOURCE_DENIED".into());
    }
    crate::shared::invocation::check_invocation_source(webview, scope, None, app)
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::workspace::data_dir(app).join(CONFIG_FILE))
}

fn config_snapshot(path: &Path) -> Result<(ExperimentConfig, Option<safe_paths::FileSnapshot>), String> {
    let root = path.parent().ok_or("AGENT_EXPERIMENT_PATH")?.to_path_buf();
    let snapshot = safe_paths::read_versioned(path, &[root], MAX_CONFIG_BYTES as usize)
        .map_err(|_| "AGENT_EXPERIMENT_UNSAFE_CONFIG")?;
    let Some(saved) = snapshot.as_ref() else {
        return Ok((ExperimentConfig::default(), None));
    };
    let config: ExperimentConfig = serde_json::from_slice(&saved.bytes)
        .map_err(|_| "AGENT_EXPERIMENT_CORRUPT_CONFIG")?;
    config.validate().map_err(|_| "AGENT_EXPERIMENT_INVALID_CONFIG")?;
    Ok((config, snapshot))
}

fn load(path: &Path) -> Result<ExperimentConfig, String> {
    config_snapshot(path).map(|(config, _)| config)
}

fn persist(path: &Path, config: &ExperimentConfig, expected: Option<&safe_paths::FileSnapshot>,
    root_identity: &str) -> Result<(), String> {
    let root = path.parent().ok_or("AGENT_EXPERIMENT_PATH")?.to_path_buf();
    let bytes = serde_json::to_vec_pretty(config).map_err(|_| "AGENT_EXPERIMENT_SERIALIZE")?;
    if bytes.len() > MAX_CONFIG_BYTES as usize {
        return Err("AGENT_EXPERIMENT_CONFIG_LIMIT".into());
    }
    // Recheck the exact approved directory and original file snapshot
    // through the shared native CAS owner, not an unchecked path rename.
    safe_paths::write_versioned_approved(path, &[root.clone()], &root, root_identity,
        expected, &bytes, MAX_CONFIG_BYTES as usize, |_, _| Ok(()))
        .map_err(|error| format!("AGENT_EXPERIMENT_PUBLICATION_UNCONFIRMED:{error}"))?;
    let (actual, snapshot) = config_snapshot(path)?;
    if actual != *config || snapshot.is_none() {
        return Err("AGENT_EXPERIMENT_PUBLICATION_UNCONFIRMED".into());
    }
    Ok(())
}

#[tauri::command]
pub fn agent_runtime_get_config(
    app: AppHandle,
    webview: Webview,
) -> Result<ExperimentConfig, String> {
    check_main(&app, &webview, "agent_runtime_get_config")?;
    let _lock = CONFIG_LOCK.try_lock().map_err(|_| "AGENT_EXPERIMENT_CONFIG_BUSY")?;
    load(&config_path(&app)?)
}

#[tauri::command]
pub fn agent_runtime_set_config(
    app: AppHandle,
    webview: Webview,
    config: ExperimentConfig,
    expected_revision: u64,
    confirmed: bool,
) -> Result<ExperimentConfig, String> {
    check_main(&app, &webview, "agent_runtime_set_config")?;
    if !confirmed {
        return Err("AGENT_EXPERIMENT_CONFIRM_REQUIRED".into());
    }
    let _lock = CONFIG_LOCK.try_lock().map_err(|_| "AGENT_EXPERIMENT_CONFIG_BUSY")?;
    let path = config_path(&app)?;
    let parent = path.parent().ok_or("AGENT_EXPERIMENT_PATH")?;
    let root_identity = safe_paths::directory_identity(parent)
        .map_err(|_| "AGENT_EXPERIMENT_UNSAFE_DIR")?;
    let (previous, expected) = config_snapshot(&path)?;
    if previous.revision != expected_revision {
        return Err("AGENT_EXPERIMENT_STALE_REVISION".into());
    }
    // Same config schema and CAS validation as independent ExperimentRouter.
    let router = ExperimentRouter::new(true);
    router
        .restore_config_for_host(previous)
        .map_err(|_| "AGENT_EXPERIMENT_INVALID_CONFIG")?;
    router
        .update(config.clone(), expected_revision)
        .map_err(|_| "AGENT_EXPERIMENT_INVALID_CONFIG")?;
    persist(&path, &config, expected.as_ref(), &root_identity)?;
    Ok(config)
}

#[tauri::command]
pub async fn agent_runtime_benchmark(
    app: AppHandle,
    webview: Webview,
    repeats: u8,
) -> Result<BenchmarkReport, String> {
    check_main(&app, &webview, "agent_runtime_benchmark")?;
    let config = {
        let _guard = CONFIG_LOCK.try_lock().map_err(|_| "AGENT_EXPERIMENT_CONFIG_BUSY")?;
        load(&config_path(&app)?)?
    };
    if !config.enabled || !config.benchmark_enabled {
        return Err("AGENT_EXPERIMENT_BENCHMARK_DISABLED".into());
    }
    // This benchmark is deterministic offline orchestration ONLY. Never runs
    // native read/write tools or calls real provider credentials.
    run_benchmark_suite(repeats)
        .await
        .map_err(|_| "AGENT_EXPERIMENT_BENCHMARK_FAILED".into())
}

/// One newly allocated task driven by the CURRENT persisted routing policy.
/// This is the only dynamic runtime dispatch path in the experiment host.
/// The fixture never has an external side effect or live model access.
#[tauri::command]
pub async fn agent_runtime_run_fixture(
    app: AppHandle,
    webview: Webview,
    fixture_id: String,
) -> Result<RoutedFixtureResult, String> {
    check_main(&app, &webview, "agent_runtime_run_fixture")?;
    let config = enabled_config(&app)?;
    let storage = crate::workspace::data_dir(&app);
    let admission = tauri::async_runtime::spawn_blocking(move || {
        ProductionExecution::begin_channel(
            &storage,
            uuid::Uuid::new_v4(),
            ChannelStart::Fixture { fixture_id, config },
        )
    })
    .await
    .map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
    serde_json::from_value(run_admitted(app, webview, admission).await?)
        .map_err(|_| "AGENT_RUN_RECEIPT_INVALID".into())
}

fn as_text<'a>(value: &'a Value, name: &str, max: usize) -> Result<&'a str, String> {
    let text = value
        .get(name)
        .and_then(Value::as_str)
        .ok_or("AGENT_EXPERIMENT_ARGUMENT_REQUIRED")?;
    if text.is_empty() || text.len() > max || text.chars().any(char::is_control) {
        return Err("AGENT_EXPERIMENT_ARGUMENT_INVALID".into());
    }
    Ok(text)
}

/// The same typed request determines approval scope and native dispatch.
/// No unknown field or malformed optional value is silently discarded.
enum NativeRead {
    BrowserTabs,
    DatabaseProfiles,
    GitStatus { repo_id: String },
    GitDiff { repo_id: String, path: Option<String> },
    CodingLsp { root: String },
}

impl NativeRead {
    fn parse(command: &str, args: &Value) -> Result<Self, String> {
        let fields = args.as_object().ok_or("AGENT_EXPERIMENT_ARGUMENT_INVALID")?;
        let exact_keys = |allowed: &[&str]| {
            fields.keys().all(|key| allowed.contains(&key.as_str()))
        };
        match command {
            "tab_list" if fields.is_empty() => Ok(Self::BrowserTabs),
            "tab_list" => Err("AGENT_BROWSER_READ_ARGUMENT_INVALID".into()),
            "db_list_connections" if fields.is_empty() => Ok(Self::DatabaseProfiles),
            "git_status" if exact_keys(&["repo_id"]) => Ok(Self::GitStatus {
                repo_id: as_text(args, "repo_id", 100)?.to_owned(),
            }),
            "git_diff" if exact_keys(&["repo_id", "path"]) => {
                let path = match fields.get("path") {
                    None | Some(Value::Null) => None,
                    Some(_) => Some(as_text(args, "path", 1024)?.to_owned()),
                };
                Ok(Self::GitDiff { repo_id: as_text(args, "repo_id", 100)?.to_owned(), path })
            }
            "coding_lsp_status" if exact_keys(&["root"]) => Ok(Self::CodingLsp {
                root: as_text(args, "root", 4096)?.to_owned(),
            }),
            "db_list_connections" | "git_status" | "git_diff" | "coding_lsp_status" =>
                Err("AGENT_EXPERIMENT_ARGUMENT_INVALID".into()),
            _ => Err("AGENT_EXPERIMENT_TOOL_DENIED".into()),
        }
    }

    fn resource(&self) -> String {
        match self {
            Self::BrowserTabs => "browser:tabs".into(),
            Self::DatabaseProfiles => "database:profiles".into(),
            Self::GitStatus { repo_id } | Self::GitDiff { repo_id, .. } => format!("git:{repo_id}"),
            Self::CodingLsp { root } => format!("coding:{root}"),
        }
    }

    fn dispatch(self, app: AppHandle, webview: Webview) -> Result<Value, String> {
        match self {
            Self::BrowserTabs => crate::capabilities::browser::public_api::agent_tab_summary(app),
            Self::DatabaseProfiles =>
                crate::capabilities::database::public_api::agent_connection_summary(app, webview),
            Self::GitStatus { repo_id } =>
                crate::capabilities::git::public_api::agent_status(app, webview, repo_id),
            Self::GitDiff { repo_id, path } =>
                crate::capabilities::git::public_api::agent_diff(app, webview, repo_id, path),
            Self::CodingLsp { root } =>
                crate::capabilities::agent::public_api::coding_lsp_summary(app, webview, &root),
        }
    }
}

fn resource_for(command: &str, args: &Value) -> Result<String, String> {
    Ok(NativeRead::parse(command, args)?.resource())
}

// Both the initial host validation and worker admission enforce the same
// approval binding. A valid proposal is not an evergreen read capability.
fn validate_read_binding(spec: &PublicApiSpec, binding: &ApprovalBinding, args: &Value)
    -> Result<NativeRead, String>
{
    let read = NativeRead::parse(&spec.command, args)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "AGENT_APPROVAL_CLOCK_INVALID")?.as_millis();
    if !current_public_api_catalog().contains(spec)
        || spec.effect != ToolEffect::ReadOnly
        || !spec.requires_host_authorization
        || binding.principal_id.is_nil()
        || binding.workspace_id.is_nil()
        || binding.session_id.is_nil()
        || channel_read_revision(&spec.command, args).ok().as_deref()
            != Some(binding.resource.resource_revision.as_str())
        || now >= u128::from(binding.expires_at_unix_ms)
        || binding.resource.resource_id != read.resource()
    {
        return Err("AGENT_READ_APPROVAL_STALE_OR_DENIED".into());
    }
    Ok(read)
}

// The worker owns its capacity after the awaiting future is dropped.
 // A queued read must not begin after that cancellation or timeout.
struct ReadWaiter(Arc<AtomicBool>);
impl Drop for ReadWaiter {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

struct NativeOwnerHost {
    app: AppHandle,
    webview: Webview,
}
impl PublicApiHost for NativeOwnerHost {
    fn validate<'a>(
        &'a self,
        spec: &'a PublicApiSpec,
        binding: &'a ApprovalBinding,
        args: &'a Value,
    ) -> foundation::Result<()> {
        validate_read_binding(spec, binding, args)
            .map_err(|_| foundation::FoundationError::PermissionDenied)?;
        check_main(&self.app, &self.webview, &spec.command)
            .map_err(|_| foundation::FoundationError::PermissionDenied)?;
        Ok(())
    }

    fn call<'a>(
        &'a self,
        spec: &'a PublicApiSpec,
        proof: ApprovalBinding,
        args: Value,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            self.validate(spec, &proof, &args)?;
            let app = self.app.clone();
            let webview = self.webview.clone();
            let approved_spec = spec.clone();
            let waiter = ReadWaiter(Arc::new(AtomicBool::new(false)));
            let abandoned = Arc::clone(&waiter.0);
            let deadline = Instant::now() + Duration::from_secs(15);
            let result = READ_LANE
                .run(move || -> Result<Value, String> {
                    // Do not substitute a newer proof or tool after queueing.
                    // Recheck the exact approved catalog entry, resource,
                    // argument shape, deadline, source and enablement.
                    let check_waiter = || {
                        if abandoned.load(Ordering::Acquire) || Instant::now() >= deadline {
                            Err("AGENT_READ_WAITER_EXPIRED".to_string())
                        } else { Ok(()) }
                    };
                    check_waiter()?;
                    enabled_config(&app)?;
                    let read = validate_read_binding(&approved_spec, &proof, &args)?;
                    check_main(&app, &webview, &approved_spec.command)?;
                    check_waiter()?;
                    let result = read.dispatch(app, webview)?;
                    check_waiter()?;
                    // Check while the worker owns its permit; do not copy a
                    // potentially huge native result onto the async runtime.
                    bounded_channel_json(&result, MAX_RESULT_BYTES)?;
                    check_waiter()?;
                    Ok(result)
                })
                .await
                .map_err(|error| {
                    foundation::FoundationError::Tool(if error == "AGENT_CHANNEL_JSON_LIMIT" {
                        "OWNER_RESULT_LIMIT".into()
                    } else { "OWNER_READ_FAILED_OR_BUSY".into() })
                })?;
            // Keep the cancellation guard alive across the entire worker wait.
            drop(waiter);
            Ok(result)
        })
    }
}

#[tauri::command]
pub async fn agent_runtime_read_tool(
    app: AppHandle,
    webview: Webview,
    command: String,
    args: Value,
    confirmed: bool,
) -> Result<Value, String> {
    check_main(&app, &webview, "agent_runtime_read_tool")?;
    if !confirmed {
        return Err("AGENT_EXPERIMENT_APPROVAL_REQUIRED".into());
    }
    enabled_config(&app)?;
    let start = checked_read(command, args)?;
    let storage = crate::workspace::data_dir(&app);
    let admission = tauri::async_runtime::spawn_blocking(move || {
        ProductionExecution::begin_channel(&storage, uuid::Uuid::new_v4(), start)
    })
    .await
    .map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
    run_admitted(app, webview, admission).await
}
fn enabled_config(app: &AppHandle) -> Result<ExperimentConfig, String> {
    let _guard = CONFIG_LOCK.try_lock().map_err(|_| "AGENT_EXPERIMENT_CONFIG_BUSY")?;
    let cfg = load(&config_path(app)?)?;
    if !cfg.enabled {
        return Err("AGENT_EXPERIMENT_DISABLED".into());
    }
    Ok(cfg)
}
fn checked_read(command: String, args: Value) -> Result<ChannelStart, String> {
    if !args.is_object()
        || bounded_channel_json(&args, 4096).is_err()
    {
        return Err("AGENT_EXPERIMENT_ARGUMENT_INVALID".into());
    }
    current_public_api_catalog()
        .into_iter()
        .find(|entry| entry.command == command && entry.effect == ToolEffect::ReadOnly)
        .ok_or("AGENT_EXPERIMENT_TOOL_DENIED")?;
    let resource = resource_for(&command, &args)?;
    Ok(ChannelStart::Read {
        command,
        args,
        resource,
    })
}
async fn run_admitted(
    app: AppHandle,
    webview: Webview,
    admission: ChannelAdmission,
) -> Result<Value, String> {
    let (execution, input) = match admission {
        ChannelAdmission::Recovered(value) => return Ok(value),
        ChannelAdmission::Execute(execution, input) => (execution, input),
    };
    // The durable receipt belongs to this task, not the IPC waiter.
    tauri::async_runtime::spawn(async move {
        match input {
            ChannelInput::Fixture {
                fixture_id,
                assignment,
            } => {
                if assignment.task_id != execution.id() {
                    return publish_channel(execution, Err("AGENT_RUN_SCOPE_MISMATCH".into())).await;
                }
                let result = run_routed_fixture_owned(
                    assignment.kind,
                    execution.id(),
                    &fixture_id,
                    assignment.max_turns,
                    execution.tasks(),
                )
                .await
                .map_err(|_| "AGENT_EXPERIMENT_FIXTURE_FAILED".to_string())
                .and_then(|value| {
                    serde_json::to_value(value).map_err(|_| "AGENT_RUN_RECEIPT_INVALID".into())
                });
                publish_channel(execution, result).await
            }
            ChannelInput::Read {
                command,
                args,
                resource,
            } => execute_read(app, webview, execution, command, args, resource).await,
        }
    })
    .await
    .map_err(|_| "AGENT_RUN_WORKER_FAILED".to_string())?
}
async fn publish_channel(
    execution: ProductionExecution,
    result: Result<Value, String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execution.finish_channel(&result)?;
        result
    })
    .await
    .map_err(|_| "AGENT_RUN_WORKER_FAILED".to_string())?
}
async fn execute_read(
    app: AppHandle,
    webview: Webview,
    mut execution: ProductionExecution,
    command: String,
    args: Value,
    resource: String,
) -> Result<Value, String> {
    let tasks = execution.tasks();
    let task_id = execution.id();
    // All failures here precede tool dispatch. Publish them through the
    // existing durable execution owner instead of dropping a running receipt
    // (which would leave an interrupted run that appears resumable).
    let setup = (|| -> Result<_, String> {
        enabled_config(&app)?;
        if resource_for(&command, &args)? != resource {
            return Err("AGENT_RUN_SCOPE_MISMATCH".into());
        }
        let spec = current_public_api_catalog()
            .into_iter()
            .find(|entry| entry.command == command && entry.effect == ToolEffect::ReadOnly)
            .ok_or("AGENT_EXPERIMENT_TOOL_DENIED")?;
        let registry = PublicApiRegistry::default();
        let host = Arc::new(NativeOwnerHost { app, webview });
        registry
            .register(spec.owner, host)
            .map_err(|_| "AGENT_EXPERIMENT_OWNER_DENIED")?;
        let tool = registry
            .read_tool(spec)
            .map_err(|_| "AGENT_EXPERIMENT_OWNER_DENIED")?;
        let tools = ToolRegistry::default();
        tools
            .register(Arc::new(tool))
            .map_err(|_| "AGENT_EXPERIMENT_REGISTRY_FAILED")?;
        let resource_revision = channel_read_revision(&command, &args)
            .map_err(|_| "AGENT_EXPERIMENT_APPROVAL_FAILED")?;
        let call = tools
            .propose(tasks, task_id, &command, args)
            .map_err(|_| "AGENT_EXPERIMENT_TOOL_DENIED")?;
        let mut secret = [0_u8; 32];
        use ring::rand::{SecureRandom, SystemRandom};
        SystemRandom::new()
            .fill(&mut secret)
            .map_err(|_| "AGENT_EXPERIMENT_APPROVAL_FAILED")?;
        let (ticket, permit) = HostApprovalIssuer::new(secret)
            .map_err(|_| "AGENT_EXPERIMENT_APPROVAL_FAILED")?
            .resolve_local_decision(
                tasks,
                task_id,
                call,
                HostApprovalDecision {
                    principal_id: uuid::Uuid::new_v4(),
                    resource: ResourceBinding {
                        resource_id: resource,
                        resource_revision,
                    },
                    accepted: true,
                    ttl: Duration::from_secs(20),
                },
            )
            .map_err(|_| "AGENT_EXPERIMENT_APPROVAL_FAILED")?;
        let permit = permit.ok_or("AGENT_EXPERIMENT_APPROVAL_DENIED")?;
        Ok((tools, ticket, permit))
    })();
    let (tools, ticket, permit) = match setup {
        Ok(approved) => approved,
        Err(error) => return publish_channel(execution, Err(error)).await,
    };
    let execution = tauri::async_runtime::spawn_blocking(move || {
        execution.checkpoint_approval(ticket.claims)?;
        Ok::<_, String>(execution)
    })
    .await
    .map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
    let result = tools
        .execute(permit, tasks, Duration::from_secs(15))
        .await
        .map_err(|_| "AGENT_EXPERIMENT_TOOL_FAILED".to_string());
    publish_channel(execution, result).await
}

/// Existing main-only AI request fields, no new Tauri command.
pub(crate) async fn dispatch_recovery(
    app: AppHandle,
    webview: Webview,
    request: super::ai::AiRequest,
) -> Result<Value, String> {
    check_main(&app, &webview, "ai_service")?;
    let parse = |value: &str| -> Result<uuid::Uuid, String> {
        let id = uuid::Uuid::parse_str(value).map_err(|_| "AGENT_RUN_ID_INVALID")?;
        if id.get_version_num() != 4 || id.to_string() != value {
            return Err("AGENT_RUN_ID_INVALID".into());
        }
        Ok(id)
    };
    let storage = crate::workspace::data_dir(&app);
    match request.action.as_str() {
        "channel_history" => {
            let cursor = if request.id.is_empty() {
                None
            } else {
                Some(parse(&request.id)?)
            };
            return tauri::async_runtime::spawn_blocking(move || {
                ProductionExecution::channel_catalog(&storage, cursor)
            })
            .await
            .map_err(|_| "AGENT_RUN_WORKER_FAILED")?;
        }
        "channel_restore" => {
            let id = parse(&request.id)?;
            return tauri::async_runtime::spawn_blocking(move || {
                ProductionExecution::read_channel(&storage, id)
            })
            .await
            .map_err(|_| "AGENT_RUN_WORKER_FAILED")?;
        }
        "channel_forget" => {
            if !request.confirmed {
                return Err("AGENT_RUN_FORGET_APPROVAL_REQUIRED".into());
            }
            let id = parse(&request.id)?;
            return tauri::async_runtime::spawn_blocking(move || {
                let archived = ProductionExecution::forget_channel(&storage, id)?;
                Ok(serde_json::json!({"forgotten":true,"archived":archived}))
            })
            .await
            .map_err(|_| "AGENT_RUN_WORKER_FAILED")?;
        }
        _ => {}
    }
    if !request.confirmed {
        return Err("AGENT_EXPERIMENT_APPROVAL_REQUIRED".into());
    }
    let config = enabled_config(&app)?;
    let run = parse(&request.request_id)?;
    let admission = match request.action.as_str() {
        "channel_fixture" => {
            tauri::async_runtime::spawn_blocking(move || {
                ProductionExecution::begin_channel(
                    &storage,
                    run,
                    ChannelStart::Fixture {
                        fixture_id: request.id,
                        config,
                    },
                )
            })
            .await
        }
        "channel_read" => {
            if request.message.len() > 4096 {
                return Err("AGENT_EXPERIMENT_ARGUMENT_INVALID".into());
            }
            let args = serde_json::from_str(&request.message)
                .map_err(|_| "AGENT_EXPERIMENT_ARGUMENT_INVALID")?;
            let start = checked_read(request.id, args)?;
            tauri::async_runtime::spawn_blocking(move || {
                ProductionExecution::begin_channel(&storage, run, start)
            })
            .await
        }
        "channel_resume" => {
            let previous = parse(&request.id)?;
            tauri::async_runtime::spawn_blocking(move || {
                ProductionExecution::resume_channel(&storage, previous, run)
            })
            .await
        }
        _ => return Err("AGENT_EXPERIMENT_ACTION_DENIED".into()),
    }
    .map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
    run_admitted(app, webview, admission).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn read_catalog_refuses_effectful_owner_paths() {
        assert!(resource_for("db_query", &json!({"sql":"SELECT 1"})).is_err());
        assert!(resource_for("git_studio_confirm", &json!({})).is_err());
        assert_eq!(resource_for("tab_list", &json!({})).unwrap(), "browser:tabs");
        assert!(resource_for("tab_list", &json!({"js":"document.title"})).is_err());
        assert!(resource_for("tab_list", &json!({"id":"tab-1"})).is_err());
        assert!(resource_for("tab_list", &Value::Null).is_err());
        assert!(resource_for("eval_in_tab", &json!({})).is_err());
        assert!(resource_for("tab_open", &json!({})).is_err());
        assert_eq!(
            resource_for("git_status", &json!({"repo_id":"test"})).unwrap(),
            "git:test"
        );
        assert!(resource_for("git_status", &json!({"repo_id":"../\n"})).is_err());
        assert!(resource_for("db_list_connections", &json!({"sql":"SELECT 1"})).is_err());
        assert!(resource_for("git_status", &json!({"repo_id":"test","path":"other"})).is_err());
        assert!(resource_for("git_diff", &json!({"repo_id":"test","path":42})).is_err());
        assert!(resource_for("coding_lsp_status", &json!({"root":"/repo","command":"run"})).is_err());
        assert_eq!(resource_for("git_diff", &json!({"repo_id":"test","path":null})).unwrap(), "git:test");
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn disk_config_is_atomic_and_cas_ready() {
        let dir = std::env::temp_dir().join(format!(
            "browseros-agent-settings-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(CONFIG_FILE);
        assert_eq!(load(&path).unwrap(), ExperimentConfig::default());
        let cfg = ExperimentConfig {
            revision: 1,
            enabled: true,
            mode: foundation::experiment::RoutingMode::Canary,
            benchmark_enabled: true,
            canary_percent: 10,
            ..ExperimentConfig::default()
        };
        let identity = safe_paths::directory_identity(&dir).unwrap();
        persist(&path, &cfg, None, &identity).unwrap();
        assert_eq!(load(&path).unwrap(), cfg);
        assert!(persist(&path, &ExperimentConfig::default(), None, &identity).is_err());
        assert_eq!(load(&path).unwrap(), cfg);
        fs::remove_dir_all(&dir).unwrap();
    }
}
