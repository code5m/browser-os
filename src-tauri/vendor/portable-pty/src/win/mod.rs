use crate::{Child, ChildKiller, ExitStatus};
use anyhow::Context as _;
use std::io::{Error as IoError, Result as IoResult};
use std::os::windows::io::AsRawHandle;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Waker};
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

#[derive(Debug, Default)]
struct ProcessWaitState {
    started: bool,
    finished: bool,
    error: Option<IoError>,
    waker: Option<Waker>,
}

/// Own the duplicated handle for the full native wait. A raw HANDLE alone
/// does not keep the kernel object alive when the polling stack unwinds.
struct ProcessWaitHandle(OwnedHandle);
unsafe impl Send for ProcessWaitHandle {}

impl ProcessWaitHandle {
    fn run(self, state: Weak<Mutex<ProcessWaitState>>) {
        loop {
            // A dropped Future must not leave a waiter blocked forever.
            // The only strong owner is WinChild (plus short polling borrows).
            if state.strong_count() == 0 {
                return;
            }
            let result = unsafe { WaitForSingleObject(self.0.as_raw_handle() as _, 250) };
            if result == WAIT_TIMEOUT_RESULT {
                continue;
            }
            let error = match result {
                WAIT_OBJECT_0_RESULT => None,
                WAIT_FAILED_RESULT => Some(IoError::last_os_error()),
                other => Some(IoError::other(format!("unexpected process wait result: {other}"))),
            };
            if let Some(state) = state.upgrade() {
                let waker = {
                    let mut state = state.lock().unwrap_or_else(|poison| poison.into_inner());
                    state.finished = true;
                    state.error = error;
                    state.waker.take()
                };
                if let Some(waker) = waker {
                    waker.wake();
                }
            }
            return;
        }
    }
}

#[derive(Debug)]
pub struct WinChild {
    proc: Mutex<OwnedHandle>,
    wait_state: Arc<Mutex<ProcessWaitState>>,
}

impl WinChild {
    pub(super) fn new(proc: OwnedHandle) -> Self {
        Self {
            proc: Mutex::new(proc),
            wait_state: Arc::new(Mutex::new(ProcessWaitState::default())),
        }
    }

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
            Err(error) => Poll::Ready(Err(error).context("Failed to retrieve process exit status")),
            Ok(None) => {
                let mut state = self.wait_state.lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                if let Some(error) = state.error.take() {
                    return Poll::Ready(Err(error).context("Failed to wait for process exit"));
                }
                // The waiter may have completed after the status probe at
                // poll entry and consumed an older waker. Observe completion
                // under the same lock instead of parking the replacement task.
                if state.finished {
                    drop(state);
                    return match self.is_complete() {
                        Ok(Some(status)) => Poll::Ready(Ok(status)),
                        Ok(None) => Poll::Ready(Err(anyhow::anyhow!(
                            "completed native wait has no process exit status"
                        ))),
                        Err(error) => Poll::Ready(Err(error.into())),
                    };
                }
                state.waker = Some(cx.waker().clone());
                if !state.started {
                    let proc = match self.proc.lock()
                        .unwrap_or_else(|poison| poison.into_inner()).try_clone()
                    {
                        Ok(proc) => proc,
                        Err(error) => return Poll::Ready(Err(error.into())),
                    };
                    state.started = true;
                    let owner = ProcessWaitHandle(proc);
                    let waiter_state = Arc::downgrade(&self.wait_state);
                    if let Err(error) = std::thread::Builder::new()
                        .name("portable-pty-process-wait".into())
                        .spawn(move || owner.run(waiter_state))
                    {
                        state.started = false;
                        return Poll::Ready(Err(error.into()));
                    }
                }
                // Repeated polls replace the waker, not the waiting thread.
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod process_wait_tests {
    use super::*;
    use std::future::Future;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;
    use std::time::{Duration, Instant};
    use winapi::um::handleapi::DuplicateHandle;
    use winapi::um::winnt::DUPLICATE_SAME_ACCESS;

    struct WakeCount(AtomicUsize);
    impl Wake for WakeCount {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn future_wait_owns_process_handle_across_repeated_polls() {
        let mut process = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "set /p gate= >NUL & exit /B 37"])
            .stdin(std::process::Stdio::piped())
            .spawn().expect("start native wait fixture");
        let mut gate = process.stdin.take().expect("controlled fixture stdin");
        let mut duplicated = std::ptr::null_mut();
        let copied = unsafe {
            DuplicateHandle(
                GetCurrentProcess(), process.as_raw_handle() as _,
                GetCurrentProcess(), &mut duplicated, 0, 0, DUPLICATE_SAME_ACCESS,
            )
        };
        assert_ne!(copied, 0, "duplicate native process handle");
        let owned = unsafe {
            <OwnedHandle as std::os::windows::io::FromRawHandle>::from_raw_handle(duplicated as _)
        };
        let mut child = WinChild::new(owned);
        let initial_wakes = Arc::new(WakeCount(AtomicUsize::new(0)));
        let initial_waker = Waker::from(Arc::clone(&initial_wakes));
        let mut initial_context = Context::from_waker(&initial_waker);
        assert!(matches!(Pin::new(&mut child).poll(&mut initial_context), Poll::Pending));
        let wake_count = Arc::new(WakeCount(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&wake_count));
        let mut context = Context::from_waker(&waker);
        // Re-poll before the long-lived fixture exits; these must share one
        // owned native waiter, not start one detached thread per poll.
        for _ in 0..4 {
            assert!(matches!(Pin::new(&mut child).poll(&mut context), Poll::Pending));
        }
        gate.write_all(b"continue\r\n").expect("release native fixture");
        drop(gate);
        let until = Instant::now() + Duration::from_secs(10);
        while wake_count.0.load(Ordering::SeqCst) == 0 {
            if Instant::now() >= until {
                let _ = process.kill();
                let _ = process.wait();
                panic!("owned native waiter did not wake the task");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(initial_wakes.0.load(Ordering::SeqCst), 0,
            "completion must notify the replacement waker");
        match Pin::new(&mut child).poll(&mut context) {
            Poll::Ready(result) =>
                assert_eq!(result.expect("owned wait handle remains valid").exit_code(), 37),
            Poll::Pending => {
                let _ = process.kill();
                let _ = process.wait();
                panic!("native waiter woke before process completion");
            }
        }
        process.wait().expect("reap native fixture");
    }
}
