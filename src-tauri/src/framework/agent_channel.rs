//! BrowserOS main-window Agent Channel experiment host.
//! Reuses existing capability public commands and their invocation/policy checks.
//! No remote WebView, model message or MCP peer can reach this control plane.
use browseros_agent_foundation as foundation;
use foundation::{
    approval::{
        ApprovalBinding, HostApprovalDecision, HostApprovalIssuer, ResourceBinding,
    },
    benchmark::{
        run_benchmark_suite, run_routed_fixture_owned, BenchmarkReport, RoutedFixtureResult,
    },
    experiment::{ExperimentConfig, ExperimentRouter},
    public_api::{current_public_api_catalog, PublicApiHost, PublicApiRegistry, PublicApiSpec},
    tool::{ToolEffect, ToolFuture, ToolRegistry},
};
use serde_json::Value;
use crate::capabilities::agent::public_api::{ChannelAdmission, ChannelInput, ChannelStart, ProductionExecution};
use crate::shared::blocking::BlockingLane;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Webview};

static CONFIG_LOCK: Mutex<()> = Mutex::new(());
const CONFIG_FILE: &str = "agent-runtime-experiment.json";
const MAX_CONFIG_BYTES: u64 = 16_384;
const MAX_RESULT_BYTES: usize = 64 * 1024;
static READ_LANE: BlockingLane = BlockingLane::new(1, "AGENT_CHANNEL_READ_BUSY", "AGENT_CHANNEL_READ_WORKER_FAILED");

fn check_main(app: &AppHandle, webview: &Webview, scope: &str) -> Result<(), String> {
    if webview.label() != "main" {
        return Err("AGENT_EXPERIMENT_SOURCE_DENIED".into());
    }
    crate::shared::invocation::check_invocation_source(webview, scope, None, app)
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::workspace::data_dir(app).join(CONFIG_FILE))
}

fn load(path: &Path) -> Result<ExperimentConfig, String> {
    if !path.exists() {
        return Ok(ExperimentConfig::default());
    }
    let meta = fs::symlink_metadata(path).map_err(|_| "AGENT_EXPERIMENT_READ_FAILED")?;
    if !meta.is_file() || meta.len() > MAX_CONFIG_BYTES {
        return Err("AGENT_EXPERIMENT_UNSAFE_CONFIG".into());
    }
    let bytes = fs::read(path).map_err(|_| "AGENT_EXPERIMENT_READ_FAILED")?;
    let config: ExperimentConfig =
        serde_json::from_slice(&bytes).map_err(|_| "AGENT_EXPERIMENT_CORRUPT_CONFIG")?;
    config
        .validate()
        .map_err(|_| "AGENT_EXPERIMENT_INVALID_CONFIG")?;
    Ok(config)
}

fn persist(path: &Path, config: &ExperimentConfig) -> Result<(), String> {
    let parent = path.parent().ok_or("AGENT_EXPERIMENT_PATH")?;
    fs::create_dir_all(parent).map_err(|_| "AGENT_EXPERIMENT_DIRECTORY")?;
    let parent_meta = fs::symlink_metadata(parent).map_err(|_| "AGENT_EXPERIMENT_DIRECTORY")?;
    if !parent_meta.is_dir() {
        return Err("AGENT_EXPERIMENT_UNSAFE_DIR".into());
    }
    if path.exists() {
        let meta = fs::symlink_metadata(path).map_err(|_| "AGENT_EXPERIMENT_READ_FAILED")?;
        if !meta.is_file() {
            return Err("AGENT_EXPERIMENT_UNSAFE_CONFIG".into());
        }
    }
    let bytes = serde_json::to_vec_pretty(config).map_err(|_| "AGENT_EXPERIMENT_SERIALIZE")?;
    if bytes.len() > MAX_CONFIG_BYTES as usize {
        return Err("AGENT_EXPERIMENT_CONFIG_LIMIT".into());
    }
    let temp = parent.join(format!(".agent-runtime-{}.tmp", uuid::Uuid::new_v4()));
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let saved = (|| {
        let mut file = opts
            .open(&temp)
            .map_err(|_| "AGENT_EXPERIMENT_WRITE_FAILED")?;
        file.write_all(&bytes)
            .map_err(|_| "AGENT_EXPERIMENT_WRITE_FAILED")?;
        file.sync_all()
            .map_err(|_| "AGENT_EXPERIMENT_FSYNC_FAILED")?;
        crate::session::atomic_replace_file(&temp, path)
            .map_err(|_| "AGENT_EXPERIMENT_RENAME_FAILED")?;
        // Windows directory handles cannot be opened by File::open without
        // FILE_FLAG_BACKUP_SEMANTICS. MoveFileExW uses WRITE_THROUGH;
        // explicit fsync of the directory is only required on Unix.
        #[cfg(unix)]
        fs::File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| "AGENT_EXPERIMENT_FSYNC_FAILED")?;
        Ok::<(), &str>(())
    })();
    if saved.is_err() {
        let _ = fs::remove_file(temp);
    }
    saved.map_err(str::to_string)
}

