//! Explicit context selection and short-lived MCP stdio sessions for the coding workbench.
use serde_json::{json, Value};
use std::{
    fs,
    hash::{Hash, Hasher},
    io::{BufRead, BufReader, Read, Write},
    os::fd::AsRawFd,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const LIMIT: u64 = 128 * 1024;
fn read(path: &Path) -> Result<String, String> {
    let meta = fs::symlink_metadata(path).map_err(|_| "CONTEXT_READ_FAILED")?;
    if !meta.is_file() || meta.len() > LIMIT {
        return Err("CONTEXT_FILE_LIMIT".into());
    }
    fs::read_to_string(path).map_err(|_| "CONTEXT_NOT_TEXT".into())
}
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
    .any(|s| lower.contains(s))
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
            for secret in secrets.iter().filter(|s| !s.is_empty()) {
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
fn roots(root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![root.join(".agents/skills"), root.join(".opencode/skills")];
    if let Some(home) = std::env::var_os("HOME") {
        for p in [
            ".agents/skills",
            ".codex/skills",
            ".codebuddy/skills",
            ".workbuddy/skills",
        ] {
            paths.push(PathBuf::from(&home).join(p));
        }
    }
    paths
        .into_iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect()
}
fn scan(
    dir: &Path,
    base: &Path,
    depth: usize,
    budget: &mut usize,
    out: &mut Vec<PathBuf>,
    skills: bool,
) -> bool {
    if depth > 6 || *budget == 0 {
        return true;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return true;
    };
    let mut incomplete = false;
    for entry in entries {
        let Ok(entry) = entry else {
            incomplete = true;
            continue;
        };
        if *budget == 0 {
            incomplete = true;
            break;
        }
        *budget -= 1;
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(kind) = entry.file_type() else {
            incomplete = true;
            continue;
        };
        if kind.is_symlink()
            || name.starts_with('.')
            || ["node_modules", "target", "dist", "vendor"].contains(&name.as_str())
        {
            continue;
        }
        if kind.is_dir() {
            incomplete |= scan(&p, base, depth + 1, budget, out, skills);
        } else if kind.is_file()
            && p.starts_with(base)
            && ((skills && name == "SKILL.md")
                || (!skills
                    && ["md", "txt", "rst"]
                        .contains(&p.extension().and_then(|x| x.to_str()).unwrap_or(""))))
        {
            out.push(p);
        }
    }
    incomplete
}
fn skills(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    let mut budget = 4000;
    for base in roots(root) {
        let _ = scan(&base, &base, 0, &mut budget, &mut out, true);
    }
    out.sort();
    out.dedup();
    out
}
fn servers(root: &Path, storage: &Path) -> Vec<(String, String, PathBuf, Value)> {
    let mut sources = vec![
        storage.join("coding-mcp.json"),
        root.join("opencode.json"),
        root.join(".opencode/opencode.json"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home);
        sources.extend([
            h.join(".codebuddy/mcp.json"),
            h.join(".workbuddy/mcp.json"),
            h.join(".config/opencode/opencode.json"),
        ]);
    }
    let mut out = vec![];
    for source in sources {
        if let Ok(text) = read(&source) {
            if let Ok(config) = serde_json::from_str::<Value>(&text) {
                let servers = config
                    .get("mcpServers")
                    .or_else(|| config.get("mcp"))
                    .or_else(|| config.get("servers"));
                if let Some(servers) = servers.and_then(Value::as_object) {
                    for (name, item) in servers.iter().take(100) {
                        let mut hash = std::collections::hash_map::DefaultHasher::new();
                        source.hash(&mut hash);
                        name.hash(&mut hash);
                        out.push((
                            format!("mcp-{:016x}", hash.finish()),
                            name.clone(),
                            source.clone(),
                            item.clone(),
                        ));
                    }
                }
            }
        }
    }
    out
}
fn discover(root: &Path, storage: &Path) -> Value {
    json!({"servers":servers(root,storage).iter().map(|(id,name,source,item)|json!({"id":id,"name":safe_text(name),"source":source,"transport":if item.get("url").is_some(){"http"}else{"stdio"},"connected":false,"requiresApproval":true})).collect::<Vec<_>>()})
}
fn resolved_config(root: &Path, storage: &Path, request: &Value) -> Result<Value, String> {
    if let Some(id) = request["server_id"].as_str() {
        let (_, _, source, mut item) = servers(root, storage)
            .into_iter()
            .find(|s| s.0 == id)
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
            if !Path::new(command).is_absolute() && !command.contains('/') {
                let path = std::env::var_os("PATH").and_then(|paths| {
                    std::env::split_paths(&paths)
                        .filter(|p| p.is_absolute())
                        .map(|p| p.join(command))
                        .find(|p| p.is_file())
                });
                item["command"] = json!(path.ok_or("MCP_EXECUTABLE_NOT_FOUND")?);
            }
        }
        // Existing user application configurations may contain credentials. They remain in native
        // memory and are never returned to the frontend. Project configs must use keyring refs.
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
    child: Child,
    input: std::process::ChildStdin,
    rx: mpsc::Receiver<Result<Value, String>>,
    deadline: Instant,
    secrets: Vec<String>,
}
impl Drop for Session {
    fn drop(&mut self) {
        // Every session owns a fresh process group, including server-spawned descendants.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Session {
    fn start(root: &Path, config: &Value, seconds: u64) -> Result<Self, String> {
        let command = config["command"].as_str().ok_or("MCP_COMMAND_REQUIRED")?;
        let path = PathBuf::from(command);
        if !path.is_absolute() || !path.is_file() {
            return Err("MCP_ABSOLUTE_EXECUTABLE_REQUIRED".into());
        }
        if [
            "sh",
            "bash",
            "dash",
            "zsh",
            "fish",
            "cmd",
            "powershell",
            "pwsh",
        ]
        .contains(&path.file_name().and_then(|s| s.to_str()).unwrap_or(""))
        {
            return Err("MCP_SHELL_COMMAND_REJECTED".into());
        }
        let args = config["args"].as_array().cloned().unwrap_or_default();
        if args.len() > 64
            || args
                .iter()
                .any(|v| v.as_str().is_none_or(|s| s.len() > 4096 || sensitive(s)))
        {
            return Err("MCP_ARGS_INVALID".into());
        }
        let mut cmd = Command::new(path);
        cmd.current_dir(root).env_clear().process_group(0);
        let mut secrets = vec![];
        for name in ["PATH", "HOME", "LANG", "TMPDIR"] {
            if let Some(v) = std::env::var_os(name) {
                cmd.env(name, v);
            }
        }
        if let Some(env) = config["env_refs"].as_object() {
            for (name, reference) in env {
                if name.len() > 100 || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err("MCP_ENV_INVALID".into());
                }
                let reference = reference
                    .as_str()
                    .filter(|s| s.starts_with("ai:") && s.len() < 180)
                    .ok_or("MCP_CREDENTIAL_REF_INVALID")?;
                let value = crate::keyring_store::KeyringStore::get_token(reference)
                    .map_err(|_| "MCP_CREDENTIAL_MISSING")?;
                secrets.push(value.clone());
                cmd.env(name, value);
            }
        }
        if let Some(env) = config["env"].as_object() {
            if env.len() > 64 {
                return Err("MCP_ENV_LIMIT".into());
            }
            for (name, value) in env {
                let value = value
                    .as_str()
                    .filter(|s| s.len() <= 16384)
                    .ok_or("MCP_ENV_INVALID")?;
                if name.is_empty()
                    || name.len() > 100
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err("MCP_ENV_INVALID".into());
                }
                secrets.push(value.to_owned());
                cmd.env(name, value);
            }
        }
        cmd.args(args.iter().filter_map(Value::as_str))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|_| "MCP_START_FAILED")?;
        let input = child.stdin.take().ok_or("MCP_STDIN_FAILED")?;
        let fd = input.as_raw_fd();
        // O_NONBLOCK plus poll keeps a server that stops reading from freezing the caller.
        if unsafe {
            libc::fcntl(
                fd,
                libc::F_SETFL,
                libc::fcntl(fd, libc::F_GETFL) | libc::O_NONBLOCK,
            )
        } < 0
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err("MCP_PIPE_SETUP_FAILED".into());
        }
        let output = child.stdout.take().ok_or("MCP_STDOUT_FAILED")?;
        let (tx, rx) = mpsc::sync_channel(32);
        thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut bytes = vec![];
                match reader
                    .by_ref()
                    .take(LIMIT + 1)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) => break,
                    Ok(_) if bytes.len() as u64 <= LIMIT => {
                        let value =
                            serde_json::from_slice(&bytes).map_err(|_| "MCP_INVALID_FRAME".into());
                        if tx.try_send(value).is_err() {
                            break;
                        }
                    }
                    _ => {
                        let _ = tx.try_send(Err("MCP_FRAME_LIMIT".into()));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            input,
            rx,
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
        let mut offset = 0;
        while offset < bytes.len() {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("MCP_WRITE_TIMEOUT".into());
            }
            let mut fd = libc::pollfd {
                fd: self.input.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut fd, 1, remaining.as_millis().min(1000) as i32) };
            if ready < 0 {
                return Err("MCP_PIPE_FAILED".into());
            }
            if ready == 0 {
                continue;
            }
            match self.input.write(&bytes[offset..]) {
                Ok(0) => return Err("MCP_WRITE_FAILED".into()),
                Ok(n) => offset += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(_) => return Err("MCP_WRITE_FAILED".into()),
            }
        }
        Ok(())
    }
    fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            let value = self
                .rx
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
/// Requests originate in the trusted workbench; MCP execution requires a fresh UI approval.
pub fn execute(root: &Path, storage: &Path, request: Value) -> Result<Value, String> {
    let root = root.canonicalize().map_err(|_| "CONTEXT_ROOT_INVALID")?;
    match request["action"].as_str().unwrap_or("") {
        "skills_list" => Ok(
            json!({"skills":skills(&root).iter().take(256).map(|p|json!({"path":p,"name":p.parent().and_then(Path::file_name).map(|s|s.to_string_lossy()),"description":read(p).map(|t|safe_text(&t).lines().find(|s|s.starts_with("description:")).unwrap_or("").chars().take(240).collect::<String>()).unwrap_or_default()})).collect::<Vec<_>>()}),
        ),
        "skill_read" => {
            let path = PathBuf::from(request["path"].as_str().ok_or("SKILL_PATH_REQUIRED")?)
                .canonicalize()
                .map_err(|_| "SKILL_NOT_FOUND")?;
            if !skills(&root).contains(&path) {
                return Err("SKILL_OUTSIDE_ALLOWED_ROOTS".into());
            }
            Ok(json!({"path":path,"text":safe_text(&read(&path)?)}))
        }
        "knowledge_search" => {
            let query = request["query"]
                .as_str()
                .filter(|q| q.len() >= 2 && q.len() <= 200)
                .ok_or("KNOWLEDGE_QUERY_REQUIRED")?
                .to_lowercase();
            let base = root
                .join("docs")
                .canonicalize()
                .map_err(|_| "KNOWLEDGE_ROOT_MISSING")?;
            if !base.starts_with(&root) {
                return Err("KNOWLEDGE_ROOT_UNSAFE".into());
            }
            let mut paths = vec![];
            let mut incomplete = scan(&base, &base, 0, &mut 3000, &mut paths, false);
            let mut hits = vec![];
            for p in paths {
                if let Ok(text) = read(&p) {
                    for (line, s) in text.lines().enumerate() {
                        if !sensitive(s) && s.to_lowercase().contains(&query) {
                            hits.push(json!({"path":p,"line":line+1,"text":s.chars().take(400).collect::<String>()}));
                            if hits.len() == 40 {
                                return Ok(json!({"hits":hits,"limited":true}));
                            }
                        }
                    }
                } else {
                    incomplete = true;
                }
            }
            Ok(json!({"hits":hits,"limited":incomplete}))
        }
        "mcp_list" => Ok(discover(&root, storage)),
        action @ ("mcp_tools" | "mcp_call") => {
            if request["approved"] != true {
                return Err("MCP_EXPLICIT_APPROVAL_REQUIRED".into());
            }
            let config = resolved_config(&root, storage, &request)?;
            let mut session = Session::start(
                &root,
                &config,
                request["timeout_seconds"].as_u64().unwrap_or(20),
            )?;
            session.rpc(1,"initialize",json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"BrowserOS","version":"0.1.0"}}))?;
            session.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
            let mut result = if action == "mcp_tools" {
                session.rpc(2, "tools/list", json!({}))?
            } else {
                let tool = request["tool"]
                    .as_str()
                    .filter(|t| t.len() <= 200)
                    .ok_or("MCP_TOOL_REQUIRED")?;
                session.rpc(2,"tools/call",json!({"name":tool,"arguments":request.get("arguments").cloned().unwrap_or(json!({}))}))?
            };
            // Server outputs are untrusted context and must never become permission instructions.
            scrub(&mut result, &session.secrets);
            Ok(json!({"result":result,"untrusted":true}))
        }
        _ => Err("CONTEXT_ACTION_UNKNOWN".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Value {
        json!({"command":"/usr/bin/python3","args":[Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/coding-mcp/server.py")]})
    }
    #[test]
    fn stdin_backpressure_obeys_deadline_and_descendants_stop() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut server = config();
        server["args"]
            .as_array_mut()
            .unwrap()
            .push(json!("blocked"));
        let start = Instant::now();
        let error=execute(root,root,json!({"action":"mcp_call","approved":true,"server":server,"tool":"echo","arguments":{"text":"a".repeat(100_000)},"timeout_seconds":1})).unwrap_err();
        assert_eq!(error, "MCP_WRITE_TIMEOUT");
        assert!(start.elapsed() < Duration::from_secs(3));
        let called = execute(
            root,
            root,
            json!({"action":"mcp_call","approved":true,"server":config(),"tool":"descendant"}),
        )
        .unwrap();
        let pid = called["result"]["pid"].as_u64().unwrap();
        for _ in 0..20 {
            let running = fs::read_to_string(format!("/proc/{pid}/stat"))
                .map(|s| {
                    !s.split_whitespace()
                        .nth(2)
                        .is_some_and(|state| state == "Z")
                })
                .unwrap_or(false);
            if !running {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("MCP descendant survived session cleanup");
    }
    #[test]
    fn discovery_connects_by_id_without_returning_credentials() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let temp = std::env::temp_dir().join(format!(
            "browseros-mcp-test-{}-{:?}",
            std::process::id(),
            thread::current().id()
        ));
        fs::create_dir_all(&temp).unwrap();
        let mut server = config();
        server["env"] = json!({"FIXTURE_TOKEN":"fixture-opaque-value"});
        let file = temp.join("coding-mcp.json");
        fs::write(&file, json!({"servers":{"fixture":server}}).to_string()).unwrap();
        let listed = discover(root, &temp);
        assert!(!listed.to_string().contains("fixture-opaque-value"));
        let id = listed["servers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == "fixture")
            .unwrap()["id"]
            .clone();
        let called = execute(
            root,
            &temp,
            json!({"action":"mcp_tools","approved":true,"server_id":id.clone()}),
        )
        .unwrap();
        assert_eq!(called["result"]["tools"][0]["name"], "echo");
        let echoed = execute(
            root,
            &temp,
            json!({"action":"mcp_call","approved":true,"server_id":id,"tool":"echo_env"}),
        )
        .unwrap();
        assert_eq!(echoed["result"]["content"][0]["text"], "[redacted]");
        fs::remove_file(file).unwrap();
        fs::remove_dir(temp).unwrap();
    }
    #[test]
    fn real_stdio_initializes_lists_and_calls() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let listed = execute(
            root,
            root,
            json!({"action":"mcp_tools","approved":true,"server":config()}),
        )
        .unwrap();
        assert_eq!(listed["result"]["tools"][0]["name"], "echo");
        let called=execute(root,root,json!({"action":"mcp_call","approved":true,"server":config(),"tool":"echo","arguments":{"text":"fixture output"}})).unwrap();
        assert_eq!(called["result"]["content"][0]["text"], "fixture output");
        assert!(called["untrusted"].as_bool().unwrap());
    }
    #[test]
    fn errors_and_timeouts_do_not_leak_remote_output() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            execute(
                root,
                root,
                json!({"action":"mcp_call","approved":true,"server":config(),"tool":"error"})
            )
            .unwrap_err(),
            "MCP_REMOTE_ERROR"
        );
        let mut server = config();
        server["args"]
            .as_array_mut()
            .unwrap()
            .push(json!("timeout"));
        let start = Instant::now();
        assert_eq!(
            execute(
                root,
                root,
                json!({"action":"mcp_tools","approved":true,"server":server,"timeout_seconds":1})
            )
            .unwrap_err(),
            "MCP_TIMEOUT_OR_CLOSED"
        );
        assert!(start.elapsed() < Duration::from_secs(3));
    }
    #[test]
    fn approval_paths_and_secret_filters() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            execute(root, root, json!({"action":"mcp_tools","server":config()})).unwrap_err(),
            "MCP_EXPLICIT_APPROVAL_REQUIRED"
        );
        assert!(execute(
            root,
            root,
            json!({"action":"skill_read","path":"/etc/passwd"})
        )
        .is_err());
        assert!(!safe_text("title\nAPI_KEY=fixture-secret\nend").contains("fixture-secret"));
        let mut server = config();
        server["command"] = json!("python3");
        assert!(Session::start(root, &server, 1).is_err());
    }
    #[test]
    fn knowledge_search_marks_unreadable_and_depth_limited_documents() {
        let root =
            std::env::temp_dir().join(format!("browseros-knowledge-{}", uuid::Uuid::new_v4()));
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("good.md"), "fixture needle\n").unwrap();
        fs::write(docs.join("not-text.md"), [0xff, 0xfe]).unwrap();
        let result = execute(
            &root,
            &root,
            json!({"action":"knowledge_search","query":"needle"}),
        )
        .unwrap();
        assert_eq!(result["hits"].as_array().unwrap().len(), 1);
        assert_eq!(result["limited"], true);
        fs::remove_file(docs.join("not-text.md")).unwrap();
        let deep = docs.join("a/b/c/d/e/f/g");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("hidden.md"), "fixture needle\n").unwrap();
        let result = execute(
            &root,
            &root,
            json!({"action":"knowledge_search","query":"needle"}),
        )
        .unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(result["hits"].as_array().unwrap().len(), 1);
        assert_eq!(result["limited"], true);
    }
}
