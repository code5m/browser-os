//! External supervisor of the existing application fault domain.
//! No GTK/WebView lifetime or layout changes. This is NOT per-tab renderer
//! isolation. A recovery restarts the whole owned app and can lose volatile
//! state. Release GUI launch selects supervision on Windows and on Linux
//! with an existing delegation or an eligible fresh systemd user scope.
//! Explicit modes remain available, including --direct-start for rollback.
//! No discovery/adoption of an existing PID, no automatic URL replay.
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command, ExitStatus},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;

const FLAG: &str = "--supervised-recovery";
const MEMORY_FLAG: &str = "--supervised-recovery-memory";
const DIRECT_FLAG: &str = "--direct-start";
const USER_SCOPE_FLAG: &str = "--owned-user-scope";
const ADDRESS: &str = "BROWSEROS_SUPERVISOR_ADDRESS";
const TOKEN: &str = "BROWSEROS_SUPERVISOR_TOKEN";
const POLL: Duration = Duration::from_millis(250);
const PULSE: Duration = Duration::from_secs(2);
const HANG: Duration = Duration::from_secs(60);
// Silence is a transport failure, not proof that the native UI is unresponsive.
const REPLY_LIMIT: Duration = Duration::from_secs(3);
const MAX_PENDING_GAP: Duration = Duration::from_secs(6);
const CLEANUP: Duration = Duration::from_secs(15);
const STARTUP_WARNING: Duration = Duration::from_secs(120);

// The native IPC may request recovery only while an authenticated supervisor
// probe is live. Losing this transport is never evidence of a native hang.
static DOMAIN_RECOVERY_READY: AtomicBool = AtomicBool::new(false);

pub(crate) fn domain_recovery_ready() -> bool {
    DOMAIN_RECOVERY_READY.load(Ordering::Acquire)
}

struct ProbeConnection;

impl Drop for ProbeConnection {
    fn drop(&mut self) {
        DOMAIN_RECOVERY_READY.store(false, Ordering::Release);
    }
}

