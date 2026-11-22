//! Native, owner-bound recovery primitives. Records contain identities, never
//! deletion paths. The caller resolves AppContainer locations through userenv.
use super::*;
use std::os::windows::io::RawHandle;
use std::mem::size_of_val;
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::Security::Authorization::SE_KERNEL_OBJECT;
use windows_sys::Win32::Security::{SECURITY_ATTRIBUTES, INHERIT_ONLY_ACE, WinBuiltinAdministratorsSid};
use windows_sys::Win32::Storage::FileSystem::{
    SetFileInformationByHandle, FileDispositionInfoEx, FILE_DISPOSITION_INFO_EX,
    FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
    FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
    OpenFileById, FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0, FileIdType,
    FILE_GENERIC_READ, FILE_GENERIC_EXECUTE,
};
use windows_sys::Win32::System::JobObjects::{
    CreateJobObjectW, OpenJobObjectW, SetInformationJobObject, QueryInformationJobObject,
    TerminateJobObject, JobObjectExtendedLimitInformation, JobObjectBasicAccountingInformation,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_ALL_ACCESS,
    JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
};

const RECORD_ROOT: &str = "sandbox-profile-journal-v1";
const RECORD_ACCESS: u32 = READ_CONTROL | FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY | FILE_TRAVERSE;
const JOB_PREFIX: &str = r"Global\BrowserOS.Coding.Job.v1.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeProfileIdentity {
    pub file: FileIdentity,
    pub volume_guid: String,
    pub created: u64,
}

fn current_owner(file: &File, policy: &Policy) -> io::Result<bool> {
    let mut owner = ptr::null_mut();
    let mut raw = ptr::null_mut();
    let code = unsafe {
        GetSecurityInfo(file.as_raw_handle() as _, SE_FILE_OBJECT, OWNER_SECURITY_INFORMATION,
            &mut owner, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut raw)
    };
    let _allocation = NativeAllocation(raw);
    if code != 0 { return Err(failure("RECOVERY_OWNER_READ", code)); }
    Ok(!owner.is_null() && unsafe { IsValidSid(owner) } != 0
        && unsafe { EqualSid(owner, policy.owner.as_ptr()) } != 0)
}

