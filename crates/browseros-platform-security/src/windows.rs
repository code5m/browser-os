//! Handle-based owner/SYSTEM private storage.
//!
//! Repair is deliberately non-transactional and idempotent: objects already
//! secured stay secured after a failure. Private-directory ACL preparation never
//! deletes data, restores an unsafe DACL, enables privileges, or accepts an
//! incomplete audit. Separate recovery deletes only proved, inactive sandbox data. Close users of the
//! directory, resolve the reported native error and retry the same operation.
//! All errors omit paths and user/account names.
//!
//! SetSecurityInfo normally propagates inheritable ACEs implicitly. Repair
//! opens each repair object with MAXIMUM_ALLOWED: Microsoft documents that this
//! mode does not propagate ACEs to children. Each child is instead opened,
//! inspected and (only if needed) repaired within our own bounded traversal.
//! https://learn.microsoft.com/windows/win32/api/aclapi/nf-aclapi-setsecurityinfo

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::mem::{offset_of, size_of};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::{Component, Path, PathBuf, Prefix};
use std::ptr;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

mod lease;
pub use lease::PreparedPrivateDirectory;
mod appcontainer;
mod recovery;
pub use appcontainer::{
    AppContainerAccess, AppContainerDirectory, AppContainerPolicy, PinnedDirectoryEntry,
    PinnedReadDirectory, PinnedReadFile,
};
pub use recovery::{
    delete_recorded_profile_tree, hold_profile_cleanup_namespace, ExclusivePrivateRecord,
    NamedPrivateJob, NativeProfileIdentity, NativeProfileScope, PrivateRecordStore,
    ProfileCleanupNamespace, ProfileDeleteReport,
};
use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    NtCreateFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT,
    FILE_SYNCHRONOUS_IO_NONALERT,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, LocalFree, RtlNtStatusToDosError, HANDLE, INVALID_HANDLE_VALUE, UNICODE_STRING,
};
use windows_sys::Win32::Globalization::CompareStringOrdinal;
use windows_sys::Win32::Security::Authorization::{
    GetSecurityInfo, SetSecurityInfo, SE_FILE_OBJECT,
};
use windows_sys::Win32::Security::{
    AddAccessAllowedAceEx, CopySid, CreateWellKnownSid, EqualSid, GetAce, GetLengthSid,
    GetSecurityDescriptorControl, GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor,
    IsValidAcl, IsValidSid, SetSecurityDescriptorControl, SetSecurityDescriptorDacl,
    SetSecurityDescriptorOwner, TokenUser, WinLocalSystemSid, ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
    ACL_REVISION, CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, INHERITED_ACE,
    OBJECT_INHERIT_ACE, OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID, SECURITY_DESCRIPTOR, SE_DACL_PRESENT, SE_DACL_PROTECTED,
    TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    FileIdBothDirectoryInfo, FileIdBothDirectoryRestartInfo, GetFileInformationByHandle,
    GetFileInformationByHandleEx, GetFileType, GetFinalPathNameByHandleW,
    BY_HANDLE_FILE_INFORMATION, FILE_ALL_ACCESS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_ID_BOTH_DIR_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_DISK,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

const READ_CONTROL: u32 = 0x0002_0000;
const SYNCHRONIZE: u32 = 0x0010_0000;
const MAXIMUM_ALLOWED: u32 = 0x0200_0000;
const FILE_READ_ATTRIBUTES: u32 = 0x80;
const FILE_LIST_DIRECTORY: u32 = 1;
const FILE_TRAVERSE: u32 = 0x20;
const FILE_ADD_SUBDIRECTORY: u32 = 4;
const ACCESS_ALLOWED_TYPE: u8 = 0;
const ACCESS_DENIED_TYPE: u8 = 1;
const DIRECTORY_FLAGS: u32 = OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE;

/// One cancellation signal and absolute deadline for the entire preparation.
/// Clones do not restart the clock. Cleanup uses an independent budget, never
/// the cancelled request budget; application shutdown may shorten its deadline.
#[derive(Clone, Debug)]
pub struct OperationBudget {
    started: Instant,
    deadline_nanos: Arc<AtomicU64>,
    cancelled: Arc<AtomicBool>,
    cleanup_owners: Arc<AtomicUsize>,
}
impl OperationBudget {
    pub fn new(duration: Duration) -> Self {
        let started = Instant::now();
        Self {
            started,
            deadline_nanos: Arc::new(AtomicU64::new(
                duration.as_nanos().min(u64::MAX as u128) as u64
            )),
            cancelled: Arc::new(AtomicBool::new(false)),
            cleanup_owners: Arc::new(AtomicUsize::new(0)),
        }
    }
    pub fn until(deadline: Instant) -> Self {
        let budget = Self::new(deadline.saturating_duration_since(Instant::now()));
        budget.shorten_to(deadline);
        budget
    }
    /// A process-wide shutdown can shorten an in-progress cleanup without
    /// restarting its clock. All clones observe the same earlier deadline.
    pub fn shorten_to(&self, deadline: Instant) {
        let nanos = deadline
            .saturating_duration_since(self.started)
            .as_nanos()
            .min(u64::MAX as u128) as u64;
        self.deadline_nanos.fetch_min(nanos, Ordering::AcqRel);
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "WINDOWS_OPERATION_CANCELLED",
            ));
        }
        if self.started.elapsed().as_nanos()
            >= u128::from(self.deadline_nanos.load(Ordering::Acquire))
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "WINDOWS_OPERATION_DEADLINE",
            ));
        }
        Ok(())
    }
    pub fn remaining(&self) -> io::Result<Duration> {
        self.check()?;
        Ok(
            Duration::from_nanos(self.deadline_nanos.load(Ordering::Acquire))
                .saturating_sub(self.started.elapsed()),
        )
    }
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
    /// Resource owners retain this barrier through asynchronous cleanup.
    /// Cancellation/deadline never prevent the caller from observing cleanup.
    pub fn hold_cleanup(&self) -> OperationCleanupGuard {
        self.cleanup_owners.fetch_add(1, Ordering::AcqRel);
        OperationCleanupGuard {
            owners: Arc::clone(&self.cleanup_owners),
        }
    }
    pub fn cleanup_complete(&self) -> bool {
        self.cleanup_owners.load(Ordering::Acquire) == 0
    }
}
pub struct OperationCleanupGuard {
    owners: Arc<AtomicUsize>,
}
impl Drop for OperationCleanupGuard {
    fn drop(&mut self) {
        self.owners.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub volume_serial: u32,
    pub file_index: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct PrivateTreeOptions {
    pub max_objects: usize,
    pub max_depth: usize,
    pub max_repairs: usize,
    pub max_duration: Duration,
}

impl Default for PrivateTreeOptions {
    fn default() -> Self {
        Self {
            max_objects: 50_000,
            max_depth: 64,
            max_repairs: 50_000,
            max_duration: Duration::from_secs(15),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PrivateTreeReport {
    pub checked_objects: usize,
    pub repaired_objects: usize,
    pub root_identity: FileIdentity,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug)]
struct StorageFailure {
    stage: &'static str,
    code: u32,
    progress: Option<(usize, usize, u128)>,
}

impl fmt::Display for StorageFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WINDOWS_PRIVATE_STORAGE_{}; win32={}; close_storage_users_and_retry",
            self.stage, self.code
        )?;
        if let Some((checked, repaired, elapsed_ms)) = self.progress {
            write!(f, "; checked={checked}; repaired={repaired}; elapsed_ms={elapsed_ms}; data_preserved=true; unsafe_acl_rollback=false")?;
        }
        Ok(())
    }
}
impl std::error::Error for StorageFailure {}

fn failure(stage: &'static str, code: u32) -> io::Error {
    io::Error::new(
        if code == 0 {
            io::ErrorKind::PermissionDenied
        } else {
            io::Error::from_raw_os_error(code as i32).kind()
        },
        StorageFailure {
            stage,
            code,
            progress: None,
        },
    )
}

fn last_failure(stage: &'static str) -> io::Error {
    failure(
        stage,
        io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32,
    )
}

fn migration_candidate(error: &io::Error) -> bool {
    error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<StorageFailure>())
        .is_some_and(|detail| {
            detail.stage == "ACL_NOT_PRIVATE"
                || (detail.progress.is_none()
                    && detail.stage == "RELATIVE_OPEN"
                    && matches!(detail.code, 2 | 3))
        })
}

struct NativeHandle(HANDLE);
impl Drop for NativeHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

