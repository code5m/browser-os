//! Independent AppContainer policy. Ordinary owner-only storage must not
//! accept package ACEs; private sandbox trees must not retain unknown ACEs.
use super::*;
use std::io::{Read, Seek, SeekFrom};
use windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW;
use windows_sys::Win32::Security::{AddAce, WinBuiltinAnyPackageSid};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
};

const DELETE_ACCESS: u32 = 0x0001_0000;
const READ_DATA: u32 = 1;
const PIN_ACCESS: u32 = READ_CONTROL | FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY | FILE_TRAVERSE;
const ACL_REVISION_DS_VALUE: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppContainerAccess {
    Modify,
    ReadExecute,
}

pub struct AppContainerPolicy {
    base: Policy,
    package: OwnedSid,
    all_packages: OwnedSid,
    access: AppContainerAccess,
    directory_traversal: bool,
}
impl AppContainerPolicy {
    pub fn new(
        package: &str,
        access: AppContainerAccess,
        directory_traversal: bool,
    ) -> io::Result<Self> {
        if package.len() > 256 || package.encode_utf16().any(|unit| unit == 0) {
            return Err(failure("PACKAGE_SID_INVALID", 0));
        }
        let wide: Vec<u16> = package.encode_utf16().chain(Some(0)).collect();
        let mut raw = ptr::null_mut();
        if unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut raw) } == 0 || raw.is_null() {
            return Err(last_failure("PACKAGE_SID_INVALID"));
        }
        let allocation = NativeAllocation(raw);
        if unsafe { IsValidSid(raw) } == 0 {
            return Err(failure("PACKAGE_SID_INVALID", 0));
        }
        let bytes = unsafe { GetLengthSid(raw) };
        if !(12..=68).contains(&bytes) {
            return Err(failure("PACKAGE_SID_INVALID", 0));
        }
        let data = unsafe { std::slice::from_raw_parts(raw as *const u8, bytes as usize) };
        // AppContainer SIDs use authority 15, subauthority 2 and seven hash
        // words. This excludes ALL APPLICATION PACKAGES/restricted packages.
        if data[1] != 8
            || data[2..8] != [0, 0, 0, 0, 0, 15]
            || u32::from_le_bytes(data[8..12].try_into().unwrap()) != 2
        {
            return Err(failure("PACKAGE_SID_NOT_CONTAINER", 0));
        }
        let mut package = OwnedSid {
            words: vec![0; (bytes as usize + 3) / 4],
            bytes,
        };
        if unsafe { CopySid(bytes, package.as_mut_ptr(), raw) } == 0 {
            return Err(last_failure("PACKAGE_SID_COPY"));
        }
        drop(allocation);
        let mut all_packages = OwnedSid {
            words: vec![0; 17],
            bytes: 68,
        };
        if unsafe {
            CreateWellKnownSid(
                WinBuiltinAnyPackageSid,
                ptr::null_mut(),
                all_packages.as_mut_ptr(),
                &mut all_packages.bytes,
            )
        } == 0
        {
            return Err(last_failure("ALL_PACKAGES_SID"));
        }
        Ok(Self {
            base: Policy::new()?,
            package,
            all_packages,
            access,
            directory_traversal,
        })
    }

    pub(super) fn package_sid(&self) -> &OwnedSid {
        &self.package
    }

    fn mask(&self) -> u32 {
        FILE_GENERIC_READ
            | FILE_GENERIC_EXECUTE
            | match self.access {
                AppContainerAccess::ReadExecute => 0,
                AppContainerAccess::Modify => FILE_GENERIC_WRITE | DELETE_ACCESS,
            }
    }
    fn entries(&self, directory: bool) -> Vec<(&OwnedSid, u32, u32)> {
        let flags = if directory { DIRECTORY_FLAGS } else { 0 };
        let mut entries = vec![(&self.base.owner, FILE_ALL_ACCESS, flags)];
        if !self.base.owner_is_system() {
            entries.push((&self.base.system, FILE_ALL_ACCESS, flags));
        }
        entries.push((&self.package, self.mask(), flags));
        if directory && self.directory_traversal {
            entries.push((&self.all_packages, FILE_TRAVERSE, 0));
        }
        entries
    }
    fn descriptor(&self, directory: bool) -> io::Result<Descriptor> {
        let entries = self.entries(directory);
        let bytes = size_of::<ACL>()
            + entries
                .iter()
                .map(|(sid, _, _)| offset_of!(ACCESS_ALLOWED_ACE, SidStart) + sid.bytes as usize)
                .sum::<usize>();
        let mut storage = vec![0u32; (bytes + 3) / 4];
        let acl = storage.as_mut_ptr() as *mut ACL;
        if unsafe { InitializeAcl(acl, (storage.len() * 4) as u32, ACL_REVISION) } == 0 {
            return Err(last_failure("PACKAGE_ACL_CREATE"));
        }
        for (sid, mask, flags) in entries {
            if unsafe { AddAccessAllowedAceEx(acl, ACL_REVISION, flags, mask, sid.as_ptr()) } == 0 {
                return Err(last_failure("PACKAGE_ACE_CREATE"));
            }
        }
        let mut descriptor = Descriptor {
            value: unsafe { std::mem::zeroed() },
            _acl: storage,
        };
        if unsafe { InitializeSecurityDescriptor(descriptor.as_ptr(), 1) } == 0
            || unsafe {
                SetSecurityDescriptorOwner(descriptor.as_ptr(), self.base.owner.as_ptr(), 0)
            } == 0
            || unsafe { SetSecurityDescriptorDacl(descriptor.as_ptr(), 1, acl, 0) } == 0
            || unsafe {
                SetSecurityDescriptorControl(
                    descriptor.as_ptr(),
                    SE_DACL_PROTECTED,
                    SE_DACL_PROTECTED,
                )
            } == 0
        {
            return Err(last_failure("PACKAGE_DESCRIPTOR_CREATE"));
        }
        Ok(descriptor)
    }
}

