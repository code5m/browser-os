//! Windows Rust LSP in a no-network AppContainer with a read-only private snapshot.
#![cfg(windows)]

use super::windows_sandbox::{send_bounded, SandboxChild, SandboxProcess, SandboxProfile};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

const MAX_FRAME: usize = 1024 * 1024;
const MAX_SESSIONS: usize = 2;
const MAX_FILES: usize = 2000;
const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 1024 * 1024;
const REPARSE: u32 = 0x0000_0400;

#[derive(Default)]
struct SessionCache {
    ready: HashMap<String, Arc<Mutex<Session>>>,
    starting: usize,
    epoch: u64,
}
static SESSIONS: OnceLock<Mutex<SessionCache>> = OnceLock::new();
fn sessions() -> &'static Mutex<SessionCache> {
    SESSIONS.get_or_init(|| Mutex::new(SessionCache::default()))
}
/// Reserve capacity without holding the global cache mutex through the slow
/// rust-analyzer process spawn and initialize handshake.
struct StartReservation { epoch: u64 }
impl Drop for StartReservation {
    fn drop(&mut self) {
        let mut cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
        cache.starting = cache.starting.saturating_sub(1);
    }
}

fn no_reparse(path: &Path) -> Result<(), String> {
    use std::path::{Component, Prefix};
    if !path.is_absolute() || !matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    ) { return Err("LSP_ROOT_INVALID".into()); }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_)) || !current.has_root() { continue; }
        let metadata = fs::symlink_metadata(&current).map_err(|_| "LSP_PATH_INSPECTION_FAILED")?;
        if metadata.file_attributes() & REPARSE != 0 {
            return Err("LSP_REPARSE_DENIED".into());
        }
    }
    Ok(())
}
/// The executable in ~/.cargo/bin is normally a rustup proxy, not the
/// language server itself. Copying that proxy to an AppContainer makes it
/// look for toolchain executables in a deliberately private USERPROFILE and
/// fail before it can answer initialize. Resolve the INSTALLED toolchain
/// binary by reading rustup metadata; never execute rustup outside sandbox.
fn rustup_analyzer_binary(home: &Path) -> Result<PathBuf, String> {
    let rustup_home = std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from).unwrap_or_else(|| home.join(".rustup"));
    let settings = fs::read_to_string(rustup_home.join("settings.toml")).unwrap_or_default();
    let default = settings.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "default_toolchain")
            .then(|| value.trim().trim_matches('"').to_string())
    });
    let mut toolchain = std::env::var("RUSTUP_TOOLCHAIN").ok().or(default)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "stable-x86_64-pc-windows-msvc".into());
    if toolchain == "stable" || toolchain == "nightly" || toolchain == "beta" {
        toolchain.push_str("-x86_64-pc-windows-msvc");
    }
    if toolchain.len() > 110
        || !toolchain.bytes().all(|byte| byte.is_ascii_alphanumeric()
            || byte == b'-' || byte == b'_' || byte == b'.')
    {
        return Err("LSP_RUSTUP_TOOLCHAIN_INVALID".into());
    }
    let candidate = rustup_home.join("toolchains")
        .join(&toolchain).join("bin").join("rust-analyzer.exe");
    if !candidate.is_file() { return Err("LSP_RUST_ANALYZER_TOOLCHAIN_MISSING".into()); }
    no_reparse(&candidate)?;
    candidate.canonicalize().map_err(|_| "LSP_RUST_ANALYZER_TOOLCHAIN_MISSING".into())
}
fn runtime() -> Result<PathBuf, String> {
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from);
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths).filter(|path| path.is_absolute()) {
            let candidate = directory.join("rust-analyzer.exe");
            if !candidate.is_file() { continue; }
            let is_cargo_proxy = directory.file_name()
                .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case("bin"))
                && directory.parent().and_then(Path::file_name)
                    .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case(".cargo"));
            if is_cargo_proxy {
                let home = home.as_ref().ok_or("LSP_USERPROFILE_UNAVAILABLE")?;
                return rustup_analyzer_binary(home);
            }
            no_reparse(&candidate)?;
            return candidate.canonicalize().map_err(|_| "LSP_RUST_ANALYZER_NOT_FOUND".into());
        }
    }
    if let Some(home) = home {
        let proxy = home.join(".cargo/bin/rust-analyzer.exe");
        if proxy.is_file() { return rustup_analyzer_binary(&home); }
    }
    Err("LSP_RUST_ANALYZER_NOT_FOUND".into())
}

