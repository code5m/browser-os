//! Windows process tree ownership. Job Objects kill descendants on close;
//! unlike direct Child::kill(), this also covers scripts spawned by ConPTY.
//! This is process lifecycle containment, NOT a security sandbox or AppContainer.
#![cfg(windows)]
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

/// RAII owner of one Windows Job Object. Never duplicate or leak the handle.
pub struct JobObject(HANDLE);
// The kernel synchronizes job operations. Every user shares an Arc<JobObject>;
// the handle is closed only when the last Arc drops.
unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
    pub fn attach_pid(pid: u32) -> Result<Self, String> {
        if pid == 0 {
            return Err("WINDOWS_JOB_INVALID_PID".into());
        }
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
            let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
            if process.is_null() {
                return Err(format!("WINDOWS_PROCESS_OPEN: {}", std::io::Error::last_os_error()));
            }
            let assigned = AssignProcessToJobObject(owner.0, process);
            let err = if assigned == 0 {
                Some(format!("WINDOWS_JOB_ASSIGN: {}", std::io::Error::last_os_error()))
            } else {
                None
            };
            CloseHandle(process);
            if let Some(err) = err {
                return Err(err);
            }
            Ok(owner)
        }
    }

    /// Explicit child-tree termination; Drop also kills all remaining processes.
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
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn job_tree_is_terminated_when_owner_closes() {
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/C", "ping -n 60 127.0.0.1 > NUL"])
            .spawn()
            .expect("spawn");
        let job = JobObject::attach_pid(child.id()).expect("attach job");
        job.terminate().expect("terminate");
        let status = child.wait().expect("wait child");
        assert!(!status.success(), "terminated job should not exit successfully");
    }
}
