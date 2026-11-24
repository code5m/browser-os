//! 崩溃与运行日志捕获：
//! 1) stderr 镜像到会话日志 —— Rust eprintln、WebKitGTK / GStreamer 原生报错
//!    都走 stderr；磁盘和终端分别使用有界队列，避免慢日志接收端堵塞 GTK
//! 2) Rust panic 钩子：panic 信息 + 完整堆栈写入 crash.log
//! 3) SIGSEGV / SIGABRT / SIGBUS / SIGILL 信号捕获：WebKitGTK 原生崩溃（段错误）
//!    不经过 panic 钩子，用信号处理器直写日志后恢复默认处理重新抛出
//!
//! 日志目录：~/.local/share/com.jizhijiandan.mvp/logs/
//!   session-时间戳.log  每次启动一个会话日志（保留最近 10 个，过载丢失有计数）
//!   crash.log           panic / 致命信号专用（追加）

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

static LOG_FD: AtomicI32 = AtomicI32::new(-1);
static SESSION_DROPPED_BYTES: AtomicU64 = AtomicU64::new(0);
static ECHO_DROPPED_BYTES: AtomicU64 = AtomicU64::new(0);

const LOG_CHUNK_BYTES: usize = 8192;
const SESSION_QUEUE_CHUNKS: usize = 128;
const ECHO_QUEUE_CHUNKS: usize = 32;
const SESSION_MAX_BYTES: u64 = 8 * 1024 * 1024;
type LogChunk = Arc<[u8]>;

/// Counters only; never return any log text to hang diagnostics.
pub(crate) fn log_loss_counters() -> (u64, u64) {
    (
        SESSION_DROPPED_BYTES.load(Ordering::Relaxed),
        ECHO_DROPPED_BYTES.load(Ordering::Relaxed),
    )
}

fn log_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        });
    base.join("com.jizhijiandan.mvp").join("logs")
}

/// 进程角色：主进程 "main"，宫格子进程 "grid{N}"（读 GRID_CHILD_INDEX 环境变量）。
/// 用于 session 日志命名（多子进程同秒启动撞名修复）与日志行内标记。
fn role_tag() -> String {
    match std::env::var("GRID_CHILD_INDEX") {
        Ok(i) => format!("grid{i}"),
        Err(_) => "main".to_string(),
    }
}

/// 在 main() 第一行调用。
pub fn init() {
    let dir = log_dir();
    let _ = fs::create_dir_all(&dir);
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
    // 文件名带角色 + pid：多宫格子进程同秒启动时不再共用同一 session 文件。
    let session_path = dir.join(format!(
        "session-{ts}-{}-pid{}.log",
        role_tag(),
        std::process::id()
    ));

    // Publish a stderr redirection ONLY after the file, pipe and reader
    // are ready. A failed open/clone/thread spawn must leave stderr intact.
    match start_stderr_mirror(&session_path) {
        Ok(()) => eprintln!(
            "[crashlog] 会话日志: {} (role={} pid={})",
            session_path.display(),
            role_tag(),
            std::process::id()
        ),
        Err(error) => eprintln!("[crashlog] stderr mirror unavailable: {error}"),
    }

    // ===== Rust panic 钩子 =====
    let crash_dir = dir.clone();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let text = format!(
            "\n===== PANIC {} =====\n{info}\n----- backtrace -----\n{bt}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        append_crash(&crash_dir, &text);
        eprintln!("{text}");
    }));

    // ===== 致命信号捕获（段错误不经过 panic 钩子） =====
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = crash_signal_handler as *const () as usize;
        sa.sa_flags = 0;
        libc::sigemptyset(&mut sa.sa_mask);
        for sig in [libc::SIGSEGV, libc::SIGABRT, libc::SIGBUS, libc::SIGILL] {
            libc::sigaction(sig, &sa, std::ptr::null_mut());
        }
    }

    prune_old_logs(&dir, 10);
}

