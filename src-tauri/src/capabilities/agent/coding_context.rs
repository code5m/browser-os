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
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const LIMIT: u64 = 128 * 1024;
fn read(path: &Path, allowed: &Path) -> Result<String, String> {
    let snapshot = crate::capabilities::workspace::safe_paths::read_versioned(
        path, &[allowed.to_path_buf()], LIMIT as usize,
    ).map_err(|_| "CONTEXT_READ_DENIED")?.ok_or("CONTEXT_READ_FAILED")?;
    String::from_utf8(snapshot.bytes).map_err(|_| "CONTEXT_NOT_TEXT".into())
}
use super::mcp_protocol::{self, safe_text, scrub, sensitive, RpcFrame};
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
        // Do not canonicalize an untrusted skill-root link into a new grant.
        .filter(|p| crate::capabilities::workspace::safe_paths::directory_identity(p).is_ok())
        .collect()
}
fn context_check(cancelled: &dyn Fn() -> bool) -> Result<(), String> {
    if cancelled() { Err("AGENT_CANCELLED".into()) } else { Ok(()) }
}

fn scan(
    dir: &Path,
    base: &Path,
    depth: usize,
    budget: &mut usize,
    out: &mut Vec<PathBuf>,
    skills: bool,
    cancelled: &dyn Fn() -> bool,
) -> Result<bool, String> {
    context_check(cancelled)?;
    if depth > 6 || *budget == 0 {
        return Ok(true);
    }
    let Ok(entries) = fs::read_dir(dir) else {
        context_check(cancelled)?;
        return Ok(true);
    };
    let mut incomplete = false;
    for entry in entries {
        context_check(cancelled)?;
        if *budget == 0 {
            incomplete = true;
            break;
        }
        // Failed iterator entries consume the same finite scan budget.
        *budget -= 1;
        let Ok(entry) = entry else {
            incomplete = true;
            continue;
        };
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
            incomplete |= scan(&p, base, depth + 1, budget, out, skills, cancelled)?;
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
    context_check(cancelled)?;
    Ok(incomplete)
}
fn skills(allowed: &[PathBuf], cancelled: &dyn Fn() -> bool) -> Result<Vec<PathBuf>, String> {
    let mut out = vec![];
    let mut budget = 4000;
    for base in allowed {
        scan(base, base, 0, &mut budget, &mut out, true, cancelled)?;
    }
    out.sort();
    out.dedup();
    context_check(cancelled)?;
    Ok(out)
}
fn servers(root: &Path, storage: &Path) -> Vec<(String, String, PathBuf, Value)> {
    let mut sources = vec![
        (storage.join("coding-mcp.json"), storage.to_path_buf()),
        (root.join("opencode.json"), root.to_path_buf()),
        (root.join(".opencode/opencode.json"), root.to_path_buf()),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home);
        sources.extend([
            (h.join(".codebuddy/mcp.json"), h.clone()),
            (h.join(".workbuddy/mcp.json"), h.clone()),
            (h.join(".config/opencode/opencode.json"), h.clone()),
        ]);
    }
    let mut out = vec![];
    for (source, allowed) in sources {
        // Config origin is the fixed host/workspace/home root, not the
        // current canonical parent of an attacker-controlled path.
        if crate::capabilities::workspace::safe_paths::directory_identity(&allowed).is_err() {
            continue;
        }
        let content = read(&source, &allowed);
        if let Ok(text) = content {
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
        if !item.is_object() {
            return Err("MCP_CONFIG_INVALID".into());
        }
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
        item["_browseros_source"] = json!(source);
        Ok(item)
    } else {
        let mut item = request["server"].clone();
        if !item.is_object() {
            return Err("MCP_CONFIG_INVALID".into());
        }
        if item.get("env").is_some() || item.get("environment").is_some() {
            return Err("MCP_PLAINTEXT_ENV_REJECTED".into());
        }
        item["_browseros_source"] = json!("inline");
        Ok(item)
    }
}
struct Session {
    child: Child,
    input: std::process::ChildStdin,
    rx: mpsc::Receiver<Result<Value, String>>,
    reader_fault: Arc<Mutex<Option<String>>>,
    deadline: Instant,
    secrets: Vec<String>,
    cancellation: Arc<AtomicBool>,
    reaped: bool,
    scope: Option<crate::shared::process_scope::ProcessScope>,
    reader: Option<thread::JoinHandle<()>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.terminate_confirmed();
        }
    }
}
impl Session {
    fn terminate_confirmed(&mut self) -> Result<(), String> {
        if self.reaped {
            return Err("MCP_TERMINATION_ALREADY_REAPED".into());
        }
        let pgid = self.child.id() as i32;
        // Signal the owned group before reaping its leader. Never signal the
        // numeric id again after reaping (it could have been reused).
        unsafe {
            libc::kill(-pgid, libc::SIGKILL);
        }
        if let Some(scope) = &self.scope {
            let _ = scope.request_kill();
        }
        let _ = self.child.kill();
        // Never signal a numeric PGID again after a wait attempt with an
        // unknown receipt. The pinned cgroup remains an independent owner.
        self.reaped = true;
        self.child.wait().map_err(|_| "MCP_TERMINATION_UNKNOWN")?;
        let deadline = Instant::now() + Duration::from_secs(5);
        if let Some(scope) = &self.scope {
            scope
                .terminate_and_confirm(deadline)
                .map_err(|_| "MCP_TERMINATION_UNKNOWN")?;
        } else {
            loop {
                if unsafe { libc::kill(-pgid, 0) } != 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err("MCP_TERMINATION_UNKNOWN".into());
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        if let Some(reader) = &self.reader {
            while !reader.is_finished() {
                if Instant::now() >= deadline {
                    return Err("MCP_TERMINATION_UNKNOWN".into());
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        if let Some(reader) = self.reader.take() {
            reader.join().map_err(|_| "MCP_TERMINATION_UNKNOWN")?;
        }
        Ok(())
    }
    #[cfg(test)]
    fn start(root: &Path, config: &Value, seconds: u64) -> Result<Self, String> {
        Self::start_with_credentials(root, config, seconds, std::collections::BTreeMap::new())
    }
    fn start_with_credentials(
        root: &Path,
        config: &Value,
        seconds: u64,
        mut credentials: std::collections::BTreeMap<String, String>,
    ) -> Result<Self, String> {
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
                let _ = reference; // Already bound and captured by the credential Owner.
                let value = credentials.remove(name).ok_or("MCP_CREDENTIAL_MISSING")?;
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
        let mut scope = crate::shared::process_scope::ProcessScope::prepare()
            .map_err(|_| "MCP_CGROUP_UNAVAILABLE")?;
        if let Some(scope) = &scope {
            scope
                .attach(&mut cmd)
                .map_err(|_| "MCP_CGROUP_UNAVAILABLE")?;
        }
        let mut child = cmd.spawn().map_err(|_| "MCP_START_FAILED")?;
        if let Some(scope) = &mut scope {
            scope.mark_spawned();
        }
        let terminate_start_failure = |child: &mut Child| {
            let pgid = child.id() as i32;
            unsafe {
                libc::kill(-pgid, libc::SIGKILL);
            }
            if let Some(scope) = &scope {
                let _ = scope.request_kill();
            }
            let _ = child.kill();
            let _ = child.wait();
        };
        let input = match child.stdin.take() {
            Some(input) => input,
            None => {
                terminate_start_failure(&mut child);
                return Err("MCP_STDIN_FAILED".into());
            }
        };
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
            terminate_start_failure(&mut child);
            return Err("MCP_PIPE_SETUP_FAILED".into());
        }
        let output = match child.stdout.take() {
            Some(output) => output,
            None => {
                terminate_start_failure(&mut child);
                return Err("MCP_STDOUT_FAILED".into());
            }
        };
        let (tx, rx) = mpsc::sync_channel(32);
        let reader_fault = Arc::new(Mutex::new(None));
        let on_read_failure = Arc::clone(&reader_fault);
        let reader = match thread::Builder::new()
            .name("browseros-mcp-reader".into())
            .spawn(move || {
                let mut reader = BufReader::new(output);
                loop {
                    let mut bytes = vec![];
                    match reader
                        .by_ref()
                        .take(LIMIT + 2)
                        .read_until(b'\n', &mut bytes)
                    {
                        Ok(0) => break,
                        Ok(_) => {
                            let value = match mcp_protocol::decode_line(&bytes, LIMIT as usize) {
                                Ok(value) => value,
                                Err(error) => {
                                    *on_read_failure.lock().unwrap_or_else(|p| p.into_inner()) = Some(error);
                                    break;
                                }
                            };
                            if tx.try_send(Ok(value)).is_err() {
                                *on_read_failure.lock().unwrap_or_else(|p| p.into_inner()) =
                                    Some("MCP_OUTPUT_BACKPRESSURE".into());
                                break;
                            }
                        }
                        Err(_) => {
                            *on_read_failure.lock().unwrap_or_else(|p| p.into_inner()) =
                                Some("MCP_FRAME_READ".into());
                            break;
                        }
                    }
                }
            }) {
            Ok(reader) => reader,
            Err(_) => {
                terminate_start_failure(&mut child);
                return Err("MCP_READER_START_FAILED".into());
            }
        };
        Ok(Self {
            child,
            input,
            rx,
            reader_fault,
            deadline: Instant::now() + Duration::from_secs(seconds.clamp(1, 60)),
            secrets,
            cancellation: Arc::new(AtomicBool::new(false)),
            reaped: false,
            scope,
            reader: Some(reader),
        })
    }
    fn send(&mut self, value: Value) -> Result<(), String> {
        self.send_until(value, self.deadline, true)
    }
    fn send_until(
        &mut self, value: Value, deadline: Instant, observe_cancellation: bool,
    ) -> Result<(), String> {
        if observe_cancellation && self.cancellation.load(Ordering::Acquire) {
            return Err("MCP_CANCEL_REQUESTED".into());
        }
        if Instant::now() >= deadline {
            return Err("MCP_WRITE_TIMEOUT".into());
        }
        let mut bytes = mcp_protocol::encode_bounded(&value, LIMIT as usize)?;
        bytes.push(b'\n');
        let mut offset = 0;
        while offset < bytes.len() {
            if observe_cancellation && self.cancellation.load(Ordering::Acquire) {
                return Err("MCP_CANCEL_REQUESTED".into());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("MCP_WRITE_TIMEOUT".into());
            }
            let mut fd = libc::pollfd {
                fd: self.input.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut fd, 1, remaining.as_millis().min(100) as i32) };
            if ready < 0 {
                return Err("MCP_PIPE_FAILED".into());
            }
            if ready == 0 {
                continue;
            }
            if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                return Err("MCP_PIPE_FAILED".into());
            }
            if fd.revents & libc::POLLOUT == 0 { continue; }
            // poll may wake after cancellation/deadline. Do not begin
            // another fragment of the original frame on that readiness.
            if observe_cancellation && self.cancellation.load(Ordering::Acquire) {
                return Err("MCP_CANCEL_REQUESTED".into());
            }
            if Instant::now() >= deadline { return Err("MCP_WRITE_TIMEOUT".into()); }
            match self.input.write(&bytes[offset..]) {
                Ok(0) => return Err("MCP_WRITE_FAILED".into()),
                Ok(n) => offset += n,
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted) => continue,
                Err(_) => return Err("MCP_WRITE_FAILED".into()),
            }
        }
        if observe_cancellation && self.cancellation.load(Ordering::Acquire) {
            return Err("MCP_CANCEL_REQUESTED".into());
        }
        if Instant::now() >= deadline { return Err("MCP_WRITE_TIMEOUT".into()); }
        Ok(())
    }
    fn notify_cancelled(&mut self, id: u64, method: &str) {
        if let Some(notification) = mcp_protocol::cancellation_notification(id, method) {
            // Separate short cleanup budget; no retries or provider outcome claim.
            let _ = self.send_until(notification, Instant::now() + Duration::from_millis(200), false);
        }
    }
    fn reader_error(&self) -> Option<String> {
        self.reader_fault.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        if let Some(error) = self.reader_error() { return Err(error); }
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            if let Some(error) = self.reader_error() { return Err(error); }
            if self.cancellation.load(Ordering::Acquire) {
                self.notify_cancelled(id, method);
                return Err("MCP_CANCEL_REQUESTED".into());
            }
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.notify_cancelled(id, method);
                return Err("MCP_TIMEOUT_OR_CLOSED".into());
            }
            let value = match self
                .rx
                .recv_timeout(remaining.min(Duration::from_millis(100)))
            {
                Ok(value) => value?,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self.reader_error().unwrap_or_else(|| "MCP_TIMEOUT_OR_CLOSED".into()))
                }
            };
            // A full queue or invalid frame ends the transport. A reply
            // buffered before that fault must not silently hide the loss.
            if let Some(error) = self.reader_error() { return Err(error); }
            match mcp_protocol::classify_frame(value, id)? {
                RpcFrame::Reply(result) => {
                    if let Some(error) = self.reader_error() { return Err(error); }
                    // A reply queued before the deadline is not a grant
                    // to accept it after cancellation or expiry.
                    if self.cancellation.load(Ordering::Acquire) {
                        self.notify_cancelled(id, method);
                        return Err("MCP_CANCEL_REQUESTED".into());
                    }
                    if Instant::now() >= self.deadline {
                        self.notify_cancelled(id, method);
                        return Err("MCP_TIMEOUT_OR_CLOSED".into());
                    }
                    return Ok(result);
                },
                RpcFrame::ServerRequest(reply) => self.send(reply)?,
                RpcFrame::Ignore => {}
            }
        }
    }
}
/// Requests originate in the trusted workbench; MCP execution requires a fresh UI approval.
pub fn execute(root: &Path, storage: &Path, request: Value) -> Result<Value, String> {
    execute_cancelable(root, storage, request, Arc::new(AtomicBool::new(false)))
}
pub(crate) fn execute_cancelable(
    root: &Path,
    storage: &Path,
    request: Value,
    cancellation: Arc<AtomicBool>,
) -> Result<Value, String> {
    execute_cancelable_with_admission(root, storage, request, cancellation, |_| Ok(()))
}

