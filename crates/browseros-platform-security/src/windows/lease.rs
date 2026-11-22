//! A preparation epoch shared by cooperating BrowserOS processes.
//!
//! Full-tree migration runs before the first consumer in an epoch. Other
//! processes join that epoch after a native owner/SYSTEM-only readiness event
//! is signalled, and keep it alive for their entire storage lifetime. There is
//! no persistent "trusted" marker, environment flag, or periodic live-cache
//! traversal. When the last participant exits the kernel deletes the evidence,
//! so the next cold start must audit the complete tree again.
//!
//! Old binaries / orphaned WebView2 processes do not participate. If their
//! live writes prevent a complete cold audit, preparation fails closed; close
//! those storage users and retry. Native I/O deadlines are checked between
//! system calls, not advertised as a hard interruption of a filesystem driver.

use super::*;
use windows_sys::Win32::Foundation::{WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::Security::Authorization::SE_KERNEL_OBJECT;
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, ReleaseMutex, SetEvent, WaitForSingleObject,
};

// Change the epoch version whenever the full-tree policy or preparation
// contract changes. Global events/mutexes coordinate the same private data
// directory across Windows logon sessions (no shared-memory privilege needed).
const EPOCH_VERSION: u32 = 2;
const EVENT_ACCESS: u32 = 0x001f_0003;
const MUTEX_ACCESS: u32 = 0x001f_0001;

struct SharedHandle(NativeHandle);
// SAFETY: ownership is unique and CloseHandle runs only after all references
// to PreparedPrivateDirectory are gone. Windows event handles may be queried
// concurrently; no mutex ownership is stored in this wrapper.
unsafe impl Send for SharedHandle {}
unsafe impl Sync for SharedHandle {}

struct MutexGuard<'a>(&'a NativeHandle);
impl Drop for MutexGuard<'_> {
    fn drop(&mut self) {
        unsafe { ReleaseMutex(self.0.0); }
    }
}

fn kernel_descriptor(policy: &Policy, access: u32) -> io::Result<Descriptor> {
    let bytes = size_of::<ACL>() + 2 * offset_of!(ACCESS_ALLOWED_ACE, SidStart)
        + policy.owner.bytes as usize + policy.system.bytes as usize;
    let mut storage = vec![0u32; (bytes + 3) / 4];
    let acl = storage.as_mut_ptr() as *mut ACL;
    if unsafe { InitializeAcl(acl, (storage.len() * 4) as u32, ACL_REVISION) } == 0
        || unsafe { AddAccessAllowedAceEx(acl, ACL_REVISION, 0, access, policy.owner.as_ptr()) } == 0
    {
        return Err(last_failure("EPOCH_ACL"));
    }
    if !policy.owner_is_system()
        && unsafe { AddAccessAllowedAceEx(acl, ACL_REVISION, 0, access, policy.system.as_ptr()) } == 0
    {
        return Err(last_failure("EPOCH_SYSTEM_ACE"));
    }
    let mut descriptor = Descriptor { value: unsafe { std::mem::zeroed() }, _acl: storage };
    if unsafe { InitializeSecurityDescriptor(descriptor.as_ptr(), 1) } == 0
        || unsafe { SetSecurityDescriptorOwner(descriptor.as_ptr(), policy.owner.as_ptr(), 0) } == 0
        || unsafe { SetSecurityDescriptorDacl(descriptor.as_ptr(), 1, acl, 0) } == 0
        || unsafe { SetSecurityDescriptorControl(descriptor.as_ptr(), SE_DACL_PROTECTED, SE_DACL_PROTECTED) } == 0
    {
        return Err(last_failure("EPOCH_DESCRIPTOR"));
    }
    Ok(descriptor)
}