fn native_descriptor(file: &File) -> io::Result<(NativeAllocation, PSID, *mut ACL, u16)> {
    let mut owner = ptr::null_mut();
    let mut acl = ptr::null_mut();
    let mut descriptor = ptr::null_mut();
    let code = unsafe {
        GetSecurityInfo(
            file.as_raw_handle() as _,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if code != 0 {
        return Err(failure("PACKAGE_SECURITY_READ", code));
    }
    let allocation = NativeAllocation(descriptor);
    let mut control = 0;
    let mut revision = 0;
    if descriptor.is_null()
        || unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0
    {
        return Err(last_failure("PACKAGE_DESCRIPTOR_READ"));
    }
    Ok((allocation, owner, acl, control))
}

fn exact_app_descriptor(
    file: &File,
    policy: &AppContainerPolicy,
    directory: bool,
    root: bool,
) -> io::Result<bool> {
    let (_allocation, owner, acl, control) = native_descriptor(file)?;
    if owner.is_null()
        || unsafe { IsValidSid(owner) } == 0
        || unsafe { EqualSid(owner, policy.base.owner.as_ptr()) } == 0
        || acl.is_null()
        || control & SE_DACL_PRESENT == 0
        || (root && control & SE_DACL_PROTECTED == 0)
    {
        return Ok(false);
    }
    if unsafe { IsValidAcl(acl) } == 0 {
        return Err(failure("PACKAGE_ACL_MALFORMED", 0));
    }
    let expected = policy.entries(directory);
    if unsafe { (*acl).AceCount as usize } != expected.len() {
        return Ok(false);
    }
    let mut seen = vec![false; expected.len()];
    for index in 0..expected.len() {
        let mut raw = ptr::null_mut();
        if unsafe { GetAce(acl, index as u32, &mut raw) } == 0 || raw.is_null() {
            return Err(last_failure("PACKAGE_ACE_READ"));
        }
        let header = unsafe { ptr::read_unaligned(raw as *const ACE_HEADER) };
        if header.AceType != ACCESS_ALLOWED_TYPE {
            return Ok(false);
        }
        let offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        if (header.AceSize as usize) < offset + 8 {
            return Ok(false);
        }
        let sid = unsafe { (raw as *mut u8).add(offset) };
        let count = unsafe { *sid.add(1) } as usize;
        let bytes = 8 + count * 4;
        if bytes > 68
            || offset + bytes != header.AceSize as usize
            || unsafe { IsValidSid(sid.cast()) } == 0
        {
            return Ok(false);
        }
        let mask = unsafe { ptr::read_unaligned((raw as *const u8).add(4) as *const u32) };
        let flags = u32::from(header.AceFlags) & !INHERITED_ACE;
        let Some(slot) = expected.iter().position(|(want, want_mask, want_flags)|
            unsafe { EqualSid(sid.cast(), want.as_ptr()) } != 0
                && mask == *want_mask && flags == *want_flags
        ) else { return Ok(false); };
        if seen[slot] {
            return Ok(false);
        }
        seen[slot] = true;
    }
    Ok(seen.into_iter().all(|seen| seen))
}

fn set_app_descriptor(
    file: &File,
    policy: &AppContainerPolicy,
    directory: bool,
    budget: &OperationBudget,
) -> io::Result<()> {
    budget.check()?;
    let before = file_identity(file)?;
    if private_object(file)? != directory {
        return Err(failure("PACKAGE_OBJECT_CHANGED", 0));
    }
    let (_allocation, owner, _, _) = native_descriptor(file)?;
    let same_owner = !owner.is_null()
        && unsafe { IsValidSid(owner) } != 0
        && unsafe { EqualSid(owner, policy.base.owner.as_ptr()) } != 0;
    let mut descriptor = policy.descriptor(directory)?;
    let flags = DACL_SECURITY_INFORMATION
        | PROTECTED_DACL_SECURITY_INFORMATION
        | if same_owner {
            0
        } else {
            OWNER_SECURITY_INFORMATION
        };
    // file is the same MAXIMUM_ALLOWED handle used for this object's audit.
    // No implicit propagation, no path command, no old unsafe ACL rollback.
    let code = unsafe {
        SetSecurityInfo(
            file.as_raw_handle() as _,
            SE_FILE_OBJECT,
            flags,
            if same_owner {
                ptr::null_mut()
            } else {
                policy.base.owner.as_ptr()
            },
            ptr::null_mut(),
            descriptor.value.Dacl,
            ptr::null(),
        )
    };
    if code != 0 {
        return Err(failure("PACKAGE_SECURITY_WRITE", code));
    }
    let _ = descriptor.as_ptr();
    budget.check()?;
    if before != file_identity(file)?
        || private_object(file)? != directory
        || !exact_app_descriptor(file, policy, directory, true)?
    {
        return Err(failure("PACKAGE_SECURITY_READBACK", 0));
    }
    Ok(())
}

fn cloned_chain(opened: &OpenedPath) -> io::Result<Vec<File>> {
    opened.chain.iter().map(File::try_clone).collect()
}

/// Own a no-delete namespace chain. Long-lived directory handles request only
/// read/control/traverse, so they coexist with a child's pinned current dir.
pub struct AppContainerDirectory {
    opened: OpenedPath,
    path: PathBuf,
}
impl AppContainerDirectory {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn identity(&self) -> io::Result<FileIdentity> {
        file_identity(self.opened.target())
    }
    pub fn identity_record(&self) -> io::Result<NativeProfileIdentity> {
        super::recovery::profile_identity(self.opened.target())
    }
    pub fn recovery_scope(&self, name: &str, package_sid: &str) -> io::Result<NativeProfileScope> {
        super::recovery::profile_scope(&self.opened, name, package_sid)
    }

    /// OS-managed ancestors retain their existing owner and DACL entries.
    /// Add only noninheritable FILE_TRAVERSE; protect private child trees with
    /// the independent exact policy before a lowbox process can start.
    pub fn open_profile(
        path: &Path,
        policy: &AppContainerPolicy,
        generic_traversal: bool,
        budget: &OperationBudget,
    ) -> io::Result<Self> {
        budget.check()?;
        // Read-only first. Existing profile/cwd pins may reject DELETE access;
        // a compliant profile must never request mutation rights at all.
        let mut opened = open_path_options(path, None, true, PIN_ACCESS, 0, None, Some(budget))?;
        if opened.chain.len() < 2 || !private_object(opened.target())? {
            return Err(failure("PACKAGE_ROOT_NOT_DIRECTORY", 0));
        }
        let root = opened.chain.len() - 1;
        let parent = root - 1;
        let need_own = !has_traversal(opened.target(), &policy.package)?;
        let need_generic =
            generic_traversal && !has_traversal(opened.target(), &policy.all_packages)?;
        let need_parent =
            generic_traversal && !has_traversal(&opened.chain[parent], &policy.all_packages)?;
        if !need_own && !need_generic && !need_parent {
            return Ok(Self {
                opened,
                path: path.to_owned(),
            });
        }
        let identities = opened
            .chain
            .iter()
            .map(file_identity)
            .collect::<io::Result<Vec<_>>>()?;
        let names: Vec<_> = path
            .components()
            .filter_map(|part| match part {
                Component::Normal(name) => Some(name.to_os_string()),
                _ => None,
            })
            .collect();
        if names.len() + 1 != opened.chain.len() || (need_parent && parent == 0) {
            return Err(failure("PACKAGE_PARENT_MISSING", 0));
        }
        // Upgrade only the objects proved noncompliant. The other ancestor
        // handles stay pinned; each close/reopen uses its held parent handle
        // and compares the original native identity before any ACL mutation.
        if need_parent {
            replace_profile_handle(
                &mut opened,
                parent,
                &names[parent - 1],
                identities[parent],
                MAXIMUM_ALLOWED,
                budget,
            )?;
            add_traversal(&opened.chain[parent], &policy.all_packages, budget)?;
        }
        if need_own || need_generic {
            replace_profile_handle(
                &mut opened,
                root,
                &names[root - 1],
                identities[root],
                MAXIMUM_ALLOWED,
                budget,
            )?;
            if need_own {
                add_traversal(opened.target(), &policy.package, budget)?;
            }
            if need_generic {
                add_traversal(opened.target(), &policy.all_packages, budget)?;
            }
            replace_profile_handle(
                &mut opened,
                root,
                &names[root - 1],
                identities[root],
                PIN_ACCESS,
                budget,
            )?;
        }
        if need_parent {
            replace_profile_handle(
                &mut opened,
                parent,
                &names[parent - 1],
                identities[parent],
                PIN_ACCESS,
                budget,
            )?;
        }
        if !has_traversal(opened.target(), &policy.package)?
            || (generic_traversal
                && (!has_traversal(opened.target(), &policy.all_packages)?
                    || !has_traversal(&opened.chain[parent], &policy.all_packages)?))
        {
            return Err(failure("PROFILE_TRAVERSAL_READBACK", 0));
        }
        Ok(Self {
            opened,
            path: path.to_owned(),
        })
    }

    pub fn ensure_child(
        &self,
        name: &OsStr,
        policy: &AppContainerPolicy,
        options: &PrivateTreeOptions,
        budget: &OperationBudget,
    ) -> io::Result<Self> {
        budget.check()?;
        if !name_is_valid(name) {
            return Err(failure("PACKAGE_COMPONENT_INVALID", 0));
        }
        let mut descriptor = policy.descriptor(true)?;
        let child = match open_relative(
            self.opened.target(),
            name,
            MAXIMUM_ALLOWED,
            true,
            false,
            ptr::null_mut(),
            true,
        ) {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let parent = self.opened.reopen_directory(FILE_ADD_SUBDIRECTORY)?;
                open_relative(
                    &parent,
                    name,
                    MAXIMUM_ALLOWED,
                    true,
                    true,
                    descriptor.as_ptr(),
                    true,
                )?
            }
            Err(error) => return Err(error),
        };
        app_tree(&child, policy, options, budget, true, true)?;
        let expected = file_identity(&child)?;
        drop(child);
        let child = open_relative(
            self.opened.target(),
            name,
            PIN_ACCESS,
            true,
            false,
            ptr::null_mut(),
            true,
        )?;
        if expected != file_identity(&child)? {
            return Err(failure("PACKAGE_CHILD_CHANGED", 0));
        }
        let mut chain = cloned_chain(&self.opened)?;
        chain.push(child);
        Ok(Self {
            opened: OpenedPath { chain },
            path: self.path.join(name),
        })
    }

    pub fn audit_root(
        &self,
        policy: &AppContainerPolicy,
        budget: &OperationBudget,
    ) -> io::Result<()> {
        budget.check()?;
        if !private_object(self.opened.target())?
            || !exact_app_descriptor(self.opened.target(), policy, true, true)?
        {
            return Err(failure("PACKAGE_ROOT_POLICY_CHANGED", 0));
        }
        Ok(())
    }

    pub fn protect_tree(
        &self,
        policy: &AppContainerPolicy,
        options: &PrivateTreeOptions,
        budget: &OperationBudget,
    ) -> io::Result<PrivateTreeReport> {
        app_tree(self.opened.target(), policy, options, budget, true, false)
    }

    pub fn audit_tree(
        &self,
        policy: &AppContainerPolicy,
        options: &PrivateTreeOptions,
        budget: &OperationBudget,
    ) -> io::Result<PrivateTreeReport> {
        app_tree(self.opened.target(), policy, options, budget, false, false)
    }

    pub fn create_file(
        &self,
        name: &OsStr,
        policy: &AppContainerPolicy,
        budget: &OperationBudget,
    ) -> io::Result<File> {
        budget.check()?;
        let mut descriptor = policy.descriptor(false)?;
        let parent = self.opened.reopen_directory(2 /* FILE_ADD_FILE */)?;
        let file = open_relative(
            &parent,
            name,
            MAXIMUM_ALLOWED,
            false,
            true,
            descriptor.as_ptr(),
            true,
        )?;
        if private_object(&file)? || !exact_app_descriptor(&file, policy, false, true)? {
            return Err(failure("PACKAGE_NEW_FILE_POLICY", 0));
        }
        Ok(file)
    }

    pub fn pin_file(
        &self,
        name: &OsStr,
        policy: &AppContainerPolicy,
        budget: &OperationBudget,
    ) -> io::Result<PinnedReadFile> {
        budget.check()?;
        let file = open_relative_with_share(
            self.opened.target(),
            name,
            READ_DATA | READ_CONTROL | FILE_READ_ATTRIBUTES,
            false,
            false,
            ptr::null_mut(),
            FILE_SHARE_READ,
        )?;
        if private_object(&file)? || !exact_app_descriptor(&file, policy, false, false)? {
            return Err(failure("PACKAGE_FILE_NOT_PRIVATE", 0));
        }
        let mut chain = cloned_chain(&self.opened)?;
        chain.push(file);
        Ok(PinnedReadFile {
            opened: OpenedPath { chain },
            path: self.path.join(name),
        })
    }
}

fn replace_profile_handle(
    opened: &mut OpenedPath,
    index: usize,
    name: &OsStr,
    expected: FileIdentity,
    access: u32,
    budget: &OperationBudget,
) -> io::Result<()> {
    budget.check()?;
    if index == 0 || index >= opened.chain.len() {
        return Err(failure("PROFILE_HANDLE_INDEX", 0));
    }
    drop(opened.chain.remove(index));
    let file = open_relative(
        &opened.chain[index - 1],
        name,
        access,
        true,
        false,
        ptr::null_mut(),
        true,
    )?;
    if file_identity(&file)? != expected {
        return Err(failure("PROFILE_COMPONENT_CHANGED", 0));
    }
    opened.chain.insert(index, file);
    Ok(())
}

/// A file and all its parents are pinned. No write/delete sharing on the file
/// prevents content/namespace replacement while staging or CreateProcess uses it.
pub struct PinnedReadFile {
    opened: OpenedPath,
    path: PathBuf,
}
impl PinnedReadFile {
    pub fn open(path: &Path, budget: &OperationBudget) -> io::Result<Self> {
        budget.check()?;
        let opened = open_path_options(
            path,
            None,
            true,
            READ_DATA | READ_CONTROL | FILE_READ_ATTRIBUTES,
            0,
            Some(FILE_SHARE_READ),
            Some(budget),
        )?;
        if file_information(opened.target())?.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return Err(failure("STAGING_SOURCE_NOT_FILE", 0));
        }
        // Source hardlinks are not mutated. Private destination hardlinks are
        // rejected by private_object before permissions are ever changed.
        let path = final_path(opened.target())?;
        Ok(Self { opened, path })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn file(&self) -> &File {
        self.opened.target()
    }
    pub fn file_mut(&mut self) -> &mut File {
        self.opened.chain.last_mut().expect("pinned file")
    }
    pub fn identity(&self) -> io::Result<FileIdentity> {
        file_identity(self.file())
    }
    pub fn rewind(&mut self) -> io::Result<()> {
        self.file_mut().seek(SeekFrom::Start(0)).map(|_| ())
    }
}
impl Read for PinnedReadFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file_mut().read(bytes)
    }
}

