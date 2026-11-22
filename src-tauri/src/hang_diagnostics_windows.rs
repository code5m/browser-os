//! Portable non-Linux UI heartbeat, stored under the prepared private root.
//! This records scheduling metadata; it does not certify native GUI behavior.
use serde_json::json;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

const MAX_LOG_BYTES: u64 = 1024 * 1024;

fn append(file: &mut File, event: &str, age_ms: u64) {
    if !file
        .metadata()
        .is_ok_and(|metadata| metadata.len() < MAX_LOG_BYTES)
    {
        return;
    }
    let record = json!({
        "event": event,
        "age_ms": age_ms,
        "pid": std::process::id(),
        "platform": std::env::consts::OS,
    });
    let _ = writeln!(file, "{record}");
}

pub fn start(app: AppHandle) {
    let directory = crate::workspace::data_dir(&app).join("diagnostics");
    if crate::capabilities::workspace::safe_paths::ensure_private_directory(&directory).is_err() {
        return;
    }
    let path = directory.join(format!(
        "hang-{}-{}-{}.jsonl",
        std::env::consts::OS,
        std::process::id(),
        uuid::Uuid::new_v4(),
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let Ok(mut file) = options.open(path) else {
        return;
    };
    append(&mut file, "watchdog_started", 0);
    let started = Instant::now();
    let last_ack = Arc::new(AtomicU64::new(1));
    let pending = Arc::new(AtomicBool::new(false));
    let _ = thread::Builder::new()
        .name("browseros-ui-watchdog".into())
        .spawn(move || {
            let mut suspect = false;
            let mut last_report = 0u64;
            loop {
                thread::sleep(Duration::from_secs(2));
                if app
                    .try_state::<crate::ExitLifecycleState>()
                    .is_some_and(|state| state.started.load(Ordering::Acquire))
                {
                    append(&mut file, "watchdog_stopped", 0);
                    return;
                }
                if !pending.swap(true, Ordering::AcqRel) {
                    let back = Arc::clone(&pending);
                    let ack = Arc::clone(&last_ack);
                    let start = started;
                    if app
                        .run_on_main_thread(move || {
                            ack.store(
                                start.elapsed().as_millis().min(u64::MAX as u128) as u64 + 1,
                                Ordering::Release,
                            );
                            back.store(false, Ordering::Release);
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                let now = started.elapsed().as_millis().min(u64::MAX as u128) as u64 + 1;
                let age = now.saturating_sub(last_ack.load(Ordering::Acquire));
                if age >= 8000 && (!suspect || now.saturating_sub(last_report) >= 15000) {
                    append(&mut file, "ui_unresponsive_suspected", age);
                    suspect = true;
                    last_report = now;
                } else if age < 8000 && suspect {
                    append(&mut file, "ui_responsive_again", age);
                    suspect = false;
                }
            }
        });
}
