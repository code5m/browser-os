//! 宫格子进程工厂 + UDS 转发（Phase 1 骨架 → Phase 2 接通 IPC → Phase 3 崩溃自愈）。
//!
//! 形态②（每宫格独立 tauri::app 子进程，隔离最干净）：每个宫格 `grid-N` 由独立的
//! 子进程承载（argv `--grid-child N` 启动，见 main.rs::run_grid_child）。子进程崩溃
//! （SIGSEGV）只杀该子进程，主进程监控线程拿到退出码后自动重启并重放状态
//! （CreateTab url + UpdateRect + SetZoom），实现"一格崩不影响其它格 + 主窗 + 页签"。
//!
//! IPC：Unix Domain Socket，行分隔 JSON（grid_ipc.rs）。
//! - 主进程每 index 绑定一个 listener（首次 spawn 时），accept 循环线程常驻，
//!   子进程重启后重连同一 listener（无需重新 bind）。
//! - 每个连接一个 reader 线程：Response 按 seq 路由给挂起请求，Event 转发为
//!   主进程 app.emit（tab-navigated / new-tab-request / open-terminal 等）。
//! - socket 路径带主进程 pid，避免跨次运行撞名。

use std::collections::{HashMap, VecDeque};
use std::io::{BufReader, Read};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::grid_ipc::{GridCmd, IpcRect, Wire};
#[path = "grid_lifecycle.rs"]
mod lifecycle;
use lifecycle::{GridLifecycle, LatestVisibility, VisibilityMode};

/// 单个子进程的通信通道（跨重启复用：writer 随连接替换，seq 单调递增）。

pub struct ChildComms {
    writer: Mutex<Option<UnixStream>>,
    // A separate native handle cancels blocked reads/writes without waiting
    // for the writer mutex. No socket is dropped while an IPC worker owns it.
    control: Mutex<Option<UnixStream>>,
    closing: AtomicBool,
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<Option<String>, String>>>>,
    seq: AtomicU64,
    generation: AtomicU64,
    // Mirrored under owner_gate from authenticated navigation observations.
    navigation_revision: AtomicU64,
    native_document: Mutex<Option<crate::grid_ipc::NativeDocumentStamp>>,
    // Published only after the exact spawned Child has entered the owner map.
    expected_pid: AtomicU32,
    owner_gate: Mutex<()>,
    spawn_epoch: AtomicU64,
    ready_generation: AtomicU64,
    connected: AtomicBool,
    // One generation's uncertain or page-affecting command permanently
    // disqualifies automatic pristine-blank reclaim.
    blank_reclaiming: AtomicBool,
    blank_reclaim_blocked: AtomicBool,
    download_observation: Mutex<Option<(u64, crate::grid_ipc::DownloadObservation)>>,
}

impl ChildComms {
    fn new() -> Self {
        Self {
            writer: Mutex::new(None),
            control: Mutex::new(None),
            closing: AtomicBool::new(false),
            pending: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(1),
            generation: AtomicU64::new(0),
            navigation_revision: AtomicU64::new(0),
            native_document: Mutex::new(None),
            expected_pid: AtomicU32::new(0),
            owner_gate: Mutex::new(()),
            spawn_epoch: AtomicU64::new(0),
            ready_generation: AtomicU64::new(0),
            connected: AtomicBool::new(false),
            blank_reclaiming: AtomicBool::new(false),
            blank_reclaim_blocked: AtomicBool::new(false),
            download_observation: Mutex::new(None),
        }
    }

    /// Invalidate any old connection before a replacement process can be
    /// launched. Do not recycle request identities across process generations.
    fn prepare_spawn(&self) -> Result<(), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let check = || if self.closing.load(Ordering::SeqCst) {
            Err("GRID_CONNECTION_CLOSING".to_string())
        } else { Ok(()) };
        let mut writer = crate::grid_ipc::lock_renderer_writer(&self.writer, deadline, check)?;
        let _owner = crate::grid_ipc::lock_renderer_writer(&self.owner_gate, deadline, check)?;
        let generation = self.generation.load(Ordering::SeqCst).checked_add(1)
            .ok_or("GRID_CONNECTION_GENERATION_EXHAUSTED")?;
        let epoch = self.spawn_epoch.load(Ordering::SeqCst).checked_add(1)
            .ok_or("GRID_SPAWN_EPOCH_EXHAUSTED")?;
        self.navigation_revision.store(0, Ordering::SeqCst);
        *self.native_document.lock().unwrap() = None;
        self.blank_reclaiming.store(false, Ordering::SeqCst);
        self.blank_reclaim_blocked.store(false, Ordering::SeqCst);
        *self.download_observation.lock().unwrap() = None;
        self.expected_pid.store(0, Ordering::SeqCst);
        self.ready_generation.store(0, Ordering::SeqCst);
        self.connected.store(false, Ordering::SeqCst);
        if let Some(control) = self.control.lock().unwrap().take() {
            let _ = control.shutdown(std::net::Shutdown::Both);
        }
        *writer = None;
        self.fail_pending("GRID_REQUEST_OUTCOME_UNKNOWN: previous renderer retired");
        self.generation.store(generation, Ordering::SeqCst);
        self.spawn_epoch.store(epoch, Ordering::SeqCst);
        Ok(())
    }

    fn download_retirement(&self, index: u32, pid: u32) -> Option<serde_json::Value> {
        if !crate::grid_ipc::is_tab_renderer(index) { return None; }
        let (generation, observation) = (*self.download_observation.lock().unwrap())?;
        Some(serde_json::json!({
            "id": crate::grid_ipc::renderer_label(index), "phase": "owner_retired",
            "pid": pid, "spawn_epoch": self.spawn_epoch.load(Ordering::SeqCst).to_string(),
            "renderer_generation": generation.to_string(),
            "requested": observation.requested, "succeeded": observation.succeeded,
            "failed": observation.failed, "observed_unfinished": observation.observed_unfinished(),
            "outcome": "unknown"
        }))
    }

    fn block_blank_reclaim_for(&self, generation: u64) {
        let _owner = self.owner_gate.lock().unwrap();
        if self.generation.load(Ordering::SeqCst) == generation {
            self.blank_reclaim_blocked.store(true, Ordering::SeqCst);
        }
    }

    fn try_claim_blank_reclaim(self: &Arc<Self>, generation: u64) -> Option<BlankReclaimGuard> {
        // The writer excludes a send that passed its admission check but has
        // not yet entered the pipe. Never wait on a potentially stalled pipe.
        let _writer = self.writer.try_lock().ok()?;
        let _owner = self.owner_gate.try_lock().ok()?;
        if !self.is_current(generation)
            || self.blank_reclaim_blocked.load(Ordering::SeqCst)
            || self.blank_reclaiming.load(Ordering::SeqCst)
            || !self.pending.try_lock().ok()?.is_empty()
        {
            return None;
        }
        self.blank_reclaiming.store(true, Ordering::SeqCst);
        Some(BlankReclaimGuard { comms: Arc::clone(self), generation })
    }

    fn publish_owner(&self, pid: u32) {
        let _owner = self.owner_gate.lock().unwrap();
        self.expected_pid.store(pid, Ordering::SeqCst);
    }

    /// Retirement admission revokes the old route before OS cleanup.
    /// Never acquire the potentially blocked writer here.
    fn revoke_owner(&self, pid: u32) {
        let _owner = self.owner_gate.lock().unwrap();
        if self.expected_pid.load(Ordering::SeqCst) != pid {
            return;
        }
        self.expected_pid.store(0, Ordering::SeqCst);
        self.ready_generation.store(0, Ordering::SeqCst);
        self.connected.store(false, Ordering::SeqCst);
        if let Some(control) = self.control.lock().unwrap().as_ref() {
            let _ = control.shutdown(std::net::Shutdown::Both);
        }
        self.fail_pending("GRID_REQUEST_OUTCOME_UNKNOWN: renderer retirement admitted");
    }

    fn owner_snapshot(&self) -> Result<(u32, u64), String> {
        let _owner = self.owner_gate.lock().unwrap();
        let pid = self.expected_pid.load(Ordering::SeqCst);
        if !self.accepts_peer(pid) {
            return Err("GRID_RENDERER_NOT_OWNED".into());
        }
        Ok((pid, self.spawn_epoch.load(Ordering::SeqCst)))
    }

    fn ready_for_owner(&self, expected: (u32, u64)) -> Result<Option<u64>, String> {
        let _owner = self.owner_gate.lock().unwrap();
        if !self.accepts_peer(expected.0) || self.spawn_epoch.load(Ordering::SeqCst) != expected.1 {
            return Err("GRID_RENDERER_OWNER_CHANGED".into());
        }
        Ok(self.is_connected().then(|| self.generation.load(Ordering::SeqCst)))
    }

    fn mark_ready(&self, generation: u64) -> bool {
        let _owner = self.owner_gate.lock().unwrap();
        if !self.is_transport_current(generation) || self.ready_generation.load(Ordering::SeqCst) != 0 {
            return false;
        }
        self.ready_generation.store(generation, Ordering::SeqCst);
        true
    }

    fn complete_response(&self, generation: u64, seq: u64, result: Result<Option<String>, String>) {
        let _owner = self.owner_gate.lock().unwrap();
        if self.is_current(generation) {
            if let Some(tx) = self.pending.lock().unwrap().remove(&seq) {
                let _ = tx.send(result);
            }
        }
    }

    fn accepts_peer(&self, pid: u32) -> bool {
        pid != 0 && self.expected_pid.load(Ordering::SeqCst) == pid
            && !self.closing.load(Ordering::SeqCst)
    }

    fn attach(&self, stream: UnixStream, peer_pid: u32) -> Result<u64, String> {
        let control = stream
            .try_clone()
            .map_err(|_| "GRID_CONTROL_CLONE_FAILED")?;
        // Never retain control while waiting for an active writer. Shutdown
        // needs only the short control lock to interrupt that writer.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let check = || if self.accepts_peer(peer_pid) { Ok(()) }
            else { Err("GRID_PEER_NOT_OWNED".to_string()) };
        let mut writer = crate::grid_ipc::lock_renderer_writer(&self.writer, deadline, check)?;
        let _owner = crate::grid_ipc::lock_renderer_writer(&self.owner_gate, deadline, check)?;
        let mut slot = self.control.lock().unwrap();
        if !self.accepts_peer(peer_pid) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return Err("GRID_PEER_NOT_OWNED".into());
        }
        let generation = self.generation.load(Ordering::SeqCst).checked_add(1)
            .ok_or("GRID_CONNECTION_GENERATION_EXHAUSTED")?;
        if let Some(old) = slot.replace(control) {
            let _ = old.shutdown(std::net::Shutdown::Both);
        }
        self.generation.store(generation, Ordering::SeqCst);
        *writer = Some(stream);
        self.ready_generation.store(0, Ordering::SeqCst);
        self.connected.store(true, Ordering::SeqCst);
        drop(slot);
        self.fail_pending("GRID_REQUEST_OUTCOME_UNKNOWN: renderer connection replaced");
        Ok(generation)
    }

    fn request_shutdown(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.connected.store(false, Ordering::SeqCst);
        if let Some(control) = self.control.lock().unwrap().as_ref() {
            let _ = control.shutdown(std::net::Shutdown::Both);
        }
        if let Ok(mut writer) = self.writer.try_lock() {
            writer.take();
        }
        if let Ok(mut pending) = self.pending.try_lock() {
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err("GRID_REQUEST_OUTCOME_UNKNOWN: connection shutdown after admission".into()));
            }
        }
    }

    fn fail_pending(&self, reason: &str) {
        let mut pending = self.pending.lock().unwrap();
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err(reason.to_owned()));
        }
    }

    fn detach_if_current(&self, generation: u64) {
        // Revoke first using the independent native cancellation handle.
        // Do not wait for a writer that may itself need this cancellation.
        {
            let _owner = self.owner_gate.lock().unwrap();
            if self.generation.load(Ordering::SeqCst) != generation { return; }
            self.ready_generation.store(0, Ordering::SeqCst);
            self.connected.store(false, Ordering::SeqCst);
            if let Some(control) = self.control.lock().unwrap().take() {
                let _ = control.shutdown(std::net::Shutdown::Both);
            }
            self.fail_pending("GRID_REQUEST_OUTCOME_UNKNOWN: renderer connection closed");
        }
        // A new attach owns the writer and increments generation. Never
        // clear that replacement stream as part of old detach cleanup.
        if let Ok(mut writer) = self.writer.try_lock() {
            let _owner = self.owner_gate.lock().unwrap();
            if self.generation.load(Ordering::SeqCst) == generation {
                *writer = None;
            }
        }
    }

    fn is_transport_current(&self, generation: u64) -> bool {
        !self.closing.load(Ordering::SeqCst)
            && self.expected_pid.load(Ordering::SeqCst) != 0
            && self.connected.load(Ordering::SeqCst)
            && self.generation.load(Ordering::SeqCst) == generation
    }

    fn is_current(&self, generation: u64) -> bool {
        self.is_transport_current(generation)
            && generation != 0
            && self.ready_generation.load(Ordering::SeqCst) == generation
    }

    fn is_connected(&self) -> bool {
        self.is_current(self.generation.load(Ordering::SeqCst))
    }
}
/// 崩溃重启时跨 spawn 保留的状态。
type SavedChildState = crate::grid_ipc::GridSavedState;

/// Observe an owned child without consuming its wait status. Keeping the
/// leader waitable pins its PID until the entire original process group is
/// gone, so a later negative-PGID kill cannot target a reused identifier.
/// https://man7.org/linux/man-pages/man2/waitid.2.html
fn peek_grid_exit(child: &Child) -> std::io::Result<Option<i32>> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            child.id() as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let observed = unsafe { info.si_pid() };
    if observed == 0 {
        return Ok(None);
    }
    if observed != child.id() as libc::pid_t {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "GRID_WAIT_IDENTITY_CHANGED",
        ));
    }
    let status = unsafe { info.si_status() };
    match info.si_code {
        libc::CLD_EXITED => Ok(Some(status)),
        libc::CLD_KILLED | libc::CLD_DUMPED => Ok(Some(128 + status)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "GRID_WAIT_STATE_UNKNOWN",
        )),
    }
}

fn proc_stat_group(stat: &[u8]) -> Option<i32> {
    // comm may contain non-UTF-8 bytes, spaces and parentheses. Parse only
    // ASCII fields after its final ')': state, ppid, then pgrp.
    let end = stat.iter().rposition(|byte| *byte == b')')?;
    std::str::from_utf8(&stat[end + 1..])
        .ok()?
        .split_whitespace()
        .nth(2)?
        .parse()
        .ok()
}

fn group_has_other_members(leader: i32) -> Result<bool, String> {
    let started = std::time::Instant::now();
    let mut entries = 0usize;
    for entry in std::fs::read_dir("/proc").map_err(|_| "GRID_PROC_GROUP_UNAVAILABLE")? {
        if entries >= 65_536 || started.elapsed() >= std::time::Duration::from_millis(100) {
            return Err("GRID_PROC_GROUP_BUDGET".into());
        }
        entries += 1;
        let entry = entry.map_err(|_| "GRID_PROC_GROUP_ENUMERATE")?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        if pid <= 0 || pid == leader {
            continue;
        }
        let file = match std::fs::File::open(entry.path().join("stat")) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("GRID_PROC_GROUP_READ".into()),
        };
        let mut bytes = Vec::new();
        match file.take(4097).read_to_end(&mut bytes) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("GRID_PROC_GROUP_READ".into()),
        }
        if bytes.len() > 4096 {
            return Err("GRID_PROC_GROUP_STAT_LIMIT".into());
        }
        let group = proc_stat_group(&bytes).ok_or("GRID_PROC_GROUP_STAT")?;
        if group == leader {
            return Ok(true);
        }
    }
    Ok(false)
}

/// An owned retirement remains queued on every incomplete/unknown result.
/// No detached waitpid thread and no PID-only orphan cleanup is created.
struct GridRetirement {
    index: u32,
    download_retired: Option<serde_json::Value>,
    child: Option<GridChildHandle>,
    identity_lost: bool,
    // Once reaped, numeric PGID observations are read-only forever. Never
    // signal a potentially reused PID/group when a late member is observed.
    identity_consumed: bool,
}
impl GridRetirement {
    fn new(index: u32, child: GridChildHandle) -> Self {
        let download_retired = child.comms.download_retirement(index, child.child.id());
        Self {
            index,
            download_retired,
            child: Some(child),
            identity_lost: false,
            identity_consumed: false,
        }
    }
    fn index(&self) -> u32 {
        self.index
    }

