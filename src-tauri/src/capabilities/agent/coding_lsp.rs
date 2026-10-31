//! Bounded Rust LSP requests in a minimal filesystem/network namespace.
//! Copies only validated source snapshots; never mounts the real project/home.
//! No Cargo, shell, build script or proc-macro execution.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{DirBuilderExt, PermissionsExt},
            process::CommandExt,
        },
    },
    path::{Component, Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Arc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

const LIMIT: usize = 1024 * 1024;
const MAX_CACHED_SESSIONS: usize = 2;
const SESSION_IDLE_TTL: Duration = Duration::from_secs(300);
static SESSION_CACHE: OnceLock<Mutex<HashMap<String, Arc<Mutex<CachedSession>>>>> = OnceLock::new();

fn profiles() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return vec![];
    };
    let mut result = vec![];
    for base in [
        PathBuf::from(&home).join(".local/share/rustup/toolchains"),
        PathBuf::from(&home).join(".rustup/toolchains"),
    ] {
        let Ok(entries) = fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.take(32).flatten() {
            if !entry.file_name().to_string_lossy().starts_with("stable-") {
                continue;
            }
            if let Ok(path) = entry.path().join("bin/rust-analyzer").canonicalize() {
                if path.starts_with(&base) && path.is_file() && !result.contains(&path) {
                    result.push(path);
                }
            }
        }
    }
    result.sort();
    result
}
fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => result.push(part.as_os_str()),
        }
    }
    result
}
fn libraries(server: &Path) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut child = Command::new("/usr/bin/ldd")
        .arg(server)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .current_dir("/")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "LSP_LIBRARY_DISCOVERY_FAILED")?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| "LSP_LIBRARY_DISCOVERY_FAILED")?
        {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("LSP_LIBRARY_DISCOVERY_TIMEOUT".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    let mut output = String::new();
    child
        .stdout
        .take()
        .ok_or("LSP_LIBRARY_DISCOVERY_FAILED")?
        .take(16385)
        .read_to_string(&mut output)
        .map_err(|_| "LSP_LIBRARY_DISCOVERY_FAILED")?;
    if !status.success() || output.len() > 16384 {
        return Err("LSP_LIBRARY_DISCOVERY_FAILED".into());
    }
    let mut result = vec![];
    for line in output.lines() {
        let text = line
            .split_once("=>")
            .map(|(_, value)| value)
            .unwrap_or(line)
            .trim();
        let Some(name) = text
            .split_whitespace()
            .next()
            .filter(|s| s.starts_with('/'))
        else {
            continue;
        };
        let target = normalized(Path::new(name));
        let source = target.canonicalize().map_err(|_| "LSP_LIBRARY_MISSING")?;
        if !source.is_file() {
            return Err("LSP_LIBRARY_INVALID".into());
        }
        if !result.iter().any(|(_, path)| path == &target) {
            result.push((source, target));
        }
    }
    if result.is_empty() {
        return Err("LSP_LIBRARY_DISCOVERY_FAILED".into());
    }
    Ok(result)
}

fn frame(reader: &mut impl BufRead) -> Result<Value, String> {
    let mut length = None;
    let mut count = 0;
    loop {
        let mut line = String::new();
        let n = reader
            .by_ref()
            .take(8193)
            .read_line(&mut line)
            .map_err(|_| "LSP_FRAME_READ_FAILED")?;
        count += n;
        if n == 0 || count > 8192 {
            return Err("LSP_FRAME_HEADER_INVALID".into());
        }
        if line == "\r\n" {
            break;
        }
        let (key, value) = line.split_once(':').ok_or("LSP_FRAME_HEADER_INVALID")?;
        if key.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err("LSP_FRAME_HEADER_INVALID".into());
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "LSP_FRAME_HEADER_INVALID")?,
            );
        }
    }
    let length = length
        .filter(|n| *n > 0 && *n <= LIMIT)
        .ok_or("LSP_FRAME_LIMIT")?;
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .map_err(|_| "LSP_FRAME_READ_FAILED")?;
    serde_json::from_slice(&body).map_err(|_| "LSP_FRAME_JSON_INVALID".into())
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        // This private UUID directory is created by us before Scratch is built;
        // only our validated source copies are written inside it, never user assets.
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Session {
    child: Child,
    input: ChildStdin,
    rx: mpsc::Receiver<Result<Value, String>>,
    deadline: Instant,
    next_request_id: u64,
    diagnostics: Value,
    document_uri: String,
}

