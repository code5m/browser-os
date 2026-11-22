//! Durable ownership and process-wide admission for Windows sandboxes.
//! Background cleanup is owned only while this host is alive. On timeout or
//! crash, the private journal remains for an identity-checked later recovery.
use super::*;
use browseros_platform_security::windows::{
    delete_recorded_profile_tree, hold_profile_cleanup_namespace, ExclusivePrivateRecord,
    NamedPrivateJob, NativeProfileIdentity, NativeProfileScope, PrivateRecordStore,
};
use serde::{Deserialize, Serialize};
use std::io::{Seek, SeekFrom};
use std::sync::{atomic::AtomicUsize, Weak};

const PROFILE_PREFIX: &str = "BrowserOS.Coding.";
const JOB_PREFIX: &str = r"Global\BrowserOS.Coding.Job.v1.";
const MAX_RECORDS: usize = 512;
// Includes startup, live processes, pipe owners and deferred profile cleanup.
// Recovery may adopt older records above this limit but never admits new work.
const MAX_NATIVE_OWNERS: usize = 32;
const MAX_JOURNAL_BYTES: usize = 16 * 1024;
const MAX_FRAME_BYTES: usize = 4096;
const MAX_STATUS_RECORDS: usize = 64;
const STATUS_BUDGET: Duration = Duration::from_millis(750);
const RETRY_BUDGET: Duration = Duration::from_secs(10);
// Linearize producer publication with the one-way shutdown latch. The gate
// covers only counters/flags; no filesystem, native cleanup, wait or wake runs
// while it is held.
static PRODUCER_GATE: Mutex<()> = Mutex::new(());
static INITIALIZING: AtomicUsize = AtomicUsize::new(0);
static STOPPING: AtomicBool = AtomicBool::new(false);
static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryReport {
    pub recovered: usize,
    pub active: usize,
    pub pending: usize,
    pub rejected: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryRecordDiagnostic {
    pub record_id: Option<String>,
    pub phase: String,
    pub code: String,
    pub retryable: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryStatus {
    pub state: String,
    pub can_start: bool,
    pub retry_available: bool,
    pub last_code: Option<String>,
    pub last_report: RecoveryReport,
    pub pending_cleanup: usize,
    pub records: Vec<RecoveryRecordDiagnostic>,
    pub truncated: bool,
}
#[derive(Clone, Default)]
struct RecoveryObservation {
    report: RecoveryReport,
    code: Option<String>,
}

struct Registry {
    store: PrivateRecordStore,
    controls: Mutex<Vec<Weak<Control>>>,
    recovering: AtomicBool,
    rejected: AtomicBool,
    pending_recovery: AtomicUsize,
    stopping: AtomicBool,
    cleanup_deadline: Mutex<Option<Instant>>,
    recovery_budget: Mutex<Option<OperationBudget>>,
    observation: Mutex<RecoveryObservation>,
}
impl Registry {
    fn new(store: PrivateRecordStore) -> Self {
        Self {
            store,
            controls: Mutex::new(Vec::new()),
            recovering: AtomicBool::new(false),
            rejected: AtomicBool::new(false),
            pending_recovery: AtomicUsize::new(0),
            stopping: AtomicBool::new(false),
            cleanup_deadline: Mutex::new(None),
            recovery_budget: Mutex::new(None),
            observation: Mutex::new(RecoveryObservation::default()),
        }
    }
    fn remember(&self, report: &RecoveryReport, code: Option<&str>) {
        *self
            .observation
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = RecoveryObservation {
            report: report.clone(),
            code: code.map(str::to_owned),
        };
    }
    fn gate_open(&self) -> bool {
        !STOPPING.load(Ordering::Acquire)
            && !self.stopping.load(Ordering::Acquire)
            && !self.recovering.load(Ordering::Acquire)
            && !self.rejected.load(Ordering::Acquire)
            && self.pending_recovery.load(Ordering::Acquire) == 0
    }
    fn can_start(&self) -> bool {
        self.gate_open() && self.active_count() < MAX_NATIVE_OWNERS
    }
    fn track(&self, budget: &OperationBudget, starting: bool) -> Result<Arc<Control>, String> {
        let mut owners = self.controls.lock().map_err(|_| "SANDBOX_OWNER_LOCK")?;
        owners.retain(|owner| owner.strong_count() != 0);
        if STOPPING.load(Ordering::Acquire)
            || self.stopping.load(Ordering::Acquire)
            || (starting && !self.gate_open())
        {
            return Err("SANDBOX_ADMISSION_CLOSED".into());
        }
        if starting && owners.len() >= MAX_NATIVE_OWNERS {
            return Err("SANDBOX_CAPACITY_BUSY".into());
        }
        let control = Arc::new(Control {
            operation: budget.clone(),
            job: Mutex::new(None),
            cleanup_budget: Mutex::new(None),
        });
        owners.push(Arc::downgrade(&control));
        Ok(control)
    }
    fn owners(&self) -> Vec<Arc<Control>> {
        let mut owners = self
            .controls
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let alive: Vec<_> = owners.iter().filter_map(Weak::upgrade).collect();
        owners.retain(|owner| owner.strong_count() != 0);
        alive
    }
    fn active_count(&self) -> usize {
        self.controls
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .iter()
            .filter(|owner| owner.strong_count() != 0)
            .count()
    }
    fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(budget) = self
            .recovery_budget
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .as_ref()
        {
            budget.cancel();
        }
        for owner in self.owners() {
            owner.stop();
        }
    }
    fn set_deadline(&self, until: Instant) {
        let mut deadline = self
            .cleanup_deadline
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *deadline = Some(deadline.map_or(until, |old| old.min(until)));
        let exact = deadline.expect("assigned above");
        drop(deadline);
        if let Some(budget) = self
            .recovery_budget
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .as_ref()
        {
            budget.shorten_to(exact);
        }
        for owner in self.owners() {
            if let Some(budget) = owner
                .cleanup_budget
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .as_ref()
            {
                budget.shorten_to(exact);
            }
        }
    }
    fn cleanup_budget(&self, control: &Control) -> OperationBudget {
        let until = self
            .cleanup_deadline
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(2));
        let budget = OperationBudget::until(until);
        *control
            .cleanup_budget
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(budget.clone());
        // A shutdown may have installed its deadline between those two locks.
        if let Some(deadline) = *self
            .cleanup_deadline
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
        {
            budget.shorten_to(deadline);
        }
        budget
    }
}
/// Pipe workers retain the same native admission slot after their request
/// returns, until the actual blocking I/O and profile cleanup both finish.
pub(crate) struct NativeIoOwner {
    _control: Arc<Control>,
    _cleanup: OperationCleanupGuard,
}

struct Control {
    operation: OperationBudget,
    job: Mutex<Option<NamedPrivateJob>>,
    cleanup_budget: Mutex<Option<OperationBudget>>,
}
impl Control {
    fn stop(&self) {
        self.operation.cancel();
        if let Some(job) = self
            .job
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .as_ref()
        {
            let _ = job.terminate();
        }
    }
}

