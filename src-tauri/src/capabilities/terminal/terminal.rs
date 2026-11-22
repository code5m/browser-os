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
//! - **退避**：指数退避有总预算，所有等待每 20 ms 响应关闭（F8）。
//! - **关闭协议**：停止输入与发送 → 终止进程树 → 释放 master/ConPTY → 回收线程。
//!   Windows reader 在关闭期间继续排空管道；未完成清理由 reaper 持有并明确报告 pending。
//! - **进程组回收**：unix 下 `setsid` 使 `pid == pgid`，`SIGTERM → 轮询 → SIGKILL`（F6）。
//!
//! 零新依赖：只用 `std::sync::mpsc` + `std::thread` + 既有 `libc`（F9）。

use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
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
/// 前台等待清理的预算；超时后资源和 JoinHandle 仍由 cleanup reaper 持有。
pub const TERM_THREAD_JOIN_GRACE_MS: u64 = 2_000;
/// Per-session input is bounded by both message size and queue count (1 MiB).
pub const TERM_INPUT_MAX_BYTES: usize = 64 * 1024;
pub const TERM_INPUT_QUEUE_CAPACITY: usize = 16;
/// All retry/queue waits observe stop within this interval.
pub const TERM_CANCEL_POLL_MS: u64 = 20;
const TERM_RECENT_CLEANUP_RESULTS: usize = 64;
/// Active and still-cleaning sessions share one resource budget.
pub const MAX_TERMINAL_SESSIONS: usize = 32;
/// Admission and shutdown use the same lock. A permit covers a spawn still
/// inside the OS, a published session, and all of its retained cleanup owners.
#[derive(Default)]
struct TerminalAdmission {
    leases: Mutex<Vec<Weak<TerminalPermitLease>>>,
    closed: AtomicBool,
    active: AtomicUsize,
    changed: Condvar,
}

fn terminal_admission() -> &'static Arc<TerminalAdmission> {
    static ADMISSION: OnceLock<Arc<TerminalAdmission>> = OnceLock::new();
    ADMISSION.get_or_init(|| Arc::new(TerminalAdmission::default()))
}

#[derive(Clone)]
struct TerminalPermit { _lease: Arc<TerminalPermitLease> }
struct TerminalPermitLease {
    admission: Arc<TerminalAdmission>,
    stop: Arc<AtomicBool>,
}

impl TerminalAdmission {
    fn acquire(self: &Arc<Self>) -> Result<TerminalPermit, String> {
        let mut leases = self.leases.lock().unwrap_or_else(|e| e.into_inner());
        if self.closed.load(Ordering::Acquire) { return Err("TERM_SHUTTING_DOWN".into()); }
        leases.retain(|lease| lease.strong_count() != 0);
        if self.active.load(Ordering::Acquire) >= MAX_TERMINAL_SESSIONS {
            return Err("TERM_SESSION_LIMIT".into());
        }
        let lease = Arc::new(TerminalPermitLease {
            admission: Arc::clone(self), stop: Arc::new(AtomicBool::new(false)),
        });
        self.active.fetch_add(1, Ordering::AcqRel);
        leases.push(Arc::downgrade(&lease));
        Ok(TerminalPermit { _lease: lease })
    }

    fn request_shutdown(&self) {
        let leases = {
            let leases = self.leases.lock().unwrap_or_else(|e| e.into_inner());
            self.closed.store(true, Ordering::Release);
            leases.iter().filter_map(Weak::upgrade).collect::<Vec<_>>()
        };
        // Do not perform PTY I/O or wait under the admission/table lock.
        // Reserved spawns see this same stop token after any in-flight OS call.
        for lease in leases { lease.stop.store(true, Ordering::Release); }
        self.changed.notify_all();
    }
}

impl TerminalPermit {
    fn acquire() -> Result<Self, String> { terminal_admission().acquire() }
    fn stop(&self) -> Arc<AtomicBool> { Arc::clone(&self._lease.stop) }
}

impl Drop for TerminalPermitLease {
    fn drop(&mut self) {
        self.admission.active.fetch_sub(1, Ordering::AcqRel);
        self.admission.changed.notify_all();
    }
}

// ----------------------------- 诊断 Probe（默认关闭） -----------------------------
//
// 开关：`MVP_TERMINAL_PROBE=1`（进程启动时读一次并缓存）。
// 只记录元数据（序号/字节数/行列/计数/水位），绝不记录终端正文、命令或任何凭据。
// 输出走 stderr 的 `[TERM_PROBE]` 前缀，与前端 `debug_log`（`[FE]` 前缀）同流，
// dev 模式下可对账五层：PTY read → 队列 → flush(utf8) → Channel send → 前端/xterm。

/// probe 开关：进程级缓存（热路径零 env 读取开销）。
pub fn probe_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("MVP_TERMINAL_PROBE").as_deref() == Ok("1"))
}

/// 每终端元数据计数（worker/pump 共享；无正文）。
#[derive(Default)]
pub struct TermProbe {
    pub read_chunks: AtomicU64,
    pub read_bytes: AtomicU64,
    pub enq_chunks: AtomicU64,
    pub enq_bytes: AtomicU64,
    pub drop_chunks: AtomicU64,
    pub drop_bytes: AtomicU64,
    pub queue_full: AtomicU64,
    /// 队列占用近似水位（enqueue +1 / dequeue -1），用于 high watermark。
    pub gauge: AtomicI64,
    pub high_watermark: AtomicI64,
    pub flush_seq: AtomicU64,
    pub flush_bytes: AtomicU64,
    pub send_ok: AtomicU64,
    pub send_fail: AtomicU64,
    pub send_bytes: AtomicU64,
}

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
    /// Input was accepted by IPC but the native writer could not deliver it.
    InputFailed,
}

