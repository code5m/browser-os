//! Windows Grid IPC, using an owner-only ACL and local-only named pipes.
//! A second authorization check compares the kernel-reported client PID to
//! the actual Job-owned Grid child. Per-connection generation checks happen
//! in the Grid manager. No TCP listener or CDP debug port is created.
#![cfg(windows)]
use std::ffi::c_void;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{GetLastError, LocalFree, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::CreateFileW;
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId,
};

// Win32 named pipe flags, supplied explicitly for one private byte-mode pipe.
const PIPE_ACCESS_DUPLEX: u32 = 3;
const PIPE_TYPE_BYTE: u32 = 0;
const PIPE_READMODE_BYTE: u32 = 0;
const PIPE_WAIT: u32 = 0;
const PIPE_REJECT_REMOTE_CLIENTS: u32 = 8;
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
const FILE_GENERIC_RW: u32 = 0xC000_0000;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const ERROR_PIPE_CONNECTED: u32 = 535;

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }
fn validate_name(path: &Path) -> io::Result<String> {
    let name = path.to_string_lossy().into_owned();
    if !name.starts_with(r"\\.\pipe\BrowserOS-grid-") || name.len() > 240 ||
        name.encode_utf16().any(|c| c == 0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "GRID_PIPE_INVALID_NAME"));
    }
    Ok(name)
}

/// Owner-rights/SYSTEM only. Default Win32 named-pipe DACL grants Everyone
/// read access, therefore passing NULL security attributes would be unsafe.
fn pipe_security() -> io::Result<(SECURITY_ATTRIBUTES, *mut c_void)> {
    let sddl = wide("D:P(A;;GA;;;OW)(A;;GA;;;SY)");
    let mut descriptor: *mut c_void = ptr::null_mut();
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(), 1, &mut descriptor, ptr::null_mut()
        )
    };
    if ok == 0 { return Err(io::Error::last_os_error()); }
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    Ok((attrs, descriptor))
}

fn create_instance(name: &[u16], first: bool) -> io::Result<File> {
    let (attrs, descriptor) = pipe_security()?;
    let first_flag = if first { FILE_FLAG_FIRST_PIPE_INSTANCE } else { 0 };
    let raw = unsafe {
        CreateNamedPipeW(name.as_ptr(), PIPE_ACCESS_DUPLEX | first_flag,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            255, 65536, 65536, 3000, &attrs)
    };
    unsafe { LocalFree(descriptor as *mut c_void); }
    if raw == INVALID_HANDLE_VALUE { return Err(io::Error::last_os_error()); }
    Ok(unsafe { File::from_raw_handle(raw as RawHandle) })
}

pub struct WindowsPipeListener {
    name: Vec<u16>,
    initial: Mutex<Option<File>>,
}
impl WindowsPipeListener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        // Bind synchronously before spawning; FIRST_PIPE_INSTANCE detects
        // name squatting, owner ACL rejects other users and remote clients.
        let name = wide(&validate_name(path)?);
        let initial = create_instance(&name, true)?;
        Ok(Self { name, initial: Mutex::new(Some(initial)) })
    }
    pub fn accept(&self) -> io::Result<(WindowsPipeStream, ())> {
        // The existing reader may still have its pipe instance open.
        // Subsequent instances therefore MUST omit FIRST_PIPE_INSTANCE.
        let file = match self.initial.lock().unwrap().take() {
            Some(file) => file,
            None => create_instance(&self.name, false)?,
        };
        let raw = file.as_raw_handle() as _;
        let connected = unsafe { ConnectNamedPipe(raw, ptr::null_mut()) };
        if connected == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
            return Err(io::Error::last_os_error());
        }
        Ok((WindowsPipeStream(file), ()))
    }
}

pub struct WindowsPipeStream(File);
impl WindowsPipeStream {
    pub fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        let name = wide(&validate_name(path.as_ref())?);
        let raw = unsafe { CreateFileW(
            name.as_ptr(), FILE_GENERIC_RW, 0,
            ptr::null(), OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL,
            ptr::null_mut(),
        )};
        if raw == INVALID_HANDLE_VALUE { return Err(io::Error::last_os_error()); }
        Ok(Self(unsafe { File::from_raw_handle(raw as RawHandle) }))
    }
    pub fn try_clone(&self) -> io::Result<Self> { self.0.try_clone().map(Self) }
    pub fn peer_pid(&self) -> io::Result<u32> {
        let mut pid = 0u32;
        if unsafe { GetNamedPipeClientProcessId(self.0.as_raw_handle() as _, &mut pid) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(pid)
    }
}
impl Read for WindowsPipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.0.read(buf) }
}
impl Write for WindowsPipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.write(buf) }
    fn flush(&mut self) -> io::Result<()> { self.0.flush() }
}
