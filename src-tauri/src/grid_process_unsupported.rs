//! Fail-closed placeholder for platforms lacking Grid process/IPC support.
//! XP-012/016: replace with a Windows Job Object + ACL-bound Named Pipe backend.
//! Not running Grid is safer than accidentally orphaning elevated child processes.
use crate::grid_ipc::{GridCmd, IpcRect};
use std::time::Duration;
use tauri::AppHandle;

#[derive(Default)]
pub struct GridProcessManager;

impl GridProcessManager {
    pub fn set_app(&self, _app: AppHandle) {}
    pub fn get_or_spawn(&self, _index: u32) -> Result<bool, String> { Err("GRID_PLATFORM_UNSUPPORTED".into()) }
    pub fn is_hibernated(&self, _index: u32) -> bool { false }
    pub fn hibernate_child(&self, _index: u32) -> bool { false }
    pub fn request(&self, _index: u32, _cmd: GridCmd, _timeout_ms: u64) -> Result<(), String> { Err("GRID_PLATFORM_UNSUPPORTED".into()) }
    pub fn read_replies(&self, _index: u32) -> Result<String, String> { Err("GRID_PLATFORM_UNSUPPORTED".into()) }
    pub fn send(&self, _index: u32, _cmd: GridCmd) {}
    pub fn record_url(&self, _index: u32, _url: &str) {}
    pub fn last_url(&self, _index: u32) -> Option<String> { None }
    pub fn record_rect(&self, _index: u32, _rect: (f64, f64, f64, f64)) {}
    pub fn record_hidden(&self, _index: u32) {}
    pub fn hibernate_idle_hidden(&self, _idle_for: Duration, _max_children: usize) -> Vec<u32> { Vec::new() }
    pub fn hibernated_count(&self) -> usize { 0 }
    pub fn indices(&self) -> Vec<u32> { Vec::new() }
    pub fn abs_rect(&self, _css: (f64, f64, f64, f64)) -> Option<IpcRect> { None }
    pub fn request_reposition_async(&self) {}
    pub fn reposition_visible(&self) {}
    pub fn set_child_focus(&self, _index: u32, _focused: bool) {}
    pub fn hide_for_blur_if_no_child_focus(&self) {}
    pub fn hide_for_blur(&self) {}
    pub fn show_for_focus(&self) {}
    pub fn kill_child(&self, _index: u32) -> bool { false }
    pub fn restart(&self, _index: u32) -> Result<(), String> { Err("GRID_PLATFORM_UNSUPPORTED".into()) }
    pub fn check_exited(&self, _index: u32) -> Option<i32> { None }
    pub fn count(&self) -> usize { 0 }
    pub fn pid_of(&self, _index: u32) -> Option<u32> { None }
    pub fn pids(&self) -> Vec<(u32, u32)> { Vec::new() }
    pub fn shutdown_all(&self) {}
}