struct CachedDocument {
    version: i64,
    content_hash: String,
}

struct CachedSession {
    // Field order matters: terminate the sandbox before removing its snapshots.
    session: Session,
    _scratch: Scratch,
    graph_fingerprint: String,
    documents: HashMap<String, CachedDocument>,
    last_used: Instant,
}

fn session_cache() -> &'static Mutex<HashMap<String, Arc<Mutex<CachedSession>>>> {
    SESSION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(root: &Path, server: &Path, scope: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(root.as_os_str().as_encoded_bytes());
    digest.update([0]);
    digest.update(server.as_os_str().as_encoded_bytes());
    digest.update([0]);
    digest.update(scope.as_bytes());
    format!("{:x}", digest.finalize())
}

fn graph_fingerprint(snapshots: &[(String, String)], active_path: &str) -> String {
    let mut digest = Sha256::new();
    for (path, content) in snapshots {
        if path == active_path {
            continue;
        }
        digest.update((path.len() as u64).to_le_bytes());
        digest.update(path.as_bytes());
        digest.update(Sha256::digest(content.as_bytes()));
    }
    format!("{:x}", digest.finalize())
}

fn remove_cached_session(key: &str, target: &Arc<Mutex<CachedSession>>) {
    let mut cache = session_cache().lock().unwrap_or_else(|e| e.into_inner());
    if cache.get(key).is_some_and(|entry| Arc::ptr_eq(entry, target)) {
        cache.remove(key);
    }
}

fn insert_cached_session(
    key: String,
    session: CachedSession,
) -> Result<Arc<Mutex<CachedSession>>, String> {
    let mut cache = session_cache().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(existing) = cache.get(&key) {
        return Ok(existing.clone());
    }
    let now = Instant::now();
    cache.retain(|_, entry| {
        if Arc::strong_count(entry) > 1 {
            return true;
        }
        match entry.try_lock() {
            Ok(guard) => now.duration_since(guard.last_used) < SESSION_IDLE_TTL,
            Err(_) => true,
        }
    });
    if cache.len() >= MAX_CACHED_SESSIONS {
        let candidate = cache
            .iter()
            .filter(|(_, entry)| Arc::strong_count(entry) == 1)
            .filter_map(|(key, entry)| {
                entry.try_lock().ok().map(|guard| (key.clone(), guard.last_used))
            })
            .min_by_key(|(_, last_used)| *last_used)
            .map(|(key, _)| key);
        if let Some(candidate) = candidate {
            cache.remove(&candidate);
        }
    }
    if cache.len() >= MAX_CACHED_SESSIONS {
        return Err("LSP_SESSION_LIMIT".into());
    }
    let entry = Arc::new(Mutex::new(session));
    cache.insert(key, entry.clone());
    Ok(entry)
}

