//! Opt-in external supervisor of the existing application fault domain.
//! No GTK/WebView lifetime or layout changes. This is NOT per-tab renderer
//! isolation. A recovery restarts the whole owned app and can lose volatile
//! state. Only the explicit --supervised-recovery launch opts into this.
//! No discovery/adoption of an existing PID, no automatic URL replay.
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command, ExitStatus},
    sync::{atomic::Ordering, mpsc},
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;

const FLAG: &str = "--supervised-recovery";
const ADDRESS: &str = "BROWSEROS_SUPERVISOR_ADDRESS";
const TOKEN: &str = "BROWSEROS_SUPERVISOR_TOKEN";
const POLL: Duration = Duration::from_millis(250);
const PULSE: Duration = Duration::from_secs(2);
const HANG: Duration = Duration::from_secs(60);
const CLEANUP: Duration = Duration::from_secs(15);
const STARTUP_WARNING: Duration = Duration::from_secs(120);

pub(crate) struct OwnedHost {
    child: Child,
    #[cfg(target_os = "linux")]
    scope: crate::shared::process_scope::ProcessScope,
    #[cfg(target_os = "windows")]
    job: crate::windows_process::JobObject,
}
impl OwnedHost {
    fn spawn(args: &[OsString], listener: &TcpListener, token: &str) -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|_| "SUPERVISOR_EXECUTABLE_UNAVAILABLE")?;
        let mut command = Command::new(exe);
        command.args(args).env(ADDRESS, listener.local_addr().map_err(|_| "SUPERVISOR_LISTENER_FAILED")?.to_string())
            .env(TOKEN, token);
        Self::spawn_command(command, None)
    }

    /// Shared owned process domain. An optional budget must be installed
    /// before any worker code can execute; no unbounded fallback.
    pub(crate) fn spawn_command(mut command: Command, memory_limit: Option<usize>) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            let mut scope = crate::shared::process_scope::ProcessScope::prepare()
                .map_err(|_| "SUPERVISOR_CGROUP_UNAVAILABLE")?
                .ok_or("SUPERVISOR_DELEGATED_CGROUP_REQUIRED")?;
            if let Some(bytes) = memory_limit {
                scope.configure_memory_limit(bytes).map_err(|_| "SUPERVISOR_MEMORY_BUDGET_UNAVAILABLE")?;
            }
            scope.attach(&mut command).map_err(|_| "SUPERVISOR_CGROUP_ATTACH_FAILED")?;
            scope.contain_nested_owners(&mut command);
            let child = command.spawn().map_err(|_| "SUPERVISOR_SPAWN_FAILED")?;
            scope.mark_spawned();
            Ok(Self { child, scope })
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(crate::windows_process::CREATE_SUSPENDED);
            let job = match memory_limit {
                Some(bytes) => crate::windows_process::JobObject::create_with_memory_limit(bytes)?,
                None => crate::windows_process::JobObject::create_for_spawn()?,
            };
            let child = command.spawn().map_err(|_| "SUPERVISOR_SPAWN_FAILED")?;
            let mut owner = Self { child, job };
            if owner.job.assign_suspended_pid(owner.child.id()).is_err() {
                // The process may not be in the job if assignment failed.
                // Only our own Child handle can be used in this error path.
                let _ = owner.child.kill();
                let _ = owner.retire();
                return Err("SUPERVISOR_JOB_ASSIGN_FAILED".into());
            }
            Ok(owner)
        }
    }

    pub(crate) fn exited(&mut self) -> Result<Option<ExitStatus>, String> {
        self.child.try_wait().map_err(|_| "SUPERVISOR_WAIT_FAILED".into())
    }

    /// A kill request is not a receipt. Both the kernel's domain accounting
    /// and the original Child wait must confirm before another launch.
    pub(crate) fn retire(&mut self) -> Result<ExitStatus, String> {
        let deadline = Instant::now() + CLEANUP;
        #[cfg(target_os = "linux")]
        self.scope.terminate_and_confirm(deadline).map_err(|_| "SUPERVISOR_SCOPE_EXIT_UNKNOWN")?;
        #[cfg(target_os = "windows")]
        {
            if !self.job.is_empty()? { self.job.terminate()?; }
            loop {
                if self.job.is_empty()? { break; }
                if Instant::now() >= deadline { return Err("SUPERVISOR_JOB_EXIT_UNKNOWN".into()); }
                thread::sleep(POLL);
            }
        }
        loop {
            if let Some(status) = self.exited()? { return Ok(status); }
            if Instant::now() >= deadline { return Err("SUPERVISOR_CHILD_EXIT_UNKNOWN".into()); }
            thread::sleep(POLL);
        }
    }

    fn wait_without_recovery(&mut self) -> Result<ExitStatus, String> {
        self.child.wait().map_err(|_| "SUPERVISOR_WAIT_FAILED")?;
        self.retire()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeDecision { Wait, Recover, NoRecovery }
fn decision(ready: bool, shutdown: bool, connected: bool, elapsed: Duration) -> ProbeDecision {
    if !connected { return ProbeDecision::NoRecovery; }
    if ready && !shutdown && elapsed >= HANG { ProbeDecision::Recover } else { ProbeDecision::Wait }
}

enum Outcome { Exited(ExitStatus), Unresponsive, ProbeUnavailable }

fn accepted_stream(listener: &TcpListener, owner: &mut OwnedHost, token: &str) -> Result<Option<TcpStream>, String> {
    let started = Instant::now();
    let mut warned = false;
    loop {
        if owner.exited()?.is_some() { return Ok(None); }
        match listener.accept() {
            Ok((mut stream, address)) if address.ip().is_loopback() => {
                if stream.set_read_timeout(Some(POLL)).is_err() || stream.set_write_timeout(Some(POLL)).is_err() { continue; }
                // One fixed-size token, no variable-length remote input.
                // Incomplete or unauthorized local connections are discarded.
                let mut hello = [0u8; 36];
                if stream.read_exact(&mut hello).is_ok() && hello.as_slice() == token.as_bytes() {
                    return Ok(Some(stream));
                }
            }
            Ok(_) => {},
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {},
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
fn monitor(owner: &mut OwnedHost, mut stream: TcpStream) -> Result<Outcome, String> {
    let mut ready = false;
    let mut shutdown = false;
    loop {
        if let Some(status) = owner.exited()? { return Ok(Outcome::Exited(status)); }
        if stream.write_all(b"P").is_err() { return Ok(Outcome::ProbeUnavailable); }
        let pending = Instant::now();
        loop {
            if let Some(status) = owner.exited()? { return Ok(Outcome::Exited(status)); }
            let mut reply = [0u8; 1];
            match stream.read_exact(&mut reply) {
                Ok(()) if reply[0] == b'R' => { ready = true; shutdown = false; break; }
                Ok(()) if reply[0] == b'S' => { shutdown = true; break; }
                Ok(()) => return Ok(Outcome::ProbeUnavailable),
                Err(error) if matches!(error.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) => {
                    if decision(ready, shutdown, true, pending.elapsed()) == ProbeDecision::Recover {
                        return Ok(Outcome::Unresponsive);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {},
                Err(_) => return Ok(Outcome::ProbeUnavailable),
            }
        }
        thread::sleep(PULSE);
    }
}
fn launch(args: Vec<OsString>) -> Result<i32, String> {
    // No loop of repeated recovery/relaunch. A normal single-instance
    // forwarding child exit is an ordinary exit, never a reason to restart.
    for attempt in 0..=1 {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| "SUPERVISOR_LISTENER_FAILED")?;
        listener.set_nonblocking(true).map_err(|_| "SUPERVISOR_LISTENER_FAILED")?;
        let token = uuid::Uuid::new_v4().to_string();
        let mut owner = OwnedHost::spawn(if attempt == 0 { &args } else { &[] }, &listener, &token)?;
        let outcome = match accepted_stream(&listener, &mut owner, &token) {
            Ok(Some(stream)) => monitor(&mut owner, stream),
            Ok(None) => owner.exited()?.map(Outcome::Exited).ok_or("SUPERVISOR_WAIT_FAILED".into()),
            Err(_) => Ok(Outcome::ProbeUnavailable),
        };
        drop(listener);
        match outcome {
            Ok(Outcome::Exited(status)) => {
                owner.retire()?;
                return Ok(status.code().unwrap_or(1));
            }
            Ok(Outcome::Unresponsive) => {
                eprintln!("[native-supervisor] owned UI unresponsive; terminating its execution domain");
                owner.retire()?;
                eprintln!("[native-supervisor] execution domain empty and child exit confirmed");
                // Do not replay original command-line URLs or requests.
                if attempt == 1 { return Err("SUPERVISOR_RESTART_LIMIT".into()); }
            }
            Ok(Outcome::ProbeUnavailable) | Err(_) => {
                // A lost control connection is not proof that the UI hung.
                // Retain the owned process, do not kill/restart on IPC loss.
                eprintln!("[native-supervisor] probe unavailable; automatic recovery disarmed");
                return Ok(owner.wait_without_recovery()?.code().unwrap_or(1));
            }
        }
    }
    Err("SUPERVISOR_RESTART_LIMIT".into())
}

pub(crate) fn launch_if_requested() -> Option<i32> {
    let mut requested = false;
    let args = std::env::args_os().skip(1).filter(|arg| {
        if arg == FLAG { requested = true; false } else { true }
    }).collect::<Vec<_>>();
    if !requested { return None; }
    // Never forward an inherited supervisor endpoint to a different owner.
    if std::env::var_os(TOKEN).is_some() || std::env::var_os(ADDRESS).is_some() {
        eprintln!("[native-supervisor] nested supervisor denied");
        return Some(1);
    }
    Some(match launch(args) {
        Ok(code) => code,
        Err(code) => { eprintln!("[native-supervisor] {code}; no replacement launched"); 1 }
    })
}

pub(crate) struct ChildEndpoint { address: SocketAddr, token: String }
pub(crate) fn take_child_endpoint() -> Option<ChildEndpoint> {
    // Consume before any descendants are created. No endpoint secret or
    // request/page data is emitted in logs or stored on disk.
    let address = std::env::var(ADDRESS).ok();
    let token = std::env::var(TOKEN).ok();
    std::env::remove_var(ADDRESS);
    std::env::remove_var(TOKEN);
    let address: SocketAddr = address?.parse().ok()?;
    let token = token?;
    if !address.ip().is_loopback() || address.port() == 0 || token.len() != 36
        || uuid::Uuid::parse_str(&token).ok()?.get_version_num() != 4 { return None; }
    Some(ChildEndpoint { address, token })
}
pub(crate) fn start_child_probe(app: tauri::AppHandle, endpoint: ChildEndpoint) {
    // A single bounded dispatch worker keeps the heartbeat transport
    // independent of a potentially blocked native dispatcher.
    let (submit, requests) = mpsc::sync_channel::<mpsc::SyncSender<()>>(1);
    let dispatcher = app.clone();
    if thread::Builder::new().name("browseros-supervisor-dispatch".into()).spawn(move || {
        for reply in requests {
            if dispatcher.run_on_main_thread(move || { let _ = reply.try_send(()); }).is_err() { break; }
        }
    }).is_err() { return; }
    let _ = thread::Builder::new().name("browseros-supervisor-probe".into()).spawn(move || {
        let Ok(mut stream) = TcpStream::connect_timeout(&endpoint.address, Duration::from_secs(3)) else { return; };
        if stream.set_write_timeout(Some(Duration::from_secs(2))).is_err()
            || stream.write_all(endpoint.token.as_bytes()).is_err() { return; }
        // Keep an outstanding dispatch across shutdown observations; never
        // accumulate a second probe while a previous callback is in flight.
        let mut pending: Option<mpsc::Receiver<()>> = None;
        loop {
            let mut command = [0u8; 1];
            if stream.read_exact(&mut command).is_err() || command[0] != b'P' { return; }
            loop {
                let exiting = app.try_state::<crate::ExitLifecycleState>()
                    .is_some_and(|state| state.started.load(Ordering::Acquire));
                if exiting {
                    if stream.write_all(b"S").is_err() { return; }
                    break;
                }
                if pending.is_none() {
                    let (reply, receive) = mpsc::sync_channel(1);
                    if submit.try_send(reply).is_err() { return; }
                    pending = Some(receive);
                }
                match pending.as_ref().expect("pending probe").recv_timeout(POLL) {
                    Ok(()) => {
                        pending = None;
                        // A shutdown may have started during the callback.
                        let exiting = app.try_state::<crate::ExitLifecycleState>()
                            .is_some_and(|state| state.started.load(Ordering::Acquire));
                        if stream.write_all(if exiting { b"S" } else { b"R" }).is_err() { return; }
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {},
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_requires_prior_ui_ack_and_live_probe_and_no_shutdown() {
        assert_eq!(decision(false, false, true, HANG), ProbeDecision::Wait);
        assert_eq!(decision(true, true, true, HANG), ProbeDecision::Wait);
        assert_eq!(decision(true, false, false, HANG), ProbeDecision::NoRecovery);
        assert_eq!(decision(true, false, true, HANG - Duration::from_secs(1)), ProbeDecision::Wait);
        assert_eq!(decision(true, false, true, HANG), ProbeDecision::Recover);
    }
}
