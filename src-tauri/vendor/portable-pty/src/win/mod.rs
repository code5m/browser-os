use crate::{Child, ChildKiller, ExitStatus};
use anyhow::Context as _;
use std::io::{Error as IoError, Result as IoResult};
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use winapi::shared::minwindef::DWORD;
// WAIT_OBJECT_0 is authoritative; exit code 259 may be a valid exit code.
use winapi::um::processthreadsapi::*;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::INFINITE;

const WAIT_OBJECT_0_RESULT: DWORD = 0;
const WAIT_TIMEOUT_RESULT: DWORD = 0x00000102;
const WAIT_FAILED_RESULT: DWORD = u32::MAX;

fn process_status(proc: &OwnedHandle) -> IoResult<Option<ExitStatus>> {
    let handle = proc.as_raw_handle() as _;
    match unsafe { WaitForSingleObject(handle, 0) } {
        WAIT_OBJECT_0_RESULT => {
            let mut code: DWORD = 0;
            if unsafe { GetExitCodeProcess(handle, &mut code) } == 0 {
                Err(IoError::last_os_error())
            } else {
                Ok(Some(ExitStatus::with_exit_code(code)))
            }
        }
        WAIT_TIMEOUT_RESULT => Ok(None),
        WAIT_FAILED_RESULT => Err(IoError::last_os_error()),
        other => Err(IoError::other(format!("unexpected process wait result: {other}"))),
    }
}

fn terminate_handle(proc: &OwnedHandle) -> IoResult<()> {
    if process_status(proc)?.is_some() {
        return Ok(()); // Already exited.
    }
    if unsafe { TerminateProcess(proc.as_raw_handle() as _, 1) } != 0 {
        return Ok(());
    }
    let error = IoError::last_os_error();
    // A process may exit between the nonblocking wait and termination.
    if matches!(process_status(proc), Ok(Some(_))) {
        Ok(())
    } else {
        Err(error)
    }
}

pub mod conpty;
mod procthreadattr;
mod psuedocon;

use filedescriptor::OwnedHandle;

#[derive(Debug)]
pub struct WinChild {
    proc: Mutex<OwnedHandle>,
}

impl WinChild {
    fn is_complete(&mut self) -> IoResult<Option<ExitStatus>> {
        let proc = self.proc.lock().unwrap();
        process_status(&proc)
    }

    fn do_kill(&mut self) -> IoResult<()> {
        let proc = self.proc.lock().unwrap();
        terminate_handle(&proc)
    }
}

impl ChildKiller for WinChild {
    fn kill(&mut self) -> IoResult<()> {
        self.do_kill()
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        let proc = self.proc.lock().unwrap().try_clone().unwrap();
        Box::new(WinChildKiller { proc })
    }
}

#[derive(Debug)]
pub struct WinChildKiller {
    proc: OwnedHandle,
}

impl ChildKiller for WinChildKiller {
    fn kill(&mut self) -> IoResult<()> {
        terminate_handle(&self.proc)
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        let proc = self.proc.try_clone().unwrap();
        Box::new(WinChildKiller { proc })
    }
}

impl Child for WinChild {
    fn try_wait(&mut self) -> IoResult<Option<ExitStatus>> {
        self.is_complete()
    }

    fn wait(&mut self) -> IoResult<ExitStatus> {
        if let Some(status) = self.try_wait()? {
            return Ok(status);
        }
        let proc = self.proc.lock().unwrap();
        match unsafe { WaitForSingleObject(proc.as_raw_handle() as _, INFINITE) } {
            WAIT_OBJECT_0_RESULT => process_status(&proc)?
                .ok_or_else(|| IoError::other("signaled process has no exit status")),
            WAIT_FAILED_RESULT => Err(IoError::last_os_error()),
            other => Err(IoError::other(format!("unexpected wait result: {other}"))),
        }
    }

    fn process_id(&self) -> Option<u32> {
        let res = unsafe { GetProcessId(self.proc.lock().unwrap().as_raw_handle() as _) };
        if res == 0 {
            None
        } else {
            Some(res)
        }
    }

    fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
        let proc = self.proc.lock().unwrap();
        Some(proc.as_raw_handle())
    }
}

impl std::future::Future for WinChild {
    type Output = anyhow::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<anyhow::Result<ExitStatus>> {
        match self.is_complete() {
            Ok(Some(status)) => Poll::Ready(Ok(status)),
            Err(err) => Poll::Ready(Err(err).context("Failed to retrieve process exit status")),
            Ok(None) => {
                struct PassRawHandleToWaiterThread(pub RawHandle);
                unsafe impl Send for PassRawHandleToWaiterThread {}

                let proc = self.proc.lock().unwrap().try_clone()?;
                let handle = PassRawHandleToWaiterThread(proc.as_raw_handle());

                let waker = cx.waker().clone();
                std::thread::spawn(move || {
                    unsafe {
                        WaitForSingleObject(handle.0 as _, INFINITE);
                    }
                    waker.wake();
                });
                Poll::Pending
            }
        }
    }
}