pub(crate) struct OwnedHost {
    child: Child,
    #[cfg(target_os = "linux")]
    scope: crate::shared::process_scope::ProcessScope,
    #[cfg(target_os = "windows")]
    job: crate::windows_process::JobObject,
    #[cfg(target_os = "windows")]
    assignment_incomplete: bool,
}
impl OwnedHost {
    fn spawn(
        args: &[OsString],
        listener: &TcpListener,
        token: &str,
        nested_memory: bool,
    ) -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|_| "SUPERVISOR_EXECUTABLE_UNAVAILABLE")?;
        let mut command = Command::new(exe);
        command
            .args(args)
            .env(
                ADDRESS,
                listener
                    .local_addr()
                    .map_err(|_| "SUPERVISOR_LISTENER_FAILED")?
                    .to_string(),
            )
            .env(TOKEN, token);
        Self::spawn_command_with_topology(command, None, nested_memory, || {
            eprintln!("[native-supervisor] spawn cleanup pending; no replacement admitted");
        })
    }

    /// Shared owned process domain. An optional budget must be installed
    /// before any worker code can execute; no unbounded fallback.
    pub(crate) fn spawn_command(
        command: Command,
        memory_limit: Option<usize>,
        on_retire_pending: impl FnOnce(),
    ) -> Result<Self, String> {
        Self::spawn_command_with_topology(command, memory_limit, false, on_retire_pending)
    }

    fn spawn_command_with_topology(
        mut command: Command,
        memory_limit: Option<usize>,
        nested_memory: bool,
        _on_retire_pending: impl FnOnce(),
    ) -> Result<Self, String> {
        if nested_memory && memory_limit.is_some() {
            return Err("SUPERVISOR_TOPOLOGY_CONFLICT".into());
        }
        #[cfg(target_os = "linux")]
        {
            let mut scope = crate::shared::process_scope::ProcessScope::prepare()
                .map_err(|_| "SUPERVISOR_CGROUP_UNAVAILABLE")?
                .ok_or("SUPERVISOR_DELEGATED_CGROUP_REQUIRED")?;
            if nested_memory {
                scope
                    .prepare_nested_memory_host()
                    .map_err(|_| "SUPERVISOR_NESTED_MEMORY_UNAVAILABLE")?;
            }
            if let Some(bytes) = memory_limit {
                scope
                    .configure_memory_limit(bytes)
                    .map_err(|_| "SUPERVISOR_MEMORY_BUDGET_UNAVAILABLE")?;
            }
            scope
                .arm_orphan_guard()
                .map_err(|_| "SUPERVISOR_ORPHAN_GUARD_UNAVAILABLE")?;
            scope
                .attach(&mut command)
                .map_err(|_| "SUPERVISOR_CGROUP_ATTACH_FAILED")?;
            scope.contain_nested_owners(&mut command);
            let child = command.spawn().map_err(|_| "SUPERVISOR_SPAWN_FAILED")?;
            scope.mark_spawned();
            Ok(Self { child, scope })
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            if nested_memory {
                return Err("SUPERVISOR_MEMORY_TOPOLOGY_LINUX_ONLY".into());
            }
            command.creation_flags(crate::windows_process::CREATE_SUSPENDED);
            let job = match memory_limit {
                Some(bytes) => crate::windows_process::JobObject::create_with_memory_limit(bytes)?,
                None => crate::windows_process::JobObject::create_for_spawn()?,
            };
            let child = command.spawn().map_err(|_| "SUPERVISOR_SPAWN_FAILED")?;
            let mut owner = Self {
                child,
                job,
                assignment_incomplete: true,
            };
            if owner.job.assign_suspended_pid(owner.child.id()).is_err() {
                // The process may not be in the job if assignment failed.
                // Only our own Child handle can be used in this error path.
                let _ = owner.child.kill();
                // Retain the real Child and Job even when assignment failed.
                // Returning a normal failure while a suspended or partially
                // assigned child is still alive would authorize another run.
                owner.retire_confirmed(_on_retire_pending);
                return Err("SUPERVISOR_JOB_ASSIGN_FAILED".into());
            }
            owner.assignment_incomplete = false;
            Ok(owner)
        }
    }

    pub(crate) fn exited(&mut self) -> Result<Option<ExitStatus>, String> {
        #[cfg(target_os = "linux")]
        self.scope.observe_orphan_guard();
        self.child
            .try_wait()
            .map_err(|_| "SUPERVISOR_WAIT_FAILED".into())
    }

    /// A kill request is not a receipt. Both the kernel's domain accounting
    /// and the original Child wait must confirm before another launch.
    pub(crate) fn retire(&mut self) -> Result<ExitStatus, String> {
        let deadline = Instant::now() + CLEANUP;
        #[cfg(target_os = "linux")]
        self.scope
            .terminate_and_confirm(deadline)
            .map_err(|_| "SUPERVISOR_SCOPE_EXIT_UNKNOWN")?;
        #[cfg(target_os = "windows")]
        {
            if !self.job.is_empty()? {
                self.job.terminate()?;
            }
            loop {
                if self.job.is_empty()? {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err("SUPERVISOR_JOB_EXIT_UNKNOWN".into());
                }
                thread::sleep(POLL);
            }
        }
        loop {
            if let Some(status) = self.exited()? {
                #[cfg(target_os = "linux")]
                self.scope
                    .finish_orphan_guard(deadline)
                    .map_err(|_| "SUPERVISOR_GUARD_EXIT_UNKNOWN")?;
                return Ok(status);
            }
            // Failed Job assignment may leave the original suspended child
            // outside the Job. An empty Job is not proof of that child's exit.
            #[cfg(target_os = "windows")]
            if self.assignment_incomplete {
                let _ = self.child.kill();
            }
            if Instant::now() >= deadline {
                return Err("SUPERVISOR_CHILD_EXIT_UNKNOWN".into());
            }
            thread::sleep(POLL);
        }
    }

    /// For a recoverable job with a retained owner, a cleanup deadline is
    /// a diagnostic boundary, not a permission to drop the owner. One worker
    /// keeps its kernel handles and continues trying until actual exit.
    pub(crate) fn retire_confirmed(&mut self, on_pending: impl FnOnce()) -> ExitStatus {
        match self.retire() {
            Ok(status) => return status,
            Err(_) => on_pending(),
        }
        loop {
            thread::sleep(POLL);
            if let Ok(status) = self.retire() {
                return status;
            }
        }
    }

    fn wait_without_recovery(&mut self) -> ExitStatus {
        // Losing the probe is not authority to terminate a healthy app.
        // Nor may a wait error drop the Job or cgroup owner on this path.
        let mut warned = false;
        loop {
            match self.exited() {
                Ok(Some(_)) => break,
                Ok(None) => {}
                Err(_) if !warned => {
                    warned = true;
                    eprintln!("[native-supervisor] child exit unconfirmed; retaining owner without restart");
                }
                Err(_) => {}
            }
            thread::sleep(POLL);
        }
        self.retire_confirmed(report_retirement_pending)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeDecision {
    Wait,
    Recover,
    NoRecovery,
}
/// R = genuine UI acknowledgement, B = live probe thread awaiting a UI
/// callback, S = recovery suspended. Gaps invalidate pending evidence.
#[derive(Default)]
struct ProbeHealth {
    ready: bool,
    pending_since: Option<Instant>,
    last_pending: Option<Instant>,
}
impl ProbeHealth {
    fn observe(&mut self, reply: u8, now: Instant) -> ProbeDecision {
        match reply {
            b'R' => {
                self.ready = true;
                self.pending_since = None;
                self.last_pending = None;
                ProbeDecision::Wait
            }
            b'S' => {
                *self = Self::default();
                ProbeDecision::Wait
            }
            b'B' if self.ready => {
                if self.last_pending.map_or(true, |previous| {
                    now.saturating_duration_since(previous) > MAX_PENDING_GAP
                }) {
                    self.pending_since = Some(now);
                }
                self.last_pending = Some(now);
                if self
                    .pending_since
                    .is_some_and(|start| now.saturating_duration_since(start) >= HANG)
                {
                    ProbeDecision::Recover
                } else {
                    ProbeDecision::Wait
                }
            }
            b'B' => ProbeDecision::Wait,
            // M is a separate, explicit user request from the live native
            // owner, not an inference from a slow or lost UI callback.
            b'M' => ProbeDecision::Recover,
            _ => ProbeDecision::NoRecovery,
        }
    }
}
fn report_retirement_pending() {
    eprintln!(
        "[native-supervisor] domain exit unconfirmed; retaining owner, no replacement admitted"
    );
}

enum Outcome {
    Exited(ExitStatus),
    Unresponsive,
    ProbeUnavailable,
}

fn accepted_stream(
    listener: &TcpListener,
    owner: &mut OwnedHost,
    token: &str,
) -> Result<Option<TcpStream>, String> {
    let started = Instant::now();
    let mut warned = false;
    loop {
        if owner.exited()?.is_some() {
            return Ok(None);
        }
        match listener.accept() {
            Ok((mut stream, address)) if address.ip().is_loopback() => {
                if stream.set_read_timeout(Some(POLL)).is_err()
                    || stream.set_write_timeout(Some(POLL)).is_err()
                {
                    continue;
                }
                // One fixed-size token, no variable-length remote input.
                // Incomplete or unauthorized local connections are discarded.
                let mut hello = [0u8; 36];
                if stream.read_exact(&mut hello).is_ok() && hello.as_slice() == token.as_bytes() {
                    return Ok(Some(stream));
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => return Err("SUPERVISOR_LISTENER_FAILED".into()),
        }
        if !warned && started.elapsed() >= STARTUP_WARNING {
            warned = true;
            // No forced termination while startup/workspace migration has
            // not reached a live UI callback.
            eprintln!("[native-supervisor] startup probe not ready; recovery not armed");
        }
        thread::sleep(POLL);
    }
}
// A recovery decision is provisional until the actual child acquires
// the same exit-admission latch used by graceful shutdown. Hold the
// connection until retirement so the child does not relinquish that claim
// merely because the parent returned from its monitoring loop.
struct RecoveryClaim<'a> {
    started: &'a AtomicBool,
}
impl<'a> RecoveryClaim<'a> {
    fn acquire(state: &'a crate::ExitLifecycleState) -> Option<Self> {
        // Do not block the probe on the shutdown worker mutex. A rejected
        // but still-live shutdown worker retains ownership too.
        let worker = state.worker.try_lock().ok()?;
        if state.completed() || worker.as_ref().is_some_and(|worker| !worker.is_finished()) {
            return None;
        }
        state
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        Some(Self {
            started: &state.started,
        })
    }
}
impl Drop for RecoveryClaim<'_> {
    fn drop(&mut self) {
        self.started.store(false, Ordering::Release);
    }
}
fn exchange(stream: &mut TcpStream, command: u8) -> Option<u8> {
    let started = Instant::now();
    stream.write_all(&[command]).ok()?;
    loop {
        if started.elapsed() >= REPLY_LIMIT {
            return None;
        }
        let mut reply = [0u8; 1];
        match stream.read_exact(&mut reply) {
            Ok(()) if started.elapsed() < REPLY_LIMIT => return Some(reply[0]),
            Ok(()) => return None,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return None,
        }
    }
}
fn monitor(owner: &mut OwnedHost, stream: &mut TcpStream) -> Result<Outcome, String> {
    let mut health = ProbeHealth::default();
    loop {
        if let Some(status) = owner.exited()? {
            return Ok(Outcome::Exited(status));
        }
        let Some(reply) = exchange(stream, b'P') else {
            return Ok(Outcome::ProbeUnavailable);
        };
        match health.observe(reply, Instant::now()) {
            ProbeDecision::Recover => {
                // The last B is not authorization to kill. Ask the live
                // transport owner for a fresh exclusive recovery claim.
                match exchange(stream, b'C') {
                    Some(b'G') => {
                        if let Some(status) = owner.exited()? {
                            return Ok(Outcome::Exited(status));
                        }
                        return Ok(Outcome::Unresponsive);
                    }
                    Some(b'D') => health = ProbeHealth::default(),
                    _ => return Ok(Outcome::ProbeUnavailable),
                }
            }
            ProbeDecision::NoRecovery => return Ok(Outcome::ProbeUnavailable),
            ProbeDecision::Wait => {}
        }
        thread::sleep(PULSE);
    }
}
fn launch(args: Vec<OsString>, nested_memory: bool) -> Result<i32, String> {
    // No loop of repeated recovery/relaunch. A normal single-instance
    // forwarding child exit is an ordinary exit, never a reason to restart.
    for attempt in 0..=1 {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| "SUPERVISOR_LISTENER_FAILED")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "SUPERVISOR_LISTENER_FAILED")?;
        let token = uuid::Uuid::new_v4().to_string();
        let mut owner = OwnedHost::spawn(
            if attempt == 0 { &args } else { &[] },
            &listener,
            &token,
            nested_memory,
        )?;
        let mut stream = accepted_stream(&listener, &mut owner, &token)
            .ok()
            .flatten();
        let outcome = match stream.as_mut() {
            Some(stream) => monitor(&mut owner, stream),
            None => Ok(match owner.exited() {
                Ok(Some(status)) => Outcome::Exited(status),
                _ => Outcome::ProbeUnavailable,
            }),
        };
        drop(listener);
        // A granted recovery connection stays open through retire_confirmed.
        // On an unavailable probe, close it before waiting without recovery.
        if !matches!(&outcome, Ok(Outcome::Unresponsive)) {
            drop(stream.take());
        }
        match outcome {
            Ok(Outcome::Exited(status)) => {
                owner.retire_confirmed(report_retirement_pending);
                return Ok(status.code().unwrap_or(1));
            }
            Ok(Outcome::Unresponsive) => {
                eprintln!(
                    "[native-supervisor] owned UI unresponsive; terminating its execution domain"
                );
                owner.retire_confirmed(report_retirement_pending);
                eprintln!("[native-supervisor] execution domain empty and child exit confirmed");
                // Do not replay original command-line URLs or requests.
                if attempt == 1 {
                    return Err("SUPERVISOR_RESTART_LIMIT".into());
                }
            }
            Ok(Outcome::ProbeUnavailable) | Err(_) => {
                // A lost control connection is not proof that the UI hung.
                // Retain the owned process, do not kill/restart on IPC loss.
                eprintln!("[native-supervisor] probe unavailable; automatic recovery disarmed");
                return Ok(owner.wait_without_recovery().code().unwrap_or(1));
            }
        }
    }
    Err("SUPERVISOR_RESTART_LIMIT".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LaunchMode {
    Direct,
    Child,
    UserScope,
    Supervised { nested_memory: bool },
}

fn ordinary_gui_args(args: &[OsString]) -> bool {
    args.iter().all(|arg| {
        arg.to_str().is_some_and(|arg| {
            url::Url::parse(arg).is_ok_and(|url| {
                matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
            })
        })
    })
}

#[cfg(target_os = "linux")]
fn scope_argv(args: &[OsString]) -> Result<Vec<OsString>, &'static str> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    if !ordinary_gui_args(args) {
        return Err("SUPERVISOR_SCOPE_ARGUMENT_INVALID");
    }
    // Scope-mode expansion defaults differ across systemd versions.
    // Transfer exact URL bytes using the existing base64 dependency.
    // The encoding is not secret protection; no payload enters unit name,
    // description, environment variable or BrowserOS diagnostics.
    args.iter()
        .map(|arg| {
            arg.to_str()
                .map(|text| OsString::from(URL_SAFE_NO_PAD.encode(text.as_bytes())))
                .ok_or("SUPERVISOR_SCOPE_ARGUMENT_INVALID")
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn decode_scope_argv(args: &[OsString]) -> Result<Vec<OsString>, &'static str> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let urls = args
        .iter()
        .map(|arg| {
            let encoded = arg.to_str().ok_or("SUPERVISOR_SCOPE_ARGUMENT_INVALID")?;
            let bytes = URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| "SUPERVISOR_SCOPE_ARGUMENT_INVALID")?;
            if URL_SAFE_NO_PAD.encode(&bytes) != encoded {
                return Err("SUPERVISOR_SCOPE_ARGUMENT_INVALID");
            }
            String::from_utf8(bytes)
                .map(OsString::from)
                .map_err(|_| "SUPERVISOR_SCOPE_ARGUMENT_INVALID")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !ordinary_gui_args(&urls)
        || urls
            .iter()
            .any(|arg| arg.to_str().is_some_and(|v| v.contains('\0')))
    {
        return Err("SUPERVISOR_SCOPE_ARGUMENT_INVALID");
    }
    Ok(urls)
}

#[cfg(target_os = "linux")]
fn enter_user_scope(runner: &std::path::Path, args: &[OsString]) -> Result<i32, &'static str> {
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe().map_err(|_| "SUPERVISOR_EXECUTABLE_UNAVAILABLE")?;
    // Installed binary has a fixed normal path. Reject potential command
    // argument expansion of custom executable paths on newer systemd.
    if !exe.to_str().is_some_and(|path| !path.contains('$')) {
        return Err("SUPERVISOR_SCOPE_EXECUTABLE_INVALID");
    }
    let id = uuid::Uuid::new_v4();
    let mut command = Command::new(runner);
    command
        .args([
            "--user",
            "--scope",
            "--quiet",
            "--no-ask-password",
            "--property=Delegate=yes",
            "--description=BrowserOS native recovery",
        ])
        .arg(format!(
            "--unit={}",
            crate::shared::process_scope::user_scope_name(id)
        ))
        .arg("--")
        .arg(exe)
        .arg(USER_SCOPE_FLAG)
        .arg(id.to_string())
        .args(scope_argv(args)?);
    // Replace this launcher rather than leave an unsupervised duplicate.
    // systemd-run scope mode joins the transient unit before exec of our
    // private bootstrap. Failure never enters another GUI launch path.
    let _error = command.exec();
    Err("SUPERVISOR_USER_SCOPE_EXEC_FAILED")
}

#[cfg(target_os = "linux")]
fn reenter_user_scope(args: &[OsString]) -> Result<i32, String> {
    let value = args
        .first()
        .and_then(|arg| arg.to_str())
        .ok_or("SUPERVISOR_SCOPE_ARGUMENT_INVALID")?;
    let id = uuid::Uuid::parse_str(value).map_err(|_| "SUPERVISOR_SCOPE_ARGUMENT_INVALID")?;
    if id.get_version_num() != 4 || id.to_string() != value {
        return Err("SUPERVISOR_SCOPE_ARGUMENT_INVALID".into());
    }
    let forwarded = decode_scope_argv(&args[1..])?;
    crate::shared::process_scope::bootstrap_user_scope(id)
        .map_err(|_| "SUPERVISOR_USER_SCOPE_DELEGATION_FAILED")?;
    launch(forwarded, true)
}

/// Keep role/flag classification pure so the release startup migration can
/// be tested without spawning a real GUI, modifying delegation or a Job.
fn choose_launch(
    args: Vec<OsString>,
    inherited: bool,
    release: bool,
    linux: bool,
    delegated: bool,
) -> Result<(LaunchMode, Vec<OsString>), &'static str> {
    let mut explicit = None;
    let mut forwarded = Vec::new();
    for arg in args {
        if arg == USER_SCOPE_FLAG {
            return Err("SUPERVISOR_SCOPE_ARGUMENT_INVALID");
        }
        let mode = if arg == FLAG {
            Some(LaunchMode::Supervised {
                nested_memory: false,
            })
        } else if arg == MEMORY_FLAG {
            Some(LaunchMode::Supervised {
                nested_memory: true,
            })
        } else if arg == DIRECT_FLAG {
            Some(LaunchMode::Direct)
        } else {
            None
        };
        if let Some(mode) = mode {
            if explicit.replace(mode).is_some() {
                return Err("SUPERVISOR_LAUNCH_MODE_CONFLICT");
            }
        } else {
            forwarded.push(arg);
        }
    }
    // The child must never wrap itself in a second supervisor, including
    // after the first recovery when its original GUI argv is intentionally empty.
    if inherited {
        if explicit.is_some() {
            return Err("SUPERVISOR_NESTED_LAUNCH_DENIED");
        }
        return Ok((LaunchMode::Child, forwarded));
    }
    if let Some(mode) = explicit {
        if mode
            == (LaunchMode::Supervised {
                nested_memory: true,
            })
            && !linux
        {
            return Err("SUPERVISOR_MEMORY_TOPOLOGY_LINUX_ONLY");
        }
        return Ok((mode, forwarded));
    }
    // Desktop/start-menu launch and the existing HTTP(S) open-URL flow.
    // Preserve diagnostic/special entry behavior rather than accidentally
    // spawning a supervisor around an unrelated command-line invocation.
    let mode = if !release || !ordinary_gui_args(&forwarded) {
        LaunchMode::Direct
    } else if !linux {
        LaunchMode::Supervised {
            nested_memory: false,
        }
    } else if delegated {
        LaunchMode::Supervised {
            nested_memory: true,
        }
    } else {
        // Ask the current user's service manager for a fresh scope only
        // on eligible release GUI launches. Never adopt the desktop cgroup.
        LaunchMode::UserScope
    };
    Ok((mode, forwarded))
}

