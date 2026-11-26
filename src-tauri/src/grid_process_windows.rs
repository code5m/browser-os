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

use crate::windows_pipe::{WindowsPipeListener as UnixListener, WindowsPipeStream as UnixStream};
use crate::windows_process::JobObject;
use std::collections::{HashMap, VecDeque};
use std::io::BufReader;
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
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<Option<String>, String>>>>,
    seq: AtomicU64,
    generation: AtomicU64,
    // Published only after the exact spawned Child has entered the owner map.
    expected_pid: AtomicU32,
    owner_gate: Mutex<()>,
    spawn_epoch: AtomicU64,
    ready_generation: AtomicU64,
    connected: AtomicBool,
    closing: AtomicBool,
    cancellation: Mutex<Option<UnixStream>>,
    listener: Mutex<Option<Arc<UnixListener>>>,
}

impl ChildComms {
    fn new() -> Self {
        Self {
            writer: Mutex::new(None),
            pending: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(1),
            generation: AtomicU64::new(0),
            expected_pid: AtomicU32::new(0),
            owner_gate: Mutex::new(()),
            spawn_epoch: AtomicU64::new(0),
            ready_generation: AtomicU64::new(0),
            connected: AtomicBool::new(false),
            closing: AtomicBool::new(false),
            cancellation: Mutex::new(None),
            listener: Mutex::new(None),
        }
    }

