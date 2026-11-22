//! Windows process tree ownership. Job Objects kill descendants on close;
//! unlike direct Child::kill(), this also covers scripts spawned by ConPTY.
//! This is process lifecycle containment, NOT a security sandbox or AppContainer.
#![cfg(windows)]
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    QueryInformationJobObject, JobObjectBasicAccountingInformation, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, THREADENTRY32, TH32CS_SNAPTHREAD,
};
use windows_sys::Win32::System::Threading::{
    GetProcessIdOfThread, OpenProcess, OpenThread, ResumeThread,
    PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

pub const CREATE_SUSPENDED: u32 = 0x0000_0004;
const PROCESS_SUSPEND_RESUME: u32 = 0x0800;
const THREAD_SUSPEND_RESUME: u32 = 0x0002;
const THREAD_QUERY_LIMITED_INFORMATION: u32 = 0x0800;
const WAIT_OBJECT_0: u32 = 0;
const SYNCHRONIZE: u32 = 0x0010_0000;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

/// RAII owner of one Windows Job Object. Every child must be assigned before
/// its primary thread is resumed; KILL_ON_JOB_CLOSE covers every descendant.
pub struct JobObject(HANDLE);
unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
    /// The Job must exist before the ConPTY child is created. The native
    /// CREATE_PROCESS attribute assigns it atomically before user code runs.
    pub fn create_for_spawn() -> Result<Self, String> {
        Self::create()
    }

    pub fn as_raw_handle(&self) -> std::os::windows::io::RawHandle {
        self.0 as _
    }

    fn create() -> Result<Self, String> {
        unsafe {
            let job = CreateJobObjectW(ptr::null(), ptr::null());
            if job.is_null() {
                return Err(format!("WINDOWS_JOB_CREATE: {}", std::io::Error::last_os_error()));
            }
            let owner = Self(job);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                owner.0,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0 {
                return Err(format!("WINDOWS_JOB_LIMITS: {}", std::io::Error::last_os_error()));
            }
            Ok(owner)
        }
    }

    /// Legacy race-prone adoption is test-only. Production spawn paths must
    /// create suspended and call attach_suspended_pid before user code runs.
    #[cfg(test)]
    pub fn attach_pid(pid: u32) -> Result<Self, String> {
        let owner = Self::create()?;
        if pid == 0 { return Err("WINDOWS_JOB_INVALID_PID".into()); }
        unsafe {
            let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
            if process.is_null() {
                return Err(format!("WINDOWS_PROCESS_OPEN: {}", std::io::Error::last_os_error()));
            }
            let assigned = AssignProcessToJobObject(owner.0, process);
            let error = if assigned == 0 {
                Some(format!("WINDOWS_JOB_ASSIGN: {}", std::io::Error::last_os_error()))
            } else { None };
            CloseHandle(process);
            if let Some(error) = error { return Err(error); }
        }
        Ok(owner)
    }

    /// Atomically closes the spawn/adoption race:
    /// 1. caller created the process with CREATE_SUSPENDED;
    /// 2. assign its still-suspended process handle to this Job Object;
    /// 3. locate and resume its only (primary) thread.
    ///
    /// No user code can run between CreateProcess and Job assignment. Thread
    /// enumeration is safe here because the suspended process cannot create
    /// additional threads before its primary thread is resumed.
    pub fn attach_suspended_pid(pid: u32) -> Result<Self, String> {
        if pid == 0 { return Err("WINDOWS_JOB_INVALID_PID".into()); }
        let owner = Self::create()?;
        unsafe {
            let process = OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_SUSPEND_RESUME,
                0, pid,
            );
            if process.is_null() {
                return Err(format!("WINDOWS_PROCESS_OPEN: {}", std::io::Error::last_os_error()));
            }
            let assigned = AssignProcessToJobObject(owner.0, process);
            if assigned == 0 {
                let error = std::io::Error::last_os_error();
                CloseHandle(process);
                return Err(format!("WINDOWS_JOB_ASSIGN: {error}"));
            }

            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                let error = std::io::Error::last_os_error();
                CloseHandle(process);
                return Err(format!("WINDOWS_THREAD_SNAPSHOT: {error}"));
            }
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            let mut primary_tid = 0u32;
            let mut thread_count = 0usize;
            let mut more = Thread32First(snapshot, &mut entry) != 0;
            while more {
                if entry.th32OwnerProcessID == pid {
                    thread_count += 1;
                    primary_tid = entry.th32ThreadID;
                }
                more = Thread32Next(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
            if thread_count != 1 {
                CloseHandle(process);
                let _ = owner.terminate();
                return Err(format!("WINDOWS_SUSPENDED_PROCESS_THREAD_COUNT:{thread_count}"));
            }
            let thread = OpenThread(
                THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                0,
                primary_tid,
            );
            if thread.is_null() {
                let error = std::io::Error::last_os_error();
                CloseHandle(process);
                let _ = owner.terminate();
                return Err(format!("WINDOWS_PRIMARY_THREAD_OPEN: {error}"));
            }
            // Thread IDs can be reused between the Toolhelp snapshot and
            // OpenThread. Re-check ownership on the opened kernel handle
            // before resuming; never resume a recycled thread ID.
            if GetProcessIdOfThread(thread) != pid {
                CloseHandle(thread);
                CloseHandle(process);
                let _ = owner.terminate();
                return Err("WINDOWS_PRIMARY_THREAD_OWNER_MISMATCH".into());
            }
            let previous = ResumeThread(thread);
            CloseHandle(thread);
            CloseHandle(process);
            if previous != 1 {
                // 1 is the exact CREATE_SUSPENDED count. 0 means user code
                // was already running; >1 means another actor changed state.
                let _ = owner.terminate();
                return Err(if previous == u32::MAX {
                    format!("WINDOWS_PRIMARY_THREAD_RESUME: {}", std::io::Error::last_os_error())
                } else {
                    format!("WINDOWS_PRIMARY_THREAD_SUSPEND_COUNT:{previous}")
                });
            }
        }
        Ok(owner)
    }

    /// Kernel accounting, not a snapshot of just the original child's PID.
    /// TerminateJobObject requests termination; it does not prove descendants
    /// have exited. Supervisors retain this Job until ActiveProcesses is zero.
    pub fn active_processes(&self) -> Result<u32, String> {
        unsafe {
            let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
            if QueryInformationJobObject(
                self.0, JobObjectBasicAccountingInformation,
                &mut accounting as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                std::ptr::null_mut(),
            ) == 0 {
                return Err(format!("WINDOWS_JOB_ACCOUNTING: {}", std::io::Error::last_os_error()));
            }
            Ok(accounting.ActiveProcesses)
        }
    }

    pub fn is_empty(&self) -> Result<bool, String> {
        self.active_processes().map(|count| count == 0)
    }

    pub fn terminate(&self) -> Result<(), String> {
        unsafe {
            if TerminateJobObject(self.0, 1) == 0 {
                return Err(format!("WINDOWS_JOB_TERMINATE: {}", std::io::Error::last_os_error()));
            }
        }
        Ok(())
    }
}
impl Drop for JobObject {
    fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
}

