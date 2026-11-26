//! Process-lifetime ownership of the existing persistent browser profile.
//! The ordinary main host and its Grid hosts are one explicitly admitted
//! group. This is not a new profile, credential export, or hostile same-user
//! sandbox. It prevents overlapping cooperating BrowserOS host epochs.
//! Native renderer descendants that outlive every host remain the separate
//! responsibility of the existing process-domain supervisor.
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub(crate) const GRID_EPOCH_ENV: &str = "BROWSEROS_GRID_PROFILE_EPOCH";
const DIRECTORY: &str = "browser-profile-owner-v1";
const GATE: &str = "admission.lock";
const LEADER: &str = "leader.lock";
const MEMBERS: &str = "members.lock";
const EPOCH_LEN: usize = 36;
const GATE_TIMEOUT: Duration = Duration::from_secs(2);

fn denied(code: &'static str) -> io::Error {
    io::Error::other(code)
}

#[cfg(target_os = "linux")]
struct LockFile {
    file: File,
    // Keep the authorized directory descriptor for the lifetime of the
    // opened lock. An independent same-UID replacement is not protected.
    _directory: File,
}

#[cfg(target_os = "windows")]
struct LockFile(browseros_platform_security::windows::PrivateReadWriteFile);

impl LockFile {
    #[cfg(target_os = "linux")]
    fn open(directory: &Path, name: &str) -> io::Result<Self> {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let parent = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(directory)?;
        let owner = unsafe { libc::geteuid() };
        let meta = parent.metadata()?;
        if !meta.is_dir() || meta.uid() != owner || meta.mode() & 0o077 != 0 {
            return Err(denied("BROWSER_PROFILE_OWNER_DIRECTORY_DENIED"));
        }
        let name = std::ffi::CString::new(name)
            .map_err(|_| denied("BROWSER_PROFILE_OWNER_LOCK_NAME_INVALID"))?;
        // Only fixed, private direct children; never follow links or
        // truncate/replace an existing lock object.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                0o600 as libc::mode_t,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let meta = file.metadata()?;
        if !meta.is_file()
            || meta.uid() != owner
            || meta.nlink() != 1
            || meta.mode() & 0o077 != 0
        {
            return Err(denied("BROWSER_PROFILE_OWNER_LOCK_DENIED"));
        }
        Ok(Self { file, _directory: parent })
    }

    #[cfg(target_os = "windows")]
    fn open(directory: &Path, name: &str) -> io::Result<Self> {
        // Existing native owner/SYSTEM file checks pin the parent chain and
        // reject hardlinks and reparse points. Do not create a second ACL path.
        browseros_platform_security::windows::open_private_read_write_file(&directory.join(name))
            .map(Self)
    }

    fn file(&self) -> &File {
        #[cfg(target_os = "linux")]
        {
            &self.file
        }
        #[cfg(target_os = "windows")]
        {
            self.0.file()
        }
    }

    fn try_acquire(&self, exclusive: bool) -> io::Result<bool> {
        // Rust's standard native file locks use flock on Unix and
        // LockFileEx on Windows. Each participant opens its own handle.
        let result = if exclusive {
            self.file().try_lock()
        } else {
            self.file().try_lock_shared()
        };
        match result {
            Ok(()) => Ok(true),
            Err(std::fs::TryLockError::WouldBlock) => Ok(false),
            Err(std::fs::TryLockError::Error(error)) => Err(error),
        }
    }
}

