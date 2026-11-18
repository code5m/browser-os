//! Short-lived MCP stdio sessions inside a no-network Windows AppContainer.
#![cfg(windows)]

use super::windows_sandbox::{send_bounded, SandboxChild, SandboxProcess, SandboxProfile};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const LIMIT: u64 = 128 * 1024;
const CONFIG_LIMIT: usize = 256 * 1024;
const PAYLOAD_LIMIT: usize = 8 * 1024 * 1024;

fn sensitive(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "password",
        "authorization:",
        "bearer ",
        "private key",
        "sk-",
        "access_token",
        "secret=",
    ]
    .iter()
    .any(|item| lower.contains(item))
}
fn safe_text(text: &str) -> String {
    text.lines()
        .map(|line| {
            if sensitive(line) {
                "[credential-bearing line omitted]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn scrub(value: &mut Value, secrets: &[String]) {
    match value {
        Value::String(text) => {
            *text = safe_text(text);
            for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
                *text = text.replace(secret, "[redacted]");
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub(item, secrets);
            }
        }
        Value::Object(items) => {
            // JSON keys are untrusted MCP output too. A server can embed
            // credentials in a property name, bypassing value-only masking.
            let previous = std::mem::take(items);
            for (index, (name, mut item)) in previous.into_iter().enumerate() {
                scrub(&mut item, secrets);
                let sensitive_name = sensitive(&name) || secrets.iter()
                    .any(|secret| !secret.is_empty() && name.contains(secret));
                let safe_name = if sensitive_name {
                    format!("[redacted-key-{index}]")
                } else {
                    name
                };
                items.insert(safe_name, item);
            }
        }
        _ => {}
    }
}
fn read_config(path: &Path) -> Result<String, String> {
    // Config sources come from a fixed set of workspace/owner paths. Never
    // follow a junction/symlink to a different file before authorizing it.
    super::windows_sandbox::no_reparse_path(path).map_err(|_| "MCP_CONFIG_REPARSE_DENIED")?;
    let canonical = path.canonicalize().map_err(|_| "MCP_CONFIG_NOT_FOUND")?;
    let parent = canonical.parent().ok_or("MCP_CONFIG_DENIED")?;
    let allowed = parent.canonicalize().map_err(|_| "MCP_CONFIG_DENIED")?;
    let bytes = crate::capabilities::workspace::safe_paths::read_regular(
        &canonical,
        &[allowed],
        CONFIG_LIMIT,
    )
    .map_err(|_| "MCP_CONFIG_READ_DENIED")?;
    String::from_utf8(bytes).map_err(|_| "MCP_CONFIG_NOT_TEXT".into())
}
fn sources(root: &Path, storage: &Path) -> Vec<PathBuf> {
    let mut sources = vec![
        storage.join("coding-mcp.json"),
        root.join("opencode.json"),
        root.join(".opencode/opencode.json"),
    ];
    if let Some(home) = std::env::var_os("USERPROFILE") {
        let home = PathBuf::from(home);
        sources.extend([
            home.join(".codebuddy/mcp.json"),
            home.join(".workbuddy/mcp.json"),
            home.join(".config/opencode/opencode.json"),
        ]);
    }
    sources
}
fn servers(root: &Path, storage: &Path) -> Vec<(String, String, PathBuf, Value)> {
    let mut output = Vec::new();
    for source in sources(root, storage) {
        let Ok(text) = read_config(&source) else {
            continue;
        };
        let Ok(config) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let servers = config
            .get("mcpServers")
            .or_else(|| config.get("mcp"))
            .or_else(|| config.get("servers"));
        if let Some(servers) = servers.and_then(Value::as_object) {
            for (name, item) in servers.iter().take(100) {
                let mut hash = DefaultHasher::new();
                source.hash(&mut hash);
                name.hash(&mut hash);
                output.push((
                    format!("mcp-{:016x}", hash.finish()),
                    name.clone(),
                    source.clone(),
                    item.clone(),
                ));
            }
        }
    }
    output
}
pub fn discover(root: &Path, storage: &Path) -> Value {
    json!({"servers":servers(root,storage).iter().map(|(id,name,source,item)|json!({
        "id":id,"name":safe_text(name),"source":source,
        "transport":if item.get("url").is_some(){"http"}else{"stdio"},
        "connected":false,"requiresApproval":true,"sandbox":"appcontainer-no-network"
    })).collect::<Vec<_>>()})
}
fn resolve_executable(command: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(command);
    if path.is_absolute() {
        return Ok(path);
    }
    if command.contains(['/', '\\']) {
        return Err("MCP_ABSOLUTE_EXECUTABLE_REQUIRED".into());
    }
    let paths = std::env::var_os("PATH").ok_or("MCP_EXECUTABLE_NOT_FOUND")?;
    let candidate = if command.to_ascii_lowercase().ends_with(".exe") {
        command.to_owned()
    } else {
        format!("{command}.exe")
    };
    std::env::split_paths(&paths)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(&candidate))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| "MCP_EXECUTABLE_NOT_FOUND".into())
}
fn resolved_config(root: &Path, storage: &Path, request: &Value) -> Result<Value, String> {
    if let Some(id) = request["server_id"].as_str() {
        let (_, _, source, mut item) = servers(root, storage)
            .into_iter()
            .find(|server| server.0 == id)
            .ok_or("MCP_SERVER_NOT_FOUND")?;
        if item.get("url").is_some() {
            return Err("MCP_HTTP_NOT_IMPLEMENTED".into());
        }
        if let Some(command) = item["command"].as_array().cloned() {
            item["command"] = command.first().cloned().unwrap_or(Value::Null);
            item["args"] = json!(command.into_iter().skip(1).collect::<Vec<_>>());
        }
        if item.get("environment").is_some() {
            item["env"] = item["environment"].clone();
        }
        if let Some(command) = item["command"].as_str() {
            item["command"] = json!(resolve_executable(command)?);
        }
        if source.starts_with(root) && item.get("env").is_some() {
            return Err("MCP_PROJECT_ENV_REQUIRES_KEYRING_REFS".into());
        }
        Ok(item)
    } else {
        let item = request["server"].clone();
        if item.get("env").is_some() || item.get("environment").is_some() {
            return Err("MCP_PLAINTEXT_ENV_REJECTED".into());
        }
        Ok(item)
    }
}