impl TermExitReason {
    pub fn as_str(self) -> &'static str {
        match self {
            TermExitReason::Eof => "eof",
            TermExitReason::ChannelDead => "channel_dead",
            TermExitReason::InputFailed => "input_failed",
        }
    }

    /// 附带给前端的可读文本（仅在 `exit` 帧里出现，用户可见）。
    pub fn text(self) -> &'static str {
        match self {
            TermExitReason::Eof => "\r\n[终端已退出]\r\n",
            TermExitReason::ChannelDead => "\r\n[输出通道已断开，终端已终止]\r\n",
            TermExitReason::InputFailed => "\r\n[输入传输失败，终端已停止；部分输入可能未发送，请核对后再重试]\r\n",
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

pub fn start_pipeline(
    reader: Box<dyn Read + Send>, out: Arc<dyn TermOutput>, id: String,
    stop: Arc<AtomicBool>, capacity: usize, flush_interval_ms: u64, lossy: bool,
    on_channel_dead: Option<Arc<dyn Fn() + Send + Sync>>, probe: Arc<TermProbe>,
) -> TerminalHandles {
    start_pipeline_with_failure(reader, out, id, stop, capacity, flush_interval_ms,
        lossy, on_channel_dead, probe, Arc::new(AtomicBool::new(false)))
}

/// 启动输出管道。参数化队列容量 / 聚合窗口 / 是否可丢弃，便于测量模式切换与单测。
fn start_pipeline_with_failure(
    reader: Box<dyn Read + Send>,
    out: Arc<dyn TermOutput>,
    id: String,
    stop: Arc<AtomicBool>,
    capacity: usize,
    flush_interval_ms: u64,
    lossy: bool,
    on_channel_dead: Option<Arc<dyn Fn() + Send + Sync>>,
    probe: Arc<TermProbe>,
    input_failed: Arc<AtomicBool>,
) -> TerminalHandles {
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(capacity);
    let drops = Arc::new(Mutex::new(DropCounter::default()));

    let w_tx: SyncSender<Vec<u8>> = tx;
    let w_stop = Arc::clone(&stop);
    let w_drops = Arc::clone(&drops);
    let w_id = id.clone();
    let w_probe = Arc::clone(&probe);
    let worker = thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; TERM_READ_BUF_BYTES];
        let probe_on = probe_enabled();
        loop {
            // On older Windows versions ClosePseudoConsole waits for output
            // consumption. Keep reading (and discarding) after stop until the
            // close owner releases HPCON; never make close wait for this join.
            #[cfg(not(windows))]
            if w_stop.load(Ordering::Acquire) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if w_stop.load(Ordering::Acquire) {
                        continue;
                    }
                    if probe_on {
                        let seq = w_probe.read_chunks.fetch_add(1, Ordering::Relaxed) + 1;
                        let cum =
                            w_probe.read_bytes.fetch_add(n as u64, Ordering::Relaxed) + n as u64;
                        eprintln!("[TERM_PROBE] {w_id} pty.read seq={seq} bytes={n} cum={cum}");
                    }
                    let chunk = buf[..n].to_vec();
                    if lossy {
                        // F1/F2：worker 永不阻塞；队列满即丢弃当前块并计数。
                        match w_tx.try_send(chunk) {
                            Ok(()) => {
                                if probe_on {
                                    w_probe.enq_chunks.fetch_add(1, Ordering::Relaxed);
                                    w_probe.enq_bytes.fetch_add(n as u64, Ordering::Relaxed);
                                    let g = w_probe.gauge.fetch_add(1, Ordering::Relaxed) + 1;
                                    w_probe.high_watermark.fetch_max(g, Ordering::Relaxed);
                                }
                            }
                            Err(TrySendError::Full(dropped)) => {
                                if probe_on {
                                    w_probe.queue_full.fetch_add(1, Ordering::Relaxed);
                                    w_probe.drop_chunks.fetch_add(1, Ordering::Relaxed);
                                    w_probe
                                        .drop_bytes
                                        .fetch_add(dropped.len() as u64, Ordering::Relaxed);
                                }
                                // 是否上报由 pump 侧的 notify_drops 定时取走决定。
                                let _ = w_drops.lock().map(|mut g| g.record(dropped.len()));
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                w_stop.store(true, Ordering::Release);
                            }
                        }
                    } else {
                        // Measurement mode remains lossless while running,
                        // but a full queue must not prevent shutdown/draining.
                        let mut pending = chunk;
                        loop {
                            if w_stop.load(Ordering::Acquire) { break; }
                            match w_tx.try_send(pending) {
                                Ok(()) => {
                                    if probe_on {
                                        w_probe.enq_chunks.fetch_add(1, Ordering::Relaxed);
                                        w_probe.enq_bytes.fetch_add(n as u64, Ordering::Relaxed);
                                        let g = w_probe.gauge.fetch_add(1, Ordering::Relaxed) + 1;
                                        w_probe.high_watermark.fetch_max(g, Ordering::Relaxed);
                                    }
                                    break;
                                }
                                Err(TrySendError::Full(chunk)) => {
                                    pending = chunk;
                                    if !wait_while_running(&w_stop, Duration::from_millis(TERM_CANCEL_POLL_MS)) {
                                        break;
                                    }
                                }
                                Err(TrySendError::Disconnected(_)) => {
                                    w_stop.store(true, Ordering::Release);
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                    if w_stop.load(Ordering::Acquire) { break; }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if !wait_while_running(&w_stop, Duration::from_millis(TERM_CANCEL_POLL_MS)) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let p_stop = Arc::clone(&stop);
    let p_drops = Arc::clone(&drops);
    let p_probe = Arc::clone(&probe);
    let pump = thread::spawn(move || {
        run_pump(
            rx,
            out,
            id,
            p_stop,
            p_drops,
            flush_interval_ms,
            on_channel_dead,
            p_probe,
            input_failed,
        );
    });

    TerminalHandles { worker, pump }
}

/// Streaming UTF-8 decoding for terminal output. A read/flush boundary is
/// not an encoding boundary: retain only an incomplete suffix (at most three
/// bytes), and replace only sequences proven invalid by from_utf8.
#[derive(Default)]
struct Utf8StreamDecoder {
    tail: Vec<u8>,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct DecodedChunk {
    text: String,
    invalid_sequences: usize,
    truncated_tail_bytes: usize,
}

impl Utf8StreamDecoder {
    fn push(&mut self, bytes: &[u8]) -> DecodedChunk {
        let input = if self.tail.is_empty() {
            Cow::Borrowed(bytes)
        } else {
            let mut joined = std::mem::take(&mut self.tail);
            joined.extend_from_slice(bytes);
            Cow::Owned(joined)
        };
        let mut remaining = input.as_ref();
        let mut decoded = DecodedChunk::default();
        while !remaining.is_empty() {
            match std::str::from_utf8(remaining) {
                Ok(valid) => {
                    decoded.text.push_str(valid);
                    break;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    decoded.text.push_str(
                        std::str::from_utf8(&remaining[..valid]).expect("UTF-8 valid prefix"),
                    );
                    remaining = &remaining[valid..];
                    match error.error_len() {
                        Some(invalid_len) => {
                            decoded.text.push('\u{FFFD}');
                            decoded.invalid_sequences += 1;
                            remaining = &remaining[invalid_len..];
                        }
                        None => {
                            // Incomplete, potentially valid UTF-8. A later
                            // flush may complete it; do not invent corruption.
                            self.tail.extend_from_slice(remaining);
                            break;
                        }
                    }
                }
            }
        }
        decoded
    }

    fn finish(&mut self) -> DecodedChunk {
        // A genuine EOF makes the remaining incomplete sequence truncated.
        // Emit one replacement and distinguish it from invalid interior bytes.
        let truncated_tail_bytes = self.tail.len();
        self.tail.clear();
        DecodedChunk {
            text: if truncated_tail_bytes == 0 { String::new() } else { "\u{FFFD}".into() },
            invalid_sequences: 0,
            truncated_tail_bytes,
        }
    }

    fn cancel(&mut self) -> usize {
        // Requested close/channel failure discards unsent output. Cancellation
        // must not turn a deliberately abandoned suffix into a false bad byte.
        let discarded = self.tail.len();
        self.tail.clear();
        discarded
    }
}

fn run_pump(
    rx: Receiver<Vec<u8>>,
    out: Arc<dyn TermOutput>,
    id: String,
    stop: Arc<AtomicBool>,
    drops: Arc<Mutex<DropCounter>>,
    flush_interval_ms: u64,
    on_channel_dead: Option<Arc<dyn Fn() + Send + Sync>>,
    probe: Arc<TermProbe>,
    input_failed: Arc<AtomicBool>,
) {
    // 测量模式窗口为 0；recv_timeout(0) 会忙轮询，故下限取 1 ms。
    let wait = Duration::from_millis(flush_interval_ms.max(1));
    let mut batch: Vec<u8> = Vec::with_capacity(TERM_BATCH_MAX_BYTES);
    let mut last_flush = Instant::now();
    let mut decoder = Utf8StreamDecoder::default();
    let mut ok = true;
    let probe_on = probe_enabled();

    loop {
        if stop.load(Ordering::Acquire) { break; }
        match rx.recv_timeout(wait) {
            Ok(chunk) => {
                if probe_on {
                    probe.gauge.fetch_sub(1, Ordering::Relaxed);
                }
                batch.extend_from_slice(&chunk);
                let elapsed = last_flush.elapsed().as_millis() as u64;
                if should_flush(batch.len(), elapsed, flush_interval_ms) {
                    if !flush_batch(&out, &id, &mut batch, &mut last_flush, &probe, &stop, &mut decoder, false) {
                        ok = false;
                        break;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // 窗口到点：先排空队列，再冲刷残余，避免小块滞留。
                while batch.len() < TERM_BATCH_MAX_BYTES && !stop.load(Ordering::Acquire) {
                    let Ok(chunk) = rx.try_recv() else { break; };
                    if probe_on {
                        probe.gauge.fetch_sub(1, Ordering::Relaxed);
                    }
                    batch.extend_from_slice(&chunk);
                }
                notify_drops(&out, &id, &drops, &stop);
                if !flush_batch(&out, &id, &mut batch, &mut last_flush, &probe, &stop, &mut decoder, false) {
                    ok = false;
                    break;
                }
                if stop.load(Ordering::Relaxed) && batch.is_empty() {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                notify_drops(&out, &id, &drops, &stop);
                if !flush_batch(&out, &id, &mut batch, &mut last_flush, &probe, &stop, &mut decoder, true) {
                    ok = false;
                }
                break;
            }
        }
    }

    if !ok || stop.load(Ordering::Acquire) {
        let tail_bytes = decoder.cancel();
        batch.clear();
        if probe_on && tail_bytes != 0 {
            eprintln!("[TERM_PROBE] {id} utf8.cancel discarded_tail_bytes={tail_bytes}");
        }
    }

    if probe_on {
        eprintln!(
            "[TERM_PROBE] {id} summary pty_bytes={} enq_bytes={} drop_chunks={} drop_bytes={} queue_full={} high_watermark={} flush_bytes={} send_ok={} send_fail={} send_bytes={}",
            probe.read_bytes.load(Ordering::Relaxed),
            probe.enq_bytes.load(Ordering::Relaxed),
            probe.drop_chunks.load(Ordering::Relaxed),
            probe.drop_bytes.load(Ordering::Relaxed),
            probe.queue_full.load(Ordering::Relaxed),
            probe.high_watermark.load(Ordering::Relaxed),
            probe.flush_bytes.load(Ordering::Relaxed),
            probe.send_ok.load(Ordering::Relaxed),
            probe.send_fail.load(Ordering::Relaxed),
            probe.send_bytes.load(Ordering::Relaxed),
        );
    }

    let reason = if input_failed.load(Ordering::Acquire) {
        TermExitReason::InputFailed
    } else if ok || stop.load(Ordering::Acquire) {
        TermExitReason::Eof
    } else {
        // F8：超预算即置 stop 并杀进程组；停止后不再进入输出发送。
        eprintln!("[terminal] {id} 输出通道不可恢复，已终止会话");
        stop.store(true, Ordering::SeqCst);
        if let Some(kill) = on_channel_dead {
            kill();
        }
        TermExitReason::ChannelDead
    };
    // Accepted input can fail later in the writer. Report that fixed control
    // frame even if no further keystroke arrives; never include command text
    // or raw OS diagnostics. Other requested closes enter no final send.
    if reason == TermExitReason::InputFailed || !stop.load(Ordering::Acquire) {
        let exit = TermMessage::Exit(reason).to_value(&id);
        let _ = out.send(&exit);
    }
}

fn flush_batch(
    out: &Arc<dyn TermOutput>,
    id: &str,
    batch: &mut Vec<u8>,
    last_flush: &mut Instant,
    probe: &Arc<TermProbe>,
    stop: &AtomicBool,
    decoder: &mut Utf8StreamDecoder,
    eof: bool,
) -> bool {
    if stop.load(Ordering::Acquire) {
        batch.clear();
        decoder.cancel();
        return false;
    }
    if batch.is_empty() && !eof { return true; }
    let probe_on = probe_enabled();
    let raw_len = batch.len();
    let mut decoded = decoder.push(batch);
    batch.clear();
    if eof {
        let tail = decoder.finish();
        decoded.text.push_str(&tail.text);
        decoded.truncated_tail_bytes += tail.truncated_tail_bytes;
    }
    *last_flush = Instant::now();
    if probe_on {
        let seq = probe.flush_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let cum = probe.flush_bytes.fetch_add(raw_len as u64, Ordering::Relaxed) + raw_len as u64;
        let invalid_sequences = decoded.invalid_sequences;
        let truncated_tail_bytes = decoded.truncated_tail_bytes;
        let pending_tail_bytes = decoder.tail.len();
        eprintln!(
            "[TERM_PROBE] {id} flush seq={seq} raw_bytes={raw_len} cum={cum} invalid_sequences={invalid_sequences} truncated_tail_bytes={truncated_tail_bytes} pending_tail_bytes={pending_tail_bytes} enc_bytes={}",
            decoded.text.len()
        );
    }
    // An incomplete suffix alone is not an empty output frame or an error.
    if decoded.text.is_empty() { return true; }
    send_with_backoff(out, id, TermMessage::Data(decoded.text), probe, stop)
}

fn notify_drops(
    out: &Arc<dyn TermOutput>, id: &str, drops: &Arc<Mutex<DropCounter>>, stop: &AtomicBool,
) {
    if stop.load(Ordering::Acquire) { return; }
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

fn send_with_backoff(
    out: &Arc<dyn TermOutput>,
    id: &str,
    msg: TermMessage,
    probe: &Arc<TermProbe>,
    stop: &AtomicBool,
) -> bool {
    let payload_bytes = match &msg {
        TermMessage::Data(d) => d.len(),
        _ => 0,
    };
    let value = msg.to_value(id);
    let mut failures: u32 = 0;
    let started = Instant::now();
    let probe_on = probe_enabled();
    loop {
        if stop.load(Ordering::Acquire) { return false; }
        match out.send(&value) {
            Ok(()) => {
                if probe_on {
                    let seq = probe.send_ok.fetch_add(1, Ordering::Relaxed) + 1;
                    let cum = probe
                        .send_bytes
                        .fetch_add(payload_bytes as u64, Ordering::Relaxed)
                        + payload_bytes as u64;
                    eprintln!(
                        "[TERM_PROBE] {id} chan.send seq={seq} ok=true bytes={payload_bytes} cum={cum}"
                    );
                }
                return true;
            }
            Err(error) => {
                let elapsed = started.elapsed().as_millis() as u64;
                if !within_retry_budget(elapsed) {
                    eprintln!("[terminal] {id} 输出发送失败且退避超预算: {error}");
                    if probe_on {
                        probe.send_fail.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[TERM_PROBE] {id} chan.send ok=false budget_exceeded");
                    }
                    return false;
                }
                let backoff = next_backoff_ms(TERM_RETRY_BASE_MS, failures);
                failures = failures.saturating_add(1);
                let remaining = TERM_RETRY_BUDGET_MS.saturating_sub(elapsed);
                if !wait_while_running(stop, Duration::from_millis(backoff.min(remaining))) {
                    return false;
                }
            }
        }
    }
}

/// Bounded cancellation latency, even at the maximum exponential backoff.
/// The callback itself must return promptly; a stuck foreign callback remains
/// owned by cleanup and is reported as pending rather than detached.
fn wait_while_running(stop: &AtomicBool, duration: Duration) -> bool {
    let deadline = Instant::now() + duration;
    loop {
        if stop.load(Ordering::Acquire) { return false; }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() { return true; }
        thread::sleep(remaining.min(Duration::from_millis(TERM_CANCEL_POLL_MS)));
    }
}

// ----------------------------- 会话与生命周期 -----------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TermInfo {
    pub id: String,
    /// 诊断 probe 是否开启（`MVP_TERMINAL_PROBE=1`）；前端据此启用对账打点。
    pub probe: bool,
}

/// Cloneable input endpoint. An acknowledgement means "accepted into this
/// session's bounded queue", not "already consumed by the shell".
#[derive(Clone)]
pub struct TerminalInput {
    tx: SyncSender<Vec<u8>>,
    stop: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<String>>>,
}

impl TerminalInput {
    pub fn enqueue(&self, bytes: &[u8]) -> Result<(), String> {
        if let Some(error) = self.failure.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            return Err(error.clone());
        }
        if self.stop.load(Ordering::Acquire) {
            return Err("TERM_INPUT_CLOSED".into());
        }
        if bytes.len() > TERM_INPUT_MAX_BYTES {
            return Err(format!("TERM_INPUT_TOO_LARGE: limit={TERM_INPUT_MAX_BYTES}"));
        }
        if bytes.is_empty() { return Ok(()); }
        match self.tx.try_send(bytes.to_vec()) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err("TERM_INPUT_QUEUE_FULL".into()),
            Err(TrySendError::Disconnected(_)) => Err("TERM_INPUT_CLOSED".into()),
        }
    }
}

struct ResizeRequest {
    size: PtySize,
    reply: tokio::sync::oneshot::Sender<Result<(), String>>,
}
#[derive(Default)]
struct ResizeOwner {
    // The running native call owns one request; only the latest next size is retained.
    pending: Mutex<Option<ResizeRequest>>,
    wake: Condvar,
    worker: Mutex<Option<JoinHandle<()>>>,
}

/// A clone never duplicates the master/HPCON. Cleanup takes the sole master
/// out of this slot; concurrent resize callers then observe a closed session.
#[derive(Clone)]
pub struct TerminalControl {
    master: Arc<Mutex<Option<Box<dyn MasterPty + Send>>>>,
    stop: Arc<AtomicBool>,
    resize_owner: Arc<ResizeOwner>,
    admission: Weak<TerminalPermitLease>,
}

impl TerminalControl {
    /// Bound async resize by the existing native session admission. Replies
    /// for replaced pending sizes finish immediately, so callers cannot form
    /// another unbounded wait queue behind a blocked ConPTY call.
    pub fn enqueue_resize(&self, cols: u16, rows: u16)
        -> Result<tokio::sync::oneshot::Receiver<Result<(), String>>, String>
    {
        if cols == 0 || rows == 0 { return Err("TERM_SIZE_INVALID".into()); }
        if self.stop.load(Ordering::Acquire) { return Err("TERM_CLOSED".into()); }
        let mut worker = self.resize_owner.worker.lock().unwrap_or_else(|e| e.into_inner());
        if worker.as_ref().is_some_and(|handle| handle.is_finished()) {
            if worker.take().expect("finished resize owner").join().is_err() {
                self.stop_resize();
                return Err("TERM_RESIZE_WORKER_PANICKED".into());
            }
        }
        let (reply, receiver) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.resize_owner.pending.lock().unwrap_or_else(|e| e.into_inner());
            if self.stop.load(Ordering::Acquire) { return Err("TERM_CLOSED".into()); }
            if let Some(previous) = pending.replace(ResizeRequest {
                size: PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }, reply,
            }) {
                let _ = previous.reply.send(Err("TERM_RESIZE_SUPERSEDED".into()));
            }
        }
        if worker.is_none() {
            let lease = match self.admission.upgrade() {
                Some(lease) => lease,
                None => { self.stop_resize(); return Err("TERM_CLOSED".into()); }
            };
            let control = self.clone();
            match thread::Builder::new().name("terminal-resize".into()).spawn(move || {
                // This lease survives the IPC future and covers real native I/O.
                let _permit = TerminalPermit { _lease: lease };
                control.run_resize_owner();
            }) {
                Ok(handle) => *worker = Some(handle),
                Err(_) => {
                    if let Some(pending) = self.resize_owner.pending.lock().unwrap_or_else(|e| e.into_inner()).take() {
                        let _ = pending.reply.send(Err("TERM_RESIZE_WORKER_START_FAILED".into()));
                    }
                    return Err("TERM_RESIZE_WORKER_START_FAILED".into());
                }
            }
        }
        self.resize_owner.wake.notify_one();
        Ok(receiver)
    }

    fn run_resize_owner(&self) {
        loop {
            let request = {
                let mut pending = self.resize_owner.pending.lock().unwrap_or_else(|e| e.into_inner());
                while pending.is_none() && !self.stop.load(Ordering::Acquire) {
                    pending = self.resize_owner.wake.wait_timeout(
                        pending, Duration::from_millis(TERM_CANCEL_POLL_MS),
                    ).unwrap_or_else(|e| e.into_inner()).0;
                }
                if self.stop.load(Ordering::Acquire) {
                    if let Some(request) = pending.take() {
                        let _ = request.reply.send(Err("TERM_CLOSED".into()));
                    }
                    return;
                }
                pending.take().expect("pending size or stopped")
            };
            let result = self.resize(request.size.cols, request.size.rows);
            let _ = request.reply.send(result);
        }
    }

    fn stop_resize(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(request) = self.resize_owner.pending.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = request.reply.send(Err("TERM_CLOSED".into()));
        }
        self.resize_owner.wake.notify_all();
    }

    fn reap_resize(&self) -> Result<bool, String> {
        let handle = {
            let mut worker = self.resize_owner.worker.lock().unwrap_or_else(|e| e.into_inner());
            if worker.as_ref().is_some_and(|handle| !handle.is_finished()) { return Ok(false); }
            worker.take()
        };
        if handle.is_some_and(|handle| handle.join().is_err()) {
            return Err("TERM_RESIZE_WORKER_PANICKED".into());
        }
        Ok(true)
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        if self.stop.load(Ordering::Acquire) { return Err("TERM_CLOSED".into()); }
        let guard = self.master.lock().unwrap_or_else(|e| e.into_inner());
        if self.stop.load(Ordering::Acquire) { return Err("TERM_CLOSED".into()); }
        guard.as_ref().ok_or("TERM_CLOSED")?
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| format!("resize 失败: {e}"))
    }

    #[cfg(test)]
    fn size(&self) -> Result<PtySize, String> {
        self.master.lock().unwrap_or_else(|e| e.into_inner())
            .as_ref().ok_or("TERM_CLOSED")?.get_size().map_err(|e| e.to_string())
    }
}

fn start_input_writer(
    mut writer: Box<dyn Write + Send>,
    stop: Arc<AtomicBool>,
    input_failed: Arc<AtomicBool>,
    on_failure: Option<Arc<dyn Fn() + Send + Sync>>,
) -> (TerminalInput, JoinHandle<Result<(), String>>) {
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(TERM_INPUT_QUEUE_CAPACITY);
    let failure = Arc::new(Mutex::new(None));
    let worker_stop = Arc::clone(&stop);
    let worker_failure = Arc::clone(&failure);
    let handle = thread::spawn(move || {
        let result = (|| -> io::Result<()> {
            while !worker_stop.load(Ordering::Acquire) {
                let bytes = match rx.recv_timeout(Duration::from_millis(TERM_CANCEL_POLL_MS)) {
                    Ok(bytes) => bytes,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                let mut offset = 0;
                while offset < bytes.len() && !worker_stop.load(Ordering::Acquire) {
                    // Do not use write_all: it retries Interrupted internally,
                    // potentially entering a new WriteFile after cancellation.
                    let end = (offset + TERM_READ_BUF_BYTES).min(bytes.len());
                    match writer.write(&bytes[offset..end]) {
                        Ok(0) => return Err(io::Error::new(io::ErrorKind::WriteZero, "PTY input closed")),
                        Ok(n) => offset += n,
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            wait_while_running(&worker_stop, Duration::from_millis(TERM_CANCEL_POLL_MS));
                        }
                        Err(error) => return Err(error),
                    }
                }
                if !worker_stop.load(Ordering::Acquire) {
                    writer.flush()?;
                }
            }
            Ok(())
        })();
        // Closing the writer happens in this worker, never while holding the
        // global terminals table. A requested cancellation is not an I/O fault.
        drop(writer);
        if worker_stop.load(Ordering::Acquire) { return Ok(()); }
        result.map_err(|error| {
            let error = format!("TERM_INPUT_WRITE_FAILED: {error}");
            *worker_failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(error.clone());
            input_failed.store(true, Ordering::Release);
            worker_stop.store(true, Ordering::Release);
            if let Some(kill) = on_failure { kill(); }
            error
        })
    });
    (TerminalInput { tx, stop, failure }, handle)
}

/// Terminal I/O is independent from the table lock. All owned resources move
/// together to the cleanup registry on explicit close or Drop.
pub struct TerminalSession {
    id: String,
    input: TerminalInput,
    control: TerminalControl,
    stop: Arc<AtomicBool>,
    resources: Option<CleanupResources>,
    cleanup: Option<Arc<CleanupCompletion>>,
}

impl TerminalSession {
    pub fn input(&self) -> TerminalInput { self.input.clone() }
    pub fn control(&self) -> TerminalControl { self.control.clone() }

    /// Register close before removing this session from the shared table.
    /// Registration performs no PTY I/O or join; callers wait after unlocking.
    pub fn request_close(&mut self) { self.begin_cleanup(); }

    #[cfg(test)]
    pub(crate) fn process_id(&self) -> Option<u32> {
        self.resources.as_ref().and_then(|r| r.child.as_ref()).and_then(|child| child.process_id())
    }

    fn begin_cleanup(&mut self) -> Arc<CleanupCompletion> {
        if let Some(completion) = &self.cleanup { return Arc::clone(completion); }
        self.control.stop_resize();
        let completion = Arc::new(CleanupCompletion::new(self.id.clone()));
        if let Some(resources) = self.resources.take() {
            cleanup_registry().register(self.id.clone(), resources, Arc::clone(&completion));
        } else {
            completion.finish(Ok(()));
        }
        self.cleanup = Some(Arc::clone(&completion));
        completion
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        // Never block a caller (or the terminals mutex) in ClosePseudoConsole.
        // The registry retains every resource even if its worker cannot start.
        if self.resources.is_some() { self.begin_cleanup(); }
    }
}

/// 创建 PTY 会话并启动输出管道。measure 使用无丢弃输出队列。
pub fn spawn_terminal(
    out: Arc<dyn TermOutput>,
    measure: bool,
) -> Result<(TermInfo, TerminalSession), String> {
    #[cfg(unix)]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    #[cfg(windows)]
    let shell = std::env::var("BROWSEROS_WINDOWS_SHELL")
        .ok().filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "powershell.exe".to_string());
    let mut cmd = CommandBuilder::new(shell);
    #[cfg(windows)]
    {
        cmd.arg("-NoLogo");
        cmd.arg("-NoProfile");
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "mvp-browser-os");
    cmd.env("CLICOLOR_FORCE", "1");
    spawn_terminal_with_command(cmd, out, measure)
}

/// The native acceptance fixtures call the same spawn/input/close owners as
/// production; they do not emulate shutdown by manually dropping a PtyPair.
pub(crate) fn spawn_terminal_with_command(
    cmd: CommandBuilder,
    out: Arc<dyn TermOutput>,
    measure: bool,
) -> Result<(TermInfo, TerminalSession), String> {
    // A hung OS close/callback still consumes its permit. Repeated opens
    // cannot create an unbounded collection of pending owners or threads.
    let permit = TerminalPermit::acquire()?;
    let stop = permit.stop();
    let id = format!("term-{}", uuid::Uuid::new_v4());
    #[cfg(windows)]
    let job = Arc::new(crate::windows_process::JobObject::create_for_spawn()
        .map_err(|e| format!("WINDOWS_TERMINAL_JOB_FAILED: {e}"))?);
    if stop.load(Ordering::Acquire) { return Err("TERM_SHUTTING_DOWN".into()); }
    if probe_enabled() {
        eprintln!("[TERM_PROBE] {id} pty.openpty cols=100 rows=24");
    }
    let pair = native_pty_system().openpty(PtySize {
        rows: 24, cols: 100, pixel_width: 0, pixel_height: 0,
    }).map_err(|e| format!("无法创建 PTY: {e}"))?;
    let control = TerminalControl {
        master: Arc::new(Mutex::new(Some(pair.master))), stop: Arc::clone(&stop),
        resize_owner: Arc::new(ResizeOwner::default()), admission: Arc::downgrade(&permit._lease),
    };
    // From this point every early return, including a spawn failure, hands
    // native close to the same retained cleanup owner. No foreground error
    // path drops a live master/ConPTY.
    let mut spawn_owner = SpawnCleanupGuard {
        id: id.clone(),
        resources: Some(CleanupResources {
            control: control.clone(), child: None, pgid: None,
            slave: Some(pair.slave),
            #[cfg(windows)]
            job: Some(Arc::clone(&job)),
            writer: None, handles: None, _permit: permit,
        }),
    };
    if stop.load(Ordering::Acquire) { return Err("TERM_SHUTTING_DOWN".into()); }
    // Obtain fallible endpoints before starting a child. A failure drops only
    // independent pipe descriptors here; the master/slave belong to the guard.
    let (reader, writer) = {
        let slot = control.master.lock().unwrap_or_else(|e| e.into_inner());
        let master = slot.as_ref().ok_or("TERM_MASTER_MISSING")?;
        #[cfg(unix)]
        make_terminal_nonblocking(&**master)?;
        let reader = master.try_clone_reader().map_err(|e| format!("reader: {e}"))?;
        let writer = master.take_writer().map_err(|e| format!("writer: {e}"))?;
        (reader, writer)
    };
    let resources = spawn_owner.resources.as_mut().ok_or("TERM_SPAWN_OWNER_MISSING")?;
    if stop.load(Ordering::Acquire) { return Err("TERM_SHUTTING_DOWN".into()); }
    #[cfg(windows)]
    let child = resources.slave.as_ref().ok_or("TERM_SLAVE_MISSING")?
        .spawn_command_in_job(cmd, job.as_raw_handle())
        .map_err(|e| format!("WINDOWS_TERMINAL_JOB_FAILED: {e}"))?;
    #[cfg(not(windows))]
    let child = resources.slave.as_ref().ok_or("TERM_SLAVE_MISSING")?
        .spawn_command(cmd).map_err(|e| format!("无法启动 shell: {e}"))?;
    let pgid = pgid_of(&child);
    resources.child = Some(child);
    resources.pgid = pgid;
    // Drop the extra HPCON owner before publishing the session.
    drop(resources.slave.take());
    // Even if shutdown won during spawn, install the Windows drain reader
    // before handing the live child/HPCON to cleanup.

    #[cfg(unix)]
    let on_dead: Option<Arc<dyn Fn() + Send + Sync>> = pgid.map(|pgid| {
        Arc::new(move || kill_group(pgid, libc::SIGKILL)) as Arc<dyn Fn() + Send + Sync>
    });
    #[cfg(windows)]
    let on_dead: Option<Arc<dyn Fn() + Send + Sync>> = {
        let job = Arc::clone(&job);
        Some(Arc::new(move || { let _ = job.terminate(); }))
    };
    #[cfg(all(not(unix), not(windows)))]
    let on_dead: Option<Arc<dyn Fn() + Send + Sync>> = None;
    let input_failed = Arc::new(AtomicBool::new(false));
    let handles = start_pipeline_with_failure(
        reader, out, id.clone(), Arc::clone(&stop),
        if measure { TERM_QUEUE_CAPACITY_MEASURE } else { TERM_QUEUE_CAPACITY },
        if measure { 0 } else { TERM_FLUSH_INTERVAL_MS },
        !measure, on_dead.clone(), Arc::new(TermProbe::default()), Arc::clone(&input_failed),
    );
    resources.handles = Some(handles);
    let (input, writer) = start_input_writer(writer, Arc::clone(&stop), input_failed, on_dead);
    resources.writer = Some(writer);
    let session = TerminalSession {
        id: id.clone(), input, control, stop,
        resources: spawn_owner.resources.take(), cleanup: None,
    };
    if session.stop.load(Ordering::Acquire) { return Err("TERM_SHUTTING_DOWN".into()); }
    session.input.enqueue(b"\n")?;
    Ok((TermInfo { id, probe: probe_enabled() }, session))
}

/// This guard exists before the first fallible endpoint or process operation.
struct SpawnCleanupGuard {
    id: String,
    resources: Option<CleanupResources>,
}
impl Drop for SpawnCleanupGuard {
    fn drop(&mut self) {
        if let Some(resources) = self.resources.take() {
            resources.control.stop.store(true, Ordering::Release);
            cleanup_registry().register(self.id.clone(), resources,
                Arc::new(CleanupCompletion::new(self.id.clone())));
        }
    }
}

#[cfg(unix)]
fn make_terminal_nonblocking(master: &dyn MasterPty) -> Result<(), String> {
    let fd = master.as_raw_fd().ok_or("TERM_NATIVE_PTY_FD_MISSING")?;
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(format!("TERM_NONBLOCKING_FAILED: {}", io::Error::last_os_error()));
    }
    Ok(())
}

/// Compatibility helper; IPC callers clone control() under the table lock
/// and call resize only after releasing that lock.
pub fn resize(session: &mut TerminalSession, cols: u16, rows: u16) -> Result<(), String> {
    session.control.resize(cols, rows)
}

/// The foreground wait is bounded. A timeout explicitly means cleanup is
/// still owned/running; retry_cleanup(id) waits for that same task.
pub fn terminate_session(session: &mut TerminalSession) -> Result<(), String> {
    session.begin_cleanup().wait(TERM_GROUP_GRACE_MS + TERM_THREAD_JOIN_GRACE_MS)
}

pub fn retry_cleanup(id: &str) -> Result<(), String> {
    wait_for_cleanup(id, TERM_GROUP_GRACE_MS + TERM_THREAD_JOIN_GRACE_MS)
}

pub(crate) fn wait_for_cleanup(id: &str, grace_ms: u64) -> Result<(), String> {
    let registry = cleanup_registry();
    registry.ensure_worker();
    let completion = {
        let state = registry.state.lock().unwrap_or_else(|e| e.into_inner());
        state.tasks.iter().find(|task| task.id == id).map(|task| Arc::clone(&task.completion))
            .or_else(|| state.completed.iter().find(|(name, _)| name == id)
                .map(|(_, completion)| Arc::clone(completion)))
    };
    completion.map_or(Ok(()), |completion| completion.wait(grace_ms))
}

/// Close the process-wide admission gate and stop every reserved/live input.
/// This never waits per session. The bridge requests_close/drains its table
/// next; in-flight spawns retain their permit and hand failure to the reaper.
pub fn request_shutdown() {
    terminal_admission().request_shutdown();
    let registry = cleanup_registry();
    registry.ensure_worker();
    registry.changed.notify_all();
}

/// Await all terminal owners against the application's single absolute deadline,
/// including tasks already removed from the IPC table and spawns still in the OS.
/// A pending/error result never drops unfinished threads or refunds a permit.
pub fn shutdown_until(deadline: Instant) -> Result<(), String> {
    shutdown_registry_until(cleanup_registry(), deadline)
}

fn shutdown_registry_until(registry: &Arc<CleanupRegistry>, deadline: Instant) -> Result<(), String> {
    let admission = &registry.admission;
    admission.request_shutdown();
    registry.changed.notify_all();
    loop {
        registry.ensure_worker();
        let active = admission.active.load(Ordering::Acquire);
        let (pending, errors) = {
            let state = registry.state.lock().unwrap_or_else(|e| e.into_inner());
            (state.tasks.len(), state.shutdown_errors.clone())
        };
        if active == 0 && pending == 0 {
            // ensure_worker joins only a finished service; the run loop exits
            // only after the admission gate is closed and every lease is gone.
            let mut worker = registry.worker.lock().unwrap_or_else(|e| e.into_inner());
            if worker.as_ref().map(|handle| handle.is_finished()).unwrap_or(false) {
                if let Some(handle) = worker.take() {
                    if handle.join().is_err() {
                        return Err("TERM_SHUTDOWN_REAPER_FAILED".into());
                    }
                }
            }
            if worker.is_none() {
                return if errors.is_empty() { Ok(()) }
                    else { Err(format!("TERM_SHUTDOWN_COMPLETED_WITH_ERRORS: {}", errors.join("; "))) };
            }
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(format!("TERM_CLEANUP_PENDING: active={active}; tasks={pending}"));
        }
        registry.changed.notify_all();
        // A bounded wait also covers an in-flight spawn that has not registered
        // cleanup yet. Never restart a per-session grace period.
        let guard = admission.leases.lock().unwrap_or_else(|e| e.into_inner());
        let _wait = admission.changed.wait_timeout(
            guard, deadline.saturating_duration_since(now)
                .min(Duration::from_millis(TERM_CANCEL_POLL_MS)),
        ).unwrap_or_else(|e| e.into_inner());
    }
}

struct CleanupCompletion {
    id: String,
    state: Mutex<CleanupStatus>,
    changed: Condvar,
}

enum CleanupStatus {
    Pending(Option<String>),
    Finished(Result<(), String>),
}

impl CleanupCompletion {
    fn new(id: String) -> Self {
        Self { id, state: Mutex::new(CleanupStatus::Pending(None)), changed: Condvar::new() }
    }

    fn pending_error(&self, error: String) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(&*state, CleanupStatus::Pending(_)) {
            *state = CleanupStatus::Pending(Some(error));
        }
    }

    fn finish(&self, result: Result<(), String>) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = CleanupStatus::Finished(result);
        self.changed.notify_all();
    }

    fn wait(&self, grace_ms: u64) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_millis(grace_ms);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            match &*state {
                CleanupStatus::Finished(result) => return result.clone(),
                CleanupStatus::Pending(error) if Instant::now() >= deadline => {
                    return Err(format!("TERM_CLEANUP_PENDING: {}{}", self.id,
                        error.as_ref().map(|e| format!("; {e}")).unwrap_or_default()));
                }
                CleanupStatus::Pending(_) => {}
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            state = self.changed.wait_timeout(state, remaining)
                .unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}

