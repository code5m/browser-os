//! M3.a · 终端输出管道与生命周期内核
//!
//! 冻结依据：`logs/checkpoints/M3-20260905-2030.md` F1~F8（WBS M3-1 / M3-2 / M3-3 主体）。
//!
//! 管线形态（F1）：
//! ```text
//! PTY reader ──worker 线程──▶ sync_channel(N) ──pump 线程──▶ TermOutput（Event 广播 / Channel 单播）
//! ```
//! - **worker 永不阻塞**：队列满即丢弃当前块并计数（F2），测量模式改为阻塞发送（禁用丢弃）。
//! - **pump 聚合**：16 ms 窗口或 64 KiB 到点即发（F3），测量模式窗口为 0（立即发）。
//! - **退避**：发送失败指数退避，上限 30 s、总预算 60 s；超预算置 `stop` + 杀进程组（F8）。
//! - **关闭协议**：`stop` 标志 + 队列 EOF 双条件退出，两线程均不泄漏（F7）。
//! - **进程组回收**：unix 下 `setsid` 使 `pid == pgid`，`SIGTERM → 轮询 → SIGKILL`（F6）。
//!
//! 零新依赖：只用 `std::sync::mpsc` + `std::thread` + 既有 `libc`（F9）。

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};

use crate::script_runner::process_group_alive;

// ----------------------------- 冻结常量 -----------------------------

/// 读缓冲（与既有实现一致）。
pub const TERM_READ_BUF_BYTES: usize = 4096;
/// 输出队列容量（F1）。
pub const TERM_QUEUE_CAPACITY: usize = 128;
/// 测量模式队列容量（F2：`driver=term-throughput` 下不丢帧）。
pub const TERM_QUEUE_CAPACITY_MEASURE: usize = 4096;
/// pump 聚合窗口（F3）。
pub const TERM_FLUSH_INTERVAL_MS: u64 = 16;
/// pump 聚合上限（F3）。
pub const TERM_BATCH_MAX_BYTES: usize = 64 * 1024;
/// 丢弃累计到多少块即上报一次（F2）。
pub const TERM_DROP_NOTIFY_CHUNKS: u64 = 64;
/// 丢弃上报的最小时间间隔（F2）。
pub const TERM_DROP_NOTIFY_INTERVAL_MS: u64 = 1_000;
/// 退避基数（F8）。
pub const TERM_RETRY_BASE_MS: u64 = 10;
/// 单次退避上限（F8）。
pub const TERM_RETRY_MAX_BACKOFF_MS: u64 = 30_000;
/// 退避总预算，超出即判通道不可恢复（F8）。
pub const TERM_RETRY_BUDGET_MS: u64 = 60_000;
/// `SIGTERM` 到 `SIGKILL` 的宽限（F6）。
pub const TERM_GROUP_GRACE_MS: u64 = 2_000;
/// 进程组存活轮询间隔（F6）。
pub const TERM_GROUP_POLL_INTERVAL_MS: u64 = 50;
/// 线程回收等待上限（F7）；超时不 join，避免阻塞退出路径。
pub const TERM_THREAD_JOIN_GRACE_MS: u64 = 2_000;

// ----------------------------- 输出契约 -----------------------------

/// 终端输出消息（前端按 `kind` 分发；`flow`/`exit` 不进终端字节流语义）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermMessage {
    /// PTY 原始输出。
    Data(String),
    /// 丢弃统计（F2：仅告知，不插入伪输出）。
    Flow {
        dropped_chunks: u64,
        dropped_bytes: u64,
    },
    /// 会话结束。
    Exit(TermExitReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermExitReason {
    /// PTY 读到 EOF（正常退出）。
    Eof,
    /// 输出通道退避超预算，已强制终止（F8）。
    ChannelDead,
}

impl TermExitReason {
    pub fn as_str(self) -> &'static str {
        match self {
            TermExitReason::Eof => "eof",
            TermExitReason::ChannelDead => "channel_dead",
        }
    }

    /// 附带给前端的可读文本（仅在 `exit` 帧里出现，用户可见）。
    pub fn text(self) -> &'static str {
        match self {
            TermExitReason::Eof => "\r\n[终端已退出]\r\n",
            TermExitReason::ChannelDead => "\r\n[输出通道已断开，终端已终止]\r\n",
        }
    }
}