    /// 每次 UDS 重连分配新的代际，防止旧 reader 迟到的 EOF 清掉新 writer。
    /// 保持 writer -> pending 的锁序；新连接建立时立即唤醒旧连接请求。
    /// Invalidate any old connection before a replacement process can be
    /// launched. Do not recycle request identities across process generations.
    fn prepare_spawn(&self) -> Result<(), String> {
        let mut writer = self.writer.lock().unwrap();
        let _owner = self.owner_gate.lock().unwrap();
        self.expected_pid.store(0, Ordering::SeqCst);
        self.ready_generation.store(0, Ordering::SeqCst);
        self.connected.store(false, Ordering::SeqCst);
        if let Some(control) = self.cancellation.lock().unwrap().take() {
            let _ = control.cancel_io();
        }
        *writer = None;
        self.fail_pending("GRID_REQUEST_OUTCOME_UNKNOWN: previous renderer retired");
        self.generation
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
            .map_err(|_| "GRID_CONNECTION_GENERATION_EXHAUSTED".to_string())?;
        self.spawn_epoch
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
            .map_err(|_| "GRID_SPAWN_EPOCH_EXHAUSTED".to_string())?;
        Ok(())
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
        if let Some(control) = self.cancellation.lock().unwrap().as_ref() {
            let _ = control.cancel_io();
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
        let control = stream.try_clone().map_err(|error| error.to_string())?;
        let mut writer = self.writer.lock().unwrap();
        let _owner = self.owner_gate.lock().unwrap();
        if !self.accepts_peer(peer_pid) {
            let _ = control.cancel_io();
            return Err("GRID_PEER_NOT_OWNED".into());
        }
        if let Some(previous) = self.cancellation.lock().unwrap().replace(control) {
            let _ = previous.cancel_io();
        }
        let generation = self.generation
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
            .map_err(|_| "GRID_CONNECTION_GENERATION_EXHAUSTED".to_string())? + 1;
        *writer = Some(stream);
        self.ready_generation.store(0, Ordering::SeqCst);
        self.connected.store(true, Ordering::SeqCst);
        self.fail_pending("GRID_REQUEST_OUTCOME_UNKNOWN: renderer connection replaced");
        Ok(generation)
    }

    /// Independent control handles can interrupt overlapped I/O even when a
    /// request owns the writer mutex. Repeated cancellation closes issue races.
    fn request_shutdown(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.connected.store(false, Ordering::SeqCst);
        if let Ok(listener) = self.listener.try_lock() {
            if let Some(listener) = listener.as_ref() {
                listener.cancel();
            }
        }
        if let Ok(control) = self.cancellation.try_lock() {
            if let Some(control) = control.as_ref() {
                let _ = control.cancel_io();
            }
        }
        if let Ok(mut pending) = self.pending.try_lock() {
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err("GRID_SHUTDOWN_IN_PROGRESS".into()));
            }
        }
        if let Ok(mut writer) = self.writer.try_lock() {
            writer.take();
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
        let _owner = self.owner_gate.lock().unwrap();
        if self.generation.load(Ordering::SeqCst) != generation {
            return;
        }
        if let Some(control) = self.cancellation.lock().unwrap().take() {
            let _ = control.cancel_io();
        }
        *writer = None;
        self.ready_generation.store(0, Ordering::SeqCst);
        self.connected.store(false, Ordering::SeqCst);
        self.fail_pending("GRID_REQUEST_OUTCOME_UNKNOWN: renderer connection closed");
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
#[derive(Default, Clone)]
struct SavedChildState {
    last_url: Option<String>,
    last_rect: Option<(f64, f64, f64, f64)>,
    hidden: bool,
    blur_hidden: bool,
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
    comms: Arc<ChildComms>,
    focused: bool,
    pub job: JobObject,
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
    // Serialize process creation/restart/close so two concurrent tab requests
    // cannot spawn duplicate Grid children for the same index.
    lifecycle_lock: Mutex<()>,
    lifecycle: Arc<GridLifecycle>,
    children: Arc<Mutex<HashMap<u32, GridChildHandle>>>,
    /// Hibernated children retain URL/rect/visibility metadata but no WebKit process.
    hibernated: Mutex<HashMap<u32, SavedChildState>>,
    /// 已绑定 listener 的 index（accept 循环线程常驻，重启复用同一 socket）
    listeners: Arc<Mutex<HashMap<u32, Arc<ChildComms>>>>,
    app: Mutex<Option<AppHandle>>,
    monitor_started: AtomicBool,
    /// Crash budget per grid cell; normal user-triggered shutdown is not counted.
    crash_history: Mutex<HashMap<u32, VecDeque<std::time::Instant>>>,
    pending_restarts: Mutex<HashMap<u32, (std::time::Instant, SavedChildState)>>,
    // One coalescing relocation worker, regardless of GTK move/resize rate.
    // The version is a latest-value notification, never an unbounded channel.
    reposition_pending: AtomicU64,
    reposition_running: AtomicBool,
    visibility: LatestVisibility,
    visibility_dispatch: Mutex<()>,
}

impl GridProcessManager {
    fn ensure_cleanup(&self) -> Result<(), String> {
        self.lifecycle
            .ensure_cleanup(Arc::clone(&self.children), Arc::clone(&self.listeners))
    }

    fn spawn_worker(&self, name: &str, work: impl FnOnce() + Send + 'static) -> Result<(), String> {
        self.ensure_cleanup()?;
        self.lifecycle.spawn(name, work)
    }

    /// Application exit only. Ordinary close_grid never closes this gate.
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

    /// 主进程 setup 时注入 AppHandle（事件转发/重放/定位换算用）。
    pub fn set_app(&self, app: AppHandle) {
        *self.app.lock().unwrap() = Some(app);
    }

    fn app(&self) -> Option<AppHandle> {
        self.app.lock().unwrap().clone()
    }

    fn socket_path(index: u32) -> PathBuf {
        // An unpredictable session-scoped endpoint plus a per-user ACL and
        // a kernel-verified client PID prevent cross-process command forgery.
        static INSTANCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        let instance = INSTANCE.get_or_init(|| uuid::Uuid::new_v4().simple().to_string());
        PathBuf::from(format!(
            r"\\.\pipe\BrowserOS-grid-{}-{}-{}",
            std::process::id(),
            instance,
            index
        ))
    }

    /// 确保该 index 的 UDS listener 已绑定且 accept 循环线程在跑（幂等）。
    /// 返回该 index 的共享 comms。
    fn ensure_listener(&self, index: u32) -> Result<Arc<ChildComms>, String> {
        self.lifecycle.check()?;
        self.ensure_cleanup()?;
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
        let path = Self::socket_path(index);
        let listener = Arc::new(
            UnixListener::bind(&path)
                .map_err(|e| format!("绑定 UDS {} 失败: {e}", path.display()))?,
        );
        let comms = Arc::new(ChildComms::new());
        *comms.listener.lock().unwrap() = Some(Arc::clone(&listener));
        listeners.insert(index, comms.clone());
        drop(listeners);

        // accept 循环线程：子进程（含崩溃重启后的新实例）连接到同一 listener。
        let comms_accept = comms.clone();
        let app = self.app();
        let accept_lifecycle = Arc::clone(&self.lifecycle);
        let result = self.spawn_worker("grid-accept", move || loop {
            if accept_lifecycle.is_stopping() || comms_accept.closing.load(Ordering::SeqCst) {
                listener.cancel();
                break;
            }
            match listener.accept() {
                Ok((stream, _)) => {
                    if accept_lifecycle.is_stopping() || comms_accept.closing.load(Ordering::SeqCst) {
                        let _ = stream.cancel_io();
                        break;
                    }
                    let Ok(peer_pid) = stream.peer_pid() else {
                        continue;
                    };
                    let start = std::time::Instant::now();
                    while comms_accept.expected_pid.load(Ordering::SeqCst) == 0
                        && !accept_lifecycle.is_stopping()
                        && start.elapsed() < std::time::Duration::from_millis(500)
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    if !comms_accept.accepts_peer(peer_pid) {
                        eprintln!("[grid-manager] grid-child-{index} pipe peer refused");
                        continue;
                    }
                    eprintln!("[grid-manager] grid-child-{index} named pipe connected");
                    let reader = match stream.try_clone() {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("[grid-manager] grid-child-{} try_clone 失败: {e}", index);
                            continue;
                        }
                    };
                    let generation = match comms_accept.attach(stream, peer_pid) {
                        Ok(generation) => generation,
                        Err(_) => continue,
                    };
                    // 每连接一个 reader 线程：Response 按 seq 路由，Event 转发 app.emit。
                    let comms_reader = comms_accept.clone();
                    let app_reader = app.clone();
                    let reader_lifecycle = Arc::clone(&accept_lifecycle);
                    let reader_spawn = accept_lifecycle.spawn("grid-reader", move || {
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
                            if reader_lifecycle.is_stopping()
                                || !comms_reader.is_current(generation)
                            {
                                break;
                            }
                            let frame = crate::grid_ipc::read_wire(&mut r);
                            // 旧连接仍有缓冲帧时，不允许覆盖新连接的响应/焦点状态。
                            if !comms_reader.is_current(generation) {
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
                                            _ if comms_reader.is_current(generation) => { let _ = app.emit(&name, payload); }
                                            _ => break,
                                        }
                                    }
                                }
                                Ok(Some(Wire::Request { .. })) => break,
                                Ok(None) => {
                                    eprintln!("[grid-manager] grid-child-{} UDS 对端关闭", index);
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
                    if let Err(error) = reader_spawn {
                        comms_accept.detach_if_current(generation);
                        eprintln!("[grid-manager] reader unavailable: {error}");
                    }
                }
                Err(e) => {
                    if accept_lifecycle.is_stopping() {
                        break;
                    }
                    eprintln!("[grid-manager] grid-child-{} accept 失败: {e}", index);
                    if accept_lifecycle.wait_or_stop(std::time::Duration::from_millis(200)) {
                        break;
                    }
                }
            }
        });
        if let Err(error) = result {
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
        let _lifecycle = self.lifecycle_lock.lock().unwrap();
        self.spawn_with_state_locked(index, saved)
    }

    fn spawn_with_state_locked(
        &self,
        index: u32,
        saved: Option<SavedChildState>,
    ) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        self.ensure_cleanup()?;
        if self.lifecycle.is_retiring(index) {
            return Err(format!("GRID_CHILD_CLEANUP_PENDING:{index}"));
        }
        if self.children.lock().unwrap().contains_key(&index) {
            return Err(format!("GRID_CHILD_ALREADY_RUNNING:{index}"));
        }
        let profile_epoch = crate::browser_profile_owner::grid_epoch()?;
        let comms = self.ensure_listener(index)?;
        comms.prepare_spawn()?;
        let exe = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        let mut command = Command::new(exe);
        command
            .arg("--grid-child")
            .arg(index.to_string())
            // 继承父进程环境（WEBKIT_DISABLE_DMABUF_RENDERER / GDK_BACKEND 等在
            // main.rs 已 set_var，子进程经环境继承获得）；额外注入子进程索引
            // （crashlog 角色区分）与 UDS socket 路径。
            .env("GRID_CHILD_INDEX", index.to_string())
            .env(crate::browser_profile_owner::GRID_EPOCH_ENV, profile_epoch)
            .env("GRID_PARENT_PID", std::process::id().to_string())
            .env("GRID_SOCK_PATH", Self::socket_path(index))
            // Windows Job Object owns the entire WebView2 process subtree.
            // 子进程 stdio 继承主进程（日志汇入同一 stderr，便于 crashlog 镜像）。
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(crate::windows_process::CREATE_SUSPENDED);
        }
        // Construct the Job first; assignment failure must retain both the
        // suspended Child and its partially assigned Job for actual cleanup.
        let job = JobObject::create_for_spawn()?;
        self.lifecycle.check()?;
        let child = command
            .spawn()
            .map_err(|e| format!("spawn 宫格子进程 grid-{index} 失败: {e}"))?;
        let pid = child.id();
        let assigned = job.assign_suspended_pid(pid);
        let saved = saved.unwrap_or_default();
        let owner = GridChildHandle {
            child,
            job,
            comms: Arc::clone(&comms),
            focused: false,
            last_url: saved.last_url,
            last_rect: saved.last_rect,
            hidden: saved.hidden,
            blur_hidden: saved.blur_hidden,
            hidden_since: saved.hidden.then(std::time::Instant::now),
        };
        if let Err(error) = assigned {
            self.lifecycle.retire(index, owner);
            return Err(format!("WINDOWS_GRID_JOB_FAILED:{error}"));
        }
        {
            let mut children = self.children.lock().unwrap();
            if let Err(error) = self.lifecycle.check() {
                self.lifecycle.retire(index, owner);
                return Err(error);
            }
            children.insert(index, owner);
            comms.publish_owner(pid);
        }
        // The in-flight permit covers the check/insert race. The cleanup
        // service drains any publication that raced the application's latch.
        self.lifecycle.check()?;
        eprintln!("[grid-manager] spawned grid-child-{} pid={}", index, pid);
        self.start_monitor();
        Ok(())
    }

    /// 获取或 spawn：若该 index 的子进程不存在则创建。返回是否为新创建。
    pub fn get_or_spawn(&self, index: u32) -> Result<bool, String> {
        let _operation = self.lifecycle.enter()?;
        let _lifecycle = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        if self.children.lock().unwrap().contains_key(&index) {
            return Ok(false);
        }
        if self.lifecycle.is_retiring(index) {
            return Err(format!("GRID_CHILD_CLEANUP_PENDING:{index}"));
        }
        let saved = self.hibernated.lock().unwrap().remove(&index).or_else(|| {
            self.pending_restarts
                .lock()
                .unwrap()
                .remove(&index)
                .map(|(_, saved)| saved)
        });
        if let Err(error) = self.spawn_with_state_locked(index, saved.clone()) {
            if let Some(saved) = saved {
                self.hibernated.lock().unwrap().insert(index, saved);
            }
            return Err(error);
        }
        // The caller owns WebView creation/replay. Returning true guarantees
        // exactly one CreateTab command after both cold spawn and hibernation.
        Ok(true)
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

    pub(crate) fn start_tab_renderer(&self, index: u32, url: &str) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        if !self.get_or_spawn(index)? {
            return Err("TAB_RENDERER_ALREADY_RUNNING".into());
        }
        // Keep the actual window hidden until the active tab receives its
        // first acknowledged rectangle. No embedded fallback exists.
        self.record_hidden(index);
        let command = GridCmd::CreateTab {
            id: crate::grid_ipc::renderer_label(index),
            url: url.to_owned(),
        };
        match self.request(index, command, 20000) {
            Ok(()) => {
                self.record_url(index, url);
                Ok(())
            }
            Err(error) => {
                match self.close_tab_renderer(index, true) {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(format!("{error}; {cleanup}")),
                }
            }
        }
    }

    /// Retire only this route. Retain the same listener across an explicit
    /// recovery so process/connection generations cannot reset.
    pub(crate) fn close_tab_renderer(&self, index: u32, release_listener: bool) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let _operation = self.lifecycle.enter()?;
        self.ensure_cleanup()?;
        let _gate = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        self.crash_history.lock().unwrap().remove(&index);
        self.hibernated.lock().unwrap().remove(&index);
        self.pending_restarts.lock().unwrap().remove(&index);
        if let Some(owner) = self.children.lock().unwrap().remove(&index) {
            self.lifecycle.retire(index, owner);
        }
        self.lifecycle.wait_route_retired_until(
            index, std::time::Instant::now() + std::time::Duration::from_secs(3),
        )?;
        if release_listener {
            if let Some(comms) = self.listeners.lock().unwrap().remove(&index) {
                comms.request_shutdown();
            }
        }
        Ok(())
    }