struct CleanupTask {
    id: String,
    // Retained outside the runner closure, so failed thread creation cannot
    // drop a master (and block) on the foreground thread.
    resources: Arc<Mutex<Option<CleanupResources>>>,
    runner: Option<JoinHandle<Result<(), String>>>,
    completion: Arc<CleanupCompletion>,
    uncertain: bool,
    // Kept independently from the runner's stack. A panic must not refund a
    // resource slot whose OS cleanup can no longer be established.
    _permit: TerminalPermit,
}

#[derive(Default)]
struct CleanupRegistryState {
    tasks: Vec<CleanupTask>,
    completed: VecDeque<(String, Arc<CleanupCompletion>)>,
    // At most MAX_TERMINAL_SESSIONS owners can finish after the gate closes.
    shutdown_errors: Vec<String>,
    service_failed: bool,
}

struct CleanupRegistry {
    admission: Arc<TerminalAdmission>,
    state: Mutex<CleanupRegistryState>,
    changed: Condvar,
    worker: Mutex<Option<JoinHandle<()>>>,
}

fn cleanup_registry() -> &'static Arc<CleanupRegistry> {
    static REGISTRY: OnceLock<Arc<CleanupRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Arc::new(CleanupRegistry::new(Arc::clone(terminal_admission()))))
}