fn gate(directory: &Path) -> io::Result<LockFile> {
    let gate = LockFile::open(directory, GATE)?;
    let deadline = Instant::now() + GATE_TIMEOUT;
    loop {
        if gate.try_acquire(true)? {
            return Ok(gate);
        }
        if Instant::now() >= deadline {
            return Err(denied("BROWSER_PROFILE_ADMISSION_BUSY"));
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn try_lock(directory: &Path, name: &str, exclusive: bool) -> io::Result<Option<LockFile>> {
    let file = LockFile::open(directory, name)?;
    if file.try_acquire(exclusive)? {
        Ok(Some(file))
    } else {
        Ok(None)
    }
}

fn parse_epoch(raw: &str) -> io::Result<Uuid> {
    let id = Uuid::parse_str(raw).map_err(|_| denied("BROWSER_PROFILE_EPOCH_INVALID"))?;
    if id.get_version_num() != 4 || raw != id.to_string() {
        return Err(denied("BROWSER_PROFILE_EPOCH_INVALID"));
    }
    Ok(id)
}

fn read_epoch(gate: &LockFile) -> io::Result<Uuid> {
    let mut file = gate.file();
    if file.metadata()?.len() != EPOCH_LEN as u64 {
        return Err(denied("BROWSER_PROFILE_EPOCH_INVALID"));
    }
    let mut raw = [0u8; EPOCH_LEN];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut raw)?;
    parse_epoch(std::str::from_utf8(&raw).map_err(|_| denied("BROWSER_PROFILE_EPOCH_INVALID"))?)
}

fn publish_epoch(gate: &LockFile, epoch: Uuid) -> io::Result<()> {
    // The gate is both the admission lock and its fixed-size epoch record.
    // A new leader may repair a short interrupted record only AFTER proving
    // there are no older host members. Never unlink or truncate the lock.
    let mut file = gate.file();
    if file.metadata()?.len() > EPOCH_LEN as u64 {
        return Err(denied("BROWSER_PROFILE_EPOCH_INVALID"));
    }
    file.seek(SeekFrom::Start(0))?;
    file.write_all(epoch.to_string().as_bytes())?;
    file.sync_data()
}

struct ProfileLease {
    epoch: Uuid,
    _member: LockFile,
    leader: Option<LockFile>,
}

impl ProfileLease {
    fn main(directory: &Path) -> io::Result<Self> {
        let gate = gate(directory)?;
        let leader = try_lock(directory, LEADER, true)?
            .ok_or_else(|| denied("BROWSER_PROFILE_LEADER_ACTIVE"))?;
        // A main host that crashed may still have live Grid hosts. Merely
        // acquiring the leader lock is NOT permission to reuse its profile.
        let empty = try_lock(directory, MEMBERS, true)?
            .ok_or_else(|| denied("BROWSER_PROFILE_PREVIOUS_HOSTS_ACTIVE"))?;
        // The admission gate serializes this unlock/reopen conversion.
        // No cooperating member can join in the intervening interval.
        drop(empty);
        let member = try_lock(directory, MEMBERS, false)?
            .ok_or_else(|| denied("BROWSER_PROFILE_MEMBER_LOCK_BUSY"))?;
        let epoch = Uuid::new_v4();
        publish_epoch(&gate, epoch)?;
        Ok(Self { epoch, _member: member, leader: Some(leader) })
    }

    fn grid(directory: &Path, expected: Uuid) -> io::Result<Self> {
        let gate = gate(directory)?;
        let member = try_lock(directory, MEMBERS, false)?
            .ok_or_else(|| denied("BROWSER_PROFILE_MEMBER_LOCK_BUSY"))?;
        // Hold membership BEFORE observing leader liveness. If the leader
        // dies immediately after the check, a new leader is still fenced.
        if try_lock(directory, LEADER, true)?.is_some() {
            return Err(denied("BROWSER_PROFILE_PARENT_RETIRED"));
        }
        if read_epoch(&gate)? != expected {
            return Err(denied("BROWSER_PROFILE_STALE_EPOCH"));
        }
        Ok(Self { epoch: expected, _member: member, leader: None })
    }
}

// Never release the host membership at UI teardown or tab close. Static
// ownership intentionally lasts until OS process termination, including
// startup failure paths; the kernel closes the underlying handles.
static HOST_PROFILE: OnceLock<ProfileLease> = OnceLock::new();

pub(crate) fn prepare(data_dir: &Path, grid: bool) -> io::Result<()> {
    if HOST_PROFILE.get().is_some() {
        return Err(denied("BROWSER_PROFILE_ALREADY_ADMITTED"));
    }
    let directory = data_dir.join(DIRECTORY);
    crate::capabilities::workspace::safe_paths::ensure_private_directory(&directory)?;
    let lease = if grid {
        let raw = std::env::var(GRID_EPOCH_ENV)
            .map_err(|_| denied("BROWSER_PROFILE_PARENT_EPOCH_REQUIRED"))?;
        // Remove inherited correlation from the subsequent WebView process
        // environment. It is not a secret or an IPC authorization token.
        std::env::remove_var(GRID_EPOCH_ENV);
        ProfileLease::grid(&directory, parse_epoch(&raw)?)?
    } else {
        std::env::remove_var(GRID_EPOCH_ENV);
        ProfileLease::main(&directory)?
    };
    HOST_PROFILE.set(lease)
        .map_err(|_| denied("BROWSER_PROFILE_ALREADY_ADMITTED"))?;
    eprintln!("BROWSER_PROFILE_HOST_READY role={}", if grid { "grid" } else { "main" });
    Ok(())
}

pub(crate) fn grid_epoch() -> Result<String, String> {
    HOST_PROFILE.get()
        .filter(|lease| lease.leader.is_some())
        .map(|lease| lease.epoch.to_string())
        .ok_or_else(|| "BROWSER_PROFILE_LEADER_REQUIRED".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHILD_ROOT: &str = "BROWSEROS_PROFILE_TEST_ROOT";
    const CHILD_EPOCH: &str = "BROWSEROS_PROFILE_TEST_EPOCH";

    fn fixture() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("browseros-profile-owner-{}", Uuid::new_v4()));
        crate::capabilities::workspace::safe_paths::ensure_private_directory(&path).unwrap();
        path
    }

    #[test]
    fn profile_member_child_fixture() {
        let Some(root) = std::env::var_os(CHILD_ROOT) else { return };
        let root = std::path::PathBuf::from(root);
        let epoch = parse_epoch(&std::env::var(CHILD_EPOCH).unwrap()).unwrap();
        let _lease = ProfileLease::grid(&root, epoch).unwrap();
        std::fs::write(root.join("fixture-ready"), b"ready").unwrap();
        // Parent controls the lifetime; EOF ends the fixture.
        let mut byte = [0u8; 1];
        assert_eq!(std::io::stdin().read(&mut byte).unwrap(), 0);
    }

    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn old_grid_process_fences_new_leader_until_member_exits() {
        use std::process::{Command, Stdio};
        let root = fixture();
        let old = ProfileLease::main(&root).unwrap();
        let epoch = old.epoch;
        let mut child = Child(Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "browser_profile_owner::tests::profile_member_child_fixture", "--nocapture"])
            .env(CHILD_ROOT, &root)
            .env(CHILD_EPOCH, epoch.to_string())
            .stdin(Stdio::piped())
            .spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.join("fixture-ready").exists() {
            assert!(child.0.try_wait().unwrap().is_none());
            assert!(Instant::now() < deadline, "fixture did not acquire membership");
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(old);
        let error = ProfileLease::main(&root).err().expect("old Grid must fence a new epoch");
        assert_eq!(error.to_string(), "BROWSER_PROFILE_PREVIOUS_HOSTS_ACTIVE");
        // Parent retirement makes further joins fail even with the old epoch.
        assert!(ProfileLease::grid(&root, epoch).is_err());
        drop(child.0.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline, "fixture did not retire");
            std::thread::sleep(Duration::from_millis(20));
        }
        let next = ProfileLease::main(&root).unwrap();
        assert_ne!(next.epoch, epoch);
        drop(next);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_epoch_cannot_be_adopted() {
        assert!(parse_epoch("").is_err());
        assert!(parse_epoch("00000000-0000-0000-0000-000000000000").is_err());
        let epoch = Uuid::new_v4();
        assert_eq!(parse_epoch(&epoch.to_string()).unwrap(), epoch);
        assert!(parse_epoch(&epoch.simple().to_string()).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn lock_link_is_never_followed_or_replaced() {
        let root = fixture();
        let target = root.join("target");
        std::fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(&target, root.join(GATE)).unwrap();
        assert!(ProfileLease::main(&root).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"keep");
        std::fs::remove_dir_all(root).unwrap();
    }
}
