//! Explicit user-started BFC service owner. Never attach to a preexisting PID or
//! kill by port. Windows uses the existing suspended-spawn Job Object owner so
//! PowerShell and its Node child exit together even on an abnormal app exit.
use std::{
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct ManagedBfcService {
    child: Mutex<Option<OwnedBfcChild>>,
}

struct OwnedBfcChild {
    child: Child,
    #[cfg(windows)]
    job: crate::windows_process::JobObject,
}

impl Drop for OwnedBfcChild {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            // A Windows Job with KILL_ON_JOB_CLOSE also covers Node descendants.
            let _ = self.job.terminate();
        }
        #[cfg(not(windows))]
        {
            let _ = self.child.kill();
        }
        let _ = self.child.try_wait();
    }
}

impl ManagedBfcService {
    pub fn is_owned(&self) -> bool {
        self.child
            .lock()
            .map(|state| state.is_some())
            .unwrap_or(false)
    }

    /// No startup from app boot. Called only by the main-window BFC action.
    pub fn start(&self, app: &AppHandle) -> Result<bool, String> {
        if app
            .state::<crate::AppState>()
            .shutdown_requested
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("BFC_SERVICE_SHUTTING_DOWN".into());
        }
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "BFC_SERVICE_OWNER_POISONED")?;
        if let Some(existing) = guard.as_mut() {
            #[cfg(windows)]
            if !existing
                .job
                .is_empty()
                .map_err(|_| "BFC_SERVICE_JOB_UNAVAILABLE")?
            {
                return Ok(false);
            }
            #[cfg(not(windows))]
            if existing
                .child
                .try_wait()
                .map_err(|_| "BFC_SERVICE_STATUS_FAILED")?
                .is_none()
            {
                return Ok(false);
            }
            // The former managed process is proven gone. Its handle is ours.
            *guard = None;
        }
        if app
            .state::<crate::AppState>()
            .shutdown_requested
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("BFC_SERVICE_SHUTTING_DOWN".into());
        }
        *guard = Some(spawn_managed_bfc(app)?);
        Ok(true)
    }

    /// Used by explicit Stop and ShutdownCoordinator. Retain the owner after a
    /// timeout; dropping the Windows Job at process exit remains fail-safe.
    pub fn stop_until(&self, deadline: Instant) -> Result<bool, String> {
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "BFC_SERVICE_OWNER_POISONED")?;
        let Some(process) = guard.as_mut() else {
            // A manually launched service is not ours and must not be stopped.
            return Ok(false);
        };
        #[cfg(windows)]
        if !process
            .job
            .is_empty()
            .map_err(|_| "BFC_SERVICE_JOB_UNAVAILABLE")?
        {
            process
                .job
                .terminate()
                .map_err(|_| "BFC_SERVICE_STOP_FAILED")?;
        }
        #[cfg(not(windows))]
        if process
            .child
            .try_wait()
            .map_err(|_| "BFC_SERVICE_STATUS_FAILED")?
            .is_none()
        {
            process
                .child
                .kill()
                .map_err(|_| "BFC_SERVICE_STOP_FAILED")?;
        }
        loop {
            let leader_gone = process
                .child
                .try_wait()
                .map_err(|_| "BFC_SERVICE_REAP_FAILED")?
                .is_some();
            #[cfg(windows)]
            let descendants_gone = process
                .job
                .is_empty()
                .map_err(|_| "BFC_SERVICE_JOB_UNAVAILABLE")?;
            #[cfg(not(windows))]
            let descendants_gone = leader_gone;
            if leader_gone && descendants_gone {
                *guard = None;
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Err("BFC_SERVICE_STOP_TIMEOUT".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

#[cfg(windows)]
fn spawn_managed_bfc(app: &AppHandle) -> Result<OwnedBfcChild, String> {
    use std::os::windows::{ffi::OsStringExt, process::CommandExt};
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    let local = std::env::var_os("LOCALAPPDATA").ok_or("BFC_SERVICE_HOME_UNAVAILABLE")?;
    let entry = std::path::PathBuf::from(local)
        .join("BrowserOS")
        .join("bfc-workbench")
        .join("server.mjs");
    if !entry.is_file() {
        return Err("BFC_SERVICE_NOT_INSTALLED".into());
    }

    // Always execute the packaged, reviewed ACL-preparing launcher, not a
    // script from the writable BFC checkout and not a shell command string.
    let script = app
        .path()
        .resolve(
            "scripts/windows/start-bfc-local.ps1",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|_| "BFC_SERVICE_LAUNCHER_MISSING")?;
    let script_meta =
        std::fs::symlink_metadata(&script).map_err(|_| "BFC_SERVICE_LAUNCHER_MISSING")?;
    if !script_meta.file_type().is_file() {
        return Err("BFC_SERVICE_LAUNCHER_UNTRUSTED".into());
    }
    let mut sys = [0u16; 32768];
    let size = unsafe { GetSystemDirectoryW(sys.as_mut_ptr(), sys.len() as u32) } as usize;
    if size == 0 || size >= sys.len() {
        return Err("BFC_SERVICE_SYSTEM_DIR_UNAVAILABLE".into());
    }
    let shell = std::path::PathBuf::from(std::ffi::OsString::from_wide(&sys[..size]))
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if !shell.is_file() {
        return Err("BFC_SERVICE_POWERSHELL_UNAVAILABLE".into());
    }

    let mut command = Command::new(shell);
    command
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(crate::windows_process::CREATE_SUSPENDED | 0x0800_0000);
    let mut child = command.spawn().map_err(|_| "BFC_SERVICE_SPAWN_FAILED")?;
    let job = match crate::windows_process::JobObject::attach_suspended_pid(child.id()) {
        Ok(job) => job,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err("BFC_SERVICE_PROCESS_OWNERSHIP_FAILED".into());
        }
    };
    Ok(OwnedBfcChild { child, job })
}

#[cfg(target_os = "linux")]
fn spawn_managed_bfc(_app: &AppHandle) -> Result<OwnedBfcChild, String> {
    use std::os::unix::process::CommandExt;
    let home = std::env::var_os("HOME").ok_or("BFC_SERVICE_HOME_UNAVAILABLE")?;
    let root = std::env::var_os("BROWSEROS_BFC_SERVICE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(home).join(".local/share/BrowserOS/bfc-workbench")
        });
    let source = std::fs::symlink_metadata(&root).map_err(|_| "BFC_SERVICE_NOT_INSTALLED")?;
    if !source.file_type().is_dir() {
        return Err("BFC_SERVICE_SOURCE_UNTRUSTED".into());
    }
    let entry = root.join("server.mjs");
    let script = std::fs::symlink_metadata(&entry).map_err(|_| "BFC_SERVICE_NOT_INSTALLED")?;
    if !script.file_type().is_file() {
        return Err("BFC_SERVICE_SOURCE_UNTRUSTED".into());
    }
    let data_dir = super::commands::bfc_data_dir().map_err(str::to_string)?;
    let parent_pid = unsafe { libc::getpid() };
    let mut command = Command::new("node");
    command
        .arg(entry)
        .env("BILIBILI_FAVORITES_DATA_DIR", data_dir)
        .env(
            "BROWSEROS_BFC_DATA_DIR",
            super::commands::bfc_data_dir().map_err(str::to_string)?,
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // A crash of BrowserOS must not leave the child Node service behind.
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent_pid {
                return Err(std::io::Error::other("BFC_PARENT_EXITED"));
            }
            Ok(())
        });
    }
    let child = command.spawn().map_err(|_| "BFC_SERVICE_SPAWN_FAILED")?;
    Ok(OwnedBfcChild { child })
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn spawn_managed_bfc(_app: &AppHandle) -> Result<OwnedBfcChild, String> {
    Err("BFC_SERVICE_PLATFORM_UNSUPPORTED".into())
}
