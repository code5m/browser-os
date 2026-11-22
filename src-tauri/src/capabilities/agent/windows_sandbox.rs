//! Windows AppContainer process isolation for explicitly approved MCP/LSP tools.
//! No network capabilities are granted. Only the selected runtime and an
//! optional private snapshot are exposed; the real workspace is never mounted.
#![cfg(windows)]

use crate::windows_process::CREATE_SUSPENDED;
mod lifecycle;
pub(crate) use lifecycle::{
    initialize_recovery_store, recover_abandoned_profiles, recovery_status, retry_recovery,
    request_shutdown, shutdown_until, RecoveryReport, NativeIoOwner,
};
pub use browseros_platform_security::windows::OperationBudget;
use browseros_platform_security::windows::{
    AppContainerAccess, AppContainerDirectory, AppContainerPolicy,
    PinnedReadDirectory, PinnedReadFile, PrivateTreeOptions, OperationCleanupGuard, NamedPrivateJob, file_identity,
};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::mpsc::{SyncSender, TrySendError};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    CloseHandle, LocalFree, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, GetAppContainerFolderPath,
};
use windows_sys::Win32::Security::{
    FreeSid, GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
    SECURITY_CAPABILITIES, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess, GetExitCodeProcess,
    InitializeProcThreadAttributeList, OpenProcessToken,
    UpdateProcThreadAttribute, WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT,
    EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROC_THREAD_ATTRIBUTE_JOB_LIST, ResumeThread,
    STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

fn win32_error(operation: &str) -> String {
    format!("{operation}:{}", std::io::Error::last_os_error())
}
fn wide(value: &std::ffi::OsStr) -> Result<Vec<u16>, String> {
    if value.encode_wide().any(|unit| unit == 0) {
        return Err("SANDBOX_NUL_PATH".into());
    }
    Ok(value.encode_wide().chain(Some(0)).collect())
}
fn wide_text(value: &str) -> Result<Vec<u16>, String> {
    wide(std::ffi::OsStr::new(value))
}
/// Side-effect-free capability preflight used by the LSP status API.
/// This only checks prerequisites; it is NOT proof that a sandbox can launch.
pub(super) fn sandbox_prerequisites_available() -> bool {
    system_binary("cmd.exe").is_ok()
        && current_user_sid().is_ok()
        && lifecycle::admission_available()
}

pub(super) fn no_reparse_path(path: &Path) -> Result<(), String> {
    use std::path::{Component, Prefix};
    if !path.is_absolute()
        || !matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
    {
        return Err("SANDBOX_RUNTIME_NOT_ABSOLUTE".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_)) || !current.has_root() {
            continue;
        }
        let metadata =
            std::fs::symlink_metadata(&current).map_err(|_| "SANDBOX_PATH_INSPECTION_FAILED")?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err("SANDBOX_REPARSE_DENIED".into());
        }
    }
    Ok(())
}
fn system_binary(name: &str) -> Result<PathBuf, String> {
    let mut buffer = [0u16; 512];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 || length as usize >= buffer.len() {
        return Err("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE".into());
    }
    let path = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize])).join(name);
    if !path.is_file() {
        return Err("SANDBOX_SYSTEM_TOOL_UNAVAILABLE".into());
    }
    Ok(path)
}
fn current_user_sid() -> Result<String, String> {
    struct Token(HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0
        || token.is_null()
    {
        return Err("SANDBOX_USER_SID_UNAVAILABLE".into());
    }
    let token = Token(token);
    let mut required = 0u32;
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required < std::mem::size_of::<TOKEN_USER>() as u32 || required > 65536 {
        return Err("SANDBOX_USER_SID_INVALID".into());
    }
    let words = (required as usize).div_ceil(std::mem::size_of::<usize>());
    let mut buffer = vec![0usize; words];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err("SANDBOX_USER_SID_UNAVAILABLE".into());
    }
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    if sid.is_null() {
        return Err("SANDBOX_USER_SID_INVALID".into());
    }
    let mut sid_text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut sid_text) } == 0 || sid_text.is_null() {
        return Err("SANDBOX_USER_SID_UNAVAILABLE".into());
    }
    let mut len = 0usize;
    unsafe {
        while len < 256 && *sid_text.add(len) != 0 {
            len += 1;
        }
    }
    let value = if len < 256 {
        unsafe { String::from_utf16(std::slice::from_raw_parts(sid_text, len)) }
            .map_err(|_| "SANDBOX_USER_SID_INVALID".to_string())
    } else {
        Err("SANDBOX_USER_SID_INVALID".into())
    };
    unsafe {
        LocalFree(sid_text as _);
    }
    let value = value?;
    let valid = value.strip_prefix("S-1-").is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix.split('-').all(|component| {
                !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
            })
    });
    if !valid {
        return Err("SANDBOX_USER_SID_INVALID".into());
    }
    Ok(value)
}

const PRIVATE_TREE_MAX_ENTRIES: usize = 8192;
const PRIVATE_TREE_MAX_DEPTH: usize = 32;

const MAX_RUNTIME_FILE: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_RUNTIME: u64 = 384 * 1024 * 1024;
const STDERR_TAIL: usize = 8 * 1024;

#[derive(Default)]
struct StderrDiagnostic {
    tail: Vec<u8>,
    code: Option<&'static str>,
}
impl StderrDiagnostic {
    fn append(&mut self, bytes: &[u8]) {
        self.tail.extend_from_slice(bytes);
        if self.tail.len() > STDERR_TAIL {
            let excess = self.tail.len() - STDERR_TAIL;
            self.tail.drain(..excess);
        }
        let lower = String::from_utf8_lossy(&self.tail).to_ascii_lowercase();
        let denied = ["access is denied", "access denied", "permission denied", "os error 5"]
            .iter().any(|phrase| lower.contains(phrase));
        if lower.contains("failed to open nul device for child") && denied {
            self.code = Some("LSP_CHILD_NULL_DEVICE_DENIED");
        } else if self.code.is_none() && lower.contains("rustc")
            && ["--print cfg", "print=cfg", "target-spec-json", "data-layout", "rustc cfg"]
                .iter().any(|phrase| lower.contains(phrase))
            && (denied || lower.contains("failed") || lower.contains("error"))
        {
            self.code = Some("LSP_RUSTC_CFG_PROBE_FAILED");
        }
    }
}

struct ProfileCleanup {
    lease: lifecycle::ProfileLease,
    last_try: Option<Instant>,
    failures: u32,
}
impl ProfileCleanup {
    fn new(lease: lifecycle::ProfileLease) -> Self {
        Self { lease, last_try: None, failures: 0 }
    }
    fn try_finish(&mut self) -> bool {
        if !lifecycle::shutdown_requested()
            && self.last_try.is_some_and(|last| last.elapsed() < Duration::from_secs(
                (1u64 << self.failures.min(5)).min(30)
            ))
        { return false; }
        self.last_try = Some(Instant::now());
        if self.lease.try_cleanup().is_ok() { return true; }
        self.failures = self.failures.saturating_add(1);
        if self.failures == 1 { eprintln!("SANDBOX_PROFILE_CLEANUP_PENDING"); }
        false
    }
}
enum CleanupTask { Process(Box<ProcessResources>), Profile(ProfileCleanup) }
struct CleanupReaper {
    queue: Arc<(Mutex<Vec<CleanupTask>>, Condvar)>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}
static CLEANUP_REAPER: OnceLock<Mutex<Option<Arc<CleanupReaper>>>> = OnceLock::new();
fn cleanup_reaper() -> Result<Arc<CleanupReaper>, String> {
    let mut slot = CLEANUP_REAPER.get_or_init(|| Mutex::new(None))
        .lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(reaper) = slot.as_ref() { return Ok(Arc::clone(reaper)); }
    // Only successful worker creation is cached. A temporary OS thread limit
    // fails before profile creation and can be retried without restarting.
    let created = (|| -> Result<CleanupReaper, String> {
        let queue = Arc::new((Mutex::new(Vec::<CleanupTask>::new()), Condvar::new()));
        let owned = Arc::clone(&queue);
        let worker = thread::Builder::new().name("browseros-sandbox-cleanup".into()).spawn(move || {
            loop {
                let (lock, wake) = &*owned;
                let mut queued = lock.lock().unwrap_or_else(|poison| poison.into_inner());
                while queued.is_empty() {
                    if lifecycle::reaper_may_exit() { return; }
                    queued = wake.wait_timeout(queued, Duration::from_millis(25))
                        .unwrap_or_else(|poison| poison.into_inner()).0;
                }
                let pending = std::mem::take(&mut *queued);
                drop(queued);
                let mut unfinished = Vec::new();
                for task in pending {
                    match task {
                        CleanupTask::Process(resources) => {
                            if resources.is_complete() { drop(resources); }
                            else {
                                let _ = resources.job.terminate();
                                unfinished.push(CleanupTask::Process(resources));
                            }
                        }
                        CleanupTask::Profile(mut profile) => {
                            if !profile.try_finish() { unfinished.push(CleanupTask::Profile(profile)); }
                        }
                    }
                }
                lock.lock().unwrap_or_else(|poison| poison.into_inner()).extend(unfinished);
                thread::sleep(Duration::from_millis(25));
            }
        }).map_err(|_| "SANDBOX_CLEANUP_REAPER_START".to_string())?;
        Ok(CleanupReaper { queue, worker: Mutex::new(Some(worker)) })
    })()?;
    let reaper = Arc::new(created);
    *slot = Some(Arc::clone(&reaper));
    Ok(reaper)
}