struct NativeAllocation(PSECURITY_DESCRIPTOR);
impl Drop for NativeAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}

struct OwnedSid {
    words: Vec<u32>,
    bytes: u32,
}
impl OwnedSid {
    fn as_ptr(&self) -> PSID {
        self.words.as_ptr() as PSID
    }
    fn as_mut_ptr(&mut self) -> PSID {
        self.words.as_mut_ptr() as PSID
    }

    fn current_user() -> io::Result<Self> {
        let mut token = ptr::null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(last_failure("TOKEN_OPEN"));
        }
        let token = NativeHandle(token);
        let mut required = 0u32;
        unsafe {
            GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
        }
        if required < size_of::<TOKEN_USER>() as u32 || required > 64 * 1024 {
            return Err(failure("TOKEN_SIZE", 0));
        }
        let mut storage =
            vec![0usize; (required as usize + size_of::<usize>() - 1) / size_of::<usize>()];
        if unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                storage.as_mut_ptr().cast(),
                required,
                &mut required,
            )
        } == 0
        {
            return Err(last_failure("TOKEN_READ"));
        }
        let sid = unsafe { (*(storage.as_ptr() as *const TOKEN_USER)).User.Sid };
        if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
            return Err(failure("TOKEN_SID", 0));
        }
        let bytes = unsafe { GetLengthSid(sid) };
        if !(8..=68).contains(&bytes) {
            return Err(failure("TOKEN_SID_SIZE", 0));
        }
        let mut result = Self {
            words: vec![0; (bytes as usize + 3) / 4],
            bytes,
        };
        if unsafe { CopySid(bytes, result.as_mut_ptr(), sid) } == 0 {
            return Err(last_failure("TOKEN_SID_COPY"));
        }
        Ok(result)
    }

    fn system() -> io::Result<Self> {
        let mut result = Self {
            words: vec![0; 17],
            bytes: 68,
        };
        if unsafe {
            CreateWellKnownSid(
                WinLocalSystemSid,
                ptr::null_mut(),
                result.as_mut_ptr(),
                &mut result.bytes,
            )
        } == 0
        {
            return Err(last_failure("SYSTEM_SID"));
        }
        Ok(result)
    }
}

struct Policy {
    owner: OwnedSid,
    system: OwnedSid,
}
struct Descriptor {
    value: SECURITY_DESCRIPTOR,
    _acl: Vec<u32>,
}
impl Descriptor {
    fn as_ptr(&mut self) -> PSECURITY_DESCRIPTOR {
        &mut self.value as *mut _ as _
    }
}
impl Policy {
    fn new() -> io::Result<Self> {
        Ok(Self {
            owner: OwnedSid::current_user()?,
            system: OwnedSid::system()?,
        })
    }

    fn owner_is_system(&self) -> bool {
        unsafe { EqualSid(self.owner.as_ptr(), self.system.as_ptr()) != 0 }
    }

    fn descriptor(&self, directory: bool) -> io::Result<Descriptor> {
        let acl_bytes = size_of::<ACL>()
            + 2 * offset_of!(ACCESS_ALLOWED_ACE, SidStart)
            + self.owner.bytes as usize
            + self.system.bytes as usize;
        let mut acl_storage = vec![0u32; (acl_bytes + 3) / 4];
        let acl = acl_storage.as_mut_ptr() as *mut ACL;
        let flags = if directory { DIRECTORY_FLAGS } else { 0 };
        if unsafe { InitializeAcl(acl, (acl_storage.len() * 4) as u32, ACL_REVISION) } == 0
            || unsafe {
                AddAccessAllowedAceEx(
                    acl,
                    ACL_REVISION,
                    flags,
                    FILE_ALL_ACCESS,
                    self.owner.as_ptr(),
                )
            } == 0
        {
            return Err(last_failure("ACL_CREATE"));
        }
        if !self.owner_is_system()
            && unsafe {
                AddAccessAllowedAceEx(
                    acl,
                    ACL_REVISION,
                    flags,
                    FILE_ALL_ACCESS,
                    self.system.as_ptr(),
                )
            } == 0
        {
            return Err(last_failure("ACL_SYSTEM"));
        }
        let mut result = Descriptor {
            value: unsafe { std::mem::zeroed() },
            _acl: acl_storage,
        };
        if unsafe { InitializeSecurityDescriptor(result.as_ptr(), 1) } == 0
            || unsafe { SetSecurityDescriptorOwner(result.as_ptr(), self.owner.as_ptr(), 0) } == 0
            || unsafe { SetSecurityDescriptorDacl(result.as_ptr(), 1, acl, 0) } == 0
            || unsafe {
                SetSecurityDescriptorControl(result.as_ptr(), SE_DACL_PROTECTED, SE_DACL_PROTECTED)
            } == 0
        {
            return Err(last_failure("DESCRIPTOR_CREATE"));
        }
        Ok(result)
    }
}

/// Owner/SYSTEM-only protected descriptor for a newly-created ordinary file.
/// Keep this value alive through the native create call: the descriptor's
/// owner pointer refers to the retained policy, and its ACL owns its SID bytes.
pub struct PrivateFileSecurity {
    descriptor: Descriptor,
    _policy: Policy,
}
impl PrivateFileSecurity {
    pub fn new() -> io::Result<Self> {
        let policy = Policy::new()?;
        let descriptor = policy.descriptor(false)?;
        Ok(Self {
            descriptor,
            _policy: policy,
        })
    }
    pub fn as_ptr(&mut self) -> PSECURITY_DESCRIPTOR {
        self.descriptor.as_ptr()
    }
}

/// Ordinal Unicode comparison of Windows path spellings. This is not a file
/// authorization predicate; security-sensitive callers use native identities.
pub fn same_path_component(a: &OsStr, b: &OsStr) -> bool {
    if a == b {
        return true;
    }
    let left: Vec<u16> = a.encode_wide().collect();
    let right: Vec<u16> = b.encode_wide().collect();
    if left.len() > i32::MAX as usize || right.len() > i32::MAX as usize {
        return false;
    }
    unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        ) == 2
    }
}

pub fn file_identity(file: &File) -> io::Result<FileIdentity> {
    let info = file_information(file)?;
    let file_index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    if file_index == 0 {
        return Err(failure("UNSUPPORTED_FILE_IDENTITY", 0));
    }
    Ok(FileIdentity {
        volume_serial: info.dwVolumeSerialNumber,
        file_index,
    })
}

fn file_information(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileType(file.as_raw_handle() as _) } != FILE_TYPE_DISK
        || unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) } == 0
    {
        return Err(last_failure("FILE_INFORMATION"));
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(failure("REPARSE_POINT", 0));
    }
    Ok(info)
}

/// Audit an already-open ordinary private file without reopening or repair.
/// Callers retain their own no-reparse/no-delete handle for the whole use.
pub fn audit_private_file(file: &File) -> io::Result<()> {
    if private_object(file)? {
        return Err(failure("PRIVATE_FILE_IS_DIRECTORY", 0));
    }
    let policy = Policy::new()?;
    if !descriptor_is_private(file, &policy, false, false)? {
        return Err(failure("PRIVATE_FILE_ACL_NOT_PRIVATE", 0));
    }
    Ok(())
}

/// A regular owner/SYSTEM-only file and its pinned native parent chain.
/// Creation carries an explicit owner SID; an existing file is only audited.
pub struct PrivateReadWriteFile {
    file: File,
    _parents: OpenedPath,
}
impl PrivateReadWriteFile {
    pub fn file(&self) -> &File {
        &self.file
    }
}