fn source_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
        "rs" | "toml" | "md" | "json"
    ) || path.file_name().and_then(|value| value.to_str()) == Some("Cargo.lock")
}
struct SnapshotBudget {
    files: usize,
    bytes: usize,
}
fn copy_snapshot_tree(
    source: &Path,
    root: &Path,
    destination: &Path,
    depth: usize,
    budget: &mut SnapshotBudget,
) -> Result<(), String> {
    if depth > 12 {
        return Err("LSP_SNAPSHOT_DEPTH_LIMIT".into());
    }
    for entry in fs::read_dir(source).map_err(|_| "LSP_SNAPSHOT_READ_FAILED")? {
        let entry = entry.map_err(|_| "LSP_SNAPSHOT_READ_FAILED")?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.')
            || ["node_modules", "target", "dist", "vendor"].contains(&name.as_str())
        {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(|_| "LSP_SNAPSHOT_READ_FAILED")?;
        if metadata.file_attributes() & REPARSE != 0 {
            return Err("LSP_SNAPSHOT_REPARSE_DENIED".into());
        }
        let relative = path.strip_prefix(root).map_err(|_| "LSP_SNAPSHOT_ESCAPE")?;
        let target = destination.join(relative);
        if metadata.is_dir() {
            fs::create_dir(&target).map_err(|_| "LSP_SNAPSHOT_WRITE_FAILED")?;
            copy_snapshot_tree(&path, root, destination, depth + 1, budget)?;
        } else if metadata.is_file() && source_file(&path) {
            if metadata.len() > MAX_FILE_BYTES as u64 {
                return Err("LSP_SNAPSHOT_FILE_LIMIT".into());
            }
            budget.files += 1;
            budget.bytes = budget
                .bytes
                .checked_add(metadata.len() as usize)
                .ok_or("LSP_SNAPSHOT_LIMIT")?;
            if budget.files > MAX_FILES || budget.bytes > MAX_TOTAL_BYTES {
                return Err("LSP_SNAPSHOT_LIMIT".into());
            }
            let bytes = crate::capabilities::workspace::safe_paths::read_regular(
                &path,
                &[root.to_path_buf()],
                MAX_FILE_BYTES,
            )
            .map_err(|_| "LSP_SNAPSHOT_READ_DENIED")?;
            fs::write(&target, bytes).map_err(|_| "LSP_SNAPSHOT_WRITE_FAILED")?;
        }
    }
    Ok(())
}
fn snapshot(root: &Path, profile: &SandboxProfile) -> Result<PathBuf, String> {
    no_reparse(root)?;
    let destination = profile.snapshot_dir()?;
    copy_snapshot_tree(
        root,
        root,
        &destination,
        0,
        &mut SnapshotBudget { files: 0, bytes: 0 },
    )?;
    profile.protect_snapshot(&destination)?;
    Ok(destination)
}
fn uri_for(path: &Path) -> Result<String, String> {
    url::Url::from_file_path(path)
        .map(|uri| uri.to_string())
        .map_err(|_| "LSP_URI_INVALID".into())
}
fn request_uri_to_snapshot(value: &mut Value, root: &Path, snapshot: &Path) -> Result<(), String> {
    match value {
        Value::String(text) if text.starts_with("file://") => {
            let uri = url::Url::parse(text).map_err(|_| "LSP_URI_INVALID")?;
            let path = uri.to_file_path().map_err(|_| "LSP_URI_INVALID")?;
            no_reparse(&path)?;
            let canonical = path.canonicalize().map_err(|_| "LSP_DOCUMENT_NOT_FOUND")?;
            let relative = canonical
                .strip_prefix(root)
                .map_err(|_| "LSP_DOCUMENT_OUTSIDE_WORKSPACE")?;
            *text = uri_for(&snapshot.join(relative))?;
        }
        Value::Array(items) => {
            for item in items {
                request_uri_to_snapshot(item, root, snapshot)?;
            }
        }
        Value::Object(items) => {
            for item in items.values_mut() {
                request_uri_to_snapshot(item, root, snapshot)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn response_uri_to_workspace(value: &mut Value, root: &Path, snapshot: &Path) {
    match value {
        Value::String(text) if text.starts_with("file://") => {
            if let Ok(uri) = url::Url::parse(text) {
                if let Ok(path) = uri.to_file_path() {
                    if let Ok(relative) = path.strip_prefix(snapshot) {
                        if let Ok(mapped) = uri_for(&root.join(relative)) {
                            *text = mapped;
                        }
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                response_uri_to_workspace(item, root, snapshot);
            }
        }
        Value::Object(items) => {
            for item in items.values_mut() {
                response_uri_to_workspace(item, root, snapshot);
            }
        }
        _ => {}
    }
}
fn frame(reader: &mut BufReader<std::fs::File>) -> Result<Value, String> {
    let mut length = None;
    let mut header_bytes = 0usize;
    loop {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take(8192)
            .read_until(b'\n', &mut line)
            .map_err(|_| "LSP_FRAME_READ")?;
        if count == 0 {
            return Err("LSP_CLOSED".into());
        }
        header_bytes += count;
        if header_bytes > 8192 {
            return Err("LSP_HEADER_LIMIT".into());
        }
        let line = String::from_utf8(line).map_err(|_| "LSP_HEADER_INVALID")?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "LSP_FRAME_INVALID")?,
            );
        }
    }
    let length = length
        .filter(|length| *length <= MAX_FRAME)
        .ok_or("LSP_FRAME_LIMIT")?;
    let mut bytes = vec![0u8; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| "LSP_FRAME_READ")?;
    serde_json::from_slice(&bytes).map_err(|_| "LSP_FRAME_INVALID".into())
}

struct Session {
    process: SandboxProcess,
    writer: mpsc::SyncSender<Vec<u8>>,
    write_result: mpsc::Receiver<Result<(), String>>,
    reader: mpsc::Receiver<Result<Value, String>>,
    reader_fault: Arc<Mutex<Option<String>>>,
    deadline: Instant,
    next_id: u64,
    root: PathBuf,
    snapshot: PathBuf,
    last_used: Instant,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.process.terminate();
    }
}
impl Session {
    fn start(root: &Path, seconds: u64) -> Result<Self, String> {
        let server = runtime()?;
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        let profile = SandboxProfile::create()?;
        let snapshot = snapshot(&root, &profile)?;
        let state = profile.state_dir()?;
        let disabled_cargo = state.join("disabled-cargo.exe");
        let disabled_rustc = state.join("disabled-rustc.exe");
        let environment = vec![
            (
                "CARGO".into(),
                disabled_cargo.to_string_lossy().into_owned(),
            ),
            (
                "RUSTC".into(),
                disabled_rustc.to_string_lossy().into_owned(),
            ),
            ("CARGO_NET_OFFLINE".into(), "true".into()),
        ];
        let child = profile.spawn(&server, &["--no-proc-macro".into()], &environment, &state)
            .map_err(|error| {
                #[cfg(test)]
                { return format!("LSP_SANDBOX_START_FAILED:{error}"); }
                #[cfg(not(test))]
                { let _ = error; "LSP_SANDBOX_START_FAILED".to_string() }
            })?;
        let SandboxChild { stdin, stdout, process } = child;
        let (writer, write_rx) = mpsc::sync_channel::<Vec<u8>>(4);
        let (write_tx, write_result) = mpsc::channel();
        thread::spawn(move || {
            let mut input = std::io::BufWriter::new(stdin);
            while let Ok(bytes) = write_rx.recv() {
                let result = input
                    .write_all(&bytes)
                    .and_then(|_| input.flush())
                    .map_err(|_| "LSP_WRITE_FAILED".to_string());
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
                let value = frame(&mut reader);
                let failed = value.is_err();
                if send_bounded(&reader_tx, value, Duration::from_secs(2)).is_err() {
                    *on_read_failure.lock().unwrap_or_else(|p| p.into_inner()) =
                        Some("LSP_OUTPUT_BACKPRESSURE".into());
                    break;
                }
                if failed { break; }
            }
        });
        let mut session = Self {
            process,
            writer,
            write_result,
            reader,
            reader_fault,
            deadline: Instant::now() + Duration::from_secs(seconds.clamp(5, 120)),
            next_id: 1,
            root,
            snapshot,
            last_used: Instant::now(),
        };
        let root_uri = uri_for(&session.snapshot)?;
        let init = session.rpc_next("initialize", json!({
            "processId":null,"rootUri":root_uri,
            "capabilities":{"textDocument":{"synchronization":{},"publishDiagnostics":{},"definition":{},"references":{},"hover":{},"completion":{},"documentSymbol":{}}},
            "initializationOptions":{"cargo":{"buildScripts":{"enable":false}},"procMacro":{"enable":false},"checkOnSave":{"enable":false}}
        }));
        #[cfg(test)]
        if let Err(ref error) = init {
            eprintln!("LSP_INITIALIZE_DIAG={error}; exit={:?}; stderr={:?}",
                session.process.exit_code(), session.process.profile().stderr_for_test());
        }
        init?;
        session.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))?;
        Ok(session)
    }
    fn reset_deadline(&mut self, seconds: u64) {
        self.deadline = Instant::now() + Duration::from_secs(seconds.clamp(5, 120));
        self.last_used = Instant::now();
    }
    fn recv_frame(&self, timeout_error: &'static str) -> Result<Value, String> {
        let fault = || self.reader_fault.lock()
            .unwrap_or_else(|p| p.into_inner()).clone();
        if let Some(error) = fault() { return Err(error); }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        match self.reader.recv_timeout(remaining) {
            Ok(value) => {
                if let Some(error) = fault() { return Err(error); }
                value
            }
            Err(mpsc::RecvTimeoutError::Timeout) =>
                Err(fault().unwrap_or_else(|| timeout_error.to_string())),
            Err(mpsc::RecvTimeoutError::Disconnected) =>
                Err(fault().unwrap_or_else(|| "LSP_CLOSED".into())),
        }
    }
    fn send(&mut self, value: Value) -> Result<(), String> {
        let body = serde_json::to_vec(&value).map_err(|_| "LSP_ENCODE_FAILED")?;
        if body.len() > MAX_FRAME {
            return Err("LSP_REQUEST_LIMIT".into());
        }
        let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        bytes.extend(body);
        let mut pending = bytes;
        loop {
            if let Some(error) = self.reader_fault.lock().unwrap_or_else(|p| p.into_inner()).clone() {
                return Err(error);
            }
            match self.writer.try_send(pending) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Disconnected(_)) => return Err("LSP_WRITE_FAILED".into()),
                Err(mpsc::TrySendError::Full(value)) => {
                    pending = value;
                    let remaining = self.deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() { return Err("LSP_WRITE_TIMEOUT".into()); }
                    thread::sleep(remaining.min(Duration::from_millis(5)));
                }
            }
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() { return Err("LSP_WRITE_TIMEOUT".into()); }
        match self.write_result.recv_timeout(remaining) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err("LSP_WRITE_TIMEOUT".into()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("LSP_WRITE_FAILED".into()),
        }
    }
    fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let value = self.recv_frame("LSP_TIMEOUT_OR_CLOSED")?;
            if value["id"] == json!(id) && value.get("method").is_none() {
                if value.get("error").is_some() {
                    return Err("LSP_REMOTE_ERROR".into());
                }
                return Ok(value.get("result").cloned().unwrap_or(Value::Null));
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32601,"message":"Client operations are disabled"}}))?;
            }
        }
    }
    fn rpc_next(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.rpc(id, method, params)
    }
    fn diagnostics(&mut self, path: &str) -> Result<Value, String> {
        let document = PathBuf::from(path);
        if document.is_absolute() || path.contains("..") {
            return Err("LSP_DOCUMENT_INVALID".into());
        }
        let host = self.root.join(&document);
        no_reparse(&host)?;
        let bytes = crate::capabilities::workspace::safe_paths::read_regular(
            &host,
            &[self.root.clone()],
            MAX_FILE_BYTES,
        )
        .map_err(|_| "LSP_DOCUMENT_READ_DENIED")?;
        let text = String::from_utf8(bytes).map_err(|_| "LSP_DOCUMENT_NOT_TEXT")?;
        let snapshot_document = self.snapshot.join(&document);
        let uri = uri_for(&snapshot_document)?;
        self.send(json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"rust","version":1,"text":text}}}))?;
        loop {
            let mut value = self.recv_frame("LSP_DIAGNOSTICS_TIMEOUT")?;
            if value["method"] == "textDocument/publishDiagnostics" && value["params"]["uri"] == uri
            {
                let mut diagnostics = value["params"]["diagnostics"].take();
                response_uri_to_workspace(&mut diagnostics, &self.root, &self.snapshot);
                return Ok(diagnostics);
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"error":{"code":-32601,"message":"Client operations are disabled"}}))?;
            }
        }
    }
}
fn allowed_method(method: &str) -> bool {
    [
        "textDocument/hover",
        "textDocument/definition",
        "textDocument/references",
        "textDocument/completion",
        "textDocument/documentSymbol",
        "textDocument/codeAction",
        "textDocument/formatting",
        "textDocument/semanticTokens/full",
        "completionItem/resolve",
        "textDocument/diagnostic",
    ]
    .contains(&method)
}
fn get_session(id: &str) -> Result<Arc<Mutex<Session>>, String> {
    sessions()
        .lock()
        .map_err(|_| "LSP_SESSION_CACHE")?
        .ready.get(id)
        .cloned()
        .ok_or_else(|| "LSP_SESSION_NOT_FOUND".into())
}
fn retire_expired() {
    let expired = {
        let mut cache = sessions()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let expired = cache
            .ready.iter()
            .filter_map(|(id, session)| {
                session
                    .try_lock()
                    .ok()
                    .filter(|session| session.last_used.elapsed() > Duration::from_secs(120))
                    .map(|_| id.clone())
            })
            .collect::<Vec<_>>();
        expired
            .iter()
            .filter_map(|id| cache.ready.remove(id))
            .collect::<Vec<_>>()
    };
    drop(expired);
}