/// The native owner publishes intent only after local validation and
/// immediately before starting an external MCP executable. The callback
/// is host-only, not supplied by the request, model or server.
pub(super) fn execute_cancelable_with_admission(
    root: &Path,
    storage: &Path,
    request: Value,
    cancellation: Arc<AtomicBool>,
    before_spawn: impl FnOnce(&Value) -> Result<(), String>,
) -> Result<Value, String> {
    execute_cancelable_with_effect_admission(
        root, storage, request, cancellation, None, |request, _| before_spawn(request),
    )
}

/// The model loop supplies the live Task/session cancellation check. The
/// request facade can use its registered run signal. No new effect admission.
pub(crate) fn execute_read_context(
    root: &Path, request: &Value, cancelled: &dyn Fn() -> bool,
) -> Result<Value, String> {
    context_check(cancelled)?;
    let root = root.canonicalize().map_err(|_| "CONTEXT_ROOT_INVALID")?;
    let result = match request["action"].as_str().unwrap_or("") {
        "skills_list" => {
            let allowed = roots(&root);
            let mut listed = Vec::new();
            for path in skills(&allowed, cancelled)?.into_iter().take(256) {
                context_check(cancelled)?;
                let Some(base) = allowed.iter().find(|base| path.starts_with(base)) else { continue; };
                let result = read(&path, base);
                context_check(cancelled)?;
                let Ok(text) = result else { continue; };
                let description = safe_text(&text).lines()
                    .find(|line| line.starts_with("description:")).unwrap_or("")
                    .chars().take(240).collect::<String>();
                listed.push(json!({"path":path,"name":path.parent().and_then(Path::file_name)
                    .map(|name| name.to_string_lossy()),"description":description}));
            }
            Ok(json!({"skills":listed}))
        }
        "skill_read" => {
            let path = PathBuf::from(request["path"].as_str().ok_or("SKILL_PATH_REQUIRED")?);
            let allowed = roots(&root);
            if !skills(&allowed, cancelled)?.contains(&path) {
                return Err("SKILL_OUTSIDE_ALLOWED_ROOTS".into());
            }
            let base = allowed.iter().find(|base| path.starts_with(base))
                .ok_or("SKILL_OUTSIDE_ALLOWED_ROOTS")?;
            context_check(cancelled)?;
            let text = safe_text(&read(&path, base)?);
            context_check(cancelled)?;
            Ok(json!({"path":path,"text":text}))
        }
        "knowledge_search" => {
            let query = request["query"]
                .as_str()
                .filter(|q| q.len() >= 2 && q.len() <= 200)
                .ok_or("KNOWLEDGE_QUERY_REQUIRED")?
                .to_lowercase();
            let base = root.join("docs");
            crate::capabilities::workspace::safe_paths::directory_identity(&base)
                .map_err(|_| "KNOWLEDGE_ROOT_UNSAFE")?;
            let mut paths = vec![];
            let mut incomplete = scan(&base, &base, 0, &mut 3000, &mut paths, false, cancelled)?;
            let mut hits = vec![];
            for p in paths {
                context_check(cancelled)?;
                let result = read(&p, &base);
                context_check(cancelled)?;
                if let Ok(text) = result {
                    for (line, s) in text.lines().enumerate() {
                        if line % 64 == 0 { context_check(cancelled)?; }
                        if !sensitive(s) && s.to_lowercase().contains(&query) {
                            hits.push(json!({"path":p,"line":line+1,"text":s.chars().take(400).collect::<String>()}));
                            if hits.len() == 40 {
                                context_check(cancelled)?;
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
        _ => Err("CONTEXT_ACTION_UNKNOWN".into()),
    };
    context_check(cancelled)?;
    result
}

pub(super) fn execute_cancelable_with_effect_admission(
    root: &Path,
    storage: &Path,
    request: Value,
    cancellation: Arc<AtomicBool>,
    run: Option<uuid::Uuid>,
    before_spawn: impl FnOnce(&Value, Option<&super::mcp_authority::Binding>) -> Result<(), String>,
) -> Result<Value, String> {
    if matches!(request["action"].as_str(), Some("skills_list" | "skill_read" | "knowledge_search")) {
        return execute_read_context(root, &request, &|| cancellation.load(std::sync::atomic::Ordering::Acquire));
    }
    let root = root.canonicalize().map_err(|_| "CONTEXT_ROOT_INVALID")?;
    match request["action"].as_str().unwrap_or("") {
        "mcp_list" => Ok(discover(&root, storage)),
        "mcp_review" => {
            let config = resolved_config(&root, storage, &request)?;
            Ok(super::mcp_review::prepare(&root, storage, &request, config)?.review)
        }
        action @ ("mcp_tools" | "mcp_call") => {
            if request["approved"] != true {
                return Err("MCP_EXPLICIT_APPROVAL_REQUIRED".into());
            }
            let config = resolved_config(&root, storage, &request)?;
            let prepared = super::mcp_review::prepare(&root, storage, &request, config)?;
            super::mcp_review::authorize(&prepared, &request)?;
            let binding = super::mcp_authority::Binding::from_review(storage, &prepared, run)?;
            let (method, params) =
                mcp_protocol::owned_tool_request(action, &request, binding.as_ref())?;
            let credentials = crate::framework::ai::mcp_credential_snapshot(
                storage,
                prepared.config["env_refs"].as_object(),
                &prepared.review["credentials"],
            )?;
            if cancellation.load(Ordering::Acquire) {
                return Err("MCP_CANCEL_REQUESTED".into());
            }
            before_spawn(&request, binding.as_ref())?;
            // Admission may have taken time. Do not spawn after a newly observed
            // cancellation; a published intent remains conservatively unresolved.
            if cancellation.load(Ordering::Acquire) {
                return Err("MCP_CANCEL_REQUESTED".into());
            }
            let mut session = Session::start_with_credentials(
                &root,
                &prepared.config,
                request["timeout_seconds"].as_u64().unwrap_or(20),
                credentials,
            )?;
            session.cancellation = cancellation;
            let outcome = (|| -> Result<Value, String> {
                let initialized =
                    session.rpc(1, "initialize", mcp_protocol::initialize_params())?;
                mcp_protocol::validate_initialize(&initialized)?;
                session.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
                let mut result = session.rpc(2, method, params)?;
                // Server outputs are untrusted context and must never become permission instructions.
                scrub(&mut result, &session.secrets);
                Ok(json!({"result":result,"untrusted":true}))
            })();
            session.terminate_confirmed()?;
            // Cancellation during native teardown cannot become a fresh success.
            if outcome.is_ok() && session.cancellation.load(Ordering::Acquire) {
                return Err("MCP_CANCEL_REQUESTED".into());
            }
            outcome
        }
        _ => Err("CONTEXT_ACTION_UNKNOWN".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercise the same read-only review and exact grant round trip as the UI.
    fn execute_reviewed(root: &Path, storage: &Path, mut request: Value) -> Result<Value, String> {
        if request["approved"] == true
            && matches!(request["action"].as_str(), Some("mcp_tools" | "mcp_call"))
        {
            let mut review_request = request.clone();
            review_request["review_action"] = request["action"].clone();
            review_request["action"] = json!("mcp_review");
            let review = super::execute(root, storage, review_request)?;
            request["review_revision"] = review["revision"].clone();
            request["approved_credential_refs"] = review["credential_refs"].clone();
            request["approved_environment_names"] = review["environment_names"].clone();
        }
        super::execute(root, storage, request)
    }

    #[test]
    fn missing_approval_never_calls_native_admission() {
        let root =
            std::env::temp_dir().join(format!("browseros-mcp-admit-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let result = execute_cancelable_with_admission(
            &root,
            &root,
            json!({"action":"mcp_call","tool":"fixture"}),
            Arc::new(AtomicBool::new(false)),
            |_| panic!("invalid request must not admit a process"),
        );
        assert_eq!(result.unwrap_err(), "MCP_EXPLICIT_APPROVAL_REQUIRED");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn changing_saved_config_after_review_is_rejected_before_start() {
        let root = std::env::temp_dir().join(format!(
            "browseros-mcp-review-root-{}",
            uuid::Uuid::new_v4()
        ));
        let storage = std::env::temp_dir().join(format!(
            "browseros-mcp-review-store-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&storage).unwrap();
        let file = storage.join("coding-mcp.json");
        let server = json!({"command":std::env::current_exe().unwrap(),"args":["old"]});
        fs::write(&file, json!({"servers":{"fixture":server}}).to_string()).unwrap();
        let id = discover(&root, &storage)["servers"][0]["id"].clone();
        let review = super::execute(
            &root,
            &storage,
            json!({"action":"mcp_review","review_action":"mcp_tools","server_id":id}),
        )
        .unwrap();
        let request = json!({"action":"mcp_tools","server_id":id,"approved":true,
            "review_revision":review["revision"],"approved_credential_refs":review["credential_refs"],
            "approved_environment_names":review["environment_names"]});
        fs::write(&file, json!({"servers":{"fixture":{"command":std::env::current_exe().unwrap(),"args":["new"]}}}).to_string()).unwrap();
        assert_eq!(
            super::execute(&root, &storage, request).unwrap_err(),
            "MCP_REVIEW_CHANGED"
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(storage).unwrap();
    }
    #[test]
    fn credential_property_names_are_scrubbed_recursively() {
        let mut response =
            json!({"nested":{"fixture-opaque-secret":"value"},"text":"fixture-opaque-secret"});
        scrub(&mut response, &["fixture-opaque-secret".into()]);
        assert!(!response.to_string().contains("fixture-opaque-secret"));
    }
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
        let error=execute_reviewed(root,root,json!({"action":"mcp_call","approved":true,"server":server,"tool":"echo","arguments":{"text":"a".repeat(100_000)},"timeout_seconds":1})).unwrap_err();
        assert_eq!(error, "MCP_WRITE_TIMEOUT");
        assert!(start.elapsed() < Duration::from_secs(3));
        let called = execute_reviewed(
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
        let called = execute_reviewed(
            root,
            &temp,
            json!({"action":"mcp_tools","approved":true,"server_id":id.clone()}),
        )
        .unwrap();
        assert_eq!(called["result"]["tools"][0]["name"], "echo");
        let echoed = execute_reviewed(
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
        let listed = execute_reviewed(
            root,
            root,
            json!({"action":"mcp_tools","approved":true,"server":config()}),
        )
        .unwrap();
        assert_eq!(listed["result"]["tools"][0]["name"], "echo");
        let called=execute_reviewed(root,root,json!({"action":"mcp_call","approved":true,"server":config(),"tool":"echo","arguments":{"text":"fixture output"}})).unwrap();
        assert_eq!(called["result"]["content"][0]["text"], "fixture output");
        assert!(called["untrusted"].as_bool().unwrap());
    }
    #[test]
    fn errors_and_timeouts_do_not_leak_remote_output() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            execute_reviewed(
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
            execute_reviewed(
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
            execute_reviewed(root, root, json!({"action":"mcp_tools","server":config()}))
                .unwrap_err(),
            "MCP_EXPLICIT_APPROVAL_REQUIRED"
        );
        assert!(execute_reviewed(
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
        let result = execute_reviewed(
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
        let result = execute_reviewed(
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
