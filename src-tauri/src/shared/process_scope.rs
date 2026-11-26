//! Linux delegated cgroup-v2 containment for existing process owners.
//! No persistent daemon, sudo, mount or chmod. An ordinary supported release
//! desktop launch may request its own transient user scope from systemd.
//! Only the newly delegated scope and fresh owned domains may enable memory.
//! Alternatively an operator can supply an existing delegated subtree.
//! This contains ordinary setsid/double-fork descendants; it is not a
//! hostile-same-UID sandbox. A separately privileged actor can move tasks.
//! See ADR D04/D08 and the deployment boundary in the task record.
use std::{
    ffi::{CStr, CString},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, RawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
            net::UnixStream,
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const MOUNT: &str = "/sys/fs/cgroup";
const CGROUP2_MAGIC: i64 = 0x63677270;
const ROOT_ENV: &str = "BROWSEROS_DELEGATED_CGROUP_ROOT";
const GUARD_FLAG: &str = "--owned-cgroup-guardian";
const GUARD_GROUP_FD: &str = "BROWSEROS_GUARD_GROUP_FD";
const GUARD_SOCKET_FD: &str = "BROWSEROS_GUARD_SOCKET_FD";
const GUARD_READY: u8 = b'R';
const GUARD_START: Duration = Duration::from_secs(3);

fn group_populated(group: &File) -> io::Result<bool> {
    let events = CStr::from_bytes_with_nul(b"cgroup.events\0").expect("static cgroup filename");
    parse_populated(&read_control(group, events)?)
}
fn group_kill(group: &File) -> io::Result<()> {
    let kill = CStr::from_bytes_with_nul(b"cgroup.kill\0").expect("static cgroup filename");
    open_at(group, kill, libc::O_WRONLY)?.write_all(b"1")
}

/// A one-shot guardian lives outside the owned domain. Only a descriptor
/// to a fresh group is inherited, never a path used as a kill target.
/// The socket is an ownership-liveness channel, not a UI heartbeat.
/// Its parent endpoint is CLOEXEC, so an in-flight fork before attachment
/// retains it until exec/exit: EOF cannot race ahead of that attachment.
struct OrphanGuard {
    control: Option<UnixStream>,
    child: Child,
    warned: bool,
}
impl OrphanGuard {
    fn spawn(group: &File, exe: &Path) -> io::Result<Self> {
        let (mut control, inherited) = UnixStream::pair()?;
        control.set_read_timeout(Some(GUARD_START))?;
        let group_fd = group.as_raw_fd();
        let socket_fd = inherited.as_raw_fd();
        let mut command = Command::new(exe);
        command
            .arg(GUARD_FLAG)
            .env(GUARD_GROUP_FD, group_fd.to_string())
            .env(GUARD_SOCKET_FD, socket_fd.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        // No allocation/locks after fork. All unrelated descriptors remain
        // CLOEXEC, including the owner's endpoint of the liveness socket.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                for fd in [group_fd, socket_fd] {
                    if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        drop(inherited);
        let mut ready = [0u8; 1];
        let accepted = control.read_exact(&mut ready).is_ok()
            && ready[0] == GUARD_READY
            && matches!(child.try_wait(), Ok(None));
        if !accepted {
            // No domain process has been admitted yet.
            drop(control);
            let _ = child.kill();
            let _ = child.wait();
            return Err(denied());
        }
        Ok(Self {
            control: Some(control),
            child,
            warned: false,
        })
    }

    fn observe(&mut self) {
        if !self.warned && !matches!(self.child.try_wait(), Ok(None)) {
            self.warned = true;
            eprintln!("[native-supervisor] Linux orphan guard unavailable; live owner retained");
        }
    }

    // The caller already has the kernel empty-domain receipt and original
    // child exit. Only then may it retire the guardian and unlink the group.
    fn finish_after_empty(&mut self, deadline: Instant) -> io::Result<()> {
        drop(self.control.take());
        loop {
            if let Some(status) = self.child.try_wait()? {
                if !status.success() && !self.warned {
                    self.warned = true;
                    eprintln!("[native-supervisor] Linux orphan guard exited without its own receipt; domain independently confirmed empty");
                }
                return Ok(());
            }
            if Instant::now() >= deadline {
                // No live domain remains. Terminate only this exact owned
                // guardian, never a discovered PID or another process group.
                self.child.kill()?;
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "BROWSEROS_GUARD_EXIT_PENDING",
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn take_guard_fd(name: &str) -> io::Result<RawFd> {
    let value = std::env::var(name).map_err(|_| denied())?;
    std::env::remove_var(name);
    let fd: RawFd = value.parse().map_err(|_| denied())?;
    if fd < 3 || unsafe { libc::fcntl(fd, libc::F_GETFD) } < 0 {
        return Err(denied());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(fd)
}
fn verify_guard_group(group: &File) -> io::Result<()> {
    cgroup_fs(group)?;
    let meta = group.metadata()?;
    let uid = unsafe { libc::geteuid() };
    if uid == 0 || !meta.is_dir() || meta.uid() != uid {
        return Err(denied());
    }
    let root = fs::canonicalize(PathBuf::from(
        std::env::var_os(ROOT_ENV).ok_or_else(denied)?,
    ))?;
    let mount = fs::canonicalize(MOUNT)?;
    let path = fs::read_link(format!("/proc/self/fd/{}", group.as_raw_fd()))?;
    let current = membership_path()?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("browseros-run-"))
        .ok_or_else(denied)?;
    let id = uuid::Uuid::parse_str(name).map_err(|_| denied())?;
    if root == mount
        || !root.starts_with(&mount)
        || path.parent() != Some(root.as_path())
        || !current.starts_with(&root)
        || current.starts_with(&path)
        || id.get_version_num() != 4
        || id.to_string() != name
        || group_populated(group)?
    {
        return Err(denied());
    }
    let kill = CStr::from_bytes_with_nul(b"cgroup.kill\0").expect("static cgroup filename");
    open_at(group, kill, libc::O_WRONLY)?;
    Ok(())
}
fn verify_guard_socket(fd: RawFd) -> io::Result<()> {
    // SO_PEERCRED is kernel-bound to the process creating the socket pair.
    // No guardian starts after a missing/changed original parent.
    let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut peer as *mut libc::ucred).cast(),
            &mut length,
        )
    } < 0
        || length as usize != std::mem::size_of::<libc::ucred>()
        || peer.pid <= 1
        || peer.pid != unsafe { libc::getppid() }
        || peer.uid != unsafe { libc::geteuid() }
    {
        return Err(denied());
    }
    let mut kind: libc::c_int = 0;
    let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&mut kind as *mut libc::c_int).cast(),
            &mut length,
        )
    } < 0
        || length as usize != std::mem::size_of::<libc::c_int>()
        || kind != libc::SOCK_STREAM
    {
        return Err(denied());
    }
    Ok(())
}
fn run_orphan_guard() -> io::Result<()> {
    let group_fd = take_guard_fd(GUARD_GROUP_FD)?;
    let socket_fd = take_guard_fd(GUARD_SOCKET_FD)?;
    if group_fd == socket_fd {
        return Err(denied());
    }
    verify_guard_socket(socket_fd)?;
    // Each validated inherited FD is owned once. No other application
    // startup, plugins, workspace paths or webview code runs in this child.
    let group = unsafe { File::from_raw_fd(group_fd) };
    let mut socket = unsafe { UnixStream::from_raw_fd(socket_fd) };
    verify_guard_group(&group)?;
    socket.write_all(&[GUARD_READY])?;
    let mut byte = [0u8; 1];
    loop {
        match socket.read(&mut byte) {
            Ok(0) => break, // last copy of the original owner endpoint closed
            Ok(_) => {}     // no byte is an instruction or a kill request
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error), // uncertainty is not EOF
        }
    }
    // The parent may have exited normally after confirming emptiness or
    // died before cleanup. The pinned cgroup descriptor is the authority.
    // Do not adopt, enumerate or signal any other PID or scope.
    let mut warned = false;
    loop {
        match group_populated(&group) {
            Ok(false) => return Ok(()),
            Ok(true) => {
                let _ = group_kill(&group);
            }
            Err(_) => {}
        }
        if !warned {
            warned = true;
            eprintln!("[native-supervisor] orphan domain cleanup requested; awaiting kernel empty receipt");
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Exact private early entry: this is not a command routed through Tauri.
/// The guardian cannot accept a caller-provided cgroup path or PID.
pub(crate) fn launch_guard_if_requested() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(GUARD_FLAG)) {
        return None;
    }
    if args.next().is_some() {
        return Some(1);
    }
    Some(match run_orphan_guard() {
        Ok(()) => 0,
        Err(_) => {
            eprintln!("[native-supervisor] invalid or unavailable orphan guard");
            1
        }
    })
}

fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "BROWSEROS_CGROUP_DELEGATION_INVALID",
    )
}
fn open_at(parent: &File, name: &CStr, flags: i32) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
fn cgroup_fs(file: &File) -> io::Result<()> {
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if stat.f_type as i64 != CGROUP2_MAGIC {
        return Err(denied());
    }
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
            if observed.replace(populated).is_some() {
                return Err(denied());
            }
        }
    }
    observed.ok_or_else(denied)
}
fn has_controller(value: &str, controller: &str) -> bool {
    value.split_whitespace().any(|entry| entry == controller)
}
fn read_control(group: &File, name: &CStr) -> io::Result<String> {
    let mut value = String::new();
    open_at(group, name, libc::O_RDONLY)?
        .take(4096)
        .read_to_string(&mut value)?;
    Ok(value)
}
fn membership_path() -> io::Result<PathBuf> {
    let value = fs::read_to_string("/proc/self/cgroup")?;
    let mut matches = value.lines().filter_map(|line| line.strip_prefix("0::"));
    let membership = matches.next().ok_or_else(denied)?;
    if matches.next().is_some() || !membership.starts_with('/') {
        return Err(denied());
    }
    fs::canonicalize(Path::new(MOUNT).join(membership.trim_start_matches('/')))
}

/// The private user-scope bootstrap derives its authority from actual
/// kernel membership, never from a caller-supplied cgroup pathname.
pub(crate) fn user_scope_name(id: uuid::Uuid) -> String {
    format!("browseros-supervisor-{id}.scope")
}
fn sole_pid(value: &str, pid: u32) -> bool {
    let mut fields = value.split_whitespace();
    fields.next().and_then(|field| field.parse::<u32>().ok()) == Some(pid)
        && fields.next().is_none()
}