pub fn open_private_read_write_file(path: &Path) -> io::Result<PrivateReadWriteFile> {
    let parent_path = path
        .parent()
        .ok_or_else(|| failure("PRIVATE_FILE_PARENT", 0))?;
    let name = path
        .file_name()
        .ok_or_else(|| failure("PRIVATE_FILE_NAME", 0))?;
    if !name_is_valid(name) {
        return Err(failure("PRIVATE_FILE_NAME", 0));
    }
    let policy = Policy::new()?;
    let parent = open_path(parent_path, None, true, READ_CONTROL | FILE_READ_ATTRIBUTES)?;
    if !private_object(parent.target())?
        || !descriptor_is_private(parent.target(), &policy, true, true)?
    {
        return Err(failure("PRIVATE_FILE_PARENT_POLICY", 0));
    }
    const READ_WRITE_ACCESS: u32 = READ_CONTROL | FILE_READ_ATTRIBUTES | 1 | 2;
    let open_existing = || {
        open_relative(
            parent.target(),
            name,
            READ_WRITE_ACCESS,
            false,
            false,
            ptr::null_mut(),
            true,
        )
    };
    let file = match open_existing() {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let writable_parent = parent.reopen_directory(2 /* FILE_ADD_FILE */)?;
            let mut descriptor = policy.descriptor(false)?;
            match open_relative(
                &writable_parent,
                name,
                READ_WRITE_ACCESS,
                false,
                true,
                descriptor.as_ptr(),
                true,
            ) {
                Ok(file) => file,
                // Another writer may have atomically created the stable lock.
                // Reopen that exact child; never truncate, repair or replace it.
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => open_existing()?,
                Err(error) => return Err(error),
            }
        }
        Err(error) => return Err(error),
    };
    audit_private_file(&file)?;
    Ok(PrivateReadWriteFile {
        file,
        _parents: parent,
    })
}

fn private_object(file: &File) -> io::Result<bool> {
    let info = file_information(file)?;
    let directory = info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    if !directory && info.nNumberOfLinks != 1 {
        return Err(failure("HARDLINK_DENIED", 0));
    }
    Ok(directory)
}

fn name_is_valid(name: &OsStr) -> bool {
    let value: Vec<u16> = name.encode_wide().collect();
    !value.is_empty()
        && value.len() <= 32760
        && value != [b'.' as u16]
        && value != [b'.' as u16, b'.' as u16]
        && !value
            .iter()
            .any(|ch| [0, b'/' as u16, b'\\' as u16, b':' as u16].contains(ch))
        && !value
            .last()
            .is_some_and(|ch| *ch == b'.' as u16 || *ch == b' ' as u16)
}

fn share_mode(repair: bool) -> u32 {
    FILE_SHARE_READ | FILE_SHARE_WRITE | if repair { 0 } else { FILE_SHARE_DELETE }
}

fn open_relative(
    parent: &File,
    name: &OsStr,
    access: u32,
    directory: bool,
    create: bool,
    descriptor: PSECURITY_DESCRIPTOR,
    repair: bool,
) -> io::Result<File> {
    open_relative_with_share(
        parent,
        name,
        access,
        directory,
        create,
        descriptor,
        share_mode(repair),
    )
}

fn open_relative_with_share(
    parent: &File,
    name: &OsStr,
    access: u32,
    directory: bool,
    create: bool,
    descriptor: PSECURITY_DESCRIPTOR,
    sharing: u32,
) -> io::Result<File> {
    if !name_is_valid(name) {
        return Err(failure("COMPONENT_INVALID", 0));
    }
    let mut name: Vec<u16> = name.encode_wide().collect();
    let bytes = (name.len() * 2) as u16;
    let mut unicode = UNICODE_STRING {
        Length: bytes,
        MaximumLength: bytes,
        Buffer: name.as_mut_ptr(),
    };
    let mut attributes: OBJECT_ATTRIBUTES = unsafe { std::mem::zeroed() };
    attributes.Length = size_of::<OBJECT_ATTRIBUTES>() as u32;
    attributes.RootDirectory = parent.as_raw_handle() as _;
    attributes.ObjectName = &mut unicode;
    // Names from handle enumeration must preserve exact spelling, including
    // case-sensitive NTFS directories. Caller paths are also resolved exactly.
    attributes.Attributes = 0;
    attributes.SecurityDescriptor = descriptor.cast();
    let mut status_block: IO_STATUS_BLOCK = unsafe { std::mem::zeroed() };
    let mut raw = ptr::null_mut();
    let status = unsafe {
        NtCreateFile(
            &mut raw,
            access | SYNCHRONIZE,
            &attributes,
            &mut status_block,
            ptr::null(),
            FILE_ATTRIBUTE_NORMAL,
            sharing,
            if create { FILE_CREATE } else { FILE_OPEN },
            FILE_OPEN_REPARSE_POINT
                | FILE_SYNCHRONOUS_IO_NONALERT
                | if directory { FILE_DIRECTORY_FILE } else { 0 },
            ptr::null(),
            0,
        )
    };
    if status < 0 || raw.is_null() {
        return Err(failure("RELATIVE_OPEN", unsafe {
            RtlNtStatusToDosError(status)
        }));
    }
    let file = unsafe { File::from_raw_handle(raw) };
    file_information(&file)?;
    if directory && !file.metadata()?.is_dir() {
        return Err(failure("DIRECTORY_REQUIRED", 0));
    }
    Ok(file)
}

struct OpenedPath {
    chain: Vec<File>,
}
impl OpenedPath {
    fn target(&self) -> &File {
        self.chain.last().expect("drive handle is always present")
    }

    /// Request creation rights through the retained native parent, never by
    /// releasing the namespace pin or relaxing its ACL. Native directory
    /// handles use the same relative-open contract as the initial traversal.
    fn reopen_directory(&self, access: u32) -> io::Result<File> {
        let original = self.target();
        if !private_object(original)? {
            return Err(failure("PARENT_CREATE_NOT_DIRECTORY", 0));
        }
        let path = final_path(original)?;
        let reopened = if self.chain.len() > 1 {
            let parent = &self.chain[self.chain.len() - 2];
            let name = path
                .file_name()
                .ok_or_else(|| failure("PARENT_CREATE_NAME", 0))?;
            open_relative(
                parent,
                name,
                access | FILE_READ_ATTRIBUTES,
                true,
                false,
                ptr::null_mut(),
                true,
            )?
        } else {
            // The volume root has no parent component. Keep its original
            // handle pinned and verify the newly opened volume identity.
            OpenOptions::new()
                .access_mode(access | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
                .share_mode(share_mode(true))
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(&path)?
        };
        if !private_object(&reopened)? || file_identity(original)? != file_identity(&reopened)? {
            return Err(failure("PARENT_CREATE_IDENTITY_CHANGED", 0));
        }
        Ok(reopened)
    }
}

fn open_path(
    path: &Path,
    create: Option<&Policy>,
    repair: bool,
    target_access: u32,
) -> io::Result<OpenedPath> {
    open_path_options(path, create, repair, target_access, 0, None, None)
}

fn open_path_options(
    path: &Path,
    create: Option<&Policy>,
    repair: bool,
    target_access: u32,
    writable_ancestors: usize,
    target_share: Option<u32>,
    budget: Option<&OperationBudget>,
) -> io::Result<OpenedPath> {
    let mut parts = path.components();
    let prefix = match parts.next() {
        Some(Component::Prefix(p))
            if matches!(
                p.kind(),
                Prefix::Disk(_)
                    | Prefix::VerbatimDisk(_)
                    | Prefix::UNC(_, _)
                    | Prefix::VerbatimUNC(_, _)
            ) =>
        {
            p
        }
        _ => return Err(failure("LOCAL_ABSOLUTE_PATH_REQUIRED", 0)),
    };
    if create.is_some() && !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) {
        return Err(failure("PRIVATE_ROOT_REQUIRES_LOCAL_DISK", 0));
    }
    if !matches!(parts.next(), Some(Component::RootDir)) {
        return Err(failure("LOCAL_ABSOLUTE_PATH_REQUIRED", 0));
    }
    let mut names = Vec::new();
    for part in parts {
        match part {
            Component::Normal(name) if name_is_valid(name) => names.push(name.to_os_string()),
            _ => return Err(failure("COMPONENT_INVALID", 0)),
        }
    }
    if names.len() > 256 {
        return Err(failure("ANCESTOR_DEPTH_LIMIT", 0));
    }
    let mut drive = PathBuf::from(prefix.as_os_str());
    drive.push(r"\");
    let access =
        FILE_READ_ATTRIBUTES | FILE_TRAVERSE | if budget.is_some() { READ_CONTROL } else { 0 };
    let file = OpenOptions::new()
        .access_mode(access | SYNCHRONIZE)
        .share_mode(share_mode(repair))
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(drive)
        .map_err(|e| failure("DRIVE_OPEN", e.raw_os_error().unwrap_or(0) as u32))?;
    file_information(&file)?;
    let mut result = OpenedPath { chain: vec![file] };
    for (index, name) in names.iter().enumerate() {
        if let Some(budget) = budget {
            budget.check()?;
        }
        let last = index + 1 == names.len();
        let requested = if last {
            target_access
        } else if names.len().saturating_sub(index + 1) <= writable_ancestors {
            MAXIMUM_ALLOWED
        } else {
            access
        };
        let parent = result.target();
        let directory = index + 1 != names.len() || create.is_some();
        let sharing = if last {
            target_share.unwrap_or_else(|| share_mode(repair))
        } else {
            share_mode(repair)
        };
        let child = match open_relative_with_share(
            parent,
            name,
            requested,
            directory,
            false,
            ptr::null_mut(),
            sharing,
        ) {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound && create.is_some() => {
                let policy = create.expect("checked above");
                let parent_writer = result.reopen_directory(FILE_ADD_SUBDIRECTORY)?;
                let mut descriptor = policy.descriptor(true)?;
                open_relative(
                    &parent_writer,
                    name,
                    requested,
                    true,
                    true,
                    descriptor.as_ptr(),
                    true,
                )?
            }
            Err(error) => return Err(error),
        };
        result.chain.push(child);
    }
    Ok(result)
}

fn final_path(file: &File) -> io::Result<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let used = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle() as _,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            0,
        )
    };
    if used == 0 || used as usize >= buffer.len() {
        return Err(last_failure("FINAL_PATH"));
    }
    Ok(PathBuf::from(OsString::from_wide(&buffer[..used as usize])))
}