    fn complete_after_reap(&mut self, pgid: i32) -> Result<bool, String> {
        // /proc enumeration is not atomic. Close its last observation window
        // after reaping with a read-only kernel group lookup. A live/unknown
        // result retains the owner, but cannot authorize any further signals.
        if unsafe { libc::kill(-pgid, 0) } == 0 {
            return Ok(false);
        }
        match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ESRCH) => {
                self.child.take();
                Ok(true)
            }
            _ => Err("GRID_REAPED_GROUP_OBSERVATION_PENDING".into()),
        }
    }

    fn poll(&mut self) -> Result<bool, String> {
        if self.identity_lost {
            return Err("GRID_CHILD_IDENTITY_LOST".into());
        }
        let Some(owner) = self.child.as_mut() else {
            return Ok(true);
        };
        if let Some(scope) = owner.memory_scope.as_mut() {
            // The pinned cgroup, not a recycled numeric PGID, owns every
            // accounted descendant. Retain the exact owner on uncertainty.
            scope.observe_orphan_guard();
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
            match scope.terminate_and_confirm(deadline) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => return Ok(false),
                Err(_) => return Err("TAB_RENDERER_SCOPE_EXIT_UNKNOWN".into()),
            }
            match owner.child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => return Ok(false),
                Err(_) => return Err("TAB_RENDERER_CHILD_EXIT_UNKNOWN".into()),
            }
            scope.finish_orphan_guard(std::time::Instant::now() + std::time::Duration::from_millis(100))
                .map_err(|_| "TAB_RENDERER_GUARD_EXIT_UNKNOWN")?;
            self.child.take();
            return Ok(true);
        }
        let pid = i32::try_from(owner.child.id()).map_err(|_| "GRID_CHILD_PID_RANGE")?;
        if pid <= 0 {
            return Err("GRID_CHILD_PID_RANGE".into());
        }
        if self.identity_consumed {
            return self.complete_after_reap(pid);
        }
        match peek_grid_exit(&owner.child) {
            Ok(_) => {}
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.identity_lost = true;
                }
                return Err("GRID_CHILD_WAIT_STATE_UNKNOWN".into());
            }
        }
        // waitid above did not reap. The owned live/waitable leader pins this
        // numeric identity throughout both signals and the group observation.
        for target in [-pid, pid] {
            if unsafe { libc::kill(target, libc::SIGKILL) } != 0
                && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
            {
                return Err("GRID_PROCESS_GROUP_TERMINATE_FAILED".into());
            }
        }
        match peek_grid_exit(&owner.child) {
            Ok(None) => return Ok(false),
            Ok(Some(_)) => {}
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.identity_lost = true;
                }
                return Err("GRID_CHILD_WAIT_STATE_UNKNOWN".into());
            }
        }
        if group_has_other_members(pid)? {
            return Ok(false);
        }
        // Reap only after all other group members (including zombies) are
        // absent. Once this consumes the identity, no further signal is sent.
        match owner.child.try_wait() {
            Ok(Some(_)) => {
                self.identity_consumed = true;
                self.complete_after_reap(pid)
            }
            Ok(None) => Ok(false),
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.identity_lost = true;
                }
                Err("GRID_CHILD_REAP_PENDING".into())
            }
        }
    }
}
/// A Grid process is an independent browser session. URL + rectangle do not
/// preserve unsent AI prompts, streaming replies or SPA memory. Without a
/// documented snapshot/restore protocol, only empty cells may be discarded.
fn grid_content_reclaimable(last_url: Option<&str>) -> bool {
    matches!(last_url, Some("about:blank"))
}

fn grid_hibernation_probe_safe(encoded: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(encoded) else {
        return false;
    };
    value.get("safe").and_then(serde_json::Value::as_bool) == Some(true)
        && value.get("url").and_then(serde_json::Value::as_str) == Some("about:blank")
}

fn grid_child_is_hibernatable(
    hidden: bool,
    blur_hidden: bool,
    focused: bool,
    hidden_since: Option<std::time::Instant>,
    now: std::time::Instant,
    idle_for: std::time::Duration,
) -> bool {
    hidden
        && !blur_hidden
        && !focused
        && hidden_since.is_some_and(|since| now.duration_since(since) >= idle_for)
}

struct BlankReclaimGuard {
    comms: Arc<ChildComms>,
    generation: u64,
}
impl Drop for BlankReclaimGuard {
    fn drop(&mut self) {
        let _owner = self.comms.owner_gate.lock().unwrap();
        if self.comms.generation.load(Ordering::SeqCst) == self.generation {
            self.comms.blank_reclaiming.store(false, Ordering::SeqCst);
        }
    }
}

pub struct GridChildHandle {
    // index is the key in GridProcessManager::children;
    // communication state is retained in GridProcessManager::listeners.
    pub child: Child,
    // Ordinary tabs require an independently limited, recursively killable
    // scope. Legacy Grid retains its existing process-group retirement.
    memory_scope: Option<crate::shared::process_scope::ProcessScope>,
    comms: Arc<ChildComms>,
    focused: bool,
    spawned_at: std::time::Instant,
    document_ready: bool,
    navigation_revision: u64,
    /// 最近一次导航 url（崩溃重启重放用）
    pub last_url: Option<String>,
    /// 最近一次定位（CSS 坐标，相对主窗内容区；崩溃重启/主窗移动重放用）
    pub last_rect: Option<(f64, f64, f64, f64)>,
    /// 显式隐藏（切视图/全部隐藏）：定位/失焦恢复时不应重新显示
    pub hidden: bool,
    /// 主窗失焦导致的隐藏（主窗重新聚焦时恢复）
    pub blur_hidden: bool,
    // Last acknowledged native visibility for a specific connection.
    // Desired hidden/blur_hidden flags are not native success receipts.
    native_window_visibility: Option<(u64, bool)>,
    /// Explicitly hidden since; used by adaptive memory hibernation.
    hidden_since: Option<std::time::Instant>,
}

/// 宫格子进程管理器：登记所有存活子进程，负责 spawn/重启/关闭/UDS 转发/崩溃自愈。
#[derive(Default)]
pub struct GridProcessManager {
    children: Arc<Mutex<HashMap<u32, GridChildHandle>>>,
    lifecycle: Arc<GridLifecycle>,
    lifecycle_lock: Mutex<()>,
    /// Hibernated children retain URL/rect/visibility metadata but no WebKit process.
    hibernated: Mutex<HashMap<u32, SavedChildState>>,
    /// 已绑定 listener 的 index（accept 循环线程常驻，重启复用同一 socket）
    listeners: Arc<Mutex<HashMap<u32, Arc<ChildComms>>>>,
    app: Mutex<Option<AppHandle>>,
    monitor_started: AtomicBool,
    /// Crash budget per grid cell; normal user-triggered shutdown is not counted.
    crash_history: Mutex<HashMap<u32, VecDeque<std::time::Instant>>>,
    pending_restarts: Mutex<HashMap<u32, (std::time::Instant, SavedChildState)>>,
    grid_route_revisions: Mutex<[u64; crate::bridge::MAX_GRID]>,
    // One coalescing relocation worker, regardless of GTK move/resize rate.
    // The version is a latest-value notification, never an unbounded channel.
    reposition_pending: AtomicU64,
    reposition_running: AtomicBool,
    visibility: LatestVisibility,
    visibility_dispatch: Mutex<()>,
}

impl GridProcessManager {
    /// 主进程 setup 时注入 AppHandle（事件转发/重放/定位换算用）。
    pub fn set_app(&self, app: AppHandle) {
        self.lifecycle.set_retirement_app(app.clone());
        *self.app.lock().unwrap() = Some(app);
    }

    fn app(&self) -> Option<AppHandle> {
        self.app.lock().unwrap().clone()
    }

    fn spawn_worker(&self, name: &str, work: impl FnOnce() + Send + 'static) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        self.lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        self.lifecycle.spawn(name, work)
    }

    /// All focus/minimize sources publish to one latest-value sender. A
    /// delayed hide can never overtake a later show from another worker.
    pub fn request_hide_for_blur_async(&self) {
        self.request_visibility(VisibilityMode::Blur, std::time::Duration::from_millis(150));
    }

    pub fn request_show_for_focus_async(&self) {
        self.request_visibility(VisibilityMode::Show, std::time::Duration::ZERO);
    }

    fn request_visibility(&self, mode: VisibilityMode, delay: std::time::Duration) {
        if self.lifecycle.is_stopping() {
            return;
        }
        // A queued focus event may only address the hosts alive at publication.
        let targets = self.visibility_targets();
        if self.visibility.request(mode, delay, targets) {
            self.start_visibility_worker();
        }
    }

    fn retry_visibility_worker(&self) {
        if !self.lifecycle.is_stopping() && self.visibility.reserve_pending() {
            self.start_visibility_worker();
        }
    }

