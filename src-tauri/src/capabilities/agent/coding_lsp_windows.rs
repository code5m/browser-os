//! Windows Rust LSP in a no-network AppContainer with a private source copy.
//! Source snapshots are read-only to the lowbox. A staged rustc is required
//! only for metadata/cfg probes; Cargo, build scripts and proc macros are disabled.
//! The host workspace is not mounted or writable by the analyzer.
#![cfg(windows)]

#[path = "windows_lsp_runtime.rs"]
mod runtime_bundle;

use super::windows_sandbox::{
    send_bounded, OperationBudget, SandboxChild, SandboxProcess, SandboxProfile,
};
use browseros_platform_security::windows::{PinnedReadDirectory, PinnedReadFile};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const MAX_FRAME: usize = 1024 * 1024;
const MAX_SESSIONS: usize = 2;
const MAX_FILES: usize = 2000;
const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 1024 * 1024;
const REPARSE: u32 = 0x0000_0400;
// The bundled rust-analyzer and its staged rustc are pinned to this target.
// An explicit target and cfg set are required for non-Cargo projects: an empty
// cfg list hides #[cfg(windows)] symbols even when the LSP itself launches.
const REVIEWED_WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";
const REVIEWED_WINDOWS_CFG: &[&str] = &[
    "windows",
    "target_os=\"windows\"",
    "target_family=\"windows\"",
    "target_arch=\"x86_64\"",
    "target_pointer_width=\"64\"",
    "target_env=\"msvc\"",
    "target_endian=\"little\"",
];

fn reviewed_target() -> Result<&'static str, String> {
    if cfg!(target_arch = "x86_64")
        && cfg!(target_env = "msvc")
        && cfg!(target_pointer_width = "64")
    {
        Ok(REVIEWED_WINDOWS_TARGET)
    } else {
        Err("LSP_TARGET_UNSUPPORTED".into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReservationPhase {
    Preparing,
    Ready,
    Cleanup,
}

struct StartingEntry {
    root: PathBuf,
    budget: OperationBudget,
    phase: ReservationPhase,
    owner_released: bool,
}

#[derive(Default)]
struct SessionCache {
    shutting_down: bool,
    ready: HashMap<String, Arc<Mutex<Session>>>,
    // A reservation owns capacity from the first preparation step until its
    // process, pipe workers and profile owner have actually finished cleanup.
    // Published sessions retain their reservation; ready.len() is not added.
    starting: HashMap<uuid::Uuid, StartingEntry>,
}
impl SessionCache {
    fn reserve_start(
        &mut self,
        root: &Path,
        budget: OperationBudget,
    ) -> Result<uuid::Uuid, String> {
        check_operation(&budget)?;
        if self.shutting_down {
            return Err("LSP_APPLICATION_SHUTTING_DOWN".into());
        }
        if self.starting.len() >= MAX_SESSIONS {
            return Err("LSP_SESSION_LIMIT".into());
        }
        let token = uuid::Uuid::new_v4();
        self.starting.insert(
            token,
            StartingEntry {
                root: root.to_path_buf(),
                budget,
                phase: ReservationPhase::Preparing,
                owner_released: false,
            },
        );
        Ok(token)
    }

    fn start_is_active(&self, token: &uuid::Uuid, root: &Path) -> bool {
        !self.shutting_down
            && self.starting.get(token).is_some_and(|reserved| {
                reserved.root == root
                    && reserved.phase == ReservationPhase::Preparing
                    && reserved.budget.check().is_ok()
            })
    }

    fn cancel_pending_for(&mut self, root: &Path) {
        for reserved in self.starting.values() {
            if reserved.root == root && reserved.phase == ReservationPhase::Preparing {
                // Cancellation does not free capacity. The owner still has to
                // observe this token and finish startup/cleanup before removal.
                reserved.budget.cancel();
            }
        }
    }

    fn release_start(&mut self, token: &uuid::Uuid) -> bool {
        if let Some(entry) = self.starting.get_mut(token) {
            if !entry.budget.cleanup_complete() {
                // Native create/spawn failure may have queued cleanup without
                // ever returning a SandboxChild to the caller.
                entry.phase = ReservationPhase::Cleanup;
                entry.owner_released = true;
                return true;
            }
        }
        self.starting.remove(token);
        false
    }

    fn reap_released(&mut self) {
        self.starting
            .retain(|_, entry| !entry.owner_released || !entry.budget.cleanup_complete());
    }

    fn pending_for(&self, root: &Path, phase: ReservationPhase) -> usize {
        self.starting
            .values()
            .filter(|entry| entry.root == root && entry.phase == phase)
            .count()
    }
}
static APPLICATION_SHUTDOWN: AtomicBool = AtomicBool::new(false);
static SESSIONS: OnceLock<Mutex<SessionCache>> = OnceLock::new();
fn sessions() -> &'static Mutex<SessionCache> {
    SESSIONS.get_or_init(|| Mutex::new(SessionCache::default()))
}

/// Quiesce admission before the coordinator waits for any other resource.
pub(crate) fn request_shutdown() {
    APPLICATION_SHUTDOWN.store(true, Ordering::Release);
    {
        let mut cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
        cache.shutting_down = true;
        for reservation in cache.starting.values() {
            reservation.budget.cancel();
        }
    }
    wake_cleanup_reaper();
}

fn retire_shutdown_sessions() {
    if !APPLICATION_SHUTDOWN.load(Ordering::Acquire) {
        return;
    }
    let candidates: Vec<_> = {
        let cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
        cache
            .ready
            .iter()
            .map(|(id, owner)| (id.clone(), owner.clone()))
            .collect()
    };
    for (id, owner) in candidates {
        let closed = match owner.try_lock() {
            Ok(mut session) => {
                // Initiate release, without adding a new per-session wait.
                let _ = session.shutdown_until(Instant::now());
                true
            }
            Err(std::sync::TryLockError::Poisoned(poison)) => {
                let mut session = poison.into_inner();
                let _ = session.shutdown_until(Instant::now());
                true
            }
            Err(std::sync::TryLockError::WouldBlock) => false,
        };
        if closed {
            let removed = {
                let mut cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
                if cache
                    .ready
                    .get(&id)
                    .is_some_and(|current| Arc::ptr_eq(current, &owner))
                {
                    cache.ready.remove(&id)
                } else {
                    None
                }
            };
            // Session/StartReservation Drop may acquire the cache again.
            drop(removed);
        }
        // Busy requests observe APPLICATION_SHUTDOWN at each receive/send poll.
        // Their owner remains in ready until a later reaper pass obtains it.
    }
}

pub(crate) fn shutdown_until(deadline: Instant) -> Result<(), String> {
    request_shutdown();
    loop {
        reap_deferred_cleanup();
        let (ready, reserved) = {
            let cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
            (cache.ready.len(), cache.starting.len())
        };
        let deferred = deferred_cleanup()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len();
        if ready == 0 && reserved == 0 && deferred == 0 {
            CLEANUP_SIGNAL.notify_all();
            let mut worker = CLEANUP_WORKER
                .get_or_init(|| Mutex::new(None))
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if worker.as_ref().is_some_and(|worker| worker.is_finished()) {
                if worker.take().unwrap().join().is_err() {
                    return Err("LSP_CLEANUP_REAPER_PANICKED".into());
                }
            }
            if worker.is_none() {
                return Ok(());
            }
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("LSP_APPLICATION_CLEANUP_PENDING: ready={ready} reserved={reserved} deferred={deferred}"));
        }
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }
}

fn operation_error(error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::TimedOut => "LSP_OPERATION_TIMEOUT",
        io::ErrorKind::Interrupted => "LSP_OPERATION_CANCELLED",
        _ => "LSP_OPERATION_BUDGET_FAILED",
    }
    .into()
}
fn check_operation(budget: &OperationBudget) -> Result<(), String> {
    if APPLICATION_SHUTDOWN.load(Ordering::Acquire) {
        return Err("LSP_APPLICATION_SHUTTING_DOWN".into());
    }
    budget.check().map_err(operation_error)
}
fn new_operation(seconds: u64) -> OperationBudget {
    OperationBudget::new(Duration::from_secs(seconds.clamp(5, 600)))
}

struct StartReservation {
    token: uuid::Uuid,
}
impl StartReservation {
    fn acquire(root: &Path, budget: OperationBudget) -> Result<Self, String> {
        check_operation(&budget)?;
        reap_deferred_cleanup();
        retry_cleanup_reaper();
        let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
        cache.reap_released();
        let token = cache.reserve_start(root, budget)?;
        Ok(Self { token })
    }

    fn set_phase(&self, phase: ReservationPhase) {
        let mut cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = cache.starting.get_mut(&self.token) {
            entry.phase = phase;
        }
    }
}
impl Drop for StartReservation {
    fn drop(&mut self) {
        let pending = {
            let mut cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
            cache.release_start(&self.token)
        };
        if pending {
            wake_cleanup_reaper();
        }
    }
}

/// One-shot requests have the same cancellable reservation and capacity limit
/// as persistent starts, including time spent loading approved module sources.
struct OneShotPermit(Option<StartReservation>);
impl OneShotPermit {
    fn acquire(root: &Path, budget: OperationBudget) -> Result<Self, String> {
        Ok(Self(Some(StartReservation::acquire(root, budget)?)))
    }
    fn into_reservation(mut self) -> StartReservation {
        self.0.take().expect("one-shot reservation is owned once")
    }
}

fn no_reparse_with_budget(path: &Path, operation: &OperationBudget) -> Result<(), String> {
    check_operation(operation)?;
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
        check_operation(operation)?;
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
fn rustup_home_with_budget(operation: &OperationBudget) -> Result<Option<PathBuf>, String> {
    check_operation(operation)?;
    let configured = std::env::var_os("RUSTUP_HOME");
    let explicitly_configured = configured.is_some();
    let home = configured.map(PathBuf::from).or_else(|| {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|home| home.join(".rustup"))
    });
    let Some(home) = home else {
        return Ok(None);
    };
    let settings = home.join("settings.toml");
    match fs::symlink_metadata(&settings) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_attributes() & REPARSE != 0 {
                return Err("LSP_RUSTUP_SETTINGS_UNSAFE".into());
            }
            no_reparse_with_budget(&settings, operation)?;
            Ok(Some(home))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !explicitly_configured => {
            Ok(None)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err("LSP_RUSTUP_SETTINGS_MISSING".into())
        }
        Err(_) => Err("LSP_RUSTUP_SETTINGS_UNREADABLE".into()),
    }
}
fn rustup_compiler_binary_with_budget(
    rustup_home: &Path,
    operation: &OperationBudget,
) -> Result<PathBuf, String> {
    check_operation(operation)?;
    let settings = crate::capabilities::workspace::safe_paths::read_regular(
        &rustup_home.join("settings.toml"),
        &[rustup_home.to_path_buf()],
        64 * 1024,
    )
    .map_err(|_| "LSP_RUSTUP_SETTINGS_UNREADABLE")?;
    check_operation(operation)?;
    let settings = String::from_utf8(settings).map_err(|_| "LSP_RUSTUP_SETTINGS_UNREADABLE")?;
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
        .join("rustc.exe");
    rustc_cfg_peer_with_budget(&candidate, operation).map_err(|error| {
        if error == "LSP_RUSTC_CFG_PEER_MISSING" {
            "LSP_RUSTC_TOOLCHAIN_MISSING".into()
        } else {
            error
        }
    })
}

struct SelectedRuntime {
    analyzer: runtime_bundle::VerifiedBundle,
    rustc: PinnedReadFile,
}

fn runtime() -> Result<SelectedRuntime, String> {
    runtime_with_budget(&new_operation(30))
}

fn runtime_with_budget(operation: &OperationBudget) -> Result<SelectedRuntime, String> {
    check_operation(operation)?;
    let compiler = installed_compiler_with_budget(operation)?;
    let rustc =
        PinnedReadFile::open(&compiler, operation).map_err(|_| "LSP_RUSTC_CFG_PEER_UNSAFE")?;
    let analyzer = runtime_bundle::open(operation)?;
    check_operation(operation)?;
    Ok(SelectedRuntime { analyzer, rustc })
}

