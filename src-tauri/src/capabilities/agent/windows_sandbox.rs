//! Windows AppContainer process isolation for explicitly approved MCP/LSP tools.
//! No network capabilities are granted. Only the selected runtime and an
//! optional private snapshot are exposed; the real workspace is never mounted.
#![cfg(windows)]

use crate::windows_process::{CREATE_SUSPENDED, JobObject};
use std::collections::BTreeMap;
use std::io::Write;
use std::os::windows::fs::OpenOptionsExt;
use std::sync::mpsc::{SyncSender, TrySendError};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::{File, OpenOptions};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::path::{Path, PathBuf};
use std::ptr;
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, HANDLE_FLAG_INHERIT, LocalFree, SetHandleInformation, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, GetAppContainerFolderPath,
};
use windows_sys::Win32::Security::{
    FreeSid, GetTokenInformation, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
    SECURITY_CAPABILITIES,
    TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList,
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, PROCESS_INFORMATION,
    STARTF_USESTDHANDLES,
    STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
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
fn no_reparse_path(path: &Path) -> Result<(), String> {
    use std::path::{Component, Prefix};
    if !path.is_absolute() || !matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    ) { return Err("SANDBOX_RUNTIME_NOT_ABSOLUTE".into()); }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_)) || !current.has_root() { continue; }
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
        fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
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
            token.0, TokenUser, buffer.as_mut_ptr().cast(), required, &mut required,
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
        while len < 256 && *sid_text.add(len) != 0 { len += 1; }
    }
    let value = if len < 256 {
        unsafe { String::from_utf16(std::slice::from_raw_parts(sid_text, len)) }
            .map_err(|_| "SANDBOX_USER_SID_INVALID".to_string())
    } else {
        Err("SANDBOX_USER_SID_INVALID".into())
    };
    unsafe { LocalFree(sid_text as _); }
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
        Ok(self)
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

    fn stage_runtime(&self, runtime: &Path) -> Result<PathBuf, String> {
        let canonical = runtime.canonicalize().map_err(|_| "SANDBOX_RUNTIME_MISSING")?;
        no_reparse_path(&canonical)?;
        let system32 = system_binary("cmd.exe")?
            .parent().ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?.to_path_buf();
        let system_root = system32.parent().ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
        let canonical_system_root = system_root.canonicalize()
            .map_err(|_| "SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
        if canonical.starts_with(&canonical_system_root) {
            return Ok(canonical);
        }
        const MAX_RUNTIME_FILE: u64 = 256 * 1024 * 1024;
        const MAX_TOTAL_RUNTIME: u64 = 384 * 1024 * 1024;
        const MAX_SIBLING_DLLS: usize = 32;

        fn copy_component(from: &Path, to: &Path, budget: &mut u64) -> Result<(), String> {
            no_reparse_path(from)?;
            let mut source = OpenOptions::new()
                .read(true).custom_flags(0x0020_0000)
                .open(from).map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
            let metadata = source.metadata().map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
            if !metadata.is_file()
                || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
                || metadata.len() > MAX_RUNTIME_FILE
            {
                return Err("SANDBOX_RUNTIME_UNSAFE".into());
            }
            *budget = budget.checked_add(metadata.len()).filter(|n| *n <= MAX_TOTAL_RUNTIME)
                .ok_or("SANDBOX_RUNTIME_SIZE_LIMIT")?;
            let mut output = OpenOptions::new().write(true).create_new(true)
                .open(to).map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
            let written = std::io::copy(&mut source, &mut output)
                .map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
            if written != metadata.len() {
                return Err("SANDBOX_RUNTIME_CHANGED_DURING_COPY".into());
            }
            output.sync_all().map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
            no_reparse_path(from)?;
            Ok(())
        }

        let directory = self.folder.join("runtime");
        std::fs::create_dir(&directory).map_err(|_| "SANDBOX_RUNTIME_STAGE_FAILED")?;
        let filename = canonical.file_name().ok_or("SANDBOX_RUNTIME_NOT_EXECUTABLE")?;
        let staged = directory.join(filename);
        let mut total = 0u64;
        copy_component(&canonical, &staged, &mut total)?;
        let mut dll_count = 0usize;
        let original_dir = canonical.parent().ok_or("SANDBOX_RUNTIME_NOT_EXECUTABLE")?;
        for entry in std::fs::read_dir(original_dir).map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")? {
            let entry = entry.map_err(|_| "SANDBOX_RUNTIME_READ_DENIED")?;
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str())
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
        Ok(staged)
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

    pub fn spawn(
        self,
        runtime: &Path,
        args: &[String],
        environment: &[(String, String)],
        current_directory: &Path,
    ) -> Result<SandboxChild, String> {
        no_reparse_path(runtime)?;
        no_reparse_path(current_directory)?;
        let canonical = runtime.canonicalize().map_err(|_| "SANDBOX_RUNTIME_MISSING")?;
        if !canonical.is_file()
            || runtime.extension().and_then(|value| value.to_str()).unwrap_or("")
                .to_ascii_lowercase() != "exe"
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
        let mut code = 0u32;
        if unsafe { GetExitCodeProcess(self.handle, &mut code) } == 0 {
            return Err(win32_error("SANDBOX_EXIT_CODE"));
        }
        Ok(if code == 259 { None } else { Some(code) })
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
fn environment_block(configured: &[(String, String)], state: &Path) -> Result<Vec<u16>, String> {
    let system32 = system_binary("cmd.exe")?
        .parent().ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?.to_path_buf();
    let windows_dir = system32.parent().ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?;
    let system_drive = windows_dir.components().next()
        .ok_or("SANDBOX_SYSTEM_DIRECTORY_UNAVAILABLE")?.as_os_str()
        .to_string_lossy().to_string();
    let mut values = BTreeMap::<String, String>::new();
    values.insert("SYSTEMROOT".into(), windows_dir.to_string_lossy().into_owned());
    values.insert("WINDIR".into(), windows_dir.to_string_lossy().into_owned());
    values.insert("SYSTEMDRIVE".into(), system_drive);
    values.insert("COMSPEC".into(), system_binary("cmd.exe")?.to_string_lossy().into_owned());
    values.insert("PATH".into(), system32.to_string_lossy().into_owned());
    values.insert("RUST_BACKTRACE".into(), "0".into());
    const RESERVED: &[&str] = &[
        "SYSTEMROOT", "SYSTEMDRIVE", "WINDIR", "COMSPEC", "PATH",
        "LOCALAPPDATA", "USERPROFILE", "HOME", "TEMP", "TMP", "APPDATA",
        "PATHEXT", "RUST_BACKTRACE",
    ];
    for (key, value) in configured {
        if key.is_empty() || key.len() > 128 || value.len() > 16384
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || value.contains('\0') || key.starts_with('=')
            || RESERVED.iter().any(|reserved| key.eq_ignore_ascii_case(reserved))
        {
            return Err("SANDBOX_ENV_INVALID".into());
        }
        let key = key.to_ascii_uppercase();
        if values.insert(key, value.clone()).is_some() {
            return Err("SANDBOX_ENV_DUPLICATE".into());
        }
    }
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|_| "SANDBOX_LOCAL_APP_DATA_UNAVAILABLE")?;
    if !Path::new(&local_app_data).is_absolute() || local_app_data.contains('\0') {
        return Err("SANDBOX_LOCAL_APP_DATA_INVALID".into());
    }
    values.insert("LOCALAPPDATA".into(), local_app_data);
    let state_text = state.to_str().ok_or("SANDBOX_STATE_NOT_UNICODE")?;
    for key in ["HOME", "USERPROFILE", "TEMP", "TMP"] {
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
                if remaining.is_zero() { return Err(()); }
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
    let pipe_security = appcontainer_pipe_security(&user_sid, &profile.sid_string)?;
    let (child_input, parent_input) = pipe(pipe_security)?;
    let (parent_output, child_output) = pipe(pipe_security)?;
    unsafe { LocalFree(pipe_security) };
    inheritable(parent_output.as_raw_handle(), false)?;
    inheritable(parent_input.as_raw_handle(), false)?;
    inheritable(child_input.as_raw_handle(), true)?;
    inheritable(child_output.as_raw_handle(), true)?;
    let null = OpenOptions::new().write(true).open("NUL")
        .map_err(|_| "SANDBOX_NULL_STDERR")?;
    inheritable(null.as_raw_handle(), true)?;

    let mut size = 0usize;
    unsafe {
        let _ = InitializeProcThreadAttributeList(ptr::null_mut(), 2, 0, &mut size);
    }
    if size == 0 {
        return Err("SANDBOX_ATTRIBUTE_SIZE".into());
    }
    let mut attribute_storage = vec![0u8; size];
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
        null.as_raw_handle() as HANDLE,
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
        cleanup_attributes(attribute_list);
        return Err(win32_error("SANDBOX_ATTRIBUTE_UPDATE"));
    }

    let runtime_text = runtime.to_str().ok_or("SANDBOX_RUNTIME_NOT_UNICODE")?;
    let mut command = quote_argument(runtime_text)?;
    for argument in args {
        command.pop();
        command.push(' ' as u16);
        command.extend(quote_argument(argument)?);
    }
    if command.last() != Some(&0) {
        command.push(0);
    }
    let application = wide(runtime.as_os_str())?;
    let directory = wide(current_directory.as_os_str())?;
    let environment = environment_block(environment, current_directory)?;
    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = child_input.as_raw_handle() as HANDLE;
    startup.StartupInfo.hStdOutput = child_output.as_raw_handle() as HANDLE;
    startup.StartupInfo.hStdError = null.as_raw_handle() as HANDLE;
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
    cleanup_attributes(attribute_list);
    drop(child_input);
    drop(child_output);
    drop(null);
    if created == 0 {
        return Err(win32_error("SANDBOX_CREATE_PROCESS"));
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
    use std::time::Duration;

    #[test]
    fn appcontainer_runs_without_host_workspace_or_loopback_access() {
        let runtime = system_binary("WindowsPowerShell\\v1.0\\powershell.exe")
            .expect("Windows PowerShell fixture");
        let host_file = std::env::temp_dir().join(format!(
            "browseros-sandbox-host-{}.txt", uuid::Uuid::new_v4()
        ));
        std::fs::write(&host_file, "host-secret-marker").unwrap();
        let profile = SandboxProfile::create().expect("create AppContainer profile");
        let state = profile.state_dir().expect("state");
        let script = "$ErrorActionPreference='Stop'; try { $c=New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1',9); 'NETWORK_ALLOWED' } catch { 'NETWORK_DENIED' }; try { Get-Content -LiteralPath $env:BROWSEROS_TEST_HOST_FILE -ErrorAction Stop | Out-Null; 'HOST_READ_ALLOWED' } catch { 'HOST_READ_DENIED' }";
        let arguments = vec![
            "-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), script.into(),
        ];
        let environment = vec![(
            "BROWSEROS_TEST_HOST_FILE".into(), host_file.to_string_lossy().into_owned(),
        )];
        let mut child = profile
            .spawn(&runtime, &arguments, &environment, &state)
            .expect("spawn AppContainer");
        drop(child.stdin);
        let mut output = Vec::new();
        child.stdout.read_to_end(&mut output).unwrap();
        let text = String::from_utf8_lossy(&output);
        assert!(
            text.contains("NETWORK_DENIED"),
            "exit={:?} text={text}", child.process.exit_code()
        );
        assert!(text.contains("HOST_READ_DENIED"), "{text}");
        let _ = std::fs::remove_file(host_file);
    }

    #[test]
    fn quoted_arguments_survive_process_creation() {
        let profile = SandboxProfile::create().unwrap();
        let state = profile.state_dir().unwrap();
        let cmd = system_binary("cmd.exe").unwrap();
        let mut child = profile
            .spawn(
                &cmd,
                &["/D".into(), "/S".into(), "/C".into(), "echo sandbox-ok".into()],
                &[], &state,
            )
            .unwrap();
        drop(child.stdin);
        let mut output = String::new();
        child.stdout.read_to_string(&mut output).unwrap();
        assert!(
            output.contains("sandbox-ok"),
            "exit={:?} output={output:?}", child.process.exit_code()
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !child.process.try_wait().unwrap() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
