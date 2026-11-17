//! Windows Rust LSP in a no-network AppContainer with a read-only private snapshot.
#![cfg(windows)]

use super::windows_sandbox::{SandboxChild, SandboxProcess, SandboxProfile, send_bounded};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
    /// Each in-flight start belongs to exactly one workspace. A close for
    /// workspace A must not invalidate an unrelated startup in workspace B.
    starting: HashMap<uuid::Uuid, PathBuf>,
    one_shot: usize,
}
impl SessionCache {
    fn reserve_start(&mut self, root: &Path) -> Result<uuid::Uuid, String> {
        if self.ready.len() + self.starting.len() + self.one_shot >= MAX_SESSIONS {
            return Err("LSP_SESSION_LIMIT".into());
        }
        let token = uuid::Uuid::new_v4();
        self.starting.insert(token, root.to_path_buf());
        Ok(token)
    }

    fn start_is_active(&self, token: &uuid::Uuid, root: &Path) -> bool {
        self.starting
            .get(token)
            .is_some_and(|reserved| reserved == root)
    }

    fn cancel_pending_for(&mut self, root: &Path) {
        self.starting.retain(|_, reserved| reserved != root);
    }
}
static SESSIONS: OnceLock<Mutex<SessionCache>> = OnceLock::new();
fn sessions() -> &'static Mutex<SessionCache> {
    SESSIONS.get_or_init(|| Mutex::new(SessionCache::default()))
}
/// Reserve capacity without holding the global cache mutex through the slow
/// rust-analyzer process spawn and initialize handshake.
struct StartReservation {
    token: uuid::Uuid,
}
impl Drop for StartReservation {
    fn drop(&mut self) {
        let mut cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
        cache.starting.remove(&self.token);
    }
}

/// A one-shot legacy request uses the same global process budget as
/// persistent Windows sessions, preventing unbounded concurrent analyzers.
struct OneShotPermit;
impl OneShotPermit {
    fn acquire() -> Result<Self, String> {
        let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
        if cache.ready.len() + cache.starting.len() + cache.one_shot >= MAX_SESSIONS {
            return Err("LSP_SESSION_LIMIT".into());
        }
        cache.one_shot += 1;
        Ok(Self)
    }
}
impl Drop for OneShotPermit {
    fn drop(&mut self) {
        let mut cache = sessions().lock().unwrap_or_else(|e| e.into_inner());
        cache.one_shot = cache.one_shot.saturating_sub(1);
    }
}