/// Preflight only. Presence of the standard user-manager socket and
/// cgroup2 controller is NOT proof that systemd will grant delegation.
pub(crate) fn user_scope_runner() -> Option<PathBuf> {
    use std::os::unix::fs::FileTypeExt;
    let uid = unsafe { libc::geteuid() };
    if uid == 0 {
        return None;
    }
    let runner = fs::canonicalize("/usr/bin/systemd-run").ok()?;
    let meta = fs::metadata(&runner).ok()?;
    if !meta.is_file() || meta.uid() != 0 || meta.mode() & 0o022 != 0 || meta.mode() & 0o111 == 0 {
        return None;
    }
    let runtime = PathBuf::from(format!("/run/user/{uid}"));
    if !["systemd/private", "bus"].iter().any(|name| {
        fs::symlink_metadata(runtime.join(name))
            .ok()
            .is_some_and(|meta| meta.uid() == uid && meta.file_type().is_socket())
    }) {
        return None;
    }
    let mount = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(MOUNT)
        .ok()?;
    cgroup_fs(&mount).ok()?;
    let controllers =
        CStr::from_bytes_with_nul(b"cgroup.controllers\0").expect("static cgroup filename");
    if !has_controller(&read_control(&mount, controllers).ok()?, "memory") {
        return None;
    }
    Some(runner)
}