fn same_namespace(left: &Path, right: &Path) -> bool {
    fn prefix(path: &Path) -> Option<Prefix<'_>> {
        match path.components().next() {
            Some(Component::Prefix(prefix)) => Some(prefix.kind()),
            _ => None,
        }
    }
    match (prefix(left), prefix(right)) {
        (
            Some(Prefix::Disk(a) | Prefix::VerbatimDisk(a)),
            Some(Prefix::Disk(b) | Prefix::VerbatimDisk(b)),
        ) => a.eq_ignore_ascii_case(&b),
        (
            Some(Prefix::UNC(a, b) | Prefix::VerbatimUNC(a, b)),
            Some(Prefix::UNC(c, d) | Prefix::VerbatimUNC(c, d)),
        ) => same_path_component(a, c) && same_path_component(b, d),
        _ => false,
    }
}

/// Reopen a lexical path without following any reparse component and compare
/// the final kernel identity. Unicode display transformations are never used.
pub fn path_matches_handle(path: &Path, file: &File) -> io::Result<bool> {
    let opened = open_path(path, None, false, FILE_READ_ATTRIBUTES)?;
    Ok(
        same_namespace(&final_path(opened.target())?, &final_path(file)?)
            && file_identity(opened.target())? == file_identity(file)?,
    )
}