fn installed_cleanup_reaper() -> Option<Arc<CleanupReaper>> {
    CLEANUP_REAPER.get().and_then(|slot|
        slot.lock().unwrap_or_else(|poison| poison.into_inner()).as_ref().cloned())
}
fn wake_cleanup_reaper() {
    if let Some(reaper) = installed_cleanup_reaper() { reaper.queue.1.notify_all(); }
}
fn join_cleanup_reaper_if_finished() -> Result<bool, String> {
    let Some(reaper) = installed_cleanup_reaper() else { return Ok(true); };
    let worker = {
        let mut worker = reaper.worker.lock().map_err(|_| "SANDBOX_CLEANUP_WORKER_LOCK")?;
        if worker.as_ref().is_some_and(|thread| !thread.is_finished()) { return Ok(false); }
        worker.take()
    };
    if let Some(worker) = worker {
        worker.join().map_err(|_| "SANDBOX_CLEANUP_WORKER_PANIC")?;
    }
    Ok(true)
}

fn enqueue_cleanup(task: CleanupTask) {
    let reaper = installed_cleanup_reaper().expect("initialized before native profile creation");
    let (queue, wake) = &*reaper.queue;
    queue.lock().unwrap_or_else(|poison| poison.into_inner()).push(task);
    wake.notify_one();
}
fn start_stderr_reader(
    mut file: File, diagnostic: Arc<Mutex<StderrDiagnostic>>,
) -> Result<thread::JoinHandle<()>, String> {
    thread::Builder::new().name("browseros-sandbox-stderr".into()).spawn(move || {
        let mut bytes = [0u8; 4096];
        loop {
            match file.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(count) => diagnostic.lock().unwrap_or_else(|poison| poison.into_inner())
                    .append(&bytes[..count]),
            }
        }
    }).map_err(|_| "SANDBOX_STDERR_THREAD_START".into())
}

pub struct SandboxProfile {
    name: String,
    sid: PSID,
    sid_string: String,
    folder: PathBuf,
    root_guard: Option<AppContainerDirectory>,
    state_guard: Mutex<Option<AppContainerDirectory>>,
    runtime_guard: Mutex<Option<AppContainerDirectory>>,
    snapshot_guard: Mutex<Option<AppContainerDirectory>>,
    extra_guards: Mutex<Vec<AppContainerDirectory>>,
    runtime_pins: Mutex<Vec<PinnedReadFile>>,
    staged_bytes: Mutex<u64>,
    preparation_failed: AtomicBool,
    budget: OperationBudget,
    diagnostic: Arc<Mutex<StderrDiagnostic>>,
    recovery_lease: Option<lifecycle::ProfileLease>,
}
unsafe impl Send for SandboxProfile {}

impl SandboxProfile {
    pub fn create() -> Result<Self, String> {
        Self::create_with_budget(OperationBudget::new(Duration::from_secs(30)))
    }

    pub fn create_with_budget(budget: OperationBudget) -> Result<Self, String> {
        budget.check().map_err(|error| error.to_string())?;
        // Initialization owns the cleanup worker before admission is enabled.
        // A rejected start cannot create a new service after shutdown joins it.
        // The fsynced owner/SID intent and exclusive native record lease
        // precede the userenv create API. All following failures already have
        // an owning SandboxProfile that queues the durable cleanup obligation.
        let recovery_lease = lifecycle::begin(&budget)?;
        let name = recovery_lease.name().to_owned();
        let expected_sid = recovery_lease.expected_sid().to_owned();
        let mut profile = Self {
            name, sid: ptr::null_mut(), sid_string: expected_sid, folder: PathBuf::new(),
            root_guard: None, state_guard: Mutex::new(None), runtime_guard: Mutex::new(None),
            snapshot_guard: Mutex::new(None), extra_guards: Mutex::new(Vec::new()),
            runtime_pins: Mutex::new(Vec::new()), staged_bytes: Mutex::new(0),
            preparation_failed: AtomicBool::new(false), budget,
            diagnostic: Arc::new(Mutex::new(StderrDiagnostic::default())),
            recovery_lease: Some(recovery_lease),
        };
        let name_wide = wide_text(&profile.name)?;
        let display = wide_text("BrowserOS isolated coding tool")?;
        let description = wide_text("No network; private MCP/LSP execution")?;
        profile.budget.check().map_err(|error| error.to_string())?;
        let mut sid = ptr::null_mut();
        let result = unsafe {
            CreateAppContainerProfile(name_wide.as_ptr(), display.as_ptr(), description.as_ptr(),
                ptr::null(), 0, &mut sid)
        };
        if result < 0 {
            if !sid.is_null() { unsafe { FreeSid(sid); } }
            profile.recovery_lease.as_mut().expect("owned intent").not_registered();
            return Err(format!("SANDBOX_PROFILE_CREATE:{result:x}"));
        }
        profile.recovery_lease.as_mut().expect("owned intent").registered();
        profile.sid = sid;
        if profile.sid.is_null() { return Err("SANDBOX_PROFILE_SID_MISSING".into()); }
        profile.finish_create()
    }

