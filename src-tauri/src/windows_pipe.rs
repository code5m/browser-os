//! Windows Grid IPC, using an owner-only ACL and local-only named pipes.
//! A second authorization check compares the kernel-reported client PID to
//! the actual Job-owned Grid child. Per-connection generation checks happen
//! in the Grid manager. No TCP listener or CDP debug port is created.
//!
//! All connected pipe handles use overlapped I/O. Reads wait for the child,
//! writes have a hard deadline, and ConnectNamedPipe has its own timeout.
//! This prevents a hung Grid child from pinning the UI writer lock forever.
#![cfg(windows)]
use std::ffi::c_void;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::io::{FromRawHandle, IntoRawHandle, RawHandle};
use std::path::Path;
use std::ptr;
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    ERROR_BROKEN_PIPE, ERROR_IO_PENDING, ERROR_OPERATION_ABORTED,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, ReadFile, WriteFile};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, DuplicateHandle, GetCurrentProcess, WaitForSingleObject,
};

const PIPE_ACCESS_DUPLEX: u32 = 3;
const PIPE_TYPE_BYTE: u32 = 0;
const PIPE_READMODE_BYTE: u32 = 0;
const PIPE_WAIT: u32 = 0;
const PIPE_REJECT_REMOTE_CLIENTS: u32 = 8;
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
const FILE_GENERIC_RW: u32 = 0xC000_0000;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const ERROR_PIPE_CONNECTED: u32 = 535;
const DUPLICATE_SAME_ACCESS: u32 = 0x0000_0002;
const INFINITE: u32 = u32::MAX;
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

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

struct Event(HANDLE);
impl Event {
    fn new() -> io::Result<Self> {
        let handle = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
        if handle.is_null() { return Err(io::Error::last_os_error()); }
        Ok(Self(handle))
    }
}
impl Drop for Event {
    fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
}

fn overlapped_wait(
    handle: HANDLE,
    event: HANDLE,
    overlapped: &mut OVERLAPPED,
    timeout_ms: u32,
) -> io::Result<u32> {
    let wait = unsafe { WaitForSingleObject(event, timeout_ms) };
    if wait == WAIT_TIMEOUT {
        unsafe { CancelIoEx(handle, overlapped); }
        // Do not return while the kernel may still reference the caller's
        // buffer/OVERLAPPED memory.
        let _ = unsafe { WaitForSingleObject(event, INFINITE) };
        let mut transferred = 0u32;
        if unsafe { GetOverlappedResult(handle, overlapped, &mut transferred, 0) } != 0 {
            return Ok(transferred);
        }
        let error = unsafe { GetLastError() };
        if error == ERROR_OPERATION_ABORTED {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "GRID_PIPE_IO_TIMEOUT"));
        }
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    if wait != WAIT_OBJECT_0 {
        let error = io::Error::last_os_error();
        unsafe { CancelIoEx(handle, overlapped); }
        let _ = unsafe { WaitForSingleObject(event, INFINITE) };
        return Err(error);
    }
    let mut transferred = 0u32;
    if unsafe { GetOverlappedResult(handle, overlapped, &mut transferred, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(transferred)
}

fn create_instance(name: &[u16], first: bool) -> io::Result<File> {
    let (attrs, descriptor) = pipe_security()?;
    let first_flag = if first { FILE_FLAG_FIRST_PIPE_INSTANCE } else { 0 };
    let raw = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | first_flag,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            255, 65536, 65536, 3000, &attrs,
        )
    };
    unsafe { CloseHandle(descriptor as HANDLE); }
    if raw == INVALID_HANDLE_VALUE { return Err(io::Error::last_os_error()); }
    Ok(unsafe { File::from_raw_handle(raw as RawHandle) })
}

pub struct WindowsPipeListener {
    name: Vec<u16>,
    initial: std::sync::Mutex<Option<File>>,
}
impl WindowsPipeListener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        // Bind synchronously before spawning; FIRST_PIPE_INSTANCE detects
        // name squatting, owner ACL rejects other users and remote clients.
        let name = wide(&validate_name(path)?);
        let initial = create_instance(&name, true)?;
        Ok(Self { name, initial: std::sync::Mutex::new(Some(initial)) })
    }

    pub fn accept(&self) -> io::Result<(WindowsPipeStream, ())> {
        let file = match self.initial.lock().unwrap().take() {
            Some(file) => file,
            None => create_instance(&self.name, false)?,
        };
        let raw = file.as_raw_handle() as HANDLE;
        let event = Event::new()?;
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        overlapped.hEvent = event.0;
        let connected = unsafe { ConnectNamedPipe(raw, &mut overlapped) };
        if connected == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_PIPE_CONNECTED {
                // The client connected between CreateNamedPipe and ConnectNamedPipe.
            } else if error == ERROR_IO_PENDING {
                overlapped_wait(
                    raw,
                    event.0,
                    &mut overlapped,
                    ACCEPT_TIMEOUT.as_millis() as u32,
                )?;
            } else {
                return Err(io::Error::from_raw_os_error(error as i32));
            }
        }
        drop(event);
        let handle = file.into_raw_handle() as HANDLE;
        Ok((WindowsPipeStream(handle), ()))
    }
}

