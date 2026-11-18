//! Windows AppContainer process isolation for explicitly approved MCP/LSP tools.
//! No network capabilities are granted. Only the selected runtime and an
//! optional private snapshot are exposed; the real workspace is never mounted.
#![cfg(windows)]

use crate::windows_process::{JobObject, CREATE_SUSPENDED};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::MetadataExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::mpsc::{SyncSender, TrySendError};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    CloseHandle, LocalFree, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, GetAppContainerFolderPath,
};
use windows_sys::Win32::Security::{
    FreeSid, GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
    SECURITY_CAPABILITIES, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess, GetExitCodeProcess,
    InitializeProcThreadAttributeList, OpenProcessToken, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT,
    EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

fn win32_error(operation: &str) -> String {
    format!("{operation}:{}", std::io::Error::last_os_error())
}
fn wide(value: &std::ffi::OsStr) -> Result<Vec<u16>, String> {
    if value.encode_wide().any(|unit| unit == 0) {
        return Err("SANDBOX_NUL_PATH".into());
    }
    Ok(value.encode_wide().chain(Some(0)).collect())
}
fn wide_text(value: &str) -> Result<Vec<u16>, String> {
    wide(std::ffi::OsStr::new(value))
}
/// Side-effect-free capability preflight used by the LSP status API.
/// This only checks prerequisites; it is NOT proof that a sandbox can launch.
pub(super) fn sandbox_prerequisites_available() -> bool {
    system_binary("cmd.exe").is_ok()
        && system_binary("icacls.exe").is_ok()
        && current_user_sid().is_ok()
}

pub(super) fn no_reparse_path(path: &Path) -> Result<(), String> {
    use std::path::{Component, Prefix};
    if !path.is_absolute()
        || !matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
    {
        return Err("SANDBOX_RUNTIME_NOT_ABSOLUTE".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_)) || !current.has_root() {
            continue;
        }
        let metadata =
            std::fs::symlink_metadata(&current).map_err(|_| "SANDBOX_PATH_INSPECTION_FAILED")?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err("SANDBOX_REPARSE_DENIED".into());
        }
    }
    Ok(())
}
fn system_binary(name: &str) -> Result<PathBuf, String> {
    let mut buffer = [0u16; 512];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 || length as usize >= buffer.len() {
        return Err("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE".into());
    }
    let path = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize])).join(name);
    if !path.is_file() {
        return Err("SANDBOX_SYSTEM_TOOL_UNAVAILABLE".into());
    }
    Ok(path)
}
fn current_user_sid() -> Result<String, String> {
    struct Token(HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0
        || token.is_null()
    {
        return Err("SANDBOX_USER_SID_UNAVAILABLE".into());
    }
    let token = Token(token);
    let mut required = 0u32;
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required < std::mem::size_of::<TOKEN_USER>() as u32 || required > 65536 {
        return Err("SANDBOX_USER_SID_INVALID".into());
    }
    let words = (required as usize).div_ceil(std::mem::size_of::<usize>());
    let mut buffer = vec![0usize; words];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err("SANDBOX_USER_SID_UNAVAILABLE".into());
    }
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    if sid.is_null() {
        return Err("SANDBOX_USER_SID_INVALID".into());
    }
    let mut sid_text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut sid_text) } == 0 || sid_text.is_null() {
        return Err("SANDBOX_USER_SID_UNAVAILABLE".into());
    }
    let mut len = 0usize;
    unsafe {
        while len < 256 && *sid_text.add(len) != 0 {
            len += 1;
        }
    }
    let value = if len < 256 {
        unsafe { String::from_utf16(std::slice::from_raw_parts(sid_text, len)) }
            .map_err(|_| "SANDBOX_USER_SID_INVALID".to_string())
    } else {
        Err("SANDBOX_USER_SID_INVALID".into())
    };
    unsafe {
        LocalFree(sid_text as _);
    }
    let value = value?;
    let valid = value.strip_prefix("S-1-").is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix.split('-').all(|component| {
                !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
            })
    });
    if !valid {
        return Err("SANDBOX_USER_SID_INVALID".into());
    }
    Ok(value)
}

pub struct SandboxProfile {
    name: String,
    sid: PSID,
    sid_string: String,
    folder: PathBuf,
}
unsafe impl Send for SandboxProfile {}

impl SandboxProfile {
    pub fn create() -> Result<Self, String> {
        let name = format!("BrowserOS.Coding.{}", uuid::Uuid::new_v4().simple());
        let name_wide = wide_text(&name)?;
        let display = wide_text("BrowserOS isolated coding tool")?;
        let description = wide_text("No network; private MCP/LSP execution")?;
        let mut sid = ptr::null_mut();
        let result = unsafe {
            CreateAppContainerProfile(
                name_wide.as_ptr(),
                display.as_ptr(),
                description.as_ptr(),
                ptr::null(),
                0,
                &mut sid,
            )
        };
        if result < 0 || sid.is_null() {
            return Err(format!("SANDBOX_PROFILE_CREATE:{result:x}"));
        }
        let profile = Self {
            name,
            sid,
            sid_string: String::new(),
            folder: PathBuf::new(),
        };
        profile.finish_create()
    }