#[tauri::command]
pub fn agent_runtime_get_config(
    app: AppHandle,
    webview: Webview,
) -> Result<ExperimentConfig, String> {
    check_main(&app, &webview, "agent_runtime_get_config")?;
    let _lock = CONFIG_LOCK.lock().map_err(|_| "AGENT_EXPERIMENT_LOCK")?;
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
    let _lock = CONFIG_LOCK.lock().map_err(|_| "AGENT_EXPERIMENT_LOCK")?;
    let path = config_path(&app)?;
    let previous = load(&path)?;
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
    persist(&path, &config)?;
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
        let _guard = CONFIG_LOCK.lock().map_err(|_| "AGENT_EXPERIMENT_LOCK")?;
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
    let admission = tauri::async_runtime::spawn_blocking(move ||
        ProductionExecution::begin_channel(&storage, uuid::Uuid::new_v4(),
            ChannelStart::Fixture { fixture_id, config }))
        .await.map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
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

fn resource_for(command: &str, args: &Value) -> Result<String, String> {
    match command {
        "tab_list" => Ok("browser:tabs".into()),
        "db_list_connections" => Ok("database:profiles".into()),
        "coding_lsp_status" => Ok(format!("coding:{}", as_text(args, "root", 4096)?)),
        "git_status" | "git_diff" => Ok(format!("git:{}", as_text(args, "repo_id", 100)?)),
        _ => Err("AGENT_EXPERIMENT_TOOL_DENIED".into()),
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
        if self.webview.label() != "main"
            || spec.effect != ToolEffect::ReadOnly
            || !spec.requires_host_authorization
            || binding.principal_id.is_nil()
            || binding.workspace_id.is_nil()
            || binding.session_id.is_nil()
            || binding.resource.resource_revision != "local-main-ui-v1"
        {
            return Err(foundation::FoundationError::PermissionDenied);
        }
        let resource = resource_for(&spec.command, args)
            .map_err(|_| foundation::FoundationError::PermissionDenied)?;
        if binding.resource.resource_id != resource {
            return Err(foundation::FoundationError::PermissionDenied);
        }
        // Always rerun source policy. Real capability APIs do so a second time.
        crate::shared::invocation::check_invocation_source(
            &self.webview,
            &spec.command,
            None,
            &self.app,
        )
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
            let command = spec.command.clone();
            let result = READ_LANE.run(move || -> Result<Value, String> {
                match command.as_str() {
                    "tab_list" => Ok(crate::capabilities::browser::public_api::agent_tab_summary(
                        app,
                    )),
                    "db_list_connections" => {
                        crate::capabilities::database::public_api::agent_connection_summary(
                            app, webview,
                        )
                    }
                    "git_status" => {
                        let repo_id = as_text(&args, "repo_id", 100)?.to_owned();
                        crate::capabilities::git::public_api::agent_status(app, webview, repo_id)
                    }
                    "git_diff" => {
                        let repo_id = as_text(&args, "repo_id", 100)?.to_owned();
                        let path = args.get("path").and_then(Value::as_str).map(str::to_owned);
                        crate::capabilities::git::public_api::agent_diff(
                            app, webview, repo_id, path,
                        )
                    }
                    "coding_lsp_status" => {
                        let root = as_text(&args, "root", 4096)?.to_owned();
                        crate::capabilities::agent::public_api::coding_lsp_summary(
                            app, webview, &root,
                        )
                    }
                    _ => Err("AGENT_EXPERIMENT_TOOL_DENIED".into()),
                }
            })
            .await
            .map_err(|_| foundation::FoundationError::Tool("OWNER_READ_FAILED_OR_BUSY".into()))?;
            if serde_json::to_vec(&result)
                .map_err(|e| foundation::FoundationError::Serialization(e.to_string()))?
                .len()
                > MAX_RESULT_BYTES
            {
                return Err(foundation::FoundationError::Tool(
                    "OWNER_RESULT_LIMIT".into(),
                ));
            }
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
    if !confirmed { return Err("AGENT_EXPERIMENT_APPROVAL_REQUIRED".into()); }
    enabled_config(&app)?;
    let start = checked_read(command, args)?;
    let storage = crate::workspace::data_dir(&app);
    let admission = tauri::async_runtime::spawn_blocking(move ||
        ProductionExecution::begin_channel(&storage, uuid::Uuid::new_v4(), start))
        .await.map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
    run_admitted(app, webview, admission).await
}
fn enabled_config(app: &AppHandle) -> Result<ExperimentConfig, String> {
    let _guard = CONFIG_LOCK.lock().map_err(|_| "AGENT_EXPERIMENT_LOCK")?;
    let cfg = load(&config_path(app)?)?;
    if !cfg.enabled { return Err("AGENT_EXPERIMENT_DISABLED".into()); }
    Ok(cfg)
}
fn checked_read(command: String, args: Value) -> Result<ChannelStart, String> {
    if !args.is_object() || serde_json::to_vec(&args).map_err(|_| "AGENT_EXPERIMENT_ARGUMENT_INVALID")?.len() > 4096 {
        return Err("AGENT_EXPERIMENT_ARGUMENT_INVALID".into());
    }
    current_public_api_catalog().into_iter()
        .find(|entry| entry.command == command && entry.effect == ToolEffect::ReadOnly)
        .ok_or("AGENT_EXPERIMENT_TOOL_DENIED")?;
    let resource = resource_for(&command, &args)?;
    Ok(ChannelStart::Read { command, args, resource })
}
async fn run_admitted(app: AppHandle, webview: Webview, admission: ChannelAdmission) -> Result<Value, String> {
    let (execution, input) = match admission {
        ChannelAdmission::Recovered(value) => return Ok(value),
        ChannelAdmission::Execute(execution, input) => (execution, input),
    };
    // The durable receipt belongs to this task, not the IPC waiter.
    tauri::async_runtime::spawn(async move {
        match input {
            ChannelInput::Fixture { fixture_id, assignment } => {
                if assignment.task_id != execution.id() { return Err("AGENT_RUN_SCOPE_MISMATCH".into()); }
                let result = run_routed_fixture_owned(assignment.kind, execution.id(), &fixture_id,
                    assignment.max_turns, execution.tasks()).await
                    .map_err(|_| "AGENT_EXPERIMENT_FIXTURE_FAILED".to_string())
                    .and_then(|value| serde_json::to_value(value).map_err(|_| "AGENT_RUN_RECEIPT_INVALID".into()));
                publish_channel(execution, result).await
            }
            ChannelInput::Read { command, args, resource } =>
                execute_read(app, webview, execution, command, args, resource).await,
        }
    }).await.map_err(|_| "AGENT_RUN_WORKER_FAILED".to_string())?
}
async fn publish_channel(execution: ProductionExecution, result: Result<Value, String>) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execution.finish_channel(&result)?;
        result
    }).await.map_err(|_| "AGENT_RUN_WORKER_FAILED".to_string())?
}
async fn execute_read(app: AppHandle, webview: Webview, mut execution: ProductionExecution,
    command: String, args: Value, resource: String) -> Result<Value, String> {
    if resource_for(&command, &args)? != resource {
        return publish_channel(execution, Err("AGENT_RUN_SCOPE_MISMATCH".into())).await;
    }
    let spec = current_public_api_catalog().into_iter()
        .find(|entry| entry.command == command && entry.effect == ToolEffect::ReadOnly)
        .ok_or("AGENT_EXPERIMENT_TOOL_DENIED")?;
    let registry = PublicApiRegistry::default();
    let host = Arc::new(NativeOwnerHost { app, webview });
    registry.register(spec.owner, host).map_err(|_| "AGENT_EXPERIMENT_OWNER_DENIED")?;
    let tool = registry.read_tool(spec).map_err(|_| "AGENT_EXPERIMENT_OWNER_DENIED")?;
    let tools = ToolRegistry::default();
    tools.register(Arc::new(tool)).map_err(|_| "AGENT_EXPERIMENT_REGISTRY_FAILED")?;
    let tasks = execution.tasks();
    let task_id = execution.id();
    let call = tools.propose(tasks, task_id, &command, args).map_err(|_| "AGENT_EXPERIMENT_TOOL_DENIED")?;
    let mut secret = [0_u8; 32];
    use ring::rand::{SecureRandom, SystemRandom};
    SystemRandom::new().fill(&mut secret).map_err(|_| "AGENT_EXPERIMENT_APPROVAL_FAILED")?;
    let (ticket, permit) = HostApprovalIssuer::new(secret).map_err(|_| "AGENT_EXPERIMENT_APPROVAL_FAILED")?
        .resolve_local_decision(tasks, task_id, call,
            HostApprovalDecision { principal_id: uuid::Uuid::new_v4(),
                resource: ResourceBinding { resource_id: resource, resource_revision: "local-main-ui-v1".into() },
                accepted: true, ttl: Duration::from_secs(20) })
        .map_err(|_| "AGENT_EXPERIMENT_APPROVAL_FAILED")?;
    let permit = permit.ok_or("AGENT_EXPERIMENT_APPROVAL_DENIED")?;
    let execution = tauri::async_runtime::spawn_blocking(move || {
        execution.checkpoint_approval(ticket.claims)?;
        Ok::<_, String>(execution)
    }).await.map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
    let result = tools.execute(permit, tasks, Duration::from_secs(15)).await
        .map_err(|_| "AGENT_EXPERIMENT_TOOL_FAILED".to_string());
    publish_channel(execution, result).await
}

