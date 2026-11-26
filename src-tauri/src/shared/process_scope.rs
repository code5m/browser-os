//! Linux delegated cgroup-v2 containment for existing process owners.
//! No daemon, sudo, mount, chmod or implicit delegation. Only the explicit
//! supervised-memory mode enables an already delegated memory controller,
//! and only inside a fresh, empty domain owned by this process.
//! The operator must supply an existing delegated subtree containing this
//! host. This contains ordinary setsid/double-fork descendants; it is not a
//! hostile-same-UID sandbox. A separately privileged actor can move tasks.
//! See ADR D04/D08 and the deployment boundary in the task record.
use std::{
    ffi::{CStr, CString},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt, fs::{MetadataExt, OpenOptionsExt}, process::CommandExt}},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const MOUNT: &str = "/sys/fs/cgroup";
const CGROUP2_MAGIC: i64 = 0x63677270;
const ROOT_ENV: &str = "BROWSEROS_DELEGATED_CGROUP_ROOT";

fn denied() -> io::Error { io::Error::new(io::ErrorKind::PermissionDenied, "BROWSEROS_CGROUP_DELEGATION_INVALID") }
fn open_at(parent: &File, name: &CStr, flags: i32) -> io::Result<File> {
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags | libc::O_CLOEXEC | libc::O_NOFOLLOW) };
    if fd < 0 { Err(io::Error::last_os_error()) } else { Ok(unsafe { File::from_raw_fd(fd) }) }
}
fn cgroup_fs(file: &File) -> io::Result<()> {
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if stat.f_type as i64 != CGROUP2_MAGIC { return Err(denied()); }
    Ok(())
}
fn parse_populated(value: &str) -> io::Result<bool> {
    let mut observed = None;
    for line in value.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() == Some("populated") {
            let populated = match (fields.next(), fields.next()) {
                (Some("0"), None) => false,
                (Some("1"), None) => true,
                _ => return Err(denied()),
            };
            if observed.replace(populated).is_some() { return Err(denied()); }
        }
    }
    observed.ok_or_else(denied)
}
fn has_controller(value: &str, controller: &str) -> bool {
    value.split_whitespace().any(|entry| entry == controller)
}
fn read_control(group: &File, name: &CStr) -> io::Result<String> {
    let mut value = String::new();
    open_at(group, name, libc::O_RDONLY)?.take(4096).read_to_string(&mut value)?;
    Ok(value)
}
fn membership_path() -> io::Result<PathBuf> {
    let value = fs::read_to_string("/proc/self/cgroup")?;
    let mut matches = value.lines().filter_map(|line| line.strip_prefix("0::"));
    let membership = matches.next().ok_or_else(denied)?;
    if matches.next().is_some() || !membership.starts_with('/') { return Err(denied()); }
    fs::canonicalize(Path::new(MOUNT).join(membership.trim_start_matches('/')))
}