impl CleanupRegistry {
    fn new(admission: Arc<TerminalAdmission>) -> Self {
        Self {
            admission, state: Mutex::new(CleanupRegistryState::default()),
            changed: Condvar::new(), worker: Mutex::new(None),
        }
    }

    fn register(self: &Arc<Self>, id: String, resources: CleanupResources, completion: Arc<CleanupCompletion>) {
        let permit = resources._permit.clone();
        self.state.lock().unwrap_or_else(|e| e.into_inner()).tasks.push(CleanupTask {
            id, resources: Arc::new(Mutex::new(Some(resources))), runner: None, completion,
            uncertain: false, _permit: permit,
        });
        self.ensure_worker();
        self.changed.notify_one();
    }

    fn ensure_worker(self: &Arc<Self>) {
        let mut worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        if worker.as_ref().map(|h| h.is_finished()).unwrap_or(false) {
            if let Some(finished) = worker.take() {
                if finished.join().is_err() {
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    state.service_failed = true;
                    state.shutdown_errors.push("TERM_SHUTDOWN_REAPER_FAILED".into());
                }
            }
        }
        if worker.is_some() { return; }
        {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.service_failed { return; }
            if self.admission.closed.load(Ordering::Acquire)
                && self.admission.active.load(Ordering::Acquire) == 0
                && state.tasks.is_empty() { return; }
        }
        let registry = Arc::clone(self);
        match thread::Builder::new().name("terminal-cleanup-reaper".into())
            .spawn(move || registry.run())
        {
            Ok(handle) => *worker = Some(handle),
            Err(error) => {
                let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                for task in &state.tasks {
                    task.completion.pending_error(format!("TERM_REAPER_START_FAILED: {error}"));
                }
                // Tasks remain registered; the next close/retry starts it again.
            }
        }
    }