    fn start_visibility_worker(&self) {
        let Some(app) = self.app() else {
            self.visibility.spawn_failed();
            return;
        };
        let started = self.spawn_worker("grid-visibility", move || {
            let state = app.state::<crate::bridge::AppState>();
            let manager = &state.grid_manager;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                while let Some(intent) = manager.visibility.next(&manager.lifecycle) {
                    // Each target acquires the dispatch lock separately.
                    // A stalled window must not retain the lock for a whole batch.
                    if manager.visibility.is_latest(intent.revision) {
                        match intent.mode {
                            VisibilityMode::Show => manager.apply_show_for_focus(&intent),
                            VisibilityMode::Blur => manager.apply_hide_for_blur_if_no_child_focus(&intent),
                            VisibilityMode::Minimized => manager.apply_hide_for_blur(&intent),
                        }
                    }
                    if !manager.visibility.finish(intent.revision) {
                        break;
                    }
                }
            }));
            if let Err(panic) = outcome {
                // A panic must not strand the reservation. The monitor can
                // retry the retained latest intent, and the lifecycle owner
                // still observes the real thread failure when it joins.
                manager.visibility.spawn_failed();
                std::panic::resume_unwind(panic);
            }
        });
        if let Err(error) = started {
            self.visibility.spawn_failed();
            eprintln!("[grid-manager] visibility worker unavailable: {error}");
        }
    }

    fn socket_dir() -> PathBuf {
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".local/share")
            });
        base.join("com.jizhijiandan.mvp").join("sock")
    }

    fn socket_path(index: u32) -> PathBuf {
        Self::socket_dir().join(format!("grid-{}-{}.sock", std::process::id(), index))
    }

    /// 确保该 index 的 UDS listener 已绑定且 accept 循环线程在跑（幂等）。
    /// 返回该 index 的共享 comms。
    fn ensure_listener(&self, index: u32) -> Result<Arc<ChildComms>, String> {
        self.lifecycle.check()?;
        // 查找/绑定/登记必须原子化；并发 Grid 请求不能各自绑定同一 socket。
        let mut listeners = self.listeners.lock().unwrap();
        if let Some(c) = listeners.get(&index) {
            return Ok(c.clone());
        }
        if crate::grid_ipc::is_tab_renderer(index)
            && listeners.keys().filter(|id| crate::grid_ipc::is_tab_renderer(**id)).count()
                >= crate::grid_ipc::MAX_ISOLATED_TAB_RENDERERS
        {
            return Err("TAB_RENDERER_LIMIT".into());
        }
        let dir = Self::socket_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建 socket 目录失败: {e}"))?;
        let path = Self::socket_path(index);
        let _ = std::fs::remove_file(&path); // 清理残留
        let listener = UnixListener::bind(&path)
            .map_err(|e| format!("绑定 UDS {} 失败: {e}", path.display()))?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "GRID_LISTENER_NONBLOCKING")?;
        self.lifecycle.check()?;
        let comms = Arc::new(ChildComms::new());
        listeners.insert(index, comms.clone());
        drop(listeners);

        // accept 循环线程：子进程（含崩溃重启后的新实例）连接到同一 listener。
        let comms_accept = comms.clone();
        let app = self.app();
        let owner = Arc::clone(&self.lifecycle);
        let accept_owner = Arc::clone(&owner);
        let started = owner.spawn("grid-accept", move || {
            while !accept_owner.is_stopping() && !comms_accept.closing.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let Ok(peer_pid) = crate::grid_ipc::linux_peer_pid(&stream) else {
                            continue;
                        };
                        // Wait only for publication of the spawned Child, not
                        // for an arbitrary process claiming a renderer index.
                        let start = std::time::Instant::now();
                        while comms_accept.expected_pid.load(Ordering::SeqCst) == 0
                            && !accept_owner.is_stopping()
                            && start.elapsed() < std::time::Duration::from_millis(500)
                        {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        if !comms_accept.accepts_peer(peer_pid) {
                            eprintln!("[grid-manager] grid-child-{index} UDS peer refused");
                            continue;
                        }
                        eprintln!("[grid-manager] grid-child-{} UDS 已连接", index);
                        if stream.set_nonblocking(false).is_err() {
                            continue;
                        }
                        let reader = match stream.try_clone() {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!(
                                    "[grid-manager] grid-child-{} try_clone 失败: {e}",
                                    index
                                );
                                continue;
                            }
                        };
                        // 子进程不读时也必须限制 writer 阻塞时长，避免请求堆积。
                        if let Err(e) =
                            stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))
                        {
                            eprintln!(
                                "[grid-manager] grid-child-{} UDS 写超时配置失败: {e}",
                                index
                            );
                            continue;
                        }
                        let generation = match comms_accept.attach(stream, peer_pid) {
                            Ok(generation) => generation,
                            Err(_) => continue,
                        };
                        // 每连接一个 reader 线程：Response 按 seq 路由，Event 转发 app.emit。
                        let comms_reader = comms_accept.clone();
                        let app_reader = app.clone();
                        let reader_owner = Arc::clone(&accept_owner);
                        let started_reader = accept_owner.spawn("grid-reader", move || {
                            let mut r = BufReader::new(reader);
                            let ready = matches!(
                                crate::grid_ipc::read_wire(&mut r),
                                Ok(Some(frame)) if crate::grid_ipc::is_renderer_ready(index, &frame)
                            );
                            if !ready || !comms_reader.mark_ready(generation) {
                                comms_reader.detach_if_current(generation);
                                return;
                            }
                            loop {
                                if reader_owner.is_stopping()
                                    || !comms_reader.is_current(generation)
                                {
                                    break;
                                }
                                let frame = crate::grid_ipc::read_wire(&mut r);
                                // 旧连接仍有缓冲帧时，不允许覆盖新连接的响应/焦点状态。
                                if reader_owner.is_stopping()
                                    || !comms_reader.is_current(generation)
                                {
                                    break;
                                }
                                match frame {
                                    Ok(Some(Wire::Response { seq, ok, err, data })) => {
                                        comms_reader.complete_response(
                                            generation,
                                            seq,
                                            if ok { Ok(data) } else { Err(err.unwrap_or_else(|| "子进程执行失败".into())) },
                                        );
                                    }
                                    Ok(Some(Wire::Event { name, mut payload })) => {
                                        if let Some(app) = &app_reader {
                                            let state = app.state::<crate::bridge::AppState>();
                                            if !state.grid_manager.accept_renderer_event(index, &comms_reader, generation, &name, &mut payload) {
                                                break;
                                            }
                                            match name.as_str() {
                                                "grid-child-focus" => {}
                                                "grid-child-blur" => state.grid_manager.request_hide_for_blur_async(),
                                                "renderer-resource" | "tab-document-changed" | "native-document-revision" => {}
                                                "renderer-download" => {
                                                    if payload["renderer_drop"] != true && comms_reader.is_current(generation) {
                                                        let _ = app.emit_to("main", "isolated-renderer-download", payload);
                                                    }
                                                }
                                                "browser-tabs://shell-shortcut" => {
                                                    if payload["renderer_drop"] != true && comms_reader.is_current(generation) {
                                                        let _ = app.emit_to("main", "browser-tabs://shell-shortcut", payload);
                                                    }
                                                }
                                                "new-tab-request" => {
                                                    if payload["renderer_drop"] != true && comms_reader.is_current(generation) {
                                                        let _ = app.emit_to("main", "new-tab-request", payload);
                                                    }
                                                }
                                                "tab-title" | "browser-resources" if payload["renderer_drop"] == true => {}
                                                "tab-title" | "browser-resources" => {
                                                    if comms_reader.is_current(generation) {
                                                        let _ = app.emit(&name, payload);
                                                    }
                                                }
                                                "grid-load-failed" if crate::grid_ipc::is_tab_renderer(index) => {
                                                    let id = crate::grid_ipc::renderer_label(index);
                                                    let url = payload["url"].as_str().unwrap_or_default();
                                                    let error = payload["error"].as_str().unwrap_or("renderer load failed");
                                                    crate::bridge::report_tab_load_failed(app, &id, url, if error.is_empty() { "renderer load failed" } else { "native renderer load failed" });
                                                }
                                                _ if comms_reader.is_current(generation) => { let _ = app.emit(&name, payload); }
                                                _ => break,
                                            }
                                        }
                                    }
                                    Ok(Some(Wire::Request { .. })) => break,
                                    Ok(None) => {
                                        eprintln!(
                                            "[grid-manager] grid-child-{} UDS 对端关闭",
                                            index
                                        );
                                        break;
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "[grid-manager] grid-child-{} UDS 读错误: {e}",
                                            index
                                        );
                                        break;
                                    }
                                }
                            }
                            comms_reader.detach_if_current(generation);
                        });
                        if started_reader.is_err() {
                            comms_accept.detach_if_current(generation);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if accept_owner.wait_or_stop(std::time::Duration::from_millis(20)) {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[grid-manager] grid-child-{} accept 失败: {e}", index);
                        if accept_owner.wait_or_stop(std::time::Duration::from_millis(200)) {
                            break;
                        }
                    }
                }
            }
            comms_accept.request_shutdown();
        });
        if let Err(error) = started {
            comms.request_shutdown();
            self.listeners.lock().unwrap().remove(&index);
            return Err(error);
        }
        Ok(comms)
    }

    /// spawn 一个新的宫格子进程（不检查是否已存在，调用方需先确保没有重复）。
    fn spawn(&self, index: u32) -> Result<(), String> {
        self.spawn_with_state(index, None)
    }

    /// spawn 并可携带崩溃前保存的状态（崩溃重启时保留 last_url/last_rect，
    /// 否则 replay 读不到历史 url 会跳过重放）。

    fn spawn_with_state(&self, index: u32, saved: Option<SavedChildState>) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        self.lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        self.spawn_locked(index, saved)
    }

    /// Caller owns an admitted operation and lifecycle_lock. Native cleanup
    /// never waits for this mutex; late successful spawns transfer to retire.
    fn spawn_locked(&self, index: u32, saved: Option<SavedChildState>) -> Result<(), String> {
        self.lifecycle.check()?;
        if self.children.lock().unwrap().contains_key(&index) {
            return Err("GRID_ALREADY_OPEN".into());
        }
        if self.lifecycle.is_retiring(index) {
            return Err("GRID_PROCESS_CLEANUP_PENDING".into());
        }
        // This is the actual serialized native spawn admission, including
        // explicit restores; a UI capacity estimate cannot bypass it.
        if crate::grid_ipc::is_tab_renderer(index) {
            let recent = self.renderer_processes().iter()
                .filter(|owner| owner.startup_reserved).count();
            let available = crate::bridge::mem_available_mb()
                .ok_or("TAB_RENDERER_MEMORY_SAMPLE_UNAVAILABLE")?;
            if crate::bridge::renderer_start_budget(Some(available), recent) == 0 {
                return Err("TAB_RENDERER_MEMORY_BUDGET".into());
            }
        }
        let profile_epoch = crate::browser_profile_owner::grid_epoch()?;
        let comms = self.ensure_listener(index)?;
        comms.prepare_spawn()?;
        let exe = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        let mut command = Command::new(exe);
        command
            .arg("--grid-child")
            .arg(index.to_string())
            .env("GRID_CHILD_INDEX", index.to_string())
            .env("GRID_PARENT_PID", std::process::id().to_string())
            .env(crate::browser_profile_owner::GRID_EPOCH_ENV, profile_epoch)
            .env("GRID_SOCK_PATH", Self::socket_path(index))
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        // Prepare and read back the kernel limit before any renderer code
        // executes. An unavailable delegation must not fall back to PGID.
        let mut memory_scope = if crate::grid_ipc::is_tab_renderer(index) {
            let mut scope = crate::shared::process_scope::ProcessScope::prepare()
                .map_err(|_| "TAB_RENDERER_CGROUP_UNAVAILABLE")?
                .ok_or("TAB_RENDERER_DELEGATED_CGROUP_REQUIRED")?;
            scope.configure_memory_limit(crate::grid_ipc::TAB_RENDERER_MEMORY_LIMIT)
                .map_err(|_| "TAB_RENDERER_CGROUP_MEMORY_LIMIT_UNCONFIRMED")?;
            scope.arm_orphan_guard().map_err(|_| "TAB_RENDERER_CGROUP_GUARD_UNAVAILABLE")?;
            scope.attach(&mut command).map_err(|_| "TAB_RENDERER_CGROUP_ATTACH_FAILED")?;
            scope.contain_nested_owners(&mut command);
            Some(scope)
        } else { None };
        let child = command.spawn()
            .map_err(|e| format!("spawn renderer {index} failed: {e}"))?;
        if let Some(scope) = &mut memory_scope { scope.mark_spawned(); }
        let pid = child.id();
        let saved = saved.unwrap_or_default();
        let handle = GridChildHandle {
            child,
            memory_scope,
            comms: Arc::clone(&comms),
            focused: false,
            spawned_at: std::time::Instant::now(),
            document_ready: false,
            navigation_revision: 0,
            last_url: saved.last_url,
            last_rect: saved.last_rect,
            hidden: saved.hidden,
            blur_hidden: saved.blur_hidden,
            native_window_visibility: None,
            hidden_since: if saved.hidden {
                Some(std::time::Instant::now())
            } else {
                None
            },
        };
        {
            let mut children = self.children.lock().unwrap();
            // Same map lock as shutdown's drain: stop cannot observe an empty
            // registry and then miss publication from this admitted operation.
            if self.lifecycle.is_stopping() {
                self.lifecycle.retire(index, handle);
                return Err("GRID_SHUTDOWN_IN_PROGRESS".into());
            }
            children.insert(index, handle);
            comms.publish_owner(pid);
        }
        eprintln!("[grid-manager] spawned grid-child-{} pid={}", index, pid);
        self.start_monitor();
        Ok(())
    }

    /// 获取或 spawn：同一生命周期锁覆盖查找、恢复元数据与实际创建。
    pub fn get_or_spawn(&self, index: u32) -> Result<bool, String> {
        let _operation = self.lifecycle.enter()?;
        self.lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.get_or_spawn_locked(index)
    }

    // Caller has admitted the operation and holds lifecycle_lock.
    fn get_or_spawn_locked(&self, index: u32) -> Result<bool, String> {
        self.lifecycle.check()?;
        if self.children.lock().unwrap().contains_key(&index) {
            return Ok(false);
        }
        if self.lifecycle.is_retiring(index) {
            return Err("GRID_PROCESS_CLEANUP_PENDING".into());
        }
        let saved = self.hibernated.lock().unwrap().remove(&index).or_else(|| {
            self.pending_restarts
                .lock()
                .unwrap()
                .remove(&index)
                .map(|(_, saved)| saved)
        });
        if let Err(error) = self.spawn_locked(index, saved.clone()) {
            if let Some(saved) = saved {
                self.hibernated.lock().unwrap().insert(index, saved);
            }
            return Err(error);
        }
        Ok(true)
    }

    /// Caller holds the native lifecycle gate. Grid listeners persist across
    /// normal retirement, so an absent route also has a reuse fence.
    fn grid_route_stamps_locked(&self, indices: &[u32])
        -> Vec<crate::grid_ipc::GridRouteStamp>
    {
        let children = self.children.lock().unwrap();
        let listeners = self.listeners.lock().unwrap();
        let revisions = self.grid_route_revisions.lock().unwrap();
        indices.iter().map(|&index| {
            let child = children.get(&index);
            let comms = child.map(|c| &c.comms).or_else(|| listeners.get(&index));
            crate::grid_ipc::GridRouteStamp {
                index, pid: child.map(|c| c.child.id()),
                epoch: comms.map(|c| c.spawn_epoch.load(Ordering::SeqCst)),
                revision: revisions[index as usize],
            }
        }).collect()
    }

    pub(crate) fn grid_route_stamps(&self, indices: &[u32])
        -> Result<Vec<crate::grid_ipc::GridRouteStamp>, String>
    {
        if indices.iter().any(|&i| i as usize >= crate::bridge::MAX_GRID) {
            return Err("GRID_INDEX_INVALID".into());
        }
        self.lifecycle.check()?;
        // Admission is performed on the IPC caller, never wait on a
        // potentially long native spawn or retirement from that thread.
        let _gate = self.lifecycle_lock.try_lock().map_err(|_| "GRID_LIFECYCLE_BUSY")?;
        Ok(self.grid_route_stamps_locked(indices))
    }

    /// Validate every selected route before mutating any, then transfer only
    /// the captured owners to the existing kernel-domain cleanup service.
    /// This does not depend on HideWindow/CloseTab acknowledgements.
    pub(crate) fn close_grid_targets(&self, targets: &[crate::grid_ipc::GridRouteStamp])
        -> Result<(), String>
    {
        self.close_grid_targets_checked(targets, &[])
    }

    pub(crate) fn close_grid_targets_checked(&self, targets: &[crate::grid_ipc::GridRouteStamp],
        retained: &[crate::grid_ipc::GridRouteStamp]) -> Result<(), String>
    {
        let _operation = self.lifecycle.enter()?;
        self.lifecycle.ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        let indices: Vec<_> = targets.iter().map(|t| t.index).collect();
        let retained_indices: Vec<_> = retained.iter().map(|t| t.index).collect();
        let mut all = indices.clone();
        all.extend_from_slice(&retained_indices);
        all.sort_unstable();
        if all.iter().any(|&i| i as usize >= crate::bridge::MAX_GRID)
            || all.windows(2).any(|pair| pair[0] == pair[1])
            || self.grid_route_stamps_locked(&indices).as_slice() != targets
            || self.grid_route_stamps_locked(&retained_indices).as_slice() != retained
        {
            return Err("GRID_CLOSE_TARGET_STALE".into());
        }
        let app = self.app().ok_or("GRID_HOST_UNAVAILABLE")?;
        let state = app.state::<crate::bridge::AppState>();
        {
            let mut revisions = self.grid_route_revisions.lock().unwrap();
            if indices.iter().any(|&index| revisions[index as usize] == u64::MAX) {
                return Err("GRID_ROUTE_REVISION_EXHAUSTED".into());
            }
            for &index in &indices { revisions[index as usize] += 1; }
        }
        // Remove restart intent before releasing the process map. No new
        // owner can be spawned under this gate until cleanup is admitted.
        for &index in &indices {
            self.crash_history.lock().unwrap().remove(&index);
            self.hibernated.lock().unwrap().remove(&index);
            self.pending_restarts.lock().unwrap().remove(&index);
        }
        {
            let mut children = self.children.lock().unwrap();
            for &index in &indices {
                if let Some(owner) = children.remove(&index) {
                    self.lifecycle.retire(index, owner);
                }
            }
        }
        // Clear only selected route preferences, while reuse remains fenced.
        for &index in &indices {
            let label = crate::grid_ipc::renderer_label(index);
            state.child_layouts.lock().unwrap().remove(&label);
            state.grid_zooms.lock().unwrap().remove(&label);
            state.last_position_at.lock().unwrap().remove(&label);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
        for index in indices {
            self.lifecycle.wait_route_retired_until(index, deadline)
                .map_err(|_| format!("GRID_CLOSE_CLEANUP_PENDING:{index}"))?;
        }
        Ok(())
    }

    /// Rebuild or cold start may adopt only a slot captured at submission.
    /// Capture the newly created owner before releasing the spawn gate.
    pub(crate) fn prepare_grid_slot(&self, target: crate::grid_ipc::GridRouteStamp)
        -> Result<crate::grid_ipc::GridPreparedSlot, String>
    {
        let _operation = self.lifecycle.enter()?;
        self.lifecycle.ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        let index = target.index;
        if index as usize >= crate::bridge::MAX_GRID
            || self.grid_route_stamps_locked(&[index]).first() != Some(&target)
        {
            return Err("GRID_BUILD_TARGET_STALE".into());
        }
        if target.pid.is_some() && !self.children.lock().unwrap().get(&index)
            .is_some_and(|child| child.document_ready)
        {
            return Err("GRID_BUILD_TARGET_NOT_READY".into());
        }
        let saved = if target.pid.is_none() {
            self.hibernated.lock().unwrap().get(&index).cloned().or_else(|| {
                self.pending_restarts.lock().unwrap().get(&index).map(|(_, state)| state.clone())
            })
        } else { None };
        let created = self.get_or_spawn_locked(index)?;
        let stamp = self.grid_route_stamps_locked(&[index])[0];
        if stamp.pid.is_none() {
            return Err("GRID_BUILD_TARGET_UNAVAILABLE".into());
        }
        Ok(crate::grid_ipc::GridPreparedSlot {
            stamp, created, previous_url: self.last_url(index), saved,
        })
    }

    /// Wait only for the process admitted above, not whatever owns its route
    /// when the connection eventually becomes ready.
    pub(crate) fn grid_binding_for_stamp(&self, stamp: crate::grid_ipc::GridRouteStamp)
        -> Result<crate::grid_ipc::RendererBinding, String>
    {
        let expected = (stamp.pid.ok_or("GRID_BUILD_TARGET_UNAVAILABLE")?,
            stamp.epoch.ok_or("GRID_BUILD_TARGET_UNAVAILABLE")?);
        let comms = {
            let children = self.children.lock().unwrap();
            let child = children.get(&stamp.index).ok_or("GRID_BUILD_TARGET_STALE")?;
            if child.comms.owner_snapshot()? != expected {
                return Err("GRID_BUILD_TARGET_STALE".into());
            }
            Arc::clone(&child.comms)
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            self.lifecycle.check()?;
            if let Some(generation) = comms.ready_for_owner(expected)? {
                let binding = self.grid_eval_binding(stamp.index)?;
                if binding.owner != expected || binding.generation != generation {
                    return Err("GRID_BUILD_TARGET_STALE".into());
                }
                return Ok(binding);
            }
            if std::time::Instant::now() >= deadline {
                return Err("GRID_RENDERER_READY_TIMEOUT".into());
            }
            if self.lifecycle.wait_or_stop(std::time::Duration::from_millis(50)) {
                return Err("GRID_SHUTDOWN_IN_PROGRESS".into());
            }
        }
    }

    /// Compensate only an unacknowledged newly created slot. Restore the
    /// pre-spawn saved metadata (if any), never navigate retained pages back
    /// or select a replacement process. Confirmed earlier cells stay intact.
    pub(crate) fn rollback_grid_creation(&self, prepared: &crate::grid_ipc::GridPreparedSlot)
        -> Result<(), String>
    {
        if !prepared.created { return Err("GRID_ROLLBACK_NOT_CREATED".into()); }
        let _operation = self.lifecycle.enter()?;
        self.lifecycle.ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        let index = prepared.stamp.index;
        if index as usize >= crate::bridge::MAX_GRID
            || self.grid_route_stamps_locked(&[index]).first() != Some(&prepared.stamp)
        {
            return Err("GRID_ROLLBACK_TARGET_STALE".into());
        }
        self.pending_restarts.lock().unwrap().remove(&index);
        {
            let mut children = self.children.lock().unwrap();
            let child = children.remove(&index).ok_or("GRID_ROLLBACK_TARGET_STALE")?;
            if let Some(saved) = prepared.saved.clone() {
                self.hibernated.lock().unwrap().insert(index, saved);
            }
            // A missing live child must not be observable as fully retired
            // between removing it and registering the cleanup owner.
            self.lifecycle.retire(index, child);
        }
        self.lifecycle.wait_route_retired_until(index,
            std::time::Instant::now() + std::time::Duration::from_millis(750))
            .map_err(|_| format!("GRID_ROLLBACK_CLEANUP_PENDING:{index}"))
    }

    /// The process-generation binding is captured for prepared tab mutations.
    /// None means the route has no live child; activation may explicitly restore it.
    pub(crate) fn tab_renderer_owner(&self, index: u32) -> Result<Option<(u32, u64)>, String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let children = self.children.lock().unwrap();
        children.get(&index).map(|child| child.comms.owner_snapshot()).transpose()
    }

    /// Read-only UI observation, not authorization to retire or restore.
    /// Removal from the live map precedes kernel-domain retirement.
    pub(crate) fn tab_renderer_inspection(&self, index: u32)
        -> Result<(Option<(u32, u64)>, &'static str, Option<serde_json::Value>), String>
    {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let children = self.children.lock().unwrap();
        if self.lifecycle.is_retiring(index) { return Ok((None, "cleanup_pending", None)); }
        let Some(child) = children.get(&index) else { return Ok((None, "absent", None)); };
        let comms = &child.comms;
        let _owner = comms.owner_gate.lock().unwrap();
        let pid = child.child.id();
        let epoch = comms.spawn_epoch.load(Ordering::SeqCst);
        if pid == 0 || epoch == 0 || comms.expected_pid.load(Ordering::SeqCst) != pid {
            return Err("TAB_RENDERER_OBSERVATION_STALE".into());
        }
        let ready = child.document_ready && comms.is_connected()
            && !comms.blank_reclaiming.load(Ordering::SeqCst)
            && comms.native_document.lock().unwrap().is_some_and(|stamp| stamp.usable());
        let observed = *comms.download_observation.lock().unwrap();
        let download = observed.map(|(generation, counts)| serde_json::json!({
            "pid": pid, "spawn_epoch": epoch.to_string(),
            "renderer_generation": generation.to_string(),
            "connection_current": comms.is_current(generation),
            "requested": counts.requested, "succeeded": counts.succeeded,
            "failed": counts.failed, "observed_unfinished": counts.observed_unfinished(),
        }));
        Ok((Some((pid, epoch)), if ready { "ready" } else { "not_ready" }, download))
    }

    pub(crate) fn tab_renderer_observation(&self, index: u32)
        -> Result<(Option<(u32, u64)>, &'static str), String>
    {
        self.tab_renderer_inspection(index).map(|(owner, state, _)| (owner, state))
    }

    pub(crate) fn tab_renderer_download_observation(&self, index: u32, expected: (u32, u64))
        -> Result<Option<serde_json::Value>, String>
    {
        let (owner, _, download) = self.tab_renderer_inspection(index)?;
        if owner != Some(expected) { return Err("TAB_RENDERER_OBSERVATION_STALE".into()); }
        Ok(download)
    }

    pub(crate) fn renderer_binding(&self, index: u32) -> Result<crate::grid_ipc::RendererBinding, String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let children = self.children.lock().unwrap();
        let child = children.get(&index).ok_or("TAB_RENDERER_DOCUMENT_UNAVAILABLE")?;
        let owner = child.comms.owner_snapshot()?;
        let generation = child.comms.generation.load(Ordering::SeqCst);
        if !child.document_ready || !child.comms.is_current(generation)
            || !child.comms.native_document.lock().unwrap().is_some_and(|stamp| stamp.usable()) {
            return Err("TAB_RENDERER_DOCUMENT_UNAVAILABLE".into());
        }
        Ok(crate::grid_ipc::RendererBinding {
            index, owner, generation, navigation_revision: child.navigation_revision,
        })
    }

    /// Grid Eval is admitted against the currently connected host, before the
    /// caller enters a blocking lane. Grid host readiness is not a paint or
    /// document-creation receipt (the ordinary-tab flag has different semantics).
    pub(crate) fn grid_eval_binding(&self, index: u32) -> Result<crate::grid_ipc::RendererBinding, String> {
        self.grid_live_binding(index)?.ok_or_else(|| "GRID_EVAL_HOST_UNAVAILABLE".into())
    }

    pub(crate) fn grid_live_binding(&self, index: u32)
        -> Result<Option<crate::grid_ipc::RendererBinding>, String>
    {
        if (index as usize) >= crate::bridge::MAX_GRID {
            return Err("GRID_INDEX_INVALID".into());
        }
        self.lifecycle.check()?;
        let children = self.children.lock().unwrap();
        let Some(child) = children.get(&index) else { return Ok(None); };
        let owner = child.comms.owner_snapshot()?;
        let generation = child.comms.generation.load(Ordering::SeqCst);
        if !child.comms.is_current(generation) || child.comms.blank_reclaiming.load(Ordering::SeqCst) {
            return Err("GRID_EVAL_HOST_UNAVAILABLE".into());
        }
        Ok(Some(crate::grid_ipc::RendererBinding {
            index, owner, generation, navigation_revision: child.navigation_revision,
        }))
    }

    /// Only this exact original Grid host/connection may accept the queued
    /// Eval. No wait_connected, spawn, retry or arbitrary command substitution.
    pub(crate) fn grid_eval_bound(
        &self, binding: crate::grid_ipc::RendererBinding, js: String,
    ) -> Result<(), String> {
        let cmd = GridCmd::Eval { id: crate::grid_ipc::renderer_label(binding.index), js };
        self.grid_bound_request(binding, cmd, 10000).map(|_| ())
    }

    /// Only the reviewed Grid operations may use this bound path. Host
    /// preferences and geometry survive same-host navigation; data and a
    /// queued navigation intent are fenced by the observed document revision.
    fn grid_bound_request(
        &self, binding: crate::grid_ipc::RendererBinding, cmd: GridCmd, timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        let _operation = self.lifecycle.enter()?;
        if (binding.index as usize) >= crate::bridge::MAX_GRID || !cmd.targets(binding.index)
            || !matches!(&cmd, GridCmd::Eval { .. } | GridCmd::ReadReplies { .. }
                | GridCmd::SetZoom { .. } | GridCmd::Navigate { .. } | GridCmd::UpdateRect { .. }
                | GridCmd::CreateTab { .. })
        {
            return Err("GRID_REQUEST_TARGET_REJECTED".into());
        }
        let document_bound = !matches!(&cmd, GridCmd::SetZoom { .. } | GridCmd::UpdateRect { .. });
        let comms = {
            let children = self.children.lock().unwrap();
            let child = children.get(&binding.index).ok_or("GRID_EVAL_TARGET_STALE")?;
            let creating = matches!(&cmd, GridCmd::CreateTab { .. });
            if creating == child.document_ready {
                return Err("GRID_DOCUMENT_NOT_READY_OR_ALREADY_CREATED".into());
            }
            if child.comms.owner_snapshot()? != binding.owner
                || (document_bound && child.navigation_revision != binding.navigation_revision)
                || !child.comms.is_current(binding.generation)
                || child.comms.blank_reclaiming.load(Ordering::SeqCst)
            {
                return Err("GRID_EVAL_TARGET_STALE".into());
            }
            Arc::clone(&child.comms)
        };
        self.request_once_fenced(comms, binding.generation,
            document_bound.then_some(binding.navigation_revision), &cmd, timeout_ms)
    }

    /// A late acknowledgement must not rewrite the URL of a replacement
    /// host, nor overwrite a newer native navigation observation.
    pub(crate) fn grid_navigate_bound(&self, binding: crate::grid_ipc::RendererBinding, url: &str)
        -> Result<(), String>
    {
        let cmd = GridCmd::Navigate { id: crate::grid_ipc::renderer_label(binding.index), url: url.to_owned() };
        self.grid_bound_request(binding, cmd, 10000)?;
        self.grid_publish_address(binding, url, false)
    }

    pub(crate) fn grid_create_bound(&self, binding: crate::grid_ipc::RendererBinding, url: &str)
        -> Result<(), String>
    {
        let cmd = GridCmd::CreateTab { id: crate::grid_ipc::renderer_label(binding.index), url: url.to_owned() };
        self.grid_bound_request(binding, cmd, 20000)?;
        self.grid_publish_address(binding, url, true)
    }

    fn grid_publish_address(&self, binding: crate::grid_ipc::RendererBinding, url: &str, created: bool)
        -> Result<(), String>
    {
        let mut children = self.children.lock().unwrap();
        let child = children.get_mut(&binding.index).ok_or("GRID_REQUEST_OUTCOME_UNKNOWN: navigation host retired")?;
        let _owner = child.comms.owner_gate.lock().unwrap();
        if child.child.id() != binding.owner.0
            || child.comms.spawn_epoch.load(Ordering::SeqCst) != binding.owner.1
            || !child.comms.is_current(binding.generation)
        {
            return Err("GRID_REQUEST_OUTCOME_UNKNOWN: navigation host changed".into());
        }
        if child.navigation_revision == binding.navigation_revision {
            child.last_url = Some(url.to_owned());
        }
        if created { child.document_ready = true; }
        Ok(())
    }

    /// Serialize the existing geometry path with focus/visibility changes.
    /// Do not publish layout intent against a replacement route.
    pub(crate) fn grid_position_bound(&self, binding: crate::grid_ipc::RendererBinding,
        css: (f64, f64, f64, f64), rect: crate::grid_ipc::IpcRect,
    ) -> Result<(), String> {
        if binding.index as usize >= crate::bridge::MAX_GRID {
            return Err("GRID_INDEX_INVALID".into());
        }
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        self.position_window_bound(binding, css, rect)
    }

    pub(crate) fn publish_grid_zoom(&self, stamp: crate::grid_ipc::GridRouteStamp, zoom: f64)
        -> Result<(), String>
    {
        if stamp.index as usize >= crate::bridge::MAX_GRID || !zoom.is_finite() || zoom <= 0.1 {
            return Err("GRID_ZOOM_INVALID".into());
        }
        self.lifecycle.check()?;
        let _gate = self.lifecycle_lock.try_lock().map_err(|_| "GRID_LIFECYCLE_BUSY")?;
        if self.grid_route_stamps_locked(&[stamp.index]).first() != Some(&stamp)
            || (stamp.pid.is_none() && !self.is_hibernated(stamp.index))
        {
            return Err("GRID_ZOOM_TARGET_STALE".into());
        }
        let app = self.app().ok_or("GRID_HOST_UNAVAILABLE")?;
        app.state::<crate::bridge::AppState>().grid_zooms.lock().unwrap()
            .insert(crate::grid_ipc::renderer_label(stamp.index), zoom);
        Ok(())
    }

    pub(crate) fn grid_zoom_bound(&self, binding: crate::grid_ipc::RendererBinding, zoom: f64)
        -> Result<(), String>
    {
        if !zoom.is_finite() || zoom <= 0.1 {
            return Err("GRID_ZOOM_INVALID".into());
        }
        let cmd = GridCmd::SetZoom { id: crate::grid_ipc::renderer_label(binding.index), zoom };
        self.grid_bound_request(binding, cmd, 3000).map(|_| ())
    }

    pub(crate) fn grid_read_replies_bound(&self, binding: crate::grid_ipc::RendererBinding)
        -> Result<String, String>
    {
        let cmd = GridCmd::ReadReplies { id: crate::grid_ipc::renderer_label(binding.index) };
        let result = self.grid_bound_request(binding, cmd, 5000)?
            .ok_or("GRID_NO_RESULT")?;
        // An old document's late reply is not the current cell's content.
        if self.grid_eval_binding(binding.index)? != binding {
            return Err("GRID_READ_TARGET_STALE".into());
        }
        Ok(result)
    }

    /// No data service may reconnect or replay on a replacement child.
    pub(crate) fn renderer_request_data(
        &self, binding: crate::grid_ipc::RendererBinding, cmd: GridCmd, timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        let _operation = self.lifecycle.enter()?;
        if !cmd.targets(binding.index) || self.renderer_binding(binding.index)? != binding {
            return Err("TAB_RENDERER_DOCUMENT_STALE".into());
        }
        let comms = {
            let children = self.children.lock().unwrap();
            let child = children.get(&binding.index).ok_or("TAB_RENDERER_DOCUMENT_UNAVAILABLE")?;
            if child.comms.owner_snapshot()? != binding.owner || child.navigation_revision != binding.navigation_revision {
                return Err("TAB_RENDERER_DOCUMENT_STALE".into());
            }
            Arc::clone(&child.comms)
        };
        self.request_once_fenced(comms, binding.generation,
            Some(binding.navigation_revision), &cmd, timeout_ms)
    }

    /// Recover only the exact original Child/Job, without the potentially
    /// blocked visibility or writer locks. Keep the listener across recovery.
    pub(crate) fn retire_bound_tab_renderer(
        &self, index: u32, expected: (u32, u64),
    ) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) || expected.0 == 0 {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let _operation = self.lifecycle.enter()?;
        self.lifecycle.ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        {
            let mut children = self.children.lock().unwrap();
            if let Some(child) = children.get(&index) {
                if child.child.id() != expected.0
                    || child.comms.spawn_epoch.load(Ordering::SeqCst) != expected.1
                {
                    return Err("TAB_RENDERER_RECOVERY_STALE".into());
                }
                let child = children.remove(&index).ok_or("TAB_RENDERER_RECOVERY_STALE")?;
                self.lifecycle.retire(index, child);
            } else if self.listeners.lock().unwrap().get(&index).is_some_and(|comms|
                comms.spawn_epoch.load(Ordering::SeqCst) != expected.1) {
                return Err("TAB_RENDERER_RECOVERY_STALE".into());
            }
        }
        // Both capture and Browser fault recovery use this retirement owner.
        // Publish an advisory state change, not an exit code or a success
        // receipt. The UI must re-inspect and never replay an old operation.
        let notify = || {
            if let Some(app) = self.app() {
                let _ = app.emit_to("main", "isolated-tab-renderer-recovery-changed",
                    serde_json::json!({"id": crate::grid_ipc::renderer_label(index)}));
            }
        };
        notify();
        let result = self.lifecycle.wait_route_retired_until(
            index, std::time::Instant::now() + std::time::Duration::from_secs(5),
        );
        drop(_gate);
        notify();
        result
    }

    /// Background reclaim is intentionally limited to pristine blank ordinary
    /// pages. The caller owns the Browser mutation and tab-transition guards.
    pub(crate) fn hibernate_blank_tab_renderer(&self, index: u32, idle_for: std::time::Duration)
        -> Result<bool, String>
    {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let _operation = self.lifecycle.enter()?;
        let Ok(_visibility) = self.visibility_dispatch.try_lock() else { return Ok(false); };
        let app = self.app().ok_or("TAB_RENDERER_HOST_UNAVAILABLE")?;
        let state = app.state::<crate::bridge::AppState>();
        let id = crate::grid_ipc::renderer_label(index);
        let eligible = |child: &GridChildHandle| {
            child.document_ready
                && child.last_url.as_deref() == Some("about:blank")
                && !child.comms.blank_reclaim_blocked.load(Ordering::SeqCst)
                && grid_child_is_hibernatable(child.hidden, child.blur_hidden, child.focused,
                    child.hidden_since, std::time::Instant::now(), idle_for)
                && state.hibernation_enabled.load(Ordering::Acquire)
                && !state.shutdown_requested.load(Ordering::Acquire)
                && state.active_tab.lock().unwrap().as_deref() != Some(id.as_str())
        };
        let (comms, binding) = {
            let children = self.children.lock().unwrap();
            let Some(child) = children.get(&index) else { return Ok(false); };
            if !eligible(child) { return Ok(false); }
            let owner = child.comms.owner_snapshot()?;
            let generation = child.comms.generation.load(Ordering::SeqCst);
            (Arc::clone(&child.comms), crate::grid_ipc::RendererBinding {
                index, owner, generation, navigation_revision: child.navigation_revision,
            })
        };
        let Some(_claim) = comms.try_claim_blank_reclaim(binding.generation) else { return Ok(false); };
        // A hidden intent alone is insufficient. Confirm native hide before
        // probing; any layout or page mutation is excluded until retirement.
        self.renderer_request_data(binding, GridCmd::HideWindow { id: id.clone() }, 1500)?;
        let probe = self.renderer_request_data(binding, GridCmd::HibernationProbe { id: id.clone() }, 1500)?;
        if !probe.as_deref().is_some_and(grid_hibernation_probe_safe) { return Ok(false); }
        self.lifecycle.ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let Ok(_gate) = self.lifecycle_lock.try_lock() else { return Ok(false); };
        self.lifecycle.check()?;
        {
            let mut children = self.children.lock().unwrap();
            let Some(child) = children.get(&index) else { return Ok(false); };
            if !Arc::ptr_eq(&child.comms, &comms) || !comms.is_current(binding.generation)
                || child.navigation_revision != binding.navigation_revision || !eligible(child)
                || !state.tabs.lock().unwrap().get(&id).is_some_and(|tab| tab.url == "about:blank")
            {
                return Ok(false);
            }
            let owner = children.remove(&index).ok_or("TAB_RENDERER_RECOVERY_STALE")?;
            self.lifecycle.retire(index, owner);
        }
        self.lifecycle.wait_route_retired_until(
            index, std::time::Instant::now() + std::time::Duration::from_secs(3),
        )?;
        Ok(true)
    }

    pub(crate) fn start_tab_renderer(&self, index: u32, url: &str) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let (expected, result) = {
            let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
            if !self.get_or_spawn(index)? {
                return Err("TAB_RENDERER_ALREADY_RUNNING".into());
            }
            let (expected, original) = {
                let mut children = self.children.lock().unwrap();
                let child = children.get_mut(&index).ok_or("TAB_RENDERER_EXITED_BEFORE_CREATE_RECEIPT")?;
                child.hidden = true;
                child.hidden_since.get_or_insert_with(std::time::Instant::now);
                child.last_url = Some(url.to_owned());
                ((child.child.id(), child.comms.spawn_epoch.load(Ordering::SeqCst)), Arc::clone(&child.comms))
            };
            let result = (|| -> Result<(), String> {
                let (comms, generation) = self.wait_connected(index, 15000)?;
                if !Arc::ptr_eq(&original, &comms) || comms.owner_snapshot()? != expected {
                    return Err("TAB_RENDERER_CREATE_TARGET_STALE".into());
                }
                let command = GridCmd::CreateTab { id: crate::grid_ipc::renderer_label(index), url: url.to_owned() };
                self.request_once_with(Arc::clone(&comms), generation, &command, 20000)?;
                let mut children = self.children.lock().unwrap();
                let child = children.get_mut(&index).ok_or("GRID_REQUEST_OUTCOME_UNKNOWN: renderer retired during create")?;
                let _owner = comms.owner_gate.lock().unwrap();
                if !Arc::ptr_eq(&child.comms, &comms) || child.child.id() != expected.0
                    || comms.expected_pid.load(Ordering::SeqCst) != expected.0
                    || comms.spawn_epoch.load(Ordering::SeqCst) != expected.1
                    || !comms.is_current(generation) || child.document_ready {
                    return Err("GRID_REQUEST_OUTCOME_UNKNOWN: renderer create owner changed".into());
                }
                child.document_ready = true;
                Ok(())
            })();
            (expected, result)
        };
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                // Retain listener generation, and never close a newer host.
                match self.retire_bound_tab_renderer(index, expected) {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(format!("{error}; {cleanup}")),
                }
            }
        }
    }

    /// Retire only this route. Retain the same listener across an explicit
    /// recovery so process/connection generations cannot reset.
    pub(crate) fn close_tab_renderer(&self, index: u32, release_listener: bool) -> Result<(), String> {
        self.close_tab_renderer_admitted(index, release_listener, None)
    }

    pub(crate) fn close_tab_renderer_bound(&self, index: u32, release_listener: bool,
        expected: Option<(u32, u64)>,
    ) -> Result<(), String> {
        self.close_tab_renderer_admitted(index, release_listener, Some(expected))
    }

    fn close_tab_renderer_admitted(&self, index: u32, release_listener: bool,
        expected: Option<Option<(u32, u64)>>,
    ) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let _operation = self.lifecycle.enter()?;
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        self.lifecycle.ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        {
            let mut children = self.children.lock().unwrap();
            if let Some(expected) = expected {
                let actual = children.get(&index).map(|child|
                    (child.child.id(), child.comms.spawn_epoch.load(Ordering::SeqCst)));
                if actual != expected {
                    return Err("TAB_RENDERER_CLOSE_TARGET_STALE".into());
                }
            }
            // Publish retirement before releasing the live-map lock:
            // inspect_renderer must not observe a false clean absence.
            if let Some(owner) = children.remove(&index) {
                self.lifecycle.retire(index, owner);
            }
        }
        self.crash_history.lock().unwrap().remove(&index);
        self.hibernated.lock().unwrap().remove(&index);
        self.pending_restarts.lock().unwrap().remove(&index);
        self.lifecycle.wait_route_retired_until(
            index, std::time::Instant::now() + std::time::Duration::from_secs(3),
        )?;
        if release_listener {
            if let Some(comms) = self.listeners.lock().unwrap().remove(&index) {
                comms.request_shutdown();
            }
            // The route is retired and will never be reused after final close.
            let _ = std::fs::remove_file(Self::socket_path(index));
        }
        Ok(())
    }

    /// Serialize normal-tab visibility with the native move/focus workers.
    /// This is a blocking-worker API; never call it on the GUI event loop.
    pub(crate) fn position_tab_renderer(
        &self, index: u32, css: (f64, f64, f64, f64),
    ) -> Result<(), String> {
        let target = self.capture_window_route(index)?;
        self.position_tab_renderer_bound(index, target, css)
    }

    pub(crate) fn position_tab_renderer_bound(&self, index: u32,
        target: Option<crate::grid_ipc::RendererBinding>, css: (f64, f64, f64, f64),
    ) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index)
            || target.is_some_and(|binding| binding.index != index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        if ![css.0, css.1, css.2, css.3].iter().all(|value| value.is_finite())
            || css.2 < 0.0 || css.3 < 0.0 {
            return Err("GRID_NATIVE_RECT_INVALID".into());
        }
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        if css.0 <= -20000.0 {
            return self.hide_tab_renderer_locked(index, target);
        }
        let binding = target.ok_or("TAB_RENDERER_DOCUMENT_UNAVAILABLE")?;
        let app = self.app().ok_or("TAB_RENDERER_HOST_UNAVAILABLE")?;
        let label = crate::grid_ipc::renderer_label(index);
        if app.state::<crate::bridge::AppState>().active_tab.lock().unwrap().as_deref() != Some(label.as_str()) {
            return Err("TAB_RENDERER_LAYOUT_SUPERSEDED".into());
        }
        let rect = self.abs_rect(css).ok_or("TAB_RENDERER_HOST_UNAVAILABLE")?;
        self.position_window_bound(binding, css, rect)
    }

    pub(crate) fn hide_tab_renderer(&self, index: u32) -> Result<(), String> {
        let target = self.capture_window_route(index)?;
        self.hide_tab_renderer_bound(index, target)
    }

    pub(crate) fn hide_tab_renderer_bound(&self, index: u32,
        target: Option<crate::grid_ipc::RendererBinding>,
    ) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index)
            || target.is_some_and(|binding| binding.index != index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        self.hide_tab_renderer_locked(index, target)
    }

    fn hide_tab_renderer_locked(&self, index: u32, target: Option<crate::grid_ipc::RendererBinding>)
        -> Result<(), String>
    {
        if let Some(binding) = target { return self.hide_window_bound(binding); }
        self.lifecycle.wait_route_retired_until(
            index, std::time::Instant::now() + std::time::Duration::from_secs(3),
        )?;
        // Absence was part of the admission. Never hide a later replacement.
        if self.children.lock().unwrap().contains_key(&index) {
            return Err("TAB_RENDERER_LAYOUT_SUPERSEDED".into());
        }
        Ok(())
    }

    pub(crate) fn hide_grid_route(&self, stamp: crate::grid_ipc::GridRouteStamp) -> Result<(), String> {
        let index = stamp.index;
        if index as usize >= crate::bridge::MAX_GRID { return Err("GRID_INDEX_INVALID".into()); }
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        let target = {
            let _gate = self.lifecycle_lock.try_lock().map_err(|_| "GRID_LIFECYCLE_BUSY")?;
            self.lifecycle.check()?;
            if self.grid_route_stamps_locked(&[index]).first() != Some(&stamp) {
                return Err("GRID_WINDOW_TARGET_STALE".into());
            }
            if let Some(saved) = self.hibernated.lock().unwrap().get_mut(&index) {
                saved.hidden = true;
                saved.blur_hidden = false;
            }
            if let Some((_, saved)) = self.pending_restarts.lock().unwrap().get_mut(&index) {
                saved.hidden = true;
                saved.blur_hidden = false;
            }
            let mut children = self.children.lock().unwrap();
            if let Some(child) = children.get_mut(&index) {
                child.hidden = true;
                child.blur_hidden = false;
                child.hidden_since.get_or_insert_with(std::time::Instant::now);
                Some(Self::window_binding(index, child).ok_or("GRID_WINDOW_TARGET_UNAVAILABLE")?)
            } else {
                if self.lifecycle.is_retiring(index) { return Err("GRID_HIDE_CLEANUP_PENDING".into()); }
                None
            }
        };
        if let Some(binding) = target { self.hide_window_bound(binding) } else { Ok(()) }
    }

    // View switches hide the captured Grid topology, including saved
    // restart/hibernation intent. Never rediscover live hosts after IPC waits.
    pub(crate) fn hide_grid_renderers(&self) -> Result<(), String> {
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        let targets = {
            // Windows reclaim holds lifecycle_lock before trying visibility;
            // never wait on that lock while holding visibility_dispatch.
            let _gate = self.lifecycle_lock.try_lock().map_err(|_| "GRID_LIFECYCLE_BUSY")?;
            self.lifecycle.check()?;
            for saved in self.hibernated.lock().unwrap().values_mut() {
                saved.hidden = true;
                saved.blur_hidden = false;
            }
            for (_, saved) in self.pending_restarts.lock().unwrap().values_mut() {
                saved.hidden = true;
                saved.blur_hidden = false;
            }
            let mut children = self.children.lock().unwrap();
            children.iter_mut().filter(|(index, _)| !crate::grid_ipc::is_tab_renderer(**index))
                .map(|(&index, child)| {
                    // Also fence a not-yet-ready replay; lack of a native
                    // acknowledgement is still reported as partial failure.
                    child.hidden = true;
                    child.blur_hidden = false;
                    child.hidden_since.get_or_insert_with(std::time::Instant::now);
                    (index, Self::window_binding(index, child))
                }).collect::<Vec<_>>()
        };
        let mut failures = Vec::new();
        for (index, target) in targets {
            let result = target.ok_or_else(|| "GRID_WINDOW_TARGET_UNAVAILABLE".to_string())
                .and_then(|binding| self.hide_window_bound(binding));
            if let Err(error) = result { failures.push(format!("{index}:{error}")); }
        }
        if failures.is_empty() { Ok(()) }
        else { Err(format!("GRID_HIDE_PARTIAL:{}", failures.join("; "))) }
    }

    pub(crate) fn select_tab_renderer(&self, index: u32, others: &[u32]) -> Result<(), String> {
        self.select_tab_renderer_bound(self.renderer_binding(index)?, others)
    }

    pub(crate) fn select_tab_renderer_bound(&self, target: crate::grid_ipc::RendererBinding, others: &[u32]) -> Result<(), String> {
        let index = target.index;
        if !crate::grid_ipc::is_tab_renderer(index)
            || others.iter().any(|id| *id == index || !crate::grid_ipc::is_tab_renderer(*id))
        {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        let app = self.app().ok_or("TAB_RENDERER_HOST_UNAVAILABLE")?;
        let state = app.state::<crate::bridge::AppState>();
        let current = self.renderer_binding(index)?;
        if current.owner != target.owner || current.generation != target.generation {
            return Err("TAB_RENDERER_ACTIVATION_STALE".into());
        }
        let peers: Vec<_> = others.iter().map(|&other| (other, self.capture_window_route(other))).collect();
        let mut failures = Vec::new();
        for (other, target) in peers {
            if let Err(error) = target.and_then(|binding| self.hide_tab_renderer_locked(other, binding)) {
                failures.push(format!("{other}:{error}"));
            }
        }
        if !failures.is_empty() {
            return Err(format!("TAB_RENDERER_ACTIVATION_PARTIAL:{}", failures.join("; ")));
        }
        // Peers may already be hidden. A later rejection is a partial
        // activation, not a no-effect failure that may be silently retired.
        let post_hide_error = |error: &str| {
            if others.is_empty() {
                error.to_owned()
            } else {
                format!("TAB_RENDERER_ACTIVATION_PARTIAL:{error}")
            }
        };
        if state.shutdown_requested.load(Ordering::Acquire) {
            return Err(post_hide_error("TAB_APPLICATION_SHUTTING_DOWN"));
        }
        // Hiding peers may wait on native IPC. Revalidate the original
        // process/transport under the native owner gates before publication.
        let children = self.children.lock().unwrap();
        let child = children.get(&index).ok_or_else(|| post_hide_error("TAB_RENDERER_ACTIVATION_STALE"))?;
        let _owner = child.comms.owner_gate.lock().unwrap();
        if !child.document_ready
            || child.comms.expected_pid.load(Ordering::SeqCst) != target.owner.0
            || child.comms.spawn_epoch.load(Ordering::SeqCst) != target.owner.1
            || !child.comms.is_current(target.generation)
        {
            return Err(post_hide_error("TAB_RENDERER_ACTIVATION_STALE"));
        }
        let label = crate::grid_ipc::renderer_label(index);
        if !state.tabs.lock().unwrap().contains_key(&label) {
            return Err(post_hide_error("TAB_RENDERER_ACTIVATION_STALE"));
        }
        *state.active_tab.lock().unwrap() = Some(label);
        Ok(())
    }

    pub(crate) fn publish_tab_renderer_info(
        &self, index: u32, mut info: crate::domain::TabInfo,
    ) -> Result<crate::domain::TabInfo, String> {
        if !crate::grid_ipc::is_tab_renderer(index) || info.id != crate::grid_ipc::renderer_label(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let app = self.app().ok_or("TAB_RENDERER_HOST_UNAVAILABLE")?;
        let state = app.state::<crate::bridge::AppState>();
        let children = self.children.lock().unwrap();
        let child = children.get(&index).ok_or("TAB_RENDERER_DOCUMENT_UNAVAILABLE")?;
        let _owner = child.comms.owner_gate.lock().unwrap();
        if !child.document_ready || !child.comms.is_connected() {
            return Err("TAB_RENDERER_DOCUMENT_UNAVAILABLE".into());
        }
        if let Some(url) = &child.last_url {
            info.url = url.clone();
        }
        let mut tabs = state.tabs.lock().unwrap();
        if state.shutdown_requested.load(Ordering::Acquire) {
            return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
        }
        if tabs.contains_key(&info.id) {
            return Err("TAB_RENDERER_ALREADY_PUBLISHED".into());
        }
        tabs.insert(info.id.clone(), info.clone());
        Ok(info)
    }

    pub(crate) fn navigate_tab_renderer(&self, index: u32, url: &str) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let binding = self.renderer_binding(index)?;
        self.navigate_tab_renderer_bound(binding, url)
    }

    pub(crate) fn navigate_tab_renderer_bound(&self, binding: crate::grid_ipc::RendererBinding, url: &str) -> Result<(), String> {
        let index = binding.index;
        if !crate::grid_ipc::is_tab_renderer(index) { return Err("TAB_RENDERER_ROUTE_INVALID".into()); }
        let command = GridCmd::Navigate { id: crate::grid_ipc::renderer_label(index), url: url.to_owned() };
        self.renderer_request_data(binding, command, 10000)?;
        let app = self.app().ok_or("GRID_REQUEST_OUTCOME_UNKNOWN: renderer host unavailable")?;
        let state = app.state::<crate::bridge::AppState>();
        let mut children = self.children.lock().unwrap();
        let child = children.get_mut(&index).ok_or("GRID_REQUEST_OUTCOME_UNKNOWN: renderer retired after navigation")?;
        let _owner = child.comms.owner_gate.lock().unwrap();
        if !Self::window_binding_matches(child, binding) {
            return Err("GRID_REQUEST_OUTCOME_UNKNOWN: renderer retired after navigation".into());
        }
        if child.navigation_revision == binding.navigation_revision {
            child.last_url = Some(url.to_owned());
        }
        let mut tabs = state.tabs.lock().unwrap();
        if state.shutdown_requested.load(Ordering::Acquire) {
            return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
        }
        if let Some(tab) = tabs.get_mut(&crate::grid_ipc::renderer_label(index)) {
            if let Some(current_url) = &child.last_url {
                tab.url = current_url.clone();
            }
        }
        Ok(())
    }

    pub fn is_hibernated(&self, index: u32) -> bool {
        self.hibernated.lock().unwrap().contains_key(&index)
    }

    /// Preserve child metadata while releasing its process. Used to roll back
    /// a failed resume without losing the URL/layout that existed pre-resume.
    pub fn hibernate_child(&self, index: u32) -> bool {
        let Ok(_operation) = self.lifecycle.enter() else {
            return false;
        };
        if self
            .lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))
            .is_err()
        {
            return false;
        }
        let _gate = self.lifecycle_lock.lock().unwrap();
        if self.lifecycle.check().is_err() {
            return false;
        }
        self.pending_restarts.lock().unwrap().remove(&index);
        let _ = self.request(
            index,
            GridCmd::CloseTab {
                id: crate::grid_ipc::renderer_label(index),
            },
            1000,
        );
        let removed = self.children.lock().unwrap().remove(&index);
        if let Some(child) = removed {
            let saved = SavedChildState {
                last_url: child.last_url.clone(),
                last_rect: child.last_rect,
                hidden: child.hidden,
                blur_hidden: child.blur_hidden,
            };
            self.lifecycle.retire(index, child);
            self.hibernated.lock().unwrap().insert(index, saved);
            true
        } else {
            false
        }
    }

    /// 等待该 index 的 UDS 连接就绪（子进程冷启动需 1~3 秒）。
    fn wait_connected(&self, index: u32, timeout_ms: u64) -> Result<(Arc<ChildComms>, u64), String> {
        self.lifecycle.check()?;
        let comms = self.listeners.lock().unwrap().get(&index).cloned()
            .ok_or_else(|| format!("grid-{index} has no renderer listener"))?;
        let expected = comms.owner_snapshot()?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            self.lifecycle.check()?;
            if let Some(generation) = comms.ready_for_owner(expected)? {
                return Ok((comms, generation));
            }
            if std::time::Instant::now() >= deadline {
                return Err(format!("GRID_RENDERER_READY_TIMEOUT:{index}"));
            }
            if self.lifecycle.wait_or_stop(std::time::Duration::from_millis(50)) {
                return Err("GRID_SHUTDOWN_IN_PROGRESS".into());
            }
        }
    }

    /// Wait for initial admission, then dispatch at most once. A missing
    /// response cannot establish that a native or external effect did not run.
    pub fn request(&self, index: u32, cmd: GridCmd, timeout_ms: u64) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        if !cmd.targets(index) {
            return Err("GRID_REQUEST_TARGET_REJECTED".into());
        }
        let (comms, generation) = self.wait_connected(index, 15000)?;
        if crate::grid_ipc::is_tab_renderer(index) {
            let children = self.children.lock().unwrap();
            let child = children.get(&index).ok_or("TAB_RENDERER_DOCUMENT_UNAVAILABLE")?;
            let creating = matches!(&cmd, GridCmd::CreateTab { .. });
            if !Arc::ptr_eq(&child.comms, &comms) || creating == child.document_ready {
                return Err("TAB_RENDERER_DOCUMENT_UNAVAILABLE".into());
            }
        }
        self.request_once_with(comms, generation, &cmd, timeout_ms).map(|_| ())
    }

    /// A delayed load retry retains the native failure's complete identity.
    pub(crate) fn retry_failed_navigation(&self, binding: crate::grid_ipc::RendererBinding, url: &str) -> Result<(), String> {
        let index = binding.index;
        let _operation = self.lifecycle.enter()?;
        if index as usize >= crate::bridge::MAX_GRID || !crate::grid_ipc::restorable_address(url)
            || !url::Url::parse(url).is_ok_and(|value| matches!(value.scheme(), "http" | "https")) {
            return Err("GRID_RETRY_ADDRESS_REJECTED".into());
        }
        let comms = {
            let children = self.children.lock().unwrap();
            let child = children.get(&index).ok_or("GRID_RENDERER_NOT_OWNED")?;
            if !child.document_ready || child.comms.owner_snapshot()? != binding.owner
                || !child.comms.is_current(binding.generation)
                || child.navigation_revision != binding.navigation_revision
                || child.last_url.as_deref() != Some(url) {
                return Err("GRID_RETRY_SUPERSEDED".into());
            }
            Arc::clone(&child.comms)
        };
        let cmd = GridCmd::Navigate { id: crate::grid_ipc::renderer_label(index), url: url.to_string() };
        self.request_once_fenced(comms, binding.generation, Some(binding.navigation_revision), &cmd, 10000).map(|_| ())
    }

    /// Fail-closed native probe of the actual Grid page. Stored last_url is
    /// only a navigation target and cannot prove the live DOM is empty.
    pub fn read_replies(&self, index: u32) -> Result<String, String> {
        self.grid_read_replies_bound(self.grid_eval_binding(index)?)
    }

    fn request_once_with(
        &self,
        comms: Arc<ChildComms>,
        generation: u64,
        cmd: &GridCmd,
        timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        self.request_once_fenced(comms, generation, None, cmd, timeout_ms)
    }

    fn request_once_fenced(
        &self,
        comms: Arc<ChildComms>,
        generation: u64,
        document_revision: Option<u64>,
        cmd: &GridCmd,
        timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        self.lifecycle.check()?;
        if comms.closing.load(Ordering::SeqCst) {
            return Err("GRID_CONNECTION_CLOSING".into());
        }
        let seq;
        let (tx, rx) = mpsc::channel();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let write_res = {
            let mut guard = crate::grid_ipc::lock_renderer_writer(&comms.writer, deadline, || {
                self.lifecycle.check()?;
                if !comms.is_current(generation) {
                    return Err("GRID_CONNECTION_GENERATION_CHANGED".into());
                }
                Ok(())
            })?;
            self.lifecycle.check()?;
            if !comms.is_current(generation) {
                return Err("GRID_CONNECTION_GENERATION_CHANGED".into());
            }
            if comms.blank_reclaiming.load(Ordering::SeqCst) && !cmd.allowed_during_blank_reclaim() {
                return Err("TAB_RENDERER_RECLAIM_IN_PROGRESS".into());
            }
            // Allocate in writer order, not caller/thread order. The worker
            // rejects replays and the counter must never wrap.
            seq = crate::grid_ipc::next_request_sequence(&comms.seq)?;
            let native_document = if cmd.document_id().is_some() {
                let observed = crate::grid_ipc::lock_renderer_writer(&comms.native_document, deadline, || {
                    self.lifecycle.check()?;
                    if !comms.is_current(generation) { return Err("GRID_CONNECTION_GENERATION_CHANGED".into()); }
                    Ok(())
                })?;
                let stamp = (*observed).filter(|stamp| stamp.usable())
                    .ok_or("GRID_NATIVE_DOCUMENT_UNAVAILABLE")?;
                Some(stamp)
            } else { None };
            let frame = crate::grid_ipc::encode_wire(&Wire::Request { seq, cmd: cmd.clone(), native_document })
                .map_err(|_| "GRID_REQUEST_FRAME_REJECTED")?;
            if comms.closing.load(Ordering::SeqCst) {
                return Err("GRID_CONNECTION_CLOSING".into());
            }
            let result = match guard.as_mut() {
                Some(w) => {
                    {
                        let _owner = crate::grid_ipc::lock_renderer_writer(&comms.owner_gate, deadline, || {
                            self.lifecycle.check()?;
                            if !comms.is_current(generation) { return Err("GRID_CONNECTION_GENERATION_CHANGED".into()); }
                            Ok(())
                        })?;
                        if !comms.is_current(generation) {
                            return Err("GRID_CONNECTION_GENERATION_CHANGED".into());
                        }
                        if native_document.is_some_and(|stamp|
                            *comms.native_document.lock().unwrap() != Some(stamp)) {
                            return Err("GRID_NATIVE_DOCUMENT_STALE".into());
                        }
                        if document_revision.is_some_and(|revision| {
                            revision != comms.navigation_revision.load(Ordering::SeqCst)
                        }) {
                            return Err("GRID_REQUEST_DOCUMENT_STALE".into());
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err("GRID_WRITER_ADMISSION_TIMEOUT".into());
                        }
                        let mut pending = crate::grid_ipc::lock_renderer_writer(&comms.pending, deadline, || {
                            self.lifecycle.check()?;
                            if !comms.is_current(generation) { return Err("GRID_CONNECTION_GENERATION_CHANGED".into()); }
                            Ok(())
                        })?;
                        if std::time::Instant::now() >= deadline {
                            return Err("GRID_WRITER_ADMISSION_TIMEOUT".into());
                        }
                        if pending.len() >= crate::grid_ipc::MAX_PENDING_RENDERER_REQUESTS {
                            return Err("GRID_REQUEST_CAPACITY_EXHAUSTED".into());
                        }
                        if !cmd.preserves_pristine_blank() {
                            comms.blank_reclaim_blocked.store(true, Ordering::SeqCst);
                        }
                        pending.insert(seq, tx);
                    }
                    crate::grid_ipc::write_frame_until(&frame, deadline, |bytes, remaining| {
                        // The socket timeout is per syscall, so renew it with
                        // the smaller remaining frame budget, not a fresh 2s.
                        w.set_write_timeout(Some(remaining))?;
                        std::io::Write::write(w, bytes)
                    })
                }
                None => return Err("GRID_NOT_CONNECTED".into()),
            };
            if result.is_err() {
                comms.connected.store(false, Ordering::SeqCst);
                if let Some(control) = comms.control.lock().unwrap().as_ref() {
                    let _ = control.shutdown(std::net::Shutdown::Both);
                }
            }
            result
        };
        if let Err(e) = write_res {
            comms.block_blank_reclaim_for(generation);
            comms.pending.lock().unwrap().remove(&seq);
            comms.detach_if_current(generation);
            return Err(format!("GRID_REQUEST_OUTCOME_UNKNOWN: UDS write failed: {e}"));
        }
        loop {
            if self.lifecycle.is_stopping() || comms.closing.load(Ordering::SeqCst) {
                comms.block_blank_reclaim_for(generation);
                comms.pending.lock().unwrap().remove(&seq);
                return Err("GRID_REQUEST_OUTCOME_UNKNOWN: shutdown after dispatch".into());
            }
            // A transport replacement after dispatch cannot establish that
            // the original action did not run. Never wait on its new owner.
            if !comms.is_current(generation) {
                comms.block_blank_reclaim_for(generation);
                comms.pending.lock().unwrap().remove(&seq);
                return Err("GRID_REQUEST_OUTCOME_UNKNOWN: original connection changed after dispatch".into());
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                comms.block_blank_reclaim_for(generation);
                comms.pending.lock().unwrap().remove(&seq);
                return Err("GRID_REQUEST_OUTCOME_UNKNOWN: 等待子进程响应超时".into());
            }
            match rx.recv_timeout(remaining.min(std::time::Duration::from_millis(100))) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    comms.block_blank_reclaim_for(generation);
                    comms.pending.lock().unwrap().remove(&seq);
                    return Err("GRID_REQUEST_OUTCOME_UNKNOWN: GRID_CONNECTION_CLOSED".into());
                }
            }
        }
    }

    /// Capture the original native window identity for queued operations.
    fn window_binding(index: u32, child: &GridChildHandle) -> Option<crate::grid_ipc::RendererBinding> {
        let owner = child.comms.owner_snapshot().ok()?;
        let generation = child.comms.generation.load(Ordering::SeqCst);
        if !child.document_ready || !child.comms.is_current(generation)
            || child.comms.blank_reclaiming.load(Ordering::SeqCst) {
            return None;
        }
        Some(crate::grid_ipc::RendererBinding {
            index, owner, generation, navigation_revision: child.navigation_revision,
        })
    }

    fn window_binding_matches(child: &GridChildHandle, binding: crate::grid_ipc::RendererBinding) -> bool {
        // Window geometry/visibility survives navigation within the SAME host.
        child.document_ready && child.child.id() == binding.owner.0
            && child.comms.spawn_epoch.load(Ordering::SeqCst) == binding.owner.1
            && child.comms.is_current(binding.generation)
            && !child.comms.blank_reclaiming.load(Ordering::SeqCst)
    }

    fn visibility_targets(&self) -> Vec<crate::grid_ipc::RendererBinding> {
        self.children.lock().unwrap().iter()
            .filter_map(|(&index, child)| Self::window_binding(index, child)).collect()
    }

    pub fn send(&self, index: u32, cmd: GridCmd) {
        let binding = self.children.lock().unwrap().get(&index)
            .and_then(|child| Self::window_binding(index, child));
        if let Some(binding) = binding { self.send_window_bound(binding, cmd); }
    }

    // Background callers already hold visibility_dispatch; a receipt is
    // checked against the same process and connection after native execution.
    fn send_window_bound(&self, binding: crate::grid_ipc::RendererBinding, cmd: GridCmd) {
        if let Err(error) = self.request_window_bound(binding, cmd, 1500) {
            eprintln!("[grid-manager] window receipt unconfirmed index={}: {error}", binding.index);
        }
    }

    fn request_window_bound(&self, binding: crate::grid_ipc::RendererBinding,
        cmd: GridCmd, timeout_ms: u64,
    ) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        if !cmd.targets(binding.index) { return Err("GRID_REQUEST_TARGET_REJECTED".into()); }
        let visible = match &cmd {
            GridCmd::UpdateRect { .. } => true,
            GridCmd::HideWindow { .. } => false,
            _ => return Err("GRID_WINDOW_COMMAND_INVALID".into()),
        };
        let comms = {
            let children = self.children.lock().unwrap();
            let child = children.get(&binding.index).ok_or("GRID_WINDOW_TARGET_STALE")?;
            if !Self::window_binding_matches(child, binding) {
                return Err("GRID_WINDOW_TARGET_STALE".into());
            }
            Arc::clone(&child.comms)
        };
        let result = self.request_once_with(comms, binding.generation, &cmd, timeout_ms).map(|_| ());
        let mut children = self.children.lock().unwrap();
        let child = children.get_mut(&binding.index)
            .ok_or("GRID_REQUEST_OUTCOME_UNKNOWN: window host retired")?;
        let _owner = child.comms.owner_gate.lock().unwrap();
        if !Self::window_binding_matches(child, binding) {
            return Err("GRID_REQUEST_OUTCOME_UNKNOWN: window host changed".into());
        }
        child.native_window_visibility = result.as_ref().ok().map(|_| (binding.generation, visible));
        result
    }

    pub(crate) fn capture_window_route(&self, index: u32)
        -> Result<Option<crate::grid_ipc::RendererBinding>, String>
    {
        let children = self.children.lock().unwrap();
        children.get(&index).map(|child| Self::window_binding(index, child)
            .ok_or_else(|| "GRID_WINDOW_TARGET_UNAVAILABLE".to_string())).transpose()
    }

    // Explicit hide and position share the same identity-bound receipt path.
    // Caller holds visibility_dispatch. Do not overwrite a replacement's flags.
    fn hide_window_bound(&self, binding: crate::grid_ipc::RendererBinding) -> Result<(), String> {
        {
            let mut children = self.children.lock().unwrap();
            let child = children.get_mut(&binding.index).ok_or("GRID_WINDOW_TARGET_STALE")?;
            let _owner = child.comms.owner_gate.lock().unwrap();
            if !Self::window_binding_matches(child, binding) {
                return Err("GRID_WINDOW_TARGET_STALE".into());
            }
            child.hidden = true;
            child.blur_hidden = false;
            child.hidden_since.get_or_insert_with(std::time::Instant::now);
        }
        self.request_window_bound(binding, GridCmd::HideWindow {
            id: crate::grid_ipc::renderer_label(binding.index),
        }, 3000)
    }

    fn position_window_bound(&self, binding: crate::grid_ipc::RendererBinding,
        css: (f64, f64, f64, f64), rect: crate::grid_ipc::IpcRect,
    ) -> Result<(), String> {
        if css.0 <= -20000.0 { return self.hide_window_bound(binding); }
        rect.validate()?;
        {
            let mut children = self.children.lock().unwrap();
            let child = children.get_mut(&binding.index).ok_or("GRID_WINDOW_TARGET_STALE")?;
            let _owner = child.comms.owner_gate.lock().unwrap();
            if !Self::window_binding_matches(child, binding) {
                return Err("GRID_WINDOW_TARGET_STALE".into());
            }
            // Store desired layout before dispatch so reclaim cannot consume
            // a foreground request whose native response is still pending.
            child.last_rect = Some(css);
            child.hidden = false;
            child.blur_hidden = false;
            child.hidden_since = None;
        }
        self.request_window_bound(binding, GridCmd::UpdateRect {
            id: crate::grid_ipc::renderer_label(binding.index), rect,
        }, 3000)
    }

    // ===== 状态记录（崩溃重放/主窗移动跟随用，由 bridge 宫格命令维护） =====

    pub fn record_url(&self, index: u32, url: &str) {
        if let Some(h) = self.children.lock().unwrap().get_mut(&index) {
            h.last_url = Some(url.to_string());
        }
    }

    /// Last committed navigation target, used to avoid reloading a healthy
    /// WebView when only the Grid geometry/layout changes.
    pub fn last_url(&self, index: u32) -> Option<String> {
        self.children
            .lock()
            .unwrap()
            .get(&index)
            .and_then(|child| child.last_url.clone())
    }

    pub fn record_rect(&self, index: u32, rect: (f64, f64, f64, f64)) {
        if let Some(h) = self.children.lock().unwrap().get_mut(&index) {
            h.last_rect = Some(rect);
            h.hidden = false;
            h.blur_hidden = false;
            h.hidden_since = None;
        }
    }

    pub fn record_hidden(&self, index: u32) {
        if let Some(h) = self.children.lock().unwrap().get_mut(&index) {
            h.hidden = true;
            h.hidden_since.get_or_insert_with(std::time::Instant::now);
        }
    }

    /// Destroy hidden, unfocused WebKit children after an adaptive idle window.
    /// Saved navigation and layout metadata allow the normal Grid activation
    /// path to recreate the child process without losing its URL or zoom owner.
    /// Adaptive reclamation remains conservative: a hidden AI Grid may have
    /// unpersisted conversation state. Never destroy arbitrary external pages
    /// based on URL/geometry alone. Reclaim only blank, still-hidden cells.
    pub fn hibernate_idle_hidden(
        &self,
        idle_for: std::time::Duration,
        max_children: usize,
    ) -> Vec<u32> {
        let Ok(_operation) = self.lifecycle.enter() else {
            return Vec::new();
        };
        if self
            .lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))
            .is_err()
        {
            return Vec::new();
        }
        if max_children == 0 {
            return Vec::new();
        }
        let now = std::time::Instant::now();
        let mut candidates: Vec<(std::time::Instant, u32)> = self
            .children
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(index, child)| {
                // Ordinary tabs can contain unsaved documents. Grid's blank
                // page reclamation policy must never retire them.
                if crate::grid_ipc::is_tab_renderer(*index) {
                    return None;
                }
                if !grid_child_is_hibernatable(
                    child.hidden,
                    child.blur_hidden,
                    child.focused,
                    child.hidden_since,
                    now,
                    idle_for,
                ) || !grid_content_reclaimable(child.last_url.as_deref())
                {
                    return None;
                }
                Some((child.hidden_since?, *index))
            })
            .collect();
        // Oldest blank cells first, with an explicit per-sweep kill budget.
        candidates.sort_unstable();
        let mut hibernated = Vec::new();
        for (hidden_since, index) in candidates.into_iter().take(max_children) {
            let Ok(_visibility) = self.visibility_dispatch.try_lock() else { continue; };
            let eligible = |child: &GridChildHandle| {
                child.document_ready && child.hidden_since == Some(hidden_since)
                    && !child.comms.blank_reclaim_blocked.load(Ordering::SeqCst)
                    && grid_content_reclaimable(child.last_url.as_deref())
                    && grid_child_is_hibernatable(child.hidden, child.blur_hidden, child.focused,
                        child.hidden_since, std::time::Instant::now(), idle_for)
            };
            let (comms, binding) = {
                let children = self.children.lock().unwrap();
                let Some(child) = children.get(&index) else { continue; };
                if !eligible(child) { continue; }
                let Ok(owner) = child.comms.owner_snapshot() else { continue; };
                (Arc::clone(&child.comms), crate::grid_ipc::RendererBinding {
                    index, owner, generation: child.comms.generation.load(Ordering::SeqCst),
                    navigation_revision: child.navigation_revision,
                })
            };
            let Some(_claim) = comms.try_claim_blank_reclaim(binding.generation) else { continue; };
            let command = GridCmd::HideWindow { id: crate::grid_ipc::renderer_label(index) };
            if self.request_once_with(Arc::clone(&comms), binding.generation, &command, 1500).is_err() { continue; }
            let command = GridCmd::HibernationProbe { id: crate::grid_ipc::renderer_label(index) };
            if !matches!(self.request_once_fenced(Arc::clone(&comms), binding.generation,
                Some(binding.navigation_revision), &command, 1500), Ok(Some(encoded)) if grid_hibernation_probe_safe(&encoded)) {
                continue;
            }
            let Ok(_gate) = self.lifecycle_lock.try_lock() else { continue; };
            if self.lifecycle.is_stopping() { break; }
            let mut children = self.children.lock().unwrap();
            let still_eligible = children.get(&index).is_some_and(|child|
                Arc::ptr_eq(&child.comms, &comms)
                    && child.child.id() == binding.owner.0
                    && comms.spawn_epoch.load(Ordering::SeqCst) == binding.owner.1
                    && comms.is_current(binding.generation)
                    && child.navigation_revision == binding.navigation_revision
                    && eligible(child));
            if !still_eligible { continue; }
            if let Some(child) = children.remove(&index) {
                self.hibernated.lock().unwrap().insert(index, SavedChildState {
                    last_url: child.last_url.clone(), last_rect: child.last_rect,
                    hidden: true, blur_hidden: false,
                });
                self.pending_restarts.lock().unwrap().remove(&index);
                self.lifecycle.retire(index, child);
                hibernated.push(index);
            }
        }
        hibernated
    }

    pub fn hibernated_count(&self) -> usize {
        self.hibernated.lock().unwrap().len()
    }

    /// 当前存活子进程 index 列表。
    pub fn indices(&self) -> Vec<u32> {
        self.children.lock().unwrap().keys().copied()
            .filter(|index| !crate::grid_ipc::is_tab_renderer(*index)).collect()
    }

    pub(crate) fn all_renderer_indices(&self) -> Vec<u32> {
        self.children.lock().unwrap().keys().copied().collect()
    }

    /// CSS 矩形（相对主窗内容区）→ 绝对屏幕物理矩形。
    pub fn abs_rect(&self, css: (f64, f64, f64, f64)) -> Option<IpcRect> {
        let app = self.app()?;
        let win = app.get_window("main")?;
        let pos = win.inner_position().ok()?;
        let scale = win.scale_factor().ok()?;
        Some(IpcRect {
            x: pos.x as f64 + css.0 * scale,
            y: pos.y as f64 + css.1 * scale,
            w: (css.2 * scale).max(1.0),
            h: (css.3 * scale).max(1.0),
        })
    }

    /// GTK move/resize callback: record latest geometry without touching the
    /// UDS writer lock. Slow grid children can take up to 2 seconds per write,
    /// which must never happen on the GTK main event loop.
    pub fn request_reposition_async(&self) {
        if self.lifecycle.is_stopping() {
            return;
        }
        self.reposition_pending.fetch_add(1, Ordering::AcqRel);
        if self.reposition_running.swap(true, Ordering::AcqRel) {
            return;
        }
        let Some(app) = self.app() else {
            self.reposition_running.store(false, Ordering::Release);
            return;
        };
        let spawn = self.spawn_worker("browseros-grid-position", move || {
                let manager = &app.state::<crate::bridge::AppState>().grid_manager;
                loop {
                    if manager.lifecycle.is_stopping() {
                        manager.reposition_running.store(false, Ordering::Release);
                        break;
                    }
                    let observed = manager.reposition_pending.load(Ordering::Acquire);
                    let started = std::time::Instant::now();
                    manager.reposition_visible();
                    let elapsed = started.elapsed().as_millis();
                    if elapsed >= 50 {
                        let pending = manager.reposition_pending.load(Ordering::Acquire);
                        eprintln!(
                            "[health] grid_reposition_background_slow elapsed_ms={elapsed} pending_updates={}",
                            pending.saturating_sub(observed)
                        );
                    }
                    if !manager.finish_reposition_iteration(observed) {
                        break;
                    }
                }
            });
        if let Err(error) = spawn {
            self.reposition_running.store(false, Ordering::Release);
            eprintln!("[grid-manager] relocation worker unavailable: {error}");
        }
    }

    /// A move arriving while the worker is sending must be replayed after the
    /// stale write finishes. Hand-off is race-safe even when a new event starts
    /// another worker during the final active=false transition.
    fn finish_reposition_iteration(&self, observed: u64) -> bool {
        if self.lifecycle.is_stopping() {
            self.reposition_running.store(false, Ordering::Release);
            return false;
        }
        if self.reposition_pending.load(Ordering::Acquire) != observed {
            return true;
        }
        self.reposition_running.store(false, Ordering::Release);
        if self.reposition_pending.load(Ordering::Acquire) == observed {
            return false;
        }
        self.reposition_running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Worker thread only: recompute latest CSS→screen rectangles and emit UDS
    /// writes in sequence. Visibility checks remain the source of truth.
    pub fn reposition_visible(&self) {
        let targets = self.visibility_targets();
        for binding in targets {
            if self.lifecycle.is_stopping() { break; }
            let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
            let css = self.children.lock().unwrap().get(&binding.index)
                .filter(|child| Self::window_binding_matches(child, binding)
                    && !child.hidden && !child.blur_hidden)
                .and_then(|child| child.last_rect);
            if let Some(rect) = css.and_then(|css| self.abs_rect(css)) {
                self.send_window_bound(binding, GridCmd::UpdateRect {
                    id: crate::grid_ipc::renderer_label(binding.index), rect,
                });
            }
        }
    }

    /// Publish renderer metadata only for the live child and connection.
    fn accept_renderer_event(&self, index: u32, comms: &Arc<ChildComms>, generation: u64,
        name: &str, payload: &mut serde_json::Value) -> bool {
        if !crate::grid_ipc::valid_renderer_event(index, name, payload) {
            return false;
        }
        let mut children = self.children.lock().unwrap();
        let Some(child) = children.get_mut(&index) else { return false };
        let _owner = comms.owner_gate.lock().unwrap();
        if !Arc::ptr_eq(&child.comms, comms) || !comms.is_current(generation) {
            return false;
        }
        match name {
            "grid-child-focus" => child.focused = true,
            "grid-child-blur" => child.focused = false,
            "native-document-revision" => {
                let Some(stamp) = crate::grid_ipc::NativeDocumentStamp::from_report(payload) else { return false; };
                let mut current = comms.native_document.lock().unwrap();
                // Native callbacks may be delivered out of order. A stale
                // instance/revision can never replace a newer observation.
                if current.is_some_and(|previous| stamp <= previous) { return true; }
                let Some(revision) = child.navigation_revision.checked_add(1) else { return false; };
                *current = Some(stamp);
                child.navigation_revision = revision;
                comms.navigation_revision.store(revision, Ordering::SeqCst);
                if comms.blank_reclaiming.load(Ordering::SeqCst) {
                    comms.blank_reclaim_blocked.store(true, Ordering::SeqCst);
                }
            }
            "tab-document-changed" => {
                let Some(revision) = child.navigation_revision.checked_add(1) else { return false; };
                child.navigation_revision = revision;
                comms.navigation_revision.store(revision, Ordering::SeqCst);
                // Invalidate an in-flight reclaim. Initial load completion
                // alone must not permanently disqualify a pristine blank
                // renderer; the native safe-page probe and existing history
                // checks still decide eligibility after loading.
                if comms.blank_reclaiming.load(Ordering::SeqCst) {
                    comms.blank_reclaim_blocked.store(true, Ordering::SeqCst);
                }
            }
            "tab-navigated" => {
                let Some(revision) = child.navigation_revision.checked_add(1) else { return false; };
                child.navigation_revision = revision;
                comms.navigation_revision.store(revision, Ordering::SeqCst);
                child.last_url = payload["url"].as_str().map(str::to_owned);
                if child.last_url.as_deref() != Some("about:blank") {
                    comms.blank_reclaim_blocked.store(true, Ordering::SeqCst);
                }
                if crate::grid_ipc::is_tab_renderer(index) {
                    if let (Some(app), Some(url)) = (self.app(), child.last_url.as_ref()) {
                        let state = app.state::<crate::bridge::AppState>();
                        let mut tabs = state.tabs.lock().unwrap();
                        if !state.shutdown_requested.load(Ordering::Acquire) {
                            if let Some(tab) = tabs.get_mut(&crate::grid_ipc::renderer_label(index)) {
                                tab.url = url.clone();
                            }
                        }
                    }
                }
            },
            "renderer-resource" => {
                if let Some(app) = self.app() {
                    let state = app.state::<crate::bridge::AppState>();
                    let id = crate::grid_ipc::renderer_label(index);
                    let live = state.tabs.lock().unwrap().contains_key(&id);
                    if live && !state.shutdown_requested.load(Ordering::Acquire) {
                        if let Ok(raw) = serde_json::from_value::<crate::bridge::RawResourceEvent>(payload["resource"].clone()) {
                            if raw.url.len() <= 8192
                                && raw.method.as_ref().is_none_or(|v| v.len() <= 32)
                                && raw.mime.as_ref().is_none_or(|v| v.len() <= 256)
                            {
                                crate::bridge::on_resource_received(&app, &id, raw);
                            }
                        }
                    }
                }
            }
            "tab-title" | "browser-resources" => {
                let id = crate::grid_ipc::renderer_label(index);
                let url = if name == "tab-title" { payload["url"].as_str() } else { payload["page_url"].as_str() };
                let mut drop_event = true;
                if let Some(app) = self.app() {
                    let state = app.state::<crate::bridge::AppState>();
                    let is_active = state.active_tab.lock().unwrap().as_deref() == Some(id.as_str());
                    let mut tabs = state.tabs.lock().unwrap();
                    if let Some(tab) = tabs.get_mut(&id) {
                        if !state.shutdown_requested.load(Ordering::Acquire)
                            && url == Some(tab.url.as_str())
                            && (name == "tab-title" || is_active)
                        {
                            drop_event = false;
                            if name == "tab-title" {
                                if let Some(title) = payload["title"].as_str() {
                                    tab.title = title.to_owned();
                                }
                            }
                        }
                    }
                }
                payload["renderer_drop"] = serde_json::json!(drop_event);
            }
            "new-tab-request" => {
                let allowed = self.app().is_some_and(|app| {
                    let state = app.state::<crate::bridge::AppState>();
                    let logical_live = if crate::grid_ipc::is_tab_renderer(index) {
                        child.document_ready && state.tabs.lock().unwrap()
                            .contains_key(&crate::grid_ipc::renderer_label(index))
                    } else {
                        (index as usize) < crate::bridge::MAX_GRID
                    };
                    logical_live && !state.shutdown_requested.load(Ordering::Acquire)
                        && payload["url"].as_str().is_some_and(|url|
                            crate::grid_ipc::popup_address_supported(url, child.last_url.as_deref()))
                });
                // Background pages retain the existing popup policy; stale,
                // unpublished, shutdown and unsupported sources cannot open.
                payload["renderer_drop"] = serde_json::json!(!allowed);
            }
            "renderer-download" => {
                // Even an early native observation disqualifies blank
                // reclaim. Notification is advisory and only for a live tab.
                comms.blank_reclaim_blocked.store(true, Ordering::SeqCst);
                let Some(observation) = crate::grid_ipc::DownloadObservation::from_report(payload) else {
                    return false;
                };
                {
                    let mut last = comms.download_observation.lock().unwrap();
                    if last.as_ref().is_some_and(|(previous_generation, previous)| {
                        if observation == *previous { *previous_generation == generation }
                        else { !observation.advances(*previous) }
                    }) {
                        payload["renderer_drop"] = serde_json::json!(true);
                        return true;
                    }
                    *last = Some((generation, observation));
                }
                let id = crate::grid_ipc::renderer_label(index);
                let live = self.app().is_some_and(|app| {
                    let state = app.state::<crate::bridge::AppState>();
                    !state.shutdown_requested.load(Ordering::Acquire)
                        && state.tabs.lock().unwrap().contains_key(&id)
                });
                payload["renderer_drop"] = serde_json::json!(!live);
                payload["pid"] = serde_json::json!(child.child.id());
                payload["spawn_epoch"] = serde_json::json!(comms.spawn_epoch.load(Ordering::SeqCst).to_string());
                payload["renderer_generation"] = serde_json::json!(generation.to_string());
                payload["observed_unfinished"] = serde_json::json!(observation.observed_unfinished());
            }
            "browser-tabs://shell-shortcut" => {
                let id = crate::grid_ipc::renderer_label(index);
                let allowed = child.document_ready && child.focused && !child.hidden
                    && !child.blur_hidden && self.app().is_some_and(|app| {
                        let state = app.state::<crate::bridge::AppState>();
                        let active = state.active_tab.lock().unwrap().as_deref() == Some(id.as_str());
                        let live = state.tabs.lock().unwrap().contains_key(&id);
                        active && live && !state.shutdown_requested.load(Ordering::Acquire)
                    });
                // A valid delayed shortcut from an inactive renderer is an
                // ignored observation, not a reason to disconnect that tab.
                payload["renderer_drop"] = serde_json::json!(!allowed);
            }
            "grid-load-failed" => {
                payload["renderer_generation"] = serde_json::json!(generation.to_string());
                payload["renderer_pid"] = serde_json::json!(child.child.id());
                payload["renderer_epoch"] = serde_json::json!(comms.spawn_epoch.load(Ordering::SeqCst).to_string());
                payload["renderer_navigation_revision"] = serde_json::json!(child.navigation_revision.to_string());
            }
            _ => {}
        }
        true
    }

    fn any_child_focused(&self) -> bool {
        self.children.lock().unwrap().values().any(|child| child.focused && child.comms.is_connected())
    }

    /// 主窗失焦（或子窗口失焦）后的统一入口：若焦点在任一宫格子窗口内则不动，
    /// 否则主窗也无焦点时才隐藏（用户点了宫格 ≠ 切走应用）。
    pub fn hide_for_blur_if_no_child_focus(&self) {
        self.request_hide_for_blur_async();
    }

    fn apply_hide_for_blur_if_no_child_focus(&self, intent: &lifecycle::VisibilityIntent) {
        if self.any_child_focused() {
            return;
        }
        let main_focused = self
            .app()
            .and_then(|app| app.get_window("main"))
            .and_then(|w| w.is_focused().ok())
            .unwrap_or(true);
        if main_focused {
            return;
        }
        self.apply_hide_for_blur(intent);
    }

    /// 主窗失焦：隐藏所有可见宫格子进程窗口（标记 blur_hidden，聚焦时恢复）。
    pub fn hide_for_blur(&self) {
        self.request_visibility(VisibilityMode::Minimized, std::time::Duration::ZERO);
    }

    fn apply_hide_for_blur(&self, intent: &lifecycle::VisibilityIntent) {
        for &binding in &intent.targets {
            let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
            if !self.visibility.is_latest(intent.revision) { break; }
            // Focus can return while an earlier renderer was awaiting a receipt.
            if intent.mode == VisibilityMode::Blur && (self.any_child_focused()
                || self.app().and_then(|app| app.get_window("main"))
                    .and_then(|window| window.is_focused().ok()).unwrap_or(true)) {
                break;
            }
            let need = {
                let mut children = self.children.lock().unwrap();
                match children.get_mut(&binding.index) {
                    Some(h) if Self::window_binding_matches(h, binding)
                        && !h.hidden && h.last_rect.is_some()
                        && (!h.blur_hidden || h.native_window_visibility != Some((binding.generation, false))) => {
                        // Desired state, not a native receipt. A newer show can
                        // compensate even if the hide's outcome is unknown.
                        h.blur_hidden = true;
                        true
                    }
                    _ => false,
                }
            };
            if need {
                let command = GridCmd::HideWindow { id: crate::grid_ipc::renderer_label(binding.index) };
                let result = self.request_window_bound(binding, command.clone(), 1500);
                if let Err(error) = result {
                    // Hide is idempotent on this exact native host. Do not
                    // retry a superseded focus intent, or replay any page action.
                    let focus_still_allows_hide = intent.mode == VisibilityMode::Minimized
                        || (!self.any_child_focused() && !self.app()
                            .and_then(|app| app.get_window("main"))
                            .and_then(|window| window.is_focused().ok()).unwrap_or(true));
                    if error.starts_with("GRID_REQUEST_OUTCOME_UNKNOWN")
                        && self.visibility.is_latest(intent.revision) && focus_still_allows_hide {
                        if let Err(retry) = self.request_window_bound(binding, command, 750) {
                            eprintln!("[grid-manager] bounded hide reconciliation unconfirmed index={}: {retry}", binding.index);
                        }
                    } else {
                        eprintln!("[grid-manager] hide unconfirmed index={}: {error}", binding.index);
                    }
                }
            }
        }
    }

    /// 主窗重新聚焦：恢复因失焦隐藏的宫格（显式 hidden 的不动）。
    pub fn show_for_focus(&self) {
        self.request_show_for_focus_async();
    }

    fn apply_show_for_focus(&self, intent: &lifecycle::VisibilityIntent) {
        for &binding in &intent.targets {
            let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
            if !self.visibility.is_latest(intent.revision) { break; }
            let css = {
                let children = self.children.lock().unwrap();
                children.get(&binding.index).filter(|child|
                    Self::window_binding_matches(child, binding) && !child.hidden
                        && (child.blur_hidden || child.native_window_visibility != Some((binding.generation, true))))
                    .and_then(|child| child.last_rect)
            };
            let Some(rect) = css.and_then(|css| self.abs_rect(css)) else { continue; };
            {
                let mut children = self.children.lock().unwrap();
                let Some(child) = children.get_mut(&binding.index) else { continue; };
                if !Self::window_binding_matches(child, binding) || child.hidden
                    || (!child.blur_hidden && child.native_window_visibility == Some((binding.generation, true))) {
                    continue;
                }
                child.blur_hidden = false;
            }
            self.send_window_bound(binding, GridCmd::UpdateRect {
                id: crate::grid_ipc::renderer_label(binding.index), rect,
            });
        }
    }

    // ===== 崩溃自愈（Phase 3） =====

    /// Observe exits without reaping the leader. Retirement owns the child
    /// until its group is empty; crash restart waits for that same index.
    fn start_monitor(&self) {
        if self.monitor_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let Some(app) = self.app() else {
            self.monitor_started.store(false, Ordering::SeqCst);
            return;
        };
        let monitor_owner = Arc::clone(&self.lifecycle);
        let started = self.spawn_worker("grid-monitor", move || {
            let mut was_minimized = false;
            loop {
                if monitor_owner.wait_or_stop(std::time::Duration::from_millis(500)) { break; }
                let state = app.state::<crate::bridge::AppState>();
                let manager = &state.grid_manager;
                manager.retry_visibility_worker();
                // Preserve the existing window visibility behavior.
                if let Some(win) = app.get_window("main") {
                    let minimized = win.is_minimized().unwrap_or(false);
                    if minimized != was_minimized {
                        was_minimized = minimized;
                        if minimized {
                            eprintln!("[grid-manager] 主窗最小化，隐藏全部宫格");
                            manager.hide_for_blur();
                        } else {
                            eprintln!("[grid-manager] 主窗恢复，还原宫格");
                            manager.show_for_focus();
                        }
                    }
                }
                {
                    let _gate = manager.lifecycle_lock.lock().unwrap();
                    if manager.lifecycle.is_stopping() { break; }
                    let mut exited = Vec::new();
                    {
                        let mut children = manager.children.lock().unwrap();
                        let indices: Vec<_> = children.keys().copied().collect();
                        for index in indices {
                            let observed = children.get_mut(&index).map(|owner| {
                                if let Some(scope) = owner.memory_scope.as_mut() {
                                    scope.observe_orphan_guard();
                                }
                                peek_grid_exit(&owner.child)
                            });
                            match observed {
                                Some(Ok(Some(code))) => {
                                    let owner = children.remove(&index).expect("same locked map");
                                    let saved = SavedChildState {
                                        last_url: owner.last_url.clone(), last_rect: owner.last_rect,
                                        hidden: owner.hidden, blur_hidden: owner.blur_hidden,
                                    };
                                    manager.lifecycle.retire(index, owner);
                                    exited.push((index, code, saved));
                                }
                                Some(Err(_)) => eprintln!("[grid-manager] GRID_WAIT_STATE_UNKNOWN index={index}"),
                                _ => {}
                            }
                        }
                    }
                    for (index, code, saved) in exited {
                        if crate::grid_ipc::is_tab_renderer(index) {
                            let _ = app.emit("isolated-tab-renderer-exited", serde_json::json!({
                                "id": crate::grid_ipc::renderer_label(index), "exit_code": code
                            ));
                            continue;
                        }
                        let (allowed, attempts) = {
                            let mut history = manager.crash_history.lock().unwrap();
                            let events = history.entry(index).or_default();
                            let now = std::time::Instant::now();
                            while events.front().is_some_and(|time| now.duration_since(*time).as_secs() > 120) {
                                events.pop_front();
                            }
                            events.push_back(now);
                            (events.len() <= 3, events.len())
                        };
                        if allowed {
                            eprintln!("[grid-manager] grid-child-{index} exited code={code}; restart waits for group cleanup");
                            manager.pending_restarts.lock().unwrap().insert(index, (
                                std::time::Instant::now() + std::time::Duration::from_millis(350 * attempts as u64),
                                saved,
                            ));
                        } else {
                            eprintln!("[grid-manager] grid-child-{index} crash circuit open (3 restarts / 120s)");
                        }
                    }
                }
                manager.restart_ready_children();
            }
        });
        if let Err(error) = started {
            self.monitor_started.store(false, Ordering::SeqCst);
            eprintln!("[grid-manager] monitor unavailable: {error}");
        }
    }

    fn restart_ready_children(&self) {
        let Ok(_operation) = self.lifecycle.enter() else {
            return;
        };
        let mut replay = Vec::new();
        {
            let _gate = self.lifecycle_lock.lock().unwrap();
            if self.lifecycle.is_stopping() {
                return;
            }
            let ready: Vec<_> = self
                .pending_restarts
                .lock()
                .unwrap()
                .iter()
                .filter(|(index, (when, _))| {
                    *when <= std::time::Instant::now() && !self.lifecycle.is_retiring(**index)
                })
                .map(|(index, _)| *index)
                .collect();
            for index in ready {
                let Some((_, saved)) = self.pending_restarts.lock().unwrap().remove(&index) else {
                    continue;
                };
                if self.children.lock().unwrap().contains_key(&index) {
                    continue;
                }
                if let Err(error) = self.spawn_locked(index, Some(saved.clone())) {
                    if !self.lifecycle.is_stopping() {
                        self.hibernated.lock().unwrap().insert(index, saved);
                    }
                    eprintln!("[grid-manager] restart failed index={index}: {error}");
                } else {
                    replay.push((self.grid_route_stamps_locked(&[index])[0], saved));
                }
            }
        }
        for (stamp, saved) in replay {
            self.replay(stamp, saved);
        }
    }

    /// Crash replay binds the new process under the spawn gate. No worker
    /// may look up a later owner by Grid index or claim that its DOM survived.
    fn replay(&self, stamp: crate::grid_ipc::GridRouteStamp, saved: SavedChildState) {
        let index = stamp.index;
        if crate::grid_ipc::is_tab_renderer(index) { return; }
        let Some(app) = self.app() else { return; };
        let started = self.spawn_worker("grid-replay", move || {
            let state = app.state::<crate::bridge::AppState>();
            let manager = &state.grid_manager;
            let url = saved.last_url.as_deref().filter(|url| crate::grid_ipc::restorable_address(url))
                .unwrap_or("about:blank");
            let prepared = crate::grid_ipc::GridPreparedSlot {
                stamp, created: true, previous_url: saved.last_url.clone(), saved: Some(saved.clone()),
            };
            let creation = manager.grid_binding_for_stamp(stamp)
                .and_then(|binding| manager.grid_create_bound(binding, url).map(|_| binding));
            let binding = match creation {
                Ok(binding) => binding,
                Err(error) => {
                    eprintln!("[grid-manager] replay creation unconfirmed index={index}: {error}");
                    let _ = manager.rollback_grid_creation(&prepared);
                    return;
                }
            };
            // Re-read visibility/geometry after taking the same native
            // dispatch lock as ordinary positioning and focus changes.
            {
                let _visibility = manager.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
                let css = {
                    let children = manager.children.lock().unwrap();
                    children.get(&index).filter(|child|
                        child.child.id() == binding.owner.0
                            && child.comms.spawn_epoch.load(Ordering::SeqCst) == binding.owner.1
                            && child.comms.is_current(binding.generation)
                            && !child.hidden && !child.blur_hidden)
                        .and_then(|child| child.last_rect)
                };
                if let Some(rect) = css.and_then(|css| manager.abs_rect(css)) {
                    if let Err(error) = manager.request_window_bound(binding, GridCmd::UpdateRect {
                        id: crate::grid_ipc::renderer_label(index), rect,
                    }, 3000) {
                        eprintln!("[grid-manager] replay layout unconfirmed index={index}: {error}");
                    }
                }
            }
            if let Err(error) = crate::bridge::apply_grid_zoom(&app, binding) {
                eprintln!("[grid-manager] replay zoom unconfirmed index={index}: {error}");
            }
        });
        if let Err(error) = started {
            // The unready process remains visible to native inspection; do
            // not silently start a second replay or target a replacement.
            eprintln!("[grid-manager] replay unavailable index={index}: {error}");
        }
    }

    /// Remove the live registration, then transfer the complete native owner.
    /// Reusing this index is rejected until retirement confirms group cleanup.
    pub fn kill_child(&self, index: u32) -> bool {
        let Ok(_operation) = self.lifecycle.enter() else {
            return false;
        };
        if self
            .lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))
            .is_err()
        {
            return false;
        }
        let _gate = self.lifecycle_lock.lock().unwrap();
        if self.lifecycle.is_stopping() {
            return false;
        }
        self.crash_history.lock().unwrap().remove(&index);
        self.hibernated.lock().unwrap().remove(&index);
        self.pending_restarts.lock().unwrap().remove(&index);
        let removed = self.children.lock().unwrap().remove(&index);
        if let Some(owner) = removed {
            self.lifecycle.retire(index, owner);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn restart(&self, index: u32) -> Result<(), String> {
        self.lifecycle.check()?;
        self.kill_child(index);
        self.lifecycle.wait_retired_until(
            std::time::Instant::now() + std::time::Duration::from_millis(750),
        )?;
        self.spawn(index)?;
        Ok(())
    }

    /// Observe without reaping; only the retirement owner consumes the child.
    #[allow(dead_code)]
    pub fn check_exited(&self, index: u32) -> Option<i32> {
        let _operation = self.lifecycle.enter().ok()?;
        self.lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))
            .ok()?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        if self.lifecycle.is_stopping() {
            return None;
        }
        let mut children = self.children.lock().unwrap();
        let code = peek_grid_exit(&children.get(&index)?.child).ok()??;
        let owner = children.remove(&index)?;
        self.lifecycle.retire(index, owner);
        Some(code)
    }

    /// 当前存活子进程数量。
    pub fn count(&self) -> usize {
        self.indices().len()
    }

    /// 指定宫格子进程的 OS pid（自检/调试用）。
    pub fn pid_of(&self, index: u32) -> Option<u32> {
        self.children
            .lock()
            .unwrap()
            .get(&index)
            .map(|h| h.child.id())
    }

    /// 所有存活宫格子进程的 (index, pid) 列表（资源统计用）。
    /// Snapshot each live native owner, including ordinary tabs. Do not
    /// include retiring owners as live; their residual RSS stays unattributed
    /// in the parent process tree until operating-system cleanup completes.
    /// Read the current Grid topology, including owners omitted by RSS
    /// inventory. The observation does not authorize closing or reopening.
    pub(crate) fn grid_route_statuses(&self) -> Option<Vec<crate::grid_ipc::GridRouteStatus>> {
        self.lifecycle.check().ok()?;
        let _gate = self.lifecycle_lock.try_lock().ok()?;
        let children = self.children.lock().ok()?;
        let sleeping = self.hibernated.lock().ok()?;
        let restarting = self.pending_restarts.lock().ok()?;
        Some((0..crate::bridge::MAX_GRID as u32).map(|index| {
            let state = if self.lifecycle.is_retiring(index) {
                "cleanup_pending"
            } else if let Some(child) = children.get(&index) {
                if child.comms.is_connected() && child.document_ready
                    && !child.comms.blank_reclaiming.load(Ordering::SeqCst)
                    && child.comms.native_document.lock().ok()
                        .is_some_and(|stamp| stamp.is_some_and(|value| value.usable())) {
                    "ready"
                } else { "not_ready" }
            } else if restarting.contains_key(&index) {
                "restart_pending"
            } else if sleeping.contains_key(&index) {
                "hibernated"
            } else { "absent" };
            crate::grid_ipc::GridRouteStatus { index, state }
        }).collect())
    }

    pub(crate) fn renderer_processes(&self) -> Vec<crate::grid_ipc::RendererProcess> {
        let children = self.children.lock().unwrap();
        let mut result: Vec<_> = children.iter().map(|(&index, child)| {
            crate::grid_ipc::RendererProcess {
                index,
                pid: child.child.id(),
                spawn_epoch: child.comms.spawn_epoch.load(Ordering::SeqCst),
                ready: child.comms.is_connected() && child.document_ready
                    && !child.comms.blank_reclaiming.load(Ordering::SeqCst)
                    && child.comms.native_document.lock().ok()
                        .is_some_and(|stamp| stamp.is_some_and(|value| value.usable())),
                startup_reserved: child.spawned_at.elapsed() < crate::grid_ipc::RENDERER_STARTUP_RESERVE
                    || !child.document_ready,
            }
        }).collect();
        result.sort_unstable_by_key(|owner| owner.index);
        result
    }

    pub fn pids(&self) -> Vec<(u32, u32)> {
        self.children
            .lock()
            .unwrap()
            .iter()
            .filter(|(i, _)| !crate::grid_ipc::is_tab_renderer(**i))
            .map(|(i, h)| (*i, h.child.id()))
            .collect()
    }

    /// Ordinary close_grid is reversible and never closes application admission.
    /// All cells share this one bounded foreground wait; pending owners remain
    /// with the cleanup service and block only reuse of their own indices.
    pub fn shutdown_all(&self) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        self.lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))?;
        {
            let _gate = self.lifecycle_lock.lock().unwrap();
            self.lifecycle.check()?;
            self.crash_history.lock().unwrap().retain(|id, _| crate::grid_ipc::is_tab_renderer(*id));
            self.hibernated.lock().unwrap().retain(|id, _| crate::grid_ipc::is_tab_renderer(*id));
            self.pending_restarts.lock().unwrap().retain(|id, _| crate::grid_ipc::is_tab_renderer(*id));
            let mut children = self.children.lock().unwrap();
            let indices: Vec<_> = children.keys().copied()
                .filter(|id| !crate::grid_ipc::is_tab_renderer(*id)).collect();
            for index in indices {
                if let Some(owner) = children.remove(&index) {
                    self.lifecycle.retire(index, owner);
                }
            }
        }
        self.lifecycle
            .wait_grid_retired_until(std::time::Instant::now() + std::time::Duration::from_millis(750))
    }

    /// Irreversible application admission, distinct from closing a Grid view.
    pub fn request_shutdown(&self) {
        self.lifecycle.request_shutdown();
    }

    pub fn shutdown_until(&self, deadline: std::time::Instant) -> Result<(), String> {
        self.lifecycle.shutdown_until(
            Arc::clone(&self.children),
            Arc::clone(&self.listeners),
            deadline,
        )
    }
}

