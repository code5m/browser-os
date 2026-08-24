//! 崩溃与运行日志捕获：
//! 1) stderr 全量镜像到日志文件 —— Rust eprintln、WebKitGTK / GStreamer 原生报错
//!    都走 stderr，从桌面图标启动时终端不可见，必须落盘才能排查
//! 2) Rust panic 钩子：panic 信息 + 完整堆栈写入 crash.log
//! 3) SIGSEGV / SIGABRT / SIGBUS / SIGILL 信号捕获：WebKitGTK 原生崩溃（段错误）
//!    不经过 panic 钩子，用信号处理器直写日志后恢复默认处理重新抛出
//!
//! 日志目录：~/.local/share/com.jizhijiandan.mvp/logs/
//!   session-时间戳.log  每次启动一个全量日志（保留最近 10 个）
//!   crash.log           panic / 致命信号专用（追加）

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};

static LOG_FD: AtomicI32 = AtomicI32::new(-1);

fn log_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| {
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

    // ===== stderr 镜像：pipe + 转发线程（原始终端与日志文件各一份） =====
    unsafe {
        let mut fds = [0i32; 2];
        if libc::pipe(fds.as_mut_ptr()) == 0 {
            let reader = fds[0];
            let writer = fds[1];
            let orig_err = libc::dup(2); // 原 stderr，线程回显用
            libc::dup2(writer, 2); // stderr → pipe（此后所有 stderr 输出都进管道）
            libc::close(writer);
            if let Ok(file) = OpenOptions::new().create(true).append(true).open(&session_path) {
                use std::os::unix::io::IntoRawFd;
                match file.try_clone() {
                    Ok(cloned) => {
                        LOG_FD.store(cloned.into_raw_fd(), Ordering::SeqCst);
                        std::thread::spawn(move || pump(reader, orig_err, file));
                    }
                    Err(_) => {
                        LOG_FD.store(file.into_raw_fd(), Ordering::SeqCst);
                    }
                }
            }
        }
    }

    eprintln!(
        "[crashlog] 会话日志: {} (role={} pid={})",
        session_path.display(),
        role_tag(),
        std::process::id()
    );

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

/// pipe 读端 → 原始终端 stderr + 会话日志文件
fn pump(reader: RawFd, orig_err: RawFd, mut file: fs::File) {
    let mut buf = [0u8; 8192];
    loop {
        let n = unsafe { libc::read(reader, buf.as_mut_ptr() as *mut _, buf.len()) };
        if n <= 0 {
            break;
        }
        let chunk = &buf[..n as usize];
        unsafe {
            libc::write(orig_err, chunk.as_ptr() as *const _, chunk.len());
        }
        let _ = file.write_all(chunk);
        let _ = file.flush();
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
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(dir.join("crash.log")) {
        let _ = f.write_all(text.as_bytes());
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
