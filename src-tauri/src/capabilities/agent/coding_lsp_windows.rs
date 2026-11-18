//! Windows Rust LSP in a no-network AppContainer with a private source copy.
//! Cargo may create lock files inside this copy; it is not a read-only tree.
//! The host workspace is not mounted or writable by the analyzer.
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
/// Generate a bounded non-Cargo project graph from the *private* copy.
/// rust-analyzer can analyze a rust-project.json without spawning cargo or
/// rustc. Its usual child-process launch opens \\.\NUL on Windows, which a
/// no-capability AppContainer token cannot access on the failing runner.
/// We deliberately expose syntax/IDE analysis, not compilation verification.
fn write_reviewed_rust_project(snapshot: &Path) -> Result<PathBuf, String> {
    let mut pending = vec![(snapshot.to_path_buf(), 0usize)];
    let mut crate_roots = Vec::<PathBuf>::new();
    let mut other_sources = Vec::<PathBuf>::new();
    let mut visited = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        if depth > 12 { return Err("LSP_PROJECT_DEPTH_LIMIT".into()); }
        for entry in fs::read_dir(&directory).map_err(|_| "LSP_PROJECT_READ_DENIED")? {
            let entry = entry.map_err(|_| "LSP_PROJECT_READ_DENIED")?;
            visited += 1;
            if visited > MAX_FILES * 2 { return Err("LSP_PROJECT_FILE_LIMIT".into()); }
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).map_err(|_| "LSP_PROJECT_READ_DENIED")?;
            if meta.file_attributes() & REPARSE != 0 {
                return Err("LSP_PROJECT_REPARSE_DENIED".into());
            }
            if meta.is_dir() {
                pending.push((path, depth + 1));
            } else if meta.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                match path.file_name().and_then(|name| name.to_str()) {
                    Some("lib.rs" | "main.rs" | "document.rs") => crate_roots.push(path),
                    _ => other_sources.push(path),
                }
            }
        }
    }
    if crate_roots.is_empty() {
        crate_roots = other_sources;
    }
    if crate_roots.is_empty() || crate_roots.len() > 32 {
        return Err("LSP_PROJECT_ROOT_LIMIT".into());
    }
    crate_roots.sort();
    crate_roots.dedup();
    let edition = fs::read_to_string(snapshot.join("Cargo.toml")).ok()
        .and_then(|source| {
            let mut package = false;
            for line in source.lines() {
                let line = line.split('#').next().unwrap_or("").trim();
                if line.starts_with('[') {
                    package = line == "[package]";
                } else if package {
                    if let Some((key, value)) = line.split_once('=') {
                        if key.trim() == "edition" {
                            let version = value.trim().trim_matches(&['"', '\''][..]);
                            return match version {
                                "2015" => Some("2015"),
                                "2018" => Some("2018"),
                                "2021" => Some("2021"),
                                "2024" => Some("2024"),
                                _ => None,
                            };
                        }
                    }
                }
            }
            None
        }).unwrap_or("2021");
    let mut crates = Vec::with_capacity(crate_roots.len());
    for (index, root_module) in crate_roots.iter().enumerate() {
        let absolute = root_module.to_str().ok_or("LSP_PROJECT_PATH_INVALID")?;
        crates.push(json!({
            "display_name":format!("reviewed_crate_{index}"),
            "root_module":absolute,
            "edition":edition,
            "deps":[],
            "cfg":[],
            "env":{},
            "is_proc_macro":false,
            "is_workspace_member":true
        }));
    }
    let document = serde_json::to_vec(&json!({"crates":crates}))
        .map_err(|_| "LSP_PROJECT_ENCODING_FAILED")?;
    if document.len() > MAX_FRAME { return Err("LSP_PROJECT_FILE_LIMIT".into()); }
    let project_path = snapshot.join("rust-project.json");
    fs::write(&project_path, document).map_err(|_| "LSP_PROJECT_WRITE_FAILED")?;
    Ok(project_path)
}

