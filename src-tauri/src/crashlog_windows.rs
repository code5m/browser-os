//! Windows-safe fallback for local panic evidence. Full ETW/minidump capture
//! needs separate Windows OS validation; never pretend Unix signal hooks exist.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
pub fn init() {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.jizhijiandan.mvp")
        .join("logs");
    if fs::create_dir_all(&dir).is_err() { return; }
    let log = dir.join("crash.log");
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log) {
            let _ = writeln!(file, "panic: {info}");
        }
    }));
}
