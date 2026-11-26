//! Out-of-band GTK main-thread responsiveness probes.
//!
//! The worker observes whether a lightweight callback reaches GTK's event
//! loop. It never blocks the UI, never reads browser content, never sends
//! telemetry, and writes only bounded process/thread metadata. The worker
//! continues to record while GTK is blocked; crashlog handles fatal signals.
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

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
    if !meta.is_file() {
        return;
    }
    let recent = path.with_extension("recent.jsonl");
    if fs::symlink_metadata(&recent).is_ok()
        || meta.len().saturating_add(bytes.len() as u64 + 1) > DIAGNOSTIC_MAX_BYTES
    {
        // Preserve the initial incident segment and roll only this run's
        // recent segment. Earlier incident files are never removed here.
        let Ok(mut tail) = options.open(recent) else {
            return;
        };
        let Ok(meta) = tail.metadata() else { return };
        if !meta.is_file() {
            return;
        }
        if meta.len().saturating_add(bytes.len() as u64 + 1) > DIAGNOSTIC_MAX_BYTES
            && tail.set_len(0).is_err()
        {
            return;
        }
        let _ = tail.write_all(&bytes);
        let _ = tail.write_all(b"\n");
        return;
    }
    let _ = file.write_all(&bytes);
    let _ = file.write_all(b"\n");
    let _ = file.flush();
}

// Kernel metadata is useful evidence, but each observation is bounded.
// Truncated reads are explicitly unavailable rather than partial PID lists.
fn read_proc_bounded(path: impl AsRef<Path>, limit: usize) -> Option<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > limit {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn read_status(path: &str) -> Value {
    let text = read_proc_bounded(path, 8192).unwrap_or_default();
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
        let name = read_proc_bounded(task.join("comm"), 128)
            .unwrap_or_default()
            .trim()
            .chars()
            .take(24)
            .collect::<String>();
        let wchan = read_proc_bounded(task.join("wchan"), 256)
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
    let mut seen = std::collections::BTreeSet::new();
    for entry in entries.flatten().take(MAX_THREADS) {
        let Some(contents) = read_proc_bounded(entry.path().join("children"), 4096) else {
            continue;
        };
        for token in contents.split_whitespace() {
            if result.len() >= 24 {
                return result;
            }
            if token.parse::<u32>().ok().filter(|pid| *pid > 0).is_none()
                || !seen.insert(token.to_owned())
            {
                continue;
            }
            let status = read_status(&format!("/proc/{token}/status"));
            result.push(json!({"pid":token,"status":status}));
        }
    }
    result
}

fn read_pressure(path: &str) -> Vec<String> {
    read_proc_bounded(path, 1024)
        .unwrap_or_default()
        .lines()
        .take(2)
        .map(|line| line.chars().take(160).collect::<String>())
        .collect()
}

fn swap_counters() -> Value {
    let text = read_proc_bounded("/proc/vmstat", 32768).unwrap_or_default();
    let mut counters = serde_json::Map::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        if let (Some(key @ ("pswpin" | "pswpout" | "pgmajfault")), Some(raw)) =
            (fields.next(), fields.next())
        {
            if let Ok(value) = raw.parse::<u64>() {
                counters.insert(key.to_string(), json!(value));
            }
        }
    }
    Value::Object(counters)
}