pub(crate) struct ChildEndpoint {
    address: SocketAddr,
    token: String,
}
fn decode_child_endpoint(
    address: Option<OsString>,
    token: Option<OsString>,
) -> Option<ChildEndpoint> {
    let address: SocketAddr = address?.into_string().ok()?.parse().ok()?;
    let token = token?.into_string().ok()?;
    if !address.ip().is_loopback()
        || address.port() == 0
        || token.len() != 36
        || uuid::Uuid::parse_str(&token).ok()?.get_version_num() != 4
    {
        return None;
    }
    Some(ChildEndpoint { address, token })
}

pub(crate) fn launch_if_requested() -> Option<i32> {
    let address = std::env::var_os(ADDRESS);
    let token = std::env::var_os(TOKEN);
    let inherited = address.is_some() || token.is_some();
    if inherited && decode_child_endpoint(address, token).is_none() {
        eprintln!("[native-supervisor] SUPERVISOR_CHILD_ENDPOINT_INVALID");
        return Some(1);
    }
    let linux = cfg!(target_os = "linux");
    let delegated = linux && std::env::var_os("BROWSEROS_DELEGATED_CGROUP_ROOT").is_some();
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == USER_SCOPE_FLAG) {
        if inherited || delegated || !linux {
            eprintln!("[native-supervisor] SUPERVISOR_NESTED_LAUNCH_DENIED");
            return Some(1);
        }
        #[cfg(target_os = "linux")]
        return Some(match reenter_user_scope(&args[1..]) {
            Ok(code) => code,
            Err(code) => {
                eprintln!("[native-supervisor] {code}; no replacement launched");
                1
            }
        });
        #[cfg(not(target_os = "linux"))]
        return Some(1);
    }
    let (mode, args) =
        match choose_launch(args, inherited, !cfg!(debug_assertions), linux, delegated) {
            Ok(selected) => selected,
            Err(error) => {
                eprintln!("[native-supervisor] {error}");
                return Some(1);
            }
        };
    match mode {
        LaunchMode::Child => None,
        LaunchMode::Direct => {
            if !cfg!(debug_assertions) {
                eprintln!(
                    "[native-supervisor] direct launch; automatic native domain recovery not armed"
                );
            }
            None
        }
        LaunchMode::UserScope => {
            #[cfg(target_os = "linux")]
            {
                let Some(runner) = crate::shared::process_scope::user_scope_runner() else {
                    // Preflight did not dispatch anything. Non-systemd or
                    // unsupported Linux retains explicit direct behavior.
                    eprintln!("[native-supervisor] user scope unavailable; direct launch without recovery");
                    return None;
                };
                Some(match enter_user_scope(&runner, &args) {
                    Ok(code) => code,
                    Err(code) => {
                        eprintln!("[native-supervisor] {code}; no replacement launched");
                        1
                    }
                })
            }
            #[cfg(not(target_os = "linux"))]
            Some(1)
        }
        LaunchMode::Supervised { nested_memory } => Some(match launch(args, nested_memory) {
            Ok(code) => code,
            // Once strict supervision was selected, never launch an
            // uncontained replacement on assignment/delegation failure.
            Err(code) => {
                eprintln!("[native-supervisor] {code}; no replacement launched");
                1
            }
        }),
    }
}

