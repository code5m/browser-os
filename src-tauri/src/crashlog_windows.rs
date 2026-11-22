//! Portable fallback for non-Linux panic evidence.
//! Native signal/minidump integration is a separate platform capability.
//! No file is opened until workspace startup has established a private root.
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static PRIVATE_LOG: OnceLock<Mutex<File>> = OnceLock::new();

pub fn init() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        if let Some(log) = PRIVATE_LOG.get() {
            if let Ok(mut file) = log.try_lock() {
                // Panic payloads may contain user input or URLs; retain source
                // location and process metadata, not those arbitrary values.
                let location = info.location();
                let _ = writeln!(
                    file,
                    "panic platform={} pid={} source={}:{}",
                    std::env::consts::OS,
                    std::process::id(),
                    location.map_or("unknown", |at| at.file()),
                    location.map_or(0, |at| at.line()),
                );
            }
        }
    }));
}

pub fn use_private_directory(root: &Path) -> io::Result<()> {
    if PRIVATE_LOG.get().is_some() {
        return Ok(());
    }
    let directory = root.join("diagnostics");
    crate::capabilities::workspace::safe_paths::ensure_private_directory(&directory)?;
    let path = directory.join(format!(
        "panic-{}-{}-{}.log",
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
    let file = options.open(path)?;
    let _ = PRIVATE_LOG.set(Mutex::new(file));
    Ok(())
}