fn installed_compiler_with_budget(operation: &OperationBudget) -> Result<PathBuf, String> {
    check_operation(operation)?;
    // Only rustc is selected from the installed toolchain. The analyzer must
    // come from the verified BrowserOS bundle, never a rustup shim or PATH.
    let missing_toolchain = if let Some(home) = rustup_home_with_budget(operation)? {
        match rustup_compiler_binary_with_budget(&home, operation) {
            Ok(compiler) => return Ok(compiler),
            Err(error) if error == "LSP_RUSTC_TOOLCHAIN_MISSING" => Some(error),
            // Invalid or unreadable configured toolchains must not silently
            // fall back to an unrelated executable later on PATH.
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    if let Some(paths) = std::env::var_os("PATH") {
        for (index, directory) in std::env::split_paths(&paths)
            .filter(|path| path.is_absolute())
            .enumerate()
        {
            check_operation(operation)?;
            if index >= 128 {
                return Err("LSP_RUNTIME_SEARCH_LIMIT".into());
            }
            let candidate = directory.join("rustc.exe");
            if !candidate.exists() {
                continue;
            }
            // A shim directory also ships rustup.exe; a real toolchain bin
            // directory does not.
            if directory.join("rustup.exe").is_file() {
                continue;
            }
            return rustc_cfg_peer_with_budget(&candidate, operation);
        }
    }
    Err(missing_toolchain.unwrap_or_else(|| "LSP_RUSTC_CFG_PEER_MISSING".into()))
}

/// A real rustc in the selected installed toolchain is required for cfg/data-layout
/// probes. Missing, reparse or non-file entries are explicit preflight failures.
fn rustc_cfg_peer(analyzer: &Path) -> Result<PathBuf, String> {
    rustc_cfg_peer_with_budget(analyzer, &new_operation(30))
}
fn rustc_cfg_peer_with_budget(
    analyzer: &Path,
    operation: &OperationBudget,
) -> Result<PathBuf, String> {
    check_operation(operation)?;
    let directory = analyzer.parent().ok_or("LSP_RUSTC_CFG_PEER_MISSING")?;
    let candidate = directory.join("rustc.exe");
    let metadata = fs::symlink_metadata(&candidate).map_err(|_| "LSP_RUSTC_CFG_PEER_MISSING")?;
    if !metadata.is_file() || metadata.file_attributes() & REPARSE != 0 {
        return Err("LSP_RUSTC_CFG_PEER_UNSAFE".into());
    }
    no_reparse_with_budget(&candidate, operation)?;
    candidate
        .canonicalize()
        .map_err(|_| "LSP_RUSTC_CFG_PEER_MISSING".into())
}

fn rustc_cfg_environment(
    profile: &SandboxProfile,
    selected: &SelectedRuntime,
) -> Result<Vec<(String, String)>, String> {
    check_operation(profile.budget())?;
    let staged = profile
        .stage_runtime_peer(selected.rustc.path())
        .map_err(|error| format!("LSP_RUSTC_CFG_STAGE_FAILED:{error}"))?;
    check_operation(profile.budget())?;
    let path = staged.to_str().ok_or("LSP_RUSTC_CFG_PATH_NOT_UNICODE")?;
    Ok(vec![("RUSTC".to_string(), path.to_string())])
}

fn lsp_toolchain_contract() -> Value {
    json!({
        "rustc_cfg_probe":"required",
        "cargo_enabled":false,
        "build_scripts_enabled":false,
        "proc_macros_enabled":false,
        "compilation_verified":false
    })
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
/// No Cargo discovery/build is enabled. rustc metadata/cfg probes use the
/// explicitly staged peer and remain subject to the lowbox restrictions.
/// Successful syntax/IDE analysis never constitutes compilation verification.
#[cfg(test)]
fn write_reviewed_rust_project(snapshot: &Path) -> Result<PathBuf, String> {
    write_reviewed_rust_project_with_budget(snapshot, &new_operation(30))
}
fn write_reviewed_rust_project_with_budget(
    snapshot: &Path,
    operation: &OperationBudget,
) -> Result<PathBuf, String> {
    check_operation(operation)?;
    let target = reviewed_target()?;
    let mut pending = vec![(snapshot.to_path_buf(), 0usize)];
    let mut crate_roots = Vec::<PathBuf>::new();
    let mut other_sources = Vec::<PathBuf>::new();
    let mut visited = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        check_operation(operation)?;
        if depth > 12 {
            return Err("LSP_PROJECT_DEPTH_LIMIT".into());
        }
        for entry in fs::read_dir(&directory).map_err(|_| "LSP_PROJECT_READ_DENIED")? {
            check_operation(operation)?;
            let entry = entry.map_err(|_| "LSP_PROJECT_READ_DENIED")?;
            visited += 1;
            if visited > MAX_FILES * 2 {
                return Err("LSP_PROJECT_FILE_LIMIT".into());
            }
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).map_err(|_| "LSP_PROJECT_READ_DENIED")?;
            if meta.file_attributes() & REPARSE != 0 {
                return Err("LSP_PROJECT_REPARSE_DENIED".into());
            }
            if meta.is_dir() {
                pending.push((path, depth + 1));
            } else if meta.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
            {
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
    check_operation(operation)?;
    let edition = read_reviewed_bytes(
        &snapshot.join("Cargo.toml"),
        snapshot,
        MAX_FILE_BYTES,
        operation,
    )
    .ok()
    .and_then(|bytes| String::from_utf8(bytes).ok())
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
    })
    .unwrap_or("2021");
    check_operation(operation)?;
    let mut crates = Vec::with_capacity(crate_roots.len());
    for (index, root_module) in crate_roots.iter().enumerate() {
        let absolute = root_module.to_str().ok_or("LSP_PROJECT_PATH_INVALID")?;
        crates.push(json!({
            "display_name":format!("reviewed_crate_{index}"),
            "root_module":absolute,
            "edition":edition,
            "deps":[],
            "cfg":REVIEWED_WINDOWS_CFG,
            "target":target,
            "env":{},
            "is_proc_macro":false,
            "is_workspace_member":true
        }));
    }
    let document =
        serde_json::to_vec(&json!({"crates":crates})).map_err(|_| "LSP_PROJECT_ENCODING_FAILED")?;
    if document.len() > MAX_FRAME {
        return Err("LSP_PROJECT_FILE_LIMIT".into());
    }
    let project_path = snapshot.join("rust-project.json");
    check_operation(operation)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&project_path)
        .map_err(|_| "LSP_PROJECT_WRITE_FAILED")?;
    for chunk in document.chunks(32 * 1024) {
        check_operation(operation)?;
        output
            .write_all(chunk)
            .map_err(|_| "LSP_PROJECT_WRITE_FAILED")?;
    }
    check_operation(operation)?;
    Ok(project_path)
}

/// Validate the reviewed project graph before giving it to rust-analyzer.
/// Pass it inline at initialize time rather than asking a lowbox file watcher
/// to re-open a generated manifest by path. This removes one mutable watch
/// target and makes it impossible to silently fall back to Cargo discovery
/// when rust-project.json itself cannot be opened.
#[cfg(test)]
fn reviewed_project_for_initialize(snapshot: &Path) -> Result<Value, String> {
    reviewed_project_for_initialize_with_budget(snapshot, &new_operation(30))
}
fn reviewed_project_for_initialize_with_budget(
    snapshot: &Path,
    operation: &OperationBudget,
) -> Result<Value, String> {
    check_operation(operation)?;
    let manifest = snapshot.join("rust-project.json");
    no_reparse_with_budget(&manifest, operation)?;
    let metadata = fs::symlink_metadata(&manifest).map_err(|_| "LSP_PROJECT_MISSING")?;
    if !metadata.is_file()
        || metadata.file_attributes() & REPARSE != 0
        || metadata.len() > MAX_FRAME as u64
    {
        return Err("LSP_PROJECT_INVALID".into());
    }
    let bytes = crate::capabilities::workspace::safe_paths::read_regular(
        &manifest,
        &[snapshot.to_path_buf()],
        MAX_FRAME,
    )
    .map_err(|_| "LSP_PROJECT_READ_DENIED")?;
    let graph: Value = serde_json::from_slice(&bytes).map_err(|_| "LSP_PROJECT_INVALID")?;
    // The generated graph is a data description, never an instruction to
    // run arbitrary build commands. Discard any unexpected top-level keys
    // (including runnables, sysroot paths and external build actions).
    if graph
        .as_object()
        .is_none_or(|fields| fields.len() != 1 || !fields.contains_key("crates"))
    {
        return Err("LSP_PROJECT_INVALID".into());
    }
    let crates = graph["crates"].as_array().ok_or("LSP_PROJECT_INVALID")?;
    if crates.is_empty() || crates.len() > 32 {
        return Err("LSP_PROJECT_ROOT_LIMIT".into());
    }
    let reviewed_root = snapshot
        .canonicalize()
        .map_err(|_| "LSP_SNAPSHOT_PATH_INVALID")?;
    for krate in crates {
        check_operation(operation)?;
        let fields = krate.as_object().ok_or("LSP_PROJECT_INVALID")?;
        if fields.keys().any(|name| {
            ![
                "display_name",
                "root_module",
                "edition",
                "deps",
                "cfg",
                "target",
                "env",
                "is_proc_macro",
                "is_workspace_member",
            ]
            .contains(&name.as_str())
        }) || krate["deps"].as_array().is_none_or(|deps| !deps.is_empty())
            || krate["env"].as_object().is_none_or(|env| !env.is_empty())
            || krate["cfg"] != json!(REVIEWED_WINDOWS_CFG)
            || krate["target"] != REVIEWED_WINDOWS_TARGET
            || krate["is_proc_macro"] != false
            || krate["is_workspace_member"] != true
        {
            return Err("LSP_PROJECT_INVALID".into());
        }
        let path = krate["root_module"].as_str().ok_or("LSP_PROJECT_INVALID")?;
        let source = Path::new(path);
        no_reparse_with_budget(source, operation)?;
        let real = source
            .canonicalize()
            .map_err(|_| "LSP_PROJECT_SOURCE_MISSING")?;
        if !real.starts_with(&reviewed_root) || !real.is_file() {
            return Err("LSP_PROJECT_SOURCE_OUTSIDE_SNAPSHOT".into());
        }
    }
    check_operation(operation)?;
    Ok(graph)
}

struct SnapshotBudget {
    entries: usize,
    files: usize,
    read_bytes: usize,
    written_bytes: usize,
}
impl SnapshotBudget {
    fn new() -> Self {
        Self {
            entries: 1,
            files: 0,
            read_bytes: 0,
            written_bytes: 0,
        }
    }
    fn visit(&mut self) -> Result<(), String> {
        self.entries = self
            .entries
            .checked_add(1)
            .ok_or("LSP_SNAPSHOT_ENTRY_LIMIT")?;
        if self.entries > MAX_FILES * 2 {
            return Err("LSP_SNAPSHOT_ENTRY_LIMIT".into());
        }
        Ok(())
    }
    fn record_read(&mut self, bytes: usize) -> Result<(), String> {
        self.read_bytes = self
            .read_bytes
            .checked_add(bytes)
            .ok_or("LSP_SNAPSHOT_LIMIT")?;
        if self.read_bytes > MAX_TOTAL_BYTES {
            return Err("LSP_SNAPSHOT_LIMIT".into());
        }
        Ok(())
    }
    fn record_write(&mut self, bytes: usize) -> Result<(), String> {
        self.written_bytes = self
            .written_bytes
            .checked_add(bytes)
            .ok_or("LSP_SNAPSHOT_LIMIT")?;
        if self.written_bytes > MAX_TOTAL_BYTES {
            return Err("LSP_SNAPSHOT_LIMIT".into());
        }
        Ok(())
    }
}

fn read_reviewed_bytes(
    path: &Path,
    root: &Path,
    limit: usize,
    operation: &OperationBudget,
) -> Result<Vec<u8>, String> {
    check_operation(operation)?;
    // The safe-path reader bounds the actual bytes read, not the earlier
    // pathname metadata. Each source read is at most one bounded file.
    let bytes = crate::capabilities::workspace::safe_paths::read_regular(
        path,
        &[root.to_path_buf()],
        limit,
    )
    .map_err(|_| "LSP_SNAPSHOT_READ_DENIED")?;
    check_operation(operation)?;
    if bytes.len() > limit {
        return Err("LSP_SNAPSHOT_LIMIT".into());
    }
    Ok(bytes)
}

fn write_snapshot_bytes(
    target: &Path,
    bytes: &[u8],
    budget: &mut SnapshotBudget,
    operation: &OperationBudget,
) -> Result<(), String> {
    check_operation(operation)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|_| "LSP_SNAPSHOT_WRITE_FAILED")?;
    let mut remaining = bytes;
    while !remaining.is_empty() {
        check_operation(operation)?;
        let available = MAX_TOTAL_BYTES.saturating_sub(budget.written_bytes);
        let count = remaining.len().min(32 * 1024).min(available);
        if count == 0 {
            return Err("LSP_SNAPSHOT_LIMIT".into());
        }
        let written = output
            .write(&remaining[..count])
            .map_err(|_| "LSP_SNAPSHOT_WRITE_FAILED")?;
        if written == 0 {
            return Err("LSP_SNAPSHOT_WRITE_FAILED".into());
        }
        budget.record_write(written)?;
        remaining = &remaining[written..];
    }
    check_operation(operation)?;
    Ok(())
}