pub(crate) fn take_child_endpoint() -> Option<ChildEndpoint> {
    // Consume before any descendants are created. No endpoint secret or
    // request/page data is emitted in logs or stored on disk.
    let address = std::env::var_os(ADDRESS);
    let token = std::env::var_os(TOKEN);
    std::env::remove_var(ADDRESS);
    std::env::remove_var(TOKEN);
    decode_child_endpoint(address, token)
}
pub(crate) fn start_child_probe(app: tauri::AppHandle, endpoint: ChildEndpoint) {
    // A single bounded dispatch worker keeps the heartbeat transport
    // independent of a potentially blocked native dispatcher.
    let (submit, requests) = mpsc::sync_channel::<mpsc::SyncSender<()>>(1);
    let dispatcher = app.clone();
    if thread::Builder::new()
        .name("browseros-supervisor-dispatch".into())
        .spawn(move || {
            for reply in requests {
                if dispatcher
                    .run_on_main_thread(move || {
                        let _ = reply.try_send(());
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .is_err()
    {
        return;
    }
    let _ = thread::Builder::new()
        .name("browseros-supervisor-probe".into())
        .spawn(move || {
            let Ok(mut stream) =
                TcpStream::connect_timeout(&endpoint.address, Duration::from_secs(3))
            else {
                return;
            };
            if stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .is_err()
                || stream.write_all(endpoint.token.as_bytes()).is_err()
            {
                return;
            }
            // One outstanding callback across requests, including shutdown.
            // A pre-shutdown acknowledgement cannot rearm recovery.
            let mut pending: Option<mpsc::Receiver<()>> = None;
            let mut discard_ack = false;
            let mut last_pending_reply = false;
            let mut last_manual_intent = None;
            let _connection = ProbeConnection;
            let exiting = || {
                app.try_state::<crate::ExitLifecycleState>()
                    .is_some_and(|state| state.started.load(Ordering::Acquire))
            };
            loop {
                let mut command = [0u8; 1];
                if stream.read_exact(&mut command).is_err() {
                    return;
                }
                if command[0] == b'C' {
                    let manual_intent = last_manual_intent.take();
                    if !last_pending_reply && manual_intent.is_none() {
                        return;
                    }
                    last_pending_reply = false;
                    let Some(state) = app.try_state::<crate::ExitLifecycleState>() else {
                        return;
                    };
                    let Some(claim) = RecoveryClaim::acquire(&state) else {
                        if stream.write_all(b"D").is_err() {
                            return;
                        }
                        continue;
                    };
                    if let Some(intent) = manual_intent {
                        // The exact target and intent must still be current
                        // after taking the same exit latch as normal shutdown.
                        if let Some(domain_claim) = crate::bridge::claim_domain_recovery(&intent) {
                            if stream.write_all(b"G").is_ok() {
                                let mut end = [0u8; 1];
                                let _ = stream.read_exact(&mut end);
                            }
                            drop(domain_claim);
                            drop(claim);
                            return;
                        }
                        drop(claim);
                        if stream.write_all(b"D").is_err() {
                            return;
                        }
                        continue;
                    }
                    // Recheck the outstanding native callback AFTER claiming
                    // exit admission. No new recovery may use an old B.
                    match pending.as_ref().map(|receive| receive.try_recv()) {
                        Some(Err(mpsc::TryRecvError::Empty)) => {
                            if stream.write_all(b"G").is_ok() {
                                // Parent holds this stream until actual retirement.
                                // On loss of the supervisor the RAII claim releases;
                                // this is not Linux hard-crash domain containment.
                                let mut end = [0u8; 1];
                                let _ = stream.read_exact(&mut end);
                            }
                            drop(claim);
                            return;
                        }
                        Some(Ok(())) => {
                            pending = None;
                            discard_ack = false;
                        }
                        _ => return,
                    }
                    drop(claim);
                    if stream.write_all(b"D").is_err() {
                        return;
                    }
                    continue;
                }
                if command[0] != b'P' {
                    return;
                }
                DOMAIN_RECOVERY_READY.store(true, Ordering::Release);
                last_pending_reply = false;
                last_manual_intent = None;
                if exiting() {
                    discard_ack |= pending.is_some();
                    if stream.write_all(b"S").is_err() {
                        return;
                    }
                    continue;
                }
                if let Some(intent) = crate::bridge::pending_domain_recovery() {
                    last_manual_intent = Some(intent);
                    if stream.write_all(b"M").is_err() {
                        return;
                    }
                    continue;
                }
                if pending.is_none() {
                    let (reply, receive) = mpsc::sync_channel(1);
                    if submit.try_send(reply).is_err() {
                        return;
                    }
                    pending = Some(receive);
                    discard_ack = false;
                }
                let reply = match pending.as_ref().expect("pending probe").recv_timeout(POLL) {
                    Ok(()) => {
                        pending = None;
                        let suspended = discard_ack || exiting();
                        discard_ack = false;
                        if suspended {
                            b'S'
                        } else {
                            b'R'
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if exiting() {
                            discard_ack = true;
                            b'S'
                        } else {
                            // Transport is alive; the native callback is pending.
                            b'B'
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                };
                last_pending_reply = reply == b'B';
                if stream.write_all(&[reply]).is_err() {
                    return;
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_launch_selects_platform_owner_without_wrapping_children() {
        let select = |args: &[&str], inherited, release, linux, delegated| {
            choose_launch(
                args.iter().map(|arg| OsString::from(*arg)).collect(),
                inherited,
                release,
                linux,
                delegated,
            )
        };
        assert_eq!(
            select(&[], false, true, false, false).unwrap().0,
            LaunchMode::Supervised {
                nested_memory: false
            }
        );
        assert_eq!(
            select(&[], false, true, true, true).unwrap().0,
            LaunchMode::Supervised {
                nested_memory: true
            }
        );
        assert_eq!(
            select(&[], false, true, true, false).unwrap().0,
            LaunchMode::UserScope
        );
        assert!(select(&[USER_SCOPE_FLAG], false, true, true, false).is_err());
        assert_eq!(
            select(&[], true, true, false, false).unwrap().0,
            LaunchMode::Child
        );
        assert_eq!(
            select(&[], true, true, true, true).unwrap().0,
            LaunchMode::Child
        );
        assert_eq!(
            select(&[], false, false, false, false).unwrap().0,
            LaunchMode::Direct
        );
        let (mode, forwarded) =
            select(&["https://example.com/path?q=1"], false, true, false, false).unwrap();
        assert_eq!(
            mode,
            LaunchMode::Supervised {
                nested_memory: false
            }
        );
        assert_eq!(
            forwarded,
            vec![OsString::from("https://example.com/path?q=1")]
        );
        assert_eq!(
            select(&["--diagnostic-fixture"], false, true, false, false)
                .unwrap()
                .0,
            LaunchMode::Direct
        );
        assert_eq!(
            select(&[DIRECT_FLAG], false, true, false, false).unwrap().0,
            LaunchMode::Direct
        );
        assert!(select(&[FLAG, DIRECT_FLAG], false, true, false, false).is_err());
        assert!(select(&[FLAG], true, true, false, false).is_err());
        assert!(select(&[MEMORY_FLAG], false, true, false, false).is_err());
        assert_eq!(
            select(&[FLAG], false, false, true, false).unwrap().0,
            LaunchMode::Supervised {
                nested_memory: false
            }
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn user_scope_forwarding_preserves_exact_urls_without_systemd_expansion() {
        let args = vec![
            OsString::from("https://example.com/?q=$HOME&value=%24"),
            OsString::from("https://example.com/你好"),
        ];
        let encoded = scope_argv(&args).unwrap();
        assert!(encoded.iter().all(|value| value
            .to_str()
            .unwrap()
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')));
        assert_eq!(decode_scope_argv(&encoded).unwrap(), args);
        assert!(scope_argv(&[OsString::from("--grid-child")]).is_err());
        assert!(decode_scope_argv(&[OsString::from("%%%")]).is_err());
        assert!(decode_scope_argv(&[OsString::from("LS1kaXJlY3Qtc3RhcnQ")]).is_err());
    }

    #[test]
    fn incomplete_or_nonlocal_child_endpoint_is_not_a_valid_role() {
        let token = OsString::from(uuid::Uuid::new_v4().to_string());
        assert!(
            decode_child_endpoint(Some("127.0.0.1:3421".into()), Some(token.clone())).is_some()
        );
        assert!(
            decode_child_endpoint(Some("192.0.2.1:3421".into()), Some(token.clone())).is_none()
        );
        assert!(decode_child_endpoint(None, Some(token)).is_none());
        assert!(decode_child_endpoint(Some("127.0.0.1:3421".into()), Some("bad".into())).is_none());
    }

    #[test]
    fn recovery_claim_and_graceful_shutdown_have_one_admission_owner() {
        let state = crate::ExitLifecycleState::default();
        let claim = RecoveryClaim::acquire(&state).expect("recovery claim");
        assert!(!state.claim());
        assert!(RecoveryClaim::acquire(&state).is_none());
        drop(claim);
        assert!(state.claim());
        assert!(RecoveryClaim::acquire(&state).is_none());
    }

    #[test]
    fn recovery_requires_continuous_pending_evidence_after_a_real_ui_ack() {
        let now = Instant::now();
        let mut health = ProbeHealth::default();
        assert_eq!(health.observe(b'B', now), ProbeDecision::Wait);
        assert_eq!(health.observe(b'B', now + HANG), ProbeDecision::Wait);
        let start = now + HANG;
        assert_eq!(health.observe(b'R', start), ProbeDecision::Wait);
        for seconds in 1..=60 {
            assert_eq!(
                health.observe(b'B', start + Duration::from_secs(seconds)),
                ProbeDecision::Wait
            );
        }
        assert_eq!(
            health.observe(b'B', start + Duration::from_secs(61)),
            ProbeDecision::Recover
        );
    }

    #[test]
    fn shutdown_and_gaps_cannot_be_counted_as_continuous_native_hang() {
        let now = Instant::now();
        let mut health = ProbeHealth::default();
        health.observe(b'R', now);
        health.observe(b'B', now);
        assert_eq!(health.observe(b'B', now + HANG), ProbeDecision::Wait);
        health.observe(b'S', now + HANG);
        for seconds in 61..=122 {
            assert_eq!(
                health.observe(b'B', now + Duration::from_secs(seconds)),
                ProbeDecision::Wait
            );
        }
        assert_eq!(
            health.observe(b'?', now + Duration::from_secs(123)),
            ProbeDecision::NoRecovery
        );
    }

    #[test]
    fn explicit_native_recovery_request_is_distinct_from_probe_silence() {
        let mut health = ProbeHealth::default();
        assert_eq!(health.observe(b'B', Instant::now()), ProbeDecision::Wait);
        assert_eq!(health.observe(b'M', Instant::now()), ProbeDecision::Recover);
        assert_eq!(
            health.observe(b'?', Instant::now()),
            ProbeDecision::NoRecovery
        );
    }

    #[test]
    fn successful_ui_ack_resets_the_pending_interval() {
        let now = Instant::now();
        let mut health = ProbeHealth::default();
        health.observe(b'R', now);
        for seconds in 0..60 {
            assert_eq!(
                health.observe(b'B', now + Duration::from_secs(seconds)),
                ProbeDecision::Wait
            );
        }
        assert_eq!(health.observe(b'R', now + HANG), ProbeDecision::Wait);
        assert_eq!(health.observe(b'B', now + HANG), ProbeDecision::Wait);
    }
}