    fn finish_create(mut self) -> Result<Self, String> {
        let mut sid_text = ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(self.sid, &mut sid_text) } == 0 || sid_text.is_null() {
            return Err(win32_error("SANDBOX_SID_TEXT"));
        }
        let mut length = 0usize;
        unsafe {
            while *sid_text.add(length) != 0 {
                length += 1;
            }
            self.sid_string = String::from_utf16(std::slice::from_raw_parts(sid_text, length))
                .map_err(|_| "SANDBOX_SID_TEXT")?;
            LocalFree(sid_text as _);
        }
        let name_wide = wide_text(&self.sid_string)?;
        let mut folder = ptr::null_mut();
        let result = unsafe { GetAppContainerFolderPath(name_wide.as_ptr(), &mut folder) };
        if result < 0 || folder.is_null() {
            return Err(format!("SANDBOX_PRIVATE_FOLDER:{result:x}"));
        }
        let mut length = 0usize;
        unsafe {
            while *folder.add(length) != 0 {
                length += 1;
            }
            self.folder = PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
                folder, length,
            )));
            CoTaskMemFree(folder as _);
        }
        std::fs::create_dir_all(&self.folder).map_err(|_| "SANDBOX_PRIVATE_FOLDER")?;
        // The AppContainer package must be able to traverse its profile root
        // before it can execute a staged runtime. Do not rewrite OS-managed
        // root ACLs or grant access to ALL APPLICATION PACKAGES.
        self.grant_profile_root_traverse()?;
        Ok(self)
    }

    /// Grant only this lowbox SID read/traverse on its Windows-managed
    /// profile root. State is independently writable; runtime and snapshot
    /// are separately read/execute-only. Do not replace the OS root DACL.
    fn grant_profile_root_traverse(&self) -> Result<(), String> {
        no_reparse_path(&self.folder)?;
        let icacls = system_binary("icacls.exe")?;
        let result = std::process::Command::new(icacls)
            .arg(&self.folder)
            .arg("/grant:r")
            .arg(format!("*{}:(RX)", self.sid_string))
            .output()
            .map_err(|_| "SANDBOX_PROFILE_TRAVERSE_ACL")?;
        if !result.status.success() {
            return Err("SANDBOX_PROFILE_TRAVERSE_ACL".into());
        }
        Ok(())
    }

    pub fn state_dir(&self) -> Result<PathBuf, String> {
        let state = self.folder.join("state");
        std::fs::create_dir(&state).map_err(|_| "SANDBOX_STATE_CREATE")?;
        self.grant_private_directory(&state)?;
        Ok(state)
    }
    fn grant_private_directory(&self, directory: &Path) -> Result<(), String> {
        no_reparse_path(directory)?;
        let icacls = system_binary("icacls.exe")?;
        let user = current_user_sid()?;
        let output = std::process::Command::new(icacls)
            .arg(directory)
            .args(["/inheritance:r", "/grant:r"])
            .arg(format!("*{user}:(OI)(CI)F"))
            .arg("*S-1-5-18:(OI)(CI)F")
            .arg(format!("*{}:(OI)(CI)M", self.sid_string))
            .arg("/T")
            .output()
            .map_err(|_| "SANDBOX_STATE_ACL")?;
        if !output.status.success() {
            return Err("SANDBOX_STATE_ACL".into());
        }
        Ok(())
    }
    pub fn snapshot_dir(&self) -> Result<PathBuf, String> {
        let snapshot = self.folder.join("snapshot");
        std::fs::create_dir(&snapshot).map_err(|_| "SANDBOX_SNAPSHOT_CREATE")?;
        Ok(snapshot)
    }
    pub fn sid_string(&self) -> &str {
        &self.sid_string
    }

    #[cfg(test)]
    pub fn stderr_for_test(&self) -> String {
        let path = self.folder.join("state/sandbox-stderr.log");
        let bytes = std::fs::read(path).unwrap_or_default();
        let tail = if bytes.len() > 16 * 1024 {
            &bytes[bytes.len() - 16 * 1024..]
        } else {
            &bytes[..]
        };
        String::from_utf8_lossy(tail).into_owned()
    }

    fn stage_runtime(&self, runtime: &Path) -> Result<PathBuf, String> {
        let canonical = runtime
            .canonicalize()
            .map_err(|_| "SANDBOX_RUNTIME_MISSING")?;
        no_reparse_path(&canonical)?;
        let system32 = system_binary("cmd.exe")?
            .parent()
            .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?
            .to_path_buf();
        // Only explicitly reviewed System32 tools bypass private staging.
        // A user-writable C:\Windows subdirectory must never be treated as
        // trusted merely because it is under the Windows installation root.
        let system32_dir = system32
            .canonicalize()
            .map_err(|_| "SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
        let name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        // Token inspection uses the genuine System32 whoami.exe.
        // Production System32 bypass policy stays restricted to cmd/curl.
        let trusted = ["cmd.exe", "curl.exe"].contains(&name.as_str());
        #[cfg(test)]
        let trusted = trusted || name == "whoami.exe";
        if canonical.parent() == Some(system32_dir.as_path()) && trusted {
            return Ok(canonical);
        }
        const MAX_RUNTIME_FILE: u64 = 256 * 1024 * 1024;
        const MAX_TOTAL_RUNTIME: u64 = 384 * 1024 * 1024;
        const MAX_SIBLING_DLLS: usize = 32;
        fn copy_component(from: &Path, to: &Path, budget: &mut u64) -> Result<(), String> {
            no_reparse_path(from)?;
            let mut source = OpenOptions::new()
                .read(true)
                .custom_flags(0x0020_0000)
                .open(from)
                .map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
            let metadata = source
                .metadata()
                .map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
            if !metadata.is_file()
                || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
                || metadata.len() > MAX_RUNTIME_FILE
            {
                return Err("SANDBOX_RUNTIME_UNSAFE".into());
            }
            *budget = budget
                .checked_add(metadata.len())
                .filter(|n| *n <= MAX_TOTAL_RUNTIME)
                .ok_or("SANDBOX_RUNTIME_SIZE_LIMIT")?;
            let mut output = match OpenOptions::new().write(true).create_new(true).open(to) {
                Ok(file) => file,
                // Shared toolchain DLLs are staged once for the first peer and
                // reused by later peers in the same private runtime. Only an
                // identical, already-staged regular file is accepted.
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Identical length is not proof of identical content.
                    // Toolchain peers may ship same-sized, incompatible DLLs.
                    no_reparse_path(to)?;
                    let mut existing = OpenOptions::new()
                        .read(true)
                        .custom_flags(0x0020_0000)
                        .open(to)
                        .map_err(|_| "SANDBOX_RUNTIME_STAGE_CONFLICT")?;
                    let current = existing.metadata()
                        .map_err(|_| "SANDBOX_RUNTIME_STAGE_CONFLICT")?;
                    if !current.is_file()
                        || current.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
                        || current.len() != metadata.len()
                    {
                        return Err("SANDBOX_RUNTIME_STAGE_CONFLICT".into());
                    }
                    let mut expected = [0u8; 64 * 1024];
                    let mut actual = [0u8; 64 * 1024];
                    let mut remaining = metadata.len();
                    while remaining > 0 {
                        let count = remaining.min(expected.len() as u64) as usize;
                        source.read_exact(&mut expected[..count])
                            .map_err(|_| "SANDBOX_RUNTIME_CHANGED_DURING_COPY")?;
                        existing.read_exact(&mut actual[..count])
                            .map_err(|_| "SANDBOX_RUNTIME_STAGE_CONFLICT")?;
                        if expected[..count] != actual[..count] {
                            return Err("SANDBOX_RUNTIME_STAGE_CONFLICT".into());
                        }
                        remaining -= count as u64;
                    }
                    return Ok(());
                }
                Err(_) => return Err("SANDBOX_RUNTIME_STAGE_FAILED".into()),
            };
            let written = std::io::copy(&mut source, &mut output)
                .map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
            if written != metadata.len() {
                return Err("SANDBOX_RUNTIME_CHANGED_DURING_COPY".into());
            }
            output
                .sync_all()
                .map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
            no_reparse_path(from)?;
            Ok(())
        }
        // Never place executable bytes under the writable state directory:
        // a lowbox child with state MODIFY rights could otherwise replace its
        // own runtime via DELETE_CHILD/rename despite a file-level RX ACL.
        // The profile root grants traverse only; runtime is a sibling of
        // writable state and is separately protected with read/execute ACLs.
        let directory = self.folder.join("runtime");
        match std::fs::create_dir(&directory) {
            Ok(()) => {}
            // A language toolchain is staged as several executables that share
            // one private runtime. Repeat staging must reuse that directory
            // instead of failing, and file-level ACLs are re-applied below.
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err("SANDBOX_RUNTIME_STAGE_FAILED".into()),
        }
        no_reparse_path(&directory)?;
        let filename = canonical
            .file_name()
            .ok_or("SANDBOX_RUNTIME_NOT_EXECUTABLE")?;
        let staged = directory.join(filename);
        let mut total = 0u64;
        copy_component(&canonical, &staged, &mut total)?;
        let mut dll_count = 0usize;
        let original_dir = canonical.parent().ok_or("SANDBOX_RUNTIME_NOT_EXECUTABLE")?;
        for entry in std::fs::read_dir(original_dir).map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")? {
            let entry = entry.map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
            let path = entry.path();
            if path
                .extension()
                .and_then(|v| v.to_str())
                .is_some_and(|v| v.eq_ignore_ascii_case("dll"))
            {
                dll_count += 1;
                if dll_count > MAX_SIBLING_DLLS {
                    return Err("SANDBOX_RUNTIME_DLL_LIMIT".into());
                }
                copy_component(&path, &directory.join(entry.file_name()), &mut total)?;
            }
        }
        self.protect_snapshot(&directory)?;
        // A directory-level (OI)(CI) grant is not enough evidence for the
        // image file itself: icacls recursion and inherited ACE flags differ
        // between files and directories. Pin read/execute directly onto every
        // staged file so CreateProcessW can map the image in the lowbox.
        self.protect_staged_files(&directory)?;
        // Pin the original executable name to a newly staged read-only file.
        // Access to the private runtime tree must not depend on writable state.
        no_reparse_path(&staged)?;
        Ok(staged)
    }

    fn protect_staged_files(&self, directory: &Path) -> Result<(), String> {
        let icacls = system_binary("icacls.exe")?;
        let user = current_user_sid()?;
        let entries = std::fs::read_dir(directory).map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
        for entry in entries {
            let path = entry.map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?.path();
            if !path.is_file() {
                continue;
            }
            no_reparse_path(&path)?;
            let output = std::process::Command::new(&icacls)
                .arg(&path)
                .args(["/inheritance:r", "/grant:r"])
                .arg(format!("*{user}:F"))
                .arg("*S-1-5-18:F")
                .arg(format!("*{}:RX", self.sid_string))
                .output()
                .map_err(|_| "SANDBOX_RUNTIME_FILE_ACL")?;
            if !output.status.success() {
                return Err("SANDBOX_RUNTIME_FILE_ACL".into());
            }
        }
        Ok(())
    }

    /// Copy a read-only source tree into the private profile and give every
    /// file the ACL shape the staged runtime images already prove works
    /// inside the lowbox. Rust Analyzer resolves the standard library from
    /// the sysroot, which lives outside the profile, so its `rust-src` checkout
    /// has to be materialised here before the analyzer starts.
    pub fn stage_readonly_tree(&self, source: &Path, destination: &Path) -> Result<PathBuf, String> {
        no_reparse_path(source)?;
        std::fs::create_dir_all(destination).map_err(|_| "SANDBOX_STAGE_TREE_FAILED")?;
        // Seed the destination with inheritable grants *before* copying, so
        // every copied file and directory receives the ACEs by inheritance.
        // Per-file `icacls` with `(OI)(CI)` flags is silently refused on
        // files, and running `icacls` once per file over a source checkout
        // costs minutes.
        let icacls = system_binary("icacls.exe")?;
        let user = current_user_sid()?;
        let output = std::process::Command::new(&icacls)
            .arg(destination)
            .args(["/inheritance:r", "/grant:r"])
            .arg(format!("*{user}:(OI)(CI)F"))
            .arg("*S-1-5-18:(OI)(CI)F")
            .arg(format!("*{}:(OI)(CI)RX", self.sid_string))
            .output()
            .map_err(|_| "SANDBOX_STAGE_TREE_ACL")?;
        if !output.status.success() {
            return Err("SANDBOX_STAGE_TREE_ACL".into());
        }
        let mut pending = vec![(source.to_path_buf(), destination.to_path_buf())];
        while let Some((from, to)) = pending.pop() {
            for entry in std::fs::read_dir(&from).map_err(|_| "SANDBOX_STAGE_TREE_FAILED")? {
                let entry = entry.map_err(|_| "SANDBOX_STAGE_TREE_FAILED")?;
                let path = entry.path();
                no_reparse_path(&path)?;
                let target = to.join(entry.file_name());
                if path.is_dir() {
                    std::fs::create_dir_all(&target).map_err(|_| "SANDBOX_STAGE_TREE_FAILED")?;
                    pending.push((path, target));
                } else if path.is_file() {
                    std::fs::copy(&path, &target).map_err(|_| "SANDBOX_STAGE_TREE_FAILED")?;
                }
            }
        }
        Ok(destination.to_path_buf())
    }

    /// Apply the same per-file ACL shape that staged runtime images already
    /// prove works inside the lowbox (owner full control, SYSTEM full control,
    /// package read/execute) to a read-only snapshot tree. Snapshot entries
    /// that only carry the package SID are still rejected by `cargo` with
    /// "Access is denied (os error 5)" while the identically-shaped staged
    /// images load fine, so the snapshot must mirror the staged layout.
    pub fn protect_snapshot_files(&self, root: &Path) -> Result<(), String> {
        no_reparse_path(root)?;
        let icacls = system_binary("icacls.exe")?;
        // `cargo metadata` resolves the workspace by writing `Cargo.lock` next
        // to the manifest, so the directory itself must accept a create
        // inside the lowbox while every existing file stays read-only.
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            let output = std::process::Command::new(&icacls)
                .arg(&directory)
                .arg("/grant")
                .arg(format!("*{}:(OI)(CI)M", self.sid_string))
                .output()
                .map_err(|_| "SANDBOX_SNAPSHOT_ACL")?;
            if !output.status.success() {
                return Err("SANDBOX_SNAPSHOT_ACL".into());
            }
            self.protect_staged_files(&directory)?;
            for entry in std::fs::read_dir(&directory).map_err(|_| "SANDBOX_SNAPSHOT_ACL")? {
                let path = entry.map_err(|_| "SANDBOX_SNAPSHOT_ACL")?.path();
                if path.is_dir() {
                    pending.push(path);
                }
            }
        }
        Ok(())
    }

    pub fn protect_snapshot(&self, snapshot: &Path) -> Result<(), String> {
        no_reparse_path(snapshot)?;
        let icacls = system_binary("icacls.exe")?;
        let user = current_user_sid()?;
        let output = std::process::Command::new(icacls)
            .arg(snapshot)
            .args(["/inheritance:r", "/grant:r"])
            .arg(format!("*{user}:(OI)(CI)F"))
            .arg("*S-1-5-18:(OI)(CI)F")
            .arg(format!("*{}:(OI)(CI)RX", self.sid_string))
            .arg("/T")
            .output()
            .map_err(|_| "SANDBOX_SNAPSHOT_ACL")?;
        if !output.status.success() {
            return Err("SANDBOX_SNAPSHOT_ACL".into());
        }
        Ok(())
    }

    /// Stage an additional executable from the same reviewed toolchain into
    /// the private runtime before the main process is spawned. Rust Analyzer
    /// invokes `cargo locate-project`, `cargo metadata` and `rustc --version`
    /// as non-building probes, so those images must exist inside the lowbox
    /// instead of being replaced by nonexistent placeholder paths.
    pub fn stage_runtime_peer(&self, peer: &Path) -> Result<PathBuf, String> {
        no_reparse_path(peer)?;
        let canonical = peer
            .canonicalize()
            .map_err(|_| "SANDBOX_TOOLCHAIN_PEER_MISSING")?;
        if !canonical.is_file()
            || canonical
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                != "exe"
        {
            return Err("SANDBOX_TOOLCHAIN_PEER_MISSING".into());
        }
        self.stage_runtime(&canonical)
    }

    pub fn spawn(
        self,
        runtime: &Path,
        args: &[String],
        environment: &[(String, String)],
        current_directory: &Path,
    ) -> Result<SandboxChild, String> {
        no_reparse_path(runtime)?;
        no_reparse_path(current_directory)?;
        let profile_root = self
            .folder
            .canonicalize()
            .map_err(|_| "SANDBOX_PRIVATE_FOLDER")?;
        let actual_workdir = current_directory
            .canonicalize()
            .map_err(|_| "SANDBOX_WORKDIR_UNAVAILABLE")?;
        if !actual_workdir.starts_with(&profile_root) || actual_workdir == profile_root {
            return Err("SANDBOX_WORKDIR_OUTSIDE_PRIVATE_PROFILE".into());
        }
        let canonical = runtime
            .canonicalize()
            .map_err(|_| "SANDBOX_RUNTIME_MISSING")?;
        if !canonical.is_file()
            || runtime
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                != "exe"
        {
            return Err("SANDBOX_RUNTIME_NOT_EXECUTABLE".into());
        }
        let staged = self.stage_runtime(runtime)?;
        spawn_inner(self, &staged, args, environment, current_directory)
    }
}