    /// Serialize normal-tab visibility with the native move/focus workers.
    /// This is a blocking-worker API; never call it on the GUI event loop.
    pub(crate) fn position_tab_renderer(
        &self, index: u32, css: (f64, f64, f64, f64),
    ) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        if css.0 <= -20000.0 {
            return self.hide_tab_renderer(index);
        }
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        let app = self.app().ok_or("TAB_RENDERER_HOST_UNAVAILABLE")?;
        let label = crate::grid_ipc::renderer_label(index);
        if app.state::<crate::bridge::AppState>().active_tab.lock().unwrap().as_deref() != Some(label.as_str()) {
            return Err("TAB_RENDERER_LAYOUT_SUPERSEDED".into());
        }
        let rect = self.abs_rect(css).ok_or("TAB_RENDERER_HOST_UNAVAILABLE")?;
        rect.validate()?;
        self.request(index, GridCmd::UpdateRect { id: label, rect }, 3000)?;
        self.record_rect(index, css);
        Ok(())
    }

    pub(crate) fn hide_tab_renderer(&self, index: u32) -> Result<(), String> {
        if !crate::grid_ipc::is_tab_renderer(index) {
            return Err("TAB_RENDERER_ROUTE_INVALID".into());
        }
        let _dispatch = self.visibility_dispatch.lock().unwrap_or_else(|p| p.into_inner());
        self.record_hidden(index);
        // A confirmed absent native process has no window to hide.
        if self.pid_of(index).is_none() && !self.lifecycle.is_retiring(index) {
            return Ok(());
        }
        self.request(index, GridCmd::HideWindow { id: crate::grid_ipc::renderer_label(index) }, 3000)
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
        // Keep get_or_spawn from reporting an existing child while this
        // operation is closing and removing it. Pipe write timeouts terminate
        // the Job directly and do not re-enter the lifecycle lock.
        let _lifecycle = self.lifecycle_lock.lock().unwrap();
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
        self.request_once_with(index, comms, generation, &cmd, timeout_ms).map(|_| ())
    }

    /// A delayed load retry must retain its original renderer generation.
    pub(crate) fn retry_failed_navigation(&self, index: u32, generation: u64, url: &str) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        let comms = {
            let children = self.children.lock().unwrap();
            let child = children.get(&index).ok_or("GRID_RENDERER_NOT_OWNED")?;
            if !child.comms.is_current(generation) || child.last_url.as_deref() != Some(url) {
                return Err("GRID_RETRY_SUPERSEDED".into());
            }
            Arc::clone(&child.comms)
        };
        let cmd = GridCmd::Navigate { id: crate::grid_ipc::renderer_label(index), url: url.to_string() };
        self.request_once_with(index, comms, generation, &cmd, 10000).map(|_| ())
    }

    /// Fail-closed native probe of the actual Grid page. Stored last_url is
    /// only a navigation target and cannot prove the live DOM is empty.
    fn confirmed_blank_grid(&self, index: u32) -> bool {
        let Ok((comms, generation)) = self.wait_connected(index, 500) else {
            return false;
        };
        match self.request_once_with(
            index,
            comms,
            generation,
            &GridCmd::HibernationProbe {
                id: crate::grid_ipc::renderer_label(index),
            },
            1000,
        ) {
            Ok(Some(encoded)) => grid_hibernation_probe_safe(&encoded),
            _ => false,
        }
    }

    pub fn read_replies(&self, index: u32) -> Result<String, String> {
        if !self.children.lock().unwrap().contains_key(&index) {
            return Err("GRID_NOT_OPEN".into());
        }
        let (comms, generation) = self.wait_connected(index, 500)?;
        self.request_once_with(
            index,
            comms,
            generation,
            &GridCmd::ReadReplies {
                id: crate::grid_ipc::renderer_label(index),
            },
            5000,
        )?
        .ok_or_else(|| "GRID_NO_RESULT".into())
    }

    fn request_once_with(
        &self,
        index: u32,
        comms: Arc<ChildComms>,
        generation: u64,
        cmd: &GridCmd,
        timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        let _operation = self.lifecycle.enter()?;
        let seq;
        let peer_pid;
        let (tx, rx) = mpsc::channel();
        let write_res = {
            let mut guard = comms.writer.lock().unwrap();
            self.lifecycle.check()?;
            if !comms.is_current(generation) {
                return Err("GRID_CONNECTION_GENERATION_CHANGED".into());
            }
            // Allocate in writer order, not caller/thread order. The worker
            // rejects replays and the counter must never wrap.
            seq = crate::grid_ipc::next_request_sequence(&comms.seq)?;
            peer_pid = comms.expected_pid.load(Ordering::SeqCst);
            let result = match guard.as_mut() {
                Some(w) => {
                    {
                        let _owner = comms.owner_gate.lock().unwrap();
                        if !comms.is_current(generation) {
                            return Err("GRID_CONNECTION_GENERATION_CHANGED".into());
                        }
                        comms.pending.lock().unwrap().insert(seq, tx);
                    }
                    crate::grid_ipc::write_wire(w, &Wire::Request { seq, cmd: cmd.clone() })
                }
                None => Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "GRID_NOT_CONNECTED")),
            };
            if result.is_err() {
                comms.connected.store(false, Ordering::SeqCst);
                if let Some(control) = comms.cancellation.lock().unwrap().as_ref() {
                    let _ = control.cancel_io();
                }
            }
            result
        };
        if let Err(e) = write_res {
            comms.pending.lock().unwrap().remove(&seq);
            comms.detach_if_current(generation);
            self.terminate_child_job(index, &comms, generation, peer_pid);
            return Err(format!("GRID_REQUEST_OUTCOME_UNKNOWN: NamedPipe write failed: {e}"));
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            if let Err(error) = self.lifecycle.check() {
                comms.pending.lock().unwrap().remove(&seq);
                return Err(error);
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                comms.pending.lock().unwrap().remove(&seq);
                return Err("GRID_REQUEST_OUTCOME_UNKNOWN: 等待子进程响应超时".into());
            }
            match rx.recv_timeout(remaining.min(std::time::Duration::from_millis(50))) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    comms.pending.lock().unwrap().remove(&seq);
                    return Err("GRID_REQUEST_OUTCOME_UNKNOWN: 子进程响应通道已关闭".into());
                }
            }
        }
    }

    /// 发后即忘（高频定位/显隐），错误只记日志。
    pub fn send(&self, index: u32, cmd: GridCmd) {
        let Ok(_operation) = self.lifecycle.enter() else {
            return;
        };
        if !cmd.targets(index) {
            return;
        }
        let comms = match self.listeners.lock().unwrap().get(&index).cloned() {
            Some(c) => c,
            None => return,
        };
        let generation = comms.generation.load(Ordering::SeqCst);
        let mut guard = comms.writer.lock().unwrap();
        if self.lifecycle.is_stopping() || !comms.is_current(generation) {
            return;
        }
        let peer_pid = comms.expected_pid.load(Ordering::SeqCst);
        let Ok(seq) = crate::grid_ipc::next_request_sequence(&comms.seq) else {
            return;
        };
        let failed = if let Some(w) = guard.as_mut() {
            match crate::grid_ipc::write_wire(w, &Wire::Request { seq, cmd }) {
                Ok(()) => false,
                Err(error) => {
                    eprintln!("[grid-manager] grid-{index} send failed: {error}");
                    true
                }
            }
        } else {
            false
        };
        if failed {
            comms.connected.store(false, Ordering::SeqCst);
            if let Some(control) = comms.cancellation.lock().unwrap().as_ref() {
                let _ = control.cancel_io();
            }
        }
        drop(guard);
        if failed {
            comms.detach_if_current(generation);
            self.terminate_child_job(index, &comms, generation, peer_pid);
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
        let _lifecycle = self.lifecycle_lock.lock().unwrap();
        if self.lifecycle.is_stopping() {
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
            let eligible = || {
                self.children.lock().unwrap().get(&index).is_some_and(|child| {
                    child.hidden_since == Some(hidden_since)
                        && grid_content_reclaimable(child.last_url.as_deref())
                        && grid_child_is_hibernatable(
                            child.hidden,
                            child.blur_hidden,
                            child.focused,
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
                    id: crate::grid_ipc::renderer_label(index),
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
            let mut children = self.children.lock().unwrap();
            let still_eligible = children.get(&index).is_some_and(|child| {
                child.hidden_since == Some(hidden_since)
                    && grid_content_reclaimable(child.last_url.as_deref())
                    && grid_child_is_hibernatable(
                        child.hidden,
                        child.blur_hidden,
                        child.focused,
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
                // Retirement remains registered until the original Job is
                // empty; get_or_spawn cannot reuse this Grid index meanwhile.
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
                        id: crate::grid_ipc::renderer_label(index),
                        rect,
                    },
                );
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
            "tab-navigated" => child.last_url = payload["url"].as_str().map(str::to_owned),
            "grid-load-failed" => {
                payload["renderer_generation"] = serde_json::json!(generation.to_string());
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
        let indices = self.all_renderer_indices();
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
                        id: crate::grid_ipc::renderer_label(index),
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
                        id: crate::grid_ipc::renderer_label(index),
                        rect,
                    },
                );
            }
        }
    }

    // ===== 崩溃自愈（Phase 3） =====

    /// 启动监控线程（幂等）：轮询 try_wait，异常退出 → 自动重启 + 状态重放。
    /// 正常关闭（kill_child/shutdown_all）会先从表中移除句柄，监控不会误判重启。
    fn start_monitor(&self) {
        if self.monitor_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let app = self.app();
        let monitor_lifecycle = Arc::clone(&self.lifecycle);
        // 监控需要访问 manager 自身——AppState 持有 manager，经 app handle 取回。
        let started = self.spawn_worker("grid-monitor", move || {
            // 主窗最小化状态（GNOME/Wayland 下最小化不一定触发 Focused(false)，
            // 必须轮询 is_minimized，否则宫格残留浮在其它应用上）
            let mut was_minimized = false;
            loop {
                if monitor_lifecycle.wait_or_stop(std::time::Duration::from_millis(500)) { break; }
                let Some(app) = app.as_ref() else { continue };
                let state = app.state::<crate::bridge::AppState>();
                let manager = &state.grid_manager;
                manager.retry_visibility_worker();
                // 主窗最小化 → 隐藏全部宫格；恢复 → 还原 blur 隐藏的格子
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
                let _lifecycle = manager.lifecycle_lock.lock().unwrap();
                if manager.lifecycle.is_stopping() { break; }
                let mut exited: Vec<(u32, i32, SavedChildState)> = Vec::new();
                {
                    let mut children = manager.children.lock().unwrap();
                    let indices: Vec<u32> = children.keys().cloned().collect();
                    for index in indices {
                        if let Some(h) = children.get_mut(&index) {
                            match h.child.try_wait() {
                                Ok(Some(status)) => {
                                    let code = exit_code_of(&status);
                                    let h = children.remove(&index).unwrap();
                                    // 保留崩溃前状态供重放（新句柄默认 last_url=None
                                    // 会导致 replay 跳过，实测踩过）
                                    exited.push((
                                        index,
                                        code,
                                        SavedChildState {
                                            last_url: h.last_url.clone(),
                                            last_rect: h.last_rect,
                                            hidden: h.hidden,
                                            blur_hidden: h.blur_hidden,
                                        },
                                    ));
                                    manager.lifecycle.retire(index, h);
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    eprintln!(
                                        "[grid-manager] grid-child-{} try_wait error: {e}",
                                        index
                                    );
                                }
                            }
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
                    // Preserve the crash circuit/backoff without attempting a
                    // new process while the old Job still owns descendants.
                    let (allowed, attempts) = {
                        let mut history = manager.crash_history.lock().unwrap();
                        let events = history.entry(index).or_default();
                        let now = std::time::Instant::now();
                        while events.front().is_some_and(|t| now.duration_since(*t).as_secs() > 120) {
                            events.pop_front();
                        }
                        events.push_back(now);
                        (events.len() <= 3, events.len())
                    };
                    if allowed {
                        manager.pending_restarts.lock().unwrap().insert(index, (
                            std::time::Instant::now()
                                + std::time::Duration::from_millis(350 * attempts as u64),
                            saved,
                        ));
                        eprintln!("[grid-manager] grid-child-{index} exited code={code}; restart waits for Job cleanup");
                    } else {
                        eprintln!("[grid-manager] grid-child-{index} crash circuit open (3 restarts / 120s)");
                    }
                }
                if manager.lifecycle.is_stopping() { break; }
                let ready: Vec<u32> = manager.pending_restarts.lock().unwrap().iter()
                    .filter(|(index, (when, _))| *when <= std::time::Instant::now()
                        && !manager.lifecycle.is_retiring(**index))
                    .map(|(index, _)| *index).collect();
                for index in ready {
                    let Some((_, saved)) = manager.pending_restarts.lock().unwrap().remove(&index) else { continue; };
                    if manager.children.lock().unwrap().contains_key(&index) { continue; }
                    if let Err(error) = manager.spawn_with_state_locked(index, Some(saved.clone())) {
                        if !manager.lifecycle.is_stopping() {
                            manager.hibernated.lock().unwrap().insert(index, saved);
                        }
                        eprintln!("[grid-manager] grid-child-{index} restart failed: {error}");
                    } else {
                        manager.replay(index);
                    }
                }
            }
        });
        if let Err(error) = started {
            self.monitor_started.store(false, Ordering::SeqCst);
            eprintln!("[grid-manager] monitor unavailable: {error}");
        }
    }

    /// 重启后状态重放：等 UDS 重连 → 重建 webview（url）→ 恢复定位（若可见）→ 恢复缩放。
    fn replay(&self, index: u32) {
        if crate::grid_ipc::is_tab_renderer(index) { return; }
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
        let started = self.spawn_worker("grid-replay", move || {
            let Some(app) = app else { return };
            let state = app.state::<crate::bridge::AppState>();
            let manager = &state.grid_manager;
            let label = crate::grid_ipc::renderer_label(index);
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

    fn terminate_child_job(&self, index: u32, comms: &ChildComms, generation: u64, peer_pid: u32) {
        let children = self.children.lock().unwrap();
        // A late failure of an old pipe must never terminate a replacement
        // Job at the same index, including before its first connection.
        if comms.generation.load(Ordering::SeqCst) != generation
            || comms.expected_pid.load(Ordering::SeqCst) != peer_pid
            || peer_pid == 0
        {
            return;
        }
        if let Some(child) = children.get(&index).filter(|child| child.child.id() == peer_pid) {
            // Do not remove the map entry here: the monitor owns crash/restart
            // bookkeeping. Closing the Job immediately prevents partial IPC
            // frames from being reused while the monitor observes exit.
            if let Err(error) = child.job.terminate() {
                eprintln!("[grid-manager] terminate Grid Job {index} failed: {error}");
            }
        }
    }

    /// 主动关闭单个宫格子进程（grid_close_one/close_grid）：先移出表（监控不重启），
    /// 再 kill。返回是否存在。
    fn kill_child_locked(&self, index: u32) -> bool {
        self.crash_history.lock().unwrap().remove(&index);
        self.hibernated.lock().unwrap().remove(&index);
        self.pending_restarts.lock().unwrap().remove(&index);
        let removed = self.children.lock().unwrap().remove(&index);
        if let Some(h) = removed {
            self.lifecycle.retire(index, h);
            eprintln!(
                "[grid-manager] retirement requested for grid-child-{}",
                index
            );
            true
        } else {
            false
        }
    }

    pub fn kill_child(&self, index: u32) -> bool {
        let Ok(_operation) = self.lifecycle.enter() else {
            return false;
        };
        let _lifecycle = self.lifecycle_lock.lock().unwrap();
        self.kill_child_locked(index)
    }

    /// 重启指定宫格子进程：先 kill 旧的（若还在），再 spawn 新的。
    #[allow(dead_code)]
    pub fn restart(&self, index: u32) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        let _lifecycle = self.lifecycle_lock.lock().unwrap();
        self.lifecycle.check()?;
        self.kill_child_locked(index);
        self.lifecycle.wait_retired_until(
            std::time::Instant::now() + std::time::Duration::from_millis(750),
        )?;
        self.spawn_with_state_locked(index, None)?;
        eprintln!("[grid-manager] restarted grid-child-{}", index);
        Ok(())
    }

    /// 检查指定宫格子进程是否还存活（try_wait 非阻塞）。
    /// 返回 Some(exit_code) 表示已退出（信号杀死时为 128+signal，如 SIGSEGV=139），None 表示仍在运行。
    #[allow(dead_code)]
    pub fn check_exited(&self, index: u32) -> Option<i32> {
        let _operation = self.lifecycle.enter().ok()?;
        let _lifecycle = self.lifecycle_lock.lock().unwrap();
        let mut children = self.children.lock().unwrap();
        if let Some(h) = children.get_mut(&index) {
            match h.child.try_wait() {
                Ok(Some(status)) => {
                    let real = exit_code_of(&status);
                    let owner = children.remove(&index).unwrap();
                    self.lifecycle.retire(index, owner);
                    Some(real)
                }
                Ok(None) => None,
                Err(_) => None,
            }
        } else {
            None
        }
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
    pub fn pids(&self) -> Vec<(u32, u32)> {
        self.children
            .lock()
            .unwrap()
            .iter()
            .filter(|(i, _)| !crate::grid_ipc::is_tab_renderer(**i))
            .map(|(i, h)| (*i, h.child.id()))
            .collect()
    }

    /// Ordinary close only: the admission gate stays open for future Grid use.
    /// All children share this one wait budget; incomplete owners stay queued.
    pub fn shutdown_all(&self) -> Result<(), String> {
        let _operation = self.lifecycle.enter()?;
        self.ensure_cleanup()?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
        {
            let _lifecycle = self.lifecycle_lock.lock().unwrap();
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
        self.lifecycle.wait_retired_until(deadline)
    }
}

/// Native ownership stays intact through assignment failure and process exit.
/// A parent exit alone never establishes that WebView2 descendants are gone.
struct GridRetirement {
    index: u32,
    owner: GridChildHandle,
    root_reaped: bool,
}
impl GridRetirement {
    fn new(index: u32, owner: GridChildHandle) -> Self {
        Self {
            index,
            owner,
            root_reaped: false,
        }
    }
    fn index(&self) -> u32 {
        self.index
    }
    fn poll(&mut self) -> Result<bool, String> {
        let job_empty = self.owner.job.is_empty()?;
        let termination = if job_empty {
            Ok(())
        } else {
            self.owner.job.terminate()
        };
        if !self.root_reaped {
            match self.owner.child.try_wait() {
                Ok(Some(_)) => self.root_reaped = true,
                Ok(None) => {
                    // Also covers an assignment error that left the original
                    // process suspended but outside the otherwise empty Job.
                    self.owner
                        .child
                        .kill()
                        .map_err(|error| format!("GRID_CHILD_TERMINATE_FAILED:{error}"))?;
                }
                Err(error) => return Err(format!("GRID_CHILD_WAIT_FAILED:{error}")),
            }
        }
        termination?;
        Ok(self.root_reaped && self.owner.job.is_empty()?)
    }
}

/// 从 ExitStatus 取退出码：正常退出用 code()，被信号杀死则 128+signal（如 SIGSEGV=139）。
fn exit_code_of(status: &std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return 128 + sig;
        }
    }
    -1
}

#[cfg(test)]
mod windows_grid_tests {
    use super::*;
    #[test]
    fn pipe_name_is_private_unique_and_stable_per_index() {
        let first = GridProcessManager::socket_path(0);
        assert!(first
            .to_string_lossy()
            .starts_with(r"\\.\pipe\BrowserOS-grid-"));
        assert_eq!(first, GridProcessManager::socket_path(0));
        assert_ne!(first, GridProcessManager::socket_path(1));
    }
    #[test]
    fn grid_hibernation_requires_a_live_blank_page_proof() {
        assert!(grid_hibernation_probe_safe(
            r#"{"safe":true,"url":"about:blank"}"#
        ));
        assert!(!grid_hibernation_probe_safe(
            r#"{"safe":true,"url":"https://bilibili.com"}"#
        ));
        assert!(!grid_hibernation_probe_safe("invalid json"));
    }
    #[test]
    fn application_grid_shutdown_does_not_wait_for_ordinary_lifecycle_lock() {
        let manager = GridProcessManager::default();
        let ordinary_close = manager.lifecycle_lock.lock().unwrap();
        let started = std::time::Instant::now();
        let result = manager.shutdown_until(started + std::time::Duration::from_secs(2));
        assert!(result.is_ok(), "{result:?}");
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        drop(ordinary_close);
        assert!(manager
            .get_or_spawn(0)
            .unwrap_err()
            .contains("GRID_SHUTDOWN_IN_PROGRESS"));
    }

    #[test]
    fn ordinary_grid_close_remains_reopenable() {
        let manager = GridProcessManager::default();
        manager.shutdown_all().unwrap();
        assert!(manager.lifecycle.enter().is_ok());
        manager
            .shutdown_until(std::time::Instant::now() + std::time::Duration::from_secs(5))
            .unwrap();
    }

    #[test]
    fn grid_retirement_waits_for_job_descendants_after_root_exit() {
        use std::os::windows::process::CommandExt;
        use std::time::{Duration, Instant};
        let marker =
            std::env::temp_dir().join(format!("browseros-grid-job-{}.pid", uuid::Uuid::new_v4()));
        let quoted = marker.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$p=Start-Process -FilePath $env:ComSpec -ArgumentList @('/c','ping -n 60 127.0.0.1 > NUL') -PassThru; Set-Content -LiteralPath '{quoted}' -Value $p.Id"
        );
        let job = JobObject::create_for_spawn().unwrap();
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(crate::windows_process::CREATE_SUSPENDED);
        let child = command.spawn().unwrap();
        let assignment = job.assign_suspended_pid(child.id());
        let owner = GridChildHandle {
            child,
            job,
            comms: Arc::new(ChildComms::new()),
            focused: false,
            last_url: None,
            last_rect: None,
            hidden: false,
            blur_hidden: false,
            hidden_since: None,
        };
        let mut retired = GridRetirement::new(0, owner);
        if let Err(error) = assignment {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline && !retired.poll().unwrap_or(false) {
                std::thread::sleep(Duration::from_millis(10));
            }
            panic!("native suspended assignment failed: {error}");
        }
        let ready_deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if retired.owner.child.try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                Instant::now() < ready_deadline,
                "fixture parent did not exit"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(std::fs::read_to_string(&marker)
            .unwrap()
            .trim()
            .parse::<u32>()
            .is_ok());
        assert!(
            !retired.owner.job.is_empty().unwrap(),
            "descendant must outlive the parent fixture"
        );
        let cleanup_deadline = Instant::now() + Duration::from_secs(8);
        while !retired.poll().unwrap() {
            assert!(
                Instant::now() < cleanup_deadline,
                "native Job cleanup did not finish"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            retired.owner.job.is_empty().unwrap(),
            "cleanup requires actual Job accounting"
        );
        assert!(retired.root_reaped);
        let _ = std::fs::remove_file(marker);
    }
}
