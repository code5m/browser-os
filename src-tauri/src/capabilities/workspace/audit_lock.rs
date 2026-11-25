//! Serialize an entire audit read/modify/atomic-replace across processes.
//!
//! The lock is a stable private file, separate from audit.json (which is
//! replaced atomically). Each operation opens its own non-inheritable handle;
//! no process-wide mutex is held while waiting for another process.

use std::fs::File;
#[cfg(not(target_os = "windows"))]
use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

const LOCK_WAIT: Duration = Duration::from_secs(2);

#[cfg(target_os = "windows")]
type AuditFile = browseros_platform_security::windows::PrivateReadWriteFile;
#[cfg(not(target_os = "windows"))]
type AuditFile = File;

pub(crate) struct AuditLock(AuditFile);
impl AuditLock {
    pub(super) fn acquire(audit_path: &Path) -> io::Result<Self> {
        let parent = audit_path
            .parent()
            .ok_or_else(|| io::Error::other("AUDIT_PARENT_UNAVAILABLE"))?;
        Self::acquire_named(&parent.join("audit-write.lock"), LOCK_WAIT)
    }

    /// Reuse the same OS-owned private lock for other cooperating owners.
    /// Caller must supply a stable name in an already protected directory.
    pub(crate) fn acquire_named(lock_path: &Path, wait: Duration) -> io::Result<Self> {
        #[cfg(not(target_os = "windows"))]
        let owned_file = {
            let mut options = OpenOptions::new();
            options.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options
                    .mode(0o600)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
            }
            options.open(lock_path)?
        };
        #[cfg(target_os = "windows")]
        let owned_file =
            browseros_platform_security::windows::open_private_read_write_file(lock_path)?;
        #[cfg(not(target_os = "windows"))]
        let file = &owned_file;
        #[cfg(target_os = "windows")]
        let file = owned_file.file();
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(io::Error::other("AUDIT_LOCK_NOT_REGULAR"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.uid() != unsafe { libc::geteuid() }
                || metadata.nlink() != 1
                || metadata.mode() & 0o077 != 0
            {
                return Err(io::Error::other("AUDIT_LOCK_NOT_PRIVATE"));
            }
        }
        let started = Instant::now();
        loop {
            match try_lock(file) {
                Ok(true) => return Ok(Self(owned_file)),
                Ok(false) if started.elapsed() < wait => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(false) => {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "AUDIT_LOCK_BUSY"))
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn file(&self) -> &File {
        #[cfg(target_os = "windows")]
        {
            self.0.file()
        }
        #[cfg(not(target_os = "windows"))]
        {
            &self.0
        }
    }
}

#[cfg(unix)]
fn try_lock(file: &File) -> io::Result<bool> {
    use std::os::fd::AsRawFd;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock || error.kind() == io::ErrorKind::Interrupted {
        Ok(false)
    } else {
        Err(error)
    }
}

#[cfg(target_os = "windows")]
fn try_lock(file: &File) -> io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::ERROR_LOCK_VIOLATION;
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;
    let mut position: OVERLAPPED = unsafe { std::mem::zeroed() };
    if unsafe {
        LockFileEx(
            file.as_raw_handle() as _,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut position,
        )
    } != 0
    {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) {
        Ok(false)
    } else {
        Err(error)
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
fn try_lock(_file: &File) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "AUDIT_LOCK_PLATFORM_UNSUPPORTED",
    ))
}

impl Drop for AuditLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            unsafe {
                libc::flock(self.file().as_raw_fd(), libc::LOCK_UN);
            }
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Storage::FileSystem::UnlockFileEx;
            use windows_sys::Win32::System::IO::OVERLAPPED;
            let mut position: OVERLAPPED = unsafe { std::mem::zeroed() };
            unsafe {
                UnlockFileEx(self.file().as_raw_handle() as _, 0, 1, 0, &mut position);
            }
        }
        // Closing the owned descriptor also releases OS locks after an error.
    }
}