impl TermMessage {
    /// 序列化为前端契约：`{ id, kind, ... }`。
    pub fn to_value(&self, id: &str) -> serde_json::Value {
        match self {
            TermMessage::Data(data) => json!({ "id": id, "kind": "data", "data": data }),
            TermMessage::Flow {
                dropped_chunks,
                dropped_bytes,
            } => json!({
                "id": id,
                "kind": "flow",
                "dropped_chunks": dropped_chunks,
                "dropped_bytes": dropped_bytes,
            }),
            TermMessage::Exit(reason) => json!({
                "id": id,
                "kind": "exit",
                "reason": reason.as_str(),
                "data": reason.text(),
            }),
        }
    }
}

/// 输出目标抽象（F4）：Event 全局广播（内部驱动/兼容）与 Channel 单播（前端）。
pub trait TermOutput: Send + Sync + 'static {
    fn send(&self, value: &serde_json::Value) -> Result<(), String>;
}

/// Event 广播 sink：`term-data` 全局事件（M0 内部驱动路径，行为与改造前一致）。
pub struct EventSink(pub AppHandle);

impl TermOutput for EventSink {
    fn send(&self, value: &serde_json::Value) -> Result<(), String> {
        self.0
            .emit("term-data", value.clone())
            .map_err(|e| e.to_string())
    }
}

/// Channel 单播 sink：每终端独立通道（F4）。
pub struct ChannelSink(pub Channel<serde_json::Value>);

impl TermOutput for ChannelSink {
    fn send(&self, value: &serde_json::Value) -> Result<(), String> {
        self.0.send(value.clone()).map_err(|e| e.to_string())
    }
}

// ----------------------------- 纯策略（可单测） -----------------------------

/// 是否应当冲刷：达到批量上限，或聚合窗口已到（F3）。
pub fn should_flush(batch_len: usize, elapsed_ms: u64, flush_interval_ms: u64) -> bool {
    batch_len >= TERM_BATCH_MAX_BYTES || elapsed_ms >= flush_interval_ms
}

/// 指数退避：基数 × 2^failures，封顶 `TERM_RETRY_MAX_BACKOFF_MS`（F8）。
pub fn next_backoff_ms(base_ms: u64, failures: u32) -> u64 {
    let shift = failures.min(20);
    base_ms
        .saturating_mul(1u64 << shift)
        .min(TERM_RETRY_MAX_BACKOFF_MS)
}

/// 退避总预算判定（F8）。
pub fn within_retry_budget(elapsed_ms: u64) -> bool {
    elapsed_ms < TERM_RETRY_BUDGET_MS
}

/// 丢弃计数器（F2）：只统计与上报，不向终端字节流插入任何内容。
#[derive(Debug, Default)]
pub struct DropCounter {
    pub chunks: u64,
    pub bytes: u64,
    notified_chunks: u64,
    last_notify: Option<Instant>,
}

impl DropCounter {
    /// 记录一次丢弃，返回是否应当上报。
    pub fn record(&mut self, bytes: usize) -> bool {
        self.chunks = self.chunks.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes as u64);
        let pending = self.chunks.saturating_sub(self.notified_chunks);
        let by_count = pending >= TERM_DROP_NOTIFY_CHUNKS;
        let by_time = match self.last_notify {
            None => true,
            Some(at) => at.elapsed().as_millis() as u64 >= TERM_DROP_NOTIFY_INTERVAL_MS,
        };
        if by_count || by_time {
            self.notified_chunks = self.chunks;
            self.last_notify = Some(Instant::now());
            true
        } else {
            false
        }
    }
}