#[cfg(test)]
mod adaptive_grid_hibernation_tests {
    use super::{
        grid_child_is_hibernatable, grid_content_reclaimable, grid_hibernation_probe_safe,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn only_explicitly_hidden_unfocused_cells_are_hibernatable() {
        let now = Instant::now();
        let idle_since = Some(now - Duration::from_secs(60));
        let idle_for = Duration::from_secs(30);
        assert!(grid_child_is_hibernatable(
            true, false, false, idle_since, now, idle_for
        ));
        assert!(!grid_child_is_hibernatable(
            false, false, false, idle_since, now, idle_for
        ));
        assert!(!grid_child_is_hibernatable(
            true, true, false, idle_since, now, idle_for
        ));
        assert!(!grid_child_is_hibernatable(
            true, false, true, idle_since, now, idle_for
        ));
        assert!(!grid_child_is_hibernatable(
            true, false, false, None, now, idle_for
        ));
        assert!(!grid_child_is_hibernatable(
            true,
            false,
            false,
            Some(now - Duration::from_secs(5)),
            now,
            idle_for
        ));
    }

    #[test]
    fn external_ai_grids_are_not_discardable_without_state_checkpoint() {
        assert!(grid_content_reclaimable(Some("about:blank")));
        assert!(grid_hibernation_probe_safe(
            r#"{"safe":true,"url":"about:blank"}"#
        ));
        for probe in [
            r#"{"safe":true,"url":"https://chat.deepseek.com"}"#,
            r#"{"safe":false,"url":"about:blank"}"#,
            r#"{"safe":"true","url":"about:blank"}"#,
            "{}",
            "null",
            "malformed",
        ] {
            assert!(!grid_hibernation_probe_safe(probe));
        }
        for url in [
            None,
            Some("https://chat.deepseek.com"),
            Some("https://www.doubao.com"),
            Some("https://kimi.moonshot.cn"),
            Some("file:///home/user/unsaved.html"),
            Some("data:text/html,hello"),
        ] {
            assert!(!grid_content_reclaimable(url));
        }
    }
}

#[cfg(test)]
mod grid_process_termination_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn owner(child: Child) -> GridChildHandle {
        GridChildHandle {
            child,
            memory_scope: None,
            comms: Arc::new(ChildComms::new()),
            focused: false,
            spawned_at: std::time::Instant::now(),
            document_ready: false,
            navigation_revision: 0,
            last_url: None,
            last_rect: None,
            hidden: false,
            blur_hidden: false,
            native_window_visibility: None,
            hidden_since: None,
        }
    }

