use super::WinChild;
use crate::cmdbuilder::CommandBuilder;
use crate::win::procthreadattr::ProcThreadAttributeList;
use anyhow::{bail, ensure, Error};
use filedescriptor::{FileDescriptor, OwnedHandle};
use lazy_static::lazy_static;
use shared_library::shared_library;
use std::ffi::OsString;
use std::io::Error as IoError;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::Path;
use std::sync::Mutex;
use std::{mem, ptr};
use winapi::shared::minwindef::DWORD;
use winapi::shared::winerror::{HRESULT, S_OK};
use winapi::um::handleapi::*;
use winapi::um::processthreadsapi::*;
use winapi::um::winbase::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
    STARTF_USESTDHANDLES, STARTUPINFOEXW,
};
use winapi::um::wincon::COORD;
use winapi::um::winnt::HANDLE;

pub type HPCON = HANDLE;

pub const PSUEDOCONSOLE_INHERIT_CURSOR: DWORD = 0x1;
pub const PSEUDOCONSOLE_RESIZE_QUIRK: DWORD = 0x2;
pub const PSEUDOCONSOLE_WIN32_INPUT_MODE: DWORD = 0x4;
#[allow(dead_code)]
pub const PSEUDOCONSOLE_PASSTHROUGH_MODE: DWORD = 0x8;

shared_library!(ConPtyFuncs,
    pub fn CreatePseudoConsole(
        size: COORD,
        hInput: HANDLE,
        hOutput: HANDLE,
        flags: DWORD,
        hpc: *mut HPCON
    ) -> HRESULT,
    pub fn ResizePseudoConsole(hpc: HPCON, size: COORD) -> HRESULT,
    pub fn ClosePseudoConsole(hpc: HPCON),
);

fn load_conpty() -> ConPtyFuncs {
    // If the kernel doesn't export these functions then their system is
    // too old and we cannot run.
    let kernel = ConPtyFuncs::open(Path::new("kernel32.dll")).expect(
        "this system does not support conpty.  Windows 10 October 2018 or newer is required",
    );

    // Never resolve conpty.dll through the process current directory or
    // workspace-controlled PATH. A relative DLL name may load attacker
    // bytes when the integrated terminal is opened inside a repository.
    // A bundled replacement is valid only beside the current executable.
    let bundled = std::env::current_exe().ok()
        .and_then(|executable| executable.parent().map(|parent| parent.join("conpty.dll")))
        .filter(|candidate| std::fs::symlink_metadata(candidate)
            .is_ok_and(|meta| meta.file_type().is_file()));
    if let Some(path) = bundled {
        if let Ok(sideloaded) = ConPtyFuncs::open(&path) {
            return sideloaded;
        }
    }
    kernel
}

lazy_static! {
    static ref CONPTY: ConPtyFuncs = load_conpty();
}

pub struct PsuedoCon {
    con: HPCON,
    // The host ends must remain open until the ConPTY client has been
    // successfully created. Closing them in new() loses the I/O endpoints
    // before CreateProcessW attaches the first client to the pseudo console.
    creation_pipes: Mutex<Option<(FileDescriptor, FileDescriptor)>>,
}

unsafe impl Send for PsuedoCon {}
unsafe impl Sync for PsuedoCon {}

impl Drop for PsuedoCon {
    fn drop(&mut self) {
        unsafe { (CONPTY.ClosePseudoConsole)(self.con) };
    }
}

fn is_live_pipe_handle(handle: HANDLE) -> bool {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return false;
    }
    let mut flags: DWORD = 0;
    unsafe { GetHandleInformation(handle, &mut flags) != 0 }
}

impl PsuedoCon {
    pub fn new(size: COORD, input: FileDescriptor, output: FileDescriptor) -> Result<Self, Error> {
        ensure!(size.X > 0 && size.Y > 0, "ConPTY dimensions must be positive");
        // The pipes belong to this process, regardless of whether the parent
        // shell/GUI exposes valid STD_INPUT_HANDLE/STD_OUTPUT_HANDLE values.
        let input_handle = input.as_raw_handle() as HANDLE;
        let output_handle = output.as_raw_handle() as HANDLE;
        ensure!(is_live_pipe_handle(input_handle),
            "ConPTY input pipe handle is invalid before CreatePseudoConsole");
        ensure!(is_live_pipe_handle(output_handle),
            "ConPTY output pipe handle is invalid before CreatePseudoConsole");
        let mut con: HPCON = INVALID_HANDLE_VALUE;
        let result = unsafe {
            (CONPTY.CreatePseudoConsole)(
                size,
                input_handle,
                output_handle,
                // Cursor inheritance requires an asynchronous handshake;
                // ordinary pseudo consoles do not use that flag.
                0,
                &mut con,
            )
        };
        ensure!(
            result == S_OK,
            "failed to create psuedo console: HRESULT {}; input_pipe_live={}; output_pipe_live={}; size={}x{}",
            result,
            is_live_pipe_handle(input_handle),
            is_live_pipe_handle(output_handle),
            size.X,
            size.Y
        );
        Ok(Self {
            con,
            creation_pipes: Mutex::new(Some((input, output))),
        })
    }