struct Initializing;
impl Initializing {
    fn enter() -> Result<Self, String> {
        let _gate = PRODUCER_GATE.lock().map_err(|_| "SANDBOX_PRODUCER_GATE")?;
        if STOPPING.load(Ordering::Acquire) {
            return Err("SANDBOX_ADMISSION_CLOSED".into());
        }
        INITIALIZING.fetch_add(1, Ordering::AcqRel);
        Ok(Self)
    }
}
impl Drop for Initializing {
    fn drop(&mut self) {
        INITIALIZING.fetch_sub(1, Ordering::AcqRel);
        super::wake_cleanup_reaper();
    }
}

fn initialize_until(parent: &Path, until: Instant) -> Result<Arc<Registry>, String> {
    let _initializing = Initializing::enter()?;
    if let Some(registry) = REGISTRY.get() {
        if !registry
            .store
            .parent_matches(parent)
            .map_err(|_| "SANDBOX_JOURNAL_PARENT")?
        {
            return Err("SANDBOX_JOURNAL_PARENT_CHANGED".into());
        }
        return Ok(Arc::clone(registry));
    }
    super::cleanup_reaper()?;
    let budget = OperationBudget::until(until);
    let store = PrivateRecordStore::open(parent, &budget).map_err(|_| "SANDBOX_JOURNAL_PREPARE")?;
    if STOPPING.load(Ordering::Acquire) {
        return Err("SANDBOX_ADMISSION_CLOSED".into());
    }
    let registry = Arc::new(Registry::new(store));
    let _ = REGISTRY.set(Arc::clone(&registry));
    let installed = REGISTRY.get().ok_or("SANDBOX_JOURNAL_NOT_INITIALIZED")?;
    if !installed
        .store
        .parent_matches(parent)
        .map_err(|_| "SANDBOX_JOURNAL_PARENT")?
    {
        return Err("SANDBOX_JOURNAL_PARENT_CHANGED".into());
    }
    if STOPPING.load(Ordering::Acquire) {
        installed.stop();
    }
    Ok(Arc::clone(installed))
}
pub(crate) fn initialize_recovery_store(parent: &Path) -> Result<(), String> {
    initialize_until(parent, Instant::now() + Duration::from_secs(2)).map(|_| ())
}