impl Drop for SandboxProfile {
    fn drop(&mut self) {
        unsafe {
            if !self.sid.is_null() {
                FreeSid(self.sid);
                self.sid = ptr::null_mut();
            }
            if let Ok(name) = wide_text(&self.name) {
                let _ = DeleteAppContainerProfile(name.as_ptr());
            }
        }
        if !self.folder.as_os_str().is_empty() {
            let _ = std::fs::remove_dir_all(&self.folder);
        }
    }
}

pub struct SandboxProcess {
    handle: HANDLE,
    profile: SandboxProfile,
    _job: JobObject,
}
unsafe impl Send for SandboxProcess {}
impl SandboxProcess {
    pub fn exit_code(&self) -> Result<Option<u32>, String> {
        // Exit code 259 is a valid terminal exit code; only the kernel
        // process signaled state can establish process completion.
        match unsafe { WaitForSingleObject(self.handle, 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0u32;
                if unsafe { GetExitCodeProcess(self.handle, &mut code) } == 0 {
                    return Err(win32_error("SANDBOX_EXIT_CODE"));
                }
                Ok(Some(code))
            }
            _ => Err(win32_error("SANDBOX_WAIT")),
        }
    }
    pub fn try_wait(&self) -> Result<bool, String> {
        match unsafe { WaitForSingleObject(self.handle, 0) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(win32_error("SANDBOX_WAIT")),
        }
    }
    pub fn terminate(&self) -> Result<(), String> {
        self._job.terminate()
    }
    pub fn profile(&self) -> &SandboxProfile {
        &self.profile
    }
}
impl Drop for SandboxProcess {
    fn drop(&mut self) {
        let _ = self._job.terminate();
        unsafe {
            let _ = WaitForSingleObject(self.handle, 2000);
            CloseHandle(self.handle);
        }
    }
}