// ----------------------------- 管道 -----------------------------

/// worker / pump 两个线程的句柄（F7：退出后可 join，不泄漏）。
pub struct TerminalHandles {
    pub worker: JoinHandle<()>,
    pub pump: JoinHandle<()>,
}

/// 启动输出管道。参数化队列容量 / 聚合窗口 / 是否可丢弃，便于测量模式切换与单测。
pub fn start_pipeline(
    reader: Box<dyn Read + Send>,
    out: Arc<dyn TermOutput>,
    id: String,
    stop: Arc<AtomicBool>,
    capacity: usize,
    flush_interval_ms: u64,
    lossy: bool,
    on_channel_dead: Option<Arc<dyn Fn() + Send + Sync>>,
) -> TerminalHandles {
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(capacity);
    let drops = Arc::new(Mutex::new(DropCounter::default()));

    let w_tx: SyncSender<Vec<u8>> = tx;
    let w_stop = Arc::clone(&stop);
    let w_drops = Arc::clone(&drops);
    let worker = thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; TERM_READ_BUF_BYTES];
        loop {
            if w_stop.load(Ordering::Relaxed) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = buf[..n].to_vec();
                    if lossy {
                        // F1/F2：worker 永不阻塞；队列满即丢弃当前块并计数。
                        match w_tx.try_send(chunk) {
                            Ok(()) => {}
                            Err(TrySendError::Full(dropped)) => {
                                // 是否上报由 pump 侧的 notify_drops 定时取走决定。
                                let _ = w_drops.lock().map(|mut g| g.record(dropped.len()));
                            }
                            Err(TrySendError::Disconnected(_)) => break,
                        }
                    } else if w_tx.send(chunk).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let p_stop = Arc::clone(&stop);
    let p_drops = Arc::clone(&drops);
    let pump = thread::spawn(move || {
        run_pump(
            rx,
            out,
            id,
            p_stop,
            p_drops,
            flush_interval_ms,
            on_channel_dead,
        );
    });

    TerminalHandles { worker, pump }
}

