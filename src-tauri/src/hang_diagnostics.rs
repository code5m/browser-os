//! Out-of-band GTK main-thread responsiveness probes.
//!
//! The worker observes whether a lightweight callback reaches GTK's event
//! loop. It never blocks the UI, never reads browser content, never sends
//! telemetry, and writes only bounded process/thread metadata. The worker
//! continues to record while GTK is blocked; crashlog handles fatal signals.
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;

const PROBE_INTERVAL: Duration = Duration::from_secs(1);
const SUSPECT_AFTER_MS: u64 = 5_000;
const DIAGNOSTIC_MAX_BYTES: u64 = 1024 * 1024;
const MAX_THREADS: usize = 64;

fn diagnostic_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    base.join("com.jizhijiandan.mvp").join("diagnostics")
}

fn append_bounded(path: &Path, event: &Value) {
    let bytes = match serde_json::to_vec(event) {
        Ok(value) if value.len() < 32_768 => value,
        _ => return,
    };
    let mut options = OpenOptions::new();
    options
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let Ok(mut file) = options.open(path) else {
        return;
    };
    let Ok(meta) = file.metadata() else { return };
    if meta.len().saturating_add(bytes.len() as u64 + 1) > DIAGNOSTIC_MAX_BYTES {
        return;
    }
    let _ = file.write_all(&bytes);
    let _ = file.write_all(b"\n");
    let _ = file.flush();
}

fn read_status(path: &str) -> Value {
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut fields = serde_json::Map::new();
    for line in text.lines() {
        for key in ["VmRSS", "VmSize", "VmSwap", "Threads", "PPid", "State"] {
            if let Some(value) = line.strip_prefix(&format!("{key}:")) {
                fields.insert(
                    key.to_owned(),
                    json!(value.trim().chars().take(64).collect::<String>()),
                );
            }
        }
    }
    Value::Object(fields)
}

fn threads_snapshot() -> Vec<Value> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir("/proc/self/task") else {
        return result;
    };
    for entry in entries.flatten().take(MAX_THREADS) {
        let tid = entry.file_name().to_string_lossy().to_string();
        if !tid.bytes().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let task = entry.path();
        let name = fs::read_to_string(task.join("comm"))
            .unwrap_or_default()
            .trim()
            .chars()
            .take(24)
            .collect::<String>();
        let wchan = fs::read_to_string(task.join("wchan"))
            .unwrap_or_default()
            .trim()
            .chars()
            .take(80)
            .collect::<String>();
        result.push(json!({"tid":tid,"comm":name,"wait_channel":wchan}));
    }
    result
}

fn children_snapshot() -> Vec<Value> {
    // /proc/self/task/*/children is a kernel-only PID list. No command-line
    // arguments, browser URLs, profile paths, cookies, or user documents.
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir("/proc/self/task") else {
        return result;
    };
    for entry in entries.flatten() {
        let Ok(contents) = fs::read_to_string(entry.path().join("children")) else {
            continue;
        };
        for token in contents.split_whitespace() {
            if result.len() >= 24 {
                return result;
            }
            if !token.bytes().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let status = read_status(&format!("/proc/{token}/status"));
            result.push(json!({"pid":token,"status":status}));
        }
    }
    result
}

fn snapshot(event: &str, stalled_ms: u64) -> Value {
    let memory_pressure = fs::read_to_string("/proc/pressure/memory")
        .unwrap_or_default()
        .lines()
        .take(2)
        .map(|line| line.chars().take(160).collect::<String>())
        .collect::<Vec<_>>();
    json!({
        "schema": "browseros-ui-hang-v1",
        "time": chrono::Utc::now().to_rfc3339(),
        "event": event,
        "pid": std::process::id(),
        "gtk_callback_age_ms": stalled_ms,
        "process": read_status("/proc/self/status"),
        "threads": threads_snapshot(),
        "children": children_snapshot(),
        "memory_pressure": memory_pressure
    })
}

/// Starts once per BrowserOS MAIN process, after the main window exists.
/// A single in-flight UI callback is maintained to prevent an unbounded
/// callback queue if the GTK thread is actually stuck.
pub fn start(app: AppHandle) {
    let dir = diagnostic_dir();
    if fs::create_dir_all(&dir).is_err() {
        eprintln!("[hang-watchdog] diagnostic directory unavailable");
        return;
    }
    // Each incident file is size-capped. Never silently remove an older
    // forensic record: it may be the only evidence of a previous hang.
    let path = dir.join(format!(
        "hang-{}-pid{}.jsonl",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        std::process::id()
    ));
    append_bounded(
        &path,
        &json!({
            "schema":"browseros-ui-hang-v1",
            "event":"watchdog_started",
            "pid":std::process::id(),
            "time":chrono::Utc::now().to_rfc3339()
        }),
    );

    let start = Instant::now();
    let last_callback = Arc::new(AtomicU64::new(1));
    let pending_callback = Arc::new(AtomicBool::new(false));
    let _ = thread::Builder::new()
        .name("browseros-ui-watchdog".into())
        .spawn(move || {
            let mut stalled = false;
            let mut last_snapshot_ms: u64 = 0;
            loop {
                thread::sleep(PROBE_INTERVAL);
                if !pending_callback.swap(true, Ordering::SeqCst) {
                    let timestamp = Arc::clone(&last_callback);
                    let pending = Arc::clone(&pending_callback);
                    let pending_error = Arc::clone(&pending_callback);
                    let started = start;
                    if app
                        .run_on_main_thread(move || {
                            timestamp.store(
                                started.elapsed().as_millis().min(u64::MAX as u128) as u64 + 1,
                                Ordering::Release,
                            );
                            pending.store(false, Ordering::Release);
                        })
                        .is_err()
                    {
                        pending_error.store(false, Ordering::Release);
                        return;
                    }
                }
                let now_ms = start.elapsed().as_millis().min(u64::MAX as u128) as u64 + 1;
                let age_ms = now_ms.saturating_sub(last_callback.load(Ordering::Acquire));
                if age_ms >= SUSPECT_AFTER_MS {
                    if !stalled || now_ms.saturating_sub(last_snapshot_ms) >= 15_000 {
                        append_bounded(&path, &snapshot("gtk_unresponsive_suspected", age_ms));
                        last_snapshot_ms = now_ms;
                    }
                    stalled = true;
                } else if stalled {
                    append_bounded(&path, &snapshot("gtk_responsive_again", age_ms));
                    stalled = false;
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_exclude_browser_content_and_are_size_bounded() {
        let snap = snapshot("unit_test", 6000);
        let encoded = serde_json::to_string(&snap).unwrap();
        assert!(!encoded.contains("cookie"));
        assert!(!encoded.contains("https://"));
        assert_eq!(snap["gtk_callback_age_ms"], 6000);
        assert!(encoded.len() < 65_536);
    }
}