    pub fn resize(&self, size: COORD) -> Result<(), Error> {
        let result = unsafe { (CONPTY.ResizePseudoConsole)(self.con, size) };
        ensure!(
            result == S_OK,
            "failed to resize console to {}x{}: HRESULT: {}",
            size.X,
            size.Y,
            result
        );
        Ok(())
    }

    pub fn spawn_command(&self, cmd: CommandBuilder) -> anyhow::Result<WinChild> {
        self.spawn_command_with_options(cmd, 0, None)
    }

    /// Attach this ConPTY child atomically to a pre-created Job during
    /// CreateProcessW. No post-spawn adoption, suspension, or ResumeThread.
    pub fn spawn_command_in_job(&self, cmd: CommandBuilder, job: HANDLE) -> anyhow::Result<WinChild> {
        ensure!(!job.is_null(), "ConPTY requires a valid Job Object");
        self.spawn_command_with_options(cmd, 0, Some(job))
    }

    /// CreateProcessW returns before the child can execute user code. The
    /// caller must assign the process to its Job Object before resuming it.
    pub fn spawn_command_suspended(&self, cmd: CommandBuilder) -> anyhow::Result<WinChild> {
        self.spawn_command_with_options(cmd, CREATE_SUSPENDED, None)
    }

    fn spawn_command_with_options(&self, cmd: CommandBuilder, extra_flags: u32, job: Option<HANDLE>) -> anyhow::Result<WinChild> {
        let mut si: STARTUPINFOEXW = unsafe { mem::zeroed() };
        si.StartupInfo.cb = mem::size_of::<STARTUPINFOEXW>() as u32;
        // Explicitly set the stdio handles as invalid handles otherwise
        // we can end up with a weird state where the spawned process can
        // inherit the explicitly redirected output handles from its parent.
        // For example, when daemonizing wezterm-mux-server, the stdio handles
        // are redirected to a log file and the spawned process would end up
        // writing its output there instead of to the pty we just created.
        si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        si.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
        si.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
        si.StartupInfo.hStdError = INVALID_HANDLE_VALUE;

        let mut attrs = ProcThreadAttributeList::with_capacity(if job.is_some() { 2 } else { 1 })?;
        attrs.set_pty(self.con)?;
        // PROC_THREAD_ATTRIBUTE_JOB_LIST requires an array of HANDLEs.
        // Keep its storage alive until CreateProcessW returns.
        let mut job_handles = [job.unwrap_or(ptr::null_mut())];
        if job.is_some() {
            attrs.set_job_list(&mut job_handles)?;
        }
        si.lpAttributeList = attrs.as_mut_ptr();

        let mut pi: PROCESS_INFORMATION = unsafe { mem::zeroed() };

        let (mut exe, mut cmdline) = cmd.cmdline()?;
        let cmd_os = OsString::from_wide(&cmdline);

        let cwd = cmd.current_directory();

        let res = unsafe {
            CreateProcessW(
                exe.as_mut_slice().as_mut_ptr(),
                cmdline.as_mut_slice().as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | extra_flags,
                cmd.environment_block().as_mut_slice().as_mut_ptr() as *mut _,
                cwd.as_ref()
                    .map(|c| c.as_slice().as_ptr())
                    .unwrap_or(ptr::null()),
                &mut si.StartupInfo,
                &mut pi,
            )
        };
        if res == 0 {
            let err = IoError::last_os_error();
            let msg = format!(
                "CreateProcessW `{:?}` in cwd `{:?}` failed: {}",
                cmd_os,
                cwd.as_ref().map(|c| OsString::from_wide(c)),
                err
            );
            log::error!("{}", msg);
            bail!("{}", msg);
        }

        // Microsoft documents releasing the host ends supplied to
        // CreatePseudoConsole after the client CreateProcess completes,
        // not immediately when CreatePseudoConsole returns.
        let _ = self.creation_pipes
            .lock().unwrap_or_else(|poison| poison.into_inner()).take();

        // Make sure we close out the thread handle so we don't leak it;
        // we do this simply by making it owned
        let _main_thread = unsafe { OwnedHandle::from_raw_handle(pi.hThread as _) };
        let proc = unsafe { OwnedHandle::from_raw_handle(pi.hProcess as _) };

        Ok(WinChild {
            proc: Mutex::new(proc),
        })
    }
}