    fn run(self: Arc<Self>) {
        loop {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let mut index = 0;
            while index < state.tasks.len() {
                let task = &mut state.tasks[index];
                if task.runner.is_none() && !task.uncertain {
                    let resources = Arc::clone(&task.resources);
                    let completion = Arc::clone(&task.completion);
                    match thread::Builder::new().name(format!("cleanup-{}", task.id)).spawn(move || {
                        let resources = resources.lock().unwrap_or_else(|e| e.into_inner())
                            .take().ok_or("TERM_CLEANUP_OWNER_MISSING")?;
                        resources.run(&completion)
                    }) {
                        Ok(handle) => task.runner = Some(handle),
                        Err(error) => task.completion.pending_error(format!("TERM_CLEANUP_START_FAILED: {error}")),
                    }
                }
                if task.runner.as_ref().map(|handle| handle.is_finished()).unwrap_or(false) {
                    let joined = task.runner.take().expect("finished cleanup runner").join();
                    match joined {
                        Ok(result) => {
                            let finished = state.tasks.remove(index);
                            if let Err(error) = &result {
                                eprintln!("[terminal] {} cleanup reported: {error}", finished.id);
                                if self.admission.closed.load(Ordering::Acquire) {
                                    state.shutdown_errors.push(error.clone());
                                }
                            }
                            finished.completion.finish(result);
                            state.completed.push_back((finished.id, finished.completion));
                            while state.completed.len() > TERM_RECENT_CLEANUP_RESULTS { state.completed.pop_front(); }
                        }
                        Err(_) => {
                            // The runner may have unwound through unknown
                            // native state. Keep its task and resource permit;
                            // never retry an empty owner as if cleanup succeeded.
                            task.uncertain = true;
                            task.completion.pending_error("TERM_CLEANUP_PANICKED".into());
                            eprintln!("[terminal] {} cleanup owner panicked; cleanup remains pending", task.id);
                            index += 1;
                        }
                    }
                } else {
                    index += 1;
                }
            }
            // The service's own JoinHandle stays in the static registry.
            // Only finished task handles are ever removed/joined.
            if state.tasks.is_empty() && self.admission.closed.load(Ordering::Acquire)
                && self.admission.active.load(Ordering::Acquire) == 0 {
                return;
            }
            if state.tasks.is_empty() && !self.admission.closed.load(Ordering::Acquire) {
                let _guard = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
            } else {
                let _guard = self.changed.wait_timeout(state, Duration::from_millis(TERM_CANCEL_POLL_MS))
                    .unwrap_or_else(|e| e.into_inner());
            }
        }
    }
}

struct CleanupResources {
    control: TerminalControl,
    child: Option<Box<dyn Child + Send + Sync>>,
    pgid: Option<i32>,
    slave: Option<Box<dyn portable_pty::SlavePty + Send>>,
    #[cfg(windows)]
    job: Option<Arc<crate::windows_process::JobObject>>,
    writer: Option<JoinHandle<Result<(), String>>>,
    handles: Option<TerminalHandles>,
    _permit: TerminalPermit,
}

fn record_cleanup_error(errors: &mut Vec<String>, error: String) {
    if !errors.contains(&error) { errors.push(error); }
}