    fn finish_create(mut self) -> Result<Self, String> {
        let mut sid_text = ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(self.sid, &mut sid_text) } == 0 || sid_text.is_null() {
            return Err(win32_error("SANDBOX_SID_TEXT"));
        }
        let mut length = 0usize;
        unsafe {
            while length < 256 && *sid_text.add(length) != 0 {
                length += 1;
            }
        }
        let parsed_sid = if length < 256 {
            unsafe { String::from_utf16(std::slice::from_raw_parts(sid_text, length)) }
                .map_err(|_| "SANDBOX_SID_TEXT")
        } else {
            Err("SANDBOX_SID_TEXT")
        };
        unsafe { LocalFree(sid_text as _) };
        let actual_sid = parsed_sid?;
        if actual_sid != self.sid_string { return Err("SANDBOX_PROFILE_SID_MISMATCH".into()); }
        let name_wide = wide_text(&self.sid_string)?;
        let mut folder = ptr::null_mut();
        let result = unsafe { GetAppContainerFolderPath(name_wide.as_ptr(), &mut folder) };
        if result < 0 || folder.is_null() {
            return Err(format!("SANDBOX_PRIVATE_FOLDER:{result:x}"));
        }
        let mut length = 0usize;
        unsafe {
            while length < 32768 && *folder.add(length) != 0 {
                length += 1;
            }
        }
        let private_folder = if length < 32768 {
            Some(PathBuf::from(unsafe {
                std::ffi::OsString::from_wide(std::slice::from_raw_parts(folder, length))
            }))
        } else {
            None
        };
        unsafe { CoTaskMemFree(folder as _) };
        self.folder = private_folder.ok_or("SANDBOX_PRIVATE_FOLDER")?;
        // CreateAppContainerProfile provisions the OS-owned folders. A
        // missing/replaced result fails closed; no path-based recursive mkdir.
        self.budget.check().map_err(|error| error.to_string())?;
        let policy = self.policy(AppContainerAccess::ReadExecute, false)?;
        // Generic package access is FILE_TRAVERSE only and noninheritable.
        // Private child directories keep own-package list/read permissions;
        // other packages cannot enumerate their contents.
        self.root_guard = Some(AppContainerDirectory::open_profile(
            &self.folder, &policy, true, &self.budget,
        ).map_err(|error| format!("SANDBOX_PROFILE_TRAVERSE_ACL:{error}"))?);
        let identity = self.root_guard.as_ref().expect("pinned profile").identity_record()
            .map_err(|_| "SANDBOX_PROFILE_NATIVE_IDENTITY")?;
        let scope = self.root_guard.as_ref().expect("pinned profile").recovery_scope(&self.name, &self.sid_string)
            .map_err(|_| "SANDBOX_PROFILE_NAMESPACE_IDENTITY")?;
        self.recovery_lease.as_mut().expect("owned intent").bind(identity, scope)?;
        self.budget.check().map_err(|error| error.to_string())?;
        Ok(self)
    }

    pub fn budget(&self) -> &OperationBudget { &self.budget }

    fn check_preparation(&self) -> Result<(), String> {
        self.budget.check().map_err(|error| error.to_string())?;
        if self.preparation_failed.load(Ordering::Acquire) {
            return Err("SANDBOX_PREPARATION_INCOMPLETE".into());
        }
        Ok(())
    }

    fn preparing<T>(&self, work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        self.check_preparation()?;
        let result = work();
        if result.is_err() { self.preparation_failed.store(true, Ordering::Release); }
        result
    }

    fn policy(&self, access: AppContainerAccess, traversal: bool) -> Result<AppContainerPolicy, String> {
        AppContainerPolicy::new(&self.sid_string, access, traversal)
            .map_err(|error| format!("SANDBOX_NATIVE_POLICY:{error}"))
    }

    fn tree_options(&self) -> PrivateTreeOptions {
        PrivateTreeOptions {
            max_objects: PRIVATE_TREE_MAX_ENTRIES, max_depth: PRIVATE_TREE_MAX_DEPTH,
            max_repairs: PRIVATE_TREE_MAX_ENTRIES, max_duration: Duration::from_secs(60),
        }
    }

    fn root_guard(&self) -> Result<&AppContainerDirectory, String> {
        self.root_guard.as_ref().ok_or_else(|| "SANDBOX_PROFILE_NOT_PREPARED".into())
    }

    pub fn state_dir(&self) -> Result<PathBuf, String> {
        self.preparing(|| {
            let policy = self.policy(AppContainerAccess::Modify, false)?;
            let mut guard = self.state_guard.lock().map_err(|_| "SANDBOX_STATE_LOCK")?;
            if let Some(directory) = guard.as_ref() {
                directory.audit_root(&policy, &self.budget)
                    .map_err(|error| format!("SANDBOX_STATE_AUDIT:{error}"))?;
                return Ok(directory.path().to_owned());
            }
            let directory = self.root_guard()?.ensure_child(
                std::ffi::OsStr::new("state"), &policy, &self.tree_options(), &self.budget,
            ).map_err(|error| format!("SANDBOX_STATE_PREPARE:{error}"))?;
            let path = directory.path().to_owned();
            *guard = Some(directory);
            Ok(path)
        })
    }

    pub fn snapshot_dir(&self) -> Result<PathBuf, String> {
        self.preparing(|| {
            let mut guard = self.snapshot_guard.lock().map_err(|_| "SANDBOX_SNAPSHOT_LOCK")?;
            if guard.is_some() { return Err("SANDBOX_SNAPSHOT_ALREADY_CREATED".into()); }
            let policy = self.policy(AppContainerAccess::ReadExecute, true)?;
            let directory = self.root_guard()?.ensure_child(
                std::ffi::OsStr::new("snapshot"), &policy, &self.tree_options(), &self.budget,
            ).map_err(|error| format!("SANDBOX_SNAPSHOT_PREPARE:{error}"))?;
            let path = directory.path().to_owned();
            *guard = Some(directory);
            Ok(path)
        })
    }

    pub fn sid_string(&self) -> &str { &self.sid_string }

    /// Sticky, fixed diagnostics only. Raw stderr and private paths never
    /// cross the production API; the in-memory tail remains bounded.
    pub fn stderr_diagnostic(&self) -> Option<&'static str> {
        self.diagnostic.lock().unwrap_or_else(|poison| poison.into_inner()).code
    }

    #[cfg(test)]
    pub fn stderr_for_test(&self) -> String {
        String::from_utf8_lossy(
            &self.diagnostic.lock().unwrap_or_else(|poison| poison.into_inner()).tail
        ).into_owned()
    }

    fn copy_component(
        &self, mut source: PinnedReadFile, directory: &AppContainerDirectory,
        name: &std::ffi::OsStr, policy: &AppContainerPolicy,
    ) -> Result<PinnedReadFile, String> {
        self.check_preparation()?;
        let expected_length = source.file().metadata().map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?.len();
        if expected_length > MAX_RUNTIME_FILE { return Err("SANDBOX_RUNTIME_SIZE_LIMIT".into()); }
        match directory.pin_file(name, policy, &self.budget) {
            Ok(mut existing) => {
                if existing.file().metadata().map_err(|_| "SANDBOX_RUNTIME_STAGE_CONFLICT")?.len() != expected_length {
                    return Err("SANDBOX_RUNTIME_STAGE_CONFLICT".into());
                }
                // Full bounded comparison, including EOF, rather than equal
                // lengths. Both sources reject write/delete sharing.
                let mut expected = [0u8; 64 * 1024];
                let mut actual = [0u8; 64 * 1024];
                let mut compared = 0u64;
                loop {
                    self.budget.check().map_err(|error| error.to_string())?;
                    let limit = MAX_RUNTIME_FILE.saturating_sub(compared).saturating_add(1)
                        .min(expected.len() as u64) as usize;
                    let count = source.read(&mut expected[..limit]).map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
                    if count == 0 { break; }
                    compared = compared.checked_add(count as u64).filter(|value| *value <= MAX_RUNTIME_FILE)
                        .ok_or("SANDBOX_RUNTIME_SIZE_LIMIT")?;
                    existing.read_exact(&mut actual[..count]).map_err(|_| "SANDBOX_RUNTIME_STAGE_CONFLICT")?;
                    if expected[..count] != actual[..count] { return Err("SANDBOX_RUNTIME_STAGE_CONFLICT".into()); }
                }
                let mut extra = [0u8; 1];
                if compared != expected_length || existing.read(&mut extra).map_err(|_| "SANDBOX_RUNTIME_STAGE_CONFLICT")? != 0 {
                    return Err("SANDBOX_RUNTIME_STAGE_CONFLICT".into());
                }
                existing.rewind().map_err(|_| "SANDBOX_RUNTIME_STAGE_CONFLICT")?;
                return Ok(existing);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("SANDBOX_RUNTIME_STAGE_CONFLICT:{error}")),
        }
        // FILE_CREATE is used only after the read-only existence probe.
        // Retained read pins never collide with an unnecessary MAX/DELETE open.
        let mut output = directory.create_file(name, policy, &self.budget)
            .map_err(|error| format!("SANDBOX_RUNTIME_STAGE_FAILED:{error}"))?;

        let identity = file_identity(&output).map_err(|_| "SANDBOX_RUNTIME_IDENTITY")?;
        let mut total = self.staged_bytes.lock().map_err(|_| "SANDBOX_RUNTIME_BUDGET_LOCK")?;
        let mut written = 0u64;
        let mut bytes = [0u8; 64 * 1024];
        loop {
            self.budget.check().map_err(|error| error.to_string())?;
            // Read at most the remaining per-file allowance plus one byte.
            // Any excess is rejected BEFORE it is written to the destination.
            let remaining = MAX_RUNTIME_FILE.saturating_sub(written)
                .min(MAX_TOTAL_RUNTIME.saturating_sub(*total));
            let count_limit = (remaining.saturating_add(1)).min(bytes.len() as u64) as usize;
            let count = source.read(&mut bytes[..count_limit])
                .map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
            if count == 0 { break; }
            let next = written.checked_add(count as u64).filter(|value| *value <= MAX_RUNTIME_FILE)
                .ok_or("SANDBOX_RUNTIME_SIZE_LIMIT")?;
            let next_total = total.checked_add(count as u64).filter(|value| *value <= MAX_TOTAL_RUNTIME)
                .ok_or("SANDBOX_RUNTIME_SIZE_LIMIT")?;
            self.budget.check().map_err(|error| error.to_string())?;
            output.write_all(&bytes[..count]).map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
            written = next;
            *total = next_total;
        }
        if written != expected_length { return Err("SANDBOX_RUNTIME_CHANGED_DURING_COPY".into()); }
        self.budget.check().map_err(|error| error.to_string())?;
        output.sync_all().map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
        drop(output);
        drop(total);
        let pinned = directory.pin_file(name, policy, &self.budget)
            .map_err(|error| format!("SANDBOX_RUNTIME_READBACK:{error}"))?;
        if pinned.identity().map_err(|_| "SANDBOX_RUNTIME_IDENTITY")? != identity {
            return Err("SANDBOX_RUNTIME_CHANGED_DURING_COPY".into());
        }
        Ok(pinned)
    }

    fn stage_runtime(&self, runtime: &Path) -> Result<PathBuf, String> {
        self.preparing(|| {
            no_reparse_path(runtime)?;
            let source = PinnedReadFile::open(runtime, &self.budget)
                .map_err(|error| format!("SANDBOX_RUNTIME_SOURCE:{error}"))?;
            let canonical = source.path().to_owned();
            let system32 = system_binary("cmd.exe")?.parent()
                .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?.canonicalize()
                .map_err(|_| "SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
            let name = canonical.file_name().and_then(|name| name.to_str())
                .ok_or("SANDBOX_RUNTIME_NOT_UNICODE")?.to_ascii_lowercase();
            let trusted = ["cmd.exe", "curl.exe"].contains(&name.as_str());
            #[cfg(test)]
            let trusted = trusted || name == "whoami.exe";
            if canonical.parent() == Some(system32.as_path()) && trusted {
                self.runtime_pins.lock().map_err(|_| "SANDBOX_RUNTIME_PIN_LOCK")?.push(source);
                return Ok(canonical);
            }
            let policy = self.policy(AppContainerAccess::ReadExecute, false)?;
            let mut runtime_guard = self.runtime_guard.lock().map_err(|_| "SANDBOX_RUNTIME_LOCK")?;
            if runtime_guard.is_none() {
                *runtime_guard = Some(self.root_guard()?.ensure_child(
                    std::ffi::OsStr::new("runtime"), &policy, &self.tree_options(), &self.budget,
                ).map_err(|error| format!("SANDBOX_RUNTIME_PREPARE:{error}"))?);
            }
            let directory = runtime_guard.as_ref().expect("prepared above");
            directory.audit_root(&policy, &self.budget).map_err(|error| format!("SANDBOX_RUNTIME_AUDIT:{error}"))?;
            let filename = canonical.file_name().ok_or("SANDBOX_RUNTIME_NOT_EXECUTABLE")?;
            let staged = directory.path().join(filename);
            // Acquire the parent while the source file's complete chain is
            // still pinned; the DLL enumeration cannot switch to another dir.
            let source_directory = PinnedReadDirectory::open(
                canonical.parent().ok_or("SANDBOX_RUNTIME_NOT_EXECUTABLE")?, &self.budget,
            ).map_err(|error| format!("SANDBOX_RUNTIME_DIRECTORY:{error}"))?;
            let pinned = self.copy_component(source, directory, filename, &policy)?;
            let mut pins = self.runtime_pins.lock().map_err(|_| "SANDBOX_RUNTIME_PIN_LOCK")?;
            pins.push(pinned);
            let entries = source_directory.entries(PRIVATE_TREE_MAX_ENTRIES, &self.budget)
                .map_err(|error| format!("SANDBOX_RUNTIME_ENUMERATE:{error}"))?;
            let mut dll_count = 0usize;
            for entry in entries {
                self.budget.check().map_err(|error| error.to_string())?;
                if Path::new(&entry.name).extension().and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("dll"))
                {
                    if entry.directory { return Err("SANDBOX_RUNTIME_DLL_NOT_FILE".into()); }
                    dll_count += 1;
                    if dll_count > 32 { return Err("SANDBOX_RUNTIME_DLL_LIMIT".into()); }
                    let source = source_directory.child_file(&entry, &self.budget)
                        .map_err(|error| format!("SANDBOX_RUNTIME_SOURCE:{error}"))?;
                    pins.push(self.copy_component(source, directory, &entry.name, &policy)?);
                }
            }
            directory.audit_tree(&policy, &self.tree_options(), &self.budget)
                .map_err(|error| format!("SANDBOX_RUNTIME_AUDIT:{error}"))?;
            Ok(staged)
        })
    }

    /// Bounded source-tree staging. Only a new direct private-profile child
    /// is accepted; callers cannot use this API to reset OS/state/host ACLs.
    pub fn stage_readonly_tree(&self, source: &Path, destination: &Path) -> Result<PathBuf, String> {
        self.preparing(|| {
            no_reparse_path(source)?;
            let relative = destination.strip_prefix(&self.folder)
                .map_err(|_| "SANDBOX_STAGE_TREE_OUTSIDE_PROFILE")?;
            let mut parts = relative.components();
            let name = match (parts.next(), parts.next()) {
                (Some(std::path::Component::Normal(name)), None) => name,
                _ => return Err("SANDBOX_STAGE_TREE_DESTINATION".into()),
            };
            if ["state", "runtime", "snapshot"].iter().any(|reserved|
                browseros_platform_security::windows::same_path_component(name, std::ffi::OsStr::new(reserved)))
            { return Err("SANDBOX_STAGE_TREE_DESTINATION".into()); }
            let policy = self.policy(AppContainerAccess::ReadExecute, true)?;
            let source = PinnedReadDirectory::open(source, &self.budget)
                .map_err(|error| format!("SANDBOX_STAGE_TREE_SOURCE:{error}"))?;
            let target = self.root_guard()?.ensure_child(name, &policy, &self.tree_options(), &self.budget)
                .map_err(|error| format!("SANDBOX_STAGE_TREE_PREPARE:{error}"))?;
            let result = target.path().to_owned();
            let mut pending = vec![(source, target, 0usize)];
            let mut visited = 1usize;
            let mut retained = Vec::new();
            while let Some((source, target, depth)) = pending.pop() {
                self.budget.check().map_err(|error| error.to_string())?;
                if depth > PRIVATE_TREE_MAX_DEPTH { return Err("SANDBOX_STAGE_TREE_DEPTH_LIMIT".into()); }
                let entries = source.entries(PRIVATE_TREE_MAX_ENTRIES.saturating_sub(visited), &self.budget)
                    .map_err(|error| format!("SANDBOX_STAGE_TREE_ENUMERATE:{error}"))?;
                visited = visited.checked_add(entries.len()).filter(|value| *value <= PRIVATE_TREE_MAX_ENTRIES)
                    .ok_or("SANDBOX_STAGE_TREE_ENTRY_LIMIT")?;
                for entry in entries {
                    self.budget.check().map_err(|error| error.to_string())?;
                    if entry.directory {
                        if depth == PRIVATE_TREE_MAX_DEPTH { return Err("SANDBOX_STAGE_TREE_DEPTH_LIMIT".into()); }
                        let from = source.child_directory(&entry, &self.budget)
                            .map_err(|error| format!("SANDBOX_STAGE_TREE_SOURCE:{error}"))?;
                        let to = target.ensure_child(&entry.name, &policy, &self.tree_options(), &self.budget)
                            .map_err(|error| format!("SANDBOX_STAGE_TREE_PREPARE:{error}"))?;
                        pending.push((from, to, depth + 1));
                    } else {
                        let file = source.child_file(&entry, &self.budget)
                            .map_err(|error| format!("SANDBOX_STAGE_TREE_SOURCE:{error}"))?;
                        let pinned = self.copy_component(file, &target, &entry.name, &policy)?;
                        self.runtime_pins.lock().map_err(|_| "SANDBOX_RUNTIME_PIN_LOCK")?.push(pinned);
                    }
                }
                target.audit_root(&policy, &self.budget).map_err(|error| format!("SANDBOX_STAGE_TREE_AUDIT:{error}"))?;
                retained.push(target);
            }
            self.extra_guards.lock().map_err(|_| "SANDBOX_STAGE_TREE_LOCK")?.extend(retained);
            Ok(result)
        })
    }

    fn protect_selected_tree(&self, root: &Path, lsp: bool) -> Result<(), String> {
        self.preparing(|| {
            // All private subtree roots were created and pinned by this profile.
            // Compare an existing native object, never accept an arbitrary host
            // path or reinterpret the OS profile root as a private child.
            let snapshot = self.snapshot_guard.lock().map_err(|_| "SANDBOX_SNAPSHOT_LOCK")?;
            if let Some(directory) = snapshot.as_ref() {
                if browseros_platform_security::windows::paths_are_same_file(root, directory.path())
                    .map_err(|_| "SANDBOX_PRIVATE_TREE_IDENTITY")?
                {
                    let policy = self.policy(AppContainerAccess::ReadExecute, true)?;
                    directory.protect_tree(&policy, &self.tree_options(), &self.budget)
                        .map_err(|error| format!("SANDBOX_SNAPSHOT_ACL:{error}"))?;
                    return Ok(());
                }
            }
            if !lsp {
                let runtime = self.runtime_guard.lock().map_err(|_| "SANDBOX_RUNTIME_LOCK")?;
                if let Some(directory) = runtime.as_ref() {
                    if browseros_platform_security::windows::paths_are_same_file(root, directory.path())
                        .map_err(|_| "SANDBOX_PRIVATE_TREE_IDENTITY")?
                    {
                        let policy = self.policy(AppContainerAccess::ReadExecute, false)?;
                        directory.protect_tree(&policy, &self.tree_options(), &self.budget)
                            .map_err(|error| format!("SANDBOX_RUNTIME_ACL:{error}"))?;
                        return Ok(());
                    }
                }
            }
            Err("SANDBOX_PRIVATE_TREE_OUTSIDE_PROFILE".into())
        })
    }

    pub fn protect_snapshot_files(&self, root: &Path) -> Result<(), String> { self.protect_selected_tree(root, false) }
    pub fn protect_snapshot(&self, root: &Path) -> Result<(), String> { self.protect_selected_tree(root, false) }
    pub fn protect_lsp_snapshot(&self, root: &Path) -> Result<(), String> { self.protect_selected_tree(root, true) }

    pub fn stage_runtime_peer(&self, peer: &Path) -> Result<PathBuf, String> {
        if !peer.extension().and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
        { return Err("SANDBOX_TOOLCHAIN_PEER_MISSING".into()); }
        self.stage_runtime(peer)
    }

    pub fn spawn(
        self, runtime: &Path, args: &[String], environment: &[(String, String)], current_directory: &Path,
    ) -> Result<SandboxChild, String> {
        self.check_preparation()?;
        if !runtime.extension().and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
        { return Err("SANDBOX_RUNTIME_NOT_EXECUTABLE".into()); }
        let state = self.state_dir()?;
        if !browseros_platform_security::windows::paths_are_same_file(current_directory, &state)
            .map_err(|_| "SANDBOX_WORKDIR_IDENTITY")?
        { return Err("SANDBOX_WORKDIR_OUTSIDE_PRIVATE_STATE".into()); }
        let staged = self.stage_runtime(runtime)?;
        {
            let state_guard = self.state_guard.lock().map_err(|_| "SANDBOX_STATE_LOCK")?;
            let policy = self.policy(AppContainerAccess::Modify, false)?;
            state_guard.as_ref().ok_or("SANDBOX_STATE_NOT_PREPARED")?
                .audit_tree(&policy, &self.tree_options(), &self.budget)
                .map_err(|error| format!("SANDBOX_STATE_AUDIT:{error}"))?;
        }
        self.budget.check().map_err(|error| error.to_string())?;
        spawn_inner(self, &staged, args, environment, &state)
    }
}