/// Resolve the opened object's actual ancestry and find the authorized root
/// by volume/file identity. Case variants on a case-sensitive volume do not
/// inherit each other's authorization.
pub fn handle_is_within_root(file: &File, root: &File) -> io::Result<bool> {
    let actual = file_identity(file)?;
    let trusted = file_identity(root)?;
    let named = final_path(file)?;
    let trusted_name = final_path(root)?;
    if !same_namespace(&named, &trusted_name) {
        return Ok(false);
    }
    if actual == trusted {
        return Ok(true);
    }
    let opened = open_path(&named, None, false, FILE_READ_ATTRIBUTES)?;
    if file_identity(opened.target())? != actual {
        return Err(failure("PATH_IDENTITY_CHANGED", 0));
    }
    for ancestor in &opened.chain {
        if file_identity(ancestor)? == trusted {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Read-only path authorization for already-existing objects.
pub fn path_is_within_root(path: &Path, root: &Path) -> io::Result<bool> {
    let path = path
        .canonicalize()
        .map_err(|e| failure("PATH_CANONICALIZE", e.raw_os_error().unwrap_or(0) as u32))?;
    let root = root
        .canonicalize()
        .map_err(|e| failure("ROOT_CANONICALIZE", e.raw_os_error().unwrap_or(0) as u32))?;
    let opened = open_path(&path, None, false, FILE_READ_ATTRIBUTES)?;
    let trusted = open_path(&root, None, false, FILE_READ_ATTRIBUTES)?;
    if !trusted.target().metadata()?.is_dir() {
        return Err(failure("ROOT_NOT_DIRECTORY", 0));
    }
    handle_is_within_root(opened.target(), trusted.target())
}

pub fn paths_are_same_file(left: &Path, right: &Path) -> io::Result<bool> {
    let left = left
        .canonicalize()
        .map_err(|e| failure("PATH_CANONICALIZE", e.raw_os_error().unwrap_or(0) as u32))?;
    let right = right
        .canonicalize()
        .map_err(|e| failure("PATH_CANONICALIZE", e.raw_os_error().unwrap_or(0) as u32))?;
    let a = open_path(&left, None, false, FILE_READ_ATTRIBUTES)?;
    let b = open_path(&right, None, false, FILE_READ_ATTRIBUTES)?;
    Ok(
        same_namespace(&final_path(a.target())?, &final_path(b.target())?)
            && file_identity(a.target())? == file_identity(b.target())?,
    )
}

fn descriptor_is_private(
    file: &File,
    policy: &Policy,
    directory: bool,
    root: bool,
) -> io::Result<bool> {
    let mut owner = ptr::null_mut();
    let mut dacl = ptr::null_mut();
    let mut raw_descriptor = ptr::null_mut();
    let code = unsafe {
        GetSecurityInfo(
            file.as_raw_handle() as _,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut dacl,
            ptr::null_mut(),
            &mut raw_descriptor,
        )
    };
    if code != 0 {
        return Err(failure("SECURITY_READ", code));
    }
    let descriptor = NativeAllocation(raw_descriptor);
    if descriptor.0.is_null() {
        return Err(failure("DESCRIPTOR_MISSING", 0));
    }
    let mut control = 0u16;
    let mut revision = 0u32;
    if unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) } == 0 {
        return Err(last_failure("DESCRIPTOR_CONTROL"));
    }
    let owner_ok = !owner.is_null()
        && unsafe { IsValidSid(owner) } != 0
        && unsafe { EqualSid(owner, policy.owner.as_ptr()) } != 0;
    if control & SE_DACL_PRESENT == 0 || dacl.is_null() {
        return Ok(false); // A null or absent DACL allows everyone.
    }
    if unsafe { IsValidAcl(dacl) } == 0 {
        return Err(failure("ACL_MALFORMED", 0));
    }
    let count = unsafe { (*dacl).AceCount } as u32;
    if count > 4096 {
        return Err(failure("ACL_SIZE_LIMIT", 0));
    }
    let mut owner_count = 0u32;
    let mut system_count = 0u32;
    let mut compliant = owner_ok && (!root || control & SE_DACL_PROTECTED != 0);
    for index in 0..count {
        let mut raw = ptr::null_mut();
        if unsafe { GetAce(dacl, index, &mut raw) } == 0 || raw.is_null() {
            return Err(last_failure("ACE_READ"));
        }
        let header = unsafe { ptr::read_unaligned(raw as *const ACE_HEADER) };
        // Do not erase an unfamiliar object/callback policy during migration.
        // Its trustee layout is different and may carry a package policy.
        if header.AceType != ACCESS_ALLOWED_TYPE && header.AceType != ACCESS_DENIED_TYPE {
            return Err(failure("UNSUPPORTED_ACE_REQUIRES_REVIEW", 0));
        }
        let sid_offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        if (header.AceSize as usize) < sid_offset + 8 {
            return Err(failure("ACE_SID_TRUNCATED", 0));
        }
        let sid = unsafe { (raw as *mut u8).add(sid_offset) };
        let subauthorities = unsafe { *sid.add(1) } as usize;
        let sid_bytes = 8usize
            .checked_add(subauthorities * 4)
            .filter(|size| *size <= 68 && sid_offset + *size <= header.AceSize as usize)
            .ok_or_else(|| failure("ACE_SID_SIZE", 0))?;
        if unsafe { IsValidSid(sid.cast()) } == 0
            || unsafe { GetLengthSid(sid.cast()) } as usize != sid_bytes
        {
            return Err(failure("ACE_SID_INVALID", 0));
        }
        let authority = unsafe { std::slice::from_raw_parts(sid.add(2), 6) };
        if authority == [0, 0, 0, 0, 0, 15] {
            return Err(failure("PACKAGE_POLICY_CONFLICT", 0));
        }
        let flags = u32::from(header.AceFlags) & !INHERITED_ACE;
        let expected_flags = if directory { DIRECTORY_FLAGS } else { 0 };
        let mask = unsafe { ptr::read_unaligned((raw as *const u8).add(4) as *const u32) };
        if header.AceType != ACCESS_ALLOWED_TYPE
            || flags != expected_flags
            || mask != FILE_ALL_ACCESS
        {
            compliant = false;
        }
        if unsafe { EqualSid(sid.cast(), policy.owner.as_ptr()) } != 0 {
            owner_count += 1;
        } else if unsafe { EqualSid(sid.cast(), policy.system.as_ptr()) } != 0 {
            system_count += 1;
        } else {
            compliant = false;
        }
    }
    let system_expected = if policy.owner_is_system() { 0 } else { 1 };
    Ok(compliant
        && owner_count == 1
        && system_count == system_expected
        && count == 1 + system_expected)
}

fn repair_object(file: &File, policy: &Policy, directory: bool) -> io::Result<()> {
    // The repair traversal opens this handle with MAXIMUM_ALLOWED from the
    // outset and without FILE_SHARE_DELETE. Reopening a second handle with
    // DELETE would conflict with that namespace pin. This same handle audits,
    // repairs and verifies; no privilege absent from the token is enabled.
    let writable = file;
    let original_identity = file_identity(file)?;
    if private_object(writable)? != directory {
        return Err(failure("OBJECT_KIND_CHANGED", 0));
    }
    let mut old_owner = ptr::null_mut();
    let mut old_descriptor = ptr::null_mut();
    let read_code = unsafe {
        GetSecurityInfo(
            writable.as_raw_handle() as _,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut old_owner,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut old_descriptor,
        )
    };
    if read_code != 0 {
        return Err(failure("OWNER_READ", read_code));
    }
    let old_descriptor = NativeAllocation(old_descriptor);
    let same_owner = !old_owner.is_null()
        && unsafe { IsValidSid(old_owner) } != 0
        && unsafe { EqualSid(old_owner, policy.owner.as_ptr()) } != 0;
    let mut descriptor = policy.descriptor(directory)?;
    let owner_flags = if same_owner {
        0
    } else {
        OWNER_SECURITY_INFORMATION
    };
    let code = unsafe {
        SetSecurityInfo(
            writable.as_raw_handle() as _,
            SE_FILE_OBJECT,
            owner_flags | DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            if same_owner {
                ptr::null_mut()
            } else {
                policy.owner.as_ptr()
            },
            ptr::null_mut(),
            descriptor.value.Dacl,
            ptr::null(),
        )
    };
    if code != 0 {
        return Err(failure("SECURITY_WRITE", code));
    }
    if private_object(writable)? != directory
        || file_identity(writable)? != original_identity
        || !descriptor_is_private(writable, policy, directory, true)?
    {
        return Err(failure("REPAIR_READBACK", 0));
    }
    // Keep descriptor allocations alive through the native call and readback.
    let _ = descriptor.as_ptr();
    drop(old_descriptor);
    Ok(())
}

#[derive(Debug)]
struct DirectoryEntry {
    name: OsString,
    identity: u64,
    directory: bool,
}

fn directory_entries(
    directory: &File,
    remaining: usize,
    start: Instant,
    duration: Duration,
) -> io::Result<Vec<DirectoryEntry>> {
    directory_entries_checked(directory, remaining, start, duration, None)
}

fn directory_entries_checked(
    directory: &File,
    remaining: usize,
    start: Instant,
    duration: Duration,
    budget: Option<&OperationBudget>,
) -> io::Result<Vec<DirectoryEntry>> {
    let header_size = offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
    let mut storage = vec![0u64; 8192];
    let bytes = storage.len() * size_of::<u64>();
    let mut restart = true;
    let mut entries = Vec::new();
    loop {
        if let Some(budget) = budget {
            budget.check()?;
        }
        if start.elapsed() > duration {
            return Err(failure("MIGRATION_BUDGET_EXCEEDED", 0));
        }
        let ok = unsafe {
            GetFileInformationByHandleEx(
                directory.as_raw_handle() as _,
                if restart {
                    FileIdBothDirectoryRestartInfo
                } else {
                    FileIdBothDirectoryInfo
                },
                storage.as_mut_ptr().cast(),
                bytes as u32,
            )
        };
        if ok == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(18) {
                break;
            } // ERROR_NO_MORE_FILES
            return Err(failure(
                "DIRECTORY_ENUMERATE",
                error.raw_os_error().unwrap_or(0) as u32,
            ));
        }
        restart = false;
        let raw = storage.as_ptr() as *const u8;
        let mut offset = 0usize;
        loop {
            if let Some(budget) = budget {
                budget.check()?;
            }
            if start.elapsed() > duration {
                return Err(failure("MIGRATION_BUDGET_EXCEEDED", 0));
            }
            if offset + header_size > bytes {
                return Err(failure("DIRECTORY_RECORD", 0));
            }
            // The fixed fields end before FileName. Read those fields without
            // reading a full trailing-array struct beyond the returned record.
            let record = unsafe { raw.add(offset) as *const FILE_ID_BOTH_DIR_INFO };
            let next =
                unsafe { ptr::read_unaligned(ptr::addr_of!((*record).NextEntryOffset)) } as usize;
            let name_bytes =
                unsafe { ptr::read_unaligned(ptr::addr_of!((*record).FileNameLength)) } as usize;
            let attributes =
                unsafe { ptr::read_unaligned(ptr::addr_of!((*record).FileAttributes)) };
            let id = unsafe { ptr::read_unaligned(ptr::addr_of!((*record).FileId)) } as u64;
            let end = offset
                .checked_add(header_size)
                .and_then(|n| n.checked_add(name_bytes))
                .filter(|end| *end <= bytes)
                .ok_or_else(|| failure("DIRECTORY_NAME_SIZE", 0))?;
            if name_bytes == 0 || name_bytes % 2 != 0 || (next != 0 && offset + next < end) {
                return Err(failure("DIRECTORY_NAME_FORMAT", 0));
            }
            let name = unsafe {
                OsString::from_wide(std::slice::from_raw_parts(
                    raw.add(offset + header_size).cast::<u16>(),
                    name_bytes / 2,
                ))
            };
            if name != OsStr::new(".") && name != OsStr::new("..") {
                if !name_is_valid(&name) || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    return Err(failure("TREE_REPARSE_OR_ALIAS", 0));
                }
                if entries.len() >= remaining {
                    return Err(failure("TREE_OBJECT_LIMIT", 0));
                }
                if id == 0 {
                    return Err(failure("TREE_UNSUPPORTED_ID", 0));
                }
                entries.push(DirectoryEntry {
                    name,
                    identity: id,
                    directory: attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                });
            }
            if next == 0 {
                break;
            }
            if next < header_size || next % 8 != 0 || offset + next >= bytes {
                return Err(failure("DIRECTORY_NEXT_RECORD", 0));
            }
            offset += next;
        }
    }
    Ok(entries)
}

struct Walk<'a> {
    options: &'a PrivateTreeOptions,
    start: Instant,
    report: PrivateTreeReport,
    identities: HashSet<FileIdentity>,
    repair: bool,
}
impl Walk<'_> {
    fn check_budget(&self, depth: usize) -> io::Result<()> {
        if depth > self.options.max_depth
            || self.report.checked_objects >= self.options.max_objects
            || self.start.elapsed() > self.options.max_duration
        {
            return Err(failure("MIGRATION_BUDGET_EXCEEDED", 0));
        }
        Ok(())
    }

    fn visit(&mut self, file: &File, policy: &Policy, depth: usize, deep: bool) -> io::Result<()> {
        self.check_budget(depth)?;
        let directory = private_object(file)?;
        let identity = file_identity(file)?;
        if !self.identities.insert(identity) {
            return Err(failure("DUPLICATE_OBJECT", 0));
        }
        self.report.checked_objects += 1;
        if !descriptor_is_private(file, policy, directory, depth == 0)? {
            if !self.repair {
                return Err(failure("ACL_NOT_PRIVATE", 0));
            }
            if self.report.repaired_objects >= self.options.max_repairs {
                return Err(failure("REPAIR_BUDGET_EXCEEDED", 0));
            }
            repair_object(file, policy, directory)?;
            self.report.repaired_objects += 1;
        }
        if deep && directory {
            let remaining = self
                .options
                .max_objects
                .saturating_sub(self.report.checked_objects);
            let entries =
                directory_entries(file, remaining, self.start, self.options.max_duration)?;
            for entry in entries {
                self.check_budget(depth + 1)?;
                let access = if self.repair {
                    MAXIMUM_ALLOWED
                } else {
                    READ_CONTROL
                        | FILE_READ_ATTRIBUTES
                        | if entry.directory {
                            FILE_LIST_DIRECTORY | FILE_TRAVERSE
                        } else {
                            0
                        }
                };
                let child = open_relative(
                    file,
                    &entry.name,
                    access,
                    entry.directory,
                    false,
                    ptr::null_mut(),
                    self.repair,
                )?;
                let current = file_identity(&child)?;
                if current.volume_serial != identity.volume_serial
                    || current.file_index != entry.identity
                {
                    return Err(failure("TREE_CHANGED_DURING_OPEN", 0));
                }
                self.visit(&child, policy, depth + 1, true)?;
            }
        }
        if self.start.elapsed() > self.options.max_duration {
            return Err(failure("MIGRATION_BUDGET_EXCEEDED", 0));
        }
        Ok(())
    }
}