struct Session {
    process: SandboxProcess,
    writer: mpsc::SyncSender<Vec<u8>>,
    write_result: mpsc::Receiver<Result<(), String>>,
    reader: mpsc::Receiver<Result<Value, String>>,
    reader_fault: Arc<Mutex<Option<String>>>,
    deadline: Instant,
    secrets: Vec<String>,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.process.terminate();
    }
}
impl Session {
    fn start(root: &Path, config: &Value, seconds: u64) -> Result<Self, String> {
        let command = config["command"].as_str().ok_or("MCP_COMMAND_REQUIRED")?;
        let executable = resolve_executable(command)?;
        // Reject a known wrapper even if the path does not exist on a
        // particular Windows SKU; no profile creation or path probing needed.
        if crate::security_policy::is_blocked_executable_path(&executable) {
            return Err("MCP_SHELL_COMMAND_REJECTED".into());
        }
        let resolved = executable.canonicalize().map_err(|_| "MCP_EXECUTABLE_NOT_FOUND")?;
        if crate::security_policy::is_blocked_executable_path(&resolved) {
            return Err("MCP_SHELL_COMMAND_REJECTED".into());
        }
        let stem = resolved.file_stem().and_then(|name| name.to_str())
            .ok_or("MCP_EXECUTABLE_NOT_FOUND")?.to_ascii_lowercase();
        let raw_args = config["args"].as_array().cloned().unwrap_or_default();
        if raw_args.len() > 64
            || raw_args.iter().any(|value| {
                value
                    .as_str()
                    .is_none_or(|arg| arg.len() > 4096 || sensitive(arg))
            })
        {
            return Err("MCP_ARGS_INVALID".into());
        }
        let profile = SandboxProfile::create()?;
        let state = profile.state_dir()?;
        let payload = state.join("payload");
        fs::create_dir(&payload).map_err(|_| "MCP_PAYLOAD_CREATE")?;
        let mut args = Vec::<String>::new();
        for value in raw_args {
            let value = value.as_str().ok_or("MCP_ARGS_INVALID")?;
            let path = PathBuf::from(value);
            if path.is_absolute() {
                // Never pass through an absolute host path, even when the
                // file is absent during validation. Only pinned readback
                // copies under our per-profile state directory are allowed.
                if !path.is_file() {
                    return Err("MCP_PAYLOAD_NOT_FOUND".into());
                }
                let canonical = path.canonicalize().map_err(|_| "MCP_PAYLOAD_NOT_FOUND")?;
                if !canonical.starts_with(root) {
                    return Err("MCP_PAYLOAD_OUTSIDE_WORKSPACE".into());
                }
                let bytes = crate::capabilities::workspace::safe_paths::read_regular(
                    &canonical,
                    &[root.to_path_buf()],
                    PAYLOAD_LIMIT,
                )
                .map_err(|_| "MCP_PAYLOAD_READ_DENIED")?;
                let name = canonical.file_name().ok_or("MCP_PAYLOAD_INVALID")?;
                let destination =
                    payload.join(format!("{}-{}", args.len(), name.to_string_lossy()));
                fs::write(&destination, bytes).map_err(|_| "MCP_PAYLOAD_WRITE")?;
                args.push(
                    destination
                        .to_str()
                        .ok_or("MCP_PAYLOAD_NOT_UNICODE")?
                        .to_owned(),
                );
            } else {
                args.push(value.to_owned());
            }
        }
        // Node resolves the main module through realpathSync, which lstat()s
        // every ancestor starting at the drive root. An AppContainer token
        // cannot read C:\ itself, so an otherwise readable private payload
        // aborts with EPERM before the server executes. Payload scripts are
        // already pinned copies inside this profile, so skipping Node's
        // main-module realpath does not widen file access.
        if stem == "node" {
            args.insert(0, "--preserve-symlinks-main".into());
        }
        let mut environment = Vec::<(String, String)>::new();
        let mut secrets = Vec::<String>::new();
        if let Some(env) = config["env_refs"].as_object() {
            for (name, reference) in env {
                if name.len() > 100
                    || !name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                {
                    return Err("MCP_ENV_INVALID".into());
                }
                let reference = reference
                    .as_str()
                    .filter(|value| value.starts_with("ai:") && value.len() < 180)
                    .ok_or("MCP_CREDENTIAL_REF_INVALID")?;
                let value = crate::keyring_store::KeyringStore::get_token(reference)
                    .map_err(|_| "MCP_CREDENTIAL_MISSING")?;
                secrets.push(value.clone());
                environment.push((name.clone(), value));
            }
        }
        if let Some(env) = config["env"].as_object() {
            if env.len() > 64 {
                return Err("MCP_ENV_LIMIT".into());
            }
            for (name, value) in env {
                let value = value
                    .as_str()
                    .filter(|value| value.len() <= 16384)
                    .ok_or("MCP_ENV_INVALID")?;
                if name.is_empty()
                    || name.len() > 100
                    || !name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                {
                    return Err("MCP_ENV_INVALID".into());
                }
                secrets.push(value.to_owned());
                environment.push((name.clone(), value.to_owned()));
            }
        }
        let child = profile
            .spawn(&executable, &args, &environment, &state)
            .map_err(|error| {
                // Native test panics include the concrete Win32 operation and
                // code; production retains an opaque error with no file paths,
                // credential contents, or process-internal diagnostics.
                #[cfg(test)]
                {
                    return format!("MCP_START_FAILED:{error}");
                }
                #[cfg(not(test))]
                {
                    let _ = error;
                    "MCP_START_FAILED".to_string()
                }
            })?;
        let SandboxChild {
            stdin,
            stdout,
            process,
        } = child;
        let (writer, write_rx) = mpsc::sync_channel::<Vec<u8>>(4);
        let (write_tx, write_result) = mpsc::channel();
        thread::spawn(move || {
            let mut input = std::io::BufWriter::new(stdin);
            while let Ok(bytes) = write_rx.recv() {
                let result = input
                    .write_all(&bytes)
                    .and_then(|_| input.flush())
                    .map_err(|_| "MCP_WRITE_FAILED".to_string());
                let failed = result.is_err();
                if write_tx.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let (reader_tx, reader) = mpsc::sync_channel(32);
        let reader_fault = Arc::new(Mutex::new(None));
        let on_read_failure = Arc::clone(&reader_fault);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                let (value, fatal) = match reader
                    .by_ref()
                    .take(LIMIT + 1)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) => break,
                    Ok(_) if bytes.len() as u64 <= LIMIT => (
                        serde_json::from_slice(&bytes).map_err(|_| "MCP_INVALID_FRAME".into()),
                        false,
                    ),
                    Ok(_) => (Err("MCP_FRAME_LIMIT".into()), true),
                    Err(_) => (Err("MCP_FRAME_READ".into()), true),
                };
                if send_bounded(&reader_tx, value, Duration::from_secs(2)).is_err() {
                    *on_read_failure.lock().unwrap_or_else(|p| p.into_inner()) =
                        Some("MCP_OUTPUT_BACKPRESSURE".into());
                    break;
                }
                if fatal {
                    break;
                }
            }
        });
        Ok(Self {
            process,
            writer,
            write_result,
            reader,
            reader_fault,
            deadline: Instant::now() + Duration::from_secs(seconds.clamp(1, 60)),
            secrets,
        })
    }
    fn send(&mut self, value: Value) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(&value).map_err(|_| "MCP_ENCODE_FAILED")?;
        if bytes.len() as u64 > LIMIT {
            return Err("MCP_REQUEST_LIMIT".into());
        }
        bytes.push(b'\n');
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("MCP_WRITE_TIMEOUT".into());
        }
        // A bounded sync channel can itself block indefinitely on send().
        // Observe the same end-to-end deadline when the writer is busy.
        let mut queued = bytes;
        loop {
            match self.writer.try_send(queued) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    return Err("MCP_WRITE_FAILED".into());
                }
                Err(mpsc::TrySendError::Full(message)) => {
                    queued = message;
                    let remaining = self.deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Err("MCP_WRITE_TIMEOUT".into());
                    }
                    thread::sleep(remaining.min(Duration::from_millis(5)));
                }
            }
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("MCP_WRITE_TIMEOUT".into());
        }
        match self.write_result.recv_timeout(remaining) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err("MCP_WRITE_TIMEOUT".into()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("MCP_WRITE_FAILED".into()),
        }
    }
    fn recv_frame(&self) -> Result<Value, String> {
        let fault = || {
            self.reader_fault
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clone()
        };
        if let Some(error) = fault() {
            return Err(error);
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        match self.reader.recv_timeout(remaining) {
            Ok(value) => {
                if let Some(error) = fault() {
                    return Err(error);
                }
                value
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                Err(fault().unwrap_or_else(|| "MCP_TIMEOUT_OR_CLOSED".into()))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(fault().unwrap_or_else(|| "MCP_CLOSED".into()))
            }
        }
    }
    fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let value = self.recv_frame()?;
            if value["id"] == json!(id) {
                if value.get("error").is_some() {
                    return Err("MCP_REMOTE_ERROR".into());
                }
                return value
                    .get("result")
                    .cloned()
                    .ok_or("MCP_RESULT_MISSING".into());
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32601,"message":"Client requests unsupported"}}))?;
            }
        }
    }
}