/// Validate the reviewed project graph before giving it to rust-analyzer.
/// Pass it inline at initialize time rather than asking a lowbox file watcher
/// to re-open a generated manifest by path. This removes one mutable watch
/// target and makes it impossible to silently fall back to Cargo discovery
/// when rust-project.json itself cannot be opened.
fn reviewed_project_for_initialize(snapshot: &Path) -> Result<Value, String> {
    let manifest = snapshot.join("rust-project.json");
    no_reparse(&manifest)?;
    let metadata = fs::symlink_metadata(&manifest)
        .map_err(|_| "LSP_PROJECT_MISSING")?;
    if !metadata.is_file()
        || metadata.file_attributes() & REPARSE != 0
        || metadata.len() > MAX_FRAME as u64
    {
        return Err("LSP_PROJECT_INVALID".into());
    }
    let bytes = crate::capabilities::workspace::safe_paths::read_regular(
        &manifest, &[snapshot.to_path_buf()], MAX_FRAME,
    ).map_err(|_| "LSP_PROJECT_READ_DENIED")?;
    let graph: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "LSP_PROJECT_INVALID")?;
    // The generated graph is a data description, never an instruction to
    // run arbitrary build commands. Discard any unexpected top-level keys
    // (including runnables, sysroot paths and external build actions).
    if graph.as_object().is_none_or(|fields|
        fields.len() != 1 || !fields.contains_key("crates")
    ) {
        return Err("LSP_PROJECT_INVALID".into());
    }
    let crates = graph["crates"].as_array().ok_or("LSP_PROJECT_INVALID")?;
    if crates.is_empty() || crates.len() > 32 {
        return Err("LSP_PROJECT_ROOT_LIMIT".into());
    }
    let reviewed_root = snapshot.canonicalize()
        .map_err(|_| "LSP_SNAPSHOT_PATH_INVALID")?;
    for krate in crates {
        let fields = krate.as_object().ok_or("LSP_PROJECT_INVALID")?;
        if fields.keys().any(|name| ![
            "display_name", "root_module", "edition", "deps", "cfg",
            "env", "is_proc_macro", "is_workspace_member",
        ].contains(&name.as_str()))
            || krate["deps"].as_array().is_none_or(|deps| !deps.is_empty())
            || krate["env"].as_object().is_none_or(|env| !env.is_empty())
            || krate["cfg"].as_array().is_none_or(|cfg| !cfg.is_empty())
            || krate["is_proc_macro"] != false
            || krate["is_workspace_member"] != true
        {
            return Err("LSP_PROJECT_INVALID".into());
        }
        let path = krate["root_module"].as_str().ok_or("LSP_PROJECT_INVALID")?;
        let source = Path::new(path);
        no_reparse(source)?;
        let real = source.canonicalize().map_err(|_| "LSP_PROJECT_SOURCE_MISSING")?;
        if !real.starts_with(&reviewed_root) || !real.is_file() {
            return Err("LSP_PROJECT_SOURCE_OUTSIDE_SNAPSHOT".into());
        }
    }
    Ok(graph)
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
    write_reviewed_rust_project(&destination)?;
    profile.protect_snapshot(&destination)?;
    grant_snapshot_read(&destination, profile.sid_string())?;
    // The staged runtime images load inside the lowbox with owner+SYSTEM
    // full control and a package `RX` entry. Snapshot files carrying only the
    // package entry are still rejected, so give them that same shape.
    profile.protect_snapshot_files(&destination)?;
    Ok(destination)
}
/// Give the private source files only this package SID's read access.
/// Windows lowbox path traversal also requires access to each ancestor.
/// The general AppContainer SID can therefore traverse/enumerate private
/// *directories* without inheritance; NEVER grant it access on source files.
/// The native cross-package negative test is a mandatory release gate.
fn grant_snapshot_read(root: &Path, sid: &str) -> Result<(), String> {
    let icacls = system_icacls()?;
    let mut ancestors = Vec::new();
    if let Some(container) = root.parent() {
        ancestors.push(container.to_path_buf());
        if let Some(package) = container.parent() {
            ancestors.push(package.to_path_buf());
        }
    }
    for directory in ancestors {
        grant_entry(&icacls, &directory, &format!("*{sid}:RX"))?;
        // No (OI)/(CI): an inherited general SID on files could reveal
        // private source to a different AppContainer.
        grant_entry(&icacls, &directory, "*S-1-15-2-1:RX")?;
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        grant_entry(&icacls, &directory, &format!("*{sid}:(OI)(CI)RX"))?;
        // Directory-only traversability; source files remain package-only.
        grant_entry(&icacls, &directory, "*S-1-15-2-1:RX")?;
        for entry in fs::read_dir(&directory).map_err(|_| "LSP_SNAPSHOT_ACL")? {
            let path = entry.map_err(|_| "LSP_SNAPSHOT_ACL")?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                grant_entry(&icacls, &path, &format!("*{sid}:RX"))?;
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
        write_reviewed_rust_project(&snapshot)?;
        profile.protect_snapshot(&snapshot)?;
        grant_snapshot_read(&snapshot, profile.sid_string())?;
        profile.protect_snapshot_files(&snapshot)?;
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
        // The no-network AppContainer must not launch Cargo/rustc children
        // through Rust std's Stdio::null (\\.\NUL is denied by the lowbox).
        // A reviewed rust-project.json is created before locking snapshot ACLs.
        // This intentionally provides IDE/syntax semantics without Cargo
        // dependency resolution, build scripts or compilation claims.
        let reviewed_graph = reviewed_project_for_initialize(&snapshot)?;
        let environment: Vec<(String, String)> = Vec::new();
        // Do not reintroduce the retired --no-proc-macro CLI flag: use LSP
        // initialization options to disable proc macros and check-on-save.
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
            deadline: Instant::now() + Duration::from_secs(seconds.clamp(5, 600)),
            next_id: 1,
            root,
            snapshot,
            last_used: Instant::now(),
            closed: false,
        };
        let root_uri = uri_for(&session.snapshot)?;
        let init = session.rpc_next("initialize", json!({
            "processId":null,
            "rootUri":root_uri.clone(),
            "workspaceFolders":[{"uri":root_uri,"name":"reviewed-snapshot"}],
            "capabilities":{"textDocument":{"synchronization":{},"publishDiagnostics":{},"definition":{},"references":{},"hover":{},"completion":{},"documentSymbol":{}}},
            "initializationOptions":{
                // rust-analyzer accepts inline rust-project.json graphs.
                // A file-path entry triggered file-watcher errors on the
                // isolated Windows profile even when the host file existed.
                "linkedProjects":[reviewed_graph],
                "cargo":{"sysroot":null,"autoreload":false,"buildScripts":{"enable":false}},
                "procMacro":{"enable":false},
                "checkOnSave":{"enable":false},
                "check":{"enable":false}
            }
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
        self.deadline = Instant::now() + Duration::from_secs(seconds.clamp(5, 600));
        self.last_used = Instant::now();
    }
    fn recv_frame(&self, timeout_error: &'static str) -> Result<Value, String> {
        let fault = || self.reader_fault
            .lock().unwrap_or_else(|p| p.into_inner()).clone();
        loop {
            if let Some(error) = fault() { return Err(error); }
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(fault().unwrap_or_else(|| timeout_error.to_string()));
            }
            // A rust-analyzer child failing to open the Windows NUL device
            // cannot be repaired by waiting longer for a notification.
            // Test builds keep a capped stderr tail; production never exposes
            // raw sandbox stderr or private file paths.
            let wait = remaining.min(Duration::from_millis(500));
            match self.reader.recv_timeout(wait) {
                Ok(value) => {
                    if let Some(error) = fault() { return Err(error); }
                    return value;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    #[cfg(test)]
                    if blocked_null_device_child(
                        &self.process.profile().stderr_for_test()
                    ) {
                        return Err("LSP_CHILD_NULL_DEVICE_DENIED".into());
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) =>
                    return Err(fault().unwrap_or_else(|| "LSP_CLOSED".into())),
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
            // Record which notifications the analyzer actually emits: a
            // silent timeout cannot distinguish "still loading the workspace"
            // from "published under a different document URI".
            #[cfg(test)]
            {
                let method = value["method"].as_str().unwrap_or("-");
                let detail = value["params"]["uri"]
                    .as_str()
                    .or(value["params"]["type"].as_str())
                    .unwrap_or("-");
                eprintln!("::error::LSP_RA_EVENT {method} {detail}");
            }
            // rust-analyzer normalises the drive letter to lower case
            // (`file:///c:/…`) while the client builds `file:///C:/…`, so an
            // exact string compare would never see its own document.
            if value["method"] == "textDocument/publishDiagnostics"
                && value["params"]["uri"]
                    .as_str()
                    .is_some_and(|published| published.eq_ignore_ascii_case(uri))
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
            let project = self.snapshot.join("rust-project.json");
            let source = self.snapshot.join("src/lib.rs");
            eprintln!(
                "LSP_SNAPSHOT_HOST_STATE root_dir={} project_file={} source_file={}",
                self.snapshot.is_dir(), project.is_file(), source.is_file()
            );
            eprintln!("LSP_SNAPSHOT_ROOT_ACES={}", tests::snapshot_acl_for_test(&self.snapshot));
            eprintln!("LSP_SNAPSHOT_PROJECT_ACES={}", tests::snapshot_acl_for_test(&project));
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
        "analysis_mode":"rust-project-no-cargo",
        "sandbox":"appcontainer-no-network-private-source-copy"
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
                "analysis_mode":"rust-project-no-cargo",
                "isolation":"appcontainer-no-network-private-source-copy"
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
                json!({"session_id":id,"profile":"rust_analyzer",
                    "analysis_mode":"rust-project-no-cargo",
                    "compilation_verified":false,
                    "sandbox":"appcontainer-no-network-private-source-copy"}),
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

/// Rust std::process::Stdio::null on Windows opens \.\NUL for Cargo
/// child stdin. This can fail in a lowbox even if directly spawned Cargo
/// works. Do not change the global NUL device DACL or escape the AppContainer.
#[cfg(test)]
fn blocked_null_device_child(stderr: &str) -> bool {
    stderr.contains("failed to open NUL device for child")
        && (stderr.contains("Access is denied") || stderr.contains("os error 5"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restricted_null_device_child_is_reported_without_raw_stderr() {
        assert!(blocked_null_device_child(
            "failed to open NUL device for child stdin: Access is denied. (os error 5)"
        ));
        assert!(!blocked_null_device_child("cargo metadata: dependency not found"));
    }

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

    /// Run `whoami /groups` inside the same AppContainer that later runs cargo.
    /// The access check for an AppContainer file needs the token's own SID set
    /// (package SID plus its restricted SIDs) to be satisfied by the DACL, so
    /// the ACL alone cannot explain a persistent denial.
    fn sandbox_token_probe() {
        let profile = match SandboxProfile::create() {
            Ok(profile) => profile,
            Err(error) => {
                eprintln!("::error::LSP_TOKEN_PROBE PROFILE:{error}");
                return;
            }
        };
        let state = match profile.state_dir() {
            Ok(state) => state,
            Err(error) => {
                eprintln!("::error::LSP_TOKEN_PROBE STATE:{error}");
                return;
            }
        };
        let system_root = std::env::var_os("SystemRoot").unwrap_or_default();
        let whoami = PathBuf::from(system_root)
            .join("System32")
            .join("whoami.exe");
        let child = match profile.spawn(
            &whoami,
            &[
                "/groups".to_string(),
                "/fo".to_string(),
                "csv".to_string(),
                "/nh".to_string(),
            ],
            &[],
            &state,
        ) {
            Ok(child) => child,
            Err(error) => {
                eprintln!("::error::LSP_TOKEN_PROBE SPAWN:{error}");
                return;
            }
        };
        let SandboxChild {
            stdin,
            mut stdout,
            process,
        } = child;
        drop(stdin);
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        let deadline = Instant::now() + Duration::from_secs(30);
        while !process.try_wait().unwrap_or(true) {
            if Instant::now() >= deadline {
                let _ = process.terminate();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let text = String::from_utf8_lossy(&out)
            .replace('\n', " | ")
            .replace('"', " ");
        let head: String = text.chars().take(1200).collect();
        eprintln!("::error::LSP_TOKEN_PROBE exit={:?} groups={head}", process.exit_code());
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
                let mut targets = vec![manifest.clone()];
                if let Some(parent) = manifest.parent() {
                    targets.push(parent.to_path_buf());
                    if let Some(package) = parent.parent() {
                        targets.push(package.to_path_buf());
                    }
                }
                // Keep the head of each listing: icacls prints the path line and
                // every ACE before the summary, so a tail-only dump hides the
                // entries that actually decide the access check.
                for target in targets {
                    let mut text = String::new();
                    if let Ok(probe) = std::process::Command::new("icacls.exe")
                        .arg(&target)
                        .output()
                    {
                        text.push_str(&String::from_utf8_lossy(&probe.stdout));
                    }
                    let head: String = text
                        .chars()
                        .take(700)
                        .collect::<String>()
                        .replace('\n', " | ");
                    eprintln!("::error::LSP_CARGO_PROBE_{label}_ACL {} :: {head}", target.display());
                }
            }
        }
        eprintln!(
            "::error::LSP_CARGO_PROBE_{label} exit={exit:?} image={image_text} stdout={} stderr={}",
            tail(&out),
            tail(&stderr)
        );
    }

    fn run_private_command_in_profile(
        profile: SandboxProfile, command: String,
    ) -> Result<(u32, String, String), String> {
        let state = profile.state_dir()?;
        let windows = std::env::var_os("SystemRoot")
            .ok_or("LSP_TEST_SYSTEMROOT_UNAVAILABLE")?;
        let cmd = PathBuf::from(windows).join("System32").join("cmd.exe");
        let tail = command;
        let SandboxChild { stdin, mut stdout, process } =
            profile.spawn(&cmd, &["/D".into(), "/C".into(), tail], &[], &state)?;
        drop(stdin);
        let (tx, rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let output = stdout.read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(|_| "LSP_PROFILE_READ_FAILED".to_string());
            let _ = tx.send(output);
        });
        let bytes = match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(data)) => data,
            _ => {
                let _ = process.terminate();
                return Err("LSP_PROFILE_READ_TIMEOUT".into());
            }
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while !process.try_wait()? {
            if Instant::now() >= deadline {
                let _ = process.terminate();
                return Err("LSP_PROFILE_EXIT_TIMEOUT".into());
            }
            thread::sleep(Duration::from_millis(20));
        }
        let exit = process.exit_code()?.ok_or("LSP_PROFILE_EXIT_UNAVAILABLE")?;
        Ok((exit, String::from_utf8_lossy(&bytes).into_owned(),
            process.profile().stderr_for_test()))
    }

    /// Only emit ACEs for the private snapshot's package SID in test logs;
    /// the public path/user profile is deliberately omitted from output.
    pub(super) fn snapshot_acl_for_test(path: &Path) -> String {
        let Ok(icacls) = system_icacls() else {
            return "icacls-unavailable".into();
        };
        match std::process::Command::new(icacls).arg(path).output() {
            Ok(out) => String::from_utf8_lossy(&out.stdout).lines()
                .filter_map(|line| line.find("S-1-15-2")
                    .map(|start| line[start..].trim().to_owned()))
                .take(12)
                .collect::<Vec<_>>().join(" | "),
            Err(_) => "icacls-execution-failed".into()
        }
    }

    #[test]
    fn private_snapshot_requires_own_package_sid_and_rejects_other_profiles() {
        let root = std::env::temp_dir().join(format!(
            "browseros-private-snapshot-{}", uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"),
            "[package]\nname='private_snapshot'\nversion='0.1.0'\nedition='2024'\n").unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn private_snapshot_marker() {}\n").unwrap();
        // Separate native smoke controls distinguish shell parsing, private
        // writable state, and read-only snapshot ACL. A failed "type" command
        // alone is not evidence that the snapshot bytes were never created.
        let command_profile = SandboxProfile::create().expect("cmd smoke profile");
        let (shell_exit, shell_out, shell_error) =
            run_private_command_in_profile(
                command_profile, "echo __BOS_LOWBOX_SHELL_OK__".into()
            ).expect("cmd smoke launched inside AppContainer");
        assert_eq!(shell_exit, 0,
            "SANDBOX_SHELL_CONTROL_FAILED stderr={shell_error}");
        assert!(shell_out.contains("__BOS_LOWBOX_SHELL_OK__"),
            "SANDBOX_SHELL_CONTROL_MISSING_OUTPUT");
        let state_profile = SandboxProfile::create().expect("state read profile");
        let state = state_profile.state_dir().expect("writable lowbox state");
        let state_file = state.join("state-read-control.txt");
        fs::write(&state_file, b"__BOS_LOWBOX_STATE_READ_OK__")
            .expect("write sandbox state fixture");
        let (state_exit, state_out, state_error) =
            run_private_command_in_profile(state_profile,
                format!("type \"{}\"", state_file.display()))
                .expect("state read control launched");
        assert_eq!(state_exit, 0,
            "SANDBOX_STATE_READ_FAILED stderr={state_error}");
        assert!(state_out.contains("__BOS_LOWBOX_STATE_READ_OK__"),
            "SANDBOX_STATE_READ_MISSING_OUTPUT");
        let owner = SandboxProfile::create().expect("owner AppContainer profile");
        let copy = snapshot(&root, &owner).expect("private source copy ACL");
        let copied_file = copy.join("src/lib.rs");
        // Prove a standalone lowbox file read works before testing shell
        // command chaining and cross-package denial.
        let baseline = SandboxProfile::create().expect("standalone read profile");
        let baseline_copy = snapshot(&root, &baseline).expect("standalone source copy");
        let baseline_file = baseline_copy.join("src/lib.rs");
        let (baseline_exit, baseline_out, baseline_error) =
            run_private_command_in_profile(
                baseline, format!("type \"{}\"", baseline_file.display())
            ).expect("standalone lowbox source read launched");
        assert_eq!(baseline_exit, 0,
            "own lowbox source file read failed: stdout={baseline_out} stderr={baseline_error}; verbatim={} dir ACL={}, file ACL={}",
            baseline_file.to_string_lossy().starts_with(r"\\?\"),
            snapshot_acl_for_test(&baseline_copy),
            snapshot_acl_for_test(&baseline_file));
        assert!(baseline_out.contains("private_snapshot_marker"),
            "own lowbox did not output source text");
        // Negative control first, while owner profile is still alive.
        let other = SandboxProfile::create().expect("different AppContainer profile");
        let other_copy = snapshot(&root, &other).expect("other package own source copy");
        // Positive control for the SAME other profile before probing owner
        // files. A nonzero exit alone can be a loader/runtime failure.
        let own_file = other_copy.join("src/lib.rs");
        let command = format!(
            "type \"{}\" & type \"{}\"", own_file.display(), copied_file.display()
        );
        let (other_exit, other_text, other_error) =
            run_private_command_in_profile(other, command)
                .expect("other package launched and attempted both file reads");
        assert_eq!(other_text.matches("private_snapshot_marker").count(), 1,
            "other package should read only its own copy: stdout={other_text} stderr={other_error}, own dir ACL={}, own file ACL={}",
            snapshot_acl_for_test(&other_copy),
            snapshot_acl_for_test(&own_file));
        assert_ne!(other_exit, 0,
            "unrelated package unexpectedly read owner source: {other_text}");
        let own_project = copy.join("rust-project.json");
        let own_command = format!(
            "type \"{}\" & type \"{}\"",
            copied_file.display(), own_project.display()
        );
        let (owner_exit, owner_text, owner_error) =
            run_private_command_in_profile(owner, own_command)
                .expect("owner profile launched to read source and project graph");
        assert_eq!(owner_exit, 0,
            "own lowbox could not read source/manifest: stdout={owner_text} stderr={owner_error}, project ACL={}",
            snapshot_acl_for_test(&own_project));
        assert!(owner_text.contains("private_snapshot_marker"));
        assert!(owner_text.contains("root_module"),
            "own lowbox could not open reviewed project graph");

        // Verify the authorized AppContainer does not retain directory
        // Modify rights from the old cargo-metadata snapshot contract.
        let writer = SandboxProfile::create().expect("write-negative profile");
        let read_only_copy = snapshot(&root, &writer).expect("read-only source copy");
        let source_file = read_only_copy.join("src/lib.rs");
        let before = fs::read(&source_file).unwrap();
        let write = format!("echo changed > \"{}\"", source_file.display());
        let (write_exit, _, _) = run_private_command_in_profile(writer, write)
            .expect("spawn lowbox denied-write fixture");
        assert_ne!(write_exit, 0, "AppContainer unexpectedly modified a source file");
        assert_eq!(fs::read(&source_file).unwrap(), before,
            "host-owned snapshot bytes changed under sandbox token");
        // Directory create and overwrite are independent Windows ACL rights.
        // A read-only source directory must not permit creating new files.
        let create = read_only_copy.join("src/unreviewed-created.rs");
        // The writer profile was consumed by the first probe. Use a separate
        // profile and its own private source copy for the create-negative check.
        let second_writer = SandboxProfile::create().expect("create-negative profile");
        let second_copy = snapshot(&root, &second_writer)
            .expect("read-only create-negative copy");
        let second_create = second_copy.join("src/unreviewed-created.rs");
        let (create_exit, _, _) = run_private_command_in_profile(
            second_writer, format!("echo injected > \"{}\"", second_create.display())
        ).expect("spawn lowbox create-negative fixture");
        assert_ne!(create_exit, 0, "AppContainer created a new source file");
        assert!(!second_create.exists(), "unauthorized snapshot file was created");
        assert!(!create.exists(), "unreviewed source file exists in the first copy");
        assert!(fs::read_to_string(root.join("src/lib.rs")).unwrap()
            .contains("private_snapshot_marker"),
            "the real host workspace was modified by sandbox operation");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_initialize_graph_is_inline_and_refuses_external_roots() {
        let root = std::env::temp_dir().join(format!(
            "browseros-lsp-inline-{}", uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn local_symbol() {}\n").unwrap();
        let manifest = write_reviewed_rust_project(&root).unwrap();
        let accepted = reviewed_project_for_initialize(&root).unwrap();
        assert_eq!(accepted["crates"].as_array().unwrap().len(), 1);
        assert!(accepted["crates"][0]["root_module"].as_str().unwrap()
            .ends_with("src\\lib.rs"));
        // The host may have altered the generated manifest after snapshot
        // creation; it must not inject a new unreviewed file in the LSP graph.
        let outside = root.parent().unwrap().join(format!(
            "browseros-lsp-unreviewed-{}.rs", uuid::Uuid::new_v4()
        ));
        fs::write(&outside, "pub fn secret() {}\n").unwrap();
        let mut runnable = accepted.clone();
        runnable["runnables"] = json!([{
            "program":"cmd.exe", "args":["/c","echo insecure"], "kind":"flycheck"
        }]);
        fs::write(&manifest, serde_json::to_vec(&runnable).unwrap()).unwrap();
        assert_eq!(reviewed_project_for_initialize(&root).unwrap_err(),
            "LSP_PROJECT_INVALID");
        let mut hostile = accepted;
        hostile["crates"][0]["root_module"] =
            json!(outside.to_str().unwrap());
        fs::write(&manifest, serde_json::to_vec(&hostile).unwrap()).unwrap();
        assert_eq!(
            reviewed_project_for_initialize(&root).unwrap_err(),
            "LSP_PROJECT_SOURCE_OUTSIDE_SNAPSHOT"
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn reviewed_rust_project_requires_local_sources_and_excludes_cargo_spawns() {
        let root = std::env::temp_dir().join(format!(
            "browseros-linked-project-{}", uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"),
            "[package]\nname='reviewed'\nversion='0.1.0'\nedition='2024'\n").unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        let project = write_reviewed_rust_project(&root).unwrap();
        let parsed: Value = serde_json::from_slice(&fs::read(&project).unwrap()).unwrap();
        assert_eq!(parsed["crates"][0]["edition"], "2024");
        assert_eq!(parsed["crates"][0]["deps"], json!([]));
        assert!(parsed["crates"][0]["root_module"].as_str().unwrap()
            .ends_with("src\\lib.rs"));
        assert_eq!(parsed["crates"].as_array().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reviewed_rust_project_rejects_snapshot_without_rust_sources() {
        let root = std::env::temp_dir().join(format!(
            "browseros-linked-empty-{}", uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        assert_eq!(write_reviewed_rust_project(&root).unwrap_err(),
            "LSP_PROJECT_ROOT_LIMIT");
        fs::remove_dir_all(root).unwrap();
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
    #[ignore = "legacy Cargo/Rustc AppContainer probes: not part of no-Cargo IDE acceptance"]
    fn legacy_cargo_subprocess_probes_are_diagnostics_only() {
        let root = std::env::temp_dir().join(format!(
            "browseros-legacy-toolchain-probe-{}", uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"),
            "[package]\nname='legacy_probe'\nversion='0.1.0'\nedition='2024'\n").unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        sandbox_token_probe();
        toolchain_probe(
            "LOCATE", "cargo",
            &["locate-project".to_owned(), "--workspace".to_owned()], &root, true,
        );
        toolchain_probe("VERSION", "cargo", &["--version".to_owned()], &root, false);
        toolchain_probe("RUSTC_VERSION", "rustc", &["--version".to_owned()], &root, false);
        fs::remove_dir_all(root).unwrap();
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
        fs::write(root.join("src/lib.rs"),
            "pub fn helper() -> i32 { 42 }\npub fn fixture() -> i32 { helper() }\nmod broken;\n").unwrap();
        fs::write(root.join("src/broken.rs"), "pub fn broken( {\n").unwrap();
        // This acceptance test measures the non-Cargo, no-network LSP path
        // only. The legacy Cargo toolchain probes are retained separately
        // as an ignored diagnostic test; their result cannot mask the IDE
        // protocol contract.
        let started = acceptance(
            "START",
            execute(
                &root,
                json!({"action":"lsp_start","approved":true,"profile":"rust_analyzer","timeout_seconds":300}),
            ),
        );
        let id = started["session_id"].as_str().unwrap();
        assert_eq!(started["analysis_mode"], "rust-project-no-cargo");
        assert_eq!(started["compilation_verified"], json!(false));
        let diagnostics = acceptance(
            "DIAGNOSTICS",
            execute(
                &root,
                json!({"action":"lsp_diagnostics","approved":true,"session_id":id,"path":"src/lib.rs","timeout_seconds":300}),
            ),
        );
        assert!(diagnostics["diagnostics"].is_array());
        let uri = uri_for(&root.join("src/lib.rs")).unwrap();
        let symbols = acceptance(
            "SYMBOLS",
            execute(
                &root,
                json!({"action":"lsp_request","approved":true,"session_id":id,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri.clone()}},"timeout_seconds":300}),
            ),
        );
        assert!(
            symbols["result"].to_string().contains("fixture")
                && symbols["result"].to_string().contains("helper"),
            "symbols missing local Rust functions: {symbols}"
        );
        // A definition and hover on a local function must work without a
        // Cargo build or sysroot discovery. The returned file URI must be
        // remapped from private snapshot to the approved host workspace.
        let line = "pub fn fixture() -> i32 { helper() }";
        let character = line.find("helper").expect("fixture call position") as u64 + 2;
        let hover = acceptance(
            "HOVER",
            execute(&root, json!({
                "action":"lsp_request", "approved":true, "session_id":id,
                "method":"textDocument/hover",
                "params":{"textDocument":{"uri":uri.clone()},
                    "position":{"line":1,"character":character}},
                "timeout_seconds":120
            })),
        );
        assert!(!hover["result"].is_null(), "hover returned null: {hover}");
        assert!(hover["result"].to_string().contains("helper"),
            "hover did not resolve fixture helper: {hover}");
        let definition = acceptance(
            "DEFINITION",
            execute(&root, json!({
                "action":"lsp_request", "approved":true, "session_id":id,
                "method":"textDocument/definition",
                "params":{"textDocument":{"uri":uri.clone()},
                    "position":{"line":1,"character":character}},
                "timeout_seconds":120
            })),
        );
        assert!(!definition["result"].is_null(),
            "definition returned null: {definition}");
        assert!(definition["result"].to_string().contains("lib.rs"),
            "definition did not map back into approved workspace: {definition}");
        assert!(!definition["result"].to_string().contains("snapshot"),
            "definition leaked the private snapshot path: {definition}");
        let broken = acceptance(
            "SYNTAX_DIAGNOSTICS",
            execute(&root, json!({
                "action":"lsp_diagnostics", "approved":true, "session_id":id,
                "path":"src/broken.rs", "timeout_seconds":120
            })),
        );
        assert!(broken["diagnostics"].as_array().is_some_and(|d| !d.is_empty()),
            "broken Rust code should publish parser diagnostics: {broken}");

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
