//! Read-only Windows packaged-app discovery and explicit user-triggered launch.
//! Never evaluates app-supplied strings as a command or script.
#![cfg(target_os = "windows")]

use serde_json::Value;
use std::collections::HashSet;
use std::io::Read;
use std::os::windows::{ffi::OsStringExt, fs::MetadataExt, process::CommandExt};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MAX_OUTPUT: u64 = 192 * 1024;

/// Only packaged AUMIDs are supported. Ordinary EXE/desktop links go through
/// check_launch_target; no shell protocols, separators, whitespace or quotes.
fn valid_packaged_id(id: &str) -> bool {
    if id.len() > 256 || id.len() < 3 || id.contains(['/', '\\', ':']) {
        return false;
    }
    let Some((package, app)) = id.split_once('!') else {
        return false;
    };
    !package.is_empty()
        && !app.is_empty()
        && !app.contains('!')
        && id.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || [b'.', b'-', b'_', b'!', b'+', b'~'].contains(&byte)
        })
}

fn windows_folder() -> Option<PathBuf> {
    let mut buffer = [0u16; 512];
    let size = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if size == 0 || size as usize >= buffer.len() {
        return None;
    }
    let system = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..size as usize]));
    Some(system.parent()?.to_path_buf())
}

fn powershell() -> Option<PathBuf> {
    let exe = windows_folder()?.join("System32/WindowsPowerShell/v1.0/powershell.exe");
    exe.is_file().then_some(exe)
}

/// PowerShell reads only the OS-maintained Start application list, with a
/// constant script. A concurrent reader prevents an output-pipe deadlock.
/// A bounded time/output budget prevents a stuck shell from freezing UI.
fn system_start_apps_json() -> Option<Vec<u8>> {
    const SCRIPT: &str = r#"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$items=@(Get-StartApps | Select-Object -First 512 | ForEach-Object {
  if($_.AppID -and $_.Name){[pscustomobject]@{name=[string]$_.Name;id=[string]$_.AppID}}
})
ConvertTo-Json -InputObject $items -Compress -Depth 3
"#;
    run_bounded_builtin_script(SCRIPT)
}

/// Execute only a repository-defined, compile-time-constant OS inventory
/// script. Shared by Start Menu shortcut discovery and AUMID enumeration.
pub(crate) fn run_bounded_builtin_script(script: &'static str) -> Option<Vec<u8>> {
    let mut child = Command::new(powershell()?)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut output = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        output
            .by_ref()
            .take(MAX_OUTPUT + 1)
            .read_to_end(&mut data)
            .ok()?;
        Some(data)
    });
    let deadline = Instant::now() + Duration::from_secs(8);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    if !success {
        return None;
    }
    let bytes = reader.join().ok()??;
    (bytes.len() as u64 <= MAX_OUTPUT).then_some(bytes)
}

/// The installed AUMID list is the allowlist. Re-check it at launch time:
/// a renderer cannot supply a fabricated shell:AppsFolder target.
pub fn installed_packaged_apps() -> Vec<(String, String)> {
    let Some(bytes) = system_start_apps_json() else {
        return Vec::new();
    };
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    let Ok(parsed) = serde_json::from_slice::<Value>(bytes) else {
        return Vec::new();
    };
    let Some(items) = parsed.as_array() else {
        return Vec::new();
    };
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for item in items.iter().take(512) {
        let (Some(name), Some(id)) = (item["name"].as_str(), item["id"].as_str()) else {
            continue;
        };
        if !valid_packaged_id(id)
            || name.is_empty()
            || name.len() > 128
            || name.chars().any(char::is_control)
        {
            continue;
        }
        if seen.insert(id.to_ascii_lowercase()) {
            result.push((name.to_owned(), id.to_owned()));
        }
        if result.len() >= 256 {
            break;
        }
    }
    result
}

pub fn launch_packaged_app(id: &str) -> Result<(), String> {
    if !valid_packaged_id(id) {
        return Err("WINDOWS_PACKAGED_APP_ID_INVALID".into());
    }
    if !installed_packaged_apps()
        .iter()
        .any(|(_, candidate)| candidate.eq_ignore_ascii_case(id))
    {
        return Err("WINDOWS_PACKAGED_APP_NOT_INSTALLED".into());
    }
    let explorer = windows_folder()
        .ok_or("WINDOWS_SHELL_UNAVAILABLE")?
        .join("explorer.exe");
    let metadata = std::fs::symlink_metadata(&explorer).map_err(|_| "WINDOWS_SHELL_UNAVAILABLE")?;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
        return Err("WINDOWS_SHELL_UNTRUSTED".into());
    }
    Command::new(explorer)
        .arg(format!("shell:AppsFolder\\{id}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "WINDOWS_PACKAGED_APP_LAUNCH_FAILED")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::valid_packaged_id;

    #[test]
    fn rejects_shell_injection_and_nonpackaged_ids() {
        assert!(valid_packaged_id(
            "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"
        ));
        assert!(!valid_packaged_id("powershell.exe"));
        assert!(!valid_packaged_id("App!A\\..\\system32"));
        assert!(!valid_packaged_id("Package!App;Start-Process"));
        assert!(!valid_packaged_id("Package!"));
        assert!(!valid_packaged_id("Package!Name!Extra"));
    }
}