/// Existing main-only AI request fields, no new Tauri command.
pub(crate) async fn dispatch_recovery(app: AppHandle, webview: Webview, request: super::ai::AiRequest) -> Result<Value, String> {
    check_main(&app, &webview, "ai_service")?;
    let parse = |value: &str| -> Result<uuid::Uuid, String> {
        let id = uuid::Uuid::parse_str(value).map_err(|_| "AGENT_RUN_ID_INVALID")?;
        if id.get_version_num() != 4 || id.to_string() != value { return Err("AGENT_RUN_ID_INVALID".into()); }
        Ok(id)
    };
    let storage = crate::workspace::data_dir(&app);
    match request.action.as_str() {
        "channel_history" => {
            let cursor = if request.id.is_empty() { None } else { Some(parse(&request.id)?) };
            return tauri::async_runtime::spawn_blocking(move || ProductionExecution::channel_catalog(&storage, cursor))
                .await.map_err(|_| "AGENT_RUN_WORKER_FAILED")?;
        }
        "channel_restore" => {
            let id = parse(&request.id)?;
            return tauri::async_runtime::spawn_blocking(move || ProductionExecution::read_channel(&storage, id))
                .await.map_err(|_| "AGENT_RUN_WORKER_FAILED")?;
        }
        "channel_forget" => {
            if !request.confirmed { return Err("AGENT_RUN_FORGET_APPROVAL_REQUIRED".into()); }
            let id = parse(&request.id)?;
            return tauri::async_runtime::spawn_blocking(move || {
                let archived = ProductionExecution::forget_channel(&storage, id)?;
                Ok(serde_json::json!({"forgotten":true,"archived":archived}))
            }).await.map_err(|_| "AGENT_RUN_WORKER_FAILED")?;
        }
        _ => {}
    }
    if !request.confirmed { return Err("AGENT_EXPERIMENT_APPROVAL_REQUIRED".into()); }
    let config = enabled_config(&app)?;
    let run = parse(&request.request_id)?;
    let admission = match request.action.as_str() {
        "channel_fixture" => tauri::async_runtime::spawn_blocking(move ||
            ProductionExecution::begin_channel(&storage, run,
                ChannelStart::Fixture { fixture_id: request.id, config })).await,
        "channel_read" => {
            if request.message.len() > 4096 { return Err("AGENT_EXPERIMENT_ARGUMENT_INVALID".into()); }
            let args = serde_json::from_str(&request.message).map_err(|_| "AGENT_EXPERIMENT_ARGUMENT_INVALID")?;
            let start = checked_read(request.id, args)?;
            tauri::async_runtime::spawn_blocking(move || ProductionExecution::begin_channel(&storage, run, start)).await
        }
        "channel_resume" => {
            let previous = parse(&request.id)?;
            tauri::async_runtime::spawn_blocking(move || ProductionExecution::resume_channel(&storage, previous, run)).await
        }
        _ => return Err("AGENT_EXPERIMENT_ACTION_DENIED".into()),
    }.map_err(|_| "AGENT_RUN_WORKER_FAILED")??;
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
        assert_eq!(
            resource_for("git_status", &json!({"repo_id":"test"})).unwrap(),
            "git:test"
        );
        assert!(resource_for("git_status", &json!({"repo_id":"../\n"})).is_err());
    }
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
        persist(&path, &cfg).unwrap();
        assert_eq!(load(&path).unwrap(), cfg);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
    }
}