/// Called only in the early exec'd BrowserOS supervisor, before threads,
/// plugins or any GUI initialization. The fresh scope is supplied by the
/// user manager. Move only this exact process into a new leaf before
/// enabling a domain controller (cgroup-v2 no-internal-process rule).
pub(crate) fn bootstrap_user_scope(id: uuid::Uuid) -> io::Result<()> {
    if id.get_version_num() != 4 || std::env::var_os(ROOT_ENV).is_some() {
        return Err(denied());
    }
    let root = membership_path()?;
    let mount = fs::canonicalize(MOUNT)?;
    let name = user_scope_name(id);
    let uid = unsafe { libc::geteuid() };
    if uid == 0
        || root == mount
        || !root.starts_with(&mount)
        || root.file_name() != Some(std::ffi::OsStr::new(&name))
    {
        return Err(denied());
    }
    let group = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&root)?;
    cgroup_fs(&group)?;
    let meta = group.metadata()?;
    if meta.uid() != uid {
        return Err(denied());
    }
    let procs = CStr::from_bytes_with_nul(b"cgroup.procs\0").expect("static cgroup filename");
    let controllers =
        CStr::from_bytes_with_nul(b"cgroup.controllers\0").expect("static cgroup filename");
    let subtree =
        CStr::from_bytes_with_nul(b"cgroup.subtree_control\0").expect("static cgroup filename");
    if !sole_pid(&read_control(&group, procs)?, std::process::id())
        || !has_controller(&read_control(&group, controllers)?, "memory")
        || !read_control(&group, subtree)?.trim().is_empty()
    {
        return Err(denied());
    }
    // Reject an existing subtree rather than adopting another owner.
    for entry in fs::read_dir(&root)? {
        if entry?.file_type()?.is_dir() {
            return Err(denied());
        }
    }
    let leaf_name = CString::new("browseros-supervisor-owner").expect("static owner leaf");
    if unsafe { libc::mkdirat(group.as_raw_fd(), leaf_name.as_ptr(), 0o700) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let leaf = open_at(&group, &leaf_name, libc::O_RDONLY | libc::O_DIRECTORY)?;
    cgroup_fs(&leaf)?;
    if group_populated(&leaf)? {
        return Err(denied());
    }
    // No PID supplied by argv or discovered from another process.
    open_at(&leaf, procs, libc::O_WRONLY)?.write_all(b"0")?;
    let leaf_path = root.join("browseros-supervisor-owner");
    let actual = fs::metadata(&root)?;
    if membership_path()? != leaf_path
        || (actual.dev(), actual.ino()) != (meta.dev(), meta.ino())
        || !read_control(&group, procs)?.trim().is_empty()
        || !sole_pid(&read_control(&leaf, procs)?, std::process::id())
    {
        return Err(denied());
    }
    open_at(&group, subtree, libc::O_WRONLY)?.write_all(b"+memory")?;
    if !has_controller(&read_control(&group, subtree)?, "memory") {
        return Err(denied());
    }
    let memory = CStr::from_bytes_with_nul(b"memory.max\0").expect("static cgroup filename");
    open_at(&leaf, memory, libc::O_RDONLY)?;
    // ProcessScope::prepare still validates each new child, recursive kill,
    // nested memory controls and the independent orphan guardian.
    std::env::set_var(ROOT_ENV, root);
    Ok(())
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
    orphan_guard: Option<OrphanGuard>,
}
struct HostLeaf {
    group: File,
    name: CString,
    identity: (u64, u64),
}
impl ProcessScope {
    pub(crate) fn prepare() -> io::Result<Option<Self>> {
        let Some(configured) = std::env::var_os(ROOT_ENV) else {
            return Ok(None);
        };
        // Never silently downgrade a configured deployment to process groups.
        let requested = PathBuf::from(configured);
        if !requested.is_absolute() {
            return Err(denied());
        }
        let root = fs::canonicalize(&requested)?;
        let mount = fs::canonicalize(MOUNT)?;
        let current = membership_path()?;
        if root == mount
            || !root.starts_with(&mount)
            || !current.starts_with(&root)
            || unsafe { libc::geteuid() } == 0
        {
            return Err(denied());
        }
        let parent = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&root)?;
        cgroup_fs(&parent)?;
        if parent.metadata()?.uid() != unsafe { libc::geteuid() } {
            return Err(denied());
        }
        let name = CString::new(format!("browseros-run-{}", uuid::Uuid::new_v4()))
            .map_err(|_| denied())?;
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Fresh group only. Cleanup is relative to the pinned parent.
        let group = match open_at(&parent, &name, libc::O_RDONLY | libc::O_DIRECTORY) {
            Ok(group) => group,
            Err(error) => {
                unsafe {
                    libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR);
                }
                return Err(error);
            }
        };
        let metadata = group.metadata()?;
        let path = root.join(std::ffi::OsStr::from_bytes(name.as_bytes()));
        let scope = Self {
            parent,
            group,
            name,
            identity: (metadata.dev(), metadata.ino()),
            path,
            spawned: false,
            host_leaf: None,
            orphan_guard: None,
        };
        cgroup_fs(&scope.group)?;
        // Require all controls before spawning any user code. The leaf
        // must be empty, and the kernel must provide recursive kill.
        if scope.populated()? {
            return Err(denied());
        }
        open_at(
            &scope.group,
            CStr::from_bytes_with_nul(b"cgroup.procs\0").expect("static cgroup filename"),
            libc::O_WRONLY,
        )?;
        open_at(
            &scope.group,
            CStr::from_bytes_with_nul(b"cgroup.kill\0").expect("static cgroup filename"),
            libc::O_WRONLY,
        )?;
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
        let controllers =
            CStr::from_bytes_with_nul(b"cgroup.controllers\0").expect("static cgroup filename");
        let subtree =
            CStr::from_bytes_with_nul(b"cgroup.subtree_control\0").expect("static cgroup filename");
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
        self.host_leaf = Some(HostLeaf {
            group,
            name,
            identity: (metadata.dev(), metadata.ino()),
        });
        let leaf = self.host_leaf.as_ref().ok_or_else(denied)?;
        cgroup_fs(&leaf.group)?;
        let events = CStr::from_bytes_with_nul(b"cgroup.events\0").expect("static cgroup filename");
        if parse_populated(&read_control(&leaf.group, events)?)? {
            return Err(denied());
        }
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
        if self.spawned || self.host_leaf.is_some() || bytes == 0 || self.populated()? {
            return Err(denied());
        }
        fn set(group: &File, name: &CStr, value: &str) -> io::Result<()> {
            open_at(group, name, libc::O_WRONLY)?.write_all(value.as_bytes())?;
            let mut actual = String::new();
            open_at(group, name, libc::O_RDONLY)?
                .take(128)
                .read_to_string(&mut actual)?;
            if actual.trim() != value {
                return Err(denied());
            }
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

    /// Arm the independent owner-liveness guard before admitting any host
    /// or renderer. Failure is a pre-dispatch failure, never an unguarded
    /// strict-mode launch. Existing non-supervisor ProcessScope callers
    /// retain their previous behavior.
    pub(crate) fn arm_orphan_guard(&mut self) -> io::Result<()> {
        if self.spawned || self.orphan_guard.is_some() || self.populated()? {
            return Err(denied());
        }
        let exe = std::env::current_exe()?;
        self.orphan_guard = Some(OrphanGuard::spawn(&self.group, &exe)?);
        Ok(())
    }
    pub(crate) fn observe_orphan_guard(&mut self) {
        if let Some(guard) = &mut self.orphan_guard {
            guard.observe();
        }
    }
    pub(crate) fn finish_orphan_guard(&mut self, deadline: Instant) -> io::Result<()> {
        if self.populated()? {
            return Err(denied());
        }
        if let Some(guard) = &mut self.orphan_guard {
            guard.finish_after_empty(deadline)?;
        }
        self.orphan_guard = None;
        Ok(())
    }

    /// Install membership BEFORE exec, not after the child may have forked.
    /// Only async-signal-safe write is used between fork and exec.
    pub(crate) fn attach(&self, command: &mut Command) -> io::Result<()> {
        let target = self
            .host_leaf
            .as_ref()
            .map_or(&self.group, |leaf| &leaf.group);
        let procs = open_at(
            target,
            CStr::from_bytes_with_nul(b"cgroup.procs\0").expect("static cgroup filename"),
            libc::O_WRONLY,
        )?;
        unsafe {
            command.pre_exec(move || loop {
                let count = libc::write(procs.as_raw_fd(), b"0".as_ptr().cast(), 1);
                if count == 1 {
                    return Ok(());
                }
                if count < 0 {
                    let error = io::Error::last_os_error();
                    if error.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(error);
                }
                return Err(io::Error::from_raw_os_error(libc::EIO));
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
        group_populated(&self.group)
    }

    /// Only a successful child spawn confers termination authority.
    pub(crate) fn mark_spawned(&mut self) {
        self.spawned = true;
    }

    pub(crate) fn request_kill(&self) -> io::Result<()> {
        if !self.spawned {
            return Err(denied());
        }
        group_kill(&self.group)
    }

    pub(crate) fn terminate_and_confirm(&self, deadline: Instant) -> io::Result<()> {
        if self.populated()? {
            self.request_kill()?;
        }
        loop {
            if !self.populated()? {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "BROWSEROS_CGROUP_TERMINATION_UNKNOWN",
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for ProcessScope {
    fn drop(&mut self) {
        // An error/unwind may request cleanup but cannot manufacture a receipt.
        if self.spawned && self.populated().ok() == Some(true) {
            let _ = self.request_kill();
        }
        if self.populated().ok() != Some(false) {
            return;
        }
        // The guardian must stop accessing the descriptor before unlink.
        // On an uncertain exit leave the directory and let its endpoint
        // close with the owner; never claim a cleanup receipt from Drop.
        if self.orphan_guard.is_some()
            && self
                .finish_orphan_guard(Instant::now() + GUARD_START)
                .is_err()
        {
            return;
        }
        // Remove only the exact host leaf we created, after recursive-empty
        // confirmation. Unexpected or replaced children are never adopted.
        if let Some(leaf) = &self.host_leaf {
            let Ok(current) = open_at(&self.group, &leaf.name, libc::O_RDONLY | libc::O_DIRECTORY)
            else {
                return;
            };
            let Ok(meta) = current.metadata() else {
                return;
            };
            if (meta.dev(), meta.ino()) != leaf.identity {
                return;
            }
            if unsafe {
                libc::unlinkat(
                    self.group.as_raw_fd(),
                    leaf.name.as_ptr(),
                    libc::AT_REMOVEDIR,
                )
            } != 0
            {
                return;
            }
        }
        let Ok(current) = open_at(&self.parent, &self.name, libc::O_RDONLY | libc::O_DIRECTORY)
        else {
            return;
        };
        let Ok(meta) = current.metadata() else {
            return;
        };
        if (meta.dev(), meta.ino()) == self.identity {
            unsafe {
                libc::unlinkat(
                    self.parent.as_raw_fd(),
                    self.name.as_ptr(),
                    libc::AT_REMOVEDIR,
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_scope_admission_requires_only_the_current_pid() {
        assert!(sole_pid("42\n", 42));
        for invalid in ["", "43\n", "42\n43\n", "42 42", "42 junk"] {
            assert!(!sole_pid(invalid, 42));
        }
        let id = uuid::Uuid::parse_str("ec111c8b-8019-4c21-b0c2-aee29735a323").unwrap();
        assert_eq!(
            user_scope_name(id),
            "browseros-supervisor-ec111c8b-8019-4c21-b0c2-aee29735a323.scope"
        );
    }

    #[test]
    #[ignore = "run this exact compiled test binary alone inside a fresh delegated systemd user scope named for BROWSEROS_TEST_USER_SCOPE_ID"]
    fn fresh_user_scope_bootstrap_exposes_nested_memory_domain() {
        let id =
            uuid::Uuid::parse_str(&std::env::var("BROWSEROS_TEST_USER_SCOPE_ID").unwrap()).unwrap();
        bootstrap_user_scope(id).unwrap();
        let mut owned = ProcessScope::prepare().unwrap().unwrap();
        owned.prepare_nested_memory_host().unwrap();
        assert!(!owned.populated().unwrap());
    }

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
        let mut scope = ProcessScope::prepare()
            .unwrap()
            .expect("explicit delegated root");
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
        assert!(membership
            .lines()
            .any(|line| line.starts_with("0::") && line.ends_with(&expected)));
        let procs = CStr::from_bytes_with_nul(b"cgroup.procs\0").unwrap();
        assert!(read_control(&scope.group, procs).unwrap().trim().is_empty());
        let subtree = CStr::from_bytes_with_nul(b"cgroup.subtree_control\0").unwrap();
        assert!(has_controller(
            &read_control(&scope.group, subtree).unwrap(),
            "memory"
        ));
        scope
            .terminate_and_confirm(Instant::now() + Duration::from_secs(5))
            .unwrap();
    }

    #[test]
    #[ignore = "requires writable delegated cgroup v2 and BROWSEROS_TEST_GUARD_EXE pointing to the real application binary"]
    fn orphan_guard_retires_owned_process_after_owner_endpoint_loss() {
        let mut scope = ProcessScope::prepare()
            .unwrap()
            .expect("explicit delegated root");
        let exe = PathBuf::from(
            std::env::var_os("BROWSEROS_TEST_GUARD_EXE").expect("real application binary"),
        );
        scope.orphan_guard = Some(OrphanGuard::spawn(&scope.group, &exe).unwrap());
        let mut command = Command::new("/bin/sleep");
        command.arg("60");
        scope.attach(&mut command).unwrap();
        let mut child = command.spawn().unwrap();
        scope.mark_spawned();
        assert!(scope.populated().unwrap());
        // Simulate loss of the owner's liveness endpoint, not a UI timeout.
        drop(scope.orphan_guard.as_mut().unwrap().control.take());
        let deadline = Instant::now() + Duration::from_secs(10);
        while scope.populated().unwrap() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!scope.populated().unwrap());
        assert!(!child.wait().unwrap().success());
        scope.finish_orphan_guard(deadline).unwrap();
    }

    #[test]
    fn empty_receipt_requires_unambiguous_kernel_population_field() {
        assert_eq!(parse_populated("populated 0\nfrozen 0\n").unwrap(), false);
        assert_eq!(parse_populated("populated 1\nfrozen 0\n").unwrap(), true);
        for invalid in [
            "",
            "frozen 0",
            "populated 0 extra",
            "populated 2",
            "populated 0\npopulated 1",
        ] {
            assert!(parse_populated(invalid).is_err());
        }
    }
}