pub(super) fn profile_identity(file: &File) -> io::Result<NativeProfileIdentity> {
    let policy = Policy::new()?;
    if !private_object(file)? || !current_owner(file, &policy)? {
        return Err(failure("RECOVERY_PROFILE_OWNER", 0));
    }
    let info = file_information(file)?;
    let mut buffer = vec![0u16; 32768];
    let used = unsafe {
        GetFinalPathNameByHandleW(file.as_raw_handle() as _, buffer.as_mut_ptr(),
            buffer.len() as u32, 1 /* VOLUME_NAME_GUID */)
    } as usize;
    let prefix: Vec<u16> = r"\\?\Volume{".encode_utf16().collect();
    let end = prefix.len() + 36;
    if used <= end || used >= buffer.len() || !buffer[..used].starts_with(&prefix)
        || buffer[end] != b'}' as u16
    { return Err(failure("RECOVERY_VOLUME_NAMESPACE", 0)); }
    let guid = &buffer[prefix.len()..end];
    if guid.iter().enumerate().any(|(index, &unit)|
        if matches!(index, 8 | 13 | 18 | 23) { unit != b'-' as u16 }
        else { unit > 127 || !(unit as u8).is_ascii_hexdigit() })
    { return Err(failure("RECOVERY_VOLUME_GUID", 0)); }
    Ok(NativeProfileIdentity {
        file: file_identity(file)?,
        volume_guid: String::from_utf16(guid).map_err(|_| failure("RECOVERY_VOLUME_GUID", 0))?.to_ascii_lowercase(),
        created: (u64::from(info.ftCreationTime.dwHighDateTime) << 32)
            | u64::from(info.ftCreationTime.dwLowDateTime),
    })
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeProfileScope {
    pub parent: NativeProfileIdentity,
    /// Verification only. Never used as an open/delete pathname from a record.
    pub parent_path: Vec<u16>,
    pub component: String,
}
pub struct ProfileCleanupNamespace {
    opened: OpenedPath,
    expected: NativeProfileIdentity,
}
fn valid_guid(value: &str) -> bool {
    value.len() == 36 && value.bytes().enumerate().all(|(index, byte)|
        if matches!(index, 8 | 13 | 18 | 23) { byte == b'-' }
        else { byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase() })
}
fn audit_namespace_writers(file: &File, package: &AppContainerPolicy) -> io::Result<()> {
    let policy = Policy::new()?;
    if !private_object(file)? || !current_owner(file, &policy)? {
        return Err(failure("RECOVERY_NAMESPACE_OWNER", 0));
    }
    let mut administrators = OwnedSid { words: vec![0; 17], bytes: 68 };
    if unsafe { CreateWellKnownSid(WinBuiltinAdministratorsSid, ptr::null_mut(),
        administrators.as_mut_ptr(), &mut administrators.bytes) } == 0
    { return Err(last_failure("RECOVERY_ADMIN_SID")); }
    let mut acl = ptr::null_mut();
    let mut raw = ptr::null_mut();
    let code = unsafe { GetSecurityInfo(file.as_raw_handle() as _, SE_FILE_OBJECT,
        DACL_SECURITY_INFORMATION, ptr::null_mut(), ptr::null_mut(), &mut acl, ptr::null_mut(), &mut raw) };
    let _allocation = NativeAllocation(raw);
    if code != 0 { return Err(failure("RECOVERY_NAMESPACE_DACL", code)); }
    if acl.is_null() || unsafe { IsValidAcl(acl) } == 0 { return Err(failure("RECOVERY_NAMESPACE_DACL", 0)); }
    let count = unsafe { (*acl).AceCount } as u32;
    if count > 4096 { return Err(failure("RECOVERY_NAMESPACE_ACE_LIMIT", 0)); }
    const READ_ONLY: u32 = 0x8000_0000 | 0x2000_0000 | FILE_GENERIC_READ | FILE_GENERIC_EXECUTE;
    for index in 0..count {
        let mut ace = ptr::null_mut();
        if unsafe { GetAce(acl, index, &mut ace) } == 0 || ace.is_null() { return Err(last_failure("RECOVERY_NAMESPACE_ACE")); }
        let header = unsafe { ptr::read_unaligned(ace as *const ACE_HEADER) };
        let offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        if ![ACCESS_ALLOWED_TYPE, ACCESS_DENIED_TYPE].contains(&header.AceType)
            || (header.AceSize as usize) < offset + 8
        { return Err(failure("RECOVERY_NAMESPACE_UNSUPPORTED_ACE", 0)); }
        let sid = unsafe { (ace as *mut u8).add(offset) };
        let bytes = 8 + 4 * usize::from(unsafe { *sid.add(1) });
        if bytes > 68 || offset + bytes != header.AceSize as usize || unsafe { IsValidSid(sid.cast()) } == 0 {
            return Err(failure("RECOVERY_NAMESPACE_ACE_SID", 0));
        }
        if header.AceType == ACCESS_DENIED_TYPE || u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0 { continue; }
        let mask = unsafe { ptr::read_unaligned((ace as *const u8).add(4) as *const u32) };
        if mask & !READ_ONLY == 0 { continue; }
        let trusted = [&policy.owner, &policy.system, &administrators, package.package_sid()]
            .iter().any(|trusted| unsafe { EqualSid(sid.cast(), trusted.as_ptr()) } != 0);
        if !trusted { return Err(failure("RECOVERY_NAMESPACE_UNTRUSTED_WRITER", 0)); }
    }
    Ok(())
}

pub(super) fn profile_scope(
    opened: &OpenedPath, name: &str, package_sid: &str,
) -> io::Result<NativeProfileScope> {
    let package = AppContainerPolicy::new(package_sid, AppContainerAccess::ReadExecute, false)?;
    let native = final_path(opened.target())?;
    let parts: Vec<_> = native.components().filter_map(|part| match part {
        Component::Normal(name) => Some(name.to_owned()), _ => None,
    }).collect();
    if parts.len() + 1 != opened.chain.len() { return Err(failure("RECOVERY_PROFILE_LAYOUT", 0)); }
    let candidates: Vec<_> = parts.iter().enumerate().filter_map(|(index, part)| {
        (same_path_component(part, OsStr::new(name)) || same_path_component(part, OsStr::new(package_sid)))
            .then_some(index + 1)
    }).collect();
    if candidates.len() != 1 || candidates[0] == 0 { return Err(failure("RECOVERY_PROFILE_LAYOUT", 0)); }
    let boundary = candidates[0];
    let parent = boundary - 1;
    // Support only Packages/<exact profile name or SID> and its AC folder.
    // userenv may delete the whole package, not merely the returned AC path.
    // Unknown layouts are retained; no guessed parent becomes a deletion guard.
    let supported_tail = parts.len() == boundary
        || (parts.len() == boundary + 1
            && same_path_component(&parts[boundary], OsStr::new("AC")));
    if parent == 0 || !same_path_component(&parts[parent - 1], OsStr::new("Packages"))
        || !supported_tail
    { return Err(failure("RECOVERY_PROFILE_LAYOUT", 0)); }
    // This Packages namespace parent is outside the profile that userenv may delete.
    // Keep it fixed across the final name-based API, and disallow any writer
    // other than the owner, SYSTEM, administrators or this inactive package.
    for directory in &opened.chain[parent..] { audit_namespace_writers(directory, &package)?; }
    let parent_path: Vec<_> = final_path(&opened.chain[parent])?.as_os_str().encode_wide().collect();
    let component = parts[parent].to_str().ok_or_else(|| failure("RECOVERY_PROFILE_LAYOUT", 0))?.to_owned();
    Ok(NativeProfileScope { parent: profile_identity(&opened.chain[parent])?, parent_path, component })
}

pub fn hold_profile_cleanup_namespace(
    scope: &NativeProfileScope, name: &str, package_sid: &str, budget: &OperationBudget,
) -> io::Result<ProfileCleanupNamespace> {
    budget.check()?;
    if !valid_guid(&scope.parent.volume_guid) || scope.parent_path.is_empty()
        || scope.parent_path.len() > 32767 || scope.parent_path.contains(&0)
        || !(same_path_component(OsStr::new(&scope.component), OsStr::new(name))
            || same_path_component(OsStr::new(&scope.component), OsStr::new(package_sid)))
    { return Err(failure("RECOVERY_NAMESPACE_RECORD", 0)); }
    // Open by recorded native identity on its recorded OS volume. The stored
    // path is used only to reject a changed namespace, never as an open input.
    let volume = format!(r"\\?\Volume{{{}}}\", scope.parent.volume_guid);
    let hint = OpenOptions::new().access_mode(FILE_READ_ATTRIBUTES | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).open(volume)
        .map_err(|error| failure("RECOVERY_VOLUME_OPEN", error.raw_os_error().unwrap_or(0) as u32))?;
    let descriptor = FILE_ID_DESCRIPTOR {
        dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32, Type: FileIdType,
        Anonymous: FILE_ID_DESCRIPTOR_0 { FileId: scope.parent.file.file_index as i64 },
    };
    let raw = unsafe { OpenFileById(hint.as_raw_handle() as _, &descriptor,
        RECORD_ACCESS | SYNCHRONIZE, share_mode(true), ptr::null(),
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT) };
    if raw == INVALID_HANDLE_VALUE || raw.is_null() { return Err(last_failure("RECOVERY_NAMESPACE_OPEN_ID")); }
    let pinned = unsafe { File::from_raw_handle(raw) };
    if profile_identity(&pinned)? != scope.parent { return Err(failure("RECOVERY_NAMESPACE_IDENTITY", 0)); }
    let native = final_path(&pinned)?;
    if !native.file_name().is_some_and(|component| same_path_component(component, OsStr::new("Packages"))) {
        return Err(failure("RECOVERY_PROFILE_LAYOUT", 0));
    }
    if native.as_os_str().encode_wide().collect::<Vec<_>>() != scope.parent_path {
        return Err(failure("RECOVERY_NAMESPACE_MOVED", 0));
    }
    let opened = open_path_options(&native, None, true, RECORD_ACCESS, 0, None, Some(budget))?;
    if profile_identity(opened.target())? != scope.parent { return Err(failure("RECOVERY_NAMESPACE_CHANGED", 0)); }
    let package = AppContainerPolicy::new(package_sid, AppContainerAccess::ReadExecute, false)?;
    audit_namespace_writers(opened.target(), &package)?;
    // Existing profile base, if still present, must also be non-writable by
    // unrelated principals before its filesystem path is passed to userenv.
    match open_relative(opened.target(), OsStr::new(&scope.component), RECORD_ACCESS,
        true, false, ptr::null_mut(), true)
    {
        Ok(profile) => audit_namespace_writers(&profile, &package)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    drop(pinned);
    drop(hint);
    Ok(ProfileCleanupNamespace { opened, expected: scope.parent.clone() })
}
impl ProfileCleanupNamespace {
    pub fn recheck(&self, package_sid: &str, budget: &OperationBudget) -> io::Result<()> {
        budget.check()?;
        if profile_identity(self.opened.target())? != self.expected { return Err(failure("RECOVERY_NAMESPACE_CHANGED", 0)); }
        let package = AppContainerPolicy::new(package_sid, AppContainerAccess::ReadExecute, false)?;
        audit_namespace_writers(self.opened.target(), &package)
    }
}

fn remove_opened_object(file: &File) -> io::Result<()> {
    // This is the opened name under a held parent, not a string pathname.
    // No DACL or attribute changes are made. Unsupported filesystems fail
    // closed and keep the durable recovery record for a later attempt.
    let disposition = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS
            | FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
    };
    if unsafe {
        SetFileInformationByHandle(file.as_raw_handle() as _, FileDispositionInfoEx,
            (&disposition as *const FILE_DISPOSITION_INFO_EX).cast(), size_of::<FILE_DISPOSITION_INFO_EX>() as u32)
    } == 0 { return Err(last_failure("RECOVERY_OBJECT_DELETE")); }
    Ok(())
}

pub struct PrivateRecordStore { opened: OpenedPath }
impl PrivateRecordStore {
    pub fn open(parent: &Path, budget: &OperationBudget) -> io::Result<Self> {
        budget.check()?;
        let mut options = PrivateTreeOptions { max_duration: budget.remaining()?, ..PrivateTreeOptions::default() };
        // The prepared application data epoch belongs to startup. This is an
        // exact shallow parent check (deep=false), never another subtree scan.
        audit_private_directory(parent, false, &options)?;
        budget.check()?;
        options.max_duration = budget.remaining()?;
        let path = parent.join(RECORD_ROOT);
        ensure_private_directory_root(&path, &options)?;
        budget.check()?;
        let opened = open_path_options(&path, None, true, RECORD_ACCESS, 0, None, Some(budget))?;
        if !descriptor_is_private(opened.target(), &Policy::new()?, true, true)? {
            return Err(failure("RECOVERY_STORE_POLICY", 0));
        }
        Ok(Self { opened })
    }
    pub fn parent_matches(&self, parent: &Path) -> io::Result<bool> {
        if self.opened.chain.len() < 2 { return Ok(false); }
        path_matches_handle(parent, &self.opened.chain[self.opened.chain.len() - 2])
    }
    pub fn entries(&self, limit: usize, budget: &OperationBudget) -> io::Result<Vec<PinnedDirectoryEntry>> {
        budget.check()?;
        // Each observer owns a fresh native enumeration cursor. Reusing the
        // retained directory handle would let a read-only status request reset
        // an in-progress recovery scan's GetFileInformationByHandleEx cursor.
        let parent = self.opened.chain.get(self.opened.chain.len().saturating_sub(2))
            .ok_or_else(|| failure("RECOVERY_STORE_PARENT", 0))?;
        let directory = open_relative(parent, OsStr::new(RECORD_ROOT), RECORD_ACCESS,
            true, false, ptr::null_mut(), true)?;
        if file_identity(&directory)? != file_identity(self.opened.target())?
            || !descriptor_is_private(&directory, &Policy::new()?, true, true)?
        { return Err(failure("RECOVERY_STORE_CHANGED", 0)); }
        let volume = file_identity(&directory)?.volume_serial;
        directory_entries_checked(&directory, limit, Instant::now(), budget.remaining()?, Some(budget))
            .map(|entries| entries.into_iter().map(|entry| PinnedDirectoryEntry {
                name: entry.name, directory: entry.directory,
                identity: FileIdentity { volume_serial: volume, file_index: entry.identity },
            }).collect())
    }
    pub fn create(&self, name: &OsStr, budget: &OperationBudget) -> io::Result<ExclusivePrivateRecord> {
        budget.check()?;
        if !record_name(name) { return Err(failure("RECOVERY_RECORD_NAME", 0)); }
        let policy = Policy::new()?;
        let mut descriptor = policy.descriptor(false)?;
        let raw = unsafe {
            ReOpenFile(self.opened.target().as_raw_handle() as _, 2 | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                share_mode(true), FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        };
        if raw == INVALID_HANDLE_VALUE || raw.is_null() { return Err(last_failure("RECOVERY_RECORD_PARENT")); }
        let parent = unsafe { File::from_raw_handle(raw) };
        if file_identity(&parent)? != file_identity(self.opened.target())? {
            return Err(failure("RECOVERY_RECORD_PARENT_CHANGED", 0));
        }
        let file = open_relative_with_share(&parent, name, MAXIMUM_ALLOWED, false, true,
            descriptor.as_ptr(), FILE_SHARE_READ)?;
        audit_private_file(&file)?;
        Ok(ExclusivePrivateRecord { file, _parents: self.opened.chain.iter().map(File::try_clone).collect::<io::Result<_>>()? })
    }
    /// A diagnostic observation, not a recovery lease. It may read the last
    /// complete journal frame while its owner is appending; it never grants
    /// mutation rights or proves that the owner is inactive.
    pub fn open_readonly(&self, entry: &PinnedDirectoryEntry, budget: &OperationBudget) -> io::Result<File> {
        budget.check()?;
        if entry.directory || !record_name(&entry.name) { return Err(failure("RECOVERY_RECORD_NAME", 0)); }
        let file = open_relative_with_share(self.opened.target(), &entry.name,
            1 /* FILE_READ_DATA */ | READ_CONTROL | FILE_READ_ATTRIBUTES,
            false, false, ptr::null_mut(), FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)?;
        if file_identity(&file)? != entry.identity { return Err(failure("RECOVERY_RECORD_CHANGED", 0)); }
        audit_private_file(&file)?;
        Ok(file)
    }

    /// A live owner holds WRITE+DELETE and refuses write/delete sharing. A
    /// second owner cannot obtain this native lease, even in another process.
    /// Busy is retained, never treated as a stale record that may be deleted.
    pub fn try_open(&self, entry: &PinnedDirectoryEntry, budget: &OperationBudget) -> io::Result<Option<ExclusivePrivateRecord>> {
        budget.check()?;
        if entry.directory || !record_name(&entry.name) { return Err(failure("RECOVERY_RECORD_NAME", 0)); }
        let file = match open_relative_with_share(self.opened.target(), &entry.name, MAXIMUM_ALLOWED,
            false, false, ptr::null_mut(), FILE_SHARE_READ)
        {
            Ok(file) => file,
            Err(error) if error.get_ref().and_then(|inner| inner.downcast_ref::<StorageFailure>())
                .is_some_and(|failure| failure.code == 32) => return Ok(None),
            Err(error) => return Err(error),
        };
        if file_identity(&file)? != entry.identity { return Err(failure("RECOVERY_RECORD_CHANGED", 0)); }
        audit_private_file(&file)?;
        Ok(Some(ExclusivePrivateRecord {
            file, _parents: self.opened.chain.iter().map(File::try_clone).collect::<io::Result<_>>()?,
        }))
    }
}
fn record_name(name: &OsStr) -> bool {
    name.to_str().and_then(|text| text.strip_suffix(".profile")).is_some_and(|id|
        id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
}
pub struct ExclusivePrivateRecord { file: File, _parents: Vec<File> }
impl ExclusivePrivateRecord {
    pub fn file(&self) -> &File { &self.file }
    pub fn file_mut(&mut self) -> &mut File { &mut self.file }
    pub fn mark_deleted(&self) -> io::Result<()> {
        audit_private_file(&self.file)?;
        remove_opened_object(&self.file)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProfileDeleteReport { pub removed_objects: usize, pub root_missing: bool }

/// Delete only a recorded OS-resolved profile tree. Keep every ancestor fixed,
/// refuse aliases/hardlinks, check the recorded volume/file/creation identity,
/// and unlink each opened object postorder. There is no remove_dir_all fallback.
pub fn delete_recorded_profile_tree(
    path: &Path, expected: &NativeProfileIdentity,
    options: &PrivateTreeOptions, budget: &OperationBudget,
) -> io::Result<ProfileDeleteReport> {
    budget.check()?;
    if options.max_objects == 0 || options.max_objects > 200_000
        || options.max_depth == 0 || options.max_depth > 128 || options.max_duration.is_zero()
    { return Err(failure("RECOVERY_DELETE_OPTIONS", 0)); }
    let parent_path = path.parent().ok_or_else(|| failure("RECOVERY_PROFILE_PARENT", 0))?;
    let name = path.file_name().filter(|name| name_is_valid(name))
        .ok_or_else(|| failure("RECOVERY_PROFILE_NAME", 0))?;
    let parent = open_path_options(parent_path, None, true, RECORD_ACCESS, 0, None, Some(budget))?;
    let root = match open_relative(parent.target(), name, MAXIMUM_ALLOWED, true, false, ptr::null_mut(), true) {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ProfileDeleteReport { removed_objects: 0, root_missing: true });
        }
        Err(error) => return Err(error),
    };
    if profile_identity(&root)? != *expected { return Err(failure("RECOVERY_PROFILE_IDENTITY_CHANGED", 0)); }
    struct Walk<'a> {
        options: &'a PrivateTreeOptions, budget: &'a OperationBudget,
        start: Instant, visited: usize, removed: usize, seen: HashSet<FileIdentity>,
    }
    impl Walk<'_> {
        fn check(&self, depth: usize) -> io::Result<()> {
            self.budget.check()?;
            if depth > self.options.max_depth || self.visited >= self.options.max_objects
                || self.start.elapsed() >= self.options.max_duration
            { return Err(failure("RECOVERY_DELETE_BUDGET", 0)); }
            Ok(())
        }
        fn visit(&mut self, file: &File, depth: usize) -> io::Result<()> {
            self.check(depth)?;
            let directory = private_object(file)?;
            let identity = file_identity(file)?;
            if !self.seen.insert(identity) { return Err(failure("RECOVERY_DELETE_DUPLICATE", 0)); }
            self.visited += 1;
            if directory {
                let remaining = self.options.max_objects.saturating_sub(self.visited);
                let duration = self.budget.remaining()?
                    .min(self.options.max_duration.saturating_sub(self.start.elapsed()));
                let entries = directory_entries_checked(file, remaining, Instant::now(), duration, Some(self.budget))?;
                for entry in entries {
                    self.check(depth + 1)?;
                    let child = open_relative(file, &entry.name, MAXIMUM_ALLOWED, entry.directory,
                        false, ptr::null_mut(), true)?;
                    let actual = file_identity(&child)?;
                    if actual.volume_serial != identity.volume_serial || actual.file_index != entry.identity {
                        return Err(failure("RECOVERY_DELETE_CHANGED", 0));
                    }
                    self.visit(&child, depth + 1)?;
                    drop(child);
                }
            }
            self.budget.check()?;
            if self.start.elapsed() >= self.options.max_duration { return Err(failure("RECOVERY_DELETE_BUDGET", 0)); }
            // Open handles to children cannot be replaced between the identity
            // check and this unlink. New children make directory deletion fail.
            remove_opened_object(file)?;
            self.removed += 1;
            Ok(())
        }
    }
    let mut walk = Walk { options, budget, start: Instant::now(), visited: 0, removed: 0, seen: HashSet::new() };
    if let Err(error) = walk.visit(&root, 0) {
        return Err(io::Error::new(error.kind(), format!(
            "{error}; removed={}; recovery_record_retained=true", walk.removed,
        )));
    }
    drop(root);
    drop(parent);
    Ok(ProfileDeleteReport { removed_objects: walk.removed, root_missing: false })
}

fn job_name(name: &str) -> io::Result<Vec<u16>> {
    if !name.strip_prefix(JOB_PREFIX).is_some_and(|id|
        id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
    { return Err(failure("RECOVERY_JOB_NAME", 0)); }
    Ok(name.encode_utf16().chain(Some(0)).collect())
}
fn job_descriptor(policy: &Policy) -> io::Result<Descriptor> {
    let bytes = size_of::<ACL>() + 2 * offset_of!(ACCESS_ALLOWED_ACE, SidStart)
        + policy.owner.bytes as usize + policy.system.bytes as usize;
    let mut words = vec![0u32; (bytes + 3) / 4];
    let acl = words.as_mut_ptr() as *mut ACL;
    if unsafe { InitializeAcl(acl, (words.len() * 4) as u32, ACL_REVISION) } == 0
        || unsafe { AddAccessAllowedAceEx(acl, ACL_REVISION, 0, JOB_OBJECT_ALL_ACCESS, policy.owner.as_ptr()) } == 0
        || (!policy.owner_is_system() && unsafe {
            AddAccessAllowedAceEx(acl, ACL_REVISION, 0, JOB_OBJECT_ALL_ACCESS, policy.system.as_ptr())
        } == 0)
    { return Err(last_failure("RECOVERY_JOB_ACL")); }
    let mut descriptor = Descriptor { value: unsafe { std::mem::zeroed() }, _acl: words };
    if unsafe { InitializeSecurityDescriptor(descriptor.as_ptr(), 1) } == 0
        || unsafe { SetSecurityDescriptorOwner(descriptor.as_ptr(), policy.owner.as_ptr(), 0) } == 0
        || unsafe { SetSecurityDescriptorDacl(descriptor.as_ptr(), 1, acl, 0) } == 0
        || unsafe { SetSecurityDescriptorControl(descriptor.as_ptr(), SE_DACL_PROTECTED, SE_DACL_PROTECTED) } == 0
    { return Err(last_failure("RECOVERY_JOB_DESCRIPTOR")); }
    Ok(descriptor)
}
fn audit_job(handle: HANDLE, policy: &Policy) -> io::Result<()> {
    let mut owner = ptr::null_mut();
    let mut acl = ptr::null_mut();
    let mut raw = ptr::null_mut();
    let code = unsafe {
        GetSecurityInfo(handle, SE_KERNEL_OBJECT, OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner, ptr::null_mut(), &mut acl, ptr::null_mut(), &mut raw)
    };
    let _allocation = NativeAllocation(raw);
    if code != 0 { return Err(failure("RECOVERY_JOB_SECURITY", code)); }
    if owner.is_null() || unsafe { IsValidSid(owner) } == 0
        || unsafe { EqualSid(owner, policy.owner.as_ptr()) } == 0
        || acl.is_null() || unsafe { IsValidAcl(acl) } == 0
    { return Err(failure("RECOVERY_JOB_OWNER", 0)); }
    let expected = if policy.owner_is_system() { 1 } else { 2 };
    if unsafe { (*acl).AceCount } != expected { return Err(failure("RECOVERY_JOB_ACE_COUNT", 0)); }
    let mut owner_seen = false;
    let mut system_seen = false;
    for index in 0..expected as u32 {
        let mut raw = ptr::null_mut();
        if unsafe { GetAce(acl, index, &mut raw) } == 0 || raw.is_null() { return Err(last_failure("RECOVERY_JOB_ACE")); }
        let header = unsafe { ptr::read_unaligned(raw as *const ACE_HEADER) };
        let offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        if header.AceType != ACCESS_ALLOWED_TYPE || header.AceFlags != 0 || (header.AceSize as usize) < offset + 8 {
            return Err(failure("RECOVERY_JOB_ACE", 0));
        }
        let sid = unsafe { (raw as *mut u8).add(offset) };
        let bytes = 8 + 4 * usize::from(unsafe { *sid.add(1) });
        if bytes > 68 || offset + bytes != header.AceSize as usize || unsafe { IsValidSid(sid.cast()) } == 0 {
            return Err(failure("RECOVERY_JOB_SID", 0));
        }
        let mask = unsafe { ptr::read_unaligned((raw as *const u8).add(4) as *const u32) };
        if mask != JOB_OBJECT_ALL_ACCESS { return Err(failure("RECOVERY_JOB_MASK", 0)); }
        if unsafe { EqualSid(sid.cast(), policy.owner.as_ptr()) } != 0 && !owner_seen { owner_seen = true; }
        else if !policy.owner_is_system() && unsafe { EqualSid(sid.cast(), policy.system.as_ptr()) } != 0 && !system_seen { system_seen = true; }
        else { return Err(failure("RECOVERY_JOB_TRUSTEE", 0)); }
    }
    if !owner_seen || (!policy.owner_is_system() && !system_seen) { return Err(failure("RECOVERY_JOB_TRUSTEE", 0)); }
    Ok(())
}

fn audit_job_limits(handle: HANDLE) -> io::Result<()> {
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { QueryInformationJobObject(handle, JobObjectExtendedLimitInformation,
        (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
        size_of_val(&limits) as u32, ptr::null_mut()) } == 0
    { return Err(last_failure("RECOVERY_JOB_LIMITS_READ")); }
    let flags = limits.BasicLimitInformation.LimitFlags;
    if flags & JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE == 0
        || flags & (JOB_OBJECT_LIMIT_BREAKAWAY_OK | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK) != 0
    { return Err(failure("RECOVERY_JOB_LIMITS_UNTRUSTED", 0)); }
    Ok(())
}

struct SharedJob(NativeHandle);
// SAFETY: the immutable Win32 Job handle has one Arc-owned close operation.
// Query/terminate are Windows-synchronized; no thread-local ownership exists.
unsafe impl Send for SharedJob {}
unsafe impl Sync for SharedJob {}
#[derive(Clone)]
pub struct NamedPrivateJob(Arc<SharedJob>);
impl NamedPrivateJob {
    pub fn create(name: &str) -> io::Result<Self> {
        let name = job_name(name)?;
        let policy = Policy::new()?;
        let mut descriptor = job_descriptor(&policy)?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: descriptor.as_ptr(), bInheritHandle: 0,
        };
        let raw = unsafe { CreateJobObjectW(&attributes, name.as_ptr()) };
        let error = unsafe { GetLastError() };
        if raw.is_null() { return Err(failure("RECOVERY_JOB_CREATE", error)); }
        let handle = NativeHandle(raw);
        if error == ERROR_ALREADY_EXISTS { return Err(failure("RECOVERY_JOB_ALREADY_EXISTS", error)); }
        audit_job(handle.0, &policy)?;
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe { SetInformationJobObject(handle.0, JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), size_of_val(&limits) as u32) } == 0
        { return Err(last_failure("RECOVERY_JOB_LIMITS")); }
        audit_job_limits(handle.0)?;
        Ok(Self(Arc::new(SharedJob(handle))))
    }
    pub fn open(name: &str) -> io::Result<Option<Self>> {
        let name = job_name(name)?;
        let raw = unsafe { OpenJobObjectW(JOB_OBJECT_ALL_ACCESS, 0, name.as_ptr()) };
        if raw.is_null() {
            let error = unsafe { GetLastError() };
            if error == 2 { return Ok(None); }
            return Err(failure("RECOVERY_JOB_OPEN", error));
        }
        let handle = NativeHandle(raw);
        audit_job(handle.0, &Policy::new()?)?;
        audit_job_limits(handle.0)?;
        Ok(Some(Self(Arc::new(SharedJob(handle)))))
    }
    pub fn as_raw_handle(&self) -> RawHandle { self.0.0.0 as _ }
    pub fn active_processes(&self) -> io::Result<u32> {
        let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { QueryInformationJobObject(self.0.0.0, JobObjectBasicAccountingInformation,
            (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(), size_of_val(&info) as u32, ptr::null_mut()) } == 0
        { return Err(last_failure("RECOVERY_JOB_ACCOUNTING")); }
        Ok(info.ActiveProcesses)
    }
    pub fn is_empty(&self) -> io::Result<bool> { self.active_processes().map(|count| count == 0) }
    pub fn terminate(&self) -> io::Result<()> {
        if unsafe { TerminateJobObject(self.0.0.0, 1) } == 0 { return Err(last_failure("RECOVERY_JOB_TERMINATE")); }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use windows_sys::Win32::Security::WinWorldSid;

    fn fixture(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("browseros-recovery-{label}-{}-{nonce}", std::process::id()))
    }
    fn budget() -> OperationBudget { OperationBudget::new(Duration::from_secs(10)) }

    #[test]
    fn native_recovery_record_lease_excludes_second_owner() {
        let parent = fixture("lease");
        ensure_private_directory(&parent, &PrivateTreeOptions::default()).unwrap();
        let budget = budget();
        let store = PrivateRecordStore::open(&parent, &budget).unwrap();
        let mut first = store.create(OsStr::new("0123456789abcdef0123456789abcdef.profile"), &budget).unwrap();
        first.file_mut().write_all(b"durable").unwrap();
        first.file().sync_all().unwrap();
        let entry = store.entries(1, &budget).unwrap().pop().unwrap();
        assert!(store.try_open(&entry, &budget).unwrap().is_none(),
            "a second native owner must not recover an active record");
        drop(first);
        let recovered = store.try_open(&entry, &budget).unwrap().expect("released native lease");
        recovered.mark_deleted().unwrap();
        drop(recovered);
        assert!(store.entries(1, &budget).unwrap().is_empty());
        drop(store);
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn native_recovery_rejects_replaced_profile_identity() {
        let root = fixture("replace");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("original"), b"original").unwrap();
        let budget = budget();
        let pin = PinnedReadDirectory::open(&root, &budget).unwrap();
        let identity = pin.identity_record().unwrap();
        drop(pin);
        let moved = root.with_extension("original");
        std::fs::rename(&root, &moved).unwrap();
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("replacement"), b"must survive").unwrap();
        assert!(delete_recorded_profile_tree(&root, &identity, &PrivateTreeOptions::default(), &budget).is_err());
        assert_eq!(std::fs::read(root.join("replacement")).unwrap(), b"must survive");
        assert_eq!(std::fs::read(moved.join("original")).unwrap(), b"original");
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(moved).unwrap();
    }

    #[test]
    fn native_recovery_postorder_removes_only_recorded_tree() {
        let parent = fixture("delete");
        let root = parent.join("profile");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::create_dir(root.join("state")).unwrap();
        std::fs::write(root.join("state/data"), b"private").unwrap();
        std::fs::write(parent.join("outside"), b"outside").unwrap();
        let budget = budget();
        let pin = PinnedReadDirectory::open(&root, &budget).unwrap();
        let identity = pin.identity_record().unwrap();
        drop(pin);
        let report = delete_recorded_profile_tree(&root, &identity, &PrivateTreeOptions::default(), &budget).unwrap();
        assert_eq!(report.removed_objects, 3);
        assert!(!root.exists());
        assert_eq!(std::fs::read(parent.join("outside")).unwrap(), b"outside");
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn native_recovery_refuses_linked_data_and_keeps_root() {
        let parent = fixture("hardlink");
        let root = parent.join("profile");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(parent.join("outside"), b"outside").unwrap();
        std::fs::hard_link(parent.join("outside"), root.join("alias")).unwrap();
        let budget = budget();
        let pin = PinnedReadDirectory::open(&root, &budget).unwrap();
        let identity = pin.identity_record().unwrap();
        drop(pin);
        let error = delete_recorded_profile_tree(&root, &identity, &PrivateTreeOptions::default(), &budget).unwrap_err();
        assert!(error.to_string().contains("HARDLINK"));
        assert!(root.is_dir());
        assert_eq!(std::fs::read(parent.join("outside")).unwrap(), b"outside");
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn native_named_job_rejects_untrusted_existing_dacl() {
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let name = format!("{JOB_PREFIX}{nonce:032x}");
        let job = NamedPrivateJob::create(&name).unwrap();
        assert!(job.is_empty().unwrap());
        assert!(NamedPrivateJob::create(&name).is_err(), "creation must never adopt an existing job");
        let trusted = NamedPrivateJob::open(&name).unwrap().unwrap();
        drop(trusted);
        let policy = Policy::new().unwrap();
        let mut world = OwnedSid { words: vec![0; 17], bytes: 68 };
        assert_ne!(unsafe { CreateWellKnownSid(WinWorldSid, ptr::null_mut(), world.as_mut_ptr(), &mut world.bytes) }, 0);
        let mut words = vec![0u32; 256];
        let acl = words.as_mut_ptr() as *mut ACL;
        assert_ne!(unsafe { InitializeAcl(acl, (words.len() * 4) as u32, ACL_REVISION) }, 0);
        for sid in [&policy.owner, &policy.system, &world] {
            assert_ne!(unsafe { AddAccessAllowedAceEx(acl, ACL_REVISION, 0, JOB_OBJECT_ALL_ACCESS, sid.as_ptr()) }, 0);
        }
        assert_eq!(unsafe {
            SetSecurityInfo(job.as_raw_handle() as _, SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(), ptr::null_mut(), acl, ptr::null())
        }, 0);
        assert!(NamedPrivateJob::open(&name).is_err(), "correct name alone is not ownership evidence");
        let descriptor = job_descriptor(&policy).unwrap();
        assert_eq!(unsafe {
            SetSecurityInfo(job.as_raw_handle() as _, SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(), ptr::null_mut(), descriptor.value.Dacl, ptr::null())
        }, 0);
        drop(job);
        assert!(NamedPrivateJob::open(&name).unwrap().is_none());
    }
    #[test]
    fn native_recovery_namespace_survives_profile_removal() {
        let base = fixture("namespace");
        let parent = base.join("Packages");
        let name = "BrowserOS.Coding.0123456789abcdef0123456789abcdef";
        let sid = "S-1-15-2-1-2-3-4-5-6-7";
        let root = parent.join(name).join("AC");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("data"), b"only this profile").unwrap();
        let budget = budget();
        let pin = PinnedReadDirectory::open(&root, &budget).unwrap();
        let identity = pin.identity_record().unwrap();
        let scope = pin.recovery_scope(name, sid).unwrap();
        drop(pin);
        let guard = hold_profile_cleanup_namespace(&scope, name, sid, &budget).unwrap();
        delete_recorded_profile_tree(&root, &identity, &PrivateTreeOptions::default(), &budget).unwrap();
        assert!(!root.exists(), "the outside namespace pin must not block profile deletion");
        guard.recheck(sid, &budget).unwrap();
        drop(guard);
        // DataRemoved restart: the original profile root is gone, but the
        // recorded outside namespace is still opened by its native file ID.
        let restored = hold_profile_cleanup_namespace(&scope, name, sid, &budget).unwrap();
        restored.recheck(sid, &budget).unwrap();
        drop(restored);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn native_recovery_namespace_rejects_untrusted_writer() {
        let base = fixture("namespace-writer");
        let parent = base.join("Packages");
        let name = "BrowserOS.Coding.0123456789abcdef0123456789abcdef";
        let sid = "S-1-15-2-1-2-3-4-5-6-7";
        let root = parent.join(name).join("AC");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        let budget = budget();
        let pin = PinnedReadDirectory::open(&root, &budget).unwrap();
        let scope = pin.recovery_scope(name, sid).unwrap();
        drop(pin);
        let opened = open_path(&parent, None, true, MAXIMUM_ALLOWED).unwrap();
        let policy = Policy::new().unwrap();
        let mut world = OwnedSid { words: vec![0; 17], bytes: 68 };
        assert_ne!(unsafe { CreateWellKnownSid(WinWorldSid, ptr::null_mut(), world.as_mut_ptr(), &mut world.bytes) }, 0);
        let mut words = vec![0u32; 256];
        let acl = words.as_mut_ptr() as *mut ACL;
        assert_ne!(unsafe { InitializeAcl(acl, (words.len() * 4) as u32, ACL_REVISION) }, 0);
        for trustee in [&policy.owner, &policy.system, &world] {
            assert_ne!(unsafe { AddAccessAllowedAceEx(acl, ACL_REVISION, 0, FILE_ALL_ACCESS, trustee.as_ptr()) }, 0);
        }
        assert_eq!(unsafe { SetSecurityInfo(opened.target().as_raw_handle() as _, SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(), ptr::null_mut(), acl, ptr::null()) }, 0);
        drop(opened);
        assert!(hold_profile_cleanup_namespace(&scope, name, sid, &budget).is_err());
        ensure_private_directory_root(&parent, &PrivateTreeOptions::default()).unwrap();
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn native_recovery_store_initialization_does_not_traverse_unrelated_children() {
        let base = fixture("shallow-store");
        let parent = base.join("app-data");
        ensure_private_directory(&parent, &PrivateTreeOptions::default()).unwrap();
        std::fs::create_dir(parent.join("unrelated")).unwrap();
        std::fs::write(base.join("outside"), b"unrelated data").unwrap();
        std::fs::hard_link(base.join("outside"), parent.join("unrelated/alias")).unwrap();
        // Deep audit would inspect and reject the linked unrelated child.
        assert!(audit_private_directory(&parent, true, &PrivateTreeOptions::default()).is_err());
        let budget = budget();
        let store = PrivateRecordStore::open(&parent, &budget).unwrap();
        assert!(store.entries(1, &budget).unwrap().is_empty());
        assert_eq!(std::fs::read(base.join("outside")).unwrap(), b"unrelated data");
        drop(store);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn native_recovery_readonly_diagnostic_observes_active_journal() {
        use std::io::Read;
        let parent = fixture("readonly-record");
        ensure_private_directory(&parent, &PrivateTreeOptions::default()).unwrap();
        let budget = budget();
        let store = PrivateRecordStore::open(&parent, &budget).unwrap();
        let mut owner = store.create(OsStr::new("0123456789abcdef0123456789abcdef.profile"), &budget).unwrap();
        owner.file_mut().write_all(b"complete frame").unwrap();
        owner.file().sync_all().unwrap();
        let entry = store.entries(1, &budget).unwrap().pop().unwrap();
        assert!(store.try_open(&entry, &budget).unwrap().is_none());
        let mut reader = store.open_readonly(&entry, &budget).unwrap();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"complete frame");
        assert!(reader.write_all(b"forbidden").is_err(), "diagnostic handles must not carry write access");
        assert_eq!(owner.file().metadata().unwrap().len(), b"complete frame".len() as u64);
        drop(reader);
        owner.mark_deleted().unwrap();
        drop(owner);
        drop(store);
        std::fs::remove_dir_all(parent).unwrap();
    }

}
