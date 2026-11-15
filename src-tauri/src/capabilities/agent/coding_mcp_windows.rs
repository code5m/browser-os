//! Short-lived MCP stdio sessions inside a no-network Windows AppContainer.
#![cfg(windows)]

use super::windows_sandbox::{SandboxChild, SandboxProcess, SandboxProfile};
use serde_json::{Value, json};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
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
            for item in items.values_mut() {
                scrub(item, secrets);
            }
        }
        _ => {}
    }
}
fn read_config(path: &Path) -> Result<String, String> {
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
        let stem = executable
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if [
            "sh",
            "bash",
            "dash",
            "zsh",
            "fish",
            "cmd",
            "powershell",
            "pwsh",
            "npx",
        ]
        .contains(&stem.as_str())
        {
            return Err("MCP_SHELL_COMMAND_REJECTED".into());
        }
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
            if path.is_absolute() && path.is_file() {
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
        let SandboxChild {
            stdin,
            stdout,
            process,
        } = profile
            .spawn(&executable, &args, &environment, &state)
            .map_err(|_| "MCP_START_FAILED")?;
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
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                match reader
                    .by_ref()
                    .take(LIMIT + 1)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) => break,
                    Ok(_) if bytes.len() as u64 <= LIMIT => {
                        let value =
                            serde_json::from_slice(&bytes).map_err(|_| "MCP_INVALID_FRAME".into());
                        if reader_tx.try_send(value).is_err() {
                            break;
                        }
                    }
                    _ => {
                        let _ = reader_tx.try_send(Err("MCP_FRAME_LIMIT".into()));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            process,
            writer,
            write_result,
            reader,
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
        self.writer.send(bytes).map_err(|_| "MCP_WRITE_FAILED")?;
        match self.write_result.recv_timeout(remaining) {
            Ok(result) => result,
            Err(_) => Err("MCP_WRITE_TIMEOUT".into()),
        }
    }
    fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            let value = self
                .reader
                .recv_timeout(remaining)
                .map_err(|_| "MCP_TIMEOUT_OR_CLOSED")??;
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
    session.rpc(1, "initialize", json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"BrowserOS","version":"0.1.0"}}))?;
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
    fn shell_commands_are_rejected_before_profile_creation() {
        let root = root();
        let error = execute(&root, &root, json!({"action":"mcp_tools","approved":true,"server":{"command":"C:\\Windows\\System32\\cmd.exe","args":[]}})).unwrap_err();
        assert_eq!(error, "MCP_START_FAILED");
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
        let start = Instant::now();
        let error = execute(&root, &root, json!({"action":"mcp_call","approved":true,"server":server,"tool":"echo","arguments":{"text":"a".repeat(100_000)},"timeout_seconds":1})).unwrap_err();
        assert_eq!(error, "MCP_WRITE_TIMEOUT");
        assert!(start.elapsed() < Duration::from_secs(4));
    }
}