pub struct SandboxChild {
    pub stdin: File,
    pub stdout: File,
    pub process: SandboxProcess,
}

fn appcontainer_pipe_security(user: &str, sid: &str) -> Result<PSECURITY_DESCRIPTOR, String> {
    // AC means ALL APPLICATION PACKAGES. Never grant the entire lowbox
    // population access to a private stdio pipe: only this AppContainer SID.
    let sddl = format!("D:(A;;GA;;;SY)(A;;GA;;;{user})(A;;GA;;;{sid})");
    let text = wide_text(&sddl)?;
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    let mut size = 0u32;
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            1,
            &mut descriptor,
            &mut size,
        )
    } == 0
    {
        return Err(win32_error("SANDBOX_PIPE_ACL"));
    }
    Ok(descriptor)
}
struct OwnedSecurityDescriptor(PSECURITY_DESCRIPTOR);
impl Drop for OwnedSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
fn pipe(security: PSECURITY_DESCRIPTOR) -> Result<(File, File), String> {
    let mut read = ptr::null_mut();
    let mut write = ptr::null_mut();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security,
        bInheritHandle: 1,
    };
    if unsafe { CreatePipe(&mut read, &mut write, &attributes, 0) } == 0 {
        return Err(win32_error("SANDBOX_PIPE"));
    }
    Ok(unsafe { (File::from_raw_handle(read), File::from_raw_handle(write)) })
}
fn inheritable(handle: RawHandle, enabled: bool) -> Result<(), String> {
    if unsafe {
        SetHandleInformation(
            handle as HANDLE,
            HANDLE_FLAG_INHERIT,
            if enabled { HANDLE_FLAG_INHERIT } else { 0 },
        )
    } == 0
    {
        return Err(win32_error("SANDBOX_HANDLE_INHERIT"));
    }
    Ok(())
}
fn quote_argument(value: &str) -> Result<Vec<u16>, String> {
    if value.contains('\0') {
        return Err("SANDBOX_ARGUMENT_NUL".into());
    }
    let mut output = String::from("\"");
    let mut slashes = 0usize;
    for character in value.chars() {
        match character {
            '\\' => slashes += 1,
            '"' => {
                output.extend(std::iter::repeat('\\').take(slashes * 2 + 1));
                output.push('"');
                slashes = 0;
            }
            _ => {
                output.extend(std::iter::repeat('\\').take(slashes));
                slashes = 0;
                output.push(character);
            }
        }
    }
    output.extend(std::iter::repeat('\\').take(slashes * 2));
    output.push('"');
    wide_text(&output)
}
/// Windows command lines are not arrays. cmd.exe /C and /K parse the
/// remainder as a single command string, unlike normal CRT argv programs.
/// Only trusted system cmd.exe receives the special tail, and production MCP
/// denies shell executables before reaching SandboxProfile::spawn.
fn process_command_line(runtime: &Path, args: &[String]) -> Result<Vec<u16>, String> {
    let runtime_text = runtime.to_str().ok_or("SANDBOX_RUNTIME_NOT_UNICODE")?;
    let mut command = quote_argument(runtime_text)?;
    command.pop(); // remove the UTF-16 NUL from quote_argument
    let system_cmd = system_binary("cmd.exe")?
        .canonicalize()
        .map_err(|_| "SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
    let is_cmd = runtime
        .to_string_lossy()
        .eq_ignore_ascii_case(&system_cmd.to_string_lossy());
    let cmd_at = if is_cmd {
        args.iter()
            .position(|v| v.eq_ignore_ascii_case("/C") || v.eq_ignore_ascii_case("/K"))
    } else {
        None
    };
    if let Some(index) = cmd_at {
        if index + 1 >= args.len()
            || args[..index].iter().any(|item| {
                !["/D", "/Q", "/E:OFF", "/E:ON", "/V:OFF", "/V:ON"]
                    .iter()
                    .any(|allowed| item.eq_ignore_ascii_case(allowed))
            })
        {
            return Err("SANDBOX_CMD_OPTIONS_INVALID".into());
        }
        for option in &args[..=index] {
            command.push(' ' as u16);
            command.extend(option.encode_utf16());
        }
        let tail = args[index + 1..].join(" ");
        if tail.trim().is_empty() || tail.len() > 8191 || tail.contains(['\0', '\r', '\n']) {
            return Err("SANDBOX_CMD_TAIL_INVALID".into());
        }
        command.push(' ' as u16);
        // No enclosing quotes or CRT escaping: cmd owns /C tail parsing.
        command.extend(tail.encode_utf16());
    } else {
        for argument in args {
            let quoted = quote_argument(argument)?;
            command.push(' ' as u16);
            command.extend(quoted.into_iter().take_while(|u| *u != 0));
        }
    }
    command.push(0);
    if command.len() > 32767 {
        return Err("SANDBOX_COMMAND_LINE_TOO_LONG".into());
    }
    Ok(command)
}

fn environment_block(
    configured: &[(String, String)],
    state: &Path,
    appcontainer_local_data: &Path,
) -> Result<Vec<u16>, String> {
    let system32 = system_binary("cmd.exe")?
        .parent()
        .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?
        .to_path_buf();
    let windows_dir = system32
        .parent()
        .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
    let system_drive = windows_dir
        .components()
        .next()
        .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?
        .as_os_str()
        .to_string_lossy()
        .to_string();
    let mut values = BTreeMap::<String, String>::new();
    values.insert(
        "SYSTEMROOT".into(),
        windows_dir.to_string_lossy().into_owned(),
    );
    values.insert("WINDIR".into(), windows_dir.to_string_lossy().into_owned());
    values.insert("SYSTEMDRIVE".into(), system_drive);
    values.insert(
        "COMSPEC".into(),
        system_binary("cmd.exe")?.to_string_lossy().into_owned(),
    );
    values.insert("PATH".into(), system32.to_string_lossy().into_owned());
    values.insert("RUST_BACKTRACE".into(), "0".into());
    const RESERVED: &[&str] = &[
        "SYSTEMROOT",
        "SYSTEMDRIVE",
        "WINDIR",
        "COMSPEC",
        "PATH",
        "LOCALAPPDATA",
        "USERPROFILE",
        "HOME",
        "TEMP",
        "TMP",
        "APPDATA",
        "PATHEXT",
        "RUST_BACKTRACE",
    ];
    for (key, value) in configured {
        if key.is_empty()
            || key.len() > 128
            || value.len() > 16384
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || value.contains('\0')
            || key.starts_with('=')
            || RESERVED
                .iter()
                .any(|reserved| key.eq_ignore_ascii_case(reserved))
        {
            return Err("SANDBOX_ENV_INVALID".into());
        }
        let key = key.to_ascii_uppercase();
        if values.insert(key, value.clone()).is_some() {
            return Err("SANDBOX_ENV_DUPLICATE".into());
        }
    }
    // GetAppContainerFolderPath, not the host-user's LOCALAPPDATA, is the
    // documented AppContainer local data root.
    no_reparse_path(appcontainer_local_data)?;
    let local_app_data = appcontainer_local_data
        .to_str()
        .ok_or("SANDBOX_LOCAL_APP_DATA_INVALID")?;
    if !appcontainer_local_data.is_absolute() || !appcontainer_local_data.is_dir() {
        return Err("SANDBOX_LOCAL_APP_DATA_INVALID".into());
    }
    values.insert("LOCALAPPDATA".into(), local_app_data.to_owned());
    let state_text = state.to_str().ok_or("SANDBOX_STATE_NOT_UNICODE")?;
    // Private roaming/temp paths keep Node/rust-analyzer away from host data.
    for key in ["HOME", "USERPROFILE", "APPDATA", "TEMP", "TMP"] {
        values.insert(key.into(), state_text.into());
    }
    let mut block = Vec::<u16>::new();
    for (key, value) in values {
        block.extend(key.encode_utf16());
        block.push('=' as u16);
        block.extend(value.encode_utf16());
        block.push(0);
    }
    block.push(0);
    Ok(block)
}

pub(super) fn send_bounded<T>(
    sender: &SyncSender<T>,
    mut value: T,
    budget: Duration,
) -> Result<(), ()> {
    let until = Instant::now() + budget;
    loop {
        match sender.try_send(value) {
            Ok(()) => return Ok(()),
            Err(TrySendError::Disconnected(_)) => return Err(()),
            Err(TrySendError::Full(pending)) => {
                value = pending;
                let remaining = until.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(());
                }
                thread::sleep(remaining.min(Duration::from_millis(5)));
            }
        }
    }
}

