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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    connected: AtomicBool,
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
            connected: AtomicBool::new(false),
        }
    }

    fn attach(&self, stream: UnixStream) -> Result<u64, String> {
        let control = stream
            .try_clone()
            .map_err(|_| "GRID_CONTROL_CLONE_FAILED")?;
        // Never retain control while waiting for an active writer. Shutdown
        // needs only the short control lock to interrupt that writer.
        let mut writer = self.writer.lock().unwrap();
        let mut slot = self.control.lock().unwrap();
        if self.closing.load(Ordering::SeqCst) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return Err("GRID_CONNECTION_CLOSING".into());
        }
        if let Some(old) = slot.replace(control) {
            let _ = old.shutdown(std::net::Shutdown::Both);
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *writer = Some(stream);
        self.connected.store(true, Ordering::SeqCst);
        drop(slot);
        self.fail_pending("子进程连接已更换");
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
                let _ = tx.send(Err("GRID_CONNECTION_CLOSING".into()));
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
        let mut writer = self.writer.lock().unwrap();
        if self.generation.load(Ordering::SeqCst) != generation {
            return;
        }
        if let Some(control) = self.control.lock().unwrap().take() {
            let _ = control.shutdown(std::net::Shutdown::Both);
        }
        *writer = None;
        self.connected.store(false, Ordering::SeqCst);
        self.fail_pending("子进程连接断开");
    }

    fn is_current(&self, generation: u64) -> bool {
        !self.closing.load(Ordering::SeqCst)
            && self.connected.load(Ordering::SeqCst)
            && self.generation.load(Ordering::SeqCst) == generation
    }

    fn is_connected(&self) -> bool {
        !self.closing.load(Ordering::SeqCst) && self.connected.load(Ordering::SeqCst)
    }
}
/// 崩溃重启时跨 spawn 保留的状态。
#[derive(Default, Clone)]
struct SavedChildState {
    last_url: Option<String>,
    last_rect: Option<(f64, f64, f64, f64)>,
    hidden: bool,
    blur_hidden: bool,
}

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
    child: Option<GridChildHandle>,
    identity_lost: bool,
    // Once reaped, numeric PGID observations are read-only forever. Never
    // signal a potentially reused PID/group when a late member is observed.
    identity_consumed: bool,
}
impl GridRetirement {
    fn new(index: u32, child: GridChildHandle) -> Self {
        Self {
            index,
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

pub struct GridChildHandle {
    // index is the key in GridProcessManager::children;
    // communication state is retained in GridProcessManager::listeners.
    pub child: Child,
    /// 最近一次导航 url（崩溃重启重放用）
    pub last_url: Option<String>,
    /// 最近一次定位（CSS 坐标，相对主窗内容区；崩溃重启/主窗移动重放用）
    pub last_rect: Option<(f64, f64, f64, f64)>,
    /// 显式隐藏（切视图/全部隐藏）：定位/失焦恢复时不应重新显示
    pub hidden: bool,
    /// 主窗失焦导致的隐藏（主窗重新聚焦时恢复）
    pub blur_hidden: bool,
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
    /// 子进程窗口焦点状态（子进程经 UDS Event 上报）：主窗 blur 时据此区分
    /// "用户点了宫格"（不隐藏）与"切到其它应用"（隐藏防幽灵浮层）
    child_focused: Mutex<HashMap<u32, bool>>,
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
        if self.visibility.request(mode, delay) {
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
                    {
                        // Relocation also sends UpdateRect (which shows the
                        // native window), so it shares this publication order.
                        let _dispatch = manager
                            .visibility_dispatch
                            .lock()
                            .unwrap_or_else(|p| p.into_inner());
                        match intent.mode {
                            VisibilityMode::Show => manager.apply_show_for_focus(),
                            VisibilityMode::Blur => manager.apply_hide_for_blur_if_no_child_focus(),
                            VisibilityMode::Minimized => manager.apply_hide_for_blur(),
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
                        let generation = match comms_accept.attach(stream) {
                            Ok(generation) => generation,
                            Err(_) => continue,
                        };
                        // 每连接一个 reader 线程：Response 按 seq 路由，Event 转发 app.emit。
                        let comms_reader = comms_accept.clone();
                        let app_reader = app.clone();
                        let reader_owner = Arc::clone(&accept_owner);
                        let started_reader = accept_owner.spawn("grid-reader", move || {
                            let mut r = BufReader::new(reader);
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
                                        let tx = comms_reader.pending.lock().unwrap().remove(&seq);
                                        if let Some(tx) = tx {
                                            let _ = tx.send(if ok {
                                                Ok(data)
                                            } else {
                                                Err(err.unwrap_or_else(|| "子进程执行失败".into()))
                                            });
                                        }
                                    }
                                    Ok(Some(Wire::Event { name, payload })) => {
                                        if let Some(app) = &app_reader {
                                            match name.as_str() {
                                                // 内部事件：子窗口焦点上报，不转发前端
                                                "grid-child-focus" => {
                                                    let state =
                                                        app.state::<crate::bridge::AppState>();
                                                    state.grid_manager.set_child_focus(index, true);
                                                }
                                                "grid-child-blur" => {
                                                    let state =
                                                        app.state::<crate::bridge::AppState>();
                                                    state
                                                        .grid_manager
                                                        .set_child_focus(index, false);
                                                    // 从宫格切到其它应用时主窗不会再次 blur，
                                                    // 子窗口 blur 后若主窗也无焦点 → 隐藏防幽灵浮层
                                                    state
                                                        .grid_manager
                                                        .request_hide_for_blur_async();
                                                }
                                                "tab-navigated" => {
                                                    // Keep the native memory guard in sync with
                                                    // same-WebView redirects and in-page links.
                                                    // Only accept navigation for this Grid child.
                                                    if let (Some(id), Some(url)) = (
                                                        payload.get("id").and_then(|v| v.as_str()),
                                                        payload.get("url").and_then(|v| v.as_str()),
                                                    ) {
                                                        if id == format!("grid-{index}")
                                                            && !url.is_empty()
                                                        {
                                                            app.state::<crate::bridge::AppState>()
                                                                .grid_manager
                                                                .record_url(index, url);
                                                        }
                                                    }
                                                    let _ = app.emit(&name, payload);
                                                }
                                                _ => {
                                                    let _ = app.emit(&name, payload);
                                                }
                                            }
                                        }
                                    }
                                    Ok(Some(Wire::Request { .. })) => {
                                        // 主进程不会收到 Request，忽略
                                    }
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
        self.ensure_listener(index)?;
        let exe = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        let child = Command::new(exe)
            .arg("--grid-child")
            .arg(index.to_string())
            .env("GRID_CHILD_INDEX", index.to_string())
            .env("GRID_SOCK_PATH", Self::socket_path(index))
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("spawn 宫格子进程 grid-{index} 失败: {e}"))?;
        let pid = child.id();
        let saved = saved.unwrap_or_default();
        let handle = GridChildHandle {
            child,
            last_url: saved.last_url,
            last_rect: saved.last_rect,
            hidden: saved.hidden,
            blur_hidden: saved.blur_hidden,
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
                id: format!("grid-{index}"),
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
    fn wait_connected(&self, index: u32, timeout_ms: u64) -> Result<Arc<ChildComms>, String> {
        self.lifecycle.check()?;
        let comms = self
            .listeners
            .lock()
            .unwrap()
            .get(&index)
            .cloned()
            .ok_or_else(|| format!("grid-{index} 无 UDS listener（未 spawn？）"))?;
        let start = std::time::Instant::now();
        while !comms.is_connected() {
            self.lifecycle.check()?;
            if comms.closing.load(Ordering::SeqCst) {
                return Err("GRID_CONNECTION_CLOSING".into());
            }
            if start.elapsed().as_millis() as u64 > timeout_ms {
                return Err(format!("等待 grid-child-{index} UDS 连接超时"));
            }
            if self
                .lifecycle
                .wait_or_stop(std::time::Duration::from_millis(50))
            {
                return Err("GRID_SHUTDOWN_IN_PROGRESS".into());
            }
        }
        Ok(comms)
    }

    /// 同步请求：等连接 → 写 Request → 等 Response（带超时）。
    /// 失败（连接断/超时）时等重连后重试一次（覆盖崩溃重启窗口期）。
    pub fn request(&self, index: u32, cmd: GridCmd, timeout_ms: u64) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        match self.request_once(index, &cmd, timeout_ms) {
            Ok(()) => Ok(()),
            // 超时不代表命令未执行；同一连接重放可能重复创建/导航，且把等待翻倍。
            Err(first) if self.lifecycle.is_stopping() || first.contains("超时") => Err(first),
            Err(first) => {
                eprintln!(
                    "[grid-manager] grid-{} 请求失败({})，等重连重试",
                    index, first
                );
                let comms = self.wait_connected(index, 8000)?;
                self.request_once_with(comms, &cmd, timeout_ms).map(|_| ())
            }
        }
    }

    fn request_once(&self, index: u32, cmd: &GridCmd, timeout_ms: u64) -> Result<(), String> {
        let comms = self.wait_connected(index, 15000)?;
        self.request_once_with(comms, cmd, timeout_ms).map(|_| ())
    }

    /// Fail-closed native probe of the actual Grid page. Stored last_url is
    /// only a navigation target and cannot prove the live DOM is empty.
    fn confirmed_blank_grid(&self, index: u32) -> bool {
        let Ok(comms) = self.wait_connected(index, 500) else {
            return false;
        };
        match self.request_once_with(
            comms,
            &GridCmd::HibernationProbe {
                id: format!("grid-{index}"),
            },
            1000,
        ) {
            Ok(Some(encoded)) => grid_hibernation_probe_safe(&encoded),
            _ => false,
        }
    }

    pub fn read_replies(&self, index: u32) -> Result<String, String> {
        let _operation = self.lifecycle.enter()?;
        if !self.children.lock().unwrap().contains_key(&index) {
            return Err("GRID_NOT_OPEN".into());
        }
        let comms = self.wait_connected(index, 500)?;
        self.request_once_with(
            comms,
            &GridCmd::ReadReplies {
                id: format!("grid-{index}"),
            },
            5000,
        )?
        .ok_or_else(|| "GRID_NO_RESULT".into())
    }

    fn request_once_with(
        &self,
        comms: Arc<ChildComms>,
        cmd: &GridCmd,
        timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        self.lifecycle.check()?;
        if comms.closing.load(Ordering::SeqCst) {
            return Err("GRID_CONNECTION_CLOSING".into());
        }
        let seq = comms.seq.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        let write_res = {
            let mut guard = comms.writer.lock().unwrap();
            self.lifecycle.check()?;
            if comms.closing.load(Ordering::SeqCst) {
                return Err("GRID_CONNECTION_CLOSING".into());
            }
            match guard.as_mut() {
                Some(w) => {
                    // 与 attach 的 writer -> pending 锁序一致：请求只登记到当前连接。
                    comms.pending.lock().unwrap().insert(seq, tx);
                    crate::grid_ipc::write_wire(
                        w,
                        &Wire::Request {
                            seq,
                            cmd: cmd.clone(),
                        },
                    )
                }
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "未连接",
                )),
            }
        };
        if let Err(e) = write_res {
            comms.pending.lock().unwrap().remove(&seq);
            return Err(format!("写 UDS 失败: {e}"));
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            if self.lifecycle.is_stopping() || comms.closing.load(Ordering::SeqCst) {
                comms.pending.lock().unwrap().remove(&seq);
                return Err("GRID_SHUTDOWN_IN_PROGRESS".into());
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                comms.pending.lock().unwrap().remove(&seq);
                return Err("等待子进程响应超时".into());
            }
            match rx.recv_timeout(remaining.min(std::time::Duration::from_millis(100))) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    comms.pending.lock().unwrap().remove(&seq);
                    return Err("GRID_CONNECTION_CLOSED".into());
                }
            }
        }
    }

    /// 发后即忘（高频定位/显隐），错误只记日志。
    pub fn send(&self, index: u32, cmd: GridCmd) {
        let Ok(_operation) = self.lifecycle.enter() else {
            return;
        };
        let comms = match self.listeners.lock().unwrap().get(&index).cloned() {
            Some(c) => c,
            None => return,
        };
        let mut guard = comms.writer.lock().unwrap();
        if self.lifecycle.is_stopping() || comms.closing.load(Ordering::SeqCst) {
            return;
        }
        if let Some(w) = guard.as_mut() {
            let seq = comms.seq.fetch_add(1, Ordering::SeqCst);
            if let Err(e) = crate::grid_ipc::write_wire(w, &Wire::Request { seq, cmd }) {
                eprintln!("[grid-manager] grid-{} send 失败: {e}", index);
            }
        }
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
        let focused = self.child_focused.lock().unwrap().clone();
        let mut candidates: Vec<(std::time::Instant, u32)> = self
            .children
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(index, child)| {
                if !grid_child_is_hibernatable(
                    child.hidden,
                    child.blur_hidden,
                    focused.get(index).copied().unwrap_or(false),
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
            let eligible = || {
                !self
                    .child_focused
                    .lock()
                    .unwrap()
                    .get(&index)
                    .copied()
                    .unwrap_or(false)
                    && self
                        .children
                        .lock()
                        .unwrap()
                        .get(&index)
                        .is_some_and(|child| {
                            child.hidden_since == Some(hidden_since)
                                && grid_content_reclaimable(child.last_url.as_deref())
                                && grid_child_is_hibernatable(
                                    child.hidden,
                                    child.blur_hidden,
                                    false,
                                    child.hidden_since,
                                    std::time::Instant::now(),
                                    idle_for,
                                )
                        })
            };
            if !eligible() || !self.confirmed_blank_grid(index) || !eligible() {
                continue;
            }
            // Graceful native close for the empty WebView; if a user resumes
            // it during IPC, do not kill the process. Rebuild a blank WebView
            // when CloseTab won the race, preserving the user's fresh layout.
            let closed = self.request(
                index,
                GridCmd::CloseTab {
                    id: format!("grid-{index}"),
                },
                1000,
            );
            if !eligible() {
                if closed.is_ok() {
                    self.replay(index);
                }
                continue;
            }
            // Revalidate under the child map lock, then kill the old process
            // before another get_or_spawn can reuse the same Grid index/socket.
            let focused_now = self
                .child_focused
                .lock()
                .unwrap()
                .get(&index)
                .copied()
                .unwrap_or(false);
            if focused_now {
                if closed.is_ok() {
                    self.replay(index);
                }
                continue;
            }
            let _gate = self.lifecycle_lock.lock().unwrap();
            if self.lifecycle.is_stopping() {
                break;
            }
            let mut children = self.children.lock().unwrap();
            let still_eligible = children.get(&index).is_some_and(|child| {
                child.hidden_since == Some(hidden_since)
                    && grid_content_reclaimable(child.last_url.as_deref())
                    && grid_child_is_hibernatable(
                        child.hidden,
                        child.blur_hidden,
                        false,
                        child.hidden_since,
                        std::time::Instant::now(),
                        idle_for,
                    )
            });
            if !still_eligible {
                drop(children);
                if closed.is_ok() {
                    self.replay(index);
                }
                continue;
            }
            if let Some(child) = children.remove(&index) {
                // Publish retirement before releasing the map/gate. Activation
                // rejects this index until the complete process group is gone.
                self.hibernated.lock().unwrap().insert(
                    index,
                    SavedChildState {
                        last_url: child.last_url.clone(),
                        last_rect: child.last_rect,
                        hidden: true,
                        blur_hidden: false,
                    },
                );
                self.pending_restarts.lock().unwrap().remove(&index);
                self.lifecycle.retire(index, child);
                hibernated.push(index);
                eprintln!("[grid-manager] hibernated empty grid-child-{}", index);
            }
        }
        hibernated
    }

    pub fn hibernated_count(&self) -> usize {
        self.hibernated.lock().unwrap().len()
    }

    /// 当前存活子进程 index 列表。
    pub fn indices(&self) -> Vec<u32> {
        self.children.lock().unwrap().keys().cloned().collect()
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
        let _dispatch = self
            .visibility_dispatch
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let items: Vec<(u32, (f64, f64, f64, f64))> = self
            .children
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, h)| !h.hidden && !h.blur_hidden)
            .filter_map(|(i, h)| h.last_rect.map(|r| (*i, r)))
            .collect();
        for (index, css) in items {
            if let Some(rect) = self.abs_rect(css) {
                self.send(
                    index,
                    GridCmd::UpdateRect {
                        id: format!("grid-{index}"),
                        rect,
                    },
                );
            }
        }
    }

    /// 记录子进程窗口焦点状态（子进程经 UDS Event 上报）。
    pub fn set_child_focus(&self, index: u32, focused: bool) {
        self.child_focused.lock().unwrap().insert(index, focused);
    }

    fn any_child_focused(&self) -> bool {
        self.child_focused.lock().unwrap().values().any(|f| *f)
    }

    /// 主窗失焦（或子窗口失焦）后的统一入口：若焦点在任一宫格子窗口内则不动，
    /// 否则主窗也无焦点时才隐藏（用户点了宫格 ≠ 切走应用）。
    pub fn hide_for_blur_if_no_child_focus(&self) {
        self.request_hide_for_blur_async();
    }

    fn apply_hide_for_blur_if_no_child_focus(&self) {
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
        self.apply_hide_for_blur();
    }

    /// 主窗失焦：隐藏所有可见宫格子进程窗口（标记 blur_hidden，聚焦时恢复）。
    pub fn hide_for_blur(&self) {
        self.request_visibility(VisibilityMode::Minimized, std::time::Duration::ZERO);
    }

    fn apply_hide_for_blur(&self) {
        let indices = self.indices();
        for index in indices {
            let need = {
                let mut children = self.children.lock().unwrap();
                match children.get_mut(&index) {
                    Some(h) if !h.hidden && !h.blur_hidden && h.last_rect.is_some() => {
                        h.blur_hidden = true;
                        true
                    }
                    _ => false,
                }
            };
            if need {
                self.send(
                    index,
                    GridCmd::HideWindow {
                        id: format!("grid-{index}"),
                    },
                );
            }
        }
    }

    /// 主窗重新聚焦：恢复因失焦隐藏的宫格（显式 hidden 的不动）。
    pub fn show_for_focus(&self) {
        self.request_show_for_focus_async();
    }

    fn apply_show_for_focus(&self) {
        let items: Vec<(u32, (f64, f64, f64, f64))> = {
            let mut children = self.children.lock().unwrap();
            children
                .iter_mut()
                .filter(|(_, h)| h.blur_hidden && !h.hidden)
                .filter_map(|(i, h)| {
                    h.blur_hidden = false;
                    h.last_rect.map(|r| (*i, r))
                })
                .collect()
        };
        for (index, css) in items {
            if let Some(rect) = self.abs_rect(css) {
                self.send(
                    index,
                    GridCmd::UpdateRect {
                        id: format!("grid-{index}"),
                        rect,
                    },
                );
            }
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
                            let observed = children.get(&index).map(|owner| peek_grid_exit(&owner.child));
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
                    replay.push(index);
                }
            }
        }
        for index in replay {
            self.replay(index);
        }
    }

    /// 重启后状态重放：等 UDS 重连 → 重建 webview（url）→ 恢复定位（若可见）→ 恢复缩放。
    fn replay(&self, index: u32) {
        let (url, rect, visible) = {
            let children = self.children.lock().unwrap();
            match children.get(&index) {
                Some(h) => (h.last_url.clone(), h.last_rect, !h.hidden && !h.blur_hidden),
                None => return,
            }
        };
        let zoom = self
            .app()
            .map(|app| {
                app.state::<crate::bridge::AppState>()
                    .grid_zooms
                    .lock()
                    .unwrap()
                    .get(&format!("grid-{index}"))
                    .copied()
                    .unwrap_or(1.0)
            })
            .unwrap_or(1.0);
        let Some(url) = url else {
            eprintln!("[grid-manager] grid-child-{} 无历史 url，跳过重放", index);
            return;
        };
        // 重放等待连接可能数秒，放独立线程避免阻塞监控循环。
        let app = self.app();
        let replay_owner = Arc::clone(&self.lifecycle);
        let started = self.spawn_worker("grid-replay", move || {
            if replay_owner.is_stopping() {
                return;
            }
            let Some(app) = app else { return };
            let state = app.state::<crate::bridge::AppState>();
            let manager = &state.grid_manager;
            let label = format!("grid-{index}");
            if let Err(e) = manager.request(
                index,
                GridCmd::CreateTab {
                    id: label.clone(),
                    url: url.clone(),
                },
                15000,
            ) {
                eprintln!(
                    "[grid-manager] grid-child-{} 重放 CreateTab 失败: {e}",
                    index
                );
                return;
            }
            if visible {
                if let Some(css) = rect {
                    if let Some(abs) = manager.abs_rect(css) {
                        manager.send(
                            index,
                            GridCmd::UpdateRect {
                                id: label.clone(),
                                rect: abs,
                            },
                        );
                    }
                }
            }
            if zoom > 0.1 && (zoom - 1.0).abs() > 0.01 {
                manager.send(index, GridCmd::SetZoom { id: label, zoom });
            }
            eprintln!(
                "[grid-manager] grid-child-{} 状态重放完成 url={}",
                index, url
            );
        });
        if let Err(error) = started {
            eprintln!("[grid-manager] replay unavailable: {error}");
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
        self.children.lock().unwrap().len()
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
    pub fn pids(&self) -> Vec<(u32, u32)> {
        self.children
            .lock()
            .unwrap()
            .iter()
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
            self.crash_history.lock().unwrap().clear();
            self.hibernated.lock().unwrap().clear();
            self.pending_restarts.lock().unwrap().clear();
            let mut children = self.children.lock().unwrap();
            for (index, owner) in children.drain() {
                self.lifecycle.retire(index, owner);
            }
        }
        self.lifecycle
            .wait_retired_until(std::time::Instant::now() + std::time::Duration::from_millis(750))
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
            last_url: None,
            last_rect: None,
            hidden: false,
            blur_hidden: false,
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
        let old_generation = comms.attach(old_socket).unwrap();
        assert!(comms.is_current(old_generation));

        let (old_tx, old_rx) = mpsc::channel();
        comms.pending.lock().unwrap().insert(1, old_tx);
        let (new_socket, _new_peer) = UnixStream::pair().unwrap();
        let new_generation = comms.attach(new_socket).unwrap();
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
    fn linux_grid_shutdown_cancels_reader_without_waiting_for_writer_lock() {
        use std::time::Duration;
        let comms = ChildComms::new();
        let (socket, _peer) = UnixStream::pair().unwrap();
        let mut reader = socket.try_clone().unwrap();
        comms.attach(socket).unwrap();
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
        assert!(comms.attach(late).is_err());
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