impl Drop for SandboxProfile {
    fn drop(&mut self) {
        self.runtime_pins.get_mut().unwrap_or_else(|poison| poison.into_inner()).clear();
        self.extra_guards.get_mut().unwrap_or_else(|poison| poison.into_inner()).clear();
        self.snapshot_guard.get_mut().unwrap_or_else(|poison| poison.into_inner()).take();
        self.runtime_guard.get_mut().unwrap_or_else(|poison| poison.into_inner()).take();
        self.state_guard.get_mut().unwrap_or_else(|poison| poison.into_inner()).take();
        self.root_guard.take();
        unsafe {
            if !self.sid.is_null() { FreeSid(self.sid); self.sid = ptr::null_mut(); }
        }
        if let Some(lease) = self.recovery_lease.take() {
            // The durable native lease and budget barrier move with cleanup.
            // Process exit may stop this worker; the next host can recover only
            // a bound, inactive profile with the exact recorded OS identity.
            enqueue_cleanup(CleanupTask::Profile(ProfileCleanup::new(lease)));
        }
    }
}

struct ProcessResources {
    handle: HANDLE,
    job: NamedPrivateJob,
    profile: SandboxProfile,
    stderr: Option<thread::JoinHandle<()>>,
}
// One owner controls these process/Job handles. Public access is read-only;
// the cleanup worker receives ownership only after SandboxProcess is dropped.
unsafe impl Send for ProcessResources {}
impl ProcessResources {
    fn is_complete(&self) -> bool {
        if unsafe { WaitForSingleObject(self.handle, 0) } != WAIT_OBJECT_0
            || self.stderr.as_ref().is_some_and(|worker| !worker.is_finished())
        { return false; }
        self.job.is_empty().unwrap_or(false)
    }
}
impl Drop for ProcessResources {
    fn drop(&mut self) {
        // Only the background owner drops resources after is_complete().
        if let Some(worker) = self.stderr.take() {
            if worker.join().is_err() { eprintln!("SANDBOX_STDERR_THREAD_PANICKED"); }
        }
        unsafe { CloseHandle(self.handle); }
    }
}