/// A fresh, descriptor-bound cgroup. No existing group is ever adopted
/// for termination. Its name is random and only this owner may populate it.
pub(crate) struct ProcessScope {
    parent: File,
    group: File,
    name: CString,
    identity: (u64, u64),
    path: PathBuf,
    spawned: bool,
    host_leaf: Option<HostLeaf>,
}
struct HostLeaf {
    group: File,
    name: CString,
    identity: (u64, u64),
}
impl ProcessScope {
    pub(crate) fn prepare() -> io::Result<Option<Self>> {
        let Some(configured) = std::env::var_os(ROOT_ENV) else { return Ok(None); };
        // Never silently downgrade a configured deployment to process groups.
        let requested = PathBuf::from(configured);
        if !requested.is_absolute() { return Err(denied()); }
        let root = fs::canonicalize(&requested)?;
        let mount = fs::canonicalize(MOUNT)?;
        let current = membership_path()?;
        if root == mount || !root.starts_with(&mount) || !current.starts_with(&root)
            || unsafe { libc::geteuid() } == 0 {
            return Err(denied());
        }
        let parent = OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC).open(&root)?;
        cgroup_fs(&parent)?;
        if parent.metadata()?.uid() != unsafe { libc::geteuid() } { return Err(denied()); }
        let name = CString::new(format!("browseros-run-{}", uuid::Uuid::new_v4())).map_err(|_| denied())?;
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Fresh group only. Cleanup is relative to the pinned parent.
        let group = match open_at(&parent, &name, libc::O_RDONLY | libc::O_DIRECTORY) {
            Ok(group) => group,
            Err(error) => {
                unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR); }
                return Err(error);
            }
        };
        let metadata = group.metadata()?;
        let path = root.join(std::ffi::OsStr::from_bytes(name.as_bytes()));
        let scope = Self { parent, group, name, identity: (metadata.dev(), metadata.ino()), path, spawned: false, host_leaf: None };
        cgroup_fs(&scope.group)?;
        // Require all controls before spawning any user code. The leaf
        // must be empty, and the kernel must provide recursive kill.
        if scope.populated()? { return Err(denied()); }
        open_at(&scope.group, CStr::from_bytes_with_nul(b"cgroup.procs\0").expect("static cgroup filename"), libc::O_WRONLY)?;
        open_at(&scope.group, CStr::from_bytes_with_nul(b"cgroup.kill\0").expect("static cgroup filename"), libc::O_WRONLY)?;
        Ok(Some(scope))
    }

    /// Explicit supervisor-memory topology. Keep the domain root process-free
    /// so Linux's no-internal-process rule permits a memory-controlled child.
    /// The host leaf and nested render jobs are siblings, all beneath the
    /// exact same recursively killable owner. Never enable a controller in
    /// the operator's delegation root or migrate the supervising process.
    pub(crate) fn prepare_nested_memory_host(&mut self) -> io::Result<()> {
        if self.spawned || self.host_leaf.is_some() || self.populated()? {
            return Err(denied());
        }
        let controllers = CStr::from_bytes_with_nul(b"cgroup.controllers\0").expect("static cgroup filename");
        let subtree = CStr::from_bytes_with_nul(b"cgroup.subtree_control\0").expect("static cgroup filename");
        if !has_controller(&read_control(&self.group, controllers)?, "memory") {
            return Err(denied());
        }
        open_at(&self.group, subtree, libc::O_WRONLY)?.write_all(b"+memory")?;
        if !has_controller(&read_control(&self.group, subtree)?, "memory") {
            return Err(denied());
        }
        let name = CString::new("browseros-host").expect("static host leaf");
        if unsafe { libc::mkdirat(self.group.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let group = open_at(&self.group, &name, libc::O_RDONLY | libc::O_DIRECTORY)?;
        let metadata = group.metadata()?;
        self.host_leaf = Some(HostLeaf { group, name, identity: (metadata.dev(), metadata.ino()) });
        let leaf = self.host_leaf.as_ref().ok_or_else(denied)?;
        cgroup_fs(&leaf.group)?;
        let events = CStr::from_bytes_with_nul(b"cgroup.events\0").expect("static cgroup filename");
        if parse_populated(&read_control(&leaf.group, events)?)? { return Err(denied()); }
        let procs = CStr::from_bytes_with_nul(b"cgroup.procs\0").expect("static cgroup filename");
        open_at(&leaf.group, procs, libc::O_WRONLY)?;
        // Verify that the enabled controller really reached a leaf.
        let memory = CStr::from_bytes_with_nul(b"memory.max\0").expect("static cgroup filename");
        open_at(&leaf.group, memory, libc::O_RDONLY)?;
        Ok(())
    }

    /// Set and read back cgroup-accounted memory and swap limits on the
    /// empty owned domain. GPU/external broker allocations are not covered.
    /// No controller delegation or permission changes are performed.
    pub(crate) fn configure_memory_limit(&self, bytes: usize) -> io::Result<()> {
        if self.spawned || self.host_leaf.is_some() || bytes == 0 || self.populated()? { return Err(denied()); }
        fn set(group: &File, name: &CStr, value: &str) -> io::Result<()> {
            open_at(group, name, libc::O_WRONLY)?.write_all(value.as_bytes())?;
            let mut actual = String::new();
            open_at(group, name, libc::O_RDONLY)?.take(128).read_to_string(&mut actual)?;
            if actual.trim() != value { return Err(denied()); }
            Ok(())
        }
        let memory = CStr::from_bytes_with_nul(b"memory.max\0").expect("static cgroup filename");
        let swap = CStr::from_bytes_with_nul(b"memory.swap.max\0").expect("static cgroup filename");
        let oom = CStr::from_bytes_with_nul(b"memory.oom.group\0").expect("static cgroup filename");
        set(&self.group, memory, &bytes.to_string())?;
        set(&self.group, swap, "0")?;
        set(&self.group, oom, "1")?;
        Ok(())
    }

    /// Install membership BEFORE exec, not after the child may have forked.
    /// Only async-signal-safe write is used between fork and exec.
    pub(crate) fn attach(&self, command: &mut Command) -> io::Result<()> {
        let target = self.host_leaf.as_ref().map_or(&self.group, |leaf| &leaf.group);
        let procs = open_at(target, CStr::from_bytes_with_nul(b"cgroup.procs\0").expect("static cgroup filename"), libc::O_WRONLY)?;
        unsafe {
            command.pre_exec(move || {
                loop {
                    let count = libc::write(procs.as_raw_fd(), b"0".as_ptr().cast(), 1);
                    if count == 1 { return Ok(()); }
                    if count < 0 {
                        let error = io::Error::last_os_error();
                        if error.kind() == io::ErrorKind::Interrupted { continue; }
                        return Err(error);
                    }
                    return Err(io::Error::from_raw_os_error(libc::EIO));
                }
            });
        }
        Ok(())
    }

    /// Nested cooperating BrowserOS owners must create their scopes inside
    /// this execution domain, never alongside it in the original delegation.
    pub(crate) fn contain_nested_owners(&self, command: &mut Command) {
        command.env(ROOT_ENV, &self.path);
    }

    fn populated(&self) -> io::Result<bool> {
        let mut value = String::new();
        open_at(&self.group, CStr::from_bytes_with_nul(b"cgroup.events\0").expect("static cgroup filename"), libc::O_RDONLY)?.take(4096).read_to_string(&mut value)?;
        parse_populated(&value)
    }

    /// Only a successful child spawn confers termination authority.
    pub(crate) fn mark_spawned(&mut self) { self.spawned = true; }

    pub(crate) fn request_kill(&self) -> io::Result<()> {
        if !self.spawned { return Err(denied()); }
        open_at(&self.group, CStr::from_bytes_with_nul(b"cgroup.kill\0").expect("static cgroup filename"), libc::O_WRONLY)?.write_all(b"1")
    }

    pub(crate) fn terminate_and_confirm(&self, deadline: Instant) -> io::Result<()> {
        if self.populated()? { self.request_kill()?; }
        loop {
            if !self.populated()? { return Ok(()); }
            if Instant::now() >= deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "BROWSEROS_CGROUP_TERMINATION_UNKNOWN"));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for ProcessScope {
    fn drop(&mut self) {
        // An error/unwind may request cleanup but cannot manufacture a receipt.
        if self.spawned && self.populated().ok() == Some(true) { let _ = self.request_kill(); }
        if self.populated().ok() != Some(false) { return; }
        // Remove only the exact host leaf we created, after recursive-empty
        // confirmation. Unexpected or replaced children are never adopted.
        if let Some(leaf) = &self.host_leaf {
            let Ok(current) = open_at(&self.group, &leaf.name, libc::O_RDONLY | libc::O_DIRECTORY) else { return; };
            let Ok(meta) = current.metadata() else { return; };
            if (meta.dev(), meta.ino()) != leaf.identity { return; }
            if unsafe { libc::unlinkat(self.group.as_raw_fd(), leaf.name.as_ptr(), libc::AT_REMOVEDIR) } != 0 { return; }
        }
        let Ok(current) = open_at(&self.parent, &self.name, libc::O_RDONLY | libc::O_DIRECTORY) else { return; };
        let Ok(meta) = current.metadata() else { return; };
        if (meta.dev(), meta.ino()) == self.identity {
            unsafe { libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), libc::AT_REMOVEDIR); }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controller_readback_requires_an_exact_token() {
        assert!(has_controller("cpu memory pids\n", "memory"));
        assert!(!has_controller("cpu memory_extra pids\n", "memory"));
        assert!(!has_controller("", "memory"));
    }

    #[test]
    #[ignore = "requires an operator-delegated writable Linux cgroup v2 memory subtree"]
    fn supervised_memory_host_runs_in_leaf_under_empty_domain() {
        use std::process::Stdio;
        let mut scope = ProcessScope::prepare().unwrap().expect("explicit delegated root");
        scope.prepare_nested_memory_host().unwrap();
        let mut command = Command::new("/bin/cat");
        command.arg("/proc/self/cgroup").stdout(Stdio::piped());
        scope.attach(&mut command).unwrap();
        scope.contain_nested_owners(&mut command);
        let child = command.spawn().unwrap();
        scope.mark_spawned();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let membership = String::from_utf8(output.stdout).unwrap();
        let expected = format!("/{}/browseros-host", scope.name.to_str().unwrap());
        assert!(membership.lines().any(|line| line.starts_with("0::") && line.ends_with(&expected)));
        let procs = CStr::from_bytes_with_nul(b"cgroup.procs\0").unwrap();
        assert!(read_control(&scope.group, procs).unwrap().trim().is_empty());
        let subtree = CStr::from_bytes_with_nul(b"cgroup.subtree_control\0").unwrap();
        assert!(has_controller(&read_control(&scope.group, subtree).unwrap(), "memory"));
        scope.terminate_and_confirm(Instant::now() + Duration::from_secs(5)).unwrap();
    }

    #[test]
    fn empty_receipt_requires_unambiguous_kernel_population_field() {
        assert_eq!(parse_populated("populated 0\nfrozen 0\n").unwrap(), false);
        assert_eq!(parse_populated("populated 1\nfrozen 0\n").unwrap(), true);
        for invalid in ["", "frozen 0", "populated 0 extra", "populated 2", "populated 0\npopulated 1"] {
            assert!(parse_populated(invalid).is_err());
        }
    }
}