fn private_directory(
    path: &Path,
    options: &PrivateTreeOptions,
    repair: bool,
    deep: bool,
    start: Instant,
) -> io::Result<PrivateTreeReport> {
    if options.max_objects == 0
        || options.max_objects > 200_000
        || options.max_depth == 0
        || options.max_depth > 128
        || options.max_repairs > options.max_objects
        || options.max_duration.is_zero()
        || options.max_duration > Duration::from_secs(60)
        || path.file_name().is_none()
    {
        return Err(failure("OPTIONS_OR_PRIVATE_ROOT_INVALID", 0));
    }
    if !matches!(path.components().next(),
        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    ) {
        return Err(failure("PRIVATE_ROOT_REQUIRES_LOCAL_DISK", 0));
    }
    if start.elapsed() > options.max_duration {
        return Err(failure("MIGRATION_BUDGET_EXCEEDED", 0));
    }
    let policy = Policy::new()?;
    let opened = open_path(
        path,
        if repair { Some(&policy) } else { None },
        repair,
        if repair {
            MAXIMUM_ALLOWED
        } else {
            READ_CONTROL | FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY | FILE_TRAVERSE
        },
    )?;
    if !private_object(opened.target())? {
        return Err(failure("DIRECTORY_REQUIRED", 0));
    }
    let mut walk = Walk {
        options,
        start,
        repair,
        identities: HashSet::new(),
        report: PrivateTreeReport {
            checked_objects: 0,
            repaired_objects: 0,
            elapsed_ms: 0,
            root_identity: file_identity(opened.target())?,
        },
    };
    if let Err(error) = walk.visit(opened.target(), &policy, 0, deep) {
        if let Some(detail) = error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<StorageFailure>())
        {
            let mut detail = detail.clone();
            detail.progress = Some((
                walk.report.checked_objects,
                walk.report.repaired_objects,
                start.elapsed().as_millis(),
            ));
            return Err(io::Error::new(error.kind(), detail));
        }
        return Err(io::Error::new(error.kind(), format!(
            "{error}; checked={}; repaired={}; elapsed_ms={}; data_preserved=true; unsafe_acl_rollback=false",
            walk.report.checked_objects, walk.report.repaired_objects, start.elapsed().as_millis(),
        )));
    }
    walk.report.elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(walk.report)
}

fn ensure_private(
    path: &Path,
    options: &PrivateTreeOptions,
    deep: bool,
) -> io::Result<PrivateTreeReport> {
    // A second process may share an already secured, active WebView2 cache.
    // Compliant trees need only read access and never request DELETE or writes.
    // Only a proven policy mismatch or a missing root permits migration;
    // inaccessible/unsupported/changing trees and exhausted budgets fail closed.
    let start = Instant::now();
    match private_directory(path, options, false, deep, start) {
        Ok(report) => Ok(report),
        Err(error) if migration_candidate(&error) => {
            private_directory(path, options, true, deep, start)
        }
        Err(error) => Err(error),
    }
}

/// Create missing directories with a private descriptor from birth, then
/// migrate only noncompliant existing objects. Close storage consumers before
/// migrating an existing noncompliant tree. Reparse points, hardlinks and
/// package-policy trees are refused; compliant active trees use read-only audit.
pub fn ensure_private_directory(
    path: &Path,
    options: &PrivateTreeOptions,
) -> io::Result<PrivateTreeReport> {
    ensure_private(path, options, true)
}

/// Prepare only the named app-data parent. MAXIMUM_ALLOWED suppresses native
/// ACE propagation: existing children and active profiles are not rewritten.
/// Call this before creating consumers beneath an inherited app-data parent.
pub fn ensure_private_directory_root(
    path: &Path,
    options: &PrivateTreeOptions,
) -> io::Result<PrivateTreeReport> {
    ensure_private(path, options, false)
}