impl CleanupResources {
    fn run(mut self, completion: &CleanupCompletion) -> Result<(), String> {
        self.control.stop_resize();
        let mut errors = Vec::new();
        #[cfg(windows)]
        if let Some(job) = &self.job {
            if let Err(error) = job.terminate() { record_cleanup_error(&mut errors, error); }
        }
        #[cfg(windows)]
        if let Some(writer) = &self.writer {
            if !writer.is_finished() {
                if let Err(error) = crate::windows_process::cancel_thread_io(writer) {
                    record_cleanup_error(&mut errors, error);
                }
            }
        }
        let mut child_reaped = match self.child.as_mut() {
            Some(child) => match terminate_group(child, self.pgid, TERM_GROUP_GRACE_MS) {
                Ok(()) => true,
                Err(error) => { record_cleanup_error(&mut errors, error); false }
            },
            None => true,
        };
        // Failed spawn may still own the slave's Arc<HPCON>. Release it on
        // this owner, before the sole master is taken by the close thread.
        drop(self.slave.take());

        // ClosePseudoConsole may block on older Windows. It runs separately
        // while the reader drains and this owner keeps cancelling input I/O.
        // No resize/input/global-table lock is held by this cleanup loop.
        let mut closer: Option<JoinHandle<()>> = None;
        let mut master_closed = false;
        let mut close_uncertain = false;
        let mut worker = self.handles.take().map(|handles| (Some(handles.worker), Some(handles.pump)));
        let mut last_kill = Instant::now();
        loop {
            if closer.is_none() && !master_closed && !close_uncertain {
                let control = self.control.clone();
                match thread::Builder::new().name("terminal-pty-close".into()).spawn(move || {
                    let master = control.master.lock().unwrap_or_else(|e| e.into_inner()).take();
                    // The slot lock is released before the native destructor.
                    // All surviving control clones now see None, not an HPCON.
                    drop(master);
                }) {
                    Ok(handle) => closer = Some(handle),
                    Err(error) => record_cleanup_error(&mut errors, format!("TERM_PTY_CLOSE_START_FAILED: {error}")),
                }
            }

            #[cfg(windows)]
            if let Some(writer) = &self.writer {
                if !writer.is_finished() {
                    if let Err(error) = crate::windows_process::cancel_thread_io(writer) {
                        record_cleanup_error(&mut errors, error);
                    }
                }
            }
            if self.writer.as_ref().map(|h| h.is_finished()).unwrap_or(false) {
                match self.writer.take().expect("finished input writer").join() {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        // This is an input-delivery failure already latched
                        // and surfaced by the fixed InputFailed frame. The
                        // writer was joined successfully, so do not report a
                        // completed resource close as permanently unfinished.
                        eprintln!("[terminal] input delivery failed before close: {error}");
                    }
                    Err(_) => record_cleanup_error(&mut errors, "TERM_INPUT_PANICKED".into()),
                }
            }
            if closer.as_ref().map(|h| h.is_finished()).unwrap_or(false) {
                if closer.take().expect("finished PTY closer").join().is_err() {
                    close_uncertain = true;
                    completion.pending_error("TERM_PTY_CLOSE_PANICKED".into());
                    record_cleanup_error(&mut errors, "TERM_PTY_CLOSE_PANICKED".into());
                } else {
                    master_closed = true;
                }
            }

            // This order is intentional: release the input/master/HPCON
            // before joining output workers. After native close has returned,
            // cancelling a final blocked read is safe even on Windows 24H2,
            // whose ClosePseudoConsole may return before pipe EOF.
            if master_closed && self.writer.is_none() {
                if let Some((reader, pump)) = &mut worker {
                    #[cfg(windows)]
                    if let Some(handle) = reader.as_ref() {
                        if !handle.is_finished() {
                            if let Err(error) = crate::windows_process::cancel_thread_io(handle) {
                                record_cleanup_error(&mut errors, error);
                            }
                        }
                    }
                    join_finished(reader, "TERM_WORKER_PANICKED", &mut errors);
                    join_finished(pump, "TERM_PUMP_PANICKED", &mut errors);
                }
            }

            if let Some(child) = self.child.as_mut().filter(|_| !child_reaped) {
                match child.try_wait() {
                    Ok(Some(_)) => match child.wait() {
                        Ok(_) => child_reaped = true,
                        Err(error) => record_cleanup_error(&mut errors, format!("TERM_CHILD_REAP_FAILED: {error}")),
                    },
                    Ok(None) => {}
                    Err(error) => record_cleanup_error(&mut errors, format!("TERM_CHILD_STATUS_FAILED: {error}")),
                }
            }
            #[cfg(windows)]
            let job_empty = match self.job.as_ref().map(|job| job.is_empty()) {
                Some(Ok(empty)) => empty,
                Some(Err(error)) => {
                    completion.pending_error(error.clone());
                    record_cleanup_error(&mut errors, error);
                    false
                }
                None => true,
            };
            #[cfg(not(windows))]
            let job_empty = true;
            #[cfg(not(windows))]
            let _ = completion;

            if (!child_reaped || !job_empty) && last_kill.elapsed() >= Duration::from_secs(1) {
                #[cfg(windows)]
                if let Some(job) = &self.job {
                    if let Err(error) = job.terminate() { record_cleanup_error(&mut errors, error); }
                }
                #[cfg(unix)]
                if !child_reaped {
                    if let Some(pgid) = self.pgid { kill_group(pgid, libc::SIGKILL); }
                }
                if let Some(child) = self.child.as_mut().filter(|_| !child_reaped) {
                    if let Err(error) = child.kill() {
                        record_cleanup_error(&mut errors, format!("TERM_CHILD_KILL_FAILED: {error}"));
                    }
                }
                last_kill = Instant::now();
            }
            let output_reaped = worker.as_ref()
                .map(|(reader, pump)| reader.is_none() && pump.is_none()).unwrap_or(true);
            let resize_reaped = match self.control.reap_resize() {
                Ok(done) => done,
                Err(error) => { record_cleanup_error(&mut errors, error); true }
            };
            if master_closed && resize_reaped && self.writer.is_none() && output_reaped && child_reaped && job_empty {
                return if errors.is_empty() { Ok(()) } else {
                    // Every owned resource is now reaped, but preserve the
                    // diagnostic outcome separately from pending/uncertain.
                    Err(format!("TERM_CLEANUP_COMPLETED_WITH_ERRORS: {}", errors.join("; ")))
                };
            }
            thread::sleep(Duration::from_millis(TERM_CANCEL_POLL_MS));
        }
    }
}

fn join_finished(handle: &mut Option<JoinHandle<()>>, panic_message: &str, errors: &mut Vec<String>) {
    if handle.as_ref().map(|h| h.is_finished()).unwrap_or(false) {
        if handle.take().expect("finished output thread").join().is_err() {
            record_cleanup_error(errors, panic_message.into());
        }
    }
}