#[cfg(test)]
fn initialize_test_store() -> Result<(), String> {
    static TEST_INIT: Mutex<()> = Mutex::new(());
    if REGISTRY.get().is_some() {
        return Ok(());
    }
    let _guard = TEST_INIT.lock().map_err(|_| "SANDBOX_TEST_JOURNAL_LOCK")?;
    if REGISTRY.get().is_none() {
        let path = std::env::temp_dir().join(format!(
            "browseros-sandbox-journal-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        browseros_platform_security::windows::ensure_private_directory(
            &path,
            &PrivateTreeOptions::default(),
        )
        .map_err(|_| "SANDBOX_TEST_JOURNAL_PREPARE")?;
        initialize_recovery_store(&path)?;
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn id_from_name(name: &str) -> Result<&str, String> {
    name.strip_prefix(PROFILE_PREFIX)
        .filter(|id| valid_id(id))
        .ok_or_else(|| "SANDBOX_JOURNAL_PROFILE_NAME".into())
}
fn sid_from_name(name: &str) -> Result<String, String> {
    use windows_sys::Win32::Security::Isolation::DeriveAppContainerSidFromAppContainerName;
    id_from_name(name)?;
    let name = super::wide_text(name)?;
    let mut sid = ptr::null_mut();
    let result = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut sid) };
    if result < 0 || sid.is_null() {
        return Err("SANDBOX_JOURNAL_DERIVE_SID".into());
    }
    struct Sid(PSID);
    impl Drop for Sid {
        fn drop(&mut self) {
            unsafe {
                FreeSid(self.0);
            }
        }
    }
    let sid = Sid(sid);
    let mut raw = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid.0, &mut raw) } == 0 || raw.is_null() {
        return Err("SANDBOX_JOURNAL_SID_TEXT".into());
    }
    let mut len = 0;
    unsafe {
        while len < 256 && *raw.add(len) != 0 {
            len += 1;
        }
    }
    let text = if len < 256 {
        unsafe { String::from_utf16(std::slice::from_raw_parts(raw, len)) }.ok()
    } else {
        None
    };
    unsafe {
        LocalFree(raw as _);
    }
    text.ok_or_else(|| "SANDBOX_JOURNAL_SID_TEXT".into())
}
fn os_profile_folder(sid: &str) -> Result<PathBuf, String> {
    let sid = super::wide_text(sid)?;
    let mut raw = ptr::null_mut();
    let result = unsafe { GetAppContainerFolderPath(sid.as_ptr(), &mut raw) };
    if result < 0 || raw.is_null() {
        return Err("SANDBOX_RECOVERY_OS_PROFILE_FOLDER".into());
    }
    let mut len = 0;
    unsafe {
        while len < 32768 && *raw.add(len) != 0 {
            len += 1;
        }
    }
    let path = if len < 32768 {
        Some(PathBuf::from(unsafe {
            std::ffi::OsString::from_wide(std::slice::from_raw_parts(raw, len))
        }))
    } else {
        None
    };
    unsafe {
        CoTaskMemFree(raw as _);
    }
    path.ok_or_else(|| "SANDBOX_RECOVERY_OS_PROFILE_FOLDER".into())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    volume_serial: u32,
    file_index: u64,
    volume_guid: String,
    created: u64,
}
impl From<NativeProfileIdentity> for Identity {
    fn from(value: NativeProfileIdentity) -> Self {
        Self {
            volume_serial: value.file.volume_serial,
            file_index: value.file.file_index,
            volume_guid: value.volume_guid,
            created: value.created,
        }
    }
}
impl Identity {
    fn native(&self) -> NativeProfileIdentity {
        NativeProfileIdentity {
            file: browseros_platform_security::windows::FileIdentity {
                volume_serial: self.volume_serial,
                file_index: self.file_index,
            },
            volume_guid: self.volume_guid.clone(),
            created: self.created,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    parent: Identity,
    parent_path: Vec<u16>,
    component: String,
}
impl From<NativeProfileScope> for Scope {
    fn from(value: NativeProfileScope) -> Self {
        Self {
            parent: value.parent.into(),
            parent_path: value.parent_path,
            component: value.component,
        }
    }
}
impl Scope {
    fn native(&self) -> NativeProfileScope {
        NativeProfileScope {
            parent: self.parent.native(),
            parent_path: self.parent_path.clone(),
            component: self.component.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Intent,
    Bound,
    DataRemoved,
    NotRegistered,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    sequence: u32,
    name: String,
    owner_sid: String,
    package_sid: String,
    phase: Phase,
    identity: Option<Identity>,
    scope: Option<Scope>,
}
impl Record {
    fn verify(&self, file_name: &str, current_owner: &str) -> Result<(), String> {
        let id = id_from_name(&self.name)?;
        if self.version != 1
            || self.sequence == 0
            || file_name != format!("{id}.profile")
            || self.owner_sid != current_owner
            || self.package_sid != sid_from_name(&self.name)?
            || ((self.phase == Phase::Bound || self.phase == Phase::DataRemoved)
                && (self.identity.is_none() || self.scope.is_none()))
            || ((self.phase == Phase::Intent || self.phase == Phase::NotRegistered)
                && (self.identity.is_some() || self.scope.is_some()))
        {
            return Err("SANDBOX_JOURNAL_IDENTITY_INVALID".into());
        }
        if let Some(identity) = &self.identity {
            if identity.file_index == 0
                || identity.volume_guid.len() != 36
                || identity
                    .volume_guid
                    .bytes()
                    .enumerate()
                    .any(|(index, byte)| {
                        if matches!(index, 8 | 13 | 18 | 23) {
                            byte != b'-'
                        } else {
                            !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase()
                        }
                    })
            {
                return Err("SANDBOX_JOURNAL_FILE_ID_INVALID".into());
            }
        }
        Ok(())
    }
}

// CRC detects incomplete writes, not a security boundary: the file's native
// owner/SYSTEM ACL and exclusive write/delete lease provide authenticity.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
fn frame(record: &Record) -> Result<Vec<u8>, String> {
    let payload = serde_json::to_vec(record).map_err(|_| "SANDBOX_JOURNAL_ENCODE")?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err("SANDBOX_JOURNAL_FRAME_LIMIT".into());
    }
    let mut output = b"BOSJ".to_vec();
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    output.extend_from_slice(&crc32(&payload).to_le_bytes());
    output.extend_from_slice(&payload);
    Ok(output)
}
fn last_record(bytes: &[u8]) -> Result<(Record, usize), String> {
    let mut offset = 0usize;
    let mut last: Option<Record> = None;
    while offset + 12 <= bytes.len() {
        if &bytes[offset..offset + 4] != b"BOSJ" {
            break;
        }
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let checksum = u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap());
        if length == 0 || length > MAX_FRAME_BYTES || offset + 12 + length > bytes.len() {
            break;
        }
        let payload = &bytes[offset + 12..offset + 12 + length];
        if crc32(payload) != checksum {
            break;
        }
        let record: Record =
            serde_json::from_slice(payload).map_err(|_| "SANDBOX_JOURNAL_PARSE")?;
        if let Some(old) = &last {
            if old.sequence.checked_add(1) != Some(record.sequence)
                || record.name != old.name
                || record.owner_sid != old.owner_sid
                || record.package_sid != old.package_sid
            {
                return Err("SANDBOX_JOURNAL_SEQUENCE".into());
            }
        } else if record.sequence != 1 {
            return Err("SANDBOX_JOURNAL_SEQUENCE".into());
        }
        offset += 12 + length;
        last = Some(record);
    }
    last.map(|record| (record, offset))
        .ok_or_else(|| "SANDBOX_JOURNAL_INCOMPLETE_INTENT".into())
}
struct Journal {
    file: ExclusivePrivateRecord,
    value: Record,
    used: u64,
}
impl Journal {
    fn create(mut file: ExclusivePrivateRecord, value: Record) -> Result<Self, String> {
        let bytes = frame(&value)?;
        let result = file
            .file_mut()
            .write_all(&bytes)
            .and_then(|_| file.file().sync_all());
        if result.is_err() {
            let _ = file.mark_deleted(); // No profile API was called yet.
            return Err("SANDBOX_JOURNAL_INTENT_FLUSH".into());
        }
        Ok(Self {
            file,
            value,
            used: bytes.len() as u64,
        })
    }
    fn read(mut file: ExclusivePrivateRecord, name: &str, owner: &str) -> Result<Self, String> {
        let mut bytes = Vec::new();
        file.file_mut()
            .seek(SeekFrom::Start(0))
            .map_err(|_| "SANDBOX_JOURNAL_READ")?;
        file.file_mut()
            .take((MAX_JOURNAL_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "SANDBOX_JOURNAL_READ")?;
        if bytes.len() > MAX_JOURNAL_BYTES {
            return Err("SANDBOX_JOURNAL_LIMIT".into());
        }
        let (value, used) = last_record(&bytes)?;
        value.verify(name, owner)?;
        Ok(Self {
            file,
            value,
            used: used as u64,
        })
    }
    fn append(
        &mut self,
        phase: Phase,
        identity: Option<Identity>,
        scope: Option<Scope>,
    ) -> Result<(), String> {
        let mut next = self.value.clone();
        next.sequence = next
            .sequence
            .checked_add(1)
            .ok_or("SANDBOX_JOURNAL_SEQUENCE")?;
        next.phase = phase;
        next.identity = identity;
        next.scope = scope;
        let bytes = frame(&next)?;
        if self.used + bytes.len() as u64 > MAX_JOURNAL_BYTES as u64 {
            return Err("SANDBOX_JOURNAL_LIMIT".into());
        }
        // Discard only an incomplete trailing frame, preserving the last
        // fsynced ownership state if a previous write was interrupted.
        self.file
            .file()
            .set_len(self.used)
            .map_err(|_| "SANDBOX_JOURNAL_WRITE")?;
        self.file
            .file_mut()
            .seek(SeekFrom::Start(self.used))
            .map_err(|_| "SANDBOX_JOURNAL_WRITE")?;
        self.file
            .file_mut()
            .write_all(&bytes)
            .and_then(|_| self.file.file().sync_all())
            .map_err(|_| "SANDBOX_JOURNAL_FLUSH")?;
        self.used += bytes.len() as u64;
        self.value = next;
        Ok(())
    }
}

pub(super) struct ProfileLease {
    registry: Arc<Registry>,
    control: Arc<Control>,
    journal: Option<Journal>,
    _cleanup_owner: OperationCleanupGuard,
    registered: bool,
    recovery: bool,
    cleaned: bool,
}
impl ProfileLease {
    pub(super) fn hold_io_owner(&self) -> NativeIoOwner {
        NativeIoOwner {
            _control: Arc::clone(&self.control),
            _cleanup: self.control.operation.hold_cleanup(),
        }
    }
    pub(super) fn name(&self) -> &str {
        &self.journal.as_ref().expect("owned journal").value.name
    }
    pub(super) fn expected_sid(&self) -> &str {
        &self
            .journal
            .as_ref()
            .expect("owned journal")
            .value
            .package_sid
    }
    pub(super) fn registered(&mut self) {
        self.registered = true;
    }
    pub(super) fn not_registered(&mut self) {
        self.registered = false;
        if let Some(journal) = &mut self.journal {
            let _ = journal.append(Phase::NotRegistered, None, None);
        }
    }
    pub(super) fn bind(
        &mut self,
        identity: NativeProfileIdentity,
        scope: NativeProfileScope,
    ) -> Result<(), String> {
        if !self.registered {
            return Err("SANDBOX_JOURNAL_UNREGISTERED".into());
        }
        self.journal
            .as_mut()
            .ok_or("SANDBOX_JOURNAL_CLOSED")?
            .append(Phase::Bound, Some(identity.into()), Some(scope.into()))
    }
    pub(super) fn create_job(&self) -> Result<NamedPrivateJob, String> {
        self.control
            .operation
            .check()
            .map_err(|error| error.to_string())?;
        let id = id_from_name(self.name())?;
        let job = NamedPrivateJob::create(&format!("{JOB_PREFIX}{id}"))
            .map_err(|_| "SANDBOX_NAMED_JOB_CREATE")?;
        *self
            .control
            .job
            .lock()
            .map_err(|_| "SANDBOX_JOB_OWNER_LOCK")? = Some(job.clone());
        self.control
            .operation
            .check()
            .map_err(|error| error.to_string())?;
        Ok(job)
    }
    fn cleanup_budget(&self) -> OperationBudget {
        self.registry.cleanup_budget(&self.control)
    }
    pub(super) fn try_cleanup(&mut self) -> Result<(), String> {
        let budget = self.cleanup_budget();
        self.try_cleanup_with(&budget)
    }
    fn try_cleanup_with(&mut self, budget: &OperationBudget) -> Result<(), String> {
        budget.check().map_err(|error| error.to_string())?;
        let journal = self.journal.as_mut().ok_or("SANDBOX_JOURNAL_CLOSED")?;
        if self.registered {
            let current_job = self
                .control
                .job
                .lock()
                .map_err(|_| "SANDBOX_JOB_OWNER_LOCK")?
                .clone();
            if let Some(job) = current_job {
                if !job.is_empty().map_err(|_| "SANDBOX_RECOVERY_JOB_QUERY")? {
                    let _ = job.terminate();
                    return Err("SANDBOX_RECOVERY_JOB_ACTIVE".into());
                }
            } else if let Some(job) = NamedPrivateJob::open(&format!(
                "{JOB_PREFIX}{}",
                id_from_name(&journal.value.name)?
            ))
            .map_err(|_| "SANDBOX_RECOVERY_JOB_OPEN")?
            {
                // An orphan recovery must never delete a profile while the
                // named Job still owns any process, including descendants.
                let active = !job.is_empty().map_err(|_| "SANDBOX_RECOVERY_JOB_QUERY")?;
                *self
                    .control
                    .job
                    .lock()
                    .map_err(|_| "SANDBOX_JOB_OWNER_LOCK")? = Some(job.clone());
                if active {
                    let _ = job.terminate();
                    return Err("SANDBOX_RECOVERY_JOB_ACTIVE".into());
                }
            }
            if journal.value.phase == Phase::Intent {
                // Only this still-live host may bind an intent after it knows
                // CreateAppContainerProfile succeeded. Restart recovery never
                // invents this missing proof.
                if self.recovery {
                    return Err("SANDBOX_RECOVERY_IDENTITY_NOT_BOUND".into());
                }
                let path = os_profile_folder(&journal.value.package_sid)?;
                let pin = PinnedReadDirectory::open(&path, budget)
                    .map_err(|_| "SANDBOX_RECOVERY_PROFILE_PIN")?;
                let identity = pin
                    .identity_record()
                    .map_err(|_| "SANDBOX_RECOVERY_PROFILE_OWNER")?;
                let scope = pin
                    .recovery_scope(&journal.value.name, &journal.value.package_sid)
                    .map_err(|_| "SANDBOX_RECOVERY_NAMESPACE_BIND")?;
                journal.append(Phase::Bound, Some(identity.into()), Some(scope.into()))?;
                drop(pin);
            }
            let namespace = hold_profile_cleanup_namespace(
                &journal
                    .value
                    .scope
                    .as_ref()
                    .ok_or("SANDBOX_RECOVERY_SCOPE_NOT_BOUND")?
                    .native(),
                &journal.value.name,
                &journal.value.package_sid,
                budget,
            )
            .map_err(|error| format!("SANDBOX_RECOVERY_NAMESPACE:{error}"))?;
            if journal.value.phase == Phase::Bound {
                let path = os_profile_folder(&journal.value.package_sid)?;
                let expected = journal
                    .value
                    .identity
                    .as_ref()
                    .ok_or("SANDBOX_RECOVERY_IDENTITY_NOT_BOUND")?
                    .native();
                let options = PrivateTreeOptions {
                    max_objects: PRIVATE_TREE_MAX_ENTRIES,
                    max_depth: PRIVATE_TREE_MAX_DEPTH,
                    max_repairs: 0,
                    max_duration: budget.remaining().map_err(|error| error.to_string())?,
                };
                delete_recorded_profile_tree(&path, &expected, &options, budget)
                    .map_err(|error| format!("SANDBOX_RECOVERY_NATIVE_DELETE:{error}"))?;
                journal.append(
                    Phase::DataRemoved,
                    journal.value.identity.clone(),
                    journal.value.scope.clone(),
                )?;
            }
            budget.check().map_err(|error| error.to_string())?;
            if journal.value.phase != Phase::DataRemoved {
                return Err("SANDBOX_RECOVERY_PHASE".into());
            }
            // The API receives only the strict recorded profile name. Never
            // pass a ledger-provided filesystem path to recursive deletion.
            namespace
                .recheck(&journal.value.package_sid, budget)
                .map_err(|_| "SANDBOX_RECOVERY_NAMESPACE_CHANGED")?;
            let name = super::wide_text(&journal.value.name)?;
            // Microsoft documents deletion of a non-existent profile as
            // success. A crash after this call but before journal removal is
            // therefore retried idempotently with the namespace guard held.
            // https://learn.microsoft.com/windows/win32/api/userenv/nf-userenv-deleteappcontainerprofile
            let result = unsafe { DeleteAppContainerProfile(name.as_ptr()) };
            if result < 0 {
                return Err("SANDBOX_RECOVERY_UNREGISTER_PENDING".into());
            }
        }
        budget.check().map_err(|error| error.to_string())?;
        journal
            .file
            .mark_deleted()
            .map_err(|_| "SANDBOX_JOURNAL_DELETE_PENDING")?;
        self.journal.take(); // Native disposition completes when this handle closes.
        self.cleaned = true;
        Ok(())
    }
}
impl Drop for ProfileLease {
    fn drop(&mut self) {
        if self.recovery && self.cleaned {
            self.registry
                .pending_recovery
                .fetch_sub(1, Ordering::AcqRel);
        }
        // An incomplete lease leaves the durable file intact. No claim that
        // this in-process cleanup owner can survive std::process::exit.
    }
}

pub(super) fn begin(budget: &OperationBudget) -> Result<ProfileLease, String> {
    #[cfg(test)]
    initialize_test_store()?;
    let registry = REGISTRY
        .get()
        .cloned()
        .ok_or("SANDBOX_JOURNAL_NOT_INITIALIZED")?;
    begin_in_registry(registry, budget)
}
fn begin_in_registry(
    registry: Arc<Registry>,
    budget: &OperationBudget,
) -> Result<ProfileLease, String> {
    let control = registry.track(budget, true)?;
    budget.check().map_err(|error| error.to_string())?;
    let cleanup_owner = budget.hold_cleanup();
    let id = uuid::Uuid::new_v4().simple().to_string();
    let name = format!("{PROFILE_PREFIX}{id}");
    let record = Record {
        version: 1,
        sequence: 1,
        package_sid: sid_from_name(&name)?,
        owner_sid: current_user_sid()?,
        name,
        phase: Phase::Intent,
        identity: None,
        scope: None,
    };
    let file = registry
        .store
        .create(std::ffi::OsStr::new(&format!("{id}.profile")), budget)
        .map_err(|_| "SANDBOX_JOURNAL_CREATE")?;
    let journal = Journal::create(file, record)?;
    Ok(ProfileLease {
        registry,
        control,
        journal: Some(journal),
        _cleanup_owner: cleanup_owner,
        registered: false,
        recovery: false,
        cleaned: false,
    })
}

pub(crate) fn recover_abandoned_profiles(
    parent: &Path,
    until: Instant,
) -> Result<RecoveryReport, String> {
    let registry = initialize_until(parent, until)?;
    recover_registry(registry, until)
}
fn recover_registry(registry: Arc<Registry>, until: Instant) -> Result<RecoveryReport, String> {
    {
        let _gate = PRODUCER_GATE.lock().map_err(|_| "SANDBOX_PRODUCER_GATE")?;
        if STOPPING.load(Ordering::Acquire) || registry.stopping.load(Ordering::Acquire) {
            return Err("SANDBOX_ADMISSION_CLOSED".into());
        }
        if registry
            .recovering
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("SANDBOX_RECOVERY_BUSY".into());
        }
    }
    registry.remember(
        &RecoveryReport::default(),
        Some("SANDBOX_RECOVERY_SCANNING"),
    );
    struct Recovering<'a> {
        registry: &'a Registry,
        completed: bool,
    }
    impl Drop for Recovering<'_> {
        fn drop(&mut self) {
            if !self.completed {
                self.registry.rejected.store(true, Ordering::Release);
                let mut observation = self
                    .registry
                    .observation
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                if observation.code.as_deref() == Some("SANDBOX_RECOVERY_SCANNING") {
                    observation.code = Some("SANDBOX_RECOVERY_SCAN_FAILED".into());
                }
            }
            self.registry
                .recovery_budget
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take();
            self.registry.recovering.store(false, Ordering::Release);
            super::wake_cleanup_reaper();
        }
    }
    let mut recovering = Recovering {
        registry: &registry,
        completed: false,
    };
    if STOPPING.load(Ordering::Acquire) || registry.stopping.load(Ordering::Acquire) {
        return Err("SANDBOX_ADMISSION_CLOSED".into());
    }
    let budget = OperationBudget::until(until);
    *registry
        .recovery_budget
        .lock()
        .map_err(|_| "SANDBOX_RECOVERY_BUDGET_LOCK")? = Some(budget.clone());
    if STOPPING.load(Ordering::Acquire) || registry.stopping.load(Ordering::Acquire) {
        budget.cancel();
    }
    let entries = match registry.store.entries(MAX_RECORDS, &budget) {
        Ok(entries) => entries,
        Err(error) => {
            let code = if budget.check().is_err() {
                "SANDBOX_RECOVERY_BUDGET"
            } else if error.to_string().contains("TREE_OBJECT_LIMIT") {
                "SANDBOX_RECOVERY_RECORD_LIMIT"
            } else {
                "SANDBOX_RECOVERY_ENUMERATE"
            };
            registry.remember(&RecoveryReport::default(), Some(code));
            registry.rejected.store(true, Ordering::Release);
            return Err(code.into());
        }
    };
    let owner = current_user_sid()?;
    let mut report = RecoveryReport::default();
    let total = entries.len();
    for (index, entry) in entries.into_iter().enumerate() {
        if budget.check().is_err() {
            report.pending += total - index;
            break;
        }
        let file = match registry.store.try_open(&entry, &budget) {
            Ok(Some(file)) => file,
            Ok(None) => {
                report.active += 1;
                continue;
            }
            Err(_) => {
                report.rejected += 1;
                continue;
            }
        };
        let Some(name) = entry.name.to_str() else {
            report.rejected += 1;
            continue;
        };
        let journal = match Journal::read(file, name, &owner) {
            Ok(journal) => journal,
            Err(_) => {
                report.rejected += 1;
                continue;
            }
        };
        if journal.value.phase == Phase::Intent {
            // Crash between native registration and binding its filesystem
            // identity is explicitly unproven. Retain and report it.
            report.rejected += 1;
            continue;
        }
        let registered = journal.value.phase != Phase::NotRegistered;
        // A busy record is another live host. A record we own with a still
        // active named Job is an orphan cleanup obligation, never a successful
        // "active host" skip. Keep its Job and terminate before touching data.
        let job = if registered {
            let job_name = format!("{JOB_PREFIX}{}", id_from_name(&journal.value.name)?);
            match NamedPrivateJob::open(&job_name) {
                Ok(job) => job,
                Err(_) => {
                    report.rejected += 1;
                    continue;
                }
            }
        } else {
            None
        };
        let control = registry.track(&budget, false)?;
        *control.job.lock().map_err(|_| "SANDBOX_JOB_OWNER_LOCK")? = job;
        registry.pending_recovery.fetch_add(1, Ordering::AcqRel);
        let mut lease = ProfileLease {
            registry: Arc::clone(&registry),
            control,
            journal: Some(journal),
            _cleanup_owner: budget.hold_cleanup(),
            registered,
            recovery: true,
            cleaned: false,
        };
        match lease.try_cleanup_with(&budget) {
            Ok(()) => {
                report.recovered += 1;
                drop(lease);
            }
            Err(_) => {
                report.pending += 1;
                super::enqueue_cleanup(CleanupTask::Profile(ProfileCleanup::new(lease)));
            }
        }
    }
    if report.rejected != 0 || report.pending != 0 {
        let code = if report.rejected != 0 {
            "SANDBOX_RECOVERY_REVIEW_REQUIRED"
        } else if budget.check().is_err() {
            "SANDBOX_RECOVERY_BUDGET"
        } else {
            "SANDBOX_RECOVERY_PENDING"
        };
        registry.remember(&report, Some(code));
        registry.rejected.store(true, Ordering::Release);
        return Err(format!(
            "SANDBOX_RECOVERY_PENDING; recovered={}; active={}; pending={}; rejected={}",
            report.recovered, report.active, report.pending, report.rejected
        ));
    }
    // Only a complete native rescan may clear this admission failure. The
    // read-only diagnostic API and completion of one queued task cannot
    // silently dismiss a different unbound or unexamined record.
    registry.remember(&report, None);
    registry.rejected.store(false, Ordering::Release);
    recovering.completed = true;
    Ok(report)
}