    #[test]
    fn killing_a_grid_child_has_a_bounded_foreground_wait() {
        let child = Command::new("/usr/bin/sleep")
            .arg("10")
            .process_group(0)
            .spawn()
            .unwrap();
        let mut retirement = GridRetirement::new(1, owner(child));
        let started = Instant::now();
        let mut complete = retirement.poll().unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        let deadline = started + Duration::from_secs(3);
        while !complete && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
            complete = retirement.poll().unwrap();
        }
        assert!(complete, "native leader/group cleanup must complete");
        assert!(retirement.child.is_none());
    }

    #[test]
    fn linux_grid_exit_observation_keeps_leader_waitable_until_retirement() {
        let child = Command::new("/usr/bin/true")
            .process_group(0)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if peek_grid_exit(&child).unwrap().is_some() {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(peek_grid_exit(&child).unwrap(), Some(0));
        assert_eq!(
            peek_grid_exit(&child).unwrap(),
            Some(0),
            "monitoring must not consume wait status or release the PID"
        );
        let mut retirement = GridRetirement::new(7, owner(child));
        assert!(retirement.poll().unwrap());
        assert!(retirement.child.is_none());
    }

    #[test]
    fn linux_grid_retirement_after_reap_only_observes_group() {
        let mut leader = Command::new("/usr/bin/sleep")
            .arg("10")
            .process_group(0)
            .spawn()
            .unwrap();
        let mut member = Command::new("/usr/bin/sleep")
            .arg("10")
            .process_group(leader.id() as i32)
            .spawn()
            .unwrap();
        leader.kill().unwrap();
        leader.wait().unwrap();
        let mut retirement = GridRetirement::new(8, owner(leader));
        retirement.identity_consumed = true;
        assert!(
            !retirement.poll().unwrap(),
            "a live group cannot be reported complete"
        );
        assert!(
            member.try_wait().unwrap().is_none(),
            "after consuming identity, cleanup must never signal numeric PGID"
        );
        member.kill().unwrap();
        member.wait().unwrap();
        assert!(retirement.poll().unwrap());
    }

    #[test]
    fn linux_grid_proc_group_parser_handles_non_utf8_and_parentheses() {
        assert_eq!(
            proc_stat_group(b"123 (name ) with\xff bytes) S 7 456 8 0 0"),
            Some(456)
        );
        assert_eq!(proc_stat_group(b"123 broken"), None);
    }

    #[test]
    fn linux_grid_view_close_does_not_latch_application_shutdown() {
        let manager = GridProcessManager::default();
        manager.shutdown_all().unwrap();
        assert!(manager.lifecycle.check().is_ok());
        manager.request_shutdown();
        assert!(manager.get_or_spawn(1).is_err());
        manager
            .shutdown_until(Instant::now() + Duration::from_secs(3))
            .unwrap();
        assert!(manager.lifecycle.check().is_err());
    }
}

