//! Windows process tree ownership. Job Objects kill descendants on close;
//! unlike direct Child::kill(), this also covers scripts spawned by ConPTY.
//! This is process lifecycle containment, NOT a security sandbox or AppContainer.
#![cfg(windows)]
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
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

    #[test]
    fn conpty_terminal_uses_atomic_job_list_and_reaps_immediate_grandchild() {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtySystem};
        use std::io::{Read, Write};
        use std::sync::{Arc, Mutex};
        use std::{thread, time::{Duration, Instant}};

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
        let pair = native_pty_system().openpty(
            PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 }
        ).expect("open ConPTY");
        // ConPTY is a bidirectional pipe. Start draining output before
        // CreateProcessW so startup control traffic never blocks the child.
        let mut reader = pair.master.try_clone_reader().expect("clone ConPTY output");
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let capture = Arc::clone(&captured);
        let _pump = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 { break; }
                let mut output = capture.lock().unwrap();
                output.extend_from_slice(&buf[..n]);
                if output.len() > 8192 {
                    let excess = output.len() - 8192;
                    output.drain(..excess);
                }
            }
        });
        let job = JobObject::create_for_spawn().expect("create kill-on-close Job");
        let mut command = CommandBuilder::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script.as_str()]);
        let mut child = pair.slave.spawn_command_in_job(command, job.as_raw_handle())
            .expect("CreateProcessW must assign Job atomically with ConPTY");
        let pid = child.process_id().expect("ConPTY child PID");
        assert_ne!(pid, 0);
        let mut writer = pair.master.take_writer().expect("open ConPTY input");
        writer.write_all(b"\r\n").expect("write initial console input");
        writer.flush().expect("flush initial console input");

        let deadline = Instant::now() + Duration::from_secs(30);
        let grandchild_pid = loop {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = text.trim().parse::<u32>() { break pid; }
            }
            if Instant::now() >= deadline {
                let output = String::from_utf8_lossy(&captured.lock().unwrap()).into_owned();
                panic!(
                    "ConPTY did not create grandchild; primary={:?} marker={:?} status={:?} output={output:?}",
                    std::fs::read_to_string(&primary).ok(),
                    std::fs::read_to_string(&marker).ok(),
                    child.try_wait()
                );
            }
            thread::sleep(Duration::from_millis(25));
        };
        assert_eq!(std::fs::read_to_string(&primary).unwrap().trim(), "started");
        job.terminate().expect("terminate ConPTY Job and all descendants");
        child.wait().expect("wait ConPTY shell");
        assert!(wait_pid_exit(grandchild_pid, Duration::from_secs(8)),
            "ConPTY grandchild PID {grandchild_pid} escaped the Job Object");
        drop(writer);
        drop(pair);
        let _ = std::fs::remove_dir_all(dir);
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
        let status = child.wait().expect("wait for signaled process");
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