fn diagnostic_id(
    entry: &browseros_platform_security::windows::PinnedDirectoryEntry,
) -> Option<String> {
    entry
        .name
        .to_str()
        .and_then(|name| name.strip_suffix(".profile"))
        .filter(|id| valid_id(id))
        .map(str::to_owned)
}
fn diagnostic_record(
    registry: &Registry,
    entry: &browseros_platform_security::windows::PinnedDirectoryEntry,
    owner: &str,
    budget: &OperationBudget,
) -> RecoveryRecordDiagnostic {
    let mut result = RecoveryRecordDiagnostic {
        record_id: diagnostic_id(entry),
        phase: "unknown".into(),
        code: "SANDBOX_RECOVERY_RECORD_UNREADABLE".into(),
        retryable: false,
    };
    let file = match registry.store.open_readonly(entry, budget) {
        Ok(file) => file,
        Err(_) => return result,
    };
    let mut bytes = Vec::new();
    if file
        .take((MAX_JOURNAL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > MAX_JOURNAL_BYTES
        || budget.check().is_err()
    {
        return result;
    }
    let Some(file_name) = entry.name.to_str() else {
        return result;
    };
    let record = match last_record(&bytes) {
        Ok((record, _)) if record.verify(file_name, owner).is_ok() => record,
        _ => {
            result.code = "SANDBOX_RECOVERY_RECORD_UNVERIFIED".into();
            return result;
        }
    };
    let (phase, code, retryable) = match record.phase {
        Phase::Intent => ("intent", "SANDBOX_RECOVERY_IDENTITY_NOT_BOUND", false),
        Phase::Bound => ("bound", "SANDBOX_RECOVERY_BOUND_RETRYABLE", true),
        Phase::DataRemoved => (
            "data_removed",
            "SANDBOX_RECOVERY_UNREGISTER_RETRYABLE",
            true,
        ),
        Phase::NotRegistered => ("not_registered", "SANDBOX_RECOVERY_RECORD_RETRYABLE", true),
    };
    result.phase = phase.into();
    result.code = code.into();
    result.retryable = retryable;
    result
}

fn status_for_registry(
    registry: &Registry,
    until: Instant,
    record_limit: usize,
) -> Result<RecoveryStatus, String> {
    let budget = OperationBudget::until(until.min(Instant::now() + STATUS_BUDGET));
    let mut records = Vec::new();
    let mut truncated = false;
    let mut diagnostic_code = None;
    if budget.check().is_err() {
        truncated = true;
        diagnostic_code = Some("SANDBOX_RECOVERY_STATUS_BUDGET");
    } else {
        let owner = current_user_sid().map_err(|_| "SANDBOX_RECOVERY_STATUS_OWNER")?;
        match registry.store.entries(MAX_RECORDS, &budget) {
            Ok(entries) => {
                let limit = record_limit.min(MAX_STATUS_RECORDS);
                truncated = entries.len() > limit;
                for entry in entries.into_iter().take(limit) {
                    if budget.check().is_err() {
                        truncated = true;
                        diagnostic_code = Some("SANDBOX_RECOVERY_STATUS_BUDGET");
                        break;
                    }
                    records.push(diagnostic_record(registry, &entry, &owner, &budget));
                    if budget.check().is_err() {
                        truncated = true;
                        diagnostic_code = Some("SANDBOX_RECOVERY_STATUS_BUDGET");
                        break;
                    }
                }
            }
            Err(error) => {
                truncated = true;
                diagnostic_code = Some(if budget.check().is_err() {
                    "SANDBOX_RECOVERY_STATUS_BUDGET"
                } else if error.to_string().contains("TREE_OBJECT_LIMIT") {
                    "SANDBOX_RECOVERY_RECORD_LIMIT"
                } else {
                    "SANDBOX_RECOVERY_STATUS_ENUMERATE"
                });
            }
        }
    }
    // This is an observation, not a deletion authorization or a claim that
    // live owners are inactive. Creating a profile/retrying recovery rechecks
    // its native admission, record lease, Job and identity at the action.
    let observation = registry
        .observation
        .lock()
        .map_err(|_| "SANDBOX_RECOVERY_STATUS_LOCK")?
        .clone();
    let pending = registry.pending_recovery.load(Ordering::Acquire);
    let stopping = STOPPING.load(Ordering::Acquire) || registry.stopping.load(Ordering::Acquire);
    let recovering = registry.recovering.load(Ordering::Acquire);
    let rejected = registry.rejected.load(Ordering::Acquire);
    let state = if stopping {
        "stopping"
    } else if recovering {
        "recovering"
    } else if rejected && observation.report.rejected != 0 {
        "review_required"
    } else if rejected || pending != 0 {
        "pending"
    } else {
        "ready"
    };
    let last_code = observation
        .code
        .or_else(|| diagnostic_code.map(str::to_owned))
        .or_else(|| (pending != 0).then(|| "SANDBOX_RECOVERY_CLEANUP_PENDING".into()));
    Ok(RecoveryStatus {
        state: state.into(),
        can_start: registry.can_start(),
        retry_available: !stopping && !recovering,
        last_code,
        last_report: observation.report,
        pending_cleanup: pending,
        records,
        truncated,
    })
}

/// Bounded, read-only metadata from the already-bound journal. It does not
/// create a store, claim an exclusive cleanup lease, change health/admission,
/// terminate a Job, or expose filesystem paths, SIDs or raw journal contents.
pub(crate) fn recovery_status(until: Instant) -> Result<RecoveryStatus, String> {
    let Some(registry) = REGISTRY.get() else {
        let stopping = STOPPING.load(Ordering::Acquire);
        return Ok(RecoveryStatus {
            state: if stopping {
                "stopping"
            } else {
                "not_initialized"
            }
            .into(),
            can_start: false,
            retry_available: false,
            last_code: Some(
                if stopping {
                    "SANDBOX_ADMISSION_CLOSED"
                } else {
                    "SANDBOX_JOURNAL_NOT_INITIALIZED"
                }
                .into(),
            ),
            last_report: RecoveryReport::default(),
            pending_cleanup: 0,
            records: Vec::new(),
            truncated: false,
        });
    };
    status_for_registry(registry, until, MAX_STATUS_RECORDS)
}

fn retry_registry(registry: Arc<Registry>, until: Instant) -> Result<RecoveryStatus, String> {
    let until = until.min(Instant::now() + RETRY_BUDGET);
    // The original producer gate/CAS serializes scans. Existing native record
    // leases still exclude this retry from live hosts and queued cleanup.
    // A failed attempt remains visible in the status instead of being
    // presented as successful deletion; unknown Intent records are retained.
    if let Err(error) = recover_registry(Arc::clone(&registry), until) {
        // In-progress/shutdown admission is already reflected by the native
        // state. Other failures must remain observable even if they occurred
        // before the scan guard could publish its normal failure snapshot.
        if error != "SANDBOX_RECOVERY_BUSY"
            && error != "SANDBOX_ADMISSION_CLOSED"
            && !registry.rejected.load(Ordering::Acquire)
        {
            registry.remember(
                &RecoveryReport::default(),
                Some("SANDBOX_RECOVERY_SCAN_FAILED"),
            );
            registry.rejected.store(true, Ordering::Release);
        }
    }
    status_for_registry(&registry, until, MAX_STATUS_RECORDS)
}

/// Explicit retry accepts no filesystem path or record identifier from a UI.
/// It uses only the journal installed by trusted startup preparation.
pub(crate) fn retry_recovery(until: Instant) -> Result<RecoveryStatus, String> {
    let registry = REGISTRY
        .get()
        .cloned()
        .ok_or("SANDBOX_JOURNAL_NOT_INITIALIZED")?;
    retry_registry(registry, until)
}

pub(super) fn admission_available() -> bool {
    // Native tests query prerequisites before their first profile creation.
    // Only test builds install this private fixture; production initialization
    // remains owned by startup and never falls back to a temporary store.
    #[cfg(test)]
    if initialize_test_store().is_err() {
        return false;
    }
    REGISTRY.get().is_some_and(|registry| registry.can_start())
}
pub(super) fn shutdown_requested() -> bool {
    STOPPING.load(Ordering::Acquire)
}

pub(super) fn reaper_may_exit() -> bool {
    STOPPING.load(Ordering::Acquire)
        && INITIALIZING.load(Ordering::Acquire) == 0
        && REGISTRY.get().map_or(true, |registry| {
            !registry.recovering.load(Ordering::Acquire) && registry.active_count() == 0
        })
}
pub(crate) fn request_shutdown() {
    {
        let _gate = PRODUCER_GATE
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        STOPPING.store(true, Ordering::Release);
    }
    if let Some(registry) = REGISTRY.get() {
        registry.stop();
    }
    super::wake_cleanup_reaper();
}
pub(crate) fn shutdown_until(until: Instant) -> Result<(), String> {
    request_shutdown();
    if let Some(registry) = REGISTRY.get() {
        registry.set_deadline(until);
    }
    loop {
        let (active, pending, recovering, rejected) = if let Some(registry) = REGISTRY.get() {
            registry.stop();
            registry.set_deadline(until);
            (
                registry.active_count(),
                registry.pending_recovery.load(Ordering::Acquire),
                registry.recovering.load(Ordering::Acquire),
                registry.rejected.load(Ordering::Acquire),
            )
        } else {
            (0, 0, false, false)
        };
        let initializing = INITIALIZING.load(Ordering::Acquire);
        super::wake_cleanup_reaper();
        // Even failed initialization may have started an idle service. Never
        // report success before the already-created cleanup thread is joined.
        if active == 0
            && pending == 0
            && !recovering
            && initializing == 0
            && super::join_cleanup_reaper_if_finished()?
        {
            return if rejected {
                Err("SANDBOX_PROFILE_RECOVERY_PENDING".into())
            } else {
                Ok(())
            };
        }
        if Instant::now() >= until {
            return Err(format!(
                "SANDBOX_SHUTDOWN_PENDING; native_owners={active}; recovery={pending}; scanning={recovering}; initializing={initializing}"
            ));
        }
        thread::sleep(
            until
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(10)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_registry(label: &str) -> (PathBuf, Arc<Registry>, OperationBudget) {
        super::super::cleanup_reaper().unwrap();
        let parent = std::env::temp_dir().join(format!(
            "browseros-journal-{label}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        browseros_platform_security::windows::ensure_private_directory(
            &parent,
            &PrivateTreeOptions::default(),
        )
        .unwrap();
        let budget = OperationBudget::new(Duration::from_secs(30));
        let store = PrivateRecordStore::open(&parent, &budget).unwrap();
        (parent, Arc::new(Registry::new(store)), budget)
    }
    fn discard_test_records(registry: &Registry) {
        let budget = OperationBudget::new(Duration::from_secs(10));
        for entry in registry.store.entries(MAX_RECORDS, &budget).unwrap() {
            let file = registry
                .store
                .try_open(&entry, &budget)
                .unwrap()
                .expect("test lease released");
            file.mark_deleted().unwrap();
            drop(file);
        }
    }

    #[test]
    fn admission_counts_native_io_after_the_profile_owner_returns() {
        let (parent, registry, budget) = local_registry("capacity");
        let mut owners = Vec::new();
        for _ in 0..MAX_NATIVE_OWNERS {
            owners.push(registry.track(&budget, true).unwrap());
        }
        assert!(!registry.can_start());
        assert!(
            matches!(registry.track(&budget, true), Err(code) if code == "SANDBOX_CAPACITY_BUSY")
        );
        let io = NativeIoOwner {
            _control: Arc::clone(&owners[0]),
            _cleanup: budget.hold_cleanup(),
        };
        drop(owners.remove(0));
        assert!(registry.track(&budget, true).is_err());
        drop(io);
        let replacement = registry.track(&budget, true).unwrap();
        assert_eq!(registry.active_count(), MAX_NATIVE_OWNERS);
        drop(replacement);
        drop(owners);
        assert!(registry.can_start());
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn recovery_retains_unbound_intent_and_blocks_new_profiles() {
        let (parent, registry, budget) = local_registry("unbound");
        let lease = begin_in_registry(Arc::clone(&registry), &budget).unwrap();
        drop(lease); // Simulate process loss before a filesystem identity was bound.
        assert!(recover_registry(
            Arc::clone(&registry),
            Instant::now() + Duration::from_secs(2)
        )
        .is_err());
        assert_eq!(registry.store.entries(2, &budget).unwrap().len(), 1);
        assert!(!registry.can_start());
        assert!(registry.track(&budget, true).is_err());
        discard_test_records(&registry);
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn recovery_skips_a_live_native_record_lease() {
        let (parent, registry, budget) = local_registry("active");
        let lease = begin_in_registry(Arc::clone(&registry), &budget).unwrap();
        let report = recover_registry(
            Arc::clone(&registry),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(report.active, 1);
        assert_eq!(report.recovered, 0);
        assert_eq!(report.rejected, 0);
        assert!(registry.can_start());
        drop(lease);
        discard_test_records(&registry);
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn abandoned_bound_profile_is_recovered_by_native_identity() {
        let (parent, registry, budget) = local_registry("bound");
        let mut lease = begin_in_registry(Arc::clone(&registry), &budget).unwrap();
        let name = wide_text(lease.name()).unwrap();
        let display = wide_text("BrowserOS recovery fixture").unwrap();
        let mut sid = ptr::null_mut();
        let result = unsafe {
            CreateAppContainerProfile(
                name.as_ptr(),
                display.as_ptr(),
                display.as_ptr(),
                ptr::null(),
                0,
                &mut sid,
            )
        };
        assert!(
            result >= 0 && !sid.is_null(),
            "native profile fixture failed: {result:x}"
        );
        unsafe {
            FreeSid(sid);
        }
        lease.registered();
        let folder = os_profile_folder(lease.expected_sid()).unwrap();
        let pin = PinnedReadDirectory::open(&folder, &budget).unwrap();
        lease
            .bind(
                pin.identity_record().unwrap(),
                pin.recovery_scope(lease.name(), lease.expected_sid())
                    .unwrap(),
            )
            .unwrap();
        drop(pin);
        std::fs::create_dir(folder.join("state")).unwrap();
        std::fs::write(folder.join("state/private"), b"recover me").unwrap();
        drop(lease); // Persisted record, no live host lease and no child Job.
        let report = recover_registry(
            Arc::clone(&registry),
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(report.recovered, 1);
        assert_eq!(report.pending, 0);
        assert!(!folder.exists());
        assert!(registry.store.entries(1, &budget).unwrap().is_empty());
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn local_shutdown_shortens_cleanup_and_preserves_pending_record() {
        let (parent, registry, budget) = local_registry("shutdown");
        let mut lease = begin_in_registry(Arc::clone(&registry), &budget).unwrap();
        let cleanup = registry.cleanup_budget(&lease.control);
        registry.stop(); // A local owner; never mutate the process-global latch in tests.
        registry.set_deadline(Instant::now());
        assert!(!registry.can_start());
        assert!(budget.check().is_err());
        assert_eq!(
            cleanup.check().unwrap_err().kind(),
            std::io::ErrorKind::TimedOut
        );
        assert!(lease.try_cleanup().is_err());
        assert!(!budget.cleanup_complete());
        drop(lease);
        let fresh = OperationBudget::new(Duration::from_secs(10));
        assert_eq!(registry.store.entries(2, &fresh).unwrap().len(), 1);
        discard_test_records(&registry);
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }

    fn sample_record() -> Record {
        let id = "0123456789abcdef0123456789abcdef";
        let name = format!("{PROFILE_PREFIX}{id}");
        Record {
            version: 1,
            sequence: 1,
            owner_sid: current_user_sid().unwrap(),
            package_sid: sid_from_name(&name).unwrap(),
            name,
            phase: Phase::Intent,
            identity: None,
            scope: None,
        }
    }
    #[test]
    fn journal_rejects_path_fields_and_sid_substitution() {
        let record = sample_record();
        record
            .verify(
                "0123456789abcdef0123456789abcdef.profile",
                &record.owner_sid,
            )
            .unwrap();
        let mut wrong = record.clone();
        wrong.package_sid = "S-1-1-0".into();
        assert!(wrong
            .verify(
                "0123456789abcdef0123456789abcdef.profile",
                &record.owner_sid
            )
            .is_err());
        let mut value = serde_json::to_value(&record).unwrap();
        value["path"] = serde_json::json!("C:\\arbitrary\\user-data");
        let payload = serde_json::to_vec(&value).unwrap();
        let mut bytes = b"BOSJ".to_vec();
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&crc32(&payload).to_le_bytes());
        bytes.extend_from_slice(&payload);
        assert!(
            last_record(&bytes).is_err(),
            "journal-supplied deletion paths must be rejected"
        );
    }

    #[test]
    fn journal_uses_last_complete_frame_after_interrupted_write() {
        let intent = sample_record();
        let mut bound = intent.clone();
        bound.sequence = 2;
        bound.phase = Phase::Bound;
        bound.identity = Some(Identity {
            volume_serial: 1,
            file_index: 2,
            volume_guid: "12345678-1234-1234-1234-123456789abc".into(),
            created: 3,
        });
        bound.scope = Some(Scope {
            parent: bound.identity.clone().unwrap(),
            parent_path: r"C:\fixture".encode_utf16().collect(),
            component: bound.name.clone(),
        });
        let mut bytes = frame(&intent).unwrap();
        bytes.extend_from_slice(&frame(&bound).unwrap());
        let intact = bytes.len();
        let mut removed = bound.clone();
        removed.sequence = 3;
        removed.phase = Phase::DataRemoved;
        let torn = frame(&removed).unwrap();
        bytes.extend_from_slice(&torn[..torn.len() / 2]);
        let (recovered, used) = last_record(&bytes).unwrap();
        assert_eq!(used, intact);
        assert_eq!(recovered.sequence, 2);
        assert!(recovered.phase == Phase::Bound);
        assert_eq!(recovered.identity.unwrap().file_index, 2);
    }
    #[test]
    fn recovery_status_preserves_unbound_intent_and_rejection() {
        let (parent, registry, budget) = local_registry("status-intent");
        let lease = begin_in_registry(Arc::clone(&registry), &budget).unwrap();
        drop(lease);
        assert!(recover_registry(
            Arc::clone(&registry),
            Instant::now() + Duration::from_secs(2)
        )
        .is_err());
        let status = status_for_registry(
            &registry,
            Instant::now() + Duration::from_secs(2),
            MAX_STATUS_RECORDS,
        )
        .unwrap();
        assert_eq!(status.state, "review_required");
        assert!(!status.can_start);
        assert!(status.retry_available);
        assert_eq!(status.records.len(), 1);
        assert_eq!(status.records[0].phase, "intent");
        assert_eq!(
            status.records[0].code,
            "SANDBOX_RECOVERY_IDENTITY_NOT_BOUND"
        );
        assert!(!status.records[0].retryable);
        assert_eq!(registry.store.entries(2, &budget).unwrap().len(), 1);
        assert!(registry.rejected.load(Ordering::Acquire));
        let retried = retry_registry(
            Arc::clone(&registry),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(retried.state, "review_required");
        assert_eq!(registry.store.entries(2, &budget).unwrap().len(), 1);
        discard_test_records(&registry);
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn explicit_recovery_retry_clears_completed_budget_failure() {
        let (parent, registry, budget) = local_registry("retry-budget");
        let mut lease = begin_in_registry(Arc::clone(&registry), &budget).unwrap();
        // No profile was registered; this durable phase is safe to clean.
        lease.not_registered();
        drop(lease);
        assert!(recover_registry(Arc::clone(&registry), Instant::now()).is_err());
        assert!(!registry.can_start());
        let status = retry_registry(
            Arc::clone(&registry),
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(status.state, "ready");
        assert!(status.can_start);
        assert_eq!(status.last_report.recovered, 1);
        assert!(status.last_code.is_none());
        assert_eq!(status.pending_cleanup, 0);
        assert!(registry.store.entries(1, &budget).unwrap().is_empty());
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn recovery_diagnostic_record_list_is_bounded_and_read_only() {
        let (parent, registry, budget) = local_registry("status-limit");
        let mut owners = Vec::new();
        for _ in 0..3 {
            owners.push(begin_in_registry(Arc::clone(&registry), &budget).unwrap());
        }
        let status =
            status_for_registry(&registry, Instant::now() + Duration::from_secs(2), 2).unwrap();
        assert_eq!(status.records.len(), 2);
        assert!(status.truncated);
        assert!(status
            .records
            .iter()
            .all(|record| record.phase == "intent" && !record.retryable));
        assert_eq!(registry.store.entries(4, &budget).unwrap().len(), 3);
        // A status read must neither steal the native owner lease nor mutate
        // the admission state because an active intent was merely observed.
        assert!(registry.can_start());
        drop(owners);
        discard_test_records(&registry);
        drop(registry);
        std::fs::remove_dir_all(parent).unwrap();
    }
}