fn snapshot(event: &str, stalled_ms: u64) -> Value {
    // The process leader is GTK's main thread. Always record it explicitly
    // even when the bounded thread inventory contains more than MAX_THREADS.
    let pid = std::process::id();
    let main_thread = format!("/proc/self/task/{pid}");
    let main_wait = read_proc_bounded(format!("{main_thread}/wchan"), 256)
        .unwrap_or_default()
        .trim()
        .chars()
        .take(80)
        .collect::<String>();
    let (session_dropped, echo_dropped) = crate::crashlog::log_loss_counters();
    json!({
        "schema": "browseros-ui-hang-v1",
        "time": chrono::Utc::now().to_rfc3339(),
        "event": event,
        "pid": pid,
        "gtk_callback_age_ms": stalled_ms,
        "process": read_status("/proc/self/status"),
        "main_thread": {
            "tid": pid,
            "wait_channel": main_wait,
            "status": read_status(&format!("{main_thread}/status"))
        },
        "inventory_limits": {"threads_scanned":MAX_THREADS,"children_returned":24},
        "threads": threads_snapshot(),
        "tab_mutation": crate::bridge::tab_mutation_diagnostic(),
        "capture": crate::capabilities::browser::public_api::capture_diagnostic(),
        "children": children_snapshot(),
        "memory_pressure": read_pressure("/proc/pressure/memory"),
        "io_pressure": read_pressure("/proc/pressure/io"),
        "cpu_pressure": read_pressure("/proc/pressure/cpu"),
        "swap_counters": swap_counters(),
        "stderr_session_dropped_bytes": session_dropped,
        "stderr_echo_dropped_bytes": echo_dropped,
        "layout_sweep": crate::bridge::layout_enforcer_status()
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
        "hang-{}-pid{}-{}.jsonl",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        std::process::id(),
        uuid::Uuid::new_v4()
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

    // A blocked disk may lose bounded diagnostics, but cannot stop heartbeat
    // sampling or allow an unbounded queue of snapshots to accumulate.
    let (event_tx, event_rx) = std::sync::mpsc::sync_channel::<Value>(16);
    let dropped_events = Arc::new(AtomicU64::new(0));
    let writer_dropped = Arc::clone(&dropped_events);
    if thread::Builder::new()
        .name("browseros-ui-diagnostic-writer".into())
        .spawn(move || {
            for mut event in event_rx {
                event["diagnostic_dropped_events"] = json!(writer_dropped.load(Ordering::Relaxed));
                append_bounded(&path, &event);
            }
        })
        .is_err()
    {
        return;
    }
    let record = move |event: Value| {
        if event_tx.try_send(event).is_err() {
            dropped_events.fetch_add(1, Ordering::Relaxed);
        }
    };

    let start = Instant::now();
    let last_callback = Arc::new(AtomicU64::new(1));
    let pending_callback = Arc::new(AtomicBool::new(false));
    let dispatch_failed = Arc::new(AtomicBool::new(false));
    let shutdown_requested = Arc::clone(&app.state::<crate::AppState>().shutdown_requested);

    // Sending a main-thread callback is normally quick, but a dispatcher
    // can itself be stalled. Never make the sampler depend on that call
    // returning. One pending request and a bounded channel are sufficient.
    let (probe_tx, probe_rx) = std::sync::mpsc::sync_channel::<()>(1);
    let callback_timestamp = Arc::clone(&last_callback);
    let callback_pending = Arc::clone(&pending_callback);
    let callback_failed = Arc::clone(&dispatch_failed);
    if thread::Builder::new()
        .name("browseros-ui-probe-submit".into())
        .spawn(move || {
            while probe_rx.recv().is_ok() {
                let timestamp = Arc::clone(&callback_timestamp);
                let pending = Arc::clone(&callback_pending);
                let started = start;
                if app
                    .run_on_main_thread(move || {
                        timestamp.store(
                            started.elapsed().as_millis().min((u64::MAX - 1) as u128) as u64 + 1,
                            Ordering::Release,
                        );
                        pending.store(false, Ordering::Release);
                    })
                    .is_err()
                {
                    callback_failed.store(true, Ordering::Release);
                    callback_pending.store(false, Ordering::Release);
                    break;
                }
            }
        })
        .is_err()
    {
        eprintln!("[hang-watchdog] probe submit worker unavailable");
        return;
    }

    let _ = thread::Builder::new()
        .name("browseros-ui-watchdog".into())
        .spawn(move || {
            let mut stalled = false;
            let mut last_snapshot_ms: u64 = 0;
            loop {
                thread::sleep(PROBE_INTERVAL);
                if shutdown_requested.load(Ordering::Acquire) {
                    break;
                }
                if dispatch_failed.load(Ordering::Acquire) {
                    let age = (start.elapsed().as_millis().min((u64::MAX - 1) as u128) as u64 + 1)
                        .saturating_sub(last_callback.load(Ordering::Acquire));
                    record(snapshot("gtk_probe_dispatch_failed", age));
                    break;
                }
                if !pending_callback.swap(true, Ordering::SeqCst) {
                    if probe_tx.try_send(()).is_err() {
                        pending_callback.store(false, Ordering::Release);
                        let age = (start.elapsed().as_millis().min((u64::MAX - 1) as u128) as u64
                            + 1)
                        .saturating_sub(last_callback.load(Ordering::Acquire));
                        record(snapshot("gtk_probe_dispatch_failed", age));
                        break;
                    }
                }
                let now_ms = start.elapsed().as_millis().min((u64::MAX - 1) as u128) as u64 + 1;
                let age_ms = now_ms.saturating_sub(last_callback.load(Ordering::Acquire));
                if age_ms >= SUSPECT_AFTER_MS {
                    if !stalled || now_ms.saturating_sub(last_snapshot_ms) >= 15_000 {
                        record(snapshot("gtk_unresponsive_suspected", age_ms));
                        last_snapshot_ms = now_ms;
                    }
                    stalled = true;
                } else if stalled {
                    record(snapshot("gtk_responsive_again", age_ms));
                    stalled = false;
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proc_metadata_reader_rejects_truncated_records() {
        let path =
            std::env::temp_dir().join(format!("browseros-diag-read-{}", uuid::Uuid::new_v4()));
        fs::write(&path, b"123 456").unwrap();
        assert_eq!(read_proc_bounded(&path, 7).as_deref(), Some("123 456"));
        assert!(read_proc_bounded(&path, 6).is_none());
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn full_diagnostic_keeps_latest_recovery_in_bounded_recent_segment() {
        let dir = std::env::temp_dir().join(format!("browseros-diag-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("hang.jsonl");
        let event = json!({"event":"fixture", "payload":"x".repeat(30_000)});
        for _ in 0..100 {
            append_bounded(&path, &event);
        }
        append_bounded(&path, &json!({"event":"gtk_responsive_again"}));
        let recent = path.with_extension("recent.jsonl");
        assert!(fs::metadata(&path).unwrap().len() <= DIAGNOSTIC_MAX_BYTES);
        assert!(fs::metadata(&recent).unwrap().len() <= DIAGNOSTIC_MAX_BYTES);
        assert!(fs::read_to_string(&recent)
            .unwrap()
            .contains("gtk_responsive_again"));
        fs::remove_dir_all(dir).unwrap();
    }
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