pub struct SandboxProcess { resources: Option<Box<ProcessResources>> }
unsafe impl Send for SandboxProcess {}
impl SandboxProcess {
    fn resources(&self) -> &ProcessResources { self.resources.as_deref().expect("owned sandbox process") }
    pub fn exit_code(&self) -> Result<Option<u32>, String> {
        let handle = self.resources().handle;
        match unsafe { WaitForSingleObject(handle, 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0;
                if unsafe { GetExitCodeProcess(handle, &mut code) } == 0 {
                    return Err(win32_error("SANDBOX_EXIT_CODE"));
                }
                Ok(Some(code))
            }
            _ => Err(win32_error("SANDBOX_WAIT")),
        }
    }
    pub fn try_wait(&self) -> Result<bool, String> {
        match unsafe { WaitForSingleObject(self.resources().handle, 0) } {
            WAIT_OBJECT_0 => Ok(true), WAIT_TIMEOUT => Ok(false),
            _ => Err(win32_error("SANDBOX_WAIT")),
        }
    }
    pub fn terminate(&self) -> Result<(), String> {
        self.resources().job.terminate().map_err(|_| "SANDBOX_JOB_TERMINATE".into())
    }
    pub fn profile(&self) -> &SandboxProfile { &self.resources().profile }
    pub(crate) fn hold_io_owner(&self) -> Result<NativeIoOwner, String> {
        self.profile().recovery_lease.as_ref()
            .map(lifecycle::ProfileLease::hold_io_owner)
            .ok_or_else(|| "SANDBOX_JOURNAL_CLOSED".into())
    }
    pub fn cleanup_complete(&self) -> bool { self.resources().is_complete() }
}
impl Drop for SandboxProcess {
    fn drop(&mut self) {
        if let Some(resources) = self.resources.take() {
            let _ = resources.job.terminate();
            // Includes the afterspawn-error path. Never release the profile,
            // stderr thread or capacity barrier while native owners are alive.
            enqueue_cleanup(CleanupTask::Process(resources));
        }
    }
}

pub struct SandboxChild {
    pub stdin: File,
    pub stdout: File,
    pub process: SandboxProcess,
}

fn appcontainer_pipe_security(user: &str, sid: &str) -> Result<PSECURITY_DESCRIPTOR, String> {
    // AC means ALL APPLICATION PACKAGES. Never grant the entire lowbox
    // population access to a private stdio pipe: only this AppContainer SID.
    let sddl = format!("D:(A;;GA;;;SY)(A;;GA;;;{user})(A;;GA;;;{sid})");
    let text = wide_text(&sddl)?;
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    let mut size = 0u32;
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            1,
            &mut descriptor,
            &mut size,
        )
    } == 0
    {
        return Err(win32_error("SANDBOX_PIPE_ACL"));
    }
    Ok(descriptor)
}
struct OwnedSecurityDescriptor(PSECURITY_DESCRIPTOR);
impl Drop for OwnedSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
fn pipe(security: PSECURITY_DESCRIPTOR) -> Result<(File, File), String> {
    let mut read = ptr::null_mut();
    let mut write = ptr::null_mut();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security,
        bInheritHandle: 1,
    };
    if unsafe { CreatePipe(&mut read, &mut write, &attributes, 0) } == 0 {
        return Err(win32_error("SANDBOX_PIPE"));
    }
    Ok(unsafe { (File::from_raw_handle(read), File::from_raw_handle(write)) })
}
fn inheritable(handle: RawHandle, enabled: bool) -> Result<(), String> {
    if unsafe {
        SetHandleInformation(
            handle as HANDLE,
            HANDLE_FLAG_INHERIT,
            if enabled { HANDLE_FLAG_INHERIT } else { 0 },
        )
    } == 0
    {
        return Err(win32_error("SANDBOX_HANDLE_INHERIT"));
    }
    Ok(())
}
fn quote_argument(value: &str) -> Result<Vec<u16>, String> {
    if value.contains('\0') {
        return Err("SANDBOX_ARGUMENT_NUL".into());
    }
    let mut output = String::from("\"");
    let mut slashes = 0usize;
    for character in value.chars() {
        match character {
            '\\' => slashes += 1,
            '"' => {
                output.extend(std::iter::repeat('\\').take(slashes * 2 + 1));
                output.push('"');
                slashes = 0;
            }
            _ => {
                output.extend(std::iter::repeat('\\').take(slashes));
                slashes = 0;
                output.push(character);
            }
        }
    }
    output.extend(std::iter::repeat('\\').take(slashes * 2));
    output.push('"');
    wide_text(&output)
}
/// Windows command lines are not arrays. cmd.exe /C and /K parse the
/// remainder as a single command string, unlike normal CRT argv programs.
/// Only trusted system cmd.exe receives the special tail, and production MCP
/// denies shell executables before reaching SandboxProfile::spawn.
fn process_command_line(runtime: &Path, args: &[String]) -> Result<Vec<u16>, String> {
    let runtime_text = runtime.to_str().ok_or("SANDBOX_RUNTIME_NOT_UNICODE")?;
    let mut command = quote_argument(runtime_text)?;
    command.pop(); // remove the UTF-16 NUL from quote_argument
    let system_cmd = system_binary("cmd.exe")?
        .canonicalize()
        .map_err(|_| "SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
    let is_cmd = runtime
        .to_string_lossy()
        .eq_ignore_ascii_case(&system_cmd.to_string_lossy());
    let cmd_at = if is_cmd {
        args.iter()
            .position(|v| v.eq_ignore_ascii_case("/C") || v.eq_ignore_ascii_case("/K"))
    } else {
        None
    };
    if let Some(index) = cmd_at {
        if index + 1 >= args.len()
            || args[..index].iter().any(|item| {
                !["/D", "/Q", "/E:OFF", "/E:ON", "/V:OFF", "/V:ON"]
                    .iter()
                    .any(|allowed| item.eq_ignore_ascii_case(allowed))
            })
        {
            return Err("SANDBOX_CMD_OPTIONS_INVALID".into());
        }
        for option in &args[..=index] {
            command.push(' ' as u16);
            command.extend(option.encode_utf16());
        }
        let tail = args[index + 1..].join(" ");
        if tail.trim().is_empty() || tail.len() > 8191 || tail.contains(['\0', '\r', '\n']) {
            return Err("SANDBOX_CMD_TAIL_INVALID".into());
        }
        command.push(' ' as u16);
        // No enclosing quotes or CRT escaping: cmd owns /C tail parsing.
        command.extend(tail.encode_utf16());
    } else {
        for argument in args {
            let quoted = quote_argument(argument)?;
            command.push(' ' as u16);
            command.extend(quoted.into_iter().take_while(|u| *u != 0));
        }
    }
    command.push(0);
    if command.len() > 32767 {
        return Err("SANDBOX_COMMAND_LINE_TOO_LONG".into());
    }
    Ok(command)
}

fn environment_block(
    configured: &[(String, String)],
    state: &Path,
    appcontainer_local_data: &Path,
) -> Result<Vec<u16>, String> {
    let system32 = system_binary("cmd.exe")?
        .parent()
        .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?
        .to_path_buf();
    let windows_dir = system32
        .parent()
        .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
    let system_drive = windows_dir
        .components()
        .next()
        .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?
        .as_os_str()
        .to_string_lossy()
        .to_string();
    let mut values = BTreeMap::<String, String>::new();
    values.insert(
        "SYSTEMROOT".into(),
        windows_dir.to_string_lossy().into_owned(),
    );
    values.insert("WINDIR".into(), windows_dir.to_string_lossy().into_owned());
    values.insert("SYSTEMDRIVE".into(), system_drive);
    values.insert(
        "COMSPEC".into(),
        system_binary("cmd.exe")?.to_string_lossy().into_owned(),
    );
    values.insert("PATH".into(), system32.to_string_lossy().into_owned());
    values.insert("RUST_BACKTRACE".into(), "0".into());
    const RESERVED: &[&str] = &[
        "SYSTEMROOT",
        "SYSTEMDRIVE",
        "WINDIR",
        "COMSPEC",
        "PATH",
        "LOCALAPPDATA",
        "USERPROFILE",
        "HOME",
        "TEMP",
        "TMP",
        "APPDATA",
        "PATHEXT",
        "RUST_BACKTRACE",
    ];
    for (key, value) in configured {
        if key.is_empty()
            || key.len() > 128
            || value.len() > 16384
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || value.contains('\0')
            || key.starts_with('=')
            || RESERVED
                .iter()
                .any(|reserved| key.eq_ignore_ascii_case(reserved))
        {
            return Err("SANDBOX_ENV_INVALID".into());
        }
        let key = key.to_ascii_uppercase();
        if values.insert(key, value.clone()).is_some() {
            return Err("SANDBOX_ENV_DUPLICATE".into());
        }
    }
    // GetAppContainerFolderPath, not the host-user's LOCALAPPDATA, is the
    // documented AppContainer local data root.
    no_reparse_path(appcontainer_local_data)?;
    let local_app_data = appcontainer_local_data
        .to_str()
        .ok_or("SANDBOX_LOCAL_APP_DATA_INVALID")?;
    if !appcontainer_local_data.is_absolute() || !appcontainer_local_data.is_dir() {
        return Err("SANDBOX_LOCAL_APP_DATA_INVALID".into());
    }
    values.insert("LOCALAPPDATA".into(), local_app_data.to_owned());
    let state_text = state.to_str().ok_or("SANDBOX_STATE_NOT_UNICODE")?;
    // Private roaming/temp paths keep Node/rust-analyzer away from host data.
    for key in ["HOME", "USERPROFILE", "APPDATA", "TEMP", "TMP"] {
        values.insert(key.into(), state_text.into());
    }
    let mut block = Vec::<u16>::new();
    for (key, value) in values {
        block.extend(key.encode_utf16());
        block.push('=' as u16);
        block.extend(value.encode_utf16());
        block.push(0);
    }
    block.push(0);
    Ok(block)
}

pub(super) fn send_bounded<T>(
    sender: &SyncSender<T>,
    mut value: T,
    budget: Duration,
) -> Result<(), ()> {
    let until = Instant::now() + budget;
    loop {
        match sender.try_send(value) {
            Ok(()) => return Ok(()),
            Err(TrySendError::Disconnected(_)) => return Err(()),
            Err(TrySendError::Full(pending)) => {
                value = pending;
                let remaining = until.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(());
                }
                thread::sleep(remaining.min(Duration::from_millis(5)));
            }
        }
    }
}