fn run_pump(
    rx: Receiver<Vec<u8>>,
    out: Arc<dyn TermOutput>,
    id: String,
    stop: Arc<AtomicBool>,
    drops: Arc<Mutex<DropCounter>>,
    flush_interval_ms: u64,
    on_channel_dead: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    // 测量模式窗口为 0；recv_timeout(0) 会忙轮询，故下限取 1 ms。
    let wait = Duration::from_millis(flush_interval_ms.max(1));
    let mut batch: Vec<u8> = Vec::with_capacity(TERM_BATCH_MAX_BYTES);
    let mut last_flush = Instant::now();
    let mut ok = true;

    loop {
        match rx.recv_timeout(wait) {
            Ok(chunk) => {
                batch.extend_from_slice(&chunk);
                let elapsed = last_flush.elapsed().as_millis() as u64;
                if should_flush(batch.len(), elapsed, flush_interval_ms) {
                    if !flush_batch(&out, &id, &mut batch, &mut last_flush) {
                        ok = false;
                        break;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // 窗口到点：先排空队列，再冲刷残余，避免小块滞留。
                while let Ok(chunk) = rx.try_recv() {
                    batch.extend_from_slice(&chunk);
                }
                notify_drops(&out, &id, &drops);
                if !flush_batch(&out, &id, &mut batch, &mut last_flush) {
                    ok = false;
                    break;
                }
                if stop.load(Ordering::Relaxed) && batch.is_empty() {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                notify_drops(&out, &id, &drops);
                if !flush_batch(&out, &id, &mut batch, &mut last_flush) {
                    ok = false;
                }
                break;
            }
        }
    }

    let reason = if ok {
        TermExitReason::Eof
    } else {
        // F8：超预算即停止——置 stop、杀进程组、发最后一帧（不再退避，避免 60 s 挂起）。
        eprintln!("[terminal] {id} 输出通道不可恢复，已终止会话");
        stop.store(true, Ordering::SeqCst);
        if let Some(kill) = on_channel_dead {
            kill();
        }
        TermExitReason::ChannelDead
    };
    let exit = TermMessage::Exit(reason).to_value(&id);
    let _ = out.send(&exit);
}

fn flush_batch(
    out: &Arc<dyn TermOutput>,
    id: &str,
    batch: &mut Vec<u8>,
    last_flush: &mut Instant,
) -> bool {
    if batch.is_empty() {
        return true;
    }
    let text = String::from_utf8_lossy(batch).to_string();
    batch.clear();
    *last_flush = Instant::now();
    send_with_backoff(out, id, TermMessage::Data(text))
}

fn notify_drops(out: &Arc<dyn TermOutput>, id: &str, drops: &Arc<Mutex<DropCounter>>) {
    let (chunks, bytes) = {
        let mut guard = match drops.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if guard.chunks == 0 {
            return;
        }
        let snapshot = (guard.chunks, guard.bytes);
        guard.chunks = 0;
        guard.bytes = 0;
        guard.notified_chunks = 0;
        snapshot
    };
    let value = TermMessage::Flow {
        dropped_chunks: chunks,
        dropped_bytes: bytes,
    }
    .to_value(id);
    // 控制帧失败不重试：丢弃统计是尽力告知，不得拖死 pump。
    let _ = out.send(&value);
}

fn send_with_backoff(out: &Arc<dyn TermOutput>, id: &str, msg: TermMessage) -> bool {
    let value = msg.to_value(id);
    let mut failures: u32 = 0;
    let started = Instant::now();
    loop {
        match out.send(&value) {
            Ok(()) => return true,
            Err(error) => {
                let elapsed = started.elapsed().as_millis() as u64;
                if !within_retry_budget(elapsed) {
                    eprintln!("[terminal] {id} 输出发送失败且退避超预算: {error}");
                    return false;
                }
                let backoff = next_backoff_ms(TERM_RETRY_BASE_MS, failures);
                failures = failures.saturating_add(1);
                thread::sleep(Duration::from_millis(backoff));
            }
        }
    }
}

// ----------------------------- 会话与生命周期 -----------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TermInfo {
    pub id: String,
}

/// 终端会话：持有 PTY 写入端、master（resize）、子进程、进程组与两个线程句柄。
pub struct TerminalSession {
    pub writer: Box<dyn std::io::Write + Send>,
    /// F5：持有 master 才能真实 resize。
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send + Sync>,
    /// F6：unix 下 = shell pid（`setsid` 后 `pid == pgid`）；非 unix 为 None（降级）。
    pub pgid: Option<i32>,
    /// F7：置位后 worker/pump 在排空后退出。
    pub stop: Arc<AtomicBool>,
    pub handles: Option<TerminalHandles>,
}

/// 创建 PTY 会话并启动输出管道。`measure = true` 走 M0 测量直通（不合并、不丢帧）。
pub fn spawn_terminal(
    out: Arc<dyn TermOutput>,
    measure: bool,
) -> Result<(TermInfo, TerminalSession), String> {
    let id = format!("term-{}", uuid::Uuid::new_v4());
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("无法创建 PTY: {e}"))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = CommandBuilder::new(shell);
    // xterm.js 是完整终端模拟器，需要正常 TERM 与 ANSI 序列，不能过滤。
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("无法启动 shell: {e}"))?;
    drop(pair.slave);

    let master = pair.master;
    let mut writer = match master.take_writer() {
        Ok(writer) => writer,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("writer: {error}"));
        }
    };
    let reader = match master.try_clone_reader() {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("reader: {error}"));
        }
    };

    let pgid = pgid_of(&child);
    let stop = Arc::new(AtomicBool::new(false));
    let capacity = if measure {
        TERM_QUEUE_CAPACITY_MEASURE
    } else {
        TERM_QUEUE_CAPACITY
    };
    // F2/F3：测量模式不合并（窗口 0）、不丢帧（lossy = false）。
    let flush_interval_ms = if measure { 0 } else { TERM_FLUSH_INTERVAL_MS };
    let on_dead: Option<Arc<dyn Fn() + Send + Sync>> = pgid.map(|pgid| {
        Arc::new(move || kill_group(pgid, libc::SIGKILL)) as Arc<dyn Fn() + Send + Sync>
    });

    let handles = start_pipeline(
        reader,
        out,
        id.clone(),
        Arc::clone(&stop),
        capacity,
        flush_interval_ms,
        !measure,
        on_dead,
    );

    // 触发初始提示符
    let _ = writer.write_all(b"\n");
    let _ = writer.flush();

    Ok((
        TermInfo { id },
        TerminalSession {
            writer,
            master,
            child,
            pgid,
            stop,
            handles: Some(handles),
        },
    ))
}