#[cfg(test)]
mod grid_connection_generation_tests {
    use super::*;

    #[test]
    fn late_old_reader_must_not_disconnect_new_socket_or_cancel_new_requests() {
        let comms = ChildComms::new();
        let (old_socket, _old_peer) = UnixStream::pair().unwrap();
        let pid = std::process::id();
        comms.expected_pid.store(pid, Ordering::SeqCst);
        let old_generation = comms.attach(old_socket, pid).unwrap();
        assert!(!comms.is_current(old_generation));
        assert!(comms.mark_ready(old_generation));
        assert!(comms.is_current(old_generation));

        let (old_tx, old_rx) = mpsc::channel();
        comms.pending.lock().unwrap().insert(1, old_tx);
        let (new_socket, _new_peer) = UnixStream::pair().unwrap();
        let new_generation = comms.attach(new_socket, pid).unwrap();
        assert!(!comms.mark_ready(old_generation));
        assert!(comms.mark_ready(new_generation));
        assert_ne!(old_generation, new_generation);
        assert!(matches!(old_rx.try_recv(), Ok(Err(_))));

        // The old reader exits after the new child has connected.
        comms.detach_if_current(old_generation);
        assert!(comms.is_current(new_generation));
        assert!(comms.writer.lock().unwrap().is_some());

        let (new_tx, new_rx) = mpsc::channel();
        comms.pending.lock().unwrap().insert(2, new_tx);
        comms.detach_if_current(old_generation);
        assert!(matches!(new_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));

        // Only the active reader may close the socket and fail its requests.
        comms.detach_if_current(new_generation);
        assert!(!comms.is_connected());
        assert!(comms.writer.lock().unwrap().is_none());
        assert!(matches!(new_rx.try_recv(), Ok(Err(_))));
    }
    #[test]
    fn peer_admission_and_spawn_fence_reject_old_generation() {
        let comms = ChildComms::new();
        let pid = std::process::id();
        let (unowned, _) = UnixStream::pair().unwrap();
        assert!(comms.attach(unowned, pid).is_err());
        comms.expected_pid.store(pid, Ordering::SeqCst);
        let (wrong, _) = UnixStream::pair().unwrap();
        assert!(comms.attach(wrong, pid.wrapping_add(1)).is_err());
        let (valid, _peer) = UnixStream::pair().unwrap();
        let generation = comms.attach(valid, pid).unwrap();
        assert!(comms.mark_ready(generation));
        let (tx, rx) = mpsc::channel();
        comms.pending.lock().unwrap().insert(1, tx);
        comms.prepare_spawn().unwrap();
        assert!(!comms.is_current(generation));
        assert!(matches!(rx.try_recv(), Ok(Err(_))));
        let (stale, _) = UnixStream::pair().unwrap();
        assert!(comms.attach(stale, pid).is_err());
    }