fn spawn_inner(
    profile: SandboxProfile,
    runtime: &Path,
    args: &[String],
    environment: &[(String, String)],
    current_directory: &Path,
) -> Result<SandboxChild, String> {
    let user_sid = current_user_sid()?;
    let pipe_security =
        OwnedSecurityDescriptor(appcontainer_pipe_security(&user_sid, &profile.sid_string)?);
    let (child_input, parent_input) = pipe(pipe_security.0)?;
    let (parent_output, child_output) = pipe(pipe_security.0)?;
    drop(pipe_security);
    inheritable(parent_output.as_raw_handle(), false)?;
    inheritable(parent_input.as_raw_handle(), false)?;
    inheritable(child_input.as_raw_handle(), true)?;
    inheritable(child_output.as_raw_handle(), true)?;
    // Native tests retain child stderr to diagnose runtime/ACL failures.
    // In production, untrusted MCP/LSP tools must not be able to append
    // unlimited stderr to disk or persist credentials in a diagnostic file.
    #[cfg(test)]
    let error_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(current_directory.join("sandbox-stderr.log"))
        .map_err(|_| "SANDBOX_STDERR_LOG")?;
    #[cfg(not(test))]
    let error_file = OpenOptions::new()
        .write(true)
        .open("NUL")
        .map_err(|_| "SANDBOX_STDERR_SINK")?;
    inheritable(error_file.as_raw_handle(), true)?;

    let mut size = 0usize;
    unsafe {
        let _ = InitializeProcThreadAttributeList(ptr::null_mut(), 2, 0, &mut size);
    }
    if size == 0 {
        return Err("SANDBOX_ATTRIBUTE_SIZE".into());
    }
    // PROC_THREAD_ATTRIBUTE_LIST requires native pointer alignment.
    let words = size.div_ceil(std::mem::size_of::<usize>());
    let mut attribute_storage = vec![0usize; words];
    let attribute_list = attribute_storage.as_mut_ptr() as *mut _;
    if unsafe { InitializeProcThreadAttributeList(attribute_list, 2, 0, &mut size) } == 0 {
        return Err(win32_error("SANDBOX_ATTRIBUTE_INIT"));
    }
    let cleanup_attributes = |list| unsafe { DeleteProcThreadAttributeList(list) };
    let mut capabilities = SECURITY_CAPABILITIES {
        AppContainerSid: profile.sid,
        Capabilities: ptr::null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    let inherited = [
        child_input.as_raw_handle() as HANDLE,
        child_output.as_raw_handle() as HANDLE,
        error_file.as_raw_handle() as HANDLE,
    ];
    let security_ok = unsafe {
        UpdateProcThreadAttribute(
            attribute_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            &mut capabilities as *mut _ as *const _,
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            ptr::null_mut(),
            ptr::null(),
        )
    };
    let handles_ok = security_ok != 0
        && unsafe {
            UpdateProcThreadAttribute(
                attribute_list,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                inherited.as_ptr() as *const _,
                std::mem::size_of_val(&inherited),
                ptr::null_mut(),
                ptr::null(),
            ) != 0
        };
    if !handles_ok {
        let error = win32_error("SANDBOX_ATTRIBUTE_UPDATE");
        cleanup_attributes(attribute_list);
        return Err(error);
    }

    let mut command = process_command_line(runtime, args)?;
    let application = wide(runtime.as_os_str())?;
    let directory = wide(current_directory.as_os_str())?;
    let environment = environment_block(environment, current_directory, &profile.folder)?;
    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = child_input.as_raw_handle() as HANDLE;
    startup.StartupInfo.hStdOutput = child_output.as_raw_handle() as HANDLE;
    startup.StartupInfo.hStdError = error_file.as_raw_handle() as HANDLE;
    startup.lpAttributeList = attribute_list;
    let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let created = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            CREATE_SUSPENDED
                | CREATE_NEW_PROCESS_GROUP
                | CREATE_UNICODE_ENVIRONMENT
                | EXTENDED_STARTUPINFO_PRESENT,
            environment.as_ptr() as *const _,
            directory.as_ptr(),
            &startup.StartupInfo,
            &mut information,
        )
    };
    // Capture GetLastError before cleaning up attribute lists or handles:
    // those Win32 calls may overwrite the real CreateProcess failure code.
    let create_error = if created == 0 {
        #[allow(unused_mut)]
        let mut error = win32_error("SANDBOX_CREATE_PROCESS");
        #[cfg(test)]
        {
            // Native CI only: expose the staged image ACL next to the Win32
            // error so an AppContainer image-access failure is diagnosable
            // from the public annotation instead of another blind run.
            error.push_str(&format!(" image={}", runtime.display()));
            if let Ok(icacls) = system_binary("icacls.exe") {
                if let Ok(probe) = std::process::Command::new(icacls).arg(runtime).output() {
                    let acl = String::from_utf8_lossy(&probe.stdout)
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    error.push_str(&format!(" acl={}", &acl[..acl.len().min(700)]));
                }
            }
        }
        Some(error)
    } else {
        None
    };
    cleanup_attributes(attribute_list);
    drop(child_input);
    drop(child_output);
    drop(error_file);
    if let Some(error) = create_error {
        return Err(error);
    }
    let job = match JobObject::attach_suspended_pid(information.dwProcessId) {
        Ok(job) => job,
        Err(error) => {
            unsafe {
                TerminateProcess(information.hProcess, 1);
                WaitForSingleObject(information.hProcess, 2000);
                CloseHandle(information.hThread);
                CloseHandle(information.hProcess);
            }
            return Err(error);
        }
    };
    unsafe { CloseHandle(information.hThread) };
    Ok(SandboxChild {
        stdin: parent_input,
        stdout: parent_output,
        process: SandboxProcess {
            handle: information.hProcess,
            profile,
            _job: job,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::time::Duration;

    // The read has a deadline so a broken sandbox child cannot block the
    // entire Windows native matrix until the outer CI timeout.
    fn run_isolated(
        executable: &Path,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<(String, String, u32), String> {
        let profile = SandboxProfile::create()?;
        let state = profile.state_dir()?;
        let SandboxChild {
            stdin,
            mut stdout,
            process,
        } = profile.spawn(executable, args, env, &state)?;
        drop(stdin);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = stdout
                .read_to_end(&mut output)
                .map(|_| output)
                .map_err(|_| "SANDBOX_FIXTURE_READ".to_string());
            let _ = sender.send(result);
        });
        let bytes = match receiver.recv_timeout(Duration::from_secs(8)) {
            Ok(result) => result?,
            Err(_) => {
                let _ = process.terminate();
                return Err("SANDBOX_FIXTURE_STDOUT_TIMEOUT".into());
            }
        };
        let deadline = Instant::now() + Duration::from_secs(3);
        while !process.try_wait()? {
            if Instant::now() >= deadline {
                let _ = process.terminate();
                return Err("SANDBOX_FIXTURE_EXIT_TIMEOUT".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let exit = process
            .exit_code()?
            .ok_or("SANDBOX_FIXTURE_EXIT_UNAVAILABLE")?;
        let stderr = std::fs::read(state.join("sandbox-stderr.log")).unwrap_or_default();
        Ok((
            String::from_utf8_lossy(&bytes).into_owned(),
            String::from_utf8_lossy(&stderr).into_owned(),
            exit,
        ))
    }

    #[test]
    fn appcontainer_runs_without_host_workspace_or_loopback_access() {
        // A closed port is not a network test: unrestricted callers fail too.
        // Use a live localhost listener and first prove host connectivity.
        let listener =
            std::net::TcpListener::bind(("127.0.0.1", 0)).expect("host loopback fixture");
        let endpoint = listener.local_addr().unwrap();
        let host_probe = std::net::TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))
            .expect("host loopback positive control");
        let (accepted, _) = listener.accept().expect("host accepted baseline");
        drop(host_probe);
        drop(accepted);
        listener.set_nonblocking(true).unwrap();
        let curl = system_binary("curl.exe").expect("Windows System32 curl fixture");
        let args = vec![
            "-sS".into(),
            "--connect-timeout".into(),
            "2".into(),
            "--max-time".into(),
            "3".into(),
            "--noproxy".into(),
            "*".into(),
            format!("http://127.0.0.1:{}/probe", endpoint.port()),
        ];
        let (stdout, stderr, exit) =
            run_isolated(&curl, &args, &[]).expect("run curl in no-network AppContainer");
        assert_ne!(exit, 0, "sandbox connected to localhost: {stdout} {stderr}");
        // The executable must have initialized and reported a curl error.
        // A DLL loader failure must not masquerade as network isolation.
        assert!(
            stderr.contains("curl:"),
            "curl did not initialize: {stderr}"
        );
        match listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            _ => panic!("sandbox reached a listening localhost endpoint"),
        }

        // Validate the SAME command string outside AppContainer first. The
        // lowbox process must not read the host-owned, explicitly locked file.
        // cmd avoids requiring PowerShell CLR/module initialization.
        // Deliberately include whitespace to cover Windows path quoting.
        let host_file = std::env::temp_dir().join(format!(
            "browseros sandbox host {}.txt",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&host_file, "host-secret-marker").unwrap();
        let user = current_user_sid().expect("host user SID");
        let icacls = system_binary("icacls.exe").unwrap();
        let acl = std::process::Command::new(icacls)
            .arg(&host_file)
            .args(["/inheritance:r", "/grant:r"])
            .arg(format!("*{user}:F"))
            .arg("*S-1-5-18:F")
            .output()
            .unwrap();
        assert!(
            acl.status.success(),
            "host ACL setup failed: {}",
            String::from_utf8_lossy(&acl.stderr)
        );
        let cmd = system_binary("cmd.exe").unwrap();
        let read_command = format!("type \"{}\"", host_file.display());
        // std::process::Command::arg() quotes the whole /C string using CRT
        // rules; cmd.exe then strips an unmatched outer quote. raw_arg()
        // appends the actual /C command tail, without that extra wrapper.
        let baseline = std::process::Command::new(&cmd)
            .args(["/D", "/C"])
            .raw_arg(&read_command)
            .output()
            .unwrap();
        assert!(
            baseline.status.success()
                && String::from_utf8_lossy(&baseline.stdout).contains("host-secret-marker"),
            "host-file positive control invalid: {}",
            String::from_utf8_lossy(&baseline.stderr)
        );
        let (stdout, stderr, exit) =
            run_isolated(&cmd, &["/D".into(), "/C".into(), read_command], &[])
                .expect("sandbox host-file probe");
        assert!(
            !stdout.contains("host-secret-marker") && exit != 0,
            "AppContainer read host-only file: stdout={stdout:?} stderr={stderr:?}"
        );
        assert!(
            !stderr.to_ascii_lowercase().contains("not recognized"),
            "cmd did not execute the host-file probe: stderr={stderr:?}"
        );
        std::fs::remove_file(host_file).unwrap();
    }

    #[test]
    fn cmd_command_line_preserves_raw_tail_and_quoted_file_path() {
        let cmd = system_binary("cmd.exe").unwrap().canonicalize().unwrap();
        let args = vec!["/D".into(), "/C".into(), "echo sandbox-ok".into()];
        let command = process_command_line(&cmd, &args).unwrap();
        let decoded = String::from_utf16_lossy(&command[..command.len() - 1]);
        assert!(decoded.ends_with(" /D /C echo sandbox-ok"), "{decoded}");
        assert!(!decoded.ends_with("\"echo sandbox-ok\""), "{decoded}");
        let args = vec![
            "/D".into(),
            "/C".into(),
            "type \"C:\\Test Folder\\test.txt\"".into(),
        ];
        let command = process_command_line(&cmd, &args).unwrap();
        let decoded = String::from_utf16_lossy(&command[..command.len() - 1]);
        assert!(
            decoded.ends_with(" /D /C type \"C:\\Test Folder\\test.txt\""),
            "{decoded}"
        );
    }

    #[test]
    fn quoted_arguments_survive_process_creation() {
        // The extra /S strips quotes and had mangled echo on CI.
        let cmd = system_binary("cmd.exe").unwrap();
        let (stdout, stderr, exit) = run_isolated(
            &cmd,
            &["/D".into(), "/C".into(), "echo sandbox-ok".into()],
            &[],
        )
        .expect("run cmd inside AppContainer");
        assert_eq!(exit, 0, "cmd exited with {exit}: {stderr}");
        assert!(
            stdout.contains("sandbox-ok"),
            "cmd quoting failed: stdout={stdout:?} stderr={stderr:?}"
        );
    }
}