impl CachedSession {
    fn update_document(&mut self, path: &str, uri: &str, content: &str) -> Result<(), String> {
        let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        let previous = self.documents.get(path);
        let version = previous.map_or(1, |doc| doc.version.saturating_add(1));
        self.session.document_uri = uri.to_string();
        if previous.is_none() {
            self.session.renew_deadline();
            self.session.send(json!({
                "jsonrpc":"2.0","method":"textDocument/didOpen",
                "params":{"textDocument":{
                    "uri":uri,"languageId":"rust","version":version,"text":content
                }}
            }))?;
        } else if previous.is_some_and(|doc| doc.content_hash != content_hash) {
            self.session.renew_deadline();
            self.session.send(json!({
                "jsonrpc":"2.0","method":"textDocument/didChange",
                "params":{
                    "textDocument":{"uri":uri,"version":version},
                    "contentChanges":[{"text":content}]
                }
            }))?;
        }
        self.documents.insert(
            path.to_string(),
            CachedDocument { version, content_hash },
        );
        self.last_used = Instant::now();
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Session {
    fn renew_deadline(&mut self) {
        self.deadline = Instant::now() + Duration::from_secs(12);
    }

    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn rpc_next(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        self.rpc(id, method, params)
    }

    fn await_document(&mut self) -> Result<(), String> {
        loop {
            let value = self
                .rx
                .recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| "LSP_DOCUMENT_READY_TIMEOUT")??;
            if value["method"] == "textDocument/publishDiagnostics"
                && value["params"]["uri"] == self.document_uri
            {
                self.diagnostics = value["params"]["diagnostics"].clone();
                return Ok(());
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32601,"message":"Client operations are disabled"}}))?;
            }
        }
    }
    fn await_ready(&mut self) -> Result<(), String> {
        loop {
            let value = self
                .rx
                .recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| "LSP_READY_TIMEOUT")??;
            if value["method"] == "experimental/serverStatus"
                && value["params"]["quiescent"] == true
            {
                return if value["params"]["health"] == "error" {
                    Err("LSP_SERVER_NOT_READY".into())
                } else {
                    Ok(())
                };
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32601,"message":"Client operations are disabled"}}))?;
            }
        }
    }
    fn start(server: &Path, scratch: &Path) -> Result<Self, String> {
        let mut cmd = Command::new("/usr/bin/bwrap");
        cmd.args([
            "--unshare-all",
            "--unshare-user",
            "--disable-userns",
            "--die-with-parent",
            "--new-session",
            "--clearenv",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            "/work",
            "--chdir",
            "/work",
            "--setenv",
            "HOME",
            "/tmp",
            "--setenv",
            "CARGO_NET_OFFLINE",
            "true",
        ]);
        cmd.arg("--ro-bind").arg(server).arg(server);
        let mut dependencies = libraries(server)?;
        if server
            .file_name()
            .is_some_and(|name| name == "rust-analyzer")
        {
            let directory = server.parent().ok_or("LSP_TOOLCHAIN_INVALID")?;
            let compiler = directory
                .join("rustc")
                .canonicalize()
                .map_err(|_| "LSP_COMPILER_METADATA_UNAVAILABLE")?;
            if !compiler.starts_with(directory) {
                return Err("LSP_TOOLCHAIN_INVALID".into());
            }
            cmd.arg("--ro-bind")
                .arg(&compiler)
                .arg(&compiler)
                .arg("--setenv")
                .arg("PATH")
                .arg(directory);
            for (source, target) in libraries(&compiler)? {
                if !dependencies.iter().any(|(_, path)| path == &target) {
                    dependencies.push((source, target));
                }
            }
        }
        for (source, target) in dependencies {
            cmd.arg("--ro-bind").arg(source).arg(target);
        }
        cmd.arg("--ro-bind")
            .arg(scratch)
            .arg("/work")
            .arg("--")
            .arg(server);
        cmd.env_clear()
            .current_dir("/")
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // Applied only to the child, never the desktop process. A malformed
        // document cannot consume unbounded CPU, address space or output files.
        unsafe {
            cmd.pre_exec(|| {
                for (resource, value) in [
                    (libc::RLIMIT_CPU, 180),
                    (libc::RLIMIT_AS, 2 * 1024 * 1024 * 1024),
                    (libc::RLIMIT_FSIZE, 16 * 1024 * 1024),
                    (libc::RLIMIT_NOFILE, 256),
                ] {
                    let limit = libc::rlimit {
                        rlim_cur: value,
                        rlim_max: value,
                    };
                    if libc::setrlimit(resource, &limit) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        let mut child = cmd.spawn().map_err(|_| "LSP_SANDBOX_START_FAILED")?;
        let setup = (|| {
            let input = child.stdin.take().ok_or("LSP_PIPE_FAILED")?;
            let fd = input.as_raw_fd();
            if unsafe {
                libc::fcntl(
                    fd,
                    libc::F_SETFL,
                    libc::fcntl(fd, libc::F_GETFL) | libc::O_NONBLOCK,
                )
            } < 0
            {
                return Err("LSP_PIPE_FAILED".to_string());
            }
            let output = child.stdout.take().ok_or("LSP_PIPE_FAILED")?;
            let (tx, rx) = mpsc::sync_channel(32);
            thread::spawn(move || {
                let mut reader = BufReader::new(output);
                loop {
                    let value = frame(&mut reader);
                    let failed = value.is_err();
                    if tx.try_send(value).is_err() || failed {
                        break;
                    }
                }
            });
            Ok((input, rx))
        })();
        match setup {
            Ok((input, rx)) => Ok(Self {
                child,
                input,
                rx,
                deadline: Instant::now() + Duration::from_secs(12),
                next_request_id: 2,
                diagnostics: json!([]),
                document_uri: DOCUMENT.into(),
            }),
            Err(error) => {
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            }
        }
    }
    fn send(&mut self, value: Value) -> Result<(), String> {
        let body = serde_json::to_vec(&value).map_err(|_| "LSP_ENCODE_FAILED")?;
        if body.len() > LIMIT {
            return Err("LSP_FRAME_LIMIT".into());
        }
        let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        bytes.extend(body);
        let mut offset = 0;
        while offset < bytes.len() {
            let left = self.deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err("LSP_TIMEOUT".into());
            }
            let mut fd = libc::pollfd {
                fd: self.input.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            if unsafe { libc::poll(&mut fd, 1, left.as_millis().min(1000) as i32) } < 0 {
                return Err("LSP_PIPE_FAILED".into());
            }
            match self.input.write(&bytes[offset..]) {
                Ok(0) => return Err("LSP_PIPE_FAILED".into()),
                Ok(n) => offset += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return Err("LSP_PIPE_FAILED".into()),
            }
        }
        Ok(())
    }
    fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        self.renew_deadline();
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let value = self
                .rx
                .recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| "LSP_TIMEOUT_OR_CLOSED")??;
            if value.get("method").is_some() && value.get("id").is_some() {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32601,"message":"Client operations are disabled"}}))?;
            } else if value["id"] == json!(id) {
                if value.get("error").is_some() {
                    return Err("LSP_SERVER_ERROR".into());
                }
                return Ok(value["result"].clone());
            } else if value["method"] == "textDocument/publishDiagnostics"
                && value["params"]["uri"] == self.document_uri
            {
                self.diagnostics = value["params"]["diagnostics"].clone();
            }
        }
    }
}