/// Prepare all resources before changing fd 2. Neither disk writes nor
/// terminal/journal writes run on the pipe reader. A stalled sink can only
/// fill its own bounded queue; excess bytes are counted instead of blocking
/// GTK behind the stderr pipe. Fatal records keep their independent fd.
fn start_stderr_mirror(path: &Path) -> std::io::Result<()> {
    let file = OpenOptions::new()
        .create_new(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let fatal_log = file.try_clone()?;
    let original_fd = unsafe { libc::fcntl(2, libc::F_DUPFD_CLOEXEC, 3) };
    if original_fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut original = unsafe { fs::File::from_raw_fd(original_fd) };
    let mut fds = [-1; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let reader = unsafe { fs::File::from_raw_fd(fds[0]) };
    let writer = unsafe { fs::File::from_raw_fd(fds[1]) };

    // Bound total queued payload to about 1.25 MiB plus the current
    // chunks. Arc shares payloads between the two independent consumers.
    let (session_tx, session_rx) = mpsc::sync_channel::<LogChunk>(SESSION_QUEUE_CHUNKS);
    let (echo_tx, echo_rx) = mpsc::sync_channel::<LogChunk>(ECHO_QUEUE_CHUNKS);
    std::thread::Builder::new()
        .name("browseros-stderr-session".into())
        .spawn(move || write_session_log(session_rx, file))?;
    std::thread::Builder::new()
        .name("browseros-stderr-echo".into())
        .spawn(move || {
            for chunk in echo_rx {
                if original.write_all(&chunk).is_err() {
                    ECHO_DROPPED_BYTES.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                }
            }
        })?;
    std::thread::Builder::new()
        .name("browseros-stderr-pump".into())
        .spawn(move || pump(reader, session_tx, echo_tx))?;

    // The reader now owns its resources and waits for input. If dup2 fails,
    // writer drops, the reader sees EOF, and both worker threads can exit.
    if unsafe { libc::dup2(writer.as_raw_fd(), 2) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    LOG_FD.store(fatal_log.into_raw_fd(), Ordering::SeqCst);
    Ok(())
}

fn write_session_log(chunks: Receiver<LogChunk>, mut file: fs::File) {
    let mut last_report = Instant::now();
    let mut reported = (0, 0);
    for chunk in chunks {
        if write_session_chunk(&mut file, &chunk, SESSION_MAX_BYTES).is_err() {
            SESSION_DROPPED_BYTES.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
        let lost = log_loss_counters();
        if lost != reported && last_report.elapsed() >= Duration::from_secs(5) {
            // Rate limit this marker; do not amplify a log flood.
            last_report = Instant::now();
            let marker = format!(
                "\n[crashlog] dropped_session_bytes={} dropped_echo_bytes={}\n",
                lost.0, lost.1
            );
            if write_session_chunk(&mut file, marker.as_bytes(), SESSION_MAX_BYTES).is_ok() {
                reported = lost;
            }
        }
    }
}

/// Reuse the same inode so the fatal-signal descriptor remains valid. Once
/// full, retain the newest bounded segment with an explicit rollover marker.
/// This runs exclusively on the disk worker, never on GTK or the pipe pump.
fn write_session_chunk(file: &mut fs::File, bytes: &[u8], limit: u64) -> std::io::Result<()> {
    const MARKER: &[u8] = b"[crashlog] session byte limit reached; older segment discarded\n";
    if bytes.len() as u64 + MARKER.len() as u64 > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "LOG_CHUNK_TOO_LARGE",
        ));
    }
    if file.metadata()?.len().saturating_add(bytes.len() as u64) > limit {
        file.set_len(0)?;
        file.write_all(MARKER)?;
    }
    file.write_all(bytes)
}

#[cfg(test)]
mod bounded_log_tests {
    use super::*;

    #[test]
    fn log_flood_keeps_latest_segment_and_same_inode() {
        let path = std::env::temp_dir().join(format!("browseros-log-{}", uuid::Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)
            .unwrap();
        let fatal = file.try_clone().unwrap();
        for _ in 0..20 {
            write_session_chunk(&mut file, &[b'x'; 80], 256).unwrap();
            assert!(file.metadata().unwrap().len() <= 256);
        }
        write_session_chunk(&mut file, b"latest-event\n", 256).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("older segment discarded"));
        assert!(text.ends_with("latest-event\n"));
        assert_eq!(
            fatal.metadata().unwrap().len(),
            file.metadata().unwrap().len()
        );
        drop(file);
        drop(fatal);
        fs::remove_file(path).unwrap();
    }
}

fn pump(mut reader: fs::File, session: SyncSender<LogChunk>, echo: SyncSender<LogChunk>) {
    let mut buf = [0u8; LOG_CHUNK_BYTES];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        let chunk: LogChunk = Arc::from(&buf[..n]);
        // Never perform potentially blocking sink I/O or a blocking send
        // on the pipe reader. Each lost chunk is counted independently.
        if session.try_send(Arc::clone(&chunk)).is_err() {
            SESSION_DROPPED_BYTES.fetch_add(n as u64, Ordering::Relaxed);
        }
        if echo.try_send(chunk).is_err() {
            ECHO_DROPPED_BYTES.fetch_add(n as u64, Ordering::Relaxed);
        }
    }
}

/// 致命信号处理器：零分配（栈上固定缓冲），写完立即 _exit。
/// 教训：曾经用 format! 分配 + raise 重新抛出，结果处理器被反复重入
/// 死循环刷屏（2 分钟写 2.6GB 日志、进程挂起无响应）——
/// 信号处理器里不做任何堆分配，终止必须走 _exit 而不是 raise。
extern "C" fn crash_signal_handler(sig: libc::c_int) {
    let fd = LOG_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        let mut buf = [0u8; 160];
        let mut len = 0usize;
        let head = b"\n===== CRASH SIGNAL ";
        let tail = b" (fatal signal, WebKitGTK native crash) =====\n";
        buf[len..len + head.len()].copy_from_slice(head);
        len += head.len();
        if sig >= 10 {
            buf[len] = b'0' + (sig / 10) as u8;
            len += 1;
        }
        buf[len] = b'0' + (sig % 10) as u8;
        len += 1;
        buf[len..len + tail.len()].copy_from_slice(tail);
        len += tail.len();
        unsafe {
            libc::write(fd, buf.as_ptr() as *const _, len);
        }
    }
    // 立即终止：128+sig 是 shell 约定的信号退出码
    unsafe {
        libc::_exit(128 + sig);
    }
}

/// panic 文本同时写入 crash.log 与会话日志
fn append_crash(dir: &Path, text: &str) {
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(dir.join("crash.log"))
    {
        if f.metadata().is_ok_and(|metadata| metadata.is_file()) {
            let _ = f.write_all(text.as_bytes());
        }
    }
    let fd = LOG_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        unsafe {
            libc::write(fd, text.as_ptr() as *const _, text.len());
        }
    }
}

/// 只保留最近 keep 个 session-*.log，防止日志目录无限膨胀
fn prune_old_logs(dir: &Path, keep: usize) {
    let mut sessions: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("session-") && n.ends_with(".log"))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    sessions.sort();
    if sessions.len() > keep {
        for p in &sessions[..sessions.len() - keep] {
            let _ = fs::remove_file(p);
        }
    }
}