    #[test]
    fn retirement_revokes_ready_wait_and_pending_response_without_writer() {
        let comms = ChildComms::new();
        let pid = std::process::id();
        comms.prepare_spawn().unwrap();
        comms.publish_owner(pid);
        let owner = comms.owner_snapshot().unwrap();
        let (socket, _peer) = UnixStream::pair().unwrap();
        let generation = comms.attach(socket, pid).unwrap();
        assert_eq!(comms.ready_for_owner(owner).unwrap(), None);
        assert!(comms.mark_ready(generation));
        assert_eq!(comms.ready_for_owner(owner).unwrap(), Some(generation));
        let (tx, rx) = mpsc::channel();
        comms.pending.lock().unwrap().insert(1, tx);
        let writer = comms.writer.lock().unwrap();
        comms.revoke_owner(pid);
        assert!(comms.ready_for_owner(owner).is_err());
        assert!(matches!(rx.try_recv(), Ok(Err(_))));
        comms.complete_response(generation, 1, Ok(None));
        assert!(!comms.is_current(generation));
        drop(writer);
        comms.prepare_spawn().unwrap();
        comms.publish_owner(pid);
        assert!(comms.ready_for_owner(owner).is_err());
    }

    #[test]
    fn linux_grid_shutdown_cancels_reader_without_waiting_for_writer_lock() {
        use std::time::Duration;
        let comms = ChildComms::new();
        let (socket, _peer) = UnixStream::pair().unwrap();
        let mut reader = socket.try_clone().unwrap();
        let pid = std::process::id();
        comms.expected_pid.store(pid, Ordering::SeqCst);
        comms.attach(socket, pid).unwrap();
        let (started, ready) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            started.send(()).unwrap();
            let mut byte = [0u8; 1];
            let result = reader.read(&mut byte);
            finished
                .send(result.map(|count| count == 0).unwrap_or(true))
                .unwrap();
        });
        ready.recv_timeout(Duration::from_secs(1)).unwrap();
        let writer = comms.writer.lock().unwrap();
        comms.request_shutdown();
        assert!(done.recv_timeout(Duration::from_secs(1)).unwrap());
        drop(writer);
        worker.join().unwrap();
        let (late, _late_peer) = UnixStream::pair().unwrap();
        assert!(comms.attach(late, pid).is_err());
        assert!(!comms.is_connected());
    }
}