/// Cancel synchronous I/O using a live, owned thread handle. This never
/// closes or duplicates a pipe behind another thread's back. Cancellation is
/// asynchronous and can race a new I/O request, so the cleanup owner repeats
/// it until the worker has actually exited and can be joined.
pub(crate) fn cancel_thread_io<T>(thread: &std::thread::JoinHandle<T>) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
    use windows_sys::Win32::System::IO::CancelSynchronousIo;
    unsafe {
        if CancelSynchronousIo(thread.as_raw_handle() as HANDLE) == 0 {
            let error = GetLastError();
            if error != ERROR_NOT_FOUND {
                return Err(format!("WINDOWS_THREAD_IO_CANCEL: {}", std::io::Error::from_raw_os_error(error as i32)));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::{process::Command, thread, time::{Duration, Instant}};

    fn wait_pid_exit(pid: u32, timeout: Duration) -> bool {
        unsafe {
            let process = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                // ERROR_INVALID_PARAMETER means the PID no longer exists.
                // Access denied and other failures are not treated as success.
                return GetLastError() == 87;
            }
            let result = windows_sys::Win32::System::Threading::WaitForSingleObject(
                process, timeout.as_millis().min(u32::MAX as u128) as u32,
            );
            CloseHandle(process);
            result == WAIT_OBJECT_0
        }
    }

    #[test]
    fn suspended_spawn_assigns_job_before_immediate_grandchild_can_run() {
        let marker = std::env::temp_dir()
            .join(format!("browseros-grandchild-{}.pid", uuid::Uuid::new_v4()));
        let marker_text = marker.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$p=Start-Process -FilePath $env:ComSpec -ArgumentList @('/c','ping -n 60 127.0.0.1 > NUL') -PassThru; Set-Content -LiteralPath '{marker_text}' -Value $p.Id; Start-Sleep -Seconds 60"
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        command.creation_flags(CREATE_SUSPENDED);
        let mut child = command.spawn().expect("create suspended PowerShell");
        assert!(!marker.exists(), "primary thread must not execute before Job assignment");
        let job = JobObject::attach_suspended_pid(child.id()).expect("assign Job then resume");

        let deadline = Instant::now() + Duration::from_secs(30);
        let grandchild_pid = loop {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = text.trim().parse::<u32>() { break pid; }
            }
            assert!(Instant::now() < deadline, "PowerShell did not create its immediate grandchild; exited={:?}", child.try_wait());
            thread::sleep(Duration::from_millis(25));
        };
        assert!(job.active_processes().expect("native Job accounting") >= 2,
            "the parent and immediate grandchild must both be accounted in the Job");
        job.terminate().expect("terminate entire Job");
        let _ = child.wait();
        assert!(wait_pid_exit(grandchild_pid, Duration::from_secs(8)),
            "grandchild PID {grandchild_pid} escaped the Job Object");
        let _ = std::fs::remove_file(marker);
    }

    #[test]
    fn dropping_job_after_parent_crash_kills_grandchild() {
        let marker = std::env::temp_dir()
            .join(format!("browseros-crash-grandchild-{}.pid", uuid::Uuid::new_v4()));
        let marker_text = marker.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$p=Start-Process -FilePath $env:ComSpec -ArgumentList @('/c','ping -n 60 127.0.0.1 > NUL') -PassThru; Set-Content -LiteralPath '{marker_text}' -Value $p.Id; Start-Sleep -Seconds 60"
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        command.creation_flags(CREATE_SUSPENDED);
        let mut child = command.spawn().expect("create suspended PowerShell");
        let job = JobObject::attach_suspended_pid(child.id()).expect("assign Job then resume");
        let deadline = Instant::now() + Duration::from_secs(30);
        let grandchild_pid = loop {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = text.trim().parse::<u32>() { break pid; }
            }
            assert!(Instant::now() < deadline, "PowerShell did not create grandchild");
            thread::sleep(Duration::from_millis(25));
        };
        // Simulate the parent process crashing: terminate only the direct
        // child, then drop the Job handle as the supervisor cleanup path does.
        let _ = child.kill();
        let _ = child.wait();
        drop(job);
        assert!(wait_pid_exit(grandchild_pid, Duration::from_secs(8)),
            "grandchild PID {grandchild_pid} survived parent crash cleanup");
        let _ = std::fs::remove_file(marker);
    }

    // These are mandatory native acceptance tests. GetConsoleWindow only
    // describes the calling process; it does not establish ConPTY support.
    // Missing native prerequisites must fail visibly, never return as PASS.

    struct NativeTerminalSink {
        captured: std::sync::Mutex<Vec<u8>>,
        fail_data: bool,
    }

    impl crate::terminal::TermOutput for NativeTerminalSink {
        fn send(&self, value: &serde_json::Value) -> Result<(), String> {
            if value.get("kind").and_then(|v| v.as_str()) == Some("data") {
                let mut captured = self.captured.lock().unwrap();
                if let Some(data) = value.get("data").and_then(|v| v.as_str()) {
                    captured.extend_from_slice(data.as_bytes());
                }
                if captured.len() > 8192 {
                    let excess = captured.len() - 8192;
                    captured.drain(..excess);
                }
                if self.fail_data { return Err("intentional native-fixture sink failure".into()); }
            }
            Ok(())
        }
    }

    fn finish_production_terminal(info: &crate::terminal::TermInfo, session: &mut crate::terminal::TerminalSession) {
        if let Err(error) = crate::terminal::terminate_session(session) {
            assert!(error.starts_with("TERM_CLEANUP_PENDING:"), "{error}");
        }
        crate::terminal::wait_for_cleanup(&info.id, 8_000)
            .expect("production close must join its actual input/reader/pump and PTY close owners");
    }

    #[test]
    fn conpty_terminal_uses_atomic_job_list_and_reaps_immediate_grandchild() {
        use portable_pty::CommandBuilder;
        use std::sync::{Arc, Mutex};
        let dir = std::env::temp_dir()
            .join(format!("browseros-conpty-job-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("grandchild.pid");
        let primary = dir.join("primary.txt");
        let marker_text = marker.to_string_lossy().replace('\'', "''");
        let primary_text = primary.to_string_lossy().replace('\'', "''");
        let script = format!(
            "Set-Content -LiteralPath '{primary_text}' -Value 'started'; $psi=New-Object System.Diagnostics.ProcessStartInfo; $psi.FileName=$env:ComSpec; $psi.Arguments='/c ping -n 60 127.0.0.1 > NUL'; $psi.UseShellExecute=$false; $p=[System.Diagnostics.Process]::Start($psi); Set-Content -LiteralPath '{marker_text}' -Value $p.Id; Start-Sleep -Seconds 60"
        );
        let sink = Arc::new(NativeTerminalSink { captured: Mutex::new(Vec::new()), fail_data: false });
        let mut command = CommandBuilder::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script.as_str()]);
        let (info, mut session) = crate::terminal::spawn_terminal_with_command(command, sink.clone(), false)
            .expect("production ConPTY spawn must assign Job atomically");
        let pid = session.process_id().expect("ConPTY child PID");
        let input = session.input();
        let control = session.control();
        let deadline = Instant::now() + Duration::from_secs(30);
        let grandchild_pid = loop {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = text.trim().parse::<u32>() { break pid; }
            }
            if Instant::now() >= deadline {
                let output = String::from_utf8_lossy(&sink.captured.lock().unwrap()).into_owned();
                panic!("ConPTY did not create grandchild; primary={:?} marker={:?} output={output:?}",
                    std::fs::read_to_string(&primary).ok(), std::fs::read_to_string(&marker).ok());
            }
            thread::sleep(Duration::from_millis(25));
        };
        assert_eq!(std::fs::read_to_string(&primary).unwrap().trim(), "started");
        // This is the same owner called by term_kill and application shutdown.
        // In particular, no fixture-only drop(writer)/drop(pair) unlocks EOF.
        finish_production_terminal(&info, &mut session);
        assert!(wait_pid_exit(pid, Duration::from_secs(8)), "ConPTY shell did not exit");
        assert!(wait_pid_exit(grandchild_pid, Duration::from_secs(8)),
            "ConPTY grandchild PID {grandchild_pid} escaped the Job Object");
        assert!(input.enqueue(b"late").is_err());
        assert!(control.resize(80, 24).is_err(), "control clones must not keep HPCON alive");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn conpty_production_close_reaps_queued_input_during_output_backoff() {
        use portable_pty::CommandBuilder;
        use std::sync::{Arc, Mutex};
        let sink = Arc::new(NativeTerminalSink { captured: Mutex::new(Vec::new()), fail_data: true });
        let mut command = CommandBuilder::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "[Console]::WriteLine('__PTY_READY__'); Start-Sleep -Seconds 60"]);
        let (info, mut session) = crate::terminal::spawn_terminal_with_command(command, sink.clone(), false)
            .expect("native ConPTY must be available; missing prerequisites are a failure");
        let pid = session.process_id().expect("native child PID");
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if String::from_utf8_lossy(&sink.captured.lock().unwrap()).contains("__PTY_READY__") { break; }
            assert!(Instant::now() < deadline, "native child did not reach non-reading state");
            thread::sleep(Duration::from_millis(10));
        }
        let input = session.input();
        let bytes = vec![b'x'; crate::terminal::TERM_INPUT_MAX_BYTES];
        let mut accepted = 0usize;
        for _ in 0..128 {
            match input.enqueue(&bytes) {
                Ok(()) => accepted += 1,
                Err(error) if error == "TERM_INPUT_QUEUE_FULL" => break,
                Err(error) => panic!("unexpected queued-input failure: {error}"),
            }
        }
        assert!(accepted > 0, "fixture must exercise the real input writer");
        finish_production_terminal(&info, &mut session);
        assert!(wait_pid_exit(pid, Duration::from_secs(8)), "native blocked-input child survived close");
        assert!(input.enqueue(b"after close").is_err());
    }

    /// This exercises CancelSynchronousIo on a real, full anonymous pipe, not
    /// a mock writer or an assumption that killing a child unblocks input.
    #[test]
    fn owned_thread_cancellation_interrupts_blocked_pipe_input() {
        use std::io::Write;
        use std::os::windows::io::FromRawHandle;
        use std::sync::mpsc;
        use windows_sys::Win32::System::Pipes::CreatePipe;
        let mut read = std::ptr::null_mut();
        let mut write = std::ptr::null_mut();
        assert_ne!(unsafe { CreatePipe(&mut read, &mut write, std::ptr::null(), 4096) }, 0,
            "create native synchronous pipe: {}", std::io::Error::last_os_error());
        let reader = unsafe { std::fs::File::from_raw_handle(read as _) };
        let mut writer = unsafe { std::fs::File::from_raw_handle(write as _) };
        let (entered_tx, entered_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            entered_tx.send(()).unwrap();
            writer.write(&vec![b'x'; 1024 * 1024])
        });
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut cancellation_error = None;
        while !worker.is_finished() && Instant::now() < deadline {
            if let Err(error) = cancel_thread_io(&worker) {
                cancellation_error = Some(error);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let cancelled = worker.is_finished();
        // Closing the still-unread read end makes even a failing fixture
        // release its writer before reporting the failure.
        drop(reader);
        let result = worker.join().expect("join synchronous pipe input");
        assert!(cancellation_error.is_none(), "{cancellation_error:?}");
        assert!(cancelled, "CancelSynchronousIo did not finish the owned input thread");
        assert!(result.map(|written| written < 1024 * 1024).unwrap_or(true),
            "a full, unread pipe must not accept the entire payload");
    }

    #[test]
    fn conpty_child_exit_code_259_is_terminal_and_kill_is_idempotent() {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtySystem};
        let pair = native_pty_system().openpty(
            PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 }
        ).expect("open ConPTY");
        let job = JobObject::create_for_spawn().expect("create Job");
        let mut command = CommandBuilder::new("cmd.exe");
        command.args(["/C", "exit /B 259"]);
        let mut child = pair.slave.spawn_command_in_job(command, job.as_raw_handle())
            .expect("spawn exit-code fixture in ConPTY Job");
        let pid = child.process_id().expect("native exit fixture PID");
        assert!(wait_pid_exit(pid, Duration::from_secs(8)), "ConPTY exit fixture timed out");
        let status = child.wait().expect("reap signaled process");
        assert_eq!(status.exit_code(), 259, "exit code 259 is not STILL_ACTIVE");
        assert_eq!(child.try_wait().unwrap().unwrap().exit_code(), 259);
        child.kill().expect("terminating an already-exited child is idempotent");
        drop(pair);
    }

    #[test]
    fn a_running_process_can_be_adopted_but_is_not_used_for_spawn_paths() {
        let mut child = Command::new("cmd.exe").args(["/C", "ping -n 60 127.0.0.1 > NUL"]).spawn().unwrap();
        let job = JobObject::attach_pid(child.id()).expect("legacy adoption");
        job.terminate().expect("terminate");
        let status = child.wait().expect("wait");
        assert!(!status.success());
    }
}
