//! Windows process tree ownership. Job Objects kill descendants on close;
//! unlike direct Child::kill(), this also covers scripts spawned by ConPTY.
//! This is process lifecycle containment, NOT a security sandbox or AppContainer.
#![cfg(windows)]
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, THREADENTRY32, TH32CS_SNAPTHREAD,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, OpenThread, ResumeThread, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

pub const CREATE_SUSPENDED: u32 = 0x0000_0004;
const PROCESS_SUSPEND_RESUME: u32 = 0x0800;
const THREAD_SUSPEND_RESUME: u32 = 0x0002;
const WAIT_OBJECT_0: u32 = 0;
const SYNCHRONIZE: u32 = 0x0010_0000;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

/// RAII owner of one Windows Job Object. Every child must be assigned before
/// its primary thread is resumed; KILL_ON_JOB_CLOSE covers every descendant.
pub struct JobObject(HANDLE);
unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
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

    /// Legacy adoption for processes whose creator cannot provide a suspended
    /// creation contract. Do not use for Grid, scripts or other untrusted work.
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
            let mut found = false;
            let mut resume_error = None;
            let mut more = Thread32First(snapshot, &mut entry) != 0;
            while more {
                if entry.th32OwnerProcessID == pid {
                    found = true;
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if thread.is_null() {
                        resume_error = Some(format!("WINDOWS_PRIMARY_THREAD_OPEN: {}",
                            std::io::Error::last_os_error()));
                    } else {
                        let previous = ResumeThread(thread);
                        CloseHandle(thread);
                        if previous == u32::MAX {
                            resume_error = Some(format!("WINDOWS_PRIMARY_THREAD_RESUME: {}",
                                std::io::Error::last_os_error()));
                        } else if previous != 1 {
                            // 1 is the exact CREATE_SUSPENDED count. 0 means
                            // user code was already running; >1 means another
                            // actor changed suspension state. Both fail closed.
                            resume_error = Some(format!("WINDOWS_PRIMARY_THREAD_SUSPEND_COUNT:{previous}"));
                        }
                    }
                    break;
                }
                more = Thread32Next(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
            CloseHandle(process);
            if !found {
                return Err("WINDOWS_PRIMARY_THREAD_NOT_FOUND".into());
            }
            if let Some(error) = resume_error {
                let _ = owner.terminate();
                return Err(error);
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
            if process.is_null() { return true; }
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

        let deadline = Instant::now() + Duration::from_secs(10);
        let grandchild_pid = loop {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = text.trim().parse::<u32>() { break pid; }
            }
            assert!(Instant::now() < deadline, "PowerShell did not create its immediate grandchild");
            thread::sleep(Duration::from_millis(25));
        };
        job.terminate().expect("terminate entire Job");
        let _ = child.wait();
        assert!(wait_pid_exit(grandchild_pid, Duration::from_secs(8)),
            "grandchild PID {grandchild_pid} escaped the Job Object");
        let _ = std::fs::remove_file(marker);
    }

    #[test]
    fn conpty_terminal_uses_suspended_spawn_and_job_assignment_before_shell_runs() {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtySystem};
        use std::{thread, time::{Duration, Instant}};

        let dir = std::env::temp_dir().join(format!("browseros-conpty-job-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("grandchild.pid");
        let marker_text = marker.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$p=Start-Process -FilePath $env:ComSpec -ArgumentList @('/c','ping -n 60 127.0.0.1 > NUL') -PassThru; Set-Content -LiteralPath '{marker_text}' -Value $p.Id; Start-Sleep -Seconds 60"
        );
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .expect("open ConPTY");
        let mut command = CommandBuilder::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script.as_str()]);
        let mut child = pair.slave.spawn_command_suspended(command)
            .expect("ConPTY process must be created suspended");
        let pid = child.process_id().expect("ConPTY child PID");
        assert!(!marker.exists(), "PowerShell must not execute before Job assignment");
        let job = JobObject::attach_suspended_pid(pid).expect("assign Job then resume");

        let deadline = Instant::now() + Duration::from_secs(10);
        let grandchild_pid = loop {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = text.trim().parse::<u32>() { break pid; }
            }
            assert!(Instant::now() < deadline, "ConPTY PowerShell did not create its grandchild");
            thread::sleep(Duration::from_millis(25));
        };
        job.terminate().expect("terminate ConPTY process tree");
        let _ = child.wait();
        assert!(wait_pid_exit(grandchild_pid, Duration::from_secs(8)),
            "ConPTY grandchild PID {grandchild_pid} escaped the Job Object");
        drop(pair);
        let _ = std::fs::remove_dir_all(dir);
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