/// The create APIs ignore the requested descriptor when an object already
/// exists. Always inspect the actual object before trusting its state.
fn audit_kernel_object(handle: HANDLE, policy: &Policy, access: u32) -> io::Result<()> {
    let mut owner = ptr::null_mut();
    let mut acl = ptr::null_mut();
    let mut raw = ptr::null_mut();
    let result = unsafe {
        GetSecurityInfo(handle, SE_KERNEL_OBJECT, OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner, ptr::null_mut(), &mut acl, ptr::null_mut(), &mut raw)
    };
    let _allocation = NativeAllocation(raw);
    if result != 0 { return Err(failure("EPOCH_READ_DESCRIPTOR", result)); }
    if owner.is_null() || unsafe { IsValidSid(owner) } == 0
        || unsafe { EqualSid(owner, policy.owner.as_ptr()) } == 0
        || acl.is_null() || unsafe { IsValidAcl(acl) } == 0
    {
        return Err(failure("EPOCH_UNTRUSTED_OWNER_OR_DACL", 0));
    }
    let expected = if policy.owner_is_system() { 1 } else { 2 };
    if unsafe { (*acl).AceCount } != expected {
        return Err(failure("EPOCH_UNEXPECTED_ACE_COUNT", 0));
    }
    let mut owner_seen = false;
    let mut system_seen = false;
    for index in 0..expected as u32 {
        let mut raw_ace = ptr::null_mut();
        if unsafe { GetAce(acl, index, &mut raw_ace) } == 0 {
            return Err(last_failure("EPOCH_READ_ACE"));
        }
        if raw_ace.is_null() { return Err(failure("EPOCH_NULL_ACE", 0)); }
        let header = unsafe { &*(raw_ace as *const ACE_HEADER) };
        if header.AceType != ACCESS_ALLOWED_TYPE || header.AceFlags != 0
            || (header.AceSize as usize) < offset_of!(ACCESS_ALLOWED_ACE, SidStart) + 8
        {
            return Err(failure("EPOCH_UNEXPECTED_ACE", 0));
        }
        let ace = unsafe { &*(raw_ace as *const ACCESS_ALLOWED_ACE) };
        let sid = (&ace.SidStart as *const u32) as PSID;
        let sid_bytes = 8 + 4 * unsafe { *((sid as *const u8).add(1)) } as usize;
        if sid_bytes > 68 || offset_of!(ACCESS_ALLOWED_ACE, SidStart) + sid_bytes != header.AceSize as usize
            || ace.Mask != access || unsafe { IsValidSid(sid) } == 0
            || unsafe { GetLengthSid(sid) } as usize != sid_bytes
        {
            return Err(failure("EPOCH_UNEXPECTED_ACE_ACCESS", 0));
        }
        if unsafe { EqualSid(sid, policy.owner.as_ptr()) } != 0 && !owner_seen {
            owner_seen = true;
        } else if !policy.owner_is_system()
            && unsafe { EqualSid(sid, policy.system.as_ptr()) } != 0 && !system_seen
        {
            system_seen = true;
        } else {
            return Err(failure("EPOCH_UNEXPECTED_TRUSTEE", 0));
        }
    }
    if !owner_seen || (!policy.owner_is_system() && !system_seen) {
        return Err(failure("EPOCH_MISSING_TRUSTEE", 0));
    }
    Ok(())
}

fn volume_namespace(file: &File) -> io::Result<String> {
    // Volume serial numbers can collide (e.g. cloned volumes). Bind the
    // kernel evidence to the OS volume GUID as well as the per-volume file ID.
    let mut buffer = vec![0u16; 32768];
    let used = unsafe {
        GetFinalPathNameByHandleW(file.as_raw_handle() as _, buffer.as_mut_ptr(),
            buffer.len() as u32, 1 /* VOLUME_NAME_GUID */)
    } as usize;
    if used == 0 || used >= buffer.len() { return Err(last_failure("EPOCH_VOLUME_NAME")); }
    let prefix: Vec<u16> = r"\\?\Volume{".encode_utf16().collect();
    if !buffer[..used].starts_with(&prefix) {
        return Err(failure("EPOCH_VOLUME_NAMESPACE", 0));
    }
    let end = prefix.len() + 36;
    if used <= end || buffer[end] != b'}' as u16 {
        return Err(failure("EPOCH_VOLUME_GUID", 0));
    }
    let guid = &buffer[prefix.len()..end];
    for (index, &unit) in guid.iter().enumerate() {
        let dash = matches!(index, 8 | 13 | 18 | 23);
        if (dash && unit != b'-' as u16)
            || (!dash && (unit > 127 || !(unit as u8).is_ascii_hexdigit()))
        {
            return Err(failure("EPOCH_VOLUME_GUID", 0));
        }
    }
    Ok(String::from_utf16(guid).map_err(|_| failure("EPOCH_VOLUME_GUID", 0))?
        .to_ascii_lowercase())
}