/// Read-only verification. A shallow audit verifies the exact protected root;
/// a deep audit also inspects each extant child and never modifies live caches.
pub fn audit_private_directory(
    path: &Path,
    deep: bool,
    options: &PrivateTreeOptions,
) -> io::Result<PrivateTreeReport> {
    private_directory(path, options, false, deep, Instant::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Security::{AddAccessDeniedAceEx, WinWorldSid};

    // Tests alter DACLs through handles opened before the alteration. This
    // lets the audit inspect a deliberately bad descriptor without depending
    // on whether that descriptor still grants a fresh open of the test file.
    fn set_test_dacl(file: &File, entries: &[(&OwnedSid, bool, u32, u32)]) {
        let bytes = size_of::<ACL>()
            + entries
                .iter()
                .map(|(sid, _, _, _)| offset_of!(ACCESS_ALLOWED_ACE, SidStart) + sid.bytes as usize)
                .sum::<usize>();
        let mut words = vec![0u32; (bytes + 3) / 4];
        let acl = words.as_mut_ptr() as *mut ACL;
        assert_ne!(
            unsafe { InitializeAcl(acl, (words.len() * 4) as u32, ACL_REVISION) },
            0
        );
        for (sid, denied, mask, flags) in entries {
            let result = unsafe {
                if *denied {
                    AddAccessDeniedAceEx(acl, ACL_REVISION, *flags, *mask, sid.as_ptr())
                } else {
                    AddAccessAllowedAceEx(acl, ACL_REVISION, *flags, *mask, sid.as_ptr())
                }
            };
            assert_ne!(result, 0);
        }
        assert_eq!(
            unsafe {
                SetSecurityInfo(
                    file.as_raw_handle() as _,
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    acl,
                    ptr::null(),
                )
            },
            0
        );
        // A negative fixture must prove Windows retained the requested ACEs;
        // normalized-away flags or merged entries cannot count as rejection.
        let mut actual = ptr::null_mut();
        let mut raw = ptr::null_mut();
        assert_eq!(
            unsafe {
                GetSecurityInfo(
                    file.as_raw_handle() as _,
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    &mut actual,
                    ptr::null_mut(),
                    &mut raw,
                )
            },
            0
        );
        let _allocation = NativeAllocation(raw);
        assert!(!actual.is_null() && unsafe { IsValidAcl(actual) } != 0);
        assert_eq!(usize::from(unsafe { (*actual).AceCount }), entries.len());
        for (index, (sid, denied, mask, flags)) in entries.iter().enumerate() {
            let mut ace = ptr::null_mut();
            assert_ne!(unsafe { GetAce(actual, index as u32, &mut ace) }, 0);
            let header = unsafe { ptr::read_unaligned(ace as *const ACE_HEADER) };
            let expected_type = if *denied {
                ACCESS_DENIED_TYPE
            } else {
                ACCESS_ALLOWED_TYPE
            };
            assert_eq!(header.AceType, expected_type);
            assert_eq!(u32::from(header.AceFlags), *flags);
            let mask_pointer = unsafe { (ace as *const u8).add(4).cast::<u32>() };
            let actual_mask = unsafe { ptr::read_unaligned(mask_pointer) };
            assert_eq!(actual_mask, *mask);
            let offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
            assert!(usize::from(header.AceSize) >= offset + sid.bytes as usize);
            let actual_sid = unsafe { (ace as *mut u8).add(offset) };
            assert_ne!(unsafe { EqualSid(actual_sid.cast(), sid.as_ptr()) }, 0);
        }
    }

    // Windows chooses a new object's owner from TokenOwner, which can be
    // Administrators for an elevated runner. These DACL tests require the
    // current user as their starting owner; never repair or replace the DACL.
    fn set_fixture_owner(file: &File, policy: &Policy) {
        let identity = file_identity(file).unwrap();
        assert_eq!(
            unsafe {
                SetSecurityInfo(
                    file.as_raw_handle() as _,
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION,
                    policy.owner.as_ptr(),
                    ptr::null_mut(),
                    ptr::null(),
                    ptr::null(),
                )
            },
            0
        );
        let mut owner = ptr::null_mut();
        let mut raw = ptr::null_mut();
        let result = unsafe {
            GetSecurityInfo(
                file.as_raw_handle() as _,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION,
                &mut owner,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                &mut raw,
            )
        };
        let _allocation = NativeAllocation(raw);
        assert_eq!(result, 0);
        assert!(
            !owner.is_null()
                && unsafe { IsValidSid(owner) } != 0
                && unsafe { EqualSid(owner, policy.owner.as_ptr()) } != 0
        );
        assert_eq!(file_identity(file).unwrap(), identity);
    }

    fn test_file(label: &str, policy: &Policy) -> (PathBuf, OpenedPath) {
        let root = fixture(label);
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("data"), b"private bytes").unwrap();
        let opened = open_path(&root.join("data"), None, true, MAXIMUM_ALLOWED).unwrap();
        set_fixture_owner(opened.target(), policy);
        assert!(descriptor_is_private(opened.target(), policy, false, false).unwrap());
        (root, opened)
    }

    fn assert_bad_file_then_restore(root: PathBuf, file: OpenedPath, policy: &Policy) {
        assert!(!descriptor_is_private(file.target(), policy, false, false).unwrap());
        assert!(audit_private_directory(&root, true, &PrivateTreeOptions::default()).is_err());
        repair_object(file.target(), policy, false).unwrap();
        drop(file);
        assert_eq!(std::fs::read(root.join("data")).unwrap(), b"private bytes");
        std::fs::remove_dir_all(root).unwrap();
    }

    fn owner_entries(policy: &Policy, denied: bool, mask: u32) -> Vec<(&OwnedSid, bool, u32, u32)> {
        let mut entries = vec![(&policy.owner, denied, mask, 0)];
        if !policy.owner_is_system() {
            entries.push((&policy.system, false, FILE_ALL_ACCESS, 0));
        }
        entries
    }

    fn fixture(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "browseros-native-security-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn native_private_tree_is_idempotent_and_preserves_bytes() {
        let root = fixture("migration");
        std::fs::create_dir_all(root.join("existing")).unwrap();
        std::fs::write(root.join("existing/data.bin"), b"preserve these bytes").unwrap();
        let first = ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        assert_eq!(first.checked_objects, 3);
        assert_eq!(
            std::fs::read(root.join("existing/data.bin")).unwrap(),
            b"preserve these bytes"
        );
        let second = ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        assert_eq!(second.repaired_objects, 0);
        assert_eq!(first.root_identity, second.root_identity);
        let audited = audit_private_directory(&root, true, &PrivateTreeOptions::default()).unwrap();
        assert_eq!(audited.checked_objects, 3);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn new_children_inherit_only_native_owner_and_system() {
        let base = fixture("inherit");
        let root = base.join("middle").join("private");
        // Every missing component must be created through its pinned parent;
        // reopening only a preexisting private root misses this native path.
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::create_dir(root.join("cache")).unwrap();
        std::fs::write(root.join("cache/entry"), b"cache").unwrap();
        let policy = Policy::new().unwrap();
        // Owner selection is separate from ACE inheritance. Keep the native
        // inherited DACL bytes/flags under test instead of calling repair.
        for path in [root.join("cache"), root.join("cache/entry")] {
            let opened = open_path(&path, None, true, MAXIMUM_ALLOWED).unwrap();
            set_fixture_owner(opened.target(), &policy);
        }
        let result = audit_private_directory(&root, true, &PrivateTreeOptions::default()).unwrap();
        assert_eq!(result.checked_objects, 3);
        assert_eq!(
            ensure_private_directory(&root, &PrivateTreeOptions::default())
                .unwrap()
                .repaired_objects,
            0
        );
        assert_eq!(
            audit_private_directory(&base, true, &PrivateTreeOptions::default())
                .unwrap()
                .checked_objects,
            5
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn private_read_write_file_preserves_identity_and_rejects_existing_extra_aces() {
        use std::io::Write;

        let root = fixture("read-write");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        let path = root.join("audit-write.lock");
        let first = open_private_read_write_file(&path).unwrap();
        let mut writer = first.file();
        writer.write_all(b"preserved").unwrap();
        first.file().sync_all().unwrap();
        audit_private_file(first.file()).unwrap();
        let identity = file_identity(first.file()).unwrap();
        let second = open_private_read_write_file(&path).unwrap();
        assert_eq!(file_identity(second.file()).unwrap(), identity);
        assert_eq!(std::fs::read(&path).unwrap(), b"preserved");
        let moved = root.with_extension("moved");
        assert!(
            std::fs::rename(&root, &moved).is_err(),
            "the private file must keep its parent namespace pinned"
        );
        drop(second);
        drop(first);
        std::fs::rename(&root, &moved).unwrap();
        std::fs::rename(&moved, &root).unwrap();

        let policy = Policy::new().unwrap();
        let opened = open_path(&path, None, true, MAXIMUM_ALLOWED).unwrap();
        let mut world = OwnedSid {
            words: vec![0; 17],
            bytes: 68,
        };
        assert_ne!(
            unsafe {
                CreateWellKnownSid(
                    WinWorldSid,
                    ptr::null_mut(),
                    world.as_mut_ptr(),
                    &mut world.bytes,
                )
            },
            0
        );
        let mut entries = owner_entries(&policy, false, FILE_ALL_ACCESS);
        entries.push((&world, false, READ_CONTROL | FILE_READ_ATTRIBUTES | 1, 0));
        set_test_dacl(opened.target(), &entries);
        drop(opened);
        let error = match open_private_read_write_file(&path) {
            Ok(_) => panic!("an existing broad ACL must not be accepted or repaired"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("PRIVATE_FILE_ACL_NOT_PRIVATE"));
        let unchanged = open_path(&path, None, true, MAXIMUM_ALLOWED).unwrap();
        assert_eq!(file_identity(unchanged.target()).unwrap(), identity);
        assert!(!descriptor_is_private(unchanged.target(), &policy, false, false).unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), b"preserved");
        drop(unchanged);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hardlinked_data_is_refused_without_changing_the_external_file() {
        let root = fixture("hardlink");
        let outside = fixture("external");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(&outside, b"external").unwrap();
        std::fs::hard_link(&outside, root.join("alias")).unwrap();
        let error = ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap_err();
        assert!(error.to_string().contains("HARDLINK_DENIED"));
        assert_eq!(std::fs::read(&outside).unwrap(), b"external");
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_file(outside).unwrap();
    }

    #[test]
    fn native_path_identity_rejects_a_sibling_and_accepts_own_child() {
        let root = fixture("root");
        let outside = fixture("sibling");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        ensure_private_directory(&outside, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("child"), b"inside").unwrap();
        std::fs::write(outside.join("child"), b"outside").unwrap();
        let trusted = open_path(&root, None, false, FILE_READ_ATTRIBUTES).unwrap();
        let own = File::open(root.join("child")).unwrap();
        let foreign = File::open(outside.join("child")).unwrap();
        assert!(handle_is_within_root(&own, trusted.target()).unwrap());
        assert!(!handle_is_within_root(&foreign, trusted.target()).unwrap());
        assert!(!path_matches_handle(&root.join("child"), &foreign).unwrap());
        drop((trusted, own, foreign));
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn unicode_ordinal_comparison_does_not_use_lossy_utf8() {
        assert!(same_path_component(OsStr::new("JÖRG"), OsStr::new("jörg")));
        assert!(!same_path_component(
            OsStr::new("workspace"),
            OsStr::new("workspace-other")
        ));
        let left = OsString::from_wide(&[0xd800]);
        let right = OsString::from_wide(&[0xd801]);
        assert!(!same_path_component(&left, &right));
    }

    #[test]
    fn low_object_budget_returns_incomplete_error_instead_of_pass() {
        let root = fixture("budget");
        std::fs::create_dir_all(root.join("child")).unwrap();
        let options = PrivateTreeOptions {
            max_objects: 1,
            max_repairs: 1,
            ..PrivateTreeOptions::default()
        };
        assert!(ensure_private_directory(&root, &options).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn an_exact_native_sid_is_required_for_each_ace() {
        let policy = Policy::new().unwrap();
        let (root, file) = test_file("sid", &policy);
        let mut other = OwnedSid {
            words: vec![0; policy.owner.words.len()],
            bytes: policy.owner.bytes,
        };
        assert_ne!(
            unsafe { CopySid(other.bytes, other.as_mut_ptr(), policy.owner.as_ptr()) },
            0
        );
        let count = unsafe { *((other.as_ptr() as *const u8).add(1)) } as usize;
        assert!(count > 0);
        other.words[2 + count - 1] ^= 0x8000_0000;
        assert_ne!(unsafe { IsValidSid(other.as_ptr()) }, 0);
        assert_eq!(
            unsafe { EqualSid(other.as_ptr(), policy.owner.as_ptr()) },
            0
        );
        let mut entries = vec![(&other, false, FILE_ALL_ACCESS, 0)];
        if !policy.owner_is_system() {
            entries.push((&policy.system, false, FILE_ALL_ACCESS, 0));
        }
        set_test_dacl(file.target(), &entries);
        assert_bad_file_then_restore(root, file, &policy);
    }

    #[test]
    fn an_owner_deny_ace_is_not_an_owner_allow_ace() {
        let policy = Policy::new().unwrap();
        let (root, file) = test_file("deny", &policy);
        set_test_dacl(
            file.target(),
            &owner_entries(&policy, true, FILE_ALL_ACCESS),
        );
        assert_bad_file_then_restore(root, file, &policy);
    }

    #[test]
    fn an_owner_read_only_mask_is_not_full_control() {
        let policy = Policy::new().unwrap();
        let (root, file) = test_file("mask", &policy);
        set_test_dacl(
            file.target(),
            &owner_entries(
                &policy,
                false,
                READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE | 1,
            ),
        );
        assert_bad_file_then_restore(root, file, &policy);
    }

    #[test]
    fn an_extra_world_ace_invalidates_an_otherwise_private_file() {
        let policy = Policy::new().unwrap();
        let (root, file) = test_file("world", &policy);
        let mut world = OwnedSid {
            words: vec![0; 17],
            bytes: 68,
        };
        assert_ne!(
            unsafe {
                CreateWellKnownSid(
                    WinWorldSid,
                    ptr::null_mut(),
                    world.as_mut_ptr(),
                    &mut world.bytes,
                )
            },
            0
        );
        let mut entries = owner_entries(&policy, false, FILE_ALL_ACCESS);
        entries.push((&world, false, READ_CONTROL | FILE_READ_ATTRIBUTES | 1, 0));
        set_test_dacl(file.target(), &entries);
        assert_bad_file_then_restore(root, file, &policy);
    }

    #[test]
    fn owner_and_system_counts_are_checked_per_object() {
        let policy = Policy::new().unwrap();
        if policy.owner_is_system() {
            // SYSTEM is the sole trustee in this policy. Exercise its exact
            // one-ACE count instead of silently skipping the negative case.
            let (root, file) = test_file("single-trustee-count", &policy);
            set_test_dacl(
                file.target(),
                &[
                    (&policy.owner, false, FILE_ALL_ACCESS, 0),
                    (&policy.owner, false, FILE_ALL_ACCESS, 0),
                ],
            );
            assert_bad_file_then_restore(root, file, &policy);
            return;
        }
        let root = fixture("per-object");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("a"), b"a").unwrap();
        std::fs::write(root.join("b"), b"b").unwrap();
        let a = open_path(&root.join("a"), None, true, MAXIMUM_ALLOWED).unwrap();
        let b = open_path(&root.join("b"), None, true, MAXIMUM_ALLOWED).unwrap();
        set_fixture_owner(a.target(), &policy);
        set_fixture_owner(b.target(), &policy);
        assert!(descriptor_is_private(a.target(), &policy, false, false).unwrap());
        assert!(descriptor_is_private(b.target(), &policy, false, false).unwrap());
        // Across both files the totals are exactly two owner and two SYSTEM
        // ACEs, but neither individual object has its required pair.
        set_test_dacl(
            a.target(),
            &[
                (&policy.owner, false, FILE_ALL_ACCESS, 0),
                (&policy.owner, false, FILE_ALL_ACCESS, 0),
            ],
        );
        set_test_dacl(
            b.target(),
            &[
                (&policy.system, false, FILE_ALL_ACCESS, 0),
                (&policy.system, false, FILE_ALL_ACCESS, 0),
            ],
        );
        assert!(!descriptor_is_private(a.target(), &policy, false, false).unwrap());
        assert!(!descriptor_is_private(b.target(), &policy, false, false).unwrap());
        assert!(audit_private_directory(&root, true, &PrivateTreeOptions::default()).is_err());
        repair_object(a.target(), &policy, false).unwrap();
        repair_object(b.target(), &policy, false).unwrap();
        drop((a, b));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_inheritance_flags_are_required_and_file_flags_are_empty() {
        let policy = Policy::new().unwrap();
        let (root, file) = test_file("flags", &policy);
        // The file's actual inherited ACEs have no propagation flags. Do not
        // fabricate OI/CI on a file: Windows can normalize those flags away.
        assert!(descriptor_is_private(file.target(), &policy, false, false).unwrap());
        drop(file);
        let directory = open_path(&root, None, true, MAXIMUM_ALLOWED).unwrap();
        for flags in [0, OBJECT_INHERIT_ACE, CONTAINER_INHERIT_ACE] {
            let mut entries = owner_entries(&policy, false, FILE_ALL_ACCESS);
            for entry in &mut entries {
                entry.3 = flags;
            }
            // set_test_dacl reads every native ACE back before the policy test.
            set_test_dacl(directory.target(), &entries);
            assert!(!descriptor_is_private(directory.target(), &policy, true, true).unwrap());
            assert!(audit_private_directory(&root, true, &PrivateTreeOptions::default()).is_err());
            repair_object(directory.target(), &policy, true).unwrap();
            assert!(descriptor_is_private(directory.target(), &policy, true, true).unwrap());
        }
        drop(directory);
        assert_eq!(std::fs::read(root.join("data")).unwrap(), b"private bytes");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shallow_parent_preparation_preserves_existing_child_dacl() {
        let policy = Policy::new().unwrap();
        let root = fixture("shallow");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("child"), b"child").unwrap();
        // A deliberately non-private child must remain untouched by preparing
        // the parent. Native MAXIMUM_ALLOWED suppresses implicit propagation.
        {
            let child = open_path(&root.join("child"), None, true, MAXIMUM_ALLOWED).unwrap();
            // Elevated tokens may create files owned by Administrators.
            // Establish this test's owner while the original full-access
            // handle exists, then remove write rights to test shallow repair.
            repair_object(child.target(), &policy, false).unwrap();
            assert!(descriptor_is_private(child.target(), &policy, false, true).unwrap());
            set_test_dacl(
                child.target(),
                &owner_entries(
                    &policy,
                    false,
                    READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE | 1,
                ),
            );
        }
        let report = ensure_private_directory_root(&root, &PrivateTreeOptions::default()).unwrap();
        assert_eq!(report.checked_objects, 1);
        assert!(audit_private_directory(&root, false, &PrivateTreeOptions::default()).is_ok());
        assert!(audit_private_directory(&root, true, &PrivateTreeOptions::default()).is_err());
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_compliant_live_file_never_requires_a_mutating_open() {
        let root = fixture("live");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("active"), b"active").unwrap();
        {
            let opened = open_path(&root.join("active"), None, true, MAXIMUM_ALLOWED).unwrap();
            set_fixture_owner(opened.target(), &Policy::new().unwrap());
        }
        let active = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(root.join("active"))
            .unwrap();
        let report = ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        assert_eq!(report.checked_objects, 2);
        assert_eq!(report.repaired_objects, 0);
        drop(active);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn namespace_comparison_accepts_unc_spelling_without_cross_share_aliases() {
        assert!(same_namespace(
            Path::new(r"\\server\share\folder"),
            Path::new(r"\\?\UNC\SERVER\SHARE\folder")
        ));
        assert!(!same_namespace(
            Path::new(r"\\server\share\folder"),
            Path::new(r"\\server\another-share\folder")
        ));
        assert!(!same_namespace(
            Path::new(r"\\server\share\folder"),
            Path::new(r"\\another-server\share\folder")
        ));
        assert!(!same_namespace(
            Path::new(r"C:\folder"),
            Path::new(r"D:\folder")
        ));
    }
}
