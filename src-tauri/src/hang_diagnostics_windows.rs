//! Windows UI event-loop heartbeat. Only metadata is logged (never web
//! URLs, DOM, script content, cookies or user account identifiers).
//! This does not replace native Windows minidump / installed GUI validation.
use serde_json::json;
use std::{
    fs::{self, OpenOptions}, io::Write, path::Path, path::PathBuf,
    sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}},
    thread, time::{Duration, Instant},
};
use tauri::AppHandle;
const MAX_LOG_BYTES: u64 = 1024 * 1024;
fn append(path: &Path, event: &str, age_ms: u64) {
    if fs::metadata(path).ok().is_some_and(|m| m.len() >= MAX_LOG_BYTES) { return; }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let record = json!({"event":event,"age_ms":age_ms,"pid":std::process::id()});
        let _ = writeln!(file, "{record}");
    }
}
pub fn start(app: AppHandle) {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.jizhijiandan.mvp").join("hang-diagnostics");
    if fs::create_dir_all(&dir).is_err() { return; }
    let path = dir.join(format!("hang-windows-pid{}.jsonl", std::process::id()));
    append(&path,"watchdog_started",0);
    let started = Instant::now();
    let last_ack = Arc::new(AtomicU64::new(1));
    let pending = Arc::new(AtomicBool::new(false));
    let _ = thread::Builder::new().name("browseros-windows-ui-watchdog".into()).spawn(move || {
        let mut suspect = false;
        let mut last_report = 0u64;
        loop {
            thread::sleep(Duration::from_secs(2));
            if !pending.swap(true,Ordering::AcqRel) {
                let back = Arc::clone(&pending);
                let ack = Arc::clone(&last_ack);
                let start = started;
                if app.run_on_main_thread(move || {
                    ack.store(start.elapsed().as_millis().min(u64::MAX as u128) as u64 + 1,Ordering::Release);
                    back.store(false,Ordering::Release);
                }).is_err() { return; }
            }
            let now = started.elapsed().as_millis().min(u64::MAX as u128) as u64 + 1;
            let age = now.saturating_sub(last_ack.load(Ordering::Acquire));
            if age >= 8000 && (!suspect || now.saturating_sub(last_report) >= 15000) {
                append(&path,"windows_ui_unresponsive_suspected",age);
                suspect=true; last_report=now;
            } else if age < 8000 && suspect {
                append(&path,"windows_ui_responsive_again",age);
                suspect=false;
            }
        }
    });
}