pub fn execute(root: &Path, request: Value) -> Result<Value, String> {
    retire_expired();
    match request["action"].as_str().unwrap_or("") {
        "lsp_status" => {
            let sandbox_available = SandboxProfile::create().is_ok();
            Ok(
                json!({"profiles":[{"id":"rust_analyzer","language":"rust","available":runtime().is_ok(),"command":"rust-analyzer.exe"}],"sandbox_available":sandbox_available,"platform":"windows","isolation":"appcontainer-no-network-readonly-snapshot"}),
            )
        }
        "lsp_start" => {
            if request["profile"].as_str().unwrap_or("") != "rust_analyzer" {
                return Err("LSP_PROFILE_UNSUPPORTED".into());
            }
            let reservation = {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                if cache.ready.len() + cache.starting >= MAX_SESSIONS {
                    return Err("LSP_SESSION_LIMIT".into());
                }
                cache.starting += 1;
                StartReservation { epoch: cache.epoch }
            };
            let session = Session::start(root, request["timeout_seconds"].as_u64().unwrap_or(30))?;
            let id = format!("lsp-{}", uuid::Uuid::new_v4().simple());
            {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                if cache.epoch != reservation.epoch {
                    return Err("LSP_SESSION_CLOSED_DURING_START".into());
                }
                cache.ready.insert(id.clone(), Arc::new(Mutex::new(session)));
            }
            drop(reservation);
            Ok(
                json!({"session_id":id,"profile":"rust_analyzer","sandbox":"appcontainer-no-network-readonly-snapshot"}),
            )
        }
        "lsp_request" => {
            let id = request["session_id"]
                .as_str()
                .ok_or("LSP_SESSION_REQUIRED")?;
            let method = request["method"]
                .as_str()
                .filter(|method| allowed_method(method))
                .ok_or("LSP_METHOD_DISABLED")?;
            let session = get_session(id)?;
            let mut session = session.try_lock().map_err(|_| "LSP_SESSION_BUSY")?;
            session.reset_deadline(request["timeout_seconds"].as_u64().unwrap_or(30));
            let mut params = request.get("params").cloned().unwrap_or(json!({}));
            let root = session.root.clone();
            let snapshot = session.snapshot.clone();
            request_uri_to_snapshot(&mut params, &root, &snapshot)?;
            let mut result = session.rpc_next(method, params)?;
            response_uri_to_workspace(&mut result, &root, &snapshot);
            session.last_used = Instant::now();
            Ok(json!({"result":result}))
        }
        "lsp_diagnostics" => {
            let id = request["session_id"]
                .as_str()
                .ok_or("LSP_SESSION_REQUIRED")?;
            let path = request["path"].as_str().ok_or("LSP_DOCUMENT_REQUIRED")?;
            let session = get_session(id)?;
            let mut session = session.try_lock().map_err(|_| "LSP_SESSION_BUSY")?;
            session.reset_deadline(request["timeout_seconds"].as_u64().unwrap_or(30));
            let diagnostics = session.diagnostics(path)?;
            session.last_used = Instant::now();
            Ok(json!({"diagnostics":diagnostics}))
        }
        "lsp_close" => {
            // Drop child processes after releasing the global cache mutex;
            // terminating a Job Object must not freeze other LSP operations.
            let removed = {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                if let Some(id) = request["session_id"].as_str() {
                    cache.ready.remove(id).into_iter().collect::<Vec<_>>()
                } else {
                    cache.epoch = cache.epoch.wrapping_add(1);
                    std::mem::take(&mut cache.ready).into_values().collect::<Vec<_>>()
                }
            };
            let count = removed.len();
            drop(removed);
            Ok(json!({"closed_sessions":count}))
        }
        _ => Err("LSP_ACTION_UNKNOWN".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_rejects_junctions_instead_of_copying_external_sources() {
        let base =
            std::env::temp_dir().join(format!("browseros-lsp-snapshot-{}", uuid::Uuid::new_v4()));
        let root = base.join("workspace");
        let outside = base.join("outside");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        fs::write(outside.join("secret.rs"), "fn must_not_copy() {}\n").unwrap();
        // Use PowerShell's Junction API. cmd.exe /S /C quote stripping
        // corrupts nested quoted absolute paths on CI Windows runners.
        let junction_path = root.join("src/link");
        let link_name = junction_path.to_string_lossy().replace('\'', "''");
        let target_name = outside.to_string_lossy().replace('\'', "''");
        let command = format!(
            "New-Item -ItemType Junction -Path '{link_name}' -Target '{target_name}' -ErrorAction Stop | Out-Null"
        );
        let junction = std::process::Command::new("powershell.exe")
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &command])
            .output()
            .unwrap();
        assert!(
            junction.status.success(),
            "junction fixture creation failed: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&junction.stdout),
            String::from_utf8_lossy(&junction.stderr),
        );
        let profile = SandboxProfile::create().unwrap();
        assert_eq!(
            snapshot(&root, &profile).unwrap_err(),
            "LSP_SNAPSHOT_REPARSE_DENIED"
        );
        drop(profile);
        fs::remove_dir(root.join("src/link")).unwrap();
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn real_rust_analyzer_runs_against_private_snapshot() {
        if runtime().is_err() {
            panic!("rust-analyzer.exe is required for Windows LSP acceptance");
        }
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-real-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='lsp_fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn fixture() -> i32 { 42 }\n").unwrap();
        let started = execute(
            &root,
            json!({"action":"lsp_start","profile":"rust_analyzer","timeout_seconds":120}),
        )
        .unwrap();
        let id = started["session_id"].as_str().unwrap();
        let diagnostics = execute(
            &root,
            json!({"action":"lsp_diagnostics","session_id":id,"path":"src/lib.rs"}),
        )
        .unwrap();
        assert!(diagnostics["diagnostics"].is_array());
        let uri = uri_for(&root.join("src/lib.rs")).unwrap();
        let symbols = execute(&root, json!({"action":"lsp_request","session_id":id,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}}})).unwrap();
        assert!(
            symbols["result"].to_string().contains("fixture"),
            "{symbols}"
        );
        assert_eq!(
            execute(&root, json!({"action":"lsp_close","session_id":id})).unwrap()["closed_sessions"],
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_execution_methods_are_not_lsp_methods() {
        assert!(!allowed_method("workspace/executeCommand"));
        assert!(!allowed_method("rust-analyzer/runFlycheck"));
        assert!(allowed_method("textDocument/documentSymbol"));
    }
}