struct AppWalk<'a> {
    policy: &'a AppContainerPolicy,
    options: &'a PrivateTreeOptions,
    budget: &'a OperationBudget,
    report: PrivateTreeReport,
    seen: HashSet<FileIdentity>,
    repair: bool,
    started: Instant,
}
impl AppWalk<'_> {
    fn check_time(&self) -> io::Result<()> {
        self.budget.check()?;
        if self.started.elapsed() >= self.options.max_duration {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "WINDOWS_PACKAGE_TREE_DEADLINE",
            ));
        }
        Ok(())
    }
    fn check(&self, depth: usize) -> io::Result<()> {
        self.check_time()?;
        if depth > self.options.max_depth || self.report.checked_objects >= self.options.max_objects
        {
            return Err(failure("PACKAGE_TREE_BUDGET", 0));
        }
        Ok(())
    }
    fn visit(&mut self, file: &File, depth: usize, can_repair: bool) -> io::Result<()> {
        self.check(depth)?;
        let directory = private_object(file)?;
        let identity = file_identity(file)?;
        if !self.seen.insert(identity) {
            return Err(failure("PACKAGE_TREE_DUPLICATE", 0));
        }
        self.report.checked_objects += 1;
        if !exact_app_descriptor(file, self.policy, directory, depth == 0)? {
            if !self.repair || !can_repair {
                return Err(failure("PACKAGE_TREE_POLICY_CHANGED", 0));
            }
            if self.report.repaired_objects >= self.options.max_repairs {
                return Err(failure("PACKAGE_TREE_REPAIR_BUDGET", 0));
            }
            set_app_descriptor(file, self.policy, directory, self.budget)?;
            self.report.repaired_objects += 1;
        }
        if directory {
            let remaining = self
                .options
                .max_objects
                .saturating_sub(self.report.checked_objects);
            let local_remaining = self
                .options
                .max_duration
                .saturating_sub(self.started.elapsed());
            let entries = directory_entries_checked(
                file,
                remaining,
                Instant::now(),
                self.budget.remaining()?.min(local_remaining),
                Some(self.budget),
            )?;
            for entry in entries {
                self.check(depth + 1)?;
                let access = READ_CONTROL
                    | FILE_READ_ATTRIBUTES
                    | if entry.directory {
                        FILE_LIST_DIRECTORY | FILE_TRAVERSE
                    } else {
                        0
                    };
                let mut child = open_relative(
                    file,
                    &entry.name,
                    access,
                    entry.directory,
                    false,
                    ptr::null_mut(),
                    true,
                )?;
                let initial = file_identity(&child)?;
                if initial.volume_serial != identity.volume_serial
                    || initial.file_index != entry.identity
                {
                    return Err(failure("PACKAGE_TREE_CHANGED", 0));
                }
                let mut mutable = false;
                if self.repair
                    && !exact_app_descriptor(&child, self.policy, entry.directory, false)?
                {
                    drop(child);
                    child = open_relative(
                        file,
                        &entry.name,
                        MAXIMUM_ALLOWED,
                        entry.directory,
                        false,
                        ptr::null_mut(),
                        true,
                    )?;
                    if initial != file_identity(&child)? {
                        return Err(failure("PACKAGE_TREE_CHANGED", 0));
                    }
                    mutable = true;
                }
                self.visit(&child, depth + 1, mutable)?;
            }
        }
        self.check_time()
    }
}
fn app_tree(
    root: &File,
    policy: &AppContainerPolicy,
    options: &PrivateTreeOptions,
    budget: &OperationBudget,
    repair: bool,
    root_mutable: bool,
) -> io::Result<PrivateTreeReport> {
    if options.max_objects == 0
        || options.max_objects > 200_000
        || options.max_depth == 0
        || options.max_depth > 128
        || options.max_repairs > options.max_objects
        || options.max_duration.is_zero()
    {
        return Err(failure("PACKAGE_TREE_OPTIONS", 0));
    }
    budget.check()?;
    if !private_object(root)? {
        return Err(failure("PACKAGE_TREE_NOT_DIRECTORY", 0));
    }
    let mut walk = AppWalk {
        policy,
        options,
        budget,
        repair,
        seen: HashSet::new(),
        started: Instant::now(),
        report: PrivateTreeReport {
            checked_objects: 0,
            repaired_objects: 0,
            root_identity: file_identity(root)?,
            elapsed_ms: 0,
        },
    };
    if let Err(error) = walk.visit(root, 0, root_mutable) {
        return Err(io::Error::new(
            error.kind(),
            format!(
                "{error}; checked={}; repaired={}; data_preserved=true; unsafe_acl_rollback=false",
                walk.report.checked_objects, walk.report.repaired_objects,
            ),
        ));
    }
    walk.report.elapsed_ms = budget.elapsed().as_millis().min(u64::MAX as u128) as u64;
    Ok(walk.report)
}