#[cfg(test)]
mod grid_reposition_coalescing_tests {
    use super::*;

    #[test]
    fn dirty_geometry_is_replayed_after_a_slow_uds_write() {
        let manager = GridProcessManager::default();
        manager.reposition_running.store(true, Ordering::Release);
        manager.reposition_pending.store(10, Ordering::Release);
        // The GTK event lands before the background writer finishes.
        manager.reposition_pending.store(11, Ordering::Release);
        assert!(manager.finish_reposition_iteration(10));
        assert!(manager.reposition_running.load(Ordering::Acquire));
        // A late event must survive another in-flight iteration, too.
        manager.reposition_pending.store(12, Ordering::Release);
        assert!(manager.finish_reposition_iteration(11));
        assert!(!manager.finish_reposition_iteration(12));
        assert!(!manager.reposition_running.load(Ordering::Acquire));
    }

    #[test]
    fn a_new_gtk_event_can_claim_the_idle_worker_after_completion() {
        let manager = GridProcessManager::default();
        manager.reposition_running.store(true, Ordering::Release);
        manager.reposition_pending.store(1, Ordering::Release);
        assert!(!manager.finish_reposition_iteration(1));
        assert!(!manager.reposition_running.load(Ordering::Acquire));
        // Event callbacks claim at most one worker; subsequent requests are
        // reduced to a monotonically increasing latest-value counter.
        assert!(!manager.reposition_running.swap(true, Ordering::AcqRel));
        manager.reposition_pending.fetch_add(1, Ordering::AcqRel);
        assert!(manager.reposition_running.swap(true, Ordering::AcqRel));
        manager.reposition_pending.fetch_add(1, Ordering::AcqRel);
        assert_eq!(manager.reposition_pending.load(Ordering::Acquire), 3);
    }

    #[test]
    fn gtk_queue_writes_no_grid_socket_without_an_app() {
        let manager = GridProcessManager::default();
        for _ in 0..1_000 {
            manager.request_reposition_async();
        }
        assert!(!manager.reposition_running.load(Ordering::Acquire));
        assert_eq!(manager.reposition_pending.load(Ordering::Acquire), 1_000);
    }
}