fn name(volume: &str, identity: FileIdentity, suffix: &str) -> Vec<u16> {
    format!(r"Global\BrowserOS.Private.v{EPOCH_VERSION}.{volume}.{:08x}.{:016x}.{suffix}",
        identity.volume_serial, identity.file_index)
        .encode_utf16().chain(std::iter::once(0)).collect()
}

fn remaining(options: &PrivateTreeOptions, started: Instant) -> io::Result<PrivateTreeOptions> {
    let max_duration = options.max_duration.checked_sub(started.elapsed())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| failure("EPOCH_BUDGET_EXCEEDED", 0))?;
    Ok(PrivateTreeOptions { max_duration, ..*options })
}

/// Must be retained until every storage consumer in this process has stopped.
/// Acquiring it is a startup operation, never an operation in a tab write lock.
pub struct PreparedPrivateDirectory {
    path: PathBuf,
    pin: OpenedPath,
    _ready: SharedHandle,
    identity: FileIdentity,
    report: PrivateTreeReport,
    reused: bool,
}

impl PreparedPrivateDirectory {
    pub fn prepare(path: &Path, options: &PrivateTreeOptions) -> io::Result<Self> {
        let started = Instant::now();
        let root_report = ensure_private_directory_root(path, &remaining(options, started)?)?;
        // Identity discovery shares DELETE so it cannot conflict with the
        // first participant's MAXIMUM_ALLOWED migration handles. The lasting
        // no-delete pin is acquired only after the cold migration completes.
        let observed = open_path(path, None, false, READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE)?;
        if !private_object(observed.target())?
            || file_identity(observed.target())? != root_report.root_identity
        {
            return Err(failure("EPOCH_ROOT_CHANGED", 0));
        }
        let identity = root_report.root_identity;
        let volume = volume_namespace(observed.target())?;
        let policy = Policy::new()?;
        let mut mutex_sd = kernel_descriptor(&policy, MUTEX_ACCESS)?;
        let mutex_attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: mutex_sd.as_ptr(),
            bInheritHandle: 0,
        };
        let mutex = NativeHandle(unsafe {
            CreateMutexW(&mutex_attributes, 0, name(&volume, identity, "prepare").as_ptr())
        });
        if mutex.0.is_null() { return Err(last_failure("EPOCH_MUTEX_CREATE")); }
        audit_kernel_object(mutex.0, &policy, MUTEX_ACCESS)?;
        let wait_ms = remaining(options, started)?.max_duration.as_millis()
            .min((u32::MAX - 1) as u128) as u32;
        match unsafe { WaitForSingleObject(mutex.0, wait_ms) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => {}
            WAIT_TIMEOUT => return Err(failure("EPOCH_PREPARATION_BUSY", 0)),
            _ => return Err(last_failure("EPOCH_MUTEX_WAIT")),
        }
        let guard = MutexGuard(&mutex);
        let mut event_sd = kernel_descriptor(&policy, EVENT_ACCESS)?;
        let event_attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: event_sd.as_ptr(),
            bInheritHandle: 0,
        };
        let ready = NativeHandle(unsafe {
            CreateEventW(&event_attributes, 1, 0, name(&volume, identity, "ready").as_ptr())
        });
        if ready.0.is_null() { return Err(last_failure("EPOCH_EVENT_CREATE")); }
        audit_kernel_object(ready.0, &policy, EVENT_ACCESS)?;
        let reused = match unsafe { WaitForSingleObject(ready.0, 0) } {
            WAIT_OBJECT_0 => true,
            WAIT_TIMEOUT => false,
            _ => return Err(last_failure("EPOCH_EVENT_WAIT")),
        };
        let mut report = if reused {
            audit_private_directory(path, false, &remaining(options, started)?)?
        } else {
            ensure_private_directory(path, &remaining(options, started)?)?
        };
        let pin = open_path(path, None, true, READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE)?;
        if report.root_identity != identity || file_identity(pin.target())? != identity
            || !descriptor_is_private(pin.target(), &policy, true, true)?
            || !path_matches_handle(path, pin.target())?
        {
            return Err(failure("EPOCH_ROOT_RECHECK_FAILED", 0));
        }
        remaining(options, started)?;
        if !reused && unsafe { SetEvent(ready.0) } == 0 {
            return Err(last_failure("EPOCH_PUBLISH_FAILED"));
        }
        report.elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        drop(guard);
        Ok(Self {
            path: path.to_path_buf(), pin, _ready: SharedHandle(ready),
            identity, report, reused,
        })
    }

    pub fn path(&self) -> &Path { &self.path }
    pub fn report(&self) -> PrivateTreeReport { self.report }
    pub fn reused_preparation(&self) -> bool { self.reused }

    /// Constant-work, read-only check of the held root, with no cache traversal.
    pub fn check_root(&self) -> io::Result<()> {
        let policy = Policy::new()?;
        if !private_object(self.pin.target())?
            || file_identity(self.pin.target())? != self.identity
            || !descriptor_is_private(self.pin.target(), &policy, true, true)?
            || !path_matches_handle(&self.path, self.pin.target())?
        {
            return Err(failure("EPOCH_ROOT_RECHECK_FAILED", 0));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!("browseros-private-epoch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn consumers_share_preparation_and_cold_restart_reaudits() {
        let path = fixture();
        std::fs::write(path.join("initial"), b"retained").unwrap();
        let first = PreparedPrivateDirectory::prepare(&path, &PrivateTreeOptions::default()).unwrap();
        assert!(!first.reused_preparation());
        let tiny = PrivateTreeOptions { max_objects: 1, max_repairs: 1, ..PrivateTreeOptions::default() };
        let second = PreparedPrivateDirectory::prepare(&path, &tiny).unwrap();
        assert!(second.reused_preparation());
        assert_eq!(second.report().checked_objects, 1);
        drop(first);
        second.check_root().unwrap();
        drop(second);
        // No consumer remains, so native readiness evidence no longer exists.
        assert!(PreparedPrivateDirectory::prepare(&path, &tiny).is_err());
        let restarted = PreparedPrivateDirectory::prepare(&path, &PrivateTreeOptions::default()).unwrap();
        assert!(!restarted.reused_preparation());
        drop(restarted);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn active_cache_churn_does_not_enter_a_deep_audit() {
        let path = fixture();
        let prepared = PreparedPrivateDirectory::prepare(&path, &PrivateTreeOptions::default()).unwrap();
        let child_path = path.clone();
        let writer = std::thread::spawn(move || {
            for index in 0..256 {
                let file = child_path.join(format!("cache-{index}"));
                std::fs::write(&file, b"cache").unwrap();
                std::fs::remove_file(file).unwrap();
            }
        });
        for _ in 0..16 {
            let joined = PreparedPrivateDirectory::prepare(&path, &PrivateTreeOptions {
                max_objects: 1, max_repairs: 1, ..PrivateTreeOptions::default()
            }).unwrap();
            assert!(joined.reused_preparation());
            joined.check_root().unwrap();
        }
        writer.join().unwrap();
        drop(prepared);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn preexisting_kernel_object_with_unexpected_access_is_not_trusted() {
        let policy = Policy::new().unwrap();
        let mut descriptor = policy.descriptor(false).unwrap();
        // A file-access mask, even with the right principals, is not the
        // exact event policy. CreateEvent alone cannot validate this.
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.as_ptr(), bInheritHandle: 0,
        };
        let event = NativeHandle(unsafe { CreateEventW(&attributes, 1, 1, ptr::null()) });
        assert!(!event.0.is_null());
        assert!(audit_kernel_object(event.0, &policy, EVENT_ACCESS).is_err());
    }
}