// OS-owned profile ancestors are NOT private child trees. Preserve every old
// ACE byte-for-byte and owner/control state, adding one exact, noninheritable FILE_TRAVERSE
// traversal ACE only when absent. Never replace their complete DACL policy.
// Historical RX ACEs have no ownership tag. They may be OS-supplied: keep them
// here rather than guessing provenance. Owned private child trees use the exact
// policy above, so their old broad package ACEs are safely replaced.
fn acl_bytes(acl: *mut ACL) -> io::Result<Vec<Vec<u8>>> {
    if acl.is_null() || unsafe { IsValidAcl(acl) } == 0 {
        return Err(failure("PROFILE_ACL_INVALID", 0));
    }
    let count = unsafe { (*acl).AceCount } as u32;
    if count > 4096 {
        return Err(failure("PROFILE_ACL_LIMIT", 0));
    }
    let mut result = Vec::new();
    for index in 0..count {
        let mut raw = ptr::null_mut();
        if unsafe { GetAce(acl, index, &mut raw) } == 0 || raw.is_null() {
            return Err(last_failure("PROFILE_ACE_READ"));
        }
        let header = unsafe { ptr::read_unaligned(raw as *const ACE_HEADER) };
        if (header.AceSize as usize) < size_of::<ACE_HEADER>() {
            return Err(failure("PROFILE_ACE_SIZE", 0));
        }
        result.push(
            unsafe { std::slice::from_raw_parts(raw as *const u8, header.AceSize as usize) }
                .to_vec(),
        );
    }
    Ok(result)
}
fn traversal_match(bytes: &[u8], sid: &OwnedSid) -> bool {
    let offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
    if bytes.len() != offset + sid.bytes as usize
        || bytes[0] != ACCESS_ALLOWED_TYPE
        || bytes[1] != 0
    {
        return false;
    }
    let raw = &bytes[offset..];
    if raw.len() < 8 {
        return false;
    }
    let sid_bytes = 8 + usize::from(raw[1]) * 4;
    if sid_bytes > 68 || sid_bytes != raw.len() {
        return false;
    }
    // IsValidAcl does not establish the bounds of a SID embedded in an ACE.
    // Validate its complete counted length before any SID API can inspect it.
    let mut aligned = [0u32; 17];
    unsafe {
        ptr::copy_nonoverlapping(raw.as_ptr(), aligned.as_mut_ptr() as *mut u8, sid_bytes);
    }
    let raw_sid = aligned.as_ptr() as PSID;
    if unsafe { IsValidSid(raw_sid) } == 0 {
        return false;
    }
    let mask = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    mask == FILE_TRAVERSE && unsafe { EqualSid(raw_sid, sid.as_ptr()) } != 0
}
fn copied_sid(raw: PSID) -> io::Result<OwnedSid> {
    if raw.is_null() || unsafe { IsValidSid(raw) } == 0 {
        return Err(failure("PROFILE_OWNER_INVALID", 0));
    }
    let bytes = unsafe { GetLengthSid(raw) };
    let mut result = OwnedSid {
        words: vec![0; (bytes as usize + 3) / 4],
        bytes,
    };
    if unsafe { CopySid(bytes, result.as_mut_ptr(), raw) } == 0 {
        return Err(last_failure("PROFILE_OWNER_COPY"));
    }
    Ok(result)
}
fn has_traversal(file: &File, sid: &OwnedSid) -> io::Result<bool> {
    let (_allocation, _, acl, control) = native_descriptor(file)?;
    if control & SE_DACL_PRESENT == 0 {
        return Err(failure("PROFILE_ACL_MISSING", 0));
    }
    let count = acl_bytes(acl)?
        .iter()
        .filter(|entry| traversal_match(entry, sid))
        .count();
    if count > 1 {
        return Err(failure("PROFILE_TRAVERSAL_DUPLICATE", 0));
    }
    Ok(count == 1)
}
fn add_traversal(file: &File, sid: &OwnedSid, budget: &OperationBudget) -> io::Result<()> {
    budget.check()?;
    if !private_object(file)? {
        return Err(failure("PROFILE_TRAVERSAL_NOT_DIRECTORY", 0));
    }
    let identity = file_identity(file)?;
    let (_allocation, owner, old_acl, control) = native_descriptor(file)?;
    if control & SE_DACL_PRESENT == 0 {
        return Err(failure("PROFILE_ACL_MISSING", 0));
    }
    let owner = copied_sid(owner)?;
    let old = acl_bytes(old_acl)?;
    let matches = old
        .iter()
        .filter(|entry| traversal_match(entry, sid))
        .count();
    if matches == 1 {
        return Ok(());
    }
    if matches > 1 {
        return Err(failure("PROFILE_TRAVERSAL_DUPLICATE", 0));
    }
    let bytes = size_of::<ACL>()
        + old.iter().map(Vec::len).sum::<usize>()
        + offset_of!(ACCESS_ALLOWED_ACE, SidStart)
        + sid.bytes as usize;
    if bytes > u16::MAX as usize {
        return Err(failure("PROFILE_ACL_LIMIT", 0));
    }
    let mut words = vec![0u32; (bytes + 3) / 4];
    let acl = words.as_mut_ptr() as *mut ACL;
    if unsafe { InitializeAcl(acl, (words.len() * 4) as u32, ACL_REVISION_DS_VALUE) } == 0 {
        return Err(last_failure("PROFILE_ACL_CREATE"));
    }
    let mut added = false;
    for entry in &old {
        if !added && u32::from(entry[1]) & INHERITED_ACE != 0 {
            if unsafe {
                AddAccessAllowedAceEx(acl, ACL_REVISION_DS_VALUE, 0, FILE_TRAVERSE, sid.as_ptr())
            } == 0
            {
                return Err(last_failure("PROFILE_TRAVERSAL_ADD"));
            }
            added = true;
        }
        if unsafe {
            AddAce(
                acl,
                ACL_REVISION_DS_VALUE,
                u32::MAX,
                entry.as_ptr().cast(),
                entry.len() as u32,
            )
        } == 0
        {
            return Err(last_failure("PROFILE_ACE_PRESERVE"));
        }
    }
    if !added
        && unsafe {
            AddAccessAllowedAceEx(acl, ACL_REVISION_DS_VALUE, 0, FILE_TRAVERSE, sid.as_ptr())
        } == 0
    {
        return Err(last_failure("PROFILE_TRAVERSAL_ADD"));
    }
    let code = unsafe {
        SetSecurityInfo(
            file.as_raw_handle() as _,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            acl,
            ptr::null(),
        )
    };
    if code != 0 {
        return Err(failure("PROFILE_TRAVERSAL_WRITE", code));
    }
    budget.check()?;
    let (_readback, actual_owner, actual_acl, actual_control) = native_descriptor(file)?;
    let actual = acl_bytes(actual_acl)?;
    let preserved: Vec<_> = actual
        .iter()
        .filter(|entry| !traversal_match(entry, sid))
        .cloned()
        .collect();
    if identity != file_identity(file)?
        || actual_owner.is_null()
        || unsafe { IsValidSid(actual_owner) } == 0
        || unsafe { EqualSid(actual_owner, owner.as_ptr()) } == 0
        || (control & SE_DACL_PROTECTED) != (actual_control & SE_DACL_PROTECTED)
        || actual
            .iter()
            .filter(|entry| traversal_match(entry, sid))
            .count()
            != 1
        || preserved != old
    {
        return Err(failure("PROFILE_TRAVERSAL_READBACK", 0));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct PinnedDirectoryEntry {
    pub name: OsString,
    pub directory: bool,
    pub identity: FileIdentity,
}
pub struct PinnedReadDirectory {
    opened: OpenedPath,
    path: PathBuf,
}
impl PinnedReadDirectory {
    pub fn identity_record(&self) -> io::Result<NativeProfileIdentity> {
        super::recovery::profile_identity(self.opened.target())
    }
    pub fn recovery_scope(&self, name: &str, package_sid: &str) -> io::Result<NativeProfileScope> {
        super::recovery::profile_scope(&self.opened, name, package_sid)
    }
    pub fn open(path: &Path, budget: &OperationBudget) -> io::Result<Self> {
        budget.check()?;
        let opened = open_path_options(path, None, true, PIN_ACCESS, 0, None, Some(budget))?;
        if !private_object(opened.target())? {
            return Err(failure("STAGING_SOURCE_NOT_DIRECTORY", 0));
        }
        let path = final_path(opened.target())?;
        Ok(Self { opened, path })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn entries(
        &self,
        remaining: usize,
        budget: &OperationBudget,
    ) -> io::Result<Vec<PinnedDirectoryEntry>> {
        budget.check()?;
        let volume = file_identity(self.opened.target())?.volume_serial;
        directory_entries_checked(
            self.opened.target(),
            remaining,
            Instant::now(),
            budget.remaining()?,
            Some(budget),
        )
        .map(|entries| {
            entries
                .into_iter()
                .map(|entry| PinnedDirectoryEntry {
                    name: entry.name,
                    directory: entry.directory,
                    identity: FileIdentity {
                        volume_serial: volume,
                        file_index: entry.identity,
                    },
                })
                .collect()
        })
    }
    pub fn child_directory(
        &self,
        entry: &PinnedDirectoryEntry,
        budget: &OperationBudget,
    ) -> io::Result<Self> {
        budget.check()?;
        if !entry.directory {
            return Err(failure("STAGING_ENTRY_KIND", 0));
        }
        let file = open_relative(
            self.opened.target(),
            &entry.name,
            PIN_ACCESS,
            true,
            false,
            ptr::null_mut(),
            true,
        )?;
        if file_identity(&file)? != entry.identity {
            return Err(failure("STAGING_SOURCE_CHANGED", 0));
        }
        let mut chain = cloned_chain(&self.opened)?;
        chain.push(file);
        Ok(Self {
            opened: OpenedPath { chain },
            path: self.path.join(&entry.name),
        })
    }
    pub fn child_file(
        &self,
        entry: &PinnedDirectoryEntry,
        budget: &OperationBudget,
    ) -> io::Result<PinnedReadFile> {
        budget.check()?;
        if entry.directory {
            return Err(failure("STAGING_ENTRY_KIND", 0));
        }
        let file = open_relative_with_share(
            self.opened.target(),
            &entry.name,
            READ_DATA | READ_CONTROL | FILE_READ_ATTRIBUTES,
            false,
            false,
            ptr::null_mut(),
            FILE_SHARE_READ,
        )?;
        if file_identity(&file)? != entry.identity {
            return Err(failure("STAGING_SOURCE_CHANGED", 0));
        }
        let mut chain = cloned_chain(&self.opened)?;
        chain.push(file);
        Ok(PinnedReadFile {
            opened: OpenedPath { chain },
            path: self.path.join(&entry.name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Security::{AddAccessDeniedAceEx, WinWorldSid};

    const PACKAGE: &str = "S-1-15-2-1-2-3-4-5-6-7";
    const OTHER_PACKAGE: &str = "S-1-15-2-7-6-5-4-3-2-1";

    fn fixture(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "browseros-package-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
    fn world() -> OwnedSid {
        let mut sid = OwnedSid {
            words: vec![0; 17],
            bytes: 68,
        };
        assert_ne!(
            unsafe {
                CreateWellKnownSid(
                    WinWorldSid,
                    ptr::null_mut(),
                    sid.as_mut_ptr(),
                    &mut sid.bytes,
                )
            },
            0
        );
        sid
    }
    fn set_acl(file: &File, entries: &[(&OwnedSid, bool, u32, u32)]) {
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
        for (sid, deny, mask, flags) in entries {
            assert_ne!(
                unsafe {
                    if *deny {
                        AddAccessDeniedAceEx(acl, ACL_REVISION, *flags, *mask, sid.as_ptr())
                    } else {
                        AddAccessAllowedAceEx(acl, ACL_REVISION, *flags, *mask, sid.as_ptr())
                    }
                },
                0
            );
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
    }
    fn policy(access: AppContainerAccess, traversal: bool) -> AppContainerPolicy {
        AppContainerPolicy::new(PACKAGE, access, traversal).unwrap()
    }

    #[test]
    fn package_native_repair_removes_world_and_other_container_from_every_object() {
        let root = fixture("remove-explicit");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("data"), b"preserved").unwrap();
        let policy = policy(AppContainerAccess::Modify, false);
        let other =
            AppContainerPolicy::new(OTHER_PACKAGE, AppContainerAccess::Modify, false).unwrap();
        let world = world();
        let root_handle = open_path(&root, None, true, MAXIMUM_ALLOWED).unwrap();
        let child = open_relative(
            root_handle.target(),
            OsStr::new("data"),
            MAXIMUM_ALLOWED,
            false,
            false,
            ptr::null_mut(),
            true,
        )
        .unwrap();
        for (file, directory) in [(root_handle.target(), true), (&child, false)] {
            let mut entries: Vec<_> = policy
                .entries(directory)
                .into_iter()
                .map(|(sid, mask, flags)| (sid, false, mask, flags))
                .collect();
            entries.push((&world, false, FILE_ALL_ACCESS, 0));
            entries.push((&other.package, false, FILE_ALL_ACCESS, 0));
            set_acl(file, &entries);
            assert!(!exact_app_descriptor(file, &policy, directory, true).unwrap());
        }
        drop(child);
        let budget = OperationBudget::new(Duration::from_secs(10));
        let report = app_tree(
            root_handle.target(),
            &policy,
            &PrivateTreeOptions::default(),
            &budget,
            true,
            true,
        )
        .unwrap();
        assert_eq!(report.checked_objects, 2);
        assert_eq!(report.repaired_objects, 2);
        assert_eq!(std::fs::read(root.join("data")).unwrap(), b"preserved");
        let second = app_tree(
            root_handle.target(),
            &policy,
            &PrivateTreeOptions::default(),
            &budget,
            true,
            true,
        )
        .unwrap();
        assert_eq!(second.repaired_objects, 0);
        assert!(app_tree(
            root_handle.target(),
            &other,
            &PrivateTreeOptions::default(),
            &budget,
            false,
            false
        )
        .is_err());
        drop(root_handle);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_exact_policy_rejects_same_sid_deny_wrong_mask_and_inherited_traversal() {
        let root = fixture("exact");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("data"), b"source").unwrap();
        let opened = open_path(&root.join("data"), None, true, MAXIMUM_ALLOWED).unwrap();
        let policy = policy(AppContainerAccess::ReadExecute, true);
        let budget = OperationBudget::new(Duration::from_secs(10));
        let expected: Vec<_> = policy
            .entries(false)
            .into_iter()
            .map(|(sid, mask, flags)| (sid, false, mask, flags))
            .collect();
        for variant in 0..3 {
            let mut entries = expected.clone();
            let package = entries.iter_mut().find(|(sid, _, _, _)|
                unsafe { EqualSid(sid.as_ptr(), policy.package.as_ptr()) } != 0).unwrap();
            match variant {
                0 => package.1 = true,
                1 => package.2 |= FILE_GENERIC_WRITE,
                _ => package.3 = OBJECT_INHERIT_ACE,
            }
            set_acl(opened.target(), &entries);
            assert!(!exact_app_descriptor(opened.target(), &policy, false, false).unwrap());
            set_app_descriptor(opened.target(), &policy, false, &budget).unwrap();
            assert!(exact_app_descriptor(opened.target(), &policy, false, true).unwrap());
        }
        // Generic package traversal must never be present on a source file.
        let mut generic_file = expected;
        generic_file.push((
            &policy.all_packages,
            false,
            FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
            0,
        ));
        set_acl(opened.target(), &generic_file);
        assert!(!exact_app_descriptor(opened.target(), &policy, false, false).unwrap());
        set_app_descriptor(opened.target(), &policy, false, &budget).unwrap();
        drop(opened);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_tree_repairs_are_bounded_and_never_follow_a_hardlink() {
        let root = fixture("bounded");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        std::fs::write(root.join("one"), b"source").unwrap();
        std::fs::hard_link(root.join("one"), root.join("alias")).unwrap();
        let opened = open_path(&root, None, true, MAXIMUM_ALLOWED).unwrap();
        let policy = policy(AppContainerAccess::ReadExecute, true);
        let budget = OperationBudget::new(Duration::from_secs(10));
        let zero = PrivateTreeOptions {
            max_repairs: 0,
            ..PrivateTreeOptions::default()
        };
        let error = app_tree(opened.target(), &policy, &zero, &budget, true, true).unwrap_err();
        assert!(error.to_string().contains("REPAIR_BUDGET"));
        let error = app_tree(
            opened.target(),
            &policy,
            &PrivateTreeOptions::default(),
            &budget,
            true,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("HARDLINK"));
        assert_eq!(std::fs::read(root.join("one")).unwrap(), b"source");
        drop(opened);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pinned_directory_counts_all_entries_and_denies_source_replacement() {
        let root = fixture("pin");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("data"), b"content").unwrap();
        let budget = OperationBudget::new(Duration::from_secs(10));
        let directory = PinnedReadDirectory::open(&root, &budget).unwrap();
        let error = match directory.entries(1, &budget) {
            Err(error) => error,
            Ok(_) => panic!("entry budget ignored"),
        };
        assert!(error.to_string().contains("TREE_OBJECT_LIMIT"));
        let entry = directory
            .entries(2, &budget)
            .unwrap()
            .into_iter()
            .find(|entry| !entry.directory)
            .unwrap();
        let mut file = directory.child_file(&entry, &budget).unwrap();
        assert!(OpenOptions::new()
            .write(true)
            .open(root.join("data"))
            .is_err());
        assert!(std::fs::rename(&root, root.with_extension("moved")).is_err());
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"content");
        drop(file);
        drop(directory);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_sid_must_be_an_exact_appcontainer_sid() {
        for sid in [
            "S-1-15-2-1",
            "S-1-15-2-2",
            "S-1-1-0",
            "S-1-15-2-1-2-3-4-5-6",
        ] {
            assert!(AppContainerPolicy::new(sid, AppContainerAccess::Modify, false).is_err());
        }
        assert!(AppContainerPolicy::new(PACKAGE, AppContainerAccess::ReadExecute, true).is_ok());
    }

    #[test]
    fn operation_budget_cancellation_keeps_resource_ownership_until_cleanup() {
        let budget = OperationBudget::new(Duration::from_secs(10));
        let copy = budget.clone();
        let owner = budget.hold_cleanup();
        copy.cancel();
        assert_eq!(
            budget.check().unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert!(!budget.cleanup_complete());
        drop(owner);
        assert!(budget.cleanup_complete());
        assert_eq!(
            OperationBudget::new(Duration::ZERO)
                .check()
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
    }
    #[test]
    fn traversal_parser_rejects_counted_sid_beyond_ace_bounds() {
        let policy = policy(AppContainerAccess::ReadExecute, true);
        let mut descriptor = policy.descriptor(true).unwrap();
        let mut raw = ptr::null_mut();
        let acl = descriptor.value.Dacl;
        let index = unsafe { (*acl).AceCount as u32 - 1 };
        assert_ne!(unsafe { GetAce(acl, index, &mut raw) }, 0);
        let header = unsafe { ptr::read_unaligned(raw as *const ACE_HEADER) };
        let mut bytes =
            unsafe { std::slice::from_raw_parts(raw as *const u8, header.AceSize as usize) }
                .to_vec();
        assert!(traversal_match(&bytes, &policy.all_packages));
        bytes[offset_of!(ACCESS_ALLOWED_ACE, SidStart) + 1] = 255;
        assert!(!traversal_match(&bytes, &policy.all_packages));
        let _ = descriptor.as_ptr();
    }

    #[test]
    fn generic_package_traversal_never_grants_directory_listing() {
        let root = fixture("traverse-only");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        let opened = open_path(&root, None, true, MAXIMUM_ALLOWED).unwrap();
        let policy = policy(AppContainerAccess::ReadExecute, true);
        let mut old: Vec<_> = policy
            .entries(true)
            .into_iter()
            .map(|(sid, mask, flags)| (sid, false, mask, flags))
            .collect();
        old.iter_mut()
            .find(|(sid, _, _, _)| unsafe {
                EqualSid(sid.as_ptr(), policy.all_packages.as_ptr()) != 0
            })
            .unwrap()
            .2 = FILE_GENERIC_READ | FILE_GENERIC_EXECUTE;
        set_acl(opened.target(), &old);
        assert!(!exact_app_descriptor(opened.target(), &policy, true, true).unwrap());
        let budget = OperationBudget::new(Duration::from_secs(10));
        let report = app_tree(
            opened.target(),
            &policy,
            &PrivateTreeOptions::default(),
            &budget,
            true,
            true,
        )
        .unwrap();
        assert_eq!(report.repaired_objects, 1);
        let (_allocation, _, acl, _) = native_descriptor(opened.target()).unwrap();
        let entries = acl_bytes(acl).unwrap();
        let generic = entries
            .iter()
            .find(|bytes| traversal_match(bytes, &policy.all_packages))
            .expect("native exact traversal ACE");
        assert_eq!(
            u32::from_le_bytes(generic[4..8].try_into().unwrap()),
            FILE_TRAVERSE
        );
        assert_eq!(generic[1], 0, "traversal must never inherit onto files");
        assert_eq!(FILE_TRAVERSE & FILE_LIST_DIRECTORY, 0);
        assert!(policy.entries(false).iter().all(|(sid, _, _)| unsafe {
            EqualSid(sid.as_ptr(), policy.all_packages.as_ptr()) == 0
        }));
        drop(opened);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compliant_profile_reopen_uses_read_only_handles() {
        let root = fixture("profile-reopen");
        ensure_private_directory(&root, &PrivateTreeOptions::default()).unwrap();
        let policy = policy(AppContainerAccess::ReadExecute, false);
        let budget = OperationBudget::new(Duration::from_secs(10));
        let first = AppContainerDirectory::open_profile(&root, &policy, false, &budget).unwrap();
        // The first no-DELETE pin makes an unconditional MAXIMUM_ALLOWED open
        // conflict. The second compliant preparation must stay read-only.
        let second = AppContainerDirectory::open_profile(&root, &policy, false, &budget).unwrap();
        assert_eq!(first.identity().unwrap(), second.identity().unwrap());
        assert!(has_traversal(second.opened.target(), &policy.package).unwrap());
        drop(second);
        drop(first);
        std::fs::remove_dir_all(root).unwrap();
    }
}
