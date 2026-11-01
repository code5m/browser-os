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
use std::io::BufReader;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::grid_ipc::{GridCmd, IpcRect, Wire};

/// 单个子进程的通信通道（跨重启复用：writer 随连接替换，seq 单调递增）。
pub struct ChildComms {
    writer: Mutex<Option<UnixStream>>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<Option<String>, String>>>>,
    seq: AtomicU64,
    generation: AtomicU64,
    connected: AtomicBool,
}

impl ChildComms {
    fn new() -> Self {
        Self {
            writer: Mutex::new(None),
            pending: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(1),
            generation: AtomicU64::new(0),
            connected: AtomicBool::new(false),
        }
    }

    /// 每次 UDS 重连分配新的代际，防止旧 reader 迟到的 EOF 清掉新 writer。
    /// 保持 writer -> pending 的锁序；新连接建立时立即唤醒旧连接请求。
    fn attach(&self, stream: UnixStream) -> u64 {
        let mut writer = self.writer.lock().unwrap();
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *writer = Some(stream);
        self.connected.store(true, Ordering::SeqCst);
        self.fail_pending("子进程连接已更换");
        generation
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
        *writer = None;
        self.connected.store(false, Ordering::SeqCst);
        self.fail_pending("子进程连接断开");
    }

    fn is_current(&self, generation: u64) -> bool {
        self.connected.load(Ordering::SeqCst)
            && self.generation.load(Ordering::SeqCst) == generation
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
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

/// 单个宫格子进程句柄。
fn terminate_grid_process_tree(child: &mut Child) {
    // Kill the process group, including WebKit helpers. Never hold the Grid
    // manager lock inside an unbounded Child::wait after SIGKILL.
    let pid = child.id() as i32;
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let _ = child.kill();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => {
                eprintln!("[grid-manager] waitpid={} failed: {}", pid, error);
                return;
            }
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    // A kernel-stuck child may exit after the deadline; reap it out of band so
    // the foreground Grid lifecycle never waits indefinitely and no zombie
    // is intentionally left behind.
    std::thread::spawn(move || {
        let waited = unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
        if waited < 0 {
            eprintln!("[grid-manager] deferred reap pid={} failed", pid);
        }
    });
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
    children: Mutex<HashMap<u32, GridChildHandle>>,
    /// Hibernated children retain URL/rect/visibility metadata but no WebKit process.
    hibernated: Mutex<HashMap<u32, SavedChildState>>,
    /// 已绑定 listener 的 index（accept 循环线程常驻，重启复用同一 socket）
    listeners: Mutex<HashMap<u32, Arc<ChildComms>>>,
    app: Mutex<Option<AppHandle>>,
    monitor_started: AtomicBool,
    /// Crash budget per grid cell; normal user-triggered shutdown is not counted.
    crash_history: Mutex<HashMap<u32, VecDeque<std::time::Instant>>>,
    /// 子进程窗口焦点状态（子进程经 UDS Event 上报）：主窗 blur 时据此区分
    /// "用户点了宫格"（不隐藏）与"切到其它应用"（隐藏防幽灵浮层）
    child_focused: Mutex<HashMap<u32, bool>>,
}

impl GridProcessManager {
    /// 主进程 setup 时注入 AppHandle（事件转发/重放/定位换算用）。
    pub fn set_app(&self, app: AppHandle) {
        *self.app.lock().unwrap() = Some(app);
    }

    fn app(&self) -> Option<AppHandle> {
        self.app.lock().unwrap().clone()
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
        let comms = Arc::new(ChildComms::new());
        listeners.insert(index, comms.clone());
        drop(listeners);

        // accept 循环线程：子进程（含崩溃重启后的新实例）连接到同一 listener。
        let comms_accept = comms.clone();
        let app = self.app();
        std::thread::spawn(move || loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    eprintln!("[grid-manager] grid-child-{} UDS 已连接", index);
                    let reader = match stream.try_clone() {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("[grid-manager] grid-child-{} try_clone 失败: {e}", index);
                            continue;
                        }
                    };
                    // 子进程不读时也必须限制 writer 阻塞时长，避免请求堆积。
                    if let Err(e) = stream.set_write_timeout(Some(std::time::Duration::from_secs(2))) {
                        eprintln!("[grid-manager] grid-child-{} UDS 写超时配置失败: {e}", index);
                        continue;
                    }
                    let generation = comms_accept.attach(stream);
                    // 每连接一个 reader 线程：Response 按 seq 路由，Event 转发 app.emit。
                    let comms_reader = comms_accept.clone();
                    let app_reader = app.clone();
                    std::thread::spawn(move || {
                        let mut r = BufReader::new(reader);
                        loop {
                            let frame = crate::grid_ipc::read_wire(&mut r);
                            // 旧连接仍有缓冲帧时，不允许覆盖新连接的响应/焦点状态。
                            if !comms_reader.is_current(generation) {
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
                                                let state = app.state::<crate::bridge::AppState>();
                                                state.grid_manager.set_child_focus(index, true);
                                            }
                                            "grid-child-blur" => {
                                                let state = app.state::<crate::bridge::AppState>();
                                                state.grid_manager.set_child_focus(index, false);
                                                // 从宫格切到其它应用时主窗不会再次 blur，
                                                // 子窗口 blur 后若主窗也无焦点 → 隐藏防幽灵浮层
                                                let app2 = app.clone();
                                                std::thread::spawn(move || {
                                                    std::thread::sleep(
                                                        std::time::Duration::from_millis(150),
                                                    );
                                                    let st =
                                                        app2.state::<crate::bridge::AppState>();
                                                    st.grid_manager
                                                        .hide_for_blur_if_no_child_focus();
                                                });
                                            }
                                            "tab-navigated" => {
                                                // Keep the native memory guard in sync with
                                                // same-WebView redirects and in-page links.
                                                // Only accept navigation for this Grid child.
                                                if let (Some(id), Some(url)) = (
                                                    payload.get("id").and_then(|v| v.as_str()),
                                                    payload.get("url").and_then(|v| v.as_str()),
                                                ) {
                                                    if id == format!("grid-{index}") && !url.is_empty() {
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
                }
                Err(e) => {
                    eprintln!("[grid-manager] grid-child-{} accept 失败: {e}", index);
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }
        });
        Ok(comms)
    }

    /// spawn 一个新的宫格子进程（不检查是否已存在，调用方需先确保没有重复）。
    fn spawn(&self, index: u32) -> Result<(), String> {
        self.spawn_with_state(index, None)
    }

    /// spawn 并可携带崩溃前保存的状态（崩溃重启时保留 last_url/last_rect，
    /// 否则 replay 读不到历史 url 会跳过重放）。
    fn spawn_with_state(&self, index: u32, saved: Option<SavedChildState>) -> Result<(), String> {
        self.ensure_listener(index)?;
        let exe = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        let child = Command::new(exe)
            .arg("--grid-child")
            .arg(index.to_string())
            // 继承父进程环境（WEBKIT_DISABLE_DMABUF_RENDERER / GDK_BACKEND 等在
            // main.rs 已 set_var，子进程经环境继承获得）；额外注入子进程索引
            // （crashlog 角色区分）与 UDS socket 路径。
            .env("GRID_CHILD_INDEX", index.to_string())
            .env("GRID_SOCK_PATH", Self::socket_path(index))
            // Give each Grid child its own process group so WebKit helper processes
            // cannot outlive a hibernated or explicitly closed Grid cell.
            .process_group(0)
            // 子进程 stdio 继承主进程（日志汇入同一 stderr，便于 crashlog 镜像）。
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("spawn 宫格子进程 grid-{index} 失败: {e}"))?;
        let pid = child.id();
        let saved = saved.unwrap_or_default();
        self.children.lock().unwrap().insert(
            index,
            GridChildHandle {
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
            },
        );
        eprintln!("[grid-manager] spawned grid-child-{} pid={}", index, pid);
        self.start_monitor();
        Ok(())
    }

    /// 获取或 spawn：若该 index 的子进程不存在则创建。返回是否为新创建。
    pub fn get_or_spawn(&self, index: u32) -> Result<bool, String> {
        if self.children.lock().unwrap().contains_key(&index) {
            return Ok(false);
        }
        let saved = self.hibernated.lock().unwrap().remove(&index);
        if let Err(error) = self.spawn_with_state(index, saved.clone()) {
            if let Some(saved) = saved {
                self.hibernated.lock().unwrap().insert(index, saved);
            }
            return Err(error);
        }
        // The caller owns WebView creation/replay. Returning true guarantees
        // exactly one CreateTab command after both cold spawn and hibernation.
        Ok(true)
    }

    pub fn is_hibernated(&self, index: u32) -> bool {
        self.hibernated.lock().unwrap().contains_key(&index)
    }

    /// Preserve child metadata while releasing its process. Used to roll back
    /// a failed resume without losing the URL/layout that existed pre-resume.
    pub fn hibernate_child(&self, index: u32) -> bool {
        let _ = self.request(
            index,
            GridCmd::CloseTab {
                id: format!("grid-{index}"),
            },
            1000,
        );
        let removed = self.children.lock().unwrap().remove(&index);
        if let Some(mut child) = removed {
            let saved = SavedChildState {
                last_url: child.last_url.clone(),
                last_rect: child.last_rect,
                hidden: child.hidden,
                blur_hidden: child.blur_hidden,
            };
            terminate_grid_process_tree(&mut child.child);
            self.hibernated.lock().unwrap().insert(index, saved);
            true
        } else {
            false
        }
    }

    /// 等待该 index 的 UDS 连接就绪（子进程冷启动需 1~3 秒）。
    fn wait_connected(&self, index: u32, timeout_ms: u64) -> Result<Arc<ChildComms>, String> {
        let comms = self
            .listeners
            .lock()
            .unwrap()
            .get(&index)
            .cloned()
            .ok_or_else(|| format!("grid-{index} 无 UDS listener（未 spawn？）"))?;
        let start = std::time::Instant::now();
        while !comms.is_connected() {
            if start.elapsed().as_millis() as u64 > timeout_ms {
                return Err(format!("等待 grid-child-{index} UDS 连接超时"));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(comms)
    }

    /// 同步请求：等连接 → 写 Request → 等 Response（带超时）。
    /// 失败（连接断/超时）时等重连后重试一次（覆盖崩溃重启窗口期）。
    pub fn request(&self, index: u32, cmd: GridCmd, timeout_ms: u64) -> Result<(), String> {
        match self.request_once(index, &cmd, timeout_ms) {
            Ok(()) => Ok(()),
            // 超时不代表命令未执行；同一连接重放可能重复创建/导航，且把等待翻倍。
            Err(first) if first.contains("超时") => Err(first),
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
        let seq = comms.seq.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        let write_res = {
            let mut guard = comms.writer.lock().unwrap();
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
                },
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
        match rx.recv_timeout(std::time::Duration::from_millis(timeout_ms)) {
            Ok(r) => r,
            Err(_) => {
                comms.pending.lock().unwrap().remove(&seq);
                Err("等待子进程响应超时".to_string())
            }
        }
    }

    /// 发后即忘（高频定位/显隐），错误只记日志。
    pub fn send(&self, index: u32, cmd: GridCmd) {
        let comms = match self.listeners.lock().unwrap().get(&index).cloned() {
            Some(c) => c,
            None => return,
        };
        let mut guard = comms.writer.lock().unwrap();
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
                ) || !grid_content_reclaimable(child.last_url.as_deref()) {
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
                    && self.children.lock().unwrap().get(&index).is_some_and(|child| {
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
            if let Some(mut child) = children.remove(&index) {
                // Hold the map lock until the old process group has exited:
                // another Grid activation must not spawn an index while an
                // old child is still connected to the same UDS listener.
                terminate_grid_process_tree(&mut child.child);
                self.hibernated.lock().unwrap().insert(
                    index,
                    SavedChildState {
                        last_url: child.last_url.clone(),
                        last_rect: child.last_rect,
                        hidden: true,
                        blur_hidden: false,
                    },
                );
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

    /// 主窗 Moved/Resized：按记忆的 CSS rect 重算绝对坐标并下发（发后即忘）。
    pub fn reposition_visible(&self) {
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
        self.hide_for_blur();
    }

    /// 主窗失焦：隐藏所有可见宫格子进程窗口（标记 blur_hidden，聚焦时恢复）。
    pub fn hide_for_blur(&self) {
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
        let items: Vec<(u32, (f64, f64, f64, f64))> = {
            let mut children = self.children.lock().unwrap();
            children
                .iter_mut()
                .filter(|(_, h)| h.blur_hidden)
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

    /// 启动监控线程（幂等）：轮询 try_wait，异常退出 → 自动重启 + 状态重放。
    /// 正常关闭（kill_child/shutdown_all）会先从表中移除句柄，监控不会误判重启。
    fn start_monitor(&self) {
        if self.monitor_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let app = self.app();
        // 监控需要访问 manager 自身——AppState 持有 manager，经 app handle 取回。
        std::thread::spawn(move || {
            // 主窗最小化状态（GNOME/Wayland 下最小化不一定触发 Focused(false)，
            // 必须轮询 is_minimized，否则宫格残留浮在其它应用上）
            let mut was_minimized = false;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let Some(app) = app.as_ref() else { continue };
                let state = app.state::<crate::bridge::AppState>();
                let manager = &state.grid_manager;
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
                                            last_url: h.last_url,
                                            last_rect: h.last_rect,
                                            hidden: h.hidden,
                                            blur_hidden: h.blur_hidden,
                                        },
                                    ));
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
                    // Circuit breaker: repeated WebKit child crashes must not starve
                    // Browser/File Manager by infinitely respawning expensive processes.
                    let should_restart = {
                        let mut history = manager.crash_history.lock().unwrap();
                        let events = history.entry(index).or_default();
                        let now = std::time::Instant::now();
                        while events
                            .front()
                            .is_some_and(|t| now.duration_since(*t).as_secs() > 120)
                        {
                            events.pop_front();
                        }
                        events.push_back(now);
                        events.len() <= 3
                    };
                    if !should_restart {
                        eprintln!("[grid-manager] grid-child-{} crashed repeatedly; circuit open (3 restarts / 120s), no auto-respawn", index);
                        continue;
                    }
                    eprintln!(
                        "[grid-manager] grid-child-{} 异常退出 code={}，自动重启",
                        index, code
                    );
                    std::thread::sleep(std::time::Duration::from_millis(
                        350 * manager
                            .crash_history
                            .lock()
                            .unwrap()
                            .get(&index)
                            .map_or(1, |h| h.len()) as u64,
                    ));
                    if let Err(e) = manager.spawn_with_state(index, Some(saved)) {
                        eprintln!("[grid-manager] grid-child-{} 重启失败: {e}", index);
                        continue;
                    }
                    manager.replay(index);
                }
            }
        });
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
        std::thread::spawn(move || {
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
    }

    /// 主动关闭单个宫格子进程（grid_close_one/close_grid）：先移出表（监控不重启），
    /// 再 kill。返回是否存在。
    pub fn kill_child(&self, index: u32) -> bool {
        self.crash_history.lock().unwrap().remove(&index);
        self.hibernated.lock().unwrap().remove(&index);
        let removed = self.children.lock().unwrap().remove(&index);
        if let Some(mut h) = removed {
            terminate_grid_process_tree(&mut h.child);
            eprintln!("[grid-manager] killed grid-child-{}", index);
            true
        } else {
            false
        }
    }

    /// 重启指定宫格子进程：先 kill 旧的（若还在），再 spawn 新的。
    #[allow(dead_code)]
    pub fn restart(&self, index: u32) -> Result<(), String> {
        self.kill_child(index);
        self.spawn(index)?;
        eprintln!("[grid-manager] restarted grid-child-{}", index);
        Ok(())
    }

    /// 检查指定宫格子进程是否还存活（try_wait 非阻塞）。
    /// 返回 Some(exit_code) 表示已退出（信号杀死时为 128+signal，如 SIGSEGV=139），None 表示仍在运行。
    #[allow(dead_code)]
    pub fn check_exited(&self, index: u32) -> Option<i32> {
        let mut children = self.children.lock().unwrap();
        if let Some(h) = children.get_mut(&index) {
            match h.child.try_wait() {
                Ok(Some(status)) => {
                    let real = exit_code_of(&status);
                    children.remove(&index);
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

    /// 关闭所有宫格子进程（主进程退出前/close_grid 调用）。
    pub fn shutdown_all(&self) {
        self.crash_history.lock().unwrap().clear();
        self.hibernated.lock().unwrap().clear();
        let mut children = self.children.lock().unwrap();
        for (index, h) in children.iter_mut() {
            terminate_grid_process_tree(&mut h.child);
            eprintln!("[grid-manager] shutdown grid-child-{}", index);
        }
        children.clear();
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
        assert!(grid_hibernation_probe_safe(r#"{"safe":true,"url":"about:blank"}"#));
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
    use super::terminate_grid_process_tree;
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::time::{Duration, Instant};

    #[test]
    fn killing_a_grid_child_has_a_bounded_foreground_wait() {
        let mut child = Command::new("/usr/bin/sleep")
            .arg("10")
            .process_group(0)
            .spawn()
            .unwrap();
        let started = Instant::now();
        terminate_grid_process_tree(&mut child);
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(child.try_wait().unwrap().is_some());
    }
}

#[cfg(test)]
mod grid_connection_generation_tests {
    use super::*;

    #[test]
    fn late_old_reader_must_not_disconnect_new_socket_or_cancel_new_requests() {
        let comms = ChildComms::new();
        let (old_socket, _old_peer) = UnixStream::pair().unwrap();
        let old_generation = comms.attach(old_socket);
        assert!(comms.is_current(old_generation));

        let (old_tx, old_rx) = mpsc::channel();
        comms.pending.lock().unwrap().insert(1, old_tx);
        let (new_socket, _new_peer) = UnixStream::pair().unwrap();
        let new_generation = comms.attach(new_socket);
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
}