pub fn execute(root: &Path, storage: &Path, request: Value) -> Result<Value, String> {
    if request["approved"] != true {
        return Err("MCP_EXPLICIT_APPROVAL_REQUIRED".into());
    }
    let action = request["action"].as_str().unwrap_or("");
    if !["mcp_tools", "mcp_call"].contains(&action) {
        return Err("MCP_ACTION_UNKNOWN".into());
    }
    let root = root.canonicalize().map_err(|_| "CONTEXT_ROOT_INVALID")?;
    let config = resolved_config(&root, storage, &request)?;
    let mut session = Session::start(
        &root,
        &config,
        request["timeout_seconds"].as_u64().unwrap_or(20),
    )?;
    let initialized = session.rpc(1, "initialize", json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"BrowserOS","version":"0.1.0"}}));
    #[cfg(test)]
    if let Err(ref error) = initialized {
        // Test-only diagnostics; never leak keyring or config-supplied secrets.
        let mut stderr = session.process.profile().stderr_for_test();
        for secret in session.secrets.iter().filter(|value| !value.is_empty()) {
            stderr = stderr.replace(secret, "[redacted]");
        }
        eprintln!(
            "MCP_INITIALIZE_DIAG={error}; exit={:?}; stderr={stderr:?}",
            session.process.exit_code()
        );
    }
    initialized?;
    session.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
    let mut result = if action == "mcp_tools" {
        session.rpc(2, "tools/list", json!({}))?
    } else {
        let tool = request["tool"]
            .as_str()
            .filter(|tool| tool.len() <= 200)
            .ok_or("MCP_TOOL_REQUIRED")?;
        session.rpc(
            2,
            "tools/call",
            json!({"name":tool,"arguments":request.get("arguments").cloned().unwrap_or(json!({}))}),
        )?
    };
    scrub(&mut result, &session.secrets);
    Ok(json!({"result":result,"untrusted":true,"sandbox":"appcontainer-no-network"}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .canonicalize()
            .unwrap()
    }
    fn config() -> Value {
        let node =
            resolve_executable("node").expect("Node.js is required for the Windows MCP fixture");
        json!({"command":node,"args":[root().join("tests/fixtures/coding-mcp/server.js")]})
    }

    #[test]
    fn real_appcontainer_stdio_initializes_lists_calls_and_scrubs_secrets() {
        let root = root();
        let listed = execute(
            &root,
            &root,
            json!({"action":"mcp_tools","approved":true,"server":config()}),
        )
        .unwrap();
        assert_eq!(listed["result"]["tools"][0]["name"], "echo");
        let called = execute(&root, &root, json!({"action":"mcp_call","approved":true,"server":config(),"tool":"echo","arguments":{"text":"fixture output"}})).unwrap();
        assert_eq!(called["result"]["content"][0]["text"], "fixture output");
        assert_eq!(called["sandbox"], "appcontainer-no-network");

        let storage =
            std::env::temp_dir().join(format!("browseros-mcp-win-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&storage).unwrap();
        let mut server = config();
        server["env"] = json!({"FIXTURE_TOKEN":"fixture-opaque-value"});
        fs::write(
            storage.join("coding-mcp.json"),
            json!({"servers":{"fixture":server}}).to_string(),
        )
        .unwrap();
        let discovered = discover(&root, &storage);
        assert!(!discovered.to_string().contains("fixture-opaque-value"));
        let id = discovered["servers"][0]["id"].clone();
        let secret = execute(
            &root,
            &storage,
            json!({"action":"mcp_call","approved":true,"server_id":id,"tool":"echo_env"}),
        )
        .unwrap();
        assert_eq!(secret["result"]["content"][0]["text"], "[redacted]");
        fs::remove_dir_all(storage).unwrap();
    }

    #[test]
    fn credential_in_json_key_is_redacted_in_nested_mcp_results() {
        let mut response = json!({
            "api_key": "visible?",
            "nested": {"fixture-opaque-value": "another secret", "text": "fixture-opaque-value"},
            "ordinary": "visible"
        });
        scrub(&mut response, &["fixture-opaque-value".to_string()]);
        let serialized = response.to_string();
        assert!(!serialized.contains("api_key"));
        assert!(!serialized.contains("fixture-opaque-value"));
        assert_eq!(response["ordinary"], "visible");
    }

    #[test]
    fn windows_lolbin_wrappers_are_rejected_without_spawning_sandbox() {
        let root = root();
        for program in ["rundll32.exe", "regsvr32.exe", "mshta.exe", "wscript.exe"] {
            let path = Path::new(r"C:\Windows\System32").join(program);
            let result = Session::start(&root, &json!({"command":path}), 1);
            assert!(matches!(result, Err(ref message) if message == "MCP_SHELL_COMMAND_REJECTED"),
                "{program} must not create an AppContainer process");
        }
    }

    #[test]
    fn shell_commands_are_rejected_before_profile_creation() {
        let root = root();
        let error = execute(&root, &root, json!({"action":"mcp_tools","approved":true,"server":{"command":"C:\\Windows\\System32\\cmd.exe","args":[]}})).unwrap_err();
        assert_eq!(error, "MCP_SHELL_COMMAND_REJECTED");
        // Direct invocation of Session exposes the specific contract without
        // turning the public error into a sandbox implementation disclosure.
        let error = match Session::start(
            &root,
            &json!({"command":"C:\\Windows\\System32\\cmd.exe"}),
            1,
        ) {
            Err(error) => error,
            Ok(_) => panic!("shell unexpectedly started"),
        };
        assert_eq!(error, "MCP_SHELL_COMMAND_REJECTED");
    }

    #[test]
    fn stdin_backpressure_obeys_deadline() {
        let root = root();
        let mut server = config();
        server["args"]
            .as_array_mut()
            .unwrap()
            .push(json!("blocked"));
        // The fixture deliberately never consumes stdin. Bypass the
        // initialize RPC here: waiting for its response tests read timeout,
        // not the large-frame stdin backpressure contract.
        let mut session = Session::start(&root, &server, 1)
            .expect("start blocked MCP fixture inside AppContainer");
        let start = Instant::now();
        let error = session
            .send(json!({
                "jsonrpc":"2.0",
                "id":2,
                "method":"tools/call",
                "params":{"name":"echo","arguments":{"text":"a".repeat(100_000)}}
            }))
            .unwrap_err();
        assert_eq!(error, "MCP_WRITE_TIMEOUT");
        assert!(start.elapsed() < Duration::from_secs(4));
    }
}