pub struct WindowsPipeStream(HANDLE);
unsafe impl Send for WindowsPipeStream {}

impl WindowsPipeStream {
    fn from_raw(handle: HANDLE) -> Self { Self(handle) }

    pub fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        let name = wide(&validate_name(path.as_ref())?);
        let raw = unsafe {
            CreateFileW(
                name.as_ptr(), FILE_GENERIC_RW, 0,
                ptr::null(), OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
                ptr::null_mut(),
            )
        };
        if raw == INVALID_HANDLE_VALUE { return Err(io::Error::last_os_error()); }
        Ok(Self::from_raw(raw))
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        let mut duplicate: HANDLE = ptr::null_mut();
        let current = unsafe { GetCurrentProcess() };
        if unsafe {
            DuplicateHandle(
                current, self.0, current, &mut duplicate,
                0, 0, DUPLICATE_SAME_ACCESS,
            )
        } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(duplicate))
    }

    pub fn peer_pid(&self) -> io::Result<u32> {
        let mut pid = 0u32;
        if unsafe { GetNamedPipeClientProcessId(self.0, &mut pid) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(pid)
    }
}

impl Drop for WindowsPipeStream {
    fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
}

impl Read for WindowsPipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() { return Ok(0); }
        let event = Event::new()?;
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        overlapped.hEvent = event.0;
        let mut transferred = 0u32;
        let ok = unsafe {
            ReadFile(
                self.0, buf.as_mut_ptr(), buf.len().min(u32::MAX as usize) as u32,
                &mut transferred, &mut overlapped,
            )
        };
        if ok != 0 { return Ok(transferred as usize); }
        let error = unsafe { GetLastError() };
        if error == ERROR_BROKEN_PIPE { return Ok(0); }
        if error != ERROR_IO_PENDING {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        match overlapped_wait(self.0, event.0, &mut overlapped, INFINITE)? {
            count => Ok(count as usize),
        }
    }
}

impl Write for WindowsPipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() { return Ok(0); }
        let event = Event::new()?;
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        overlapped.hEvent = event.0;
        let mut transferred = 0u32;
        let ok = unsafe {
            WriteFile(
                self.0, buf.as_ptr(), buf.len().min(u32::MAX as usize) as u32,
                &mut transferred, &mut overlapped,
            )
        };
        if ok != 0 { return Ok(transferred as usize); }
        let error = unsafe { GetLastError() };
        if error != ERROR_IO_PENDING {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        overlapped_wait(
            self.0,
            event.0,
            &mut overlapped,
            WRITE_TIMEOUT.as_millis() as u32,
        ).map(|count| count as usize)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid_ipc::{read_wire, write_wire, GridCmd, Wire};
    #[test]
    fn local_owner_pipe_roundtrip_validates_actual_client_pid() {
        let name = format!(
            r"\\.\pipe\BrowserOS-grid-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple(),
        );
        let path = std::path::PathBuf::from(name);
        let listener = WindowsPipeListener::bind(&path).expect("owner-private pipe");
        let server = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().expect("client connection");
            assert_eq!(connection.peer_pid().expect("peer PID"), std::process::id());
            let mut reader = std::io::BufReader::new(connection.try_clone().expect("reader"));
            let received = read_wire(&mut reader).expect("read").expect("frame");
            assert!(matches!(received, Wire::Request { seq: 7, cmd: GridCmd::Ping }));
            write_wire(&mut connection, &Wire::ok(7)).expect("write response");
        });
        let mut client = WindowsPipeStream::connect(&path).expect("client");
        write_wire(&mut client, &Wire::Request { seq: 7, cmd: GridCmd::Ping })
            .expect("write request");
        let mut reader = std::io::BufReader::new(client);
        assert!(matches!(read_wire(&mut reader).unwrap(), Some(Wire::Response {seq:7,ok:true,..})));
        server.join().expect("server thread");
    }
}