fn no_reparse(path: &Path) -> Result<(), String> {
    use std::path::{Component, Prefix};
    if !path.is_absolute()
        || !matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
    {
        return Err("LSP_ROOT_INVALID".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_)) || !current.has_root() {
            continue;
        }
        let metadata = fs::symlink_metadata(&current).map_err(|_| "LSP_PATH_INSPECTION_FAILED")?;
        if metadata.file_attributes() & REPARSE != 0 {
            return Err("LSP_REPARSE_DENIED".into());
        }
    }
    Ok(())
}
/// A rustup shim is the small `cargo`/`rustc`/`rust-analyzer` proxy that lives
/// wherever CARGO_HOME points. Copying that shim into an AppContainer makes
/// every toolchain call answer "no default is configured", because the shim
/// resolves its toolchain through a deliberately private USERPROFILE. Resolve
/// the INSTALLED toolchain images from rustup metadata; never run rustup.
fn rustup_home() -> Option<PathBuf> {
    std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|home| home.join(".rustup"))
        })
        .filter(|path| path.join("settings.toml").is_file())
}
fn rustup_analyzer_binary(rustup_home: &Path) -> Result<PathBuf, String> {
    let settings = fs::read_to_string(rustup_home.join("settings.toml")).unwrap_or_default();
    let default = settings.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "default_toolchain").then(|| value.trim().trim_matches('"').to_string())
    });
    let mut toolchain = std::env::var("RUSTUP_TOOLCHAIN")
        .ok()
        .or(default)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "stable-x86_64-pc-windows-msvc".into());
    if toolchain == "stable" || toolchain == "nightly" || toolchain == "beta" {
        toolchain.push_str("-x86_64-pc-windows-msvc");
    }
    if toolchain.len() > 110
        || !toolchain.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
        })
    {
        return Err("LSP_RUSTUP_TOOLCHAIN_INVALID".into());
    }
    let candidate = rustup_home
        .join("toolchains")
        .join(&toolchain)
        .join("bin")
        .join("rust-analyzer.exe");
    if !candidate.is_file() {
        return Err("LSP_RUST_ANALYZER_TOOLCHAIN_MISSING".into());
    }
    no_reparse(&candidate)?;
    candidate
        .canonicalize()
        .map_err(|_| "LSP_RUST_ANALYZER_TOOLCHAIN_MISSING".into())
}
fn runtime() -> Result<PathBuf, String> {
    // Prefer the installed toolchain image. Staging a rustup shim, or staging
    // the shim's cargo/rustc peers, cannot work inside the sandbox.
    if let Some(home) = rustup_home() {
        if let Ok(analyzer) = rustup_analyzer_binary(&home) {
            return Ok(analyzer);
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths).filter(|path| path.is_absolute()) {
            let candidate = directory.join("rust-analyzer.exe");
            if !candidate.is_file() {
                continue;
            }
            // A shim directory also ships rustup.exe; a real toolchain bin
            // directory does not.
            if directory.join("rustup.exe").is_file() {
                continue;
            }
            no_reparse(&candidate)?;
            return candidate
                .canonicalize()
                .map_err(|_| "LSP_RUST_ANALYZER_NOT_FOUND".into());
        }
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
    grant_snapshot_read(&destination, profile.sid_string())?;
    Ok(destination)
}
/// `protect_snapshot` stores the lowbox grant as an inheritable
/// `(OI)(CI)RX` entry on the snapshot directory. On Windows that does not
/// reliably materialise as an explicit read right on files that already
/// existed when the ACL was applied, so a sandboxed `cargo` reading the
/// manifest failed with `Access is denied (os error 5)`. Pin `RX` directly
/// onto every file, and grant traverse on the ancestors outside the snapshot.
/// The grant is additive, so the owner and SYSTEM entries stay untouched.
fn grant_snapshot_read(root: &Path, sid: &str) -> Result<(), String> {
    let icacls = system_icacls()?;
    // The `AC` container and the package profile directory are ancestors of
    // the snapshot, so the recursive walk below can never reach them. Without
    // a traverse right there, an open inside the snapshot fails with
    // "Access is denied (os error 5)" before the file's own ACE is consulted.
    let mut ancestors = Vec::new();
    if let Some(container) = root.parent() {
        ancestors.push(container.to_path_buf());
        if let Some(package) = container.parent() {
            ancestors.push(package.to_path_buf());
        }
    }
    for directory in ancestors {
        grant_entry(&icacls, &directory, &format!("*{sid}:RX"))?;
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(|_| "LSP_SNAPSHOT_ACL")? {
            let path = entry.map_err(|_| "LSP_SNAPSHOT_ACL")?.path();
            let rights = if path.is_dir() {
                format!("*{sid}:(OI)(CI)RX")
            } else {
                format!("*{sid}:RX")
            };
            grant_entry(&icacls, &path, &rights)?;
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok(())
}
fn system_icacls() -> Result<PathBuf, String> {
    let system_root = std::env::var_os("SystemRoot").ok_or("LSP_SNAPSHOT_ACL")?;
    let icacls = PathBuf::from(system_root)
        .join("System32")
        .join("icacls.exe");
    if !icacls.is_file() {
        return Err("LSP_SNAPSHOT_ACL".into());
    }
    Ok(icacls)
}
fn grant_entry(icacls: &Path, path: &Path, rights: &str) -> Result<(), String> {
    let output = std::process::Command::new(icacls)
        .arg(path)
        .arg("/grant")
        .arg(rights)
        .output()
        .map_err(|_| "LSP_SNAPSHOT_ACL")?;
    if !output.status.success() {
        return Err("LSP_SNAPSHOT_ACL".into());
    }
    Ok(())
}
fn uri_for(path: &Path) -> Result<String, String> {
    url::Url::from_file_path(path)
        .map(|uri| uri.to_string())
        .map_err(|_| "LSP_URI_INVALID".into())
}
fn request_uri_to_snapshot(value: &mut Value, root: &Path, snapshot: &Path) -> Result<(), String> {
    match value {
        Value::String(text)
            if text
                .get(..7)
                .is_some_and(|part| part.eq_ignore_ascii_case("file://")) =>
        {
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
        Value::String(text)
            if text
                .get(..7)
                .is_some_and(|part| part.eq_ignore_ascii_case("file://")) =>
        {
            let mapped = url::Url::parse(text)
                .ok()
                .and_then(|uri| uri.to_file_path().ok())
                .and_then(|path| path.strip_prefix(snapshot).ok().map(Path::to_path_buf))
                .filter(|relative| {
                    relative
                        .components()
                        .all(|part| matches!(part, std::path::Component::Normal(_)))
                })
                .and_then(|relative| uri_for(&root.join(relative)).ok());
            *text = mapped.unwrap_or_else(|| "[outside reviewed snapshot]".into());
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
    closed: bool,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.process.terminate();
    }
}
impl Session {
    fn start(root: &Path, seconds: u64) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        let profile = SandboxProfile::create()?;
        let snapshot = snapshot(&root, &profile)?;
        Self::start_with_snapshot(root, profile, snapshot, seconds)
    }

    fn start_documents(
        root: &Path,
        documents: &[(String, String)],
        seconds: u64,
    ) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        if documents.is_empty() || documents.len() > 128 {
            return Err("LSP_DOCUMENT_LIMIT".into());
        }
        let profile = SandboxProfile::create()?;
        let snapshot = profile.snapshot_dir()?;
        let mut total = 0usize;
        for (relative, content) in documents {
            let path = Path::new(relative);
            if path.is_absolute()
                || !path
                    .components()
                    .all(|part| matches!(part, std::path::Component::Normal(_)))
                || !relative.ends_with(".rs")
                || content.len() > MAX_FILE_BYTES
            {
                return Err("LSP_DOCUMENT_DENIED".into());
            }
            total = total
                .checked_add(content.len())
                .ok_or("LSP_DOCUMENT_LIMIT")?;
            if total > 4 * 1024 * 1024 {
                return Err("LSP_DOCUMENT_LIMIT".into());
            }
            let target = snapshot.join(path);
            fs::create_dir_all(target.parent().ok_or("LSP_DOCUMENT_DENIED")?)
                .map_err(|_| "LSP_SNAPSHOT_WRITE_FAILED")?;
            fs::write(&target, content).map_err(|_| "LSP_SNAPSHOT_WRITE_FAILED")?;
        }
        profile.protect_snapshot(&snapshot)?;
        grant_snapshot_read(&snapshot, profile.sid_string())?;
        Self::start_with_snapshot(root, profile, snapshot, seconds)
    }

    fn start_with_snapshot(
        root: PathBuf,
        profile: SandboxProfile,
        snapshot: PathBuf,
        seconds: u64,
    ) -> Result<Self, String> {
        let server = runtime()?;
        let state = profile.state_dir()?;
        // Rust Analyzer loads the workspace through `cargo locate-project`,
        // `cargo metadata` and `rustc --version`. Those are non-building
        // probes, so the reviewed toolchain images are staged into the private
        // runtime instead of being replaced by nonexistent placeholder paths.
        // Build scripts, proc macros and check-on-save stay disabled below.
        let toolchain_bin = server
            .parent()
            .ok_or("LSP_TOOLCHAIN_BIN_UNAVAILABLE")?
            .to_path_buf();
        let cargo = profile.stage_runtime_peer(&toolchain_bin.join("cargo.exe"))?;
        let rustc = profile.stage_runtime_peer(&toolchain_bin.join("rustc.exe"))?;
        let cargo_home = state.join("cargo-home");
        fs::create_dir(&cargo_home).map_err(|_| "LSP_CARGO_HOME_UNAVAILABLE")?;
        let environment = vec![
            ("CARGO".into(), cargo.to_string_lossy().into_owned()),
            ("RUSTC".into(), rustc.to_string_lossy().into_owned()),
            (
                "CARGO_HOME".into(),
                cargo_home.to_string_lossy().into_owned(),
            ),
            ("CARGO_NET_OFFLINE".into(), "true".into()),
        ];
        // Current rust-analyzer rejects the retired --no-proc-macro flag
        // ("unexpected flag"). Do not pass it: staging copies only the
        // toolchain images, so proc-macro-srv is absent, and build scripts,
        // proc macros and check-on-save stay disabled through initialization
        // options, which keeps compilation from running inside the sandbox.
        let child = profile
            .spawn(&server, &[], &environment, &state)
            .map_err(|error| {
                #[cfg(test)]
                {
                    return format!("LSP_SANDBOX_START_FAILED:{error}");
                }
                #[cfg(not(test))]
                {
                    let _ = error;
                    "LSP_SANDBOX_START_FAILED".to_string()
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
                if failed {
                    break;
                }
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
            closed: false,
        };
        let root_uri = uri_for(&session.snapshot)?;
        let init = session.rpc_next("initialize", json!({
            "processId":null,"rootUri":root_uri,
            "capabilities":{"textDocument":{"synchronization":{},"publishDiagnostics":{},"definition":{},"references":{},"hover":{},"completion":{},"documentSymbol":{}}},
            "initializationOptions":{"cargo":{"buildScripts":{"enable":false}},"procMacro":{"enable":false},"checkOnSave":{"enable":false}}
        }));
        #[cfg(test)]
        if let Err(ref error) = init {
            eprintln!(
                "LSP_INITIALIZE_DIAG={error}; exit={:?}; stderr={:?}",
                session.process.exit_code(),
                session.process.profile().stderr_for_test()
            );
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
                Err(fault().unwrap_or_else(|| timeout_error.to_string()))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(fault().unwrap_or_else(|| "LSP_CLOSED".into()))
            }
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
            if let Some(error) = self
                .reader_fault
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clone()
            {
                return Err(error);
            }
            match self.writer.try_send(pending) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Disconnected(_)) => return Err("LSP_WRITE_FAILED".into()),
                Err(mpsc::TrySendError::Full(value)) => {
                    pending = value;
                    let remaining = self.deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Err("LSP_WRITE_TIMEOUT".into());
                    }
                    thread::sleep(remaining.min(Duration::from_millis(5)));
                }
            }
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("LSP_WRITE_TIMEOUT".into());
        }
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
    fn open_document(&mut self, relative: &Path, text: &str) -> Result<String, String> {
        let uri = uri_for(&self.snapshot.join(relative))?;
        self.send(
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
                "textDocument":{"uri":uri,"languageId":"rust","version":1,"text":text}
            }}),
        )?;
        Ok(uri)
    }

    fn await_diagnostics(&mut self, uri: &str) -> Result<Value, String> {
        loop {
            let mut value = self.recv_frame("LSP_DIAGNOSTICS_TIMEOUT")?;
            if value["method"] == "textDocument/publishDiagnostics" && value["params"]["uri"] == uri
            {
                return Ok(value["params"]["diagnostics"].take());
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"error":{
                    "code":-32601,"message":"Client operations are disabled"
                }}))?;
            }
        }
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
        let uri = self.open_document(&document, &text)?;
        let mut diagnostics = self.await_diagnostics(&uri)?;
        response_uri_to_workspace(&mut diagnostics, &self.root, &self.snapshot);
        Ok(diagnostics)
    }

    /// Test builds append the sandboxed analyzer's exit code and captured
    /// stderr, so a native acceptance failure names its real cause instead of
    /// only a panic location. Production errors stay opaque.
    fn describe_error(&self, error: String) -> String {
        #[cfg(test)]
        {
            format!(
                "{error} exit={:?} stderr={:?}",
                self.process.exit_code(),
                self.process.profile().stderr_for_test()
            )
        }
        #[cfg(not(test))]
        {
            error
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
/// The Linux API sends a single approved Rust document per call. Keep
/// Windows persistent session support, but accept this document request
/// contract by creating only the reviewed document(s) in a short-lived
/// AppContainer snapshot. No entire-workspace copy without module approval.
fn remap_reviewed_uris(
    value: &mut Value,
    snapshot: &Path,
    workspace: &Path,
    single_public_uri: Option<&str>,
) {
    match value {
        Value::String(text)
            if text
                .get(..7)
                .is_some_and(|part| part.eq_ignore_ascii_case("file://")) =>
        {
            let mapped = url::Url::parse(text)
                .ok()
                .and_then(|uri| uri.to_file_path().ok())
                .and_then(|path| path.strip_prefix(snapshot).ok().map(Path::to_path_buf))
                .and_then(|relative| {
                    if !relative
                        .components()
                        .all(|part| matches!(part, std::path::Component::Normal(_)))
                    {
                        return None;
                    }
                    if let Some(public) = single_public_uri {
                        (relative == Path::new("document.rs")).then(|| public.to_owned())
                    } else {
                        uri_for(&workspace.join(relative)).ok()
                    }
                });
            *text = mapped.unwrap_or_else(|| "[outside reviewed snapshot]".into());
        }
        Value::Array(values) => {
            for item in values {
                remap_reviewed_uris(item, snapshot, workspace, single_public_uri);
            }
        }
        Value::Object(values) => {
            for item in values.values_mut() {
                remap_reviewed_uris(item, snapshot, workspace, single_public_uri);
            }
        }
        _ => {}
    }
}

fn one_shot_document_request(root: &Path, request: &Value) -> Result<Value, String> {
    let method = match request["method"].as_str() {
        Some("completion") => "textDocument/completion",
        Some("definition") => "textDocument/definition",
        Some("hover") => "textDocument/hover",
        Some("diagnostics") => "textDocument/documentSymbol",
        _ => return Err("LSP_METHOD_DENIED".into()),
    };
    let path = request["path"].as_str().ok_or("LSP_DOCUMENT_REQUIRED")?;
    let content = request["content"].as_str().ok_or("LSP_DOCUMENT_REQUIRED")?;
    super::coding::validate_lsp_document(root, path, content)?;
    if content.len() > MAX_FILE_BYTES {
        return Err("LSP_DOCUMENT_LIMIT".into());
    }
    let symbol_only = method == "textDocument/documentSymbol";
    let line = request["line"]
        .as_u64()
        .or_else(|| symbol_only.then_some(0))
        .filter(|n| *n < 100_000)
        .ok_or("LSP_POSITION_INVALID")?;
    let character = request["character"]
        .as_u64()
        .or_else(|| symbol_only.then_some(0))
        .filter(|n| *n <= 262_144)
        .ok_or("LSP_POSITION_INVALID")?;
    let text_line = content
        .split('\n')
        .nth(line as usize)
        .ok_or("LSP_POSITION_INVALID")?;
    if character as usize > text_line.encode_utf16().count() {
        return Err("LSP_POSITION_INVALID".into());
    }
    if let Some(selected) = request["server_path"].as_str() {
        let real = runtime()?;
        let requested = Path::new(selected)
            .canonicalize()
            .map_err(|_| "LSP_SERVER_UNAVAILABLE")?;
        if requested != real {
            return Err("LSP_SERVER_NOT_TRUSTED_PROFILE".into());
        }
    }
    let _permit = OneShotPermit::acquire()?;
    let timeout = request["timeout_seconds"].as_u64().unwrap_or(30);
    let (documents, relative, scope, isolated_document) =
        if let Some(module) = request["module_root"].as_str() {
            if request["approved_module_sources"] != true {
                return Err("LSP_MODULE_APPROVAL_REQUIRED".into());
            }
            super::coding::validate_lsp_document(root, module, "")?;
            let directory = Path::new(module)
                .parent()
                .ok_or("LSP_MODULE_ROOT_INVALID")?;
            if !Path::new(path).starts_with(directory) {
                return Err("LSP_DOCUMENT_OUTSIDE_MODULE_ROOT".into());
            }
            let mut docs = super::coding::lsp_module_sources(root, module)?;
            if let Some(file) = docs.iter_mut().find(|file| file.0 == path) {
                file.1 = content.to_owned();
            } else {
                docs.push((path.to_owned(), content.to_owned()));
            }
            if !docs.iter().any(|file| file.0 == module) {
                return Err("LSP_MODULE_ROOT_MISSING".into());
            }
            (docs, path.to_owned(), "isolated_rust_module_sources", false)
        } else {
            (
                vec![("document.rs".into(), content.to_owned())],
                "document.rs".into(),
                "isolated_current_rust_document",
                true,
            )
        };
    let mut session = Session::start_documents(root, &documents, timeout)?;
    let private_uri = session.open_document(Path::new(&relative), content)?;
    let public_uri = uri_for(&root.join(path))?;
    let mut diagnostics = if request["method"] == "diagnostics" {
        session.await_diagnostics(&private_uri)?
    } else {
        Value::Null
    };
    let params = if method == "textDocument/documentSymbol" {
        json!({"textDocument":{"uri":private_uri}})
    } else {
        json!({"textDocument":{"uri":private_uri},
            "position":{"line":line,"character":character}})
    };
    let mut result = session.rpc_next(method, params)?;
    let public = isolated_document.then_some(public_uri.as_str());
    remap_reviewed_uris(&mut result, &session.snapshot, root, public);
    remap_reviewed_uris(&mut diagnostics, &session.snapshot, root, public);
    Ok(json!({
        "result":result,
        "diagnostics":diagnostics,
        "document_uri":public_uri,
        "document_hash":format!("{:x}", Sha256::digest(content.as_bytes())),
        "scope":scope,
        "compilation_verified":false,
        "sandbox":"appcontainer-no-network-readonly-snapshot"
    }))
}

fn get_session(root: &Path, id: &str) -> Result<Arc<Mutex<Session>>, String> {
    let entry = sessions()
        .lock()
        .map_err(|_| "LSP_SESSION_CACHE")?
        .ready
        .get(id)
        .cloned()
        .ok_or_else(|| "LSP_SESSION_NOT_FOUND".to_string())?;
    // A session ID is not authorization: root must match the workspace
    // currently approved by the caller. Fail closed if the session is busy.
    let owner = entry.try_lock().map_err(|_| "LSP_SESSION_BUSY")?;
    if owner.root != root {
        return Err("LSP_SESSION_WORKSPACE_MISMATCH".into());
    }
    if owner.closed {
        return Err("LSP_SESSION_NOT_FOUND".into());
    }
    drop(owner);
    Ok(entry)
}
fn retire_expired() {
    let expired = {
        let mut cache = sessions()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let ids = cache
            .ready
            .iter()
            .filter_map(|(id, entry)| {
                let Ok(mut session) = entry.try_lock() else {
                    return None;
                };
                if session.last_used.elapsed() <= Duration::from_secs(120) {
                    return None;
                }
                session.closed = true;
                Some(id.clone())
            })
            .collect::<Vec<_>>();
        ids.iter()
            .filter_map(|id| cache.ready.remove(id))
            .collect::<Vec<_>>()
    };
    drop(expired);
}

pub fn execute(root: &Path, request: Value) -> Result<Value, String> {
    retire_expired();
    let action = request["action"].as_str().unwrap_or("");
    // Capability discovery is a global read-only operation; no workspace
    // has to be selected yet. Other operations require a real pinned root.
    let root = if action == "lsp_status" {
        PathBuf::new()
    } else {
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        no_reparse(&root)?;
        if !root.is_dir() {
            return Err("LSP_ROOT_INVALID".into());
        }
        root
    };
    if matches!(action, "lsp_start" | "lsp_request" | "lsp_diagnostics")
        && request["approved"] != true
    {
        return Err("LSP_APPROVAL_REQUIRED".into());
    }
    match action {
        "lsp_status" => {
            // Status must be a non-mutating capability query. Previously it
            // created/deleted a Windows AppContainer OS profile on EVERY call.
            // Report prerequisites separately from native launch verification.
            let sandbox_available = super::windows_sandbox::sandbox_prerequisites_available();
            Ok(json!({
                "profiles":[{"id":"rust_analyzer","language":"rust","available":runtime().is_ok(),"command":"rust-analyzer.exe"}],
                "sandbox_available":sandbox_available,
                "sandbox_verified":false,
                "platform":"windows",
                "isolation":"appcontainer-no-network-readonly-snapshot"
            }))
        }
        "lsp_start" => {
            if request["profile"].as_str().unwrap_or("") != "rust_analyzer" {
                return Err("LSP_PROFILE_UNSUPPORTED".into());
            }
            let reservation = {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                let token = cache.reserve_start(&root)?;
                StartReservation { token }
            };
            let session = Session::start(&root, request["timeout_seconds"].as_u64().unwrap_or(30))?;
            let id = format!("lsp-{}", uuid::Uuid::new_v4().simple());
            {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                if !cache.start_is_active(&reservation.token, &root) {
                    // Reservation::drop re-locks the cache. Release this
                    // guard before returning on a cancelled startup.
                    drop(cache);
                    return Err("LSP_SESSION_CLOSED_DURING_START".into());
                }
                cache
                    .ready
                    .insert(id.clone(), Arc::new(Mutex::new(session)));
            }
            drop(reservation);
            Ok(
                json!({"session_id":id,"profile":"rust_analyzer","sandbox":"appcontainer-no-network-readonly-snapshot"}),
            )
        }
        "lsp_request" => {
            if request["session_id"].is_null() {
                return one_shot_document_request(&root, &request);
            }
            let id = request["session_id"]
                .as_str()
                .ok_or("LSP_SESSION_REQUIRED")?;
            let method = request["method"]
                .as_str()
                .filter(|method| allowed_method(method))
                .ok_or("LSP_METHOD_DISABLED")?;
            let session = get_session(&root, id)?;
            let mut session = session.try_lock().map_err(|_| "LSP_SESSION_BUSY")?;
            if session.closed {
                return Err("LSP_SESSION_NOT_FOUND".into());
            }
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
            let session = get_session(&root, id)?;
            let mut session = session.try_lock().map_err(|_| "LSP_SESSION_BUSY")?;
            if session.closed {
                return Err("LSP_SESSION_NOT_FOUND".into());
            }
            session.reset_deadline(request["timeout_seconds"].as_u64().unwrap_or(30));
            let diagnostics = match session.diagnostics(path) {
                Ok(value) => value,
                Err(error) => return Err(session.describe_error(error)),
            };
            session.last_used = Instant::now();
            Ok(json!({"diagnostics":diagnostics}))
        }
        "lsp_close" => {
            // A caller may close only its own workspace's sessions. Do not
            // hold the global cache mutex while acquiring per-session locks
            // or terminating a Job Object.
            let candidates: Vec<_> = {
                let cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                cache
                    .ready
                    .iter()
                    .filter(|(id, _)| {
                        request["session_id"]
                            .as_str()
                            .is_none_or(|wanted| id.as_str() == wanted)
                    })
                    .map(|(id, entry)| (id.clone(), Arc::clone(entry)))
                    .collect()
            };
            let mut owned = Vec::new();
            let mut busy_candidates = 0usize;
            for (id, entry) in candidates {
                let matches_workspace = match entry.try_lock() {
                    Ok(mut session) if session.root == root => {
                        // Mark as closed while holding its mutex. Requests
                        // that previously cloned this Arc must fail closed.
                        session.closed = true;
                        true
                    }
                    Err(_) if request["session_id"].as_str().is_some() => {
                        return Err("LSP_SESSION_BUSY".into());
                    }
                    Err(_) => {
                        busy_candidates += 1;
                        false
                    }
                    _ => false,
                };
                // The MutexGuard temporary is gone before Arc ownership
                // moves into the collection of sessions to terminate.
                if matches_workspace {
                    owned.push((id, entry));
                }
            }
            let removed: Vec<_> = {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                if request["session_id"].as_str().is_none() {
                    // Only invalidate starts from this workspace. The previous
                    // global epoch accidentally cancelled unrelated workspaces.
                    cache.cancel_pending_for(&root);
                }
                owned
                    .iter()
                    .filter_map(|(id, entry)| {
                        if cache
                            .ready
                            .get(id)
                            .is_some_and(|existing| Arc::ptr_eq(existing, entry))
                        {
                            cache.ready.remove(id)
                        } else {
                            None
                        }
                    })
                    .collect()
            };
            let count = removed.len();
            drop(removed);
            Ok(json!({
                "closed_sessions":count,
                "skipped_busy_candidates":busy_candidates,
                "close_complete":busy_candidates == 0
            }))
        }
        _ => Err("LSP_ACTION_UNKNOWN".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Emit the native failure as a one-line workflow command so the public
    /// Actions page names the failing capability and its reason, instead of
    /// showing only the panic location.
    fn acceptance<T>(label: &str, result: Result<T, String>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                eprintln!("::error::LSP_ACCEPTANCE_{label} {error}");
                panic!("LSP_ACCEPTANCE_{label}: {error}");
            }
        }
    }

    /// Run one reviewed toolchain image inside the AppContainer and report
    /// its real exit status and captured stderr. Rust Analyzer collapses a
    /// sandboxed `cargo` failure into "command failed", so this probe names
    /// the underlying AppContainer path or ACL cause on the public run page.
    fn toolchain_probe(
        label: &str,
        binary: &str,
        args: &[String],
        source_root: &Path,
        with_manifest: bool,
    ) {
        let profile = match SandboxProfile::create() {
            Ok(profile) => profile,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} PROFILE:{error}");
                return;
            }
        };
        // The snapshot has to live in the same AppContainer profile as the
        // cargo.exe that reads it, otherwise the probe only proves that a
        // foreign profile cannot see another profile's private snapshot.
        let manifest = if with_manifest {
            match snapshot(source_root, &profile) {
                Ok(snapshot_root) => Some(snapshot_root.join("Cargo.toml")),
                Err(error) => {
                    eprintln!("::error::LSP_CARGO_PROBE_{label} SNAPSHOT:{error}");
                    return;
                }
            }
        } else {
            None
        };
        let state = match profile.state_dir() {
            Ok(state) => state,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} STATE:{error}");
                return;
            }
        };
        let server = match runtime() {
            Ok(server) => server,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} RUNTIME:{error}");
                return;
            }
        };
        let bin = match server.parent() {
            Some(parent) => parent.to_path_buf(),
            None => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} TOOLCHAIN_BIN");
                return;
            }
        };
        let cargo = match profile.stage_runtime_peer(&bin.join("cargo.exe")) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} STAGE_CARGO:{error}");
                return;
            }
        };
        let rustc = match profile.stage_runtime_peer(&bin.join("rustc.exe")) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} STAGE_RUSTC:{error}");
                return;
            }
        };
        let home = state.join("cargo-home");
        if let Err(error) = fs::create_dir(&home) {
            eprintln!("::error::LSP_CARGO_PROBE_{label} CARGO_HOME:{error}");
            return;
        }
        let image: &Path = if binary == "cargo" { &cargo } else { &rustc };
        let environment = vec![
            ("CARGO".to_string(), cargo.to_string_lossy().into_owned()),
            ("RUSTC".to_string(), rustc.to_string_lossy().into_owned()),
            (
                "CARGO_HOME".to_string(),
                home.to_string_lossy().into_owned(),
            ),
            ("CARGO_NET_OFFLINE".to_string(), "true".to_string()),
        ];
        let mut full_args = args.to_vec();
        if let Some(manifest) = manifest.as_ref() {
            full_args.push("--manifest-path".to_string());
            full_args.push(manifest.to_string_lossy().into_owned());
        }
        let image_text = image.display().to_string();
        let child = match profile.spawn(image, &full_args, &environment, &state) {
            Ok(child) => child,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} SPAWN({image_text}) {error}");
                return;
            }
        };
        let SandboxChild {
            stdin,
            mut stdout,
            process,
        } = child;
        drop(stdin);
        let mut output = Vec::new();
        let _ = stdout.read_to_end(&mut output);
        let deadline = Instant::now() + Duration::from_secs(30);
        while !process.try_wait().unwrap_or(true) {
            if Instant::now() >= deadline {
                let _ = process.terminate();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let exit = process.exit_code();
        let stderr = process.profile().stderr_for_test();
        let tail = |text: &str| -> String {
            let bytes = text.as_bytes();
            let start = bytes.len().saturating_sub(900);
            String::from_utf8_lossy(&bytes[start..]).replace('\n', " | ")
        };
        let out = String::from_utf8_lossy(&output).into_owned();
        let succeeded = exit.as_ref().ok().and_then(|code| code.as_ref()) == Some(&0);
        if !succeeded {
            if let Some(manifest) = manifest.as_ref() {
                let mut acl = String::new();
                let mut targets = vec![manifest.clone()];
                if let Some(parent) = manifest.parent() {
                    targets.push(parent.to_path_buf());
                    if let Some(package) = parent.parent() {
                        targets.push(package.to_path_buf());
                    }
                }
                for target in targets {
                    if let Ok(probe) = std::process::Command::new("icacls.exe")
                        .arg(&target)
                        .output()
                    {
                        acl.push_str(&String::from_utf8_lossy(&probe.stdout));
                    }
                }
                eprintln!("::error::LSP_CARGO_PROBE_{label}_ACL {}", tail(&acl));
            }
        }
        eprintln!(
            "::error::LSP_CARGO_PROBE_{label} exit={exit:?} image={image_text} stdout={} stderr={}",
            tail(&out),
            tail(&stderr)
        );
    }

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
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &command,
            ])
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
        // Name the sandboxed toolchain failure before asserting on Rust
        // Analyzer: a probe error is a real capability failure, not a
        // diagnostics-timing artefact.
        {
            toolchain_probe(
                "LOCATE",
                "cargo",
                &["locate-project".to_string(), "--workspace".to_string()],
                &root,
                true,
            );
            toolchain_probe("VERSION", "cargo", &["--version".to_string()], &root, false);
            toolchain_probe(
                "RUSTC_VERSION",
                "rustc",
                &["--version".to_string()],
                &root,
                false,
            );
        }
        let started = acceptance(
            "START",
            execute(
                &root,
                json!({"action":"lsp_start","approved":true,"profile":"rust_analyzer","timeout_seconds":120}),
            ),
        );
        let id = started["session_id"].as_str().unwrap();
        let diagnostics = acceptance(
            "DIAGNOSTICS",
            execute(
                &root,
                json!({"action":"lsp_diagnostics","approved":true,"session_id":id,"path":"src/lib.rs","timeout_seconds":120}),
            ),
        );
        assert!(diagnostics["diagnostics"].is_array());
        let uri = uri_for(&root.join("src/lib.rs")).unwrap();
        let symbols = acceptance(
            "SYMBOLS",
            execute(
                &root,
                json!({"action":"lsp_request","approved":true,"session_id":id,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}},"timeout_seconds":120}),
            ),
        );
        assert!(
            symbols["result"].to_string().contains("fixture"),
            "{symbols}"
        );
        let closed = acceptance(
            "CLOSE",
            execute(&root, json!({"action":"lsp_close","session_id":id})),
        );
        assert_eq!(closed["closed_sessions"], 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_one_workspace_does_not_cancel_another_workspace_start() {
        let first = PathBuf::from(r"C:\isolated\workspace-a");
        let second = PathBuf::from(r"C:\isolated\workspace-b");
        let mut cache = SessionCache::default();
        let first_start = cache.reserve_start(&first).unwrap();
        let second_start = cache.reserve_start(&second).unwrap();
        cache.cancel_pending_for(&first);
        assert!(!cache.start_is_active(&first_start, &first));
        assert!(cache.start_is_active(&second_start, &second));
        assert_eq!(cache.starting.len(), 1);
        cache.starting.remove(&second_start);
        assert!(cache.starting.is_empty());
    }

    #[test]
    fn linux_document_contract_requires_scope_approval_before_windows_spawn() {
        let base =
            std::env::temp_dir().join(format!("browseros-lsp-document-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("src/lib.rs"), "fn fixture() {}").unwrap();
        let request = json!({
            "action":"lsp_request", "approved":true,
            "method":"diagnostics", "path":"src/lib.rs",
            "content":"fn fixture() {}", "line":0, "character":0,
            "module_root":"src/lib.rs"
        });
        assert_eq!(
            execute(&base, request).unwrap_err(),
            "LSP_MODULE_APPROVAL_REQUIRED"
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn single_document_uri_rewrite_does_not_expose_other_file_urls() {
        let snapshot = PathBuf::from(r"C:\isolated\snapshot");
        let root = PathBuf::from(r"C:\approved\workspace");
        let private = uri_for(&snapshot.join("document.rs")).unwrap();
        let public = uri_for(&root.join("src/lib.rs")).unwrap();
        let mut value = json!({
            "target": private,
            "external":"file:///C:/unapproved/secrets.rs"
        });
        remap_reviewed_uris(&mut value, &snapshot, &root, Some(&public));
        assert_eq!(value["target"], public);
        assert_eq!(value["external"], "[outside reviewed snapshot]");
    }

    #[test]
    fn persistent_response_uris_mask_unreviewed_locations() {
        let snapshot = PathBuf::from(r"C:\isolated\snapshot");
        let root = PathBuf::from(r"C:\approved\workspace");
        let original_uri = uri_for(&snapshot.join("src/main.rs")).unwrap();
        let public_uri = uri_for(&root.join("src/main.rs")).unwrap();
        let mut reply = json!({
            "approved": original_uri,
            "outside": "file:///C:/private/token.txt",
            "upper": "FILE:///C:/outside/secret.rs"
        });
        response_uri_to_workspace(&mut reply, &root, &snapshot);
        assert_eq!(reply["approved"], public_uri);
        assert_eq!(reply["outside"], "[outside reviewed snapshot]");
        assert_eq!(reply["upper"], "[outside reviewed snapshot]");
    }

    #[test]
    fn workspace_sessions_require_explicit_approval() {
        let temp =
            std::env::temp_dir().join(format!("browseros-lsp-approval-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&temp).unwrap();
        let denied = execute(
            &temp,
            json!({"action":"lsp_start","profile":"rust_analyzer"}),
        );
        assert_eq!(denied.unwrap_err(), "LSP_APPROVAL_REQUIRED");
        let denied = execute(
            &temp,
            json!({"action":"lsp_request","session_id":"foreign","method":"textDocument/hover"}),
        );
        assert_eq!(denied.unwrap_err(), "LSP_APPROVAL_REQUIRED");
        fs::remove_dir(temp).unwrap();
    }

    #[test]
    fn project_execution_methods_are_not_lsp_methods() {
        assert!(!allowed_method("workspace/executeCommand"));
        assert!(!allowed_method("rust-analyzer/runFlycheck"));
        assert!(allowed_method("textDocument/documentSymbol"));
    }
}