/// 进程组回收（F6）：`SIGTERM` → 轮询存活 → 仍活则 `SIGKILL` → wait。
pub fn terminate_group(
    child: &mut Box<dyn Child + Send + Sync>,
    pgid: Option<i32>,
    grace_ms: u64,
) -> Result<(), String> {
    let mut kill_error = None;
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
        #[cfg(not(unix))]
        Some(_) => {
            kill_error = child.kill().err();
        }
        None => {
            kill_error = child.kill().err();
        }
    }
    // A failed kill must never lead to an unbounded wait on a live child.
    // wait() is only used after the kernel-backed try_wait reports exit.
    let deadline = Instant::now() + Duration::from_millis(grace_ms.max(2000));
    loop {
        match child.try_wait().map_err(|error| format!("TERM_CHILD_STATUS_FAILED: {error}"))? {
            Some(_) => return child.wait().map(|_| ())
                .map_err(|error| format!("TERM_CHILD_REAP_FAILED: {error}")),
            None if Instant::now() >= deadline => {
                return Err(match kill_error {
                    Some(error) => format!("TERM_CHILD_KILL_FAILED: {error}"),
                    None => "TERM_CHILD_REAP_TIMEOUT".into(),
                });
            }
            None => thread::sleep(Duration::from_millis(TERM_GROUP_POLL_INTERVAL_MS)),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Sender;

    #[cfg(target_os = "windows")]
    #[test]
    fn conpty_rejects_invalid_terminal_dimensions() {
        // Invalid u16 dimensions must not overflow signed COORD. This test
        // works without creating a pseudoconsole or requiring a console host.
        let system = native_pty_system();
        for (rows, cols) in [(0, 80), (24, 0), (u16::MAX, 80), (24, u16::MAX)] {
            let size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
            assert!(system.openpty(size).is_err(),
                "ConPTY accepted invalid size {rows}x{cols}");
        }
    }

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


    #[test]
    fn utf8_decoder_preserves_every_split_of_multibyte_output() {
        // Includes a literal replacement character: valid U+FFFD is data,
        // not evidence that an input sequence was invalid.
        let expected = "A中🙂文\u{FFFD}Z";
        let bytes = expected.as_bytes();
        for split in 0..=bytes.len() {
            let mut decoder = Utf8StreamDecoder::default();
            let first = decoder.push(&bytes[..split]);
            assert_eq!(first.invalid_sequences, 0, "split={split}");
            assert!(decoder.tail.len() <= 3);
            let second = decoder.push(&bytes[split..]);
            assert_eq!(second.invalid_sequences, 0, "split={split}");
            let eof = decoder.finish();
            assert_eq!(eof.truncated_tail_bytes, 0, "split={split}");
            assert_eq!(first.text + &second.text + &eof.text, expected, "split={split}");
        }
        let mut decoder = Utf8StreamDecoder::default();
        let mut received = String::new();
        for byte in bytes {
            let chunk = decoder.push(&[*byte]);
            assert_eq!(chunk.invalid_sequences, 0);
            assert!(decoder.tail.len() <= 3);
            received.push_str(&chunk.text);
        }
        assert_eq!(received, expected);
        assert_eq!(decoder.finish(), DecodedChunk::default());
    }

    #[test]
    fn utf8_decoder_replaces_only_invalid_bytes_and_incomplete_eof() {
        let mut decoder = Utf8StreamDecoder::default();
        let first = decoder.push(&[b'A', 0xff, 0xe4]);
        assert_eq!(first.text, "A\u{FFFD}");
        assert_eq!(first.invalid_sequences, 1);
        assert_eq!(first.truncated_tail_bytes, 0);
        let second = decoder.push(&[0xb8, 0xad, b'B', 0xf0, 0x9f]);
        assert_eq!(second.text, "中B");
        assert_eq!(second.invalid_sequences, 0);
        let eof = decoder.finish();
        assert_eq!(eof.text, "\u{FFFD}");
        assert_eq!(eof.invalid_sequences, 0);
        assert_eq!(eof.truncated_tail_bytes, 2);
        assert_eq!(decoder.finish(), DecodedChunk::default(), "EOF is idempotent");

        let mut decoder = Utf8StreamDecoder::default();
        assert_eq!(decoder.push(&[0xe4]).text, "");
        let broken = decoder.push(b"x");
        assert_eq!(broken.text, "\u{FFFD}x");
        assert_eq!(broken.invalid_sequences, 1, "a completed invalid prefix is not an EOF tail");
    }

    #[test]
    fn utf8_decoder_cancel_discards_unfinished_suffix() {
        let mut decoder = Utf8StreamDecoder::default();
        assert_eq!(decoder.push(&[b'A', 0xf0, 0x9f]).text, "A");
        assert_eq!(decoder.cancel(), 2);
        assert_eq!(decoder.finish(), DecodedChunk::default());
        assert_eq!(decoder.push(b"B").text, "B");
    }

    struct ChunkReader(VecDeque<Vec<u8>>);
    impl Read for ChunkReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let Some(chunk) = self.0.pop_front() else { return Ok(0); };
            assert!(chunk.len() <= buf.len());
            buf[..chunk.len()].copy_from_slice(&chunk);
            Ok(chunk.len())
        }
    }

    /// Encoding/pipeline regression only; native PTY coverage is a separate
    /// required production-session test, not inferred from this fixture.
    #[test]
    fn pipeline_preserves_split_utf8_across_flushes() {
        let chunks = VecDeque::from([
            vec![b'A', 0xe4], vec![0xb8], vec![0xad, 0xf0, 0x9f],
            vec![0x99, 0x82], vec![0xe6], vec![0x96, 0x87, b'Z'],
        ]);
        let (tx, rx) = mpsc::channel();
        let handles = start_pipeline(
            Box::new(ChunkReader(chunks)), Arc::new(CollectSink { tx: Mutex::new(tx) }),
            "utf8-split".into(), Arc::new(AtomicBool::new(false)),
            8, 0, false, None, Arc::new(TermProbe::default()),
        );
        let mut received = String::new();
        let mut data_frames = 0;
        loop {
            let (kind, data) = rx.recv_timeout(Duration::from_secs(3))
                .expect("pipeline must deliver output and EOF");
            if kind == "exit" { break; }
            if kind == "data" { data_frames += 1; received.push_str(&data); }
        }
        handles.worker.join().unwrap();
        handles.pump.join().unwrap();
        assert!(data_frames > 1, "fixture must exercise multiple independent flushes");
        assert_eq!(received, "A中🙂文Z");
    }

    #[test]
    fn terminal_admission_shutdown_rejects_new_and_stops_reserved_session() {
        // Local instance: this test must not close the application's global
        // gate and invalidate other native terminal tests in the same binary.
        let admission = Arc::new(TerminalAdmission::default());
        let reserved = admission.acquire().unwrap();
        let active = admission.acquire().unwrap();
        let cleanup_owner = active.clone();
        let reserved_stop = reserved.stop();
        let active_stop = active.stop();
        assert_eq!(admission.active.load(Ordering::Acquire), 2);
        admission.request_shutdown();
        assert!(reserved_stop.load(Ordering::Acquire), "spawn not yet published must stop");
        assert!(active_stop.load(Ordering::Acquire));
        assert!(matches!(admission.acquire(), Err(error) if error == "TERM_SHUTTING_DOWN"));
        drop(active);
        assert_eq!(admission.active.load(Ordering::Acquire), 2, "pending cleanup retains its slot");
        drop(cleanup_owner);
        assert_eq!(admission.active.load(Ordering::Acquire), 1);
        drop(reserved);
        assert_eq!(admission.active.load(Ordering::Acquire), 0);
        admission.request_shutdown();
        assert!(admission.closed.load(Ordering::Acquire), "shutdown is monotonic");
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

    /// Real PTY output and shutdown use the production session owner.
    #[test]
    fn pipeline_delivers_real_pty_output_and_stops() {
        #[cfg(unix)]
        let cmd = CommandBuilder::new("/bin/sh");
        #[cfg(windows)]
        let cmd = {
            let mut cmd = CommandBuilder::new(
                std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into())
            );
            cmd.args(["/D", "/Q"]);
            cmd
        };
        let (tx, rx) = mpsc::channel();
        let (info, mut session) = spawn_terminal_with_command(
            cmd, Arc::new(CollectSink { tx: Mutex::new(tx) }), false,
        ).expect("spawn native production terminal");
        #[cfg(unix)]
        session.input().enqueue(b"printf '__T_%s__\\n' OK\n").expect("queue Unix input");
        #[cfg(windows)]
        session.input().enqueue(b"echo __T_^O^K__\r\n").expect("queue ConPTY input");
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seen = String::new();
        while Instant::now() < deadline {
            while let Ok((kind, data)) = rx.try_recv() {
                if kind == "data" { seen.push_str(&data); }
            }
            if seen.contains("__T_OK__") { break; }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(seen.contains("__T_OK__"), "shell-generated output must reach sink: {seen:?}");
        let first = terminate_session(&mut session);
        if let Err(error) = first {
            assert!(error.starts_with("TERM_CLEANUP_PENDING:"), "{error}");
        }
        wait_for_cleanup(&info.id, 8_000).expect("production close must reap input/reader/pump");
        assert!(session.input().enqueue(b"late").is_err());
        assert!(session.control().resize(80, 24).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resize_changes_real_pty_size() {
        let (info, mut session) = spawn_terminal_with_command(
            CommandBuilder::new("/bin/sh"), Arc::new(NoopSink), false,
        ).expect("spawn sh");
        // This endpoint can outlive the session without retaining the master.
        let control = session.control();
        control.resize(120, 40).expect("real native resize");
        let size = control.size().expect("get_size");
        assert_eq!(size.cols, 120);
        assert_eq!(size.rows, 40);
        let first = terminate_session(&mut session);
        if let Err(error) = first {
            assert!(error.starts_with("TERM_CLEANUP_PENDING:"), "{error}");
        }
        wait_for_cleanup(&info.id, 8_000).expect("reap resized terminal");
        assert!(control.size().is_err(), "control clones must not retain a closed master");
    }

    #[derive(Default)]
    struct Gate {
        open: Mutex<bool>,
        changed: Condvar,
    }

    impl Gate {
        fn wait(&self) {
            let mut open = self.open.lock().unwrap();
            while !*open { open = self.changed.wait(open).unwrap(); }
        }
        fn release(&self) {
            *self.open.lock().unwrap() = true;
            self.changed.notify_all();
        }
    }

    struct ReleaseGate(Arc<Gate>);
    impl Drop for ReleaseGate {
        fn drop(&mut self) { self.0.release(); }
    }

    struct BlockedWriter {
        gate: Arc<Gate>,
        entered: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
    }

    impl Write for BlockedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.entered.store(true, Ordering::Release);
            self.gate.wait();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }
    impl Drop for BlockedWriter {
        fn drop(&mut self) { self.closed.store(true, Ordering::Release); }
    }

    struct BlockedReader {
        gate: Arc<Gate>,
        entered: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
    }
    impl Read for BlockedReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            self.entered.store(true, Ordering::Release);
            self.gate.wait();
            Ok(0)
        }
    }
    impl Drop for BlockedReader {
        fn drop(&mut self) { self.closed.store(true, Ordering::Release); }
    }

    fn wait_flag(flag: &AtomicBool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !flag.load(Ordering::Acquire) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(flag.load(Ordering::Acquire), "worker did not enter fixture I/O");
    }

    #[test]
    fn input_queue_is_bounded_while_writer_is_blocked() {
        let gate = Arc::new(Gate::default());
        let _release = ReleaseGate(Arc::clone(&gate));
        let entered = Arc::new(AtomicBool::new(false));
        let closed = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (input, handle) = start_input_writer(Box::new(BlockedWriter {
            gate: Arc::clone(&gate), entered: Arc::clone(&entered), closed: Arc::clone(&closed),
        }), Arc::clone(&stop), Arc::new(AtomicBool::new(false)), None);
        input.enqueue(b"blocked").unwrap();
        wait_flag(&entered);
        for _ in 0..TERM_INPUT_QUEUE_CAPACITY { input.enqueue(b"queued").unwrap(); }
        assert_eq!(input.enqueue(b"overflow").unwrap_err(), "TERM_INPUT_QUEUE_FULL");
        assert!(input.enqueue(&vec![0; TERM_INPUT_MAX_BYTES + 1]).unwrap_err()
            .starts_with("TERM_INPUT_TOO_LARGE"));
        stop.store(true, Ordering::Release);
        assert_eq!(input.enqueue(b"after close").unwrap_err(), "TERM_INPUT_CLOSED");
        gate.release();
        handle.join().unwrap().unwrap();
        assert!(closed.load(Ordering::Acquire));
    }

    #[derive(Debug, Clone)]
    struct ExitedChild;
    impl portable_pty::ChildKiller for ExitedChild {
        fn kill(&mut self) -> io::Result<()> { Ok(()) }
        fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
            Box::new(self.clone())
        }
    }
    impl Child for ExitedChild {
        fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
            Ok(Some(portable_pty::ExitStatus::with_exit_code(0)))
        }
        fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
            Ok(portable_pty::ExitStatus::with_exit_code(0))
        }
        fn process_id(&self) -> Option<u32> { None }
        #[cfg(windows)]
        fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> { None }
    }

    struct DelayedCloseMaster {
        close_gate: Arc<Gate>,
        io_gate: Arc<Gate>,
        closed: Arc<AtomicBool>,
    }
    impl MasterPty for DelayedCloseMaster {
        fn resize(&self, _: PtySize) -> Result<(), portable_pty::Error> { Ok(()) }
        fn get_size(&self) -> Result<PtySize, portable_pty::Error> { Ok(PtySize::default()) }
        fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, portable_pty::Error> {
            Ok(Box::new(io::empty()))
        }
        fn take_writer(&self) -> Result<Box<dyn Write + Send>, portable_pty::Error> {
            Ok(Box::new(io::sink()))
        }
        #[cfg(unix)]
        fn process_group_leader(&self) -> Option<libc::pid_t> { None }
        #[cfg(unix)]
        fn as_raw_fd(&self) -> Option<portable_pty::unix::RawFd> { None }
        #[cfg(unix)]
        fn tty_name(&self) -> Option<std::path::PathBuf> { None }
    }
    impl Drop for DelayedCloseMaster {
        fn drop(&mut self) {
            self.close_gate.wait();
            self.closed.store(true, Ordering::Release);
            self.io_gate.release();
        }
    }

    struct BlockedResizeMaster {
        gate: Arc<Gate>,
        entered: Arc<AtomicBool>,
        sizes: Arc<Mutex<Vec<(u16, u16)>>>,
    }
    impl MasterPty for BlockedResizeMaster {
        fn resize(&self, size: PtySize) -> Result<(), portable_pty::Error> {
            self.entered.store(true, Ordering::Release);
            self.gate.wait();
            self.sizes.lock().unwrap().push((size.cols, size.rows));
            Ok(())
        }
        fn get_size(&self) -> Result<PtySize, portable_pty::Error> { Ok(PtySize::default()) }
        fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, portable_pty::Error> { Ok(Box::new(io::empty())) }
        fn take_writer(&self) -> Result<Box<dyn Write + Send>, portable_pty::Error> { Ok(Box::new(io::sink())) }
        #[cfg(unix)]
        fn process_group_leader(&self) -> Option<libc::pid_t> { None }
        #[cfg(unix)]
        fn as_raw_fd(&self) -> Option<portable_pty::unix::RawFd> { None }
        #[cfg(unix)]
        fn tty_name(&self) -> Option<std::path::PathBuf> { None }
    }

    #[test]
    fn native_resize_keeps_only_the_latest_pending_size() {
        let admission = Arc::new(TerminalAdmission::default());
        let permit = admission.acquire().unwrap();
        let gate = Arc::new(Gate::default());
        let _release = ReleaseGate(Arc::clone(&gate));
        let entered = Arc::new(AtomicBool::new(false));
        let sizes = Arc::new(Mutex::new(Vec::new()));
        let control = TerminalControl {
            master: Arc::new(Mutex::new(Some(Box::new(BlockedResizeMaster {
                gate: Arc::clone(&gate), entered: Arc::clone(&entered), sizes: Arc::clone(&sizes),
            })))),
            stop: permit.stop(), resize_owner: Arc::new(ResizeOwner::default()),
            admission: Arc::downgrade(&permit._lease),
        };
        let first = control.enqueue_resize(80, 24).unwrap();
        wait_flag(&entered);
        let obsolete = control.enqueue_resize(90, 25).unwrap();
        let latest = control.enqueue_resize(120, 40).unwrap();
        assert_eq!(obsolete.blocking_recv().unwrap().unwrap_err(), "TERM_RESIZE_SUPERSEDED");
        gate.release();
        first.blocking_recv().unwrap().unwrap();
        latest.blocking_recv().unwrap().unwrap();
        assert_eq!(*sizes.lock().unwrap(), vec![(80, 24), (120, 40)]);
        control.stop_resize();
        let handle = control.resize_owner.worker.lock().unwrap().take().unwrap();
        handle.join().unwrap();
        assert!(control.enqueue_resize(100, 30).is_err());
    }

    #[test]
    fn blocked_resize_retains_global_native_capacity_until_real_completion() {
        let admission = Arc::new(TerminalAdmission::default());
        let permit = admission.acquire().unwrap();
        let gate = Arc::new(Gate::default());
        let _release = ReleaseGate(Arc::clone(&gate));
        let entered = Arc::new(AtomicBool::new(false));
        let control = TerminalControl {
            master: Arc::new(Mutex::new(Some(Box::new(BlockedResizeMaster {
                gate: Arc::clone(&gate), entered: Arc::clone(&entered), sizes: Arc::new(Mutex::new(Vec::new())),
            })))),
            stop: permit.stop(), resize_owner: Arc::new(ResizeOwner::default()),
            admission: Arc::downgrade(&permit._lease),
        };
        let request = control.enqueue_resize(80, 24).unwrap();
        wait_flag(&entered);
        control.stop_resize();
        drop(permit);
        assert_eq!(admission.active.load(Ordering::Acquire), 1);
        gate.release();
        request.blocking_recv().unwrap().unwrap();
        let handle = control.resize_owner.worker.lock().unwrap().take().unwrap();
        handle.join().unwrap();
        assert_eq!(admission.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn terminal_shutdown_deadline_retains_pending_owners_until_all_reaped() {
        let admission = Arc::new(TerminalAdmission::default());
        let registry = Arc::new(CleanupRegistry::new(Arc::clone(&admission)));
        let close_gate = Arc::new(Gate::default());
        let _release = ReleaseGate(Arc::clone(&close_gate));
        let mut closed = Vec::new();
        for index in 0..2 {
            let permit = admission.acquire().unwrap();
            let flag = Arc::new(AtomicBool::new(false));
            closed.push(Arc::clone(&flag));
            let control = TerminalControl {
                master: Arc::new(Mutex::new(Some(Box::new(DelayedCloseMaster {
                    close_gate: Arc::clone(&close_gate),
                    io_gate: Arc::new(Gate::default()), closed: flag,
                })))),
                stop: permit.stop(),
                resize_owner: Arc::new(ResizeOwner::default()), admission: Arc::downgrade(&permit._lease),
            };
            let id = format!("local-shutdown-{index}");
            registry.register(id.clone(), CleanupResources {
                control, child: Some(Box::new(ExitedChild)), pgid: None, slave: None,
                #[cfg(windows)]
                job: None,
                writer: None, handles: None, _permit: permit,
            }, Arc::new(CleanupCompletion::new(id)));
        }
        let deadline = Instant::now() + Duration::from_millis(40);
        let result = shutdown_registry_until(&registry, deadline);
        assert!(matches!(result, Err(error) if error.starts_with("TERM_CLEANUP_PENDING:")));
        assert_eq!(admission.active.load(Ordering::Acquire), 2);
        assert_eq!(registry.state.lock().unwrap().tasks.len(), 2);
        assert!(registry.worker.lock().unwrap().is_some(), "pending service handle stays owned");
        assert!(closed.iter().all(|flag| !flag.load(Ordering::Acquire)));
        close_gate.release();
        shutdown_registry_until(&registry, Instant::now() + Duration::from_secs(3))
            .expect("both owners must be reaped against the same deadline");
        assert!(closed.iter().all(|flag| flag.load(Ordering::Acquire)));
        assert_eq!(admission.active.load(Ordering::Acquire), 0);
        assert!(registry.state.lock().unwrap().tasks.is_empty());
        assert!(registry.worker.lock().unwrap().is_none(), "completed service is joined too");
    }

    /// A deterministic ownership/order regression, not native ConPTY proof.
    /// A blocked destructor must remain registered after a foreground timeout.
    #[test]
    fn pending_cleanup_retains_master_and_workers_until_reaped() {
        let close_gate = Arc::new(Gate::default());
        let _release_close = ReleaseGate(Arc::clone(&close_gate));
        let io_gate = Arc::new(Gate::default());
        let _release_io = ReleaseGate(Arc::clone(&io_gate));
        let master_closed = Arc::new(AtomicBool::new(false));
        let reader_closed = Arc::new(AtomicBool::new(false));
        let writer_closed = Arc::new(AtomicBool::new(false));
        let reader_entered = Arc::new(AtomicBool::new(false));
        let writer_entered = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let id = format!("cleanup-regression-{}", uuid::Uuid::new_v4());
        let control = TerminalControl {
            master: Arc::new(Mutex::new(Some(Box::new(DelayedCloseMaster {
                close_gate: Arc::clone(&close_gate), io_gate: Arc::clone(&io_gate),
                closed: Arc::clone(&master_closed),
            })))),
            stop: Arc::clone(&stop),
            resize_owner: Arc::new(ResizeOwner::default()), admission: Weak::new(),
        };
        let handles = start_pipeline(Box::new(BlockedReader {
            gate: Arc::clone(&io_gate), entered: Arc::clone(&reader_entered),
            closed: Arc::clone(&reader_closed),
        }), Arc::new(NoopSink), id.clone(), Arc::clone(&stop), 1, 1, false, None,
            Arc::new(TermProbe::default()));
        let (input, writer) = start_input_writer(Box::new(BlockedWriter {
            gate: Arc::clone(&io_gate), entered: Arc::clone(&writer_entered),
            closed: Arc::clone(&writer_closed),
        }), Arc::clone(&stop), Arc::new(AtomicBool::new(false)), None);
        input.enqueue(b"blocked input").unwrap();
        wait_flag(&reader_entered);
        wait_flag(&writer_entered);
        let mut session = TerminalSession {
            id: id.clone(), input, control: control.clone(), stop,
            resources: Some(CleanupResources {
                control: control.clone(), child: Some(Box::new(ExitedChild)), pgid: None, slave: None,
                #[cfg(windows)]
                job: None,
                writer: Some(writer), handles: Some(handles), _permit: TerminalPermit::acquire().unwrap(),
            }),
            cleanup: None,
        };
        let completion = session.begin_cleanup();
        assert!(completion.wait(40).unwrap_err().starts_with("TERM_CLEANUP_PENDING:"));
        drop(session);
        assert!(wait_for_cleanup(&id, 0).unwrap_err().starts_with("TERM_CLEANUP_PENDING:"));
        assert!(!master_closed.load(Ordering::Acquire));
        assert!(!reader_closed.load(Ordering::Acquire));
        assert!(control.resize(80, 24).is_err());
        close_gate.release();
        wait_for_cleanup(&id, 2_000).expect("same retained task must eventually reap every handle");
        assert!(master_closed.load(Ordering::Acquire));
        assert!(writer_closed.load(Ordering::Acquire));
        assert!(reader_closed.load(Ordering::Acquire));
        assert!(control.size().is_err(), "surviving control clone must hold no master");
    }

    #[test]
    fn spawn_error_guard_retains_native_close_until_completed() {
        let close_gate = Arc::new(Gate::default());
        let _release = ReleaseGate(Arc::clone(&close_gate));
        let closed = Arc::new(AtomicBool::new(false));
        let id = format!("spawn-cleanup-regression-{}", uuid::Uuid::new_v4());
        let control = TerminalControl {
            master: Arc::new(Mutex::new(Some(Box::new(DelayedCloseMaster {
                close_gate: Arc::clone(&close_gate), io_gate: Arc::new(Gate::default()),
                closed: Arc::clone(&closed),
            })))),
            stop: Arc::new(AtomicBool::new(false)),
            resize_owner: Arc::new(ResizeOwner::default()), admission: Weak::new(),
        };
        let owner = SpawnCleanupGuard {
            id: id.clone(),
            resources: Some(CleanupResources {
                control: control.clone(), child: None, pgid: None, slave: None,
                #[cfg(windows)]
                job: None,
                writer: None, handles: None, _permit: TerminalPermit::acquire().unwrap(),
            }),
        };
        let before = Instant::now();
        drop(owner);
        assert!(before.elapsed() < Duration::from_secs(1), "spawn errors must not block in native close");
        assert!(wait_for_cleanup(&id, 40).unwrap_err().starts_with("TERM_CLEANUP_PENDING:"));
        assert!(!closed.load(Ordering::Acquire));
        close_gate.release();
        wait_for_cleanup(&id, 2_000).expect("spawn-error owner must eventually reap native close");
        assert!(closed.load(Ordering::Acquire));
    }

    struct FailingInput;
    impl Write for FailingInput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "sensitive-fixture-detail"))
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }

    #[test]
    fn asynchronous_input_failure_emits_a_fixed_exit_without_another_keystroke() {
        let gate = Arc::new(Gate::default());
        let _release = ReleaseGate(Arc::clone(&gate));
        let entered = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let handles = start_pipeline_with_failure(Box::new(BlockedReader {
            gate: Arc::clone(&gate), entered: Arc::clone(&entered),
            closed: Arc::new(AtomicBool::new(false)),
        }), Arc::new(CollectSink { tx: Mutex::new(tx) }), "input-failure".into(),
            Arc::clone(&stop), 1, 1, true, None, Arc::new(TermProbe::default()), Arc::clone(&failed));
        wait_flag(&entered);
        let (input, writer) = start_input_writer(Box::new(FailingInput),
            Arc::clone(&stop), failed, None);
        input.enqueue(b"accepted before native failure").unwrap();
        let (kind, text) = rx.recv_timeout(Duration::from_secs(2))
            .expect("fixed failure frame must not wait for another input request");
        assert_eq!(kind, "exit");
        assert!(text.contains("输入传输失败"));
        assert!(!text.contains("sensitive-fixture-detail"));
        assert!(!text.contains("accepted before native failure"));
        assert!(writer.join().unwrap().is_err());
        gate.release();
        handles.worker.join().unwrap();
        handles.pump.join().unwrap();
    }

    #[test]
    fn maximum_backoff_wait_is_cancelled_promptly() {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let (entered_tx, entered_rx) = mpsc::channel();
        let waiter = thread::spawn(move || {
            entered_tx.send(()).unwrap();
            wait_while_running(&worker_stop, Duration::from_millis(TERM_RETRY_MAX_BACKOFF_MS))
        });
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let started = Instant::now();
        stop.store(true, Ordering::Release);
        assert!(!waiter.join().unwrap());
        assert!(started.elapsed() < Duration::from_secs(1), "stop must not wait a 30-second backoff");
    }

    /// T3：进程组回收 —— `sleep 300` 随 shell 一起被杀，不留孤儿（F6）。
    #[cfg(unix)]
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
        #[cfg(target_os = "linux")]
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

        let failed = TermMessage::Exit(TermExitReason::InputFailed).to_value("term-1");
        assert_eq!(failed["kind"], "exit");
        assert_eq!(failed["reason"], "input_failed");
        assert!(failed["data"].as_str().unwrap().contains("输入传输失败"));
    }
}