fn snapshot_native_error(error: io::Error) -> String {
    if matches!(
        error.kind(),
        io::ErrorKind::Interrupted | io::ErrorKind::TimedOut
    ) {
        return operation_error(error);
    }
    let stage = error.to_string();
    if stage.contains("REPARSE") {
        "LSP_SNAPSHOT_REPARSE_DENIED".into()
    } else if stage.contains("TREE_OBJECT_LIMIT") {
        "LSP_SNAPSHOT_ENTRY_LIMIT".into()
    } else {
        "LSP_SNAPSHOT_READ_DENIED".into()
    }
}
fn read_pinned_snapshot_file(
    mut source: PinnedReadFile,
    budget: &mut SnapshotBudget,
    operation: &OperationBudget,
) -> Result<Vec<u8>, String> {
    check_operation(operation)?;
    let expected = source
        .file()
        .metadata()
        .map_err(|_| "LSP_SNAPSHOT_READ_DENIED")?
        .len();
    if expected > MAX_FILE_BYTES as u64 {
        return Err("LSP_SNAPSHOT_FILE_LIMIT".into());
    }
    if expected > MAX_TOTAL_BYTES.saturating_sub(budget.read_bytes) as u64 {
        return Err("LSP_SNAPSHOT_LIMIT".into());
    }
    let expected = expected as usize;
    let mut bytes = Vec::with_capacity(expected);
    let mut chunk = [0u8; 32 * 1024];
    while bytes.len() < expected {
        check_operation(operation)?;
        let count = chunk
            .len()
            .min(expected - bytes.len())
            .min(MAX_TOTAL_BYTES.saturating_sub(budget.read_bytes));
        if count == 0 {
            return Err("LSP_SNAPSHOT_LIMIT".into());
        }
        let read = source
            .read(&mut chunk[..count])
            .map_err(|_| "LSP_SNAPSHOT_READ_DENIED")?;
        if read == 0 {
            return Err("LSP_SNAPSHOT_SOURCE_CHANGED".into());
        }
        budget.record_read(read)?;
        bytes.extend_from_slice(&chunk[..read]);
    }
    check_operation(operation)?;
    // Avoid a limit+1 sentinel read: the bounded actual-byte accounting is
    // strict even if the source grows after its first metadata read.
    if source
        .file()
        .metadata()
        .map_err(|_| "LSP_SNAPSHOT_READ_DENIED")?
        .len()
        != expected as u64
    {
        return Err("LSP_SNAPSHOT_SOURCE_CHANGED".into());
    }
    Ok(bytes)
}
fn copy_snapshot_tree(
    source: &Path,
    root: &Path,
    destination: &Path,
    depth: usize,
    budget: &mut SnapshotBudget,
    operation: &OperationBudget,
) -> Result<(), String> {
    check_operation(operation)?;
    let relative = source
        .strip_prefix(root)
        .map_err(|_| "LSP_SNAPSHOT_ESCAPE")?;
    let source = PinnedReadDirectory::open(source, operation).map_err(snapshot_native_error)?;
    copy_pinned_snapshot_tree(&source, relative, destination, depth, budget, operation)
}
fn copy_pinned_snapshot_tree(
    source: &PinnedReadDirectory,
    relative: &Path,
    destination: &Path,
    depth: usize,
    budget: &mut SnapshotBudget,
    operation: &OperationBudget,
) -> Result<(), String> {
    check_operation(operation)?;
    if depth > 12 {
        return Err("LSP_SNAPSHOT_DEPTH_LIMIT".into());
    }
    let remaining = (MAX_FILES * 2).saturating_sub(budget.entries);
    let entries = source
        .entries(remaining, operation)
        .map_err(snapshot_native_error)?;
    for entry in entries {
        check_operation(operation)?;
        // Count every enumerated entry, including directories and excluded
        // names. Native enumeration itself is bounded by the remaining count.
        budget.visit()?;
        let excluded = entry.name.to_str().is_some_and(|name| {
            name.starts_with('.') || ["node_modules", "target", "dist", "vendor"].contains(&name)
        });
        if excluded {
            continue;
        }
        let relative = relative.join(&entry.name);
        let target = destination.join(&relative);
        if entry.directory {
            let child = source
                .child_directory(&entry, operation)
                .map_err(snapshot_native_error)?;
            fs::create_dir(&target).map_err(|_| "LSP_SNAPSHOT_WRITE_FAILED")?;
            copy_pinned_snapshot_tree(
                &child,
                &relative,
                destination,
                depth + 1,
                budget,
                operation,
            )?;
        } else if source_file(&relative) {
            budget.files = budget.files.checked_add(1).ok_or("LSP_SNAPSHOT_LIMIT")?;
            if budget.files > MAX_FILES {
                return Err("LSP_SNAPSHOT_LIMIT".into());
            }
            let file = source
                .child_file(&entry, operation)
                .map_err(snapshot_native_error)?;
            let bytes = read_pinned_snapshot_file(file, budget, operation)?;
            write_snapshot_bytes(&target, &bytes, budget, operation)?;
        }
    }
    check_operation(operation)?;
    Ok(())
}
fn snapshot(root: &Path, profile: &SandboxProfile) -> Result<PathBuf, String> {
    let operation = profile.budget();
    check_operation(operation)?;
    no_reparse_with_budget(root, operation)?;
    let destination = profile.snapshot_dir()?;
    copy_snapshot_tree(
        root,
        root,
        &destination,
        0,
        &mut SnapshotBudget::new(),
        operation,
    )?;
    write_reviewed_rust_project_with_budget(&destination, operation)?;
    profile.protect_lsp_snapshot(&destination)?;
    check_operation(operation)?;
    Ok(destination)
}
fn uri_for(path: &Path) -> Result<String, String> {
    url::Url::from_file_path(path)
        .map(|uri| uri.to_string())
        .map_err(|_| "LSP_URI_INVALID".into())
}
fn request_uri_to_snapshot(
    value: &mut Value,
    root: &Path,
    snapshot: &Path,
    operation: &OperationBudget,
) -> Result<(), String> {
    check_operation(operation)?;
    match value {
        Value::String(text)
            if text
                .get(..7)
                .is_some_and(|part| part.eq_ignore_ascii_case("file://")) =>
        {
            let uri = url::Url::parse(text).map_err(|_| "LSP_URI_INVALID")?;
            let path = uri.to_file_path().map_err(|_| "LSP_URI_INVALID")?;
            no_reparse_with_budget(&path, operation)?;
            let canonical = path.canonicalize().map_err(|_| "LSP_DOCUMENT_NOT_FOUND")?;
            let relative = canonical
                .strip_prefix(root)
                .map_err(|_| "LSP_DOCUMENT_OUTSIDE_WORKSPACE")?;
            *text = uri_for(&snapshot.join(relative))?;
        }
        Value::Array(items) => {
            for item in items {
                request_uri_to_snapshot(item, root, snapshot, operation)?;
            }
        }
        Value::Object(items) => {
            for item in items.values_mut() {
                request_uri_to_snapshot(item, root, snapshot, operation)?;
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

fn join_finished_worker(handle: &mut Option<thread::JoinHandle<()>>) -> Result<bool, String> {
    if handle.as_ref().is_some_and(|worker| worker.is_finished()) {
        handle
            .take()
            .unwrap()
            .join()
            .map_err(|_| "LSP_IO_THREAD_PANICKED".to_string())?;
    }
    Ok(handle.is_none())
}

struct DeferredCleanup {
    process: SandboxProcess,
    writer_thread: Option<thread::JoinHandle<()>>,
    reader_thread: Option<thread::JoinHandle<()>>,
    // Declared last: process/profile ownership is released before capacity.
    _reservation: Option<StartReservation>,
}
impl DeferredCleanup {
    fn poll(&mut self) -> bool {
        let _ = self.process.terminate();
        for handle in [&mut self.writer_thread, &mut self.reader_thread] {
            if join_finished_worker(handle).is_err() {
                eprintln!("[lsp] deferred cleanup: LSP_IO_THREAD_PANICKED");
            }
        }
        self.process.cleanup_complete()
            && self.writer_thread.is_none()
            && self.reader_thread.is_none()
    }
}
static DEFERRED_CLEANUP: OnceLock<Mutex<Vec<DeferredCleanup>>> = OnceLock::new();
fn deferred_cleanup() -> &'static Mutex<Vec<DeferredCleanup>> {
    DEFERRED_CLEANUP.get_or_init(|| Mutex::new(Vec::new()))
}
static CLEANUP_SIGNAL: Condvar = Condvar::new();
static CLEANUP_WORKER: OnceLock<Mutex<Option<thread::JoinHandle<()>>>> = OnceLock::new();

fn ensure_cleanup_reaper() {
    let slot = CLEANUP_WORKER.get_or_init(|| Mutex::new(None));
    let mut worker = slot.lock().unwrap_or_else(|p| p.into_inner());
    if worker.as_ref().is_some_and(|worker| worker.is_finished()) {
        if worker.take().unwrap().join().is_err() {
            eprintln!("[lsp] LSP_CLEANUP_REAPER_PANICKED");
        }
    }
    if worker.is_some() {
        return;
    }
    match thread::Builder::new()
        .name("browseros-lsp-cleanup".into())
        .spawn(|| {
            loop {
                reap_deferred_cleanup();
                let owners = deferred_cleanup().lock().unwrap_or_else(|p| p.into_inner());
                let (native_pending, application_pending) = {
                    let cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
                    (
                        cache.starting.values().any(|entry| entry.owner_released),
                        !cache.ready.is_empty() || !cache.starting.is_empty(),
                    )
                };
                if APPLICATION_SHUTDOWN.load(Ordering::Acquire)
                    && owners.is_empty()
                    && !application_pending
                {
                    break;
                }
                if !owners.is_empty()
                    || native_pending
                    || APPLICATION_SHUTDOWN.load(Ordering::Acquire)
                {
                    let (owners, _) = CLEANUP_SIGNAL
                        .wait_timeout(owners, Duration::from_millis(50))
                        .unwrap_or_else(|p| p.into_inner());
                    drop(owners);
                } else {
                    // The sender takes this same mutex before notifying, so an
                    // enqueue between the condition check and wait cannot be lost.
                    drop(
                        CLEANUP_SIGNAL
                            .wait(owners)
                            .unwrap_or_else(|p| p.into_inner()),
                    );
                }
            }
        }) {
        Ok(handle) => *worker = Some(handle),
        Err(_) => {
            // Leave every resource and slot owned. The next entry retries
            // creation and close_complete continues to report pending cleanup.
            eprintln!("[lsp] LSP_CLEANUP_REAPER_START_FAILED");
        }
    }
}
fn wake_cleanup_reaper() {
    ensure_cleanup_reaper();
    let owners = deferred_cleanup().lock().unwrap_or_else(|p| p.into_inner());
    CLEANUP_SIGNAL.notify_one();
    drop(owners);
}
fn retry_cleanup_reaper() {
    let pending = !deferred_cleanup()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .is_empty()
        || sessions()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .starting
            .values()
            .any(|entry| entry.owner_released);
    if pending {
        wake_cleanup_reaper();
    }
}
fn queue_deferred_cleanup(owner: DeferredCleanup) {
    deferred_cleanup()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(owner);
    wake_cleanup_reaper();
}

fn reap_deferred_cleanup() {
    retire_shutdown_sessions();
    let completed = {
        let mut owners = deferred_cleanup().lock().unwrap_or_else(|p| p.into_inner());
        let mut completed = Vec::new();
        let mut index = 0;
        while index < owners.len() {
            if owners[index].poll() {
                completed.push(owners.swap_remove(index));
            } else {
                index += 1;
            }
        }
        completed
    };
    drop(completed);
    let mut cache = sessions().lock().unwrap_or_else(|p| p.into_inner());
    cache.reap_released();
}

fn diagnostic_identifier(initialize: &Value) -> Result<Option<String>, String> {
    let provider = initialize["capabilities"]["diagnosticProvider"]
        .as_object()
        .ok_or("LSP_DIAGNOSTICS_UNSUPPORTED")?;
    if !provider
        .get("interFileDependencies")
        .is_some_and(Value::is_boolean)
        || !provider
            .get("workspaceDiagnostics")
            .is_some_and(Value::is_boolean)
    {
        return Err("LSP_DIAGNOSTICS_CAPABILITY_INVALID".into());
    }
    match provider.get("identifier") {
        None => Ok(None),
        Some(Value::String(identifier)) => Ok(Some(identifier.clone())),
        Some(_) => Err("LSP_DIAGNOSTICS_CAPABILITY_INVALID".into()),
    }
}

fn full_diagnostic_items(mut report: Value) -> Result<Value, String> {
    if report["kind"] != "full" || !report["items"].is_array() {
        return Err("LSP_DIAGNOSTICS_REPORT_INVALID".into());
    }
    Ok(report["items"].take())
}

fn rpc_response(value: &Value, expected_id: u64) -> Result<Option<Value>, String> {
    if value.get("method").is_some() || value["id"] != json!(expected_id) {
        return Ok(None);
    }
    if value["jsonrpc"] != "2.0" {
        return Err("LSP_RESPONSE_INVALID".into());
    }
    if value.get("error").is_some() {
        return Err("LSP_REMOTE_ERROR".into());
    }
    value
        .get("result")
        .cloned()
        .map(Some)
        .ok_or_else(|| "LSP_RESPONSE_INVALID".into())
}

// The approved private snapshot is immutable after startup. A no-op watcher
// registration is sufficient: didOpen/didChange carry later in-memory edits.
// Reject every unrelated server-requested operation.
fn immutable_snapshot_watcher_registration(value: &Value) -> bool {
    let (field, registering) = match value["method"].as_str() {
        Some("client/registerCapability") => ("registrations", true),
        Some("client/unregisterCapability") => ("unregisterations", false),
        _ => return false,
    };
    if value["jsonrpc"] != "2.0" || value.get("id").is_none() {
        return false;
    }
    value["params"][field].as_array().is_some_and(|entries| {
        !entries.is_empty()
            && entries.len() <= 16
            && entries.iter().all(|entry| {
                entry["method"] == "workspace/didChangeWatchedFiles"
                    && entry["id"]
                        .as_str()
                        .is_some_and(|id| !id.is_empty() && id.len() <= 128)
                    && (!registering || entry["registerOptions"]["watchers"].is_array())
            })
    })
}

fn server_status_ready(value: &Value) -> Result<bool, String> {
    if value["jsonrpc"] != "2.0"
        || value["method"] != "experimental/serverStatus"
        || value.get("id").is_some()
    {
        return Err("LSP_SERVER_STATUS_INVALID".into());
    }
    let status = &value["params"];
    let quiescent = status["quiescent"]
        .as_bool()
        .ok_or("LSP_SERVER_STATUS_INVALID")?;
    match status["health"].as_str() {
        Some("ok" | "warning") => Ok(quiescent),
        Some("error") if quiescent => Err("LSP_SERVER_NOT_READY".into()),
        Some("error") => Ok(false),
        _ => Err("LSP_SERVER_STATUS_INVALID".into()),
    }
}

// rust-analyzer can report quiescent before the initial VFS source import has
// rebuilt the crate graph. Source text may answer documentSymbol syntactically
// in this state, while hover/definition return null. Require a real workspace
// crate owned by our pinned inline graph before publishing a session.
fn reviewed_crate_graph_ready(value: &Value) -> Result<bool, String> {
    let graph = value.as_str().ok_or("LSP_CRATE_GRAPH_INVALID")?;
    if !graph.starts_with("digraph rust_analyzer_crate_graph {") {
        return Err("LSP_CRATE_GRAPH_INVALID".into());
    }
    Ok(graph.contains("reviewed_crate_0"))
}

struct OpenDocument {
    uri: String,
    version: i32,
    content_hash: String,
}

struct Session {
    process: Option<SandboxProcess>,
    writer: Option<mpsc::SyncSender<Vec<u8>>>,
    writer_thread: Option<thread::JoinHandle<()>>,
    reader_thread: Option<thread::JoinHandle<()>>,
    write_result: mpsc::Receiver<Result<(), String>>,
    reader: mpsc::Receiver<Result<Value, String>>,
    reader_fault: Arc<Mutex<Option<String>>>,
    operation: OperationBudget,
    cleanup_budget: OperationBudget,
    next_id: u64,
    diagnostic_identifier: Option<String>,
    documents: HashMap<String, OpenDocument>,
    root: PathBuf,
    snapshot: PathBuf,
    last_used: Instant,
    closed: bool,
    reservation: Option<StartReservation>,
}
impl Drop for Session {
    fn drop(&mut self) {
        if !self.closed {
            if APPLICATION_SHUTDOWN.load(Ordering::Acquire) {
                let _ = self.shutdown_until(Instant::now());
            } else {
                let _ = self.shutdown();
            }
        }
        self.writer.take();
        if !self.cleanup_complete() {
            // A timeout must not detach workers or drop the Job/profile owner.
            // The bounded deferred collection keeps the capacity lease until
            // all workers and the process have actually finished.
            if let Some(reservation) = &self.reservation {
                reservation.set_phase(ReservationPhase::Cleanup);
            }
            if let Some(process) = self.process.take() {
                let _ = process.terminate();
                let owner = DeferredCleanup {
                    process,
                    writer_thread: self.writer_thread.take(),
                    reader_thread: self.reader_thread.take(),
                    _reservation: self.reservation.take(),
                };
                queue_deferred_cleanup(owner);
            }
        } else {
            self.process.take();
            self.reservation.take();
        }
    }
}
impl Session {
    fn process(&self) -> &SandboxProcess {
        self.process
            .as_ref()
            .expect("live session retains its process owner")
    }
    fn resources_stopped(&self) -> bool {
        self.process
            .as_ref()
            .is_none_or(|process| process.cleanup_complete())
            && self.writer_thread.is_none()
            && self.reader_thread.is_none()
    }
    fn cleanup_complete(&self) -> bool {
        self.process.is_none() && self.resources_stopped() && self.cleanup_budget.cleanup_complete()
    }
    fn shutdown(&mut self) -> Result<(), String> {
        self.shutdown_until(Instant::now() + Duration::from_secs(3))
    }
    fn shutdown_until(&mut self, until: Instant) -> Result<(), String> {
        self.closed = true;
        if self.process.is_none() {
            self.reservation.take();
            return if self.cleanup_budget.cleanup_complete() {
                Ok(())
            } else {
                Err("LSP_PROFILE_CLEANUP_INCOMPLETE".into())
            };
        }
        self.operation.cancel();
        if let Some(reservation) = &self.reservation {
            reservation.set_phase(ReservationPhase::Cleanup);
        }
        let terminate_error = self.process().terminate().err();
        self.writer.take();
        while Instant::now() < until
            && (!self.process().cleanup_complete()
                || self
                    .writer_thread
                    .as_ref()
                    .is_some_and(|handle| !handle.is_finished())
                || self
                    .reader_thread
                    .as_ref()
                    .is_some_and(|handle| !handle.is_finished()))
        {
            thread::sleep(Duration::from_millis(10));
        }
        let mut errors = Vec::new();
        if !self.process().cleanup_complete() {
            errors.push("LSP_PROCESS_CLEANUP_INCOMPLETE".to_string());
        }
        if let Some(error) = terminate_error {
            errors.push(error);
        }
        for handle in [&mut self.writer_thread, &mut self.reader_thread] {
            match join_finished_worker(handle) {
                Ok(true) => {}
                Ok(false) => errors.push("LSP_CLEANUP_TIMEOUT".into()),
                Err(error) => errors.push(error),
            }
        }
        if self.resources_stopped() {
            // Destroy the native owner before checking the profile teardown
            // barrier. A stopped process alone does not mean its private
            // profile was removed or that the capacity slot may be reused.
            self.process.take();
            while !self.cleanup_budget.cleanup_complete() && Instant::now() < until {
                thread::sleep(Duration::from_millis(10));
            }
            self.reservation.take();
        }
        if !self.cleanup_budget.cleanup_complete() {
            errors.push("LSP_PROFILE_CLEANUP_INCOMPLETE".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(";"))
        }
    }

    #[cfg(test)]
    fn start(root: &Path, seconds: u64) -> Result<Self, String> {
        let operation = new_operation(seconds);
        check_operation(&operation)?;
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        let reservation = StartReservation::acquire(&root, operation.clone())?;
        Self::start_reserved(reservation, &root, operation)
    }
    fn start_reserved(
        reservation: StartReservation,
        root: &Path,
        operation: OperationBudget,
    ) -> Result<Self, String> {
        check_operation(&operation)?;
        no_reparse_with_budget(root, &operation)?;
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        no_reparse_with_budget(&root, &operation)?;
        let profile = SandboxProfile::create_with_budget(operation.clone())?;
        let snapshot = snapshot(&root, &profile)?;
        Self::start_with_snapshot(reservation, root, profile, snapshot, operation, None)
    }
    fn start_documents_reserved(
        reservation: StartReservation,
        root: &Path,
        documents: &[(String, String)],
        operation: OperationBudget,
        selected_server: Option<SelectedRuntime>,
    ) -> Result<Self, String> {
        check_operation(&operation)?;
        no_reparse_with_budget(root, &operation)?;
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        no_reparse_with_budget(&root, &operation)?;
        if documents.is_empty() || documents.len() > 128 {
            return Err("LSP_DOCUMENT_LIMIT".into());
        }
        let profile = SandboxProfile::create_with_budget(operation.clone())?;
        let snapshot = profile.snapshot_dir()?;
        let mut total = 0usize;
        let mut copied = SnapshotBudget::new();
        for (relative, content) in documents {
            check_operation(&operation)?;
            let path = Path::new(relative);
            if path.is_absolute()
                || relative.len() > 4096
                || !path
                    .components()
                    .all(|part| matches!(part, std::path::Component::Normal(_)))
                || path.components().count() > 12
                || !relative.ends_with(".rs")
                || content.len() > MAX_FILE_BYTES
            {
                return Err("LSP_DOCUMENT_DENIED".into());
            }
            for _ in path.components() {
                copied.visit()?;
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
            check_operation(&operation)?;
            write_snapshot_bytes(&target, content.as_bytes(), &mut copied, &operation)?;
        }
        write_reviewed_rust_project_with_budget(&snapshot, &operation)?;
        profile.protect_lsp_snapshot(&snapshot)?;
        check_operation(&operation)?;
        Self::start_with_snapshot(
            reservation,
            root,
            profile,
            snapshot,
            operation,
            selected_server,
        )
    }

    fn start_with_snapshot(
        reservation: StartReservation,
        root: PathBuf,
        profile: SandboxProfile,
        snapshot: PathBuf,
        operation: OperationBudget,
        selected_server: Option<SelectedRuntime>,
    ) -> Result<Self, String> {
        check_operation(&operation)?;
        let server = match selected_server {
            Some(server) => server,
            None => runtime_with_budget(&operation)?,
        };
        let state = profile.state_dir()?;
        // Keep the source graph read-only and disable Cargo/build execution.
        // rustc is required only for analyzer cfg/data-layout probes. Staging
        // errors must fail before spawning, not become a later timeout.
        let reviewed_graph = reviewed_project_for_initialize_with_budget(&snapshot, &operation)?;
        let environment = rustc_cfg_environment(&profile, &server)?;
        // Do not reintroduce the retired --no-proc-macro CLI flag: use LSP
        // initialization options to disable proc macros and check-on-save.
        check_operation(&operation)?;
        let child = profile
            .spawn_standalone(server.analyzer.path(), &[], &environment, &state)
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
        let (reader_tx, reader) = mpsc::sync_channel(32);
        let reader_fault = Arc::new(Mutex::new(None));
        let on_read_failure = Arc::clone(&reader_fault);
        // Establish ownership before spawning either I/O thread. Any later
        // startup error runs Session::drop and closes the entire process tree.
        let mut session = Self {
            process: Some(process),
            writer: Some(writer),
            writer_thread: None,
            reader_thread: None,
            write_result,
            reader,
            reader_fault,
            cleanup_budget: operation.clone(),
            operation,
            next_id: 1,
            diagnostic_identifier: None,
            documents: HashMap::new(),
            root,
            snapshot,
            last_used: Instant::now(),
            closed: false,
            reservation: Some(reservation),
        };
        check_operation(&session.operation)?;
        session.writer_thread = Some(
            thread::Builder::new()
                .name("browseros-lsp-write".into())
                .spawn(move || {
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
                })
                .map_err(|_| "LSP_WRITER_THREAD_START_FAILED")?,
        );
        check_operation(&session.operation)?;
        session.reader_thread = Some(
            thread::Builder::new()
                .name("browseros-lsp-read".into())
                .spawn(move || {
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
                })
                .map_err(|_| "LSP_READER_THREAD_START_FAILED")?,
        );
        let root_uri = uri_for(&session.snapshot)?;
        let init = session.rpc_next(
            "initialize",
            json!({
                "processId":null,
                "rootUri":root_uri.clone(),
                "workspaceFolders":[{"uri":root_uri,"name":"reviewed-snapshot"}],
                "capabilities":{
                    "experimental":{"serverStatusNotification":true},
                    "workspace":{"didChangeWatchedFiles":{"dynamicRegistration":true}},
                    "textDocument":{
                        "synchronization":{},"definition":{},"references":{},"hover":{},
                        "completion":{},"documentSymbol":{},
                        "diagnostic":{"dynamicRegistration":false,"relatedDocumentSupport":false}
                    }
                },
                "initializationOptions":{
                    // rust-analyzer accepts inline rust-project.json graphs.
                    // A file-path entry triggered file-watcher errors on the
                    // isolated Windows profile even when the host file existed.
                    "linkedProjects":[reviewed_graph],
                    // No disk watcher is needed for an immutable private copy.
                    "files":{"watcher":"client"},
                    // Only the preapproved inline rust-project is loaded; this flag
                    // permits VFS's initial source import to rebuild its crate
                    // graph. It never authorizes cargo metadata/build scripts.
                    "cargo":{"sysroot":null,"autoreload":true,"buildScripts":{"enable":false}},
                    "procMacro":{"enable":false},
                    "checkOnSave":{"enable":false},
                    "check":{"enable":false}
                }
            }),
        );
        #[cfg(test)]
        if let Err(ref error) = init {
            eprintln!(
                "LSP_INITIALIZE_DIAG={error}; exit={:?}; stderr={:?}",
                session.process().exit_code(),
                session.process().profile().stderr_for_test()
            );
        }
        let initialized = init.map_err(|error| session.describe_error(error))?;
        session.diagnostic_identifier = diagnostic_identifier(&initialized)?;
        check_operation(&session.operation)?;
        session.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))?;
        // START must not publish a session whose source graph is still loading.
        // The same preparation deadline covers readiness; no nested timer starts.
        session
            .await_ready()
            .map_err(|error| session.describe_error(error))?;
        Ok(session)
    }
    fn begin_request(&mut self, operation: OperationBudget) {
        self.operation = operation;
        self.last_used = Instant::now();
    }
    fn observed_failure(&self) -> Option<String> {
        // Prefer the sticky native failure over a downstream EOF/backpressure
        // symptom. No raw stderr or path reaches the production response.
        if let Some(code) = self.process().profile().stderr_diagnostic() {
            return Some(code.to_owned());
        }
        self.reader_fault
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
    fn recv_frame(&self, timeout_error: &'static str) -> Result<Value, String> {
        loop {
            check_operation(&self.operation)?;
            if let Some(error) = self.observed_failure() {
                return Err(error);
            }
            let remaining = self.operation.remaining().map_err(operation_error)?;
            let wait = remaining.min(Duration::from_millis(100));
            match self.reader.recv_timeout(wait) {
                Ok(value) => {
                    check_operation(&self.operation)?;
                    if let Some(error) = self.observed_failure() {
                        return Err(error);
                    }
                    return value;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(error) = self.observed_failure() {
                        return Err(error);
                    }
                    if self.operation.remaining().is_err() {
                        check_operation(&self.operation)?;
                        return Err(timeout_error.into());
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self
                        .observed_failure()
                        .unwrap_or_else(|| "LSP_CLOSED".into()))
                }
            }
        }
    }
    fn send(&mut self, value: Value) -> Result<(), String> {
        check_operation(&self.operation)?;
        let body = serde_json::to_vec(&value).map_err(|_| "LSP_ENCODE_FAILED")?;
        if body.len() > MAX_FRAME {
            return Err("LSP_REQUEST_LIMIT".into());
        }
        let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        bytes.extend(body);
        let mut pending = bytes;
        loop {
            check_operation(&self.operation)?;
            if let Some(error) = self.observed_failure() {
                return Err(error);
            }
            let writer = self.writer.as_ref().ok_or("LSP_SESSION_CLOSED")?;
            match writer.try_send(pending) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Disconnected(_)) => return Err("LSP_WRITE_FAILED".into()),
                Err(mpsc::TrySendError::Full(value)) => {
                    pending = value;
                    let remaining = self.operation.remaining().map_err(operation_error)?;
                    thread::sleep(remaining.min(Duration::from_millis(5)));
                }
            }
        }
        loop {
            check_operation(&self.operation)?;
            if let Some(error) = self.observed_failure() {
                return Err(error);
            }
            let remaining = self.operation.remaining().map_err(operation_error)?;
            match self
                .write_result
                .recv_timeout(remaining.min(Duration::from_millis(100)))
            {
                Ok(result) => {
                    check_operation(&self.operation)?;
                    return result;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err("LSP_WRITE_FAILED".into()),
            }
        }
    }
    fn answer_server_request(&mut self, value: &Value) -> Result<(), String> {
        let response = if immutable_snapshot_watcher_registration(value) {
            json!({"jsonrpc":"2.0","id":value["id"],"result":null})
        } else {
            json!({"jsonrpc":"2.0","id":value["id"],"error":{
                "code":-32601,"message":"Client operations are disabled"
            }})
        };
        self.send(response)
    }
    fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let value = self.recv_frame("LSP_TIMEOUT_OR_CLOSED")?;
            if let Some(result) = rpc_response(&value, id)? {
                return Ok(result);
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.answer_server_request(&value)?;
            }
        }
    }
    fn rpc_next(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.rpc(id, method, params)
    }
    fn await_ready(&mut self) -> Result<(), String> {
        loop {
            let value = self.recv_frame("LSP_READY_TIMEOUT")?;
            #[cfg(test)]
            if value["method"] == "experimental/serverStatus"
                && value["params"]["quiescent"] == true
            {
                let health = value["params"]["health"].as_str().unwrap_or("invalid");
                let detail = value["params"]["message"].as_str().unwrap_or("");
                let brief = detail.lines().take(8).collect::<Vec<_>>().join(" ");
                eprintln!("LSP_READY_STATUS health={health} message={brief}");
            }
            if value["method"] == "experimental/serverStatus"
                && value.get("id").is_none()
                && server_status_ready(&value)?
            {
                // A ready notification alone is not semantic readiness. With
                // the immutable client watcher, the VFS initially loads files
                // after the first crate-graph build. Wait for that reload under
                // this same startup budget; never return syntax-only success.
                loop {
                    check_operation(&self.operation)?;
                    let graph =
                        self.rpc_next("rust-analyzer/viewCrateGraph", json!({"full":false}))?;
                    if reviewed_crate_graph_ready(&graph)? {
                        return Ok(());
                    }
                    let remaining = self.operation.remaining().map_err(operation_error)?;
                    thread::sleep(remaining.min(Duration::from_millis(100)));
                }
            }
            if value.get("method").is_some() && value.get("id").is_some() {
                self.answer_server_request(&value)?;
            }
        }
    }

    fn open_document(&mut self, relative: &Path, text: &str) -> Result<String, String> {
        check_operation(&self.operation)?;
        let requested_uri = uri_for(&self.snapshot.join(relative))?;
        // Match the existing Windows URI comparison without changing authority:
        // native reviewed reads and private-snapshot access still authorize I/O.
        let key = requested_uri.to_ascii_lowercase();
        let content_hash = format!("{:x}", Sha256::digest(text.as_bytes()));
        let previous = self.documents.get(&key);
        if let Some(document) = previous {
            if document.content_hash == content_hash {
                return Ok(document.uri.clone());
            }
        } else if self.documents.len() >= MAX_FILES {
            return Err("LSP_DOCUMENT_LIMIT".into());
        }
        let version = match previous {
            Some(document) => document
                .version
                .checked_add(1)
                .ok_or("LSP_DOCUMENT_VERSION_EXHAUSTED")?,
            None => 1,
        };
        let uri = previous
            .map(|document| document.uri.clone())
            .unwrap_or(requested_uri);
        let notification = if previous.is_none() {
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
                "textDocument":{"uri":uri,"languageId":"rust","version":version,"text":text}
            }})
        } else {
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
                "textDocument":{"uri":uri,"version":version},
                "contentChanges":[{"text":text}]
            }})
        };
        self.send(notification)?;
        // Only a completed native write advances the synchronized version.
        self.documents.insert(
            key,
            OpenDocument {
                uri: uri.clone(),
                version,
                content_hash,
            },
        );
        Ok(uri)
    }

    fn pull_diagnostics(&mut self, uri: &str) -> Result<Value, String> {
        // Never supply previousResultId: an empty diagnostic set is valid, but
        // an unchanged/stale report cannot replace this synchronized document.
        let key = uri.to_ascii_lowercase();
        let document = self
            .documents
            .get(&key)
            .ok_or("LSP_DOCUMENT_NOT_SYNCHRONIZED")?;
        let version = document.version;
        let mut params = json!({"textDocument":{"uri":document.uri.as_str()}});
        if let Some(identifier) = &self.diagnostic_identifier {
            params["identifier"] = json!(identifier);
        }
        let report = self.rpc_next("textDocument/diagnostic", params)?;
        if !self
            .documents
            .get(&key)
            .is_some_and(|document| document.version == version)
        {
            return Err("LSP_DOCUMENT_CHANGED_DURING_DIAGNOSTICS".into());
        }
        full_diagnostic_items(report)
    }

    fn diagnostics(&mut self, path: &str) -> Result<Value, String> {
        let document = PathBuf::from(path);
        if document.is_absolute() || path.contains("..") {
            return Err("LSP_DOCUMENT_INVALID".into());
        }
        let host = self.root.join(&document);
        no_reparse_with_budget(&host, &self.operation)?;
        let bytes = read_reviewed_bytes(&host, &self.root, MAX_FILE_BYTES, &self.operation)?;
        let text = String::from_utf8(bytes).map_err(|_| "LSP_DOCUMENT_NOT_TEXT")?;
        let uri = self.open_document(&document, &text)?;
        let mut diagnostics = self.pull_diagnostics(&uri)?;
        response_uri_to_workspace(&mut diagnostics, &self.root, &self.snapshot);
        Ok(diagnostics)
    }

    /// Test builds retain bounded native evidence. Production emits fixed
    /// diagnostic codes and an optional numeric exit code, never raw stderr.
    fn describe_error(&self, error: String) -> String {
        #[cfg(test)]
        {
            let project = self.snapshot.join("rust-project.json");
            let source = self.snapshot.join("src/lib.rs");
            eprintln!(
                "LSP_SNAPSHOT_HOST_STATE root_dir={} project_file={} source_file={}",
                self.snapshot.is_dir(),
                project.is_file(),
                source.is_file()
            );
            eprintln!(
                "LSP_SNAPSHOT_ROOT_ACES={}",
                tests::snapshot_acl_for_test(&self.snapshot)
            );
            eprintln!(
                "LSP_SNAPSHOT_PROJECT_ACES={}",
                tests::snapshot_acl_for_test(&project)
            );
            format!(
                "{error} exit={:?} stderr={:?}",
                self.process().exit_code(),
                self.process().profile().stderr_for_test()
            )
        }
        #[cfg(not(test))]
        {
            let diagnostic = self
                .process
                .as_ref()
                .and_then(|process| process.profile().stderr_diagnostic())
                .unwrap_or(error.as_str());
            let exit = self
                .process
                .as_ref()
                .and_then(|process| process.exit_code().ok().flatten());
            format!("{diagnostic}; exit={exit:?}")
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

fn one_shot_document_request(
    root: &Path,
    request: &Value,
    operation: OperationBudget,
) -> Result<Value, String> {
    check_operation(&operation)?;
    let permit = OneShotPermit::acquire(root, operation.clone())?;
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
    let selected_server = if let Some(selected) = request["server_path"].as_str() {
        let real = runtime_with_budget(&operation)?;
        let requested = Path::new(selected)
            .canonicalize()
            .map_err(|_| "LSP_SERVER_UNAVAILABLE")?;
        if requested != real.analyzer.path() {
            return Err("LSP_SERVER_NOT_TRUSTED_PROFILE".into());
        }
        check_operation(&operation)?;
        Some(real)
    } else {
        None
    };
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
            let mut docs = super::coding::lsp_module_sources_with_checkpoint(root, module, || {
                check_operation(&operation)
            })?;
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
    check_operation(&operation)?;
    let mut session = Session::start_documents_reserved(
        permit.into_reservation(),
        root,
        &documents,
        operation,
        selected_server,
    )?;
    let private_uri = session.open_document(Path::new(&relative), content)?;
    let public_uri = uri_for(&root.join(path))?;
    let mut diagnostics = if request["method"] == "diagnostics" {
        session.pull_diagnostics(&private_uri)?
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
    session.shutdown()?;
    Ok(json!({
        "result":result,
        "diagnostics":diagnostics,
        "document_uri":public_uri,
        "document_hash":format!("{:x}", Sha256::digest(content.as_bytes())),
        "scope":scope,
        "compilation_verified":false,
        "analysis_mode":"rust-project-no-cargo",
        "toolchain":lsp_toolchain_contract(),
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
                if !session.closed && session.last_used.elapsed() <= Duration::from_secs(120) {
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
    for entry in expired {
        if let Ok(mut session) = entry.try_lock() {
            let _ = session.shutdown();
        }
        // An incomplete owner moves to deferred cleanup when its final Arc
        // drops. Its reservation remains occupied until that cleanup finishes.
    }
}

pub fn execute(root: &Path, request: Value) -> Result<Value, String> {
    // This clock starts before canonicalization, reservation, snapshot, ACL
    // changes, runtime staging or process creation. Nested stages never reset it.
    let operation = new_operation(request["timeout_seconds"].as_u64().unwrap_or(30));
    check_operation(&operation)?;
    reap_deferred_cleanup();
    retry_cleanup_reaper();
    retire_expired();
    check_operation(&operation)?;
    let action = request["action"].as_str().unwrap_or("");
    // Capability discovery is a global read-only operation; no workspace
    // has to be selected yet. Other operations require a real pinned root.
    let root = if action == "lsp_status" {
        PathBuf::new()
    } else {
        no_reparse_with_budget(root, &operation)?;
        let root = root.canonicalize().map_err(|_| "LSP_ROOT_INVALID")?;
        no_reparse_with_budget(&root, &operation)?;
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
            let toolchain = runtime_with_budget(&operation);
            Ok(json!({
                "profiles":[{"id":"rust_analyzer","name":"Rust Analyzer","language":"rust",
                    "available":toolchain.is_ok(),"command":"rust-analyzer.exe",
                    "path":toolchain.as_ref().ok().map(|selected| selected.analyzer.path().to_string_lossy().into_owned()),
                    "unavailable_reason":toolchain.as_ref().err()}],
                "toolchain":lsp_toolchain_contract(),
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
            let reservation = StartReservation::acquire(&root, operation.clone())?;
            let token = reservation.token;
            let session = Session::start_reserved(reservation, &root, operation)?;
            let id = format!("lsp-{}", uuid::Uuid::new_v4().simple());
            {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                if !cache.start_is_active(&token, &root) {
                    // Reservation::drop re-locks the cache. Release this
                    // guard before returning on a cancelled startup.
                    drop(cache);
                    return Err("LSP_SESSION_CLOSED_DURING_START".into());
                }
                if let Some(entry) = cache.starting.get_mut(&token) {
                    entry.phase = ReservationPhase::Ready;
                }
                cache
                    .ready
                    .insert(id.clone(), Arc::new(Mutex::new(session)));
            }
            Ok(json!({"session_id":id,"profile":"rust_analyzer",
                    "analysis_mode":"rust-project-no-cargo",
                    "toolchain":lsp_toolchain_contract(),
                    "compilation_verified":false,
                    "sandbox":"appcontainer-no-network-private-source-copy"}))
        }
        "lsp_request" => {
            if request["session_id"].is_null() {
                return one_shot_document_request(&root, &request, operation);
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
            session.begin_request(operation);
            let mut params = request.get("params").cloned().unwrap_or(json!({}));
            let root = session.root.clone();
            let snapshot = session.snapshot.clone();
            request_uri_to_snapshot(&mut params, &root, &snapshot, &session.operation)?;
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
            session.begin_request(operation);
            let diagnostics = match session.diagnostics(path) {
                Ok(value) => value,
                Err(error) => return Err(session.describe_error(error)),
            };
            session.last_used = Instant::now();
            Ok(json!({"diagnostics":diagnostics}))
        }
        "lsp_close" => {
            let candidates: Vec<_> = {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                if request["session_id"].as_str().is_none() {
                    cache.cancel_pending_for(&root);
                }
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
            let mut completed = 0usize;
            let mut cleanup_errors = Vec::new();
            for (id, entry) in candidates {
                let matches_workspace = match entry.try_lock() {
                    Ok(mut session) if session.root == root => {
                        session.closed = true;
                        if let Err(error) = session.shutdown() {
                            cleanup_errors.push(error);
                        }
                        if session.cleanup_complete() {
                            completed += 1;
                        }
                        true
                    }
                    Err(_) if request["session_id"].as_str().is_some() => {
                        return Err("LSP_SESSION_BUSY".into())
                    }
                    Err(_) => {
                        busy_candidates += 1;
                        false
                    }
                    _ => false,
                };
                if matches_workspace {
                    owned.push((id, entry));
                }
            }
            let removed: Vec<_> = {
                let mut cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                owned
                    .iter()
                    .filter_map(|(id, entry)| {
                        if cache
                            .ready
                            .get(id)
                            .is_some_and(|current| Arc::ptr_eq(current, entry))
                        {
                            cache.ready.remove(id)
                        } else {
                            None
                        }
                    })
                    .collect()
            };
            drop(removed);
            drop(owned);
            reap_deferred_cleanup();
            let (pending_starts, pending_cleanup) = {
                let cache = sessions().lock().map_err(|_| "LSP_SESSION_CACHE")?;
                (
                    cache.pending_for(&root, ReservationPhase::Preparing),
                    cache.pending_for(&root, ReservationPhase::Cleanup),
                )
            };
            Ok(json!({
                "closed_sessions":completed,
                "skipped_busy_candidates":busy_candidates,
                "pending_starts":pending_starts,
                "pending_cleanup":pending_cleanup,
                "close_complete":busy_candidates == 0 && pending_starts == 0
                    && pending_cleanup == 0 && cleanup_errors.is_empty(),
                "cleanup_errors":cleanup_errors
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
    fn shutdown_admission_rejects_new_reservations_and_late_publication() {
        let mut cache = SessionCache::default();
        let root = Path::new(r"C:\fixture");
        let operation = new_operation(30);
        let token = cache.reserve_start(root, operation.clone()).unwrap();
        assert!(cache.start_is_active(&token, root));
        cache.shutting_down = true;
        for entry in cache.starting.values() {
            entry.budget.cancel();
        }
        assert!(!cache.start_is_active(&token, root));
        assert_eq!(
            cache.reserve_start(root, new_operation(30)).unwrap_err(),
            "LSP_APPLICATION_SHUTTING_DOWN"
        );
        assert_eq!(
            cache.starting.len(),
            1,
            "cancellation does not release an owned capacity lease"
        );
        cache.release_start(&token);
        assert!(cache.starting.is_empty());
    }

    #[test]
    fn restricted_null_device_child_is_reported_without_raw_stderr() {
        assert!(blocked_null_device_child(
            "failed to open NUL device for child stdin: Access is denied. (os error 5)"
        ));
        assert!(!blocked_null_device_child(
            "cargo metadata: dependency not found"
        ));
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

    /// Keep both the native child and the stdout worker owned on every test
    /// error path. Diagnostic probes must not hang before reaching their timer.
    struct ProbeCapture {
        process: Option<SandboxProcess>,
        worker: Option<thread::JoinHandle<()>>,
        output: mpsc::Receiver<Result<Vec<u8>, String>>,
    }
    impl ProbeCapture {
        fn start(child: SandboxChild) -> Result<Self, String> {
            reap_deferred_cleanup();
            let SandboxChild {
                stdin,
                stdout,
                process,
            } = child;
            drop(stdin);
            let (sender, output) = mpsc::sync_channel(1);
            let mut owner = Self {
                process: Some(process),
                worker: None,
                output,
            };
            owner.worker = Some(
                thread::Builder::new()
                    .name("browseros-lsp-probe-read".into())
                    .spawn(move || {
                        let mut bytes = Vec::new();
                        let result = stdout
                            .take((MAX_FRAME + 1) as u64)
                            .read_to_end(&mut bytes)
                            .map_err(|_| "LSP_PROBE_READ_FAILED".to_string())
                            .and_then(|_| {
                                if bytes.len() > MAX_FRAME {
                                    Err("LSP_PROBE_OUTPUT_LIMIT".into())
                                } else {
                                    Ok(bytes)
                                }
                            });
                        let _ = sender.send(result);
                    })
                    .map_err(|_| "LSP_PROBE_THREAD_START_FAILED")?,
            );
            Ok(owner)
        }
        fn finish(&mut self, timeout: Duration) -> Result<(u32, Vec<u8>, String), String> {
            let deadline = Instant::now() + timeout;
            let mut bytes = None;
            loop {
                if bytes.is_none() {
                    match self.output.try_recv() {
                        Ok(result) => bytes = Some(result?),
                        Err(mpsc::TryRecvError::Empty) => {}
                        Err(mpsc::TryRecvError::Disconnected) => {
                            return Err("LSP_PROBE_READER_CLOSED".into())
                        }
                    }
                }
                let process = self.process.as_ref().ok_or("LSP_PROBE_OWNER_MISSING")?;
                if bytes.is_some()
                    && process.cleanup_complete()
                    && self
                        .worker
                        .as_ref()
                        .is_none_or(|worker| worker.is_finished())
                {
                    join_finished_worker(&mut self.worker)?;
                    let exit = process.exit_code()?.ok_or("LSP_PROBE_EXIT_UNAVAILABLE")?;
                    // cleanup_complete includes stderr EOF, so this is the
                    // complete bounded diagnostic tail, not a race with the pump.
                    return Ok((
                        exit,
                        bytes.take().unwrap(),
                        process.profile().stderr_for_test(),
                    ));
                }
                if Instant::now() >= deadline {
                    let _ = process.terminate();
                    return Err("LSP_PROBE_CLEANUP_TIMEOUT".into());
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    impl Drop for ProbeCapture {
        fn drop(&mut self) {
            if let Some(process) = self.process.take() {
                let _ = process.terminate();
                let _ = join_finished_worker(&mut self.worker);
                if !process.cleanup_complete() || self.worker.is_some() {
                    queue_deferred_cleanup(DeferredCleanup {
                        process,
                        writer_thread: None,
                        reader_thread: self.worker.take(),
                        _reservation: None,
                    });
                }
            }
            reap_deferred_cleanup();
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
        let captured =
            ProbeCapture::start(child).and_then(|mut owner| owner.finish(Duration::from_secs(30)));
        let (exit, out, stderr) = match captured {
            Ok(result) => result,
            Err(error) => {
                eprintln!("::error::LSP_TOKEN_PROBE {error}");
                return;
            }
        };
        let text = String::from_utf8_lossy(&out)
            .replace('\n', " | ")
            .replace('"', " ");
        let head: String = text.chars().take(1200).collect();
        let stderr_head: String = stderr.chars().take(900).collect();
        eprintln!("::error::LSP_TOKEN_PROBE exit={exit} groups={head} stderr={stderr_head}");
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
        let bin = match server.rustc.path().parent() {
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
        let mut owner = match ProbeCapture::start(child) {
            Ok(owner) => owner,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} {error}");
                return;
            }
        };
        let (exit, output, stderr) = match owner.finish(Duration::from_secs(30)) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("::error::LSP_CARGO_PROBE_{label} {error}");
                return;
            }
        };
        let tail = |text: &str| -> String {
            let bytes = text.as_bytes();
            let start = bytes.len().saturating_sub(900);
            String::from_utf8_lossy(&bytes[start..]).replace('\n', " | ")
        };
        let out = String::from_utf8_lossy(&output).into_owned();
        let succeeded = exit == 0;
        if !succeeded {
            if let Some(manifest) = manifest.as_ref() {
                let mut targets = vec![manifest.clone()];
                if let Some(parent) = manifest.parent() {
                    targets.push(parent.to_path_buf());
                    if let Some(package) = parent.parent() {
                        targets.push(package.to_path_buf());
                    }
                }
                for (index, target) in targets.iter().enumerate() {
                    let summary = snapshot_acl_for_test(target);
                    eprintln!("::error::LSP_CARGO_PROBE_{label}_ACL object={index} {summary}");
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
        profile: SandboxProfile,
        command: String,
    ) -> Result<(u32, String, String), String> {
        run_private_command_in_profile_checked(profile, command, || {})
    }

    /// Same as `run_private_command_in_profile`, but runs `check` while the
    /// sandbox process — and therefore its AppContainer profile folder — is
    /// still alive. Dropping the profile calls `DeleteAppContainerProfile` and
    /// removes the private folder, so host-side assertions about staged
    /// snapshot bytes must happen before this function returns.
    fn run_private_command_in_profile_checked<F: FnOnce()>(
        profile: SandboxProfile,
        command: String,
        check: F,
    ) -> Result<(u32, String, String), String> {
        let state = profile.state_dir()?;
        let windows = std::env::var_os("SystemRoot").ok_or("LSP_TEST_SYSTEMROOT_UNAVAILABLE")?;
        let cmd = PathBuf::from(windows).join("System32").join("cmd.exe");
        let tail = command;
        let child = profile.spawn(&cmd, &["/D".into(), "/C".into(), tail], &[], &state)?;
        let mut owner = ProbeCapture::start(child)?;
        let (exit, bytes, stderr) = owner.finish(Duration::from_secs(15))?;
        // Host-side byte/existence checks must run before `process` (and the
        // profile it owns) is dropped and its private folder is removed.
        check();
        Ok((exit, String::from_utf8_lossy(&bytes).into_owned(), stderr))
    }

    /// `cmd.exe` cannot resolve a verbatim `\\?\` path, so lowbox shell
    /// probes must use the plain form of the same private file. Its built-in
    /// commands (notably `type`) also reject `/` as a path separator — a path
    /// built as `dir.join("src/lib.rs")` keeps the forward slash on Windows,
    /// and `type` then fails with "系统找不到指定的文件" even though the file
    /// exists and its package ACE is correct. Normalise both here so the probe
    /// tests the ACL contract instead of cmd.exe's path syntax.
    fn plain_path_for_shell(path: &Path) -> String {
        let text = path.to_string_lossy().into_owned();
        let plain = text.strip_prefix(r"\\?\").unwrap_or(&text);
        plain.replace('/', "\\")
    }

    /// Native, read-only diagnostic metadata for a pinned object. This is
    /// deliberately not the ordinary owner-only policy: these objects use an
    /// AppContainer package ACL, whose actual access checks are exercised below.
    /// No localized command output, account names, paths or raw SIDs are emitted.
    pub(super) fn snapshot_acl_for_test(path: &Path) -> String {
        use std::os::windows::fs::OpenOptionsExt;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SE_FILE_OBJECT};
        use windows_sys::Win32::Security::{
            GetSecurityDescriptorControl, IsValidAcl, IsValidSid, ACL, DACL_SECURITY_INFORMATION,
            OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED,
        };
        struct Descriptor(PSECURITY_DESCRIPTOR);
        impl Drop for Descriptor {
            fn drop(&mut self) {
                unsafe {
                    LocalFree(self.0.cast());
                }
            }
        }
        let file = match fs::OpenOptions::new()
            .access_mode(0x0002_0000) // READ_CONTROL only.
            .share_mode(0x0000_0001 | 0x0000_0002) // No FILE_SHARE_DELETE.
            .custom_flags(0x0200_0000 | 0x0020_0000) // BACKUP_SEMANTICS | OPEN_REPARSE_POINT.
            .open(path)
        {
            Ok(file) => file,
            Err(error) => return format!("native-acl-open:{}", error.raw_os_error().unwrap_or(-1)),
        };
        match file.metadata() {
            Ok(metadata) if metadata.file_attributes() & REPARSE == 0 => {}
            _ => return "native-acl-object-rejected".into(),
        }
        let mut owner: PSID = std::ptr::null_mut();
        let mut acl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle().cast(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                &mut acl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        let descriptor = Descriptor(descriptor);
        if status != 0 || descriptor.0.is_null() {
            return format!("native-acl-read:{status}");
        }
        let owner_valid = !owner.is_null() && unsafe { IsValidSid(owner) } != 0;
        let acl_valid = !acl.is_null() && unsafe { IsValidAcl(acl) } != 0;
        let mut control = 0u16;
        let mut revision = 0u32;
        if unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) } == 0 {
            return "native-acl-control-unavailable".into();
        }
        let ace_count = if acl_valid {
            unsafe { (*acl).AceCount }
        } else {
            0
        };
        format!(
            "native-acl owner_valid={owner_valid} dacl_present={} dacl_valid={acl_valid} protected={} ace_count={ace_count}",
            !acl.is_null(), control & SE_DACL_PROTECTED != 0,
        )
    }

    #[test]
    fn private_snapshot_requires_own_package_sid_and_rejects_other_profiles() {
        let root = std::env::temp_dir().join(format!(
            "browseros-private-snapshot-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='private_snapshot'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn private_snapshot_marker() {}\n",
        )
        .unwrap();
        // Separate native smoke controls distinguish shell parsing, private
        // writable state, and read-only snapshot ACL. A failed "type" command
        // alone is not evidence that the snapshot bytes were never created.
        let command_profile = SandboxProfile::create().expect("cmd smoke profile");
        let (shell_exit, shell_out, shell_error) =
            run_private_command_in_profile(command_profile, "echo __BOS_LOWBOX_SHELL_OK__".into())
                .expect("cmd smoke launched inside AppContainer");
        assert_eq!(
            shell_exit, 0,
            "SANDBOX_SHELL_CONTROL_FAILED stderr={shell_error}"
        );
        assert!(
            shell_out.contains("__BOS_LOWBOX_SHELL_OK__"),
            "SANDBOX_SHELL_CONTROL_MISSING_OUTPUT"
        );
        let state_profile = SandboxProfile::create().expect("state read profile");
        let state = state_profile.state_dir().expect("writable lowbox state");
        let state_file = state_profile
            .stage_state_file(
                std::ffi::OsStr::new("state-read-control.txt"),
                b"__BOS_LOWBOX_STATE_READ_OK__",
            )
            .expect("write sandbox state fixture with the profile owner and DACL");
        let (state_exit, state_out, state_error) = run_private_command_in_profile(
            state_profile,
            format!("type \"{}\"", plain_path_for_shell(&state_file)),
        )
        .expect("state read control launched");
        assert_eq!(
            state_exit,
            0,
            "SANDBOX_STATE_READ_FAILED stderr={state_error} dirACL={} fileACL={}",
            snapshot_acl_for_test(&state),
            snapshot_acl_for_test(&state_file)
        );
        assert!(
            state_out.contains("__BOS_LOWBOX_STATE_READ_OK__"),
            "SANDBOX_STATE_READ_MISSING_OUTPUT"
        );
        let owner = SandboxProfile::create().expect("owner AppContainer profile");
        let copy = snapshot(&root, &owner).expect("private source copy ACL");
        let copied_file = copy.join("src/lib.rs");
        // Prove a standalone lowbox file read works before testing shell
        // command chaining and cross-package denial.
        let baseline = SandboxProfile::create().expect("standalone read profile");
        let baseline_copy = snapshot(&root, &baseline).expect("standalone source copy");
        let baseline_file = baseline_copy.join("src/lib.rs");
        let (baseline_exit, baseline_out, baseline_error) = run_private_command_in_profile(
            baseline,
            format!("type \"{}\"", plain_path_for_shell(&baseline_file)),
        )
        .expect("standalone lowbox source read launched");
        assert_eq!(baseline_exit, 0,
            "own lowbox source file read failed: stdout={baseline_out} stderr={baseline_error}; verbatim={} dir ACL={}, file ACL={}",
            baseline_file.to_string_lossy().starts_with(r"\\?\"),
            snapshot_acl_for_test(&baseline_copy),
            snapshot_acl_for_test(&baseline_file));
        assert!(
            baseline_out.contains("private_snapshot_marker"),
            "own lowbox did not output source text"
        );
        // Negative control first, while owner profile is still alive.
        let other = SandboxProfile::create().expect("different AppContainer profile");
        let other_copy = snapshot(&root, &other).expect("other package own source copy");
        // Positive control for the SAME other profile before probing owner
        // files. A nonzero exit alone can be a loader/runtime failure.
        let own_file = other_copy.join("src/lib.rs");
        let command = format!(
            "type \"{}\" & type \"{}\"",
            plain_path_for_shell(&own_file),
            plain_path_for_shell(&copied_file)
        );
        let (other_exit, other_text, other_error) = run_private_command_in_profile(other, command)
            .expect("other package launched and attempted both file reads");
        assert_eq!(other_text.matches("private_snapshot_marker").count(), 1,
            "other package should read only its own copy: stdout={other_text} stderr={other_error}, own dir ACL={}, own file ACL={}",
            snapshot_acl_for_test(&other_copy),
            snapshot_acl_for_test(&own_file));
        assert_ne!(
            other_exit, 0,
            "unrelated package unexpectedly read owner source: {other_text}"
        );
        let own_project = copy.join("rust-project.json");
        let own_command = format!(
            "type \"{}\" & type \"{}\"",
            plain_path_for_shell(&copied_file),
            plain_path_for_shell(&own_project)
        );
        let (owner_exit, owner_text, owner_error) =
            run_private_command_in_profile(owner, own_command)
                .expect("owner profile launched to read source and project graph");
        assert_eq!(owner_exit, 0,
            "own lowbox could not read source/manifest: stdout={owner_text} stderr={owner_error}, project ACL={}",
            snapshot_acl_for_test(&own_project));
        assert!(owner_text.contains("private_snapshot_marker"));
        assert!(
            owner_text.contains("root_module"),
            "own lowbox could not open reviewed project graph"
        );

        // Verify the authorized AppContainer does not retain directory
        // Modify rights from the old cargo-metadata snapshot contract.
        let writer = SandboxProfile::create().expect("write-negative profile");
        let read_only_copy = snapshot(&root, &writer).expect("read-only source copy");
        let source_file = read_only_copy.join("src/lib.rs");
        // Directory create and overwrite are independent Windows ACL rights.
        // A read-only source directory must not permit creating new files.
        let create = read_only_copy.join("src/unreviewed-created.rs");
        let before = fs::read(&source_file).unwrap();
        let write = format!("echo changed > \"{}\"", plain_path_for_shell(&source_file));
        // Host-side byte/existence checks run in the callback while the
        // profile still owns this private folder; after the call returns the
        // profile is dropped and the whole folder is removed.
        let (write_exit, _, _) = run_private_command_in_profile_checked(writer, write, || {
            assert_eq!(
                fs::read(&source_file).unwrap(),
                before,
                "host-owned snapshot bytes changed under sandbox token"
            );
            assert!(
                !create.exists(),
                "unreviewed source file exists in the first copy"
            );
        })
        .expect("spawn lowbox denied-write fixture");
        assert_ne!(
            write_exit, 0,
            "AppContainer unexpectedly modified a source file"
        );
        // The writer profile was consumed by the first probe. Use a separate
        // profile and its own private source copy for the create-negative check.
        let second_writer = SandboxProfile::create().expect("create-negative profile");
        let second_copy = snapshot(&root, &second_writer).expect("read-only create-negative copy");
        let second_create = second_copy.join("src/unreviewed-created.rs");
        let (create_exit, _, _) = run_private_command_in_profile_checked(
            second_writer,
            format!(
                "echo injected > \"{}\"",
                plain_path_for_shell(&second_create)
            ),
            || {
                assert!(
                    !second_create.exists(),
                    "unauthorized snapshot file was created"
                )
            },
        )
        .expect("spawn lowbox create-negative fixture");
        assert_ne!(create_exit, 0, "AppContainer created a new source file");
        assert!(
            fs::read_to_string(root.join("src/lib.rs"))
                .unwrap()
                .contains("private_snapshot_marker"),
            "the real host workspace was modified by sandbox operation"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_initialize_graph_is_inline_and_refuses_external_roots() {
        let root =
            std::env::temp_dir().join(format!("browseros-lsp-inline-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn local_symbol() {}\n").unwrap();
        let manifest = write_reviewed_rust_project(&root).unwrap();
        let accepted = reviewed_project_for_initialize(&root).unwrap();
        assert_eq!(accepted["crates"].as_array().unwrap().len(), 1);
        assert_eq!(accepted["crates"][0]["target"], REVIEWED_WINDOWS_TARGET);
        assert_eq!(accepted["crates"][0]["cfg"], json!(REVIEWED_WINDOWS_CFG));
        assert!(accepted["crates"][0]["root_module"]
            .as_str()
            .unwrap()
            .ends_with("src\\lib.rs"));
        // The host may have altered the generated manifest after snapshot
        // creation; it must not inject a new unreviewed file in the LSP graph.
        let outside = root.parent().unwrap().join(format!(
            "browseros-lsp-unreviewed-{}.rs",
            uuid::Uuid::new_v4()
        ));
        fs::write(&outside, "pub fn secret() {}\n").unwrap();
        let mut runnable = accepted.clone();
        runnable["runnables"] = json!([{
            "program":"cmd.exe", "args":["/c","echo insecure"], "kind":"flycheck"
        }]);
        fs::write(&manifest, serde_json::to_vec(&runnable).unwrap()).unwrap();
        assert_eq!(
            reviewed_project_for_initialize(&root).unwrap_err(),
            "LSP_PROJECT_INVALID"
        );
        let mut foreign_cfg = accepted.clone();
        foreign_cfg["crates"][0]["cfg"] = json!(["unix"]);
        fs::write(&manifest, serde_json::to_vec(&foreign_cfg).unwrap()).unwrap();
        assert_eq!(
            reviewed_project_for_initialize(&root).unwrap_err(),
            "LSP_PROJECT_INVALID"
        );
        let mut foreign_target = accepted.clone();
        foreign_target["crates"][0]["target"] = json!("x86_64-unknown-linux-gnu");
        fs::write(&manifest, serde_json::to_vec(&foreign_target).unwrap()).unwrap();
        assert_eq!(
            reviewed_project_for_initialize(&root).unwrap_err(),
            "LSP_PROJECT_INVALID"
        );
        let mut hostile = accepted;
        hostile["crates"][0]["root_module"] = json!(outside.to_str().unwrap());
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
        let root =
            std::env::temp_dir().join(format!("browseros-linked-project-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='reviewed'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        let project = write_reviewed_rust_project(&root).unwrap();
        let parsed: Value = serde_json::from_slice(&fs::read(&project).unwrap()).unwrap();
        assert_eq!(parsed["crates"][0]["edition"], "2024");
        assert_eq!(parsed["crates"][0]["deps"], json!([]));
        assert!(parsed["crates"][0]["root_module"]
            .as_str()
            .unwrap()
            .ends_with("src\\lib.rs"));
        assert_eq!(parsed["crates"].as_array().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reviewed_rust_project_rejects_snapshot_without_rust_sources() {
        let root =
            std::env::temp_dir().join(format!("browseros-linked-empty-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        assert_eq!(
            write_reviewed_rust_project(&root).unwrap_err(),
            "LSP_PROJECT_ROOT_LIMIT"
        );
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
            "browseros-legacy-toolchain-probe-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='legacy_probe'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
        sandbox_token_probe();
        toolchain_probe(
            "LOCATE",
            "cargo",
            &["locate-project".to_owned(), "--workspace".to_owned()],
            &root,
            true,
        );
        toolchain_probe("VERSION", "cargo", &["--version".to_owned()], &root, false);
        toolchain_probe(
            "RUSTC_VERSION",
            "rustc",
            &["--version".to_owned()],
            &root,
            false,
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cfg_peer_missing_or_directory_fails_before_runtime_spawn() {
        let root =
            std::env::temp_dir().join(format!("browseros-cfg-peer-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let analyzer = root.join("rust-analyzer.exe");
        fs::write(&analyzer, b"fixture").unwrap();
        assert_eq!(
            rustc_cfg_peer(&analyzer).unwrap_err(),
            "LSP_RUSTC_CFG_PEER_MISSING"
        );
        fs::create_dir(root.join("rustc.exe")).unwrap();
        assert_eq!(
            rustc_cfg_peer(&analyzer).unwrap_err(),
            "LSP_RUSTC_CFG_PEER_UNSAFE"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cfg_probe_does_not_advertise_cargo_or_compilation() {
        let contract = lsp_toolchain_contract();
        assert_eq!(contract["rustc_cfg_probe"], "required");
        for field in [
            "cargo_enabled",
            "build_scripts_enabled",
            "proc_macros_enabled",
            "compilation_verified",
        ] {
            assert_eq!(contract[field], false);
        }
    }

    #[test]
    fn pull_diagnostics_require_negotiation_and_a_complete_report() {
        for capabilities in [json!({}), json!({"diagnosticProvider":false})] {
            assert_eq!(
                diagnostic_identifier(&json!({"capabilities":capabilities})).unwrap_err(),
                "LSP_DIAGNOSTICS_UNSUPPORTED"
            );
        }
        let provider = json!({
            "identifier":"rust-analyzer",
            "interFileDependencies":true,
            "workspaceDiagnostics":false
        });
        assert_eq!(
            diagnostic_identifier(&json!({"capabilities":{"diagnosticProvider":provider}}))
                .unwrap(),
            Some("rust-analyzer".into())
        );
        let anonymous = json!({"interFileDependencies":false,"workspaceDiagnostics":false});
        assert_eq!(
            diagnostic_identifier(&json!({"capabilities":{"diagnosticProvider":anonymous}}))
                .unwrap(),
            None
        );
        for invalid in [
            json!({}),
            json!({
                "identifier":17,"interFileDependencies":true,"workspaceDiagnostics":false
            }),
        ] {
            assert_eq!(
                diagnostic_identifier(&json!({"capabilities":{"diagnosticProvider":invalid}}))
                    .unwrap_err(),
                "LSP_DIAGNOSTICS_CAPABILITY_INVALID"
            );
        }
        let report = json!({"kind":"full","items":[]});
        let reply = json!({"jsonrpc":"2.0","id":7,"result":report});
        let accepted = rpc_response(&reply, 7).unwrap().unwrap();
        assert_eq!(full_diagnostic_items(accepted).unwrap(), json!([]));
        for mismatched in [
            json!({"jsonrpc":"2.0","id":6,"result":report}),
            json!({"jsonrpc":"2.0","id":"7","result":report}),
            json!({"jsonrpc":"2.0","method":"server/request","id":7,"params":{}}),
        ] {
            assert_eq!(rpc_response(&mismatched, 7).unwrap(), None);
        }
        for malformed in [
            json!({"jsonrpc":"2.0","id":7}),
            json!({"id":7,"result":report}),
        ] {
            assert_eq!(
                rpc_response(&malformed, 7).unwrap_err(),
                "LSP_RESPONSE_INVALID"
            );
        }
        let failure = json!({"jsonrpc":"2.0","id":7,"error":{"code":-32603}});
        assert_eq!(rpc_response(&failure, 7).unwrap_err(), "LSP_REMOTE_ERROR");
        let items = json!([{
            "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
            "severity":1,"message":"syntax error"
        }]);
        assert_eq!(
            full_diagnostic_items(json!({"kind":"full","items":items})).unwrap(),
            items
        );
        for report in [
            json!({"kind":"unchanged","resultId":"old-document"}),
            json!({"kind":"full"}),
            json!({"kind":"full","items":null}),
            json!({"items":[]}),
        ] {
            assert_eq!(
                full_diagnostic_items(report).unwrap_err(),
                "LSP_DIAGNOSTICS_REPORT_INVALID"
            );
        }
        let status_event =
            |params| json!({"jsonrpc":"2.0","method":"experimental/serverStatus","params":params});
        for health in ["ok", "warning"] {
            let loading = status_event(json!({"health":health,"quiescent":false}));
            let ready = status_event(json!({"health":health,"quiescent":true}));
            assert!(!server_status_ready(&loading).unwrap());
            assert!(server_status_ready(&ready).unwrap());
        }
        let failed = status_event(json!({"health":"error","quiescent":true}));
        assert_eq!(
            server_status_ready(&failed).unwrap_err(),
            "LSP_SERVER_NOT_READY"
        );
        for status in [
            json!({"health":"ok"}),
            json!({"health":"ok","quiescent":"true"}),
            json!({"health":"unknown","quiescent":true}),
        ] {
            assert_eq!(
                server_status_ready(&status_event(status)).unwrap_err(),
                "LSP_SERVER_STATUS_INVALID"
            );
        }
        let mut missing_version = status_event(json!({"health":"ok","quiescent":true}));
        missing_version.as_object_mut().unwrap().remove("jsonrpc");
        let mut request = status_event(json!({"health":"ok","quiescent":true}));
        request["id"] = json!(7);
        for malformed in [missing_version, request] {
            assert_eq!(
                server_status_ready(&malformed).unwrap_err(),
                "LSP_SERVER_STATUS_INVALID"
            );
        }
    }

    #[test]
    fn semantic_ready_requires_a_loaded_reviewed_crate() {
        assert!(
            !reviewed_crate_graph_ready(&json!("digraph rust_analyzer_crate_graph {\n}\n"))
                .unwrap()
        );
        assert!(reviewed_crate_graph_ready(&json!(
            "digraph rust_analyzer_crate_graph {\n  _0[label=\"reviewed_crate_0\"];\n}\n"
        ))
        .unwrap());
        for invalid in [json!(null), json!({}), json!("not a crate graph")] {
            assert_eq!(
                reviewed_crate_graph_ready(&invalid).unwrap_err(),
                "LSP_CRATE_GRAPH_INVALID"
            );
        }
    }

    #[test]
    fn immutable_snapshot_registration_rejects_non_watcher_operations() {
        let watch = json!({
            "jsonrpc":"2.0","id":7,"method":"client/registerCapability",
            "params":{"registrations":[{
                "id":"workspace/didChangeWatchedFiles",
                "method":"workspace/didChangeWatchedFiles",
                "registerOptions":{"watchers":[{"globPattern":"**/*.rs"}]}
            }]}
        });
        assert!(immutable_snapshot_watcher_registration(&watch));
        let mut foreign = watch.clone();
        foreign["params"]["registrations"][0]["method"] = json!("workspace/executeCommand");
        assert!(!immutable_snapshot_watcher_registration(&foreign));
        let mut missing_watchers = watch.clone();
        missing_watchers["params"]["registrations"][0]["registerOptions"] = json!({});
        assert!(!immutable_snapshot_watcher_registration(&missing_watchers));
        let mut absent_id = watch.clone();
        absent_id.as_object_mut().unwrap().remove("id");
        assert!(!immutable_snapshot_watcher_registration(&absent_id));
        let unregistration = json!({
            "jsonrpc":"2.0","id":8,"method":"client/unregisterCapability",
            "params":{"unregisterations":[{
                "id":"workspace/didChangeWatchedFiles",
                "method":"workspace/didChangeWatchedFiles"
            }]}
        });
        assert!(immutable_snapshot_watcher_registration(&unregistration));
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
        fs::write(
            root.join("src/lib.rs"),
            "#[cfg(all(windows, target_os = \"windows\", target_pointer_width = \"64\"))] pub fn helper() -> i32 { 42 }\npub fn fixture() -> i32 { helper() }\nmod broken;\n",
        )
        .unwrap();
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
        assert_eq!(started["toolchain"]["rustc_cfg_probe"], "required");
        assert_eq!(started["toolchain"]["cargo_enabled"], false);
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
        // The cfg-gated helper must resolve using real rustc metadata inside
        // this AppContainer, without a Cargo build or sysroot discovery.
        // A failed cfg probe cannot pass as a no-metadata language service.
        // The returned file URI must be
        // remapped from private snapshot to the approved host workspace.
        let line = "pub fn fixture() -> i32 { helper() }";
        let character = line.find("helper").expect("fixture call position") as u64 + 2;
        let hover = acceptance(
            "HOVER",
            execute(
                &root,
                json!({
                    "action":"lsp_request", "approved":true, "session_id":id,
                    "method":"textDocument/hover",
                    "params":{"textDocument":{"uri":uri.clone()},
                        "position":{"line":1,"character":character}},
                    "timeout_seconds":120
                }),
            ),
        );
        if hover["result"].is_null() {
            // A syntax-only documentSymbol response can pass even when the
            // private crate graph never loaded. Ask the SAME pinned analyzer
            // for direct graph evidence before reporting a semantic failure.
            // These diagnostics only execute in the Windows native test.
            let entry = get_session(&root.canonicalize().unwrap(), id).unwrap();
            let mut native = entry.lock().unwrap();
            native.begin_request(new_operation(30));
            let private_uri = uri_for(&native.snapshot.join("src/lib.rs")).unwrap();
            for (label, method, params) in [
                (
                    "ANALYZER_STATUS",
                    "rust-analyzer/analyzerStatus",
                    json!({"textDocument":{"uri":private_uri}}),
                ),
                (
                    "CRATE_GRAPH",
                    "rust-analyzer/viewCrateGraph",
                    json!({"full":false}),
                ),
            ] {
                match native.rpc_next(method, params) {
                    Ok(response) => {
                        let raw = response.as_str().unwrap_or("UNEXPECTED_NON_STRING");
                        let sanitized = raw.replace(
                            &native.snapshot.to_string_lossy().to_string(),
                            "[private-snapshot]",
                        );
                        let summary = sanitized.lines().take(60).collect::<Vec<_>>().join("\n");
                        eprintln!("LSP_SEMANTIC_PROBE_{label}={summary}");
                    }
                    Err(error) => eprintln!("LSP_SEMANTIC_PROBE_{label}_ERROR={error}"),
                }
            }
        }
        assert!(!hover["result"].is_null(), "hover returned null: {hover}");
        assert!(
            hover["result"].to_string().contains("helper"),
            "hover did not resolve fixture helper: {hover}"
        );
        let definition = acceptance(
            "DEFINITION",
            execute(
                &root,
                json!({
                    "action":"lsp_request", "approved":true, "session_id":id,
                    "method":"textDocument/definition",
                    "params":{"textDocument":{"uri":uri.clone()},
                        "position":{"line":1,"character":character}},
                    "timeout_seconds":120
                }),
            ),
        );
        assert!(
            !definition["result"].is_null(),
            "definition returned null: {definition}"
        );
        assert!(
            definition["result"].to_string().contains("lib.rs"),
            "definition did not map back into approved workspace: {definition}"
        );
        assert!(
            !definition["result"].to_string().contains("snapshot"),
            "definition leaked the private snapshot path: {definition}"
        );
        let broken = acceptance(
            "SYNTAX_DIAGNOSTICS",
            execute(
                &root,
                json!({
                    "action":"lsp_diagnostics", "approved":true, "session_id":id,
                    "path":"src/broken.rs", "timeout_seconds":120
                }),
            ),
        );
        assert!(
            broken["diagnostics"]
                .as_array()
                .is_some_and(|d| !d.is_empty()),
            "broken Rust code should publish parser diagnostics: {broken}"
        );

        // Exercise one live document through changed and unchanged contents.
        // Repeated diagnostics still require a current full response; they must
        // neither reopen the document nor advance a version that was not sent.
        let synchronized_version = || {
            let entry = get_session(&root.canonicalize().unwrap(), id).unwrap();
            let session = entry.lock().unwrap();
            let key = uri_for(&session.snapshot.join("src/broken.rs"))
                .unwrap()
                .to_ascii_lowercase();
            session.documents.get(&key).unwrap().version
        };
        let first_version = synchronized_version();
        assert_eq!(first_version, 1);
        let diagnose_broken = |stage| {
            acceptance(
                stage,
                execute(
                    &root,
                    json!({
                        "action":"lsp_diagnostics", "approved":true, "session_id":id,
                        "path":"src/broken.rs", "timeout_seconds":120
                    }),
                ),
            )
        };
        fs::write(root.join("src/broken.rs"), "pub fn repaired() {}\n").unwrap();
        let valid = diagnose_broken("VALID_DIAGNOSTICS");
        assert_eq!(valid["diagnostics"], json!([]));
        assert_eq!(synchronized_version(), first_version + 1);
        fs::write(root.join("src/broken.rs"), "pub fn broken_again( {\n").unwrap();
        let invalid = diagnose_broken("CHANGED_SYNTAX_DIAGNOSTICS");
        assert!(
            invalid["diagnostics"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "changed invalid Rust must produce diagnostics: {invalid}"
        );
        let changed_version = synchronized_version();
        assert_eq!(changed_version, first_version + 2);
        let repeated = diagnose_broken("REPEATED_SYNTAX_DIAGNOSTICS");
        assert_eq!(repeated["diagnostics"], invalid["diagnostics"]);
        assert_eq!(synchronized_version(), changed_version);
        fs::write(root.join("src/broken.rs"), "pub fn repaired_again() {}\n").unwrap();
        let cleared = diagnose_broken("CLEARED_SYNTAX_DIAGNOSTICS");
        assert_eq!(cleared["diagnostics"], json!([]));
        assert_eq!(synchronized_version(), changed_version + 1);

        let closed = acceptance(
            "CLOSE",
            execute(&root, json!({"action":"lsp_close","session_id":id})),
        );
        assert_eq!(closed["closed_sessions"], 1);
        assert_eq!(closed["close_complete"], true);
        assert_eq!(closed["pending_starts"], 0);
        assert_eq!(closed["pending_cleanup"], 0);
        assert_eq!(closed["cleanup_errors"], json!([]));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_one_workspace_does_not_cancel_another_workspace_start() {
        let first = PathBuf::from(r"C:\isolated\workspace-a");
        let second = PathBuf::from(r"C:\isolated\workspace-b");
        let mut cache = SessionCache::default();
        let first_start = cache.reserve_start(&first, new_operation(30)).unwrap();
        let second_start = cache.reserve_start(&second, new_operation(30)).unwrap();
        cache.cancel_pending_for(&first);
        assert!(!cache.start_is_active(&first_start, &first));
        assert!(cache.start_is_active(&second_start, &second));
        assert_eq!(
            cache.starting.len(),
            2,
            "cancelled preparation still owns capacity"
        );
        assert_eq!(
            cache.reserve_start(&second, new_operation(30)).unwrap_err(),
            "LSP_SESSION_LIMIT"
        );
        cache.starting.remove(&first_start);
        cache.starting.remove(&second_start);
        assert!(cache.starting.is_empty());
    }

    #[test]
    fn cancelled_and_cleanup_reservations_do_not_release_capacity() {
        let root = PathBuf::from(r"C:\isolated\capacity");
        let mut cache = SessionCache::default();
        let first = cache.reserve_start(&root, new_operation(30)).unwrap();
        let second = cache.reserve_start(&root, new_operation(30)).unwrap();
        cache.cancel_pending_for(&root);
        assert_eq!(cache.pending_for(&root, ReservationPhase::Preparing), 2);
        assert_eq!(
            cache.reserve_start(&root, new_operation(30)).unwrap_err(),
            "LSP_SESSION_LIMIT"
        );
        cache.starting.get_mut(&first).unwrap().phase = ReservationPhase::Cleanup;
        assert_eq!(
            cache.reserve_start(&root, new_operation(30)).unwrap_err(),
            "LSP_SESSION_LIMIT"
        );
        cache.starting.remove(&first);
        let replacement = cache.reserve_start(&root, new_operation(30)).unwrap();
        assert!(!cache.start_is_active(&second, &root));
        assert!(cache.start_is_active(&replacement, &root));
    }

    #[test]
    fn failed_native_start_keeps_capacity_until_cleanup_guard_drops() {
        let root = PathBuf::from(r"C:\isolated\failed-native-start");
        let mut cache = SessionCache::default();
        let operation = new_operation(30);
        let native_owner = operation.hold_cleanup();
        let failed = cache.reserve_start(&root, operation.clone()).unwrap();
        operation.cancel();
        cache.release_start(&failed);
        assert_eq!(cache.pending_for(&root, ReservationPhase::Cleanup), 1);
        let other = cache.reserve_start(&root, new_operation(30)).unwrap();
        assert_eq!(
            cache.reserve_start(&root, new_operation(30)).unwrap_err(),
            "LSP_SESSION_LIMIT"
        );
        cache.reap_released();
        assert!(cache.starting.contains_key(&failed));
        drop(native_owner);
        cache.reap_released();
        assert!(!cache.starting.contains_key(&failed));
        assert!(cache.starting.contains_key(&other));
        assert!(cache.reserve_start(&root, new_operation(30)).is_ok());
    }

    #[test]
    fn released_native_cleanup_slot_is_reaped_without_another_request() {
        let root = PathBuf::from(format!(r"C:\isolated\cleanup-{}", uuid::Uuid::new_v4()));
        let operation = new_operation(30);
        let native_owner = operation.hold_cleanup();
        let reservation = StartReservation::acquire(&root, operation.clone()).unwrap();
        let token = reservation.token;
        operation.cancel();
        drop(reservation);
        assert!(sessions().lock().unwrap().starting.contains_key(&token));
        drop(native_owner);
        // No execute/acquire call drives cleanup here: the owned worker must
        // observe the native completion barrier and release capacity itself.
        let until = Instant::now() + Duration::from_secs(3);
        while sessions().lock().unwrap().starting.contains_key(&token) && Instant::now() < until {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!sessions().lock().unwrap().starting.contains_key(&token));
        assert!(CLEANUP_WORKER.get().unwrap().lock().unwrap().is_some());
    }

    #[test]
    fn cancelled_or_expired_preparation_cannot_enter_snapshot_io() {
        let fixture = std::env::temp_dir().join(format!("lsp-budget-{}", uuid::Uuid::new_v4()));
        let source = fixture.join("source");
        let destination = fixture.join("copy");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(source.join("lib.rs"), b"fn fixture() {}").unwrap();
        let cancelled = new_operation(30);
        cancelled.cancel();
        let error = copy_snapshot_tree(
            &source,
            &source,
            &destination,
            0,
            &mut SnapshotBudget::new(),
            &cancelled,
        )
        .unwrap_err();
        assert_eq!(error, "LSP_OPERATION_CANCELLED");
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
        let expired = OperationBudget::new(Duration::ZERO);
        assert_eq!(
            check_operation(&expired).unwrap_err(),
            "LSP_OPERATION_TIMEOUT"
        );
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn snapshot_counts_ignored_entries_and_empty_directories() {
        let fixture = std::env::temp_dir().join(format!("lsp-entries-{}", uuid::Uuid::new_v4()));
        let source = fixture.join("source");
        let destination = fixture.join("copy");
        fs::create_dir_all(source.join("empty")).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(source.join(".ignored"), b"not a copied source").unwrap();
        fs::write(source.join("image.bin"), b"not a copied source").unwrap();
        let mut budget = SnapshotBudget::new();
        copy_snapshot_tree(
            &source,
            &source,
            &destination,
            0,
            &mut budget,
            &new_operation(30),
        )
        .unwrap();
        assert_eq!(budget.entries, 4); // root + directory + two excluded files
        assert_eq!(budget.files, 0);
        assert_eq!(budget.read_bytes, 0);
        let other = fixture.join("limited");
        fs::create_dir(&other).unwrap();
        let mut limit = SnapshotBudget::new();
        limit.entries = MAX_FILES * 2;
        assert_eq!(
            copy_snapshot_tree(&source, &source, &other, 0, &mut limit, &new_operation(30),)
                .unwrap_err(),
            "LSP_SNAPSHOT_ENTRY_LIMIT"
        );
        assert_eq!(fs::read_dir(&other).unwrap().count(), 0);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn actual_snapshot_bytes_obey_remaining_read_and_write_capacity() {
        let fixture = std::env::temp_dir().join(format!("lsp-bytes-{}", uuid::Uuid::new_v4()));
        let source = fixture.join("source");
        let destination = fixture.join("copy");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(source.join("lib.rs"), vec![b'x'; 64]).unwrap();
        let mut budget = SnapshotBudget::new();
        budget.read_bytes = MAX_TOTAL_BYTES - 8;
        assert!(copy_snapshot_tree(
            &source,
            &source,
            &destination,
            0,
            &mut budget,
            &new_operation(30),
        )
        .is_err());
        assert_eq!(budget.written_bytes, 0);
        assert!(!destination.join("lib.rs").exists());
        let mut writes = SnapshotBudget::new();
        writes.written_bytes = MAX_TOTAL_BYTES - 3;
        assert_eq!(
            write_snapshot_bytes(
                &destination.join("bounded.rs"),
                b"12345678",
                &mut writes,
                &new_operation(30),
            )
            .unwrap_err(),
            "LSP_SNAPSHOT_LIMIT"
        );
        assert_eq!(
            fs::metadata(destination.join("bounded.rs")).unwrap().len(),
            3
        );
        assert_eq!(writes.written_bytes, MAX_TOTAL_BYTES);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn unfinished_io_worker_stays_owned_until_it_can_be_joined() {
        let (release, waiting) = mpsc::channel();
        let (started, ready) = mpsc::channel();
        let mut handle = Some(thread::spawn(move || {
            started.send(()).unwrap();
            let _ = waiting.recv();
        }));
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!join_finished_worker(&mut handle).unwrap());
        assert!(
            handle.is_some(),
            "pending cleanup must retain the JoinHandle"
        );
        release.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while handle.as_ref().is_some_and(|thread| !thread.is_finished()) && Instant::now() < until
        {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(join_finished_worker(&mut handle).unwrap());
        assert!(handle.is_none());
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
