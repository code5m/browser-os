//! Non-Linux UI heartbeat. Keep sampling independent of potentially
//! blocked UI dispatch and disk I/O, as on Linux. No browser content.
use serde_json::{json, Value};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, AtomicU64, Ordering}, mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

const MAX_LOG_BYTES: u64 = 1024 * 1024;
const MAX_EVENT_BYTES: usize = 32768;

fn create_log(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).append(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// One writer owns its two exact files. Preserve the initial incident
/// segment and roll only its recent segment, never older incidents.
struct IncidentLog {
    initial: File,
    recent_path: PathBuf,
    recent: Option<File>,
}
impl IncidentLog {
    fn new(path: &Path) -> io::Result<Self> {
        Ok(Self { initial:create_log(path)?, recent_path:path.with_extension("recent.jsonl"), recent:None })
    }
    fn append(&mut self, event: &Value) -> io::Result<()> {
        let mut bytes = serde_json::to_vec(event).map_err(io::Error::other)?;
        if bytes.len() >= MAX_EVENT_BYTES { return Err(io::Error::other("diagnostic event too large")); }
        bytes.push(b'\n');
        let length = bytes.len() as u64;
        if self.recent.is_none() && self.initial.metadata()?.len().saturating_add(length) <= MAX_LOG_BYTES {
            return self.initial.write_all(&bytes);
        }
        if self.recent.is_none() {
            self.recent = Some(create_log(&self.recent_path)?);
        }
        let recent = self.recent.as_mut().expect("recent log initialized");
        if recent.metadata()?.len().saturating_add(length) > MAX_LOG_BYTES {
            recent.set_len(0)?;
        }
        recent.write_all(&bytes)
    }
}

fn snapshot(event: &str, age_ms: u64) -> Value {
    json!({
        "schema":"browseros-ui-hang-v1",
        "time":chrono::Utc::now().to_rfc3339(),
        "event":event,"age_ms":age_ms,"pid":std::process::id(),
        "platform":std::env::consts::OS,
        "tab_mutation":crate::bridge::tab_mutation_diagnostic(),
        "capture":crate::capabilities::browser::public_api::capture_diagnostic(),
    })
}

pub fn start(app: AppHandle) {
    let directory = crate::workspace::data_dir(&app).join("diagnostics");
    let path = directory.join(format!("hang-{}-{}-{}.jsonl", std::env::consts::OS,
        std::process::id(), uuid::Uuid::new_v4()));
    // Directory preparation and all disk I/O happen off the sampler.
    let (events, receive_events) = mpsc::sync_channel::<Value>(16);
    let lost = Arc::new(AtomicU64::new(0));
    let writer_lost = Arc::clone(&lost);
    if thread::Builder::new().name("browseros-ui-diagnostic-writer".into()).spawn(move || {
        if crate::capabilities::workspace::safe_paths::ensure_private_directory(&directory).is_err() { return; }
        let Ok(mut log) = IncidentLog::new(&path) else { return; };
        for mut event in receive_events {
            event["diagnostic_dropped_events"] = json!(writer_lost.load(Ordering::Relaxed));
            if log.append(&event).is_err() { writer_lost.fetch_add(1, Ordering::Relaxed); }
        }
    }).is_err() { return; }
    let record = move |event: Value| {
        if events.try_send(event).is_err() { lost.fetch_add(1, Ordering::Relaxed); }
    };

    let started = Instant::now();
    let last_ack = Arc::new(AtomicU64::new(1));
    let pending = Arc::new(AtomicBool::new(false));
    let dispatch_failed = Arc::new(AtomicBool::new(false));
    let (submit, requests) = mpsc::sync_channel::<()>(1);
    let dispatcher = app.clone();
    let timestamp = Arc::clone(&last_ack);
    let pending_callback = Arc::clone(&pending);
    let failed = Arc::clone(&dispatch_failed);
    if thread::Builder::new().name("browseros-ui-probe-submit".into()).spawn(move || {
        while requests.recv().is_ok() {
            let ack = Arc::clone(&timestamp);
            let back = Arc::clone(&pending_callback);
            if dispatcher.run_on_main_thread(move || {
                ack.store(started.elapsed().as_millis().min((u64::MAX-1) as u128) as u64 + 1, Ordering::Release);
                back.store(false, Ordering::Release);
            }).is_err() {
                failed.store(true, Ordering::Release);
                pending_callback.store(false, Ordering::Release);
                break;
            }
        }
    }).is_err() { return; }

    let _ = thread::Builder::new().name("browseros-ui-watchdog".into()).spawn(move || {
        record(snapshot("watchdog_started",0));
        let mut suspect = false;
        let mut last_report = 0u64;
        loop {
            thread::sleep(Duration::from_secs(2));
            if app.try_state::<crate::ExitLifecycleState>()
                .is_some_and(|state| state.started.load(Ordering::Acquire)) {
                record(snapshot("watchdog_stopped",0));
                return;
            }
            let now = started.elapsed().as_millis().min((u64::MAX-1) as u128) as u64 + 1;
            let age = now.saturating_sub(last_ack.load(Ordering::Acquire));
            if dispatch_failed.load(Ordering::Acquire) {
                record(snapshot("ui_probe_dispatch_failed",age));
                return;
            }
            if !pending.swap(true, Ordering::AcqRel) && submit.try_send(()).is_err() {
                record(snapshot("ui_probe_dispatch_failed",age));
                return;
            }
            if age >= 8000 && (!suspect || now.saturating_sub(last_report) >= 15000) {
                record(snapshot("ui_unresponsive_suspected",age));
                suspect = true;
                last_report = now;
            } else if age < 8000 && suspect {
                record(snapshot("ui_responsive_again",age));
                suspect = false;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incident_log_retains_initial_and_bounded_recent_segment() {
        let dir = std::env::temp_dir().join(format!("browseros-diag-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("incident.jsonl");
        let mut log = IncidentLog::new(&path).unwrap();
        let sample = json!({"event":"fixture","payload":"x".repeat(30_000)});
        for _ in 0..100 { log.append(&sample).unwrap(); }
        log.append(&json!({"event":"ui_responsive_again"})).unwrap();
        let recent = path.with_extension("recent.jsonl");
        drop(log);
        assert!(std::fs::metadata(&path).unwrap().len() <= MAX_LOG_BYTES);
        assert!(std::fs::metadata(&recent).unwrap().len() <= MAX_LOG_BYTES);
        assert!(std::fs::read_to_string(&recent).unwrap().contains("ui_responsive_again"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