fn pipe_security_for_stderr(profile: &SandboxProfile) -> Result<(File, File), String> {
    let user = current_user_sid()?;
    let security = OwnedSecurityDescriptor(appcontainer_pipe_security(&user, &profile.sid_string)?);
    pipe(security.0)
}

fn spawn_inner(
    profile: SandboxProfile,
    runtime: &Path,
    args: &[String],
    environment: &[(String, String)],
    current_directory: &Path,
) -> Result<SandboxChild, String> {
    let user_sid = current_user_sid()?;
    let pipe_security =
        OwnedSecurityDescriptor(appcontainer_pipe_security(&user_sid, &profile.sid_string)?);
    let (child_input, parent_input) = pipe(pipe_security.0)?;
    let (parent_output, child_output) = pipe(pipe_security.0)?;
    drop(pipe_security);
    inheritable(parent_output.as_raw_handle(), false)?;
    inheritable(parent_input.as_raw_handle(), false)?;
    inheritable(child_input.as_raw_handle(), true)?;
    inheritable(child_output.as_raw_handle(), true)?;
    let (parent_error, child_error) = pipe_security_for_stderr(&profile)?;
    inheritable(parent_error.as_raw_handle(), false)?;
    inheritable(child_error.as_raw_handle(), true)?;
    let job = profile.recovery_lease.as_ref().ok_or("SANDBOX_JOURNAL_CLOSED")?.create_job()?;
    let inherited_jobs = [job.as_raw_handle() as HANDLE];
    profile.budget.check().map_err(|error| error.to_string())?;

    // Finish all fallible argument/environment work before the Win32
    // attribute list is initialized. No ? may bypass its cleanup afterward.
    let mut command = process_command_line(runtime, args)?;
    let application = wide(runtime.as_os_str())?;
    let directory = wide(current_directory.as_os_str())?;
    let environment = environment_block(environment, current_directory, &profile.folder)?;
    let mut size = 0usize;
    unsafe {
        let _ = InitializeProcThreadAttributeList(ptr::null_mut(), 3, 0, &mut size);
    }
    if size == 0 {
        return Err("SANDBOX_ATTRIBUTE_SIZE".into());
    }
    // PROC_THREAD_ATTRIBUTE_LIST requires native pointer alignment.
    let words = size.div_ceil(std::mem::size_of::<usize>());
    let mut attribute_storage = vec![0usize; words];
    let attribute_list = attribute_storage.as_mut_ptr() as *mut _;
    if unsafe { InitializeProcThreadAttributeList(attribute_list, 3, 0, &mut size) } == 0 {
        return Err(win32_error("SANDBOX_ATTRIBUTE_INIT"));
    }
    let cleanup_attributes = |list| unsafe { DeleteProcThreadAttributeList(list) };
    let mut capabilities = SECURITY_CAPABILITIES {
        AppContainerSid: profile.sid,
        Capabilities: ptr::null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    let inherited = [
        child_input.as_raw_handle() as HANDLE,
        child_output.as_raw_handle() as HANDLE,
        child_error.as_raw_handle() as HANDLE,
    ];
    let security_ok = unsafe {
        UpdateProcThreadAttribute(
            attribute_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            &mut capabilities as *mut _ as *const _,
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            ptr::null_mut(),
            ptr::null(),
        )
    };
    let handles_ok = security_ok != 0
        && unsafe {
            UpdateProcThreadAttribute(
                attribute_list,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                inherited.as_ptr() as *const _,
                std::mem::size_of_val(&inherited),
                ptr::null_mut(),
                ptr::null(),
            ) != 0
        };
    let jobs_ok = handles_ok && unsafe {
        UpdateProcThreadAttribute(attribute_list, 0, PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            inherited_jobs.as_ptr().cast(), std::mem::size_of_val(&inherited_jobs),
            ptr::null_mut(), ptr::null()) != 0
    };
    if !jobs_ok {
        let error = win32_error("SANDBOX_ATTRIBUTE_UPDATE");
        cleanup_attributes(attribute_list);
        return Err(error);
    }

    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = child_input.as_raw_handle() as HANDLE;
    startup.StartupInfo.hStdOutput = child_output.as_raw_handle() as HANDLE;
    startup.StartupInfo.hStdError = child_error.as_raw_handle() as HANDLE;
    startup.lpAttributeList = attribute_list;
    let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let created = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            CREATE_SUSPENDED
                | CREATE_NEW_PROCESS_GROUP
                | CREATE_UNICODE_ENVIRONMENT
                | EXTENDED_STARTUPINFO_PRESENT,
            environment.as_ptr() as *const _,
            directory.as_ptr(),
            &startup.StartupInfo,
            &mut information,
        )
    };
    let create_error = if created == 0 { Some(win32_error("SANDBOX_CREATE_PROCESS")) } else { None };
    cleanup_attributes(attribute_list);
    drop(child_input);
    drop(child_output);
    drop(child_error);
    if let Some(error) = create_error { return Err(error); }

    // The native Job attribute owns the child before it can execute. Publish
    // every native resource into one supervisor before any further fallible
    // work, including stderr-thread creation, budget checks and ResumeThread.
    struct PrimaryThread(HANDLE);
    impl Drop for PrimaryThread { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }
    let primary = PrimaryThread(information.hThread);
    let diagnostic = Arc::clone(&profile.diagnostic);
    let mut process = SandboxProcess { resources: Some(Box::new(ProcessResources {
        handle: information.hProcess, job, profile, stderr: None,
    })) };
    process.resources.as_mut().expect("owned process").stderr =
        Some(start_stderr_reader(parent_error, diagnostic)?);
    process.profile().budget.check().map_err(|error| error.to_string())?;
    let previous = unsafe { ResumeThread(primary.0) };
    if previous != 1 {
        return Err(if previous == u32::MAX { win32_error("SANDBOX_PRIMARY_THREAD_RESUME") }
            else { format!("SANDBOX_PRIMARY_THREAD_SUSPEND_COUNT:{previous}") });
    }
    process.profile().budget.check().map_err(|error| error.to_string())?;
    drop(primary);
    Ok(SandboxChild { stdin: parent_input, stdout: parent_output, process })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::time::Duration;

    // Bound fixture waits and retain the process/profile while assertions need
    // the private tree. Production ownership uses the background cleanup owner.
    fn run_profile_retained(
        profile: SandboxProfile, executable: &Path, args: &[String], env: &[(String, String)],
    ) -> Result<(String, String, u32, SandboxProcess), String> {
        let state = profile.state_dir()?;
        let SandboxChild { stdin, mut stdout, process } = profile.spawn(executable, args, env, &state)?;
        drop(stdin);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = (&mut stdout).take(1024 * 1024).read_to_end(&mut output)
                .map(|_| output).map_err(|_| "SANDBOX_FIXTURE_READ".to_string());
            let _ = sender.send(result);
        });
        let bytes = match receiver.recv_timeout(Duration::from_secs(8)) {
            Ok(result) => result?,
            Err(_) => {
                let _ = process.terminate();
                let _ = receiver.recv_timeout(Duration::from_secs(3));
                if reader.is_finished() { let _ = reader.join(); }
                return Err("SANDBOX_FIXTURE_STDOUT_TIMEOUT".into());
            }
        };
        reader.join().map_err(|_| "SANDBOX_FIXTURE_READER_PANIC")?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !process.cleanup_complete() {
            if Instant::now() >= deadline {
                let _ = process.terminate();
                return Err("SANDBOX_FIXTURE_EXIT_TIMEOUT".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let exit = process.exit_code()?.ok_or("SANDBOX_FIXTURE_EXIT_UNAVAILABLE")?;
        let stderr = process.profile().stderr_for_test();
        Ok((String::from_utf8_lossy(&bytes).into_owned(), stderr, exit, process))
    }

    fn run_isolated(
        executable: &Path, args: &[String], env: &[(String, String)],
    ) -> Result<(String, String, u32), String> {
        let (stdout, stderr, exit, process) =
            run_profile_retained(SandboxProfile::create()?, executable, args, env)?;
        drop(process);
        Ok((stdout, stderr, exit))
    }

    fn wait_cleanup(budget: &OperationBudget) {
        let until = Instant::now() + Duration::from_secs(15);
        while !budget.cleanup_complete() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(budget.cleanup_complete(), "native profile cleanup is still owned");
    }

    #[test]
    fn appcontainer_runs_without_host_workspace_or_loopback_access() {
        // A closed port is not a network test: unrestricted callers fail too.
        // Use a live localhost listener and first prove host connectivity.
        let listener =
            std::net::TcpListener::bind(("127.0.0.1", 0)).expect("host loopback fixture");
        let endpoint = listener.local_addr().unwrap();
        let host_probe = std::net::TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))
            .expect("host loopback positive control");
        let (accepted, _) = listener.accept().expect("host accepted baseline");
        drop(host_probe);
        drop(accepted);
        listener.set_nonblocking(true).unwrap();
        let curl = system_binary("curl.exe").expect("Windows System32 curl fixture");
        let args = vec![
            "-sS".into(),
            "--connect-timeout".into(),
            "2".into(),
            "--max-time".into(),
            "3".into(),
            "--noproxy".into(),
            "*".into(),
            format!("http://127.0.0.1:{}/probe", endpoint.port()),
        ];
        let (stdout, stderr, exit) =
            run_isolated(&curl, &args, &[]).expect("run curl in no-network AppContainer");
        assert_ne!(exit, 0, "sandbox connected to localhost: {stdout} {stderr}");
        // The executable must have initialized and reported a curl error.
        // A DLL loader failure must not masquerade as network isolation.
        assert!(
            stderr.contains("curl:"),
            "curl did not initialize: {stderr}"
        );
        match listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            _ => panic!("sandbox reached a listening localhost endpoint"),
        }

        // Validate the SAME command string outside AppContainer first. The
        // lowbox process must not read the host-owned, explicitly locked file.
        // cmd avoids requiring PowerShell CLR/module initialization.
        // Deliberately include whitespace to cover Windows path quoting.
        let host_folder = std::env::temp_dir().join(format!(
            "browseros sandbox host {}", uuid::Uuid::new_v4()
        ));
        browseros_platform_security::windows::ensure_private_directory(
            &host_folder, &PrivateTreeOptions::default(),
        ).expect("native owner/SYSTEM host fixture");
        let host_file = host_folder.join("secret.txt");
        std::fs::write(&host_file, "host-secret-marker").unwrap();
        let cmd = system_binary("cmd.exe").unwrap();
        let read_command = format!("type \"{}\"", host_file.display());
        // std::process::Command::arg() quotes the whole /C string using CRT
        // rules; cmd.exe then strips an unmatched outer quote. raw_arg()
        // appends the actual /C command tail, without that extra wrapper.
        let baseline = std::process::Command::new(&cmd)
            .args(["/D", "/C"])
            .raw_arg(&read_command)
            .output()
            .unwrap();
        assert!(
            baseline.status.success()
                && String::from_utf8_lossy(&baseline.stdout).contains("host-secret-marker"),
            "host-file positive control invalid: {}",
            String::from_utf8_lossy(&baseline.stderr)
        );
        let (stdout, stderr, exit) =
            run_isolated(&cmd, &["/D".into(), "/C".into(), read_command], &[])
                .expect("sandbox host-file probe");
        assert!(
            !stdout.contains("host-secret-marker") && exit != 0,
            "AppContainer read host-only file: stdout={stdout:?} stderr={stderr:?}"
        );
        assert!(
            !stderr.to_ascii_lowercase().contains("not recognized"),
            "cmd did not execute the host-file probe: stderr={stderr:?}"
        );
        std::fs::remove_dir_all(host_folder).unwrap();
    }

    #[test]
    fn state_directory_stays_pinned_until_profile_teardown() {
        let profile = SandboxProfile::create().expect("create AppContainer profile");
        let state = profile.state_dir().expect("initialize private state");
        std::fs::write(state.join("retained-data"), b"retain").unwrap();
        assert_eq!(profile.state_dir().unwrap(), state);
        assert_eq!(std::fs::read(state.join("retained-data")).unwrap(), b"retain");
        let moved = state.with_file_name("moved-state");
        assert!(std::fs::rename(&state, &moved).is_err(),
            "live state must not be renamed away from its retained handle");
        let budget = profile.budget().clone();
        drop(profile);
        wait_cleanup(&budget);
        assert!(!state.exists(), "profile teardown must release the state pin");
    }

    #[test]
    fn native_private_tree_depth_limit_preserves_existing_data() {
        let profile = SandboxProfile::create().unwrap();
        let root = profile.folder.join("preexisting");
        std::fs::create_dir(&root).unwrap();
        let mut deepest = root.clone();
        for _ in 0..=PRIVATE_TREE_MAX_DEPTH {
            deepest.push("d");
            std::fs::create_dir(&deepest).unwrap();
        }
        std::fs::write(deepest.join("keep"), b"preserve").unwrap();
        let policy = profile.policy(AppContainerAccess::Modify, false).unwrap();
        let error = match profile.root_guard().unwrap().ensure_child(
            std::ffi::OsStr::new("preexisting"), &policy, &profile.tree_options(), profile.budget(),
        ) {
            Err(error) => error,
            Ok(_) => panic!("native traversal accepted an over-depth tree"),
        };
        assert!(error.to_string().contains("PACKAGE_TREE_BUDGET"));
        assert_eq!(std::fs::read(deepest.join("keep")).unwrap(), b"preserve");
        let budget = profile.budget().clone();
        drop(profile);
        wait_cleanup(&budget);
    }

    #[test]
    fn cmd_command_line_preserves_raw_tail_and_quoted_file_path() {
        let cmd = system_binary("cmd.exe").unwrap().canonicalize().unwrap();
        let args = vec!["/D".into(), "/C".into(), "echo sandbox-ok".into()];
        let command = process_command_line(&cmd, &args).unwrap();
        let decoded = String::from_utf16_lossy(&command[..command.len() - 1]);
        assert!(decoded.ends_with(" /D /C echo sandbox-ok"), "{decoded}");
        assert!(!decoded.ends_with("\"echo sandbox-ok\""), "{decoded}");
        let args = vec![
            "/D".into(),
            "/C".into(),
            "type \"C:\\Test Folder\\test.txt\"".into(),
        ];
        let command = process_command_line(&cmd, &args).unwrap();
        let decoded = String::from_utf16_lossy(&command[..command.len() - 1]);
        assert!(
            decoded.ends_with(" /D /C type \"C:\\Test Folder\\test.txt\""),
            "{decoded}"
        );
    }

    #[test]
    fn quoted_arguments_survive_process_creation() {
        // The extra /S strips quotes and had mangled echo on CI.
        let cmd = system_binary("cmd.exe").unwrap();
        let (stdout, stderr, exit) = run_isolated(
            &cmd,
            &["/D".into(), "/C".into(), "echo sandbox-ok".into()],
            &[],
        )
        .expect("run cmd inside AppContainer");
        assert_eq!(exit, 0, "cmd exited with {exit}: {stderr}");
        assert!(
            stdout.contains("sandbox-ok"),
            "cmd quoting failed: stdout={stdout:?} stderr={stderr:?}"
        );
    }
    #[test]
    fn appcontainer_own_state_create_write_read_and_cross_package_denial() {
        let cmd = system_binary("cmd.exe").unwrap();
        let own = SandboxProfile::create().unwrap();
        let state = own.state_dir().unwrap();
        let command = "echo own-state-marker> own-data.txt & type own-data.txt".to_string();
        let (stdout, stderr, exit, retained) = run_profile_retained(
            own, &cmd, &["/D".into(), "/C".into(), command], &[],
        ).unwrap();
        assert_eq!(exit, 0, "own package state create/write/read failed: {stderr}");
        assert!(stdout.contains("own-state-marker"));
        assert!(std::fs::read_to_string(state.join("own-data.txt")).unwrap().contains("own-state-marker"));

        let target = state.join("own-data.txt");
        let read = format!("type \"{}\"", target.display());
        let (stdout, stderr, exit) = run_isolated(
            &cmd, &["/D".into(), "/C".into(), read], &[],
        ).unwrap();
        assert_ne!(exit, 0, "other package read succeeded: {stdout} {stderr}");
        assert!(!stdout.contains("own-state-marker"));

        let write = format!("echo other-package-marker> \"{}\"", target.display());
        let (_, stderr, exit) = run_isolated(
            &cmd, &["/D".into(), "/C".into(), write], &[],
        ).unwrap();
        assert_ne!(exit, 0, "other package write succeeded: {stderr}");
        let actual = std::fs::read_to_string(&target).unwrap();
        assert!(actual.contains("own-state-marker"));
        assert!(!actual.contains("other-package-marker"));
        let budget = retained.profile().budget().clone();
        drop(retained);
        wait_cleanup(&budget);
    }

    #[test]
    fn own_package_reads_snapshot_but_cannot_overwrite_source() {
        let profile = SandboxProfile::create().unwrap();
        let snapshot = profile.snapshot_dir().unwrap();
        let source = snapshot.join("source.txt");
        std::fs::write(&source, b"source-read-marker").unwrap();
        profile.protect_lsp_snapshot(&snapshot).unwrap();
        let cmd = system_binary("cmd.exe").unwrap();
        let command = format!(
            "(echo forbidden-write-marker>> \"{}\") & type \"{}\"",
            source.display(), source.display(),
        );
        let (stdout, stderr, exit, retained) = run_profile_retained(
            profile, &cmd, &["/D".into(), "/C".into(), command], &[],
        ).unwrap();
        assert_eq!(exit, 0, "own package could not read snapshot: {stderr}");
        assert!(stdout.contains("source-read-marker"));
        assert!(!stdout.contains("forbidden-write-marker"));
        assert_eq!(std::fs::read(&source).unwrap(), b"source-read-marker");
        assert!(!stderr.is_empty(), "denied source write produced no diagnostic");
        let budget = retained.profile().budget().clone();
        drop(retained);
        wait_cleanup(&budget);
    }

    #[test]
    fn runtime_actual_byte_budget_rejects_before_destination_write() {
        let source = std::env::temp_dir().join(format!("browseros-bounded-{}.bin", uuid::Uuid::new_v4()));
        std::fs::write(&source, b"12345678").unwrap();
        let profile = SandboxProfile::create().unwrap();
        let policy = profile.policy(AppContainerAccess::ReadExecute, false).unwrap();
        let destination = profile.root_guard().unwrap().ensure_child(
            std::ffi::OsStr::new("bounded-copy"), &policy, &profile.tree_options(), profile.budget(),
        ).unwrap();
        *profile.staged_bytes.lock().unwrap() = MAX_TOTAL_RUNTIME - 3;
        let input = PinnedReadFile::open(&source, profile.budget()).unwrap();
        let error = match profile.copy_component(
            input, &destination, std::ffi::OsStr::new("runtime.bin"), &policy,
        ) {
            Err(error) => error,
            Ok(_) => panic!("actual runtime byte cap was ignored"),
        };
        assert_eq!(error, "SANDBOX_RUNTIME_SIZE_LIMIT");
        assert_eq!(std::fs::metadata(destination.path().join("runtime.bin")).unwrap().len(), 0);
        assert_eq!(*profile.staged_bytes.lock().unwrap(), MAX_TOTAL_RUNTIME - 3);
        drop(destination);
        std::fs::remove_file(source).unwrap();
        let budget = profile.budget().clone();
        drop(profile);
        wait_cleanup(&budget);
    }

    #[test]
    fn repeated_runtime_component_compares_bytes_without_mutating_open() {
        let source = std::env::temp_dir().join(format!("browseros-duplicate-{}.bin", uuid::Uuid::new_v4()));
        std::fs::write(&source, b"original").unwrap();
        let profile = SandboxProfile::create().unwrap();
        let policy = profile.policy(AppContainerAccess::ReadExecute, false).unwrap();
        let destination = profile.root_guard().unwrap().ensure_child(
            std::ffi::OsStr::new("repeat-copy"), &policy, &profile.tree_options(), profile.budget(),
        ).unwrap();
        let first = profile.copy_component(PinnedReadFile::open(&source, profile.budget()).unwrap(),
            &destination, std::ffi::OsStr::new("runtime.bin"), &policy).unwrap();
        let again = profile.copy_component(PinnedReadFile::open(&source, profile.budget()).unwrap(),
            &destination, std::ffi::OsStr::new("runtime.bin"), &policy).unwrap();
        assert_eq!(first.identity().unwrap(), again.identity().unwrap());
        assert_eq!(*profile.staged_bytes.lock().unwrap(), 8);
        std::fs::write(&source, b"tampered").unwrap();
        assert!(profile.copy_component(PinnedReadFile::open(&source, profile.budget()).unwrap(),
            &destination, std::ffi::OsStr::new("runtime.bin"), &policy).is_err());
        assert_eq!(std::fs::read(destination.path().join("runtime.bin")).unwrap(), b"original");
        drop(again);
        drop(first);
        drop(destination);
        std::fs::remove_file(source).unwrap();
        let budget = profile.budget().clone();
        drop(profile);
        wait_cleanup(&budget);
    }

    #[test]
    fn cancelled_budget_stops_before_profile_creation_and_keeps_cleanup_observable() {
        let budget = OperationBudget::new(Duration::from_secs(10));
        budget.cancel();
        let error = match SandboxProfile::create_with_budget(budget.clone()) {
            Err(error) => error, Ok(_) => panic!("cancelled preparation created a profile"),
        };
        assert!(error.contains("WINDOWS_OPERATION_CANCELLED"));
        assert!(budget.cleanup_complete());
    }

    #[test]
    fn stderr_diagnostics_are_bounded_sticky_and_path_free() {
        let mut diagnostic = StderrDiagnostic::default();
        diagnostic.append(b"rustc --print cfg failed at C:\\private\\workspace\\rustc.exe");
        assert_eq!(diagnostic.code, Some("LSP_RUSTC_CFG_PROBE_FAILED"));
        diagnostic.append(b"failed to open nul device for child: Access is denied (os error 5)");
        assert_eq!(diagnostic.code, Some("LSP_CHILD_NULL_DEVICE_DENIED"));
        diagnostic.append(&vec![b'x'; STDERR_TAIL * 2]);
        assert_eq!(diagnostic.tail.len(), STDERR_TAIL);
        assert_eq!(diagnostic.code, Some("LSP_CHILD_NULL_DEVICE_DENIED"));
        assert!(!diagnostic.code.unwrap().contains("private"));
    }

    #[test]
    fn appcontainer_other_package_cannot_list_private_snapshot() {
        let profile = SandboxProfile::create().unwrap();
        let snapshot = profile.snapshot_dir().unwrap();
        let file = snapshot.join("visible-only-to-own-package.rs");
        std::fs::write(&file, b"own-list-read-marker").unwrap();
        profile.protect_lsp_snapshot(&snapshot).unwrap();
        let cmd = system_binary("cmd.exe").unwrap();
        let own_command = format!("dir /b \"{}\" & type \"{}\"", snapshot.display(), file.display());
        let (stdout, stderr, exit, retained) = run_profile_retained(
            profile, &cmd, &["/D".into(), "/C".into(), own_command], &[],
        ).unwrap();
        assert_eq!(exit, 0, "own package list/read failed: {stderr}");
        assert!(stdout.contains("visible-only-to-own-package.rs"));
        assert!(stdout.contains("own-list-read-marker"));
        let other_command = format!("dir /b \"{}\"", snapshot.display());
        let (stdout, stderr, exit) = run_isolated(
            &cmd, &["/D".into(), "/C".into(), other_command], &[],
        ).unwrap();
        assert_ne!(exit, 0, "another package enumerated the private snapshot: {stdout} {stderr}");
        assert!(!stdout.contains("visible-only-to-own-package.rs"));
        let budget = retained.profile().budget().clone();
        drop(retained);
        wait_cleanup(&budget);
    }

}