pub fn execute(root: &Path, request: Value) -> Result<Value, String> {
    let available = profiles();
    if request["action"] == "lsp_status" {
        return Ok(
            json!({"profiles":available.iter().map(|path|json!({"path":path,"name":"Rust · isolated source document"})).collect::<Vec<_>>(),"sandbox_available":Path::new("/usr/bin/bwrap").is_file()}),
        );
    }
    if request["approved"] != true {
        return Err("LSP_APPROVAL_REQUIRED".into());
    }
    let _guard = REQUEST_LOCK.try_lock().map_err(|_| "LSP_BUSY")?;
    let method = match request["method"].as_str() {
        Some("completion") => "textDocument/completion",
        Some("definition") => "textDocument/definition",
        Some("hover") => "textDocument/hover",
        Some("diagnostics") => "textDocument/documentSymbol",
        _ => return Err("LSP_METHOD_DENIED".into()),
    };
    let server = PathBuf::from(
        request["server_path"]
            .as_str()
            .ok_or("LSP_SERVER_REQUIRED")?,
    )
    .canonicalize()
    .map_err(|_| "LSP_SERVER_UNAVAILABLE")?;
    if !available.contains(&server) {
        return Err("LSP_SERVER_NOT_TRUSTED_PROFILE".into());
    }
    let path = request["path"].as_str().ok_or("LSP_DOCUMENT_REQUIRED")?;
    let content = request["content"].as_str().ok_or("LSP_DOCUMENT_REQUIRED")?;
    super::coding::validate_lsp_document(root, path, content)?;
    let line = request["line"]
        .as_u64()
        .filter(|n| *n < 100000)
        .ok_or("LSP_POSITION_INVALID")?;
    let character = request["character"]
        .as_u64()
        .filter(|n| *n <= 262144)
        .ok_or("LSP_POSITION_INVALID")?;
    let text_line = content
        .split('\n')
        .nth(line as usize)
        .ok_or("LSP_POSITION_INVALID")?;
    if character as usize > text_line.encode_utf16().count() {
        return Err("LSP_POSITION_INVALID".into());
    }
    let scratch_path = std::env::temp_dir().join(format!("browseros-lsp-{}", uuid::Uuid::new_v4()));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&scratch_path)
        .map_err(|_| "LSP_SCRATCH_FAILED")?;
    let scratch = Scratch(scratch_path);
    let entry = request["module_root"].as_str();
    let (root_module, document_path, mut snapshots) = if let Some(entry) = entry {
        if request["approved_module_sources"] != true {
            return Err("LSP_MODULE_APPROVAL_REQUIRED".into());
        }
        super::coding::validate_lsp_document(root, entry, "")?;
        let directory = Path::new(entry).parent().ok_or("LSP_MODULE_ROOT_INVALID")?;
        if !Path::new(path).starts_with(directory) {
            return Err("LSP_DOCUMENT_OUTSIDE_MODULE_ROOT".into());
        }
        let mut files = super::coding::lsp_module_sources(root, entry)?;
        if let Some(file) = files.iter_mut().find(|file| file.0 == path) {
            file.1 = content.into();
        } else {
            files.push((path.into(), content.into()));
        }
        if !files.iter().any(|file| file.0 == entry) {
            return Err("LSP_MODULE_ROOT_MISSING".into());
        }
        if files.len() > 128
            || files.iter().map(|file| file.1.len()).sum::<usize>() > 4 * 1024 * 1024
        {
            return Err("LSP_MODULE_SIZE_LIMIT".into());
        }
        (entry.to_owned(), path.to_owned(), files)
    } else {
        (
            "document.rs".into(),
            "document.rs".into(),
            vec![("document.rs".into(), content.into())],
        )
    };
    snapshots.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, text) in &snapshots {
        let target = scratch.0.join(name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| "LSP_SCRATCH_FAILED")?;
        }
        fs::write(&target, text).map_err(|_| "LSP_SCRATCH_FAILED")?;
        fs::set_permissions(target, fs::Permissions::from_mode(0o400))
            .map_err(|_| "LSP_SCRATCH_FAILED")?;
    }
    let uri = |name: &str| {
        url::Url::from_file_path(Path::new("/work").join(name))
            .map(|uri| uri.to_string())
            .map_err(|_| "LSP_DOCUMENT_URI_INVALID")
    };
    let document_uri = uri(&document_path)?;
    let mut session = Session::start(&server, &scratch.0)?;
    session.document_uri = document_uri.clone();
    let config = json!({"linkedProjects":[{"crates":[{"root_module":format!("/work/{root_module}"),"edition":"2021","deps":[],"cfg":[],"env":{},"is_workspace_member":true,"display_name":"document","source":{"include_dirs":["/work"],"exclude_dirs":[]}}]}],"cargo":{"sysroot":null,"noDeps":true,"autoreload":true,"buildScripts":{"enable":false,"rebuildOnSave":false}},"procMacro":{"enable":false},"checkOnSave":false,"files":{"watcher":"server"},"cachePriming":{"enable":false},"numThreads":2});
    // Automatic graph reload is required when VFS finishes loading the root file.
    // This only reloads our inline graph; Cargo and project executables are absent.
    let init=session.rpc(1,"initialize",json!({"processId":null,"rootUri":"file:///work","capabilities":{"experimental":{"serverStatusNotification":true},"textDocument":{"completion":{"completionItem":{"snippetSupport":true}},"hover":{"contentFormat":["plaintext"]}}},"initializationOptions":config}))?;
    session.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))?;
    session.await_ready()?;
    session.send(json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":document_uri,"languageId":"rust","version":1,"text":content}}}))?;
    session.await_document()?;
    let result = session.rpc(
        2,
        method,
        json!({"textDocument":{"uri":document_uri},"position":{"line":line,"character":character}}),
    )?;
    let mut targets = vec![];
    if method == "textDocument/definition" {
        let items = if let Some(items) = result.as_array() {
            items.clone()
        } else {
            vec![result.clone()]
        };
        for item in items.into_iter().take(32) {
            let target_uri = item["uri"]
                .as_str()
                .or_else(|| item["targetUri"].as_str())
                .unwrap_or("");
            if let Some((name, text)) = snapshots
                .iter()
                .find(|file| uri(&file.0).is_ok_and(|uri| uri == target_uri))
            {
                let project_path = if entry.is_some() { name.as_str() } else { path };
                let range = item
                    .get("targetSelectionRange")
                    .or_else(|| item.get("range"))
                    .or_else(|| item.get("targetRange"));
                targets.push(json!({"uri":target_uri,"path":project_path,"range":range,"hash":format!("{:x}",Sha256::digest(text.as_bytes()))}));
            }
        }
    }
    Ok(
        json!({"result":result,"definition_targets":targets,"diagnostics":session.diagnostics,"document_hash":format!("{:x}",Sha256::digest(content.as_bytes())),"server":init["serverInfo"],"document_uri":document_uri,"files_analyzed":snapshots.len(),"scope":if entry.is_some(){"isolated_rust_module_sources"}else{"isolated_current_rust_document"}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    static REAL_LSP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    #[test]
    #[ignore = "explicit bubblewrap isolation integration"]
    fn sandbox_has_only_readonly_document_and_no_host_or_shell() {
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-boundary-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("document.rs"), "fn main() {}").unwrap();
        fs::set_permissions(root.join("document.rs"), fs::Permissions::from_mode(0o400)).unwrap();
        let mut session = Session::start(Path::new("/usr/bin/dash"), &root).unwrap();
        let script="if [ -r /work/document.rs ] && [ ! -w /work/document.rs ] && [ ! -r /etc/passwd ] && [ ! -e /bin/sh ] && [ ! -e /usr/bin/cargo ] && [ ! -e /work/build.rs ]; then printf 'Content-Length: 13\\r\\n\\r\\n{\"safe\":true}'; else printf 'Content-Length: 14\\r\\n\\r\\n{\"safe\":false}'; fi\n";
        session.input.write_all(script.as_bytes()).unwrap();
        let result = session
            .rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        drop(session);
        let _ = fs::remove_file(root.join("document.rs"));
        let _ = fs::remove_dir(root);
        assert_eq!(result, json!({"safe":true}));
    }
    #[test]
    fn frame_limits_and_headers() {
        let _guard = REAL_LSP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            frame(&mut &b"Content-Length: 2\r\n\r\n{}"[..]).unwrap(),
            json!({})
        );
        for bytes in [
            &b"Content-Length: 99999999\r\n\r\n"[..],
            &b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}"[..],
            &b"Content-Length: 2\r\n\r\nx"[..],
        ] {
            assert!(frame(&mut &*bytes).is_err());
        }
        assert_eq!(
            execute(Path::new("/tmp"), json!({"action":"lsp_request"})).unwrap_err(),
            "LSP_APPROVAL_REQUIRED"
        );
        assert_eq!(
            execute(
                Path::new("/tmp"),
                json!({"action":"lsp_request","approved":true,"method":"workspace/executeCommand"})
            )
            .unwrap_err(),
            "LSP_METHOD_DENIED"
        );
    }
    #[test]
    fn concurrent_requests_are_rejected_without_starting_a_server() {
        let _guard = REAL_LSP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _occupied = REQUEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let result = execute(
            Path::new("/tmp"),
            json!({"action":"lsp_request","approved":true,"method":"completion"}),
        );
        assert_eq!(result.unwrap_err(), "LSP_BUSY");
    }
    #[test]
    fn document_guards_reject_outside_secrets_and_symlinks() {
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-guards-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let validate = |path: &str, content: &str| {
            crate::capabilities::agent::coding::validate_lsp_document(&root, path, content)
        };
        assert!(validate("main.rs", "fn main(){}").is_ok());
        for path in [
            "../outside.rs",
            "/tmp/outside.rs",
            ".git/main.rs",
            ".env.rs",
            "main.ts",
        ] {
            assert!(validate(path, "fn main(){}").is_err(), "{path}");
        }
        assert!(validate("main.rs", &"x".repeat(262145)).is_err());
        assert!(validate("main.rs", "-----BEGIN PRIVATE KEY-----").is_err());
        std::os::unix::fs::symlink("/tmp", root.join("alias")).unwrap();
        assert!(validate("alias/main.rs", "fn main(){}").is_err());
        fs::remove_file(root.join("alias")).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn module_sources_are_bounded_and_reject_secret_contents() {
        let root = std::env::temp_dir().join(format!(
            "browseros-lsp-module-guards-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("main.rs"), "fn main(){}").unwrap();
        std::os::unix::fs::symlink("/tmp", root.join("outside")).unwrap();
        let collect = || crate::capabilities::agent::coding::lsp_module_sources(&root, "main.rs");
        assert_eq!(collect().unwrap().len(), 1);
        fs::write(root.join("value.rs"), "-----BEGIN PRIVATE KEY-----").unwrap();
        assert_eq!(collect().unwrap_err(), "AGENT_SECRET_CONTENT_DENIED");
        fs::remove_file(root.join("value.rs")).unwrap();
        for index in 0..128 {
            fs::write(root.join(format!("file{index}.rs")), "fn value(){}").unwrap();
        }
        assert_eq!(collect().unwrap_err(), "LSP_MODULE_FILE_LIMIT");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "explicit installed rust-analyzer sandbox integration"]
    fn isolated_rust_analyzer_definition() {
        let _guard = REAL_LSP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let server = profiles()
            .into_iter()
            .next()
            .expect("installed rust-analyzer");
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let result = execute(
            &root,
            json!({"action":"lsp_request","approved":true,"server_path":server,"method":"definition","path":"main.rs","content":"fn square(x:i32)->i32{x*x}\nfn main(){let answer=square(4);}","line":1,"character":22}),
        );
        let _ = fs::remove_dir(&root);
        let result = result.expect("real isolated LSP request");
        assert!(
            result["result"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "{result}"
        );
        assert_eq!(result["result"][0]["uri"], DOCUMENT);
    }
    #[test]
    #[ignore = "explicit installed rust-analyzer sandbox integration"]
    fn isolated_cross_file_rust_module_definition() {
        let _guard = REAL_LSP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let server = profiles()
            .into_iter()
            .next()
            .expect("installed rust-analyzer");
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-modules-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(
            root.join("src/main.rs"),
            "mod math;\nfn main(){let answer=math::square(4);}",
        )
        .unwrap();
        fs::write(root.join("src/math.rs"), "pub fn square(x:i32)->i32{x*x}").unwrap();
        fs::write(
            root.join("src/credentials.rs"),
            "-----BEGIN PRIVATE KEY-----",
        )
        .unwrap();
        fs::write(root.join("Cargo.toml"), "not parsed or executed").unwrap();
        let request = json!({"action":"lsp_request","approved":true,"approved_module_sources":true,"module_root":"src/main.rs","server_path":server,"method":"definition","path":"src/main.rs","content":"mod math;\nfn main(){let answer=math::square(4);}","line":1,"character":29});
        let mut denied = request.clone();
        denied["approved_module_sources"] = json!(false);
        assert_eq!(
            execute(&root, denied).unwrap_err(),
            "LSP_MODULE_APPROVAL_REQUIRED"
        );
        let result = execute(&root, request);
        fs::remove_dir_all(root).unwrap();
        let result = result.expect("real isolated module definition");
        assert_eq!(result["scope"], "isolated_rust_module_sources");
        assert_eq!(result["files_analyzed"], 2);
        assert_eq!(
            result["definition_targets"][0]["path"], "src/math.rs",
            "{result}"
        );
        assert_eq!(
            result["definition_targets"][0]["uri"],
            "file:///work/src/math.rs"
        );
        assert_eq!(
            result["definition_targets"][0]["hash"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
    }
    #[test]
    #[ignore = "explicit installed rust-analyzer and approval workflow integration"]
    fn diagnostic_approval_result_drives_reviewed_repair() {
        let _guard = REAL_LSP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-repair-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let storage = root.join("state");
        let content = "fn main(){let = ;}";
        fs::write(root.join("main.rs"), content).unwrap();
        let requested=super::super::coding::execute(&root,&storage,json!({"action":"chat","message":"check code","mode":"build"}),|_,_|Ok(json!({"role":"assistant","tool_calls":[{"id":"diagnostic-request","type":"function","function":{"name":"request_operation","arguments":"{\"kind\":\"code_diagnostics\",\"input\":{\"path\":\"main.rs\"}}"}}]}))).unwrap();
        assert_eq!(requested["status"], "approval_required");
        assert_eq!(fs::read_to_string(root.join("main.rs")).unwrap(), content);
        let server = profiles()
            .into_iter()
            .next()
            .expect("installed rust-analyzer");
        let diagnosed=execute(&root,json!({"action":"lsp_request","approved":true,"server_path":server,"method":"diagnostics","path":"main.rs","content":content,"line":0,"character":0})).unwrap();
        assert!(diagnosed["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        assert_eq!(diagnosed["document_hash"].as_str().unwrap().len(), 64);
        let recorded=json!({"path":"main.rs","diagnostics":diagnosed["diagnostics"],"document_hash":diagnosed["document_hash"],"compilation_verified":false}).to_string();
        let id = &requested["session_id"];
        let operation = &requested["operations"][0]["id"];
        super::super::coding::execute(&root,&storage,json!({"action":"resolve_operation","session_id":id,"operation_id":operation,"status":"succeeded","result":recorded}),|_,_|panic!("recording must not call a model")).unwrap();
        let mut rounds = 0;
        let repaired=super::super::coding::execute(&root,&storage,json!({"action":"chat","session_id":id,"message":"use reviewed diagnostics","mode":"build"}),|messages,_|{
            rounds+=1;
            assert!(messages.iter().any(|message|message["content"].as_str().is_some_and(|text|text.contains("untrusted data")&&text.contains("compilation_verified"))));
            Ok(if rounds==1{json!({"role":"assistant","tool_calls":[{"id":"repair-proposal","type":"function","function":{"name":"write_file","arguments":json!({"path":"main.rs","content":"fn main(){}","expected_hash":diagnosed["document_hash"]}).to_string()}}]})}else{json!({"role":"assistant","content":"Review the repair proposal."})})
        }).unwrap();
        assert_eq!(repaired["status"], "review");
        assert_eq!(repaired["changes"][0]["content"], "fn main(){}");
        assert_eq!(
            fs::read_to_string(root.join("main.rs")).unwrap(),
            content,
            "the model must not apply its own repair"
        );
        super::super::coding::execute(
            &root,
            &storage,
            json!({"action":"apply","session_id":id}),
            |_, _| panic!("applying a reviewed proposal must not call a model"),
        )
        .unwrap();
        let applied = fs::read_to_string(root.join("main.rs")).unwrap();
        assert_eq!(applied, "fn main(){}");
        let verified=execute(&root,json!({"action":"lsp_request","approved":true,"server_path":server,"method":"diagnostics","path":"main.rs","content":applied,"line":0,"character":0})).unwrap();
        assert!(
            verified["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .all(|item| item["severity"] != 1),
            "{verified}"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "explicit installed rust-analyzer sandbox integration"]
    fn isolated_rust_analyzer_completion_hover_and_diagnostics() {
        let _guard = REAL_LSP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let server = profiles()
            .into_iter()
            .next()
            .expect("installed rust-analyzer");
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-features-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let call = |method: &str, content: &str, line: u64, character: u64| {
            execute(&root,json!({"action":"lsp_request","approved":true,"server_path":server,"method":method,"path":"main.rs","content":content,"line":line,"character":character})).unwrap()
        };
        let completed = call(
            "completion",
            "fn square(x:i32)->i32{x*x}\nfn main(){squ}",
            1,
            13,
        );
        let items = completed["result"]
            .as_array()
            .or_else(|| completed["result"]["items"].as_array())
            .unwrap();
        assert!(
            items.iter().any(|item| item["label"]
                .as_str()
                .is_some_and(|name| name.starts_with("square"))),
            "{completed}"
        );
        let hover = call(
            "hover",
            "fn square(x:i32)->i32{x*x}\nfn main(){let answer=square(4);}",
            1,
            22,
        );
        assert!(!hover["result"]["contents"].is_null(), "{hover}");
        let diagnostics = call("diagnostics", "fn main(){let = ;}", 0, 0);
        assert!(
            diagnostics["diagnostics"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "{diagnostics}"
        );
        let _ = fs::remove_dir(root);
    }
}