/// 真实 resize（F5）。
pub fn resize(session: &mut TerminalSession, cols: u16, rows: u16) -> Result<(), String> {
    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("resize 失败: {e}"))
}

/// 终止会话：置 `stop` → 杀进程组 → 回收线程（F6/F7/F8）。
pub fn terminate_session(session: &mut TerminalSession) -> Result<(), String> {
    session.stop.store(true, Ordering::SeqCst);
    let mut errors = Vec::new();
    if let Err(error) = terminate_group(&mut session.child, session.pgid, TERM_GROUP_GRACE_MS) {
        errors.push(error);
    }
    if let Some(handles) = session.handles.take() {
        join_pipeline(handles, TERM_THREAD_JOIN_GRACE_MS);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// 进程组回收（F6）：`SIGTERM` → 轮询存活 → 仍活则 `SIGKILL` → wait。
pub fn terminate_group(
    child: &mut Box<dyn Child + Send + Sync>,
    pgid: Option<i32>,
    grace_ms: u64,
) -> Result<(), String> {
    match pgid {
        #[cfg(unix)]
        Some(pgid) => {
            kill_group(pgid, libc::SIGTERM);
            let deadline = Instant::now() + Duration::from_millis(grace_ms);
            while Instant::now() < deadline {
                if !process_group_alive(pgid) {
                    break;
                }
                thread::sleep(Duration::from_millis(TERM_GROUP_POLL_INTERVAL_MS));
            }
            if process_group_alive(pgid) {
                kill_group(pgid, libc::SIGKILL);
            }
        }
        // 非 unix / 无进程组：降级为杀直接子进程（与改造前一致）。
        None => {
            let _ = child.kill();
        }
    }
    child
        .wait()
        .map(|_| ())
        .map_err(|e| format!("终端等待失败: {e}"))
}

#[cfg(unix)]
fn pgid_of(child: &Box<dyn Child + Send + Sync>) -> Option<i32> {
    // portable-pty unix 路径在子进程中 setsid()，故 pid == pgid == sid。
    child.process_id().map(|pid| pid as i32)
}

#[cfg(not(unix))]
fn pgid_of(_child: &Box<dyn Child + Send + Sync>) -> Option<i32> {
    None
}

#[cfg(unix)]
fn kill_group(pgid: i32, sig: i32) {
    unsafe {
        libc::killpg(pgid, sig);
    }
}

#[cfg(not(unix))]
fn kill_group(_pgid: i32, _sig: i32) {}

/// 等待 worker/pump 结束；超时则不再 join（退出路径不得被线程拖住）。
fn join_pipeline(handles: TerminalHandles, grace_ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(grace_ms);
    while Instant::now() < deadline {
        if handles.worker.is_finished() && handles.pump.is_finished() {
            break;
        }
        thread::sleep(Duration::from_millis(TERM_GROUP_POLL_INTERVAL_MS));
    }
    if handles.worker.is_finished() {
        let _ = handles.worker.join();
    } else {
        eprintln!("[terminal] worker 线程未在 {grace_ms} ms 内结束，放弃 join");
    }
    if handles.pump.is_finished() {
        let _ = handles.pump.join();
    } else {
        eprintln!("[terminal] pump 线程未在 {grace_ms} ms 内结束，放弃 join");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Sender;

    /// 测试用 sink：收集消息，不依赖 Tauri 运行时。
    struct CollectSink {
        tx: Mutex<Sender<(String, String)>>,
    }

    impl TermOutput for CollectSink {
        fn send(&self, value: &serde_json::Value) -> Result<(), String> {
            let kind = value
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let data = value
                .get("data")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            self.tx
                .lock()
                .unwrap()
                .send((kind, data))
                .map_err(|e| e.to_string())
        }
    }

    struct NoopSink;

    impl TermOutput for NoopSink {
        fn send(&self, _value: &serde_json::Value) -> Result<(), String> {
            Ok(())
        }
    }

    /// T4备选：sink 契约最少实现（编译期保证 trait 可被任意实现者满足）。
    #[test]
    fn term_output_is_object_safe() {
        let sink: Arc<dyn TermOutput> = Arc::new(NoopSink);
        assert!(sink.send(&json!({"kind":"data"})).is_ok());
    }

    /// T1：合并策略（F3）。
    #[test]
    fn flush_policy_boundaries() {
        assert!(should_flush(0, 16, 16), "窗口到点即发");
        assert!(
            should_flush(TERM_BATCH_MAX_BYTES, 0, 16),
            "达到批量上限即发"
        );
        assert!(!should_flush(10, 5, 16), "未满未到点不刷");
        assert!(should_flush(1, 0, 0), "测量模式窗口 0：任何字节立即发");
    }

    /// T1：退避序列（F8）。
    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(next_backoff_ms(TERM_RETRY_BASE_MS, 0), 10);
        assert_eq!(next_backoff_ms(TERM_RETRY_BASE_MS, 1), 20);
        assert_eq!(next_backoff_ms(TERM_RETRY_BASE_MS, 2), 40);
        assert_eq!(
            next_backoff_ms(TERM_RETRY_BASE_MS, 30),
            TERM_RETRY_MAX_BACKOFF_MS
        );
        assert!(within_retry_budget(0));
        assert!(!within_retry_budget(TERM_RETRY_BUDGET_MS));
    }

    /// T6：丢弃计数与上报节流（F2）。
    #[test]
    fn drop_counter_reports_periodically() {
        let mut counter = DropCounter::default();
        assert!(counter.record(10), "首次丢弃立即上报");
        assert_eq!(counter.chunks, 1);
        assert_eq!(counter.bytes, 10);
        assert!(!counter.record(10), "未到块数/时间阈值不上报");
        let mut reported = false;
        for _ in 0..TERM_DROP_NOTIFY_CHUNKS {
            if counter.record(1) {
                reported = true;
            }
        }
        assert!(reported, "累计到块数阈值后应上报一次");
        assert!(counter.chunks > TERM_DROP_NOTIFY_CHUNKS);
        assert_eq!(counter.bytes, 20 + TERM_DROP_NOTIFY_CHUNKS as u64);
    }

    /// T2 + T5：真实 PTY —— 输出经 mpsc+pump 到达 sink；`stop` 后两线程结束（F1/F7）。
    #[test]
    fn pipeline_delivers_real_pty_output_and_stops() {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");
        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.env("TERM", "xterm-256color");
        let mut child = pair.slave.spawn_command(cmd).expect("spawn sh");
        drop(pair.slave);
        let master = pair.master;
        let mut writer = master.take_writer().expect("writer");
        let reader = master.try_clone_reader().expect("reader");

        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let handles = start_pipeline(
            reader,
            Arc::new(CollectSink { tx: Mutex::new(tx) }),
            "term-test".to_string(),
            Arc::clone(&stop),
            TERM_QUEUE_CAPACITY,
            TERM_FLUSH_INTERVAL_MS,
            true,
            None,
        );

        writer.write_all(b"echo __T_OK__\n").expect("write to pty");
        writer.flush().expect("flush");

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seen = String::new();
        while Instant::now() < deadline {
            while let Ok((kind, data)) = rx.try_recv() {
                if kind == "data" {
                    seen.push_str(&data);
                }
            }
            if seen.contains("__T_OK__") {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(seen.contains("__T_OK__"), "真实 PTY 输出应到达 sink");

        // T5：置 stop 后两个线程都应结束（worker 读到 stop，pump 排空后退出）。
        stop.store(true, Ordering::SeqCst);
        let _ = child.kill();
        let _ = child.wait();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if handles.worker.is_finished() && handles.pump.is_finished() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(handles.worker.is_finished(), "worker 线程应结束");
        assert!(handles.pump.is_finished(), "pump 线程应结束");
    }

    /// T4：真实 resize —— `master.resize` 之后 PTY 尺寸确实变了（F5 的运行时取证，
    /// 静态夹具只能证明代码里调了 resize，证明不了它生效）。
    #[test]
    fn resize_changes_real_pty_size() {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");
        let cmd = CommandBuilder::new("/bin/sh");
        let child = pair.slave.spawn_command(cmd).expect("spawn sh");
        drop(pair.slave);
        let master = pair.master;
        let writer = master.take_writer().expect("writer");
        let pgid = pgid_of(&child);
        let mut session = TerminalSession {
            writer,
            master,
            child,
            pgid,
            stop: Arc::new(AtomicBool::new(false)),
            handles: None,
        };

        resize(&mut session, 120, 40).expect("resize 应成功（不再是空实现）");
        let size = session.master.get_size().expect("get_size");
        assert_eq!(size.cols, 120, "列数应真实生效");
        assert_eq!(size.rows, 40, "行数应真实生效");

        // 收口：杀进程组，避免测试残留孤儿 shell。
        terminate_session(&mut session).expect("terminate_session");
    }

    /// T3：进程组回收 —— `sleep 300` 随 shell 一起被杀，不留孤儿（F6）。
    #[test]
    fn terminate_group_kills_whole_process_group() {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");
        let cmd = CommandBuilder::new("/bin/sh");
        let mut child = pair.slave.spawn_command(cmd).expect("spawn sh");
        drop(pair.slave);
        let master = pair.master;
        let mut writer = master.take_writer().expect("writer");
        writer.write_all(b"sleep 300\n").expect("write");
        writer.flush().expect("flush");
        // 等 shell 把 sleep 拉起来
        thread::sleep(Duration::from_millis(300));

        let pgid = pgid_of(&child).expect("unix 下应取到 pid/pgid");
        assert!(process_group_alive(pgid), "kill 前进程组应存活");

        terminate_group(&mut child, Some(pgid), TERM_GROUP_GRACE_MS).expect("terminate_group");
        assert!(
            !process_group_alive(pgid),
            "kill 后进程组不应存活（sleep 不得成为孤儿）"
        );
        assert!(
            !std::path::Path::new(&format!("/proc/{pgid}")).exists(),
            "shell 进程目录应已消失"
        );
    }

    /// 契约：`TermMessage::to_value` 的三种形态（前端按 kind 分发）。
    #[test]
    fn term_message_payload_contract() {
        let data = TermMessage::Data("hi".to_string()).to_value("term-1");
        assert_eq!(data["id"], "term-1");
        assert_eq!(data["kind"], "data");
        assert_eq!(data["data"], "hi");

        let flow = TermMessage::Flow {
            dropped_chunks: 3,
            dropped_bytes: 30,
        }
        .to_value("term-1");
        assert_eq!(flow["kind"], "flow");
        assert_eq!(flow["dropped_chunks"], 3);

        let exit = TermMessage::Exit(TermExitReason::Eof).to_value("term-1");
        assert_eq!(exit["kind"], "exit");
        assert_eq!(exit["reason"], "eof");
    }
}
