//! Lifecycle boundary for targets without a native Grid process adapter.
//!
//! This is deliberately not an IPC implementation. Mutating/request operations
//! fail explicitly; inspection and shutdown describe an empty process set.
//! The bridge rejects user operations before changing Grid UI metadata.
use crate::grid_ipc::{GridCmd, IpcRect};
use tauri::AppHandle;

fn unsupported() -> String {
    format!("GRID_PLATFORM_UNSUPPORTED:{}", std::env::consts::OS)
}

#[derive(Default)]
pub struct GridProcessManager;

impl GridProcessManager {
    pub fn set_app(&self, _app: AppHandle) {}

    pub fn get_or_spawn(&self, _index: u32) -> Result<bool, String> {
        Err(unsupported())
    }

    pub(crate) fn tab_renderer_owner(&self, _index: u32) -> Result<Option<(u32, u64)>, String> {
        Err(unsupported())
    }
    pub(crate) fn select_tab_renderer(&self, _index: u32, _others: &[u32]) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn publish_tab_renderer_info(&self, _index: u32, _info: crate::domain::TabInfo)
        -> Result<crate::domain::TabInfo, String> {
        Err(unsupported())
    }
    pub(crate) fn navigate_tab_renderer(&self, _index: u32, _url: &str) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn tab_renderer_observation(&self, _index: u32) -> Result<(Option<(u32, u64)>, &'static str), String> { Err(unsupported()) }
    pub(crate) fn renderer_binding(&self, _index: u32) -> Result<crate::grid_ipc::RendererBinding, String> { Err(unsupported()) }
    pub(crate) fn grid_eval_binding(&self, _index: u32) -> Result<crate::grid_ipc::RendererBinding, String> { Err(unsupported()) }
    pub(crate) fn grid_eval_bound(&self, _binding: crate::grid_ipc::RendererBinding, _js: String) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_zoom_bound(&self, _binding: crate::grid_ipc::RendererBinding, _zoom: f64) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_navigate_bound(&self, _binding: crate::grid_ipc::RendererBinding, _url: &str) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_position_bound(&self, _binding: crate::grid_ipc::RendererBinding, _css: (f64, f64, f64, f64), _rect: crate::grid_ipc::IpcRect) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_read_replies_bound(&self, _binding: crate::grid_ipc::RendererBinding) -> Result<String, String> { Err(unsupported()) }
    pub(crate) fn renderer_request_data(&self, _binding: crate::grid_ipc::RendererBinding, _cmd: GridCmd, _timeout_ms: u64) -> Result<Option<String>, String> { Err(unsupported()) }
    pub(crate) fn retire_bound_tab_renderer(&self, _index: u32, _expected: (u32, u64)) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn hibernate_blank_tab_renderer(&self, _index: u32, _idle_for: std::time::Duration) -> Result<bool, String> {
        Err(unsupported())
    }
    pub(crate) fn start_tab_renderer(&self, _index: u32, _url: &str) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn close_tab_renderer(&self, _index: u32, _release_listener: bool) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn position_tab_renderer(&self, _index: u32, _css: (f64, f64, f64, f64)) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn hide_tab_renderer(&self, _index: u32) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn all_renderer_indices(&self) -> Vec<u32> { Vec::new() }

    pub fn is_hibernated(&self, _index: u32) -> bool {
        false
    }
    pub fn hibernate_child(&self, _index: u32) -> bool {
        false
    }

    pub fn request(&self, _index: u32, _cmd: GridCmd, _timeout_ms: u64) -> Result<(), String> {
        Err(unsupported())
    }

    pub(crate) fn retry_failed_navigation(&self, _index: u32, _generation: u64, _url: &str) -> Result<(), String> {
        Err(unsupported())
    }

    pub fn read_replies(&self, _index: u32) -> Result<String, String> {
        Err(unsupported())
    }

    // Best-effort lifecycle notifications have no recipient on this adapter.
    // They neither create a process nor record a successful command response.
    pub fn send(&self, _index: u32, _cmd: GridCmd) {}
    pub fn record_url(&self, _index: u32, _url: &str) {}
    pub fn last_url(&self, _index: u32) -> Option<String> {
        None
    }
    pub fn record_rect(&self, _index: u32, _rect: (f64, f64, f64, f64)) {}
    pub fn record_hidden(&self, _index: u32) {}

    pub fn hibernate_idle_hidden(
        &self,
        _idle_for: std::time::Duration,
        _max_children: usize,
    ) -> Vec<u32> {
        Vec::new()
    }

    pub fn hibernated_count(&self) -> usize {
        0
    }
    pub fn indices(&self) -> Vec<u32> {
        Vec::new()
    }
    pub fn abs_rect(&self, _css: (f64, f64, f64, f64)) -> Option<IpcRect> {
        None
    }
    pub fn request_hide_for_blur_async(&self) {}
    pub fn request_show_for_focus_async(&self) {}
    pub fn request_reposition_async(&self) {}
    pub fn reposition_visible(&self) {}
    pub fn hide_for_blur_if_no_child_focus(&self) {}
    pub fn hide_for_blur(&self) {}
    pub fn show_for_focus(&self) {}
    pub fn kill_child(&self, _index: u32) -> bool {
        false
    }

    pub fn restart(&self, _index: u32) -> Result<(), String> {
        Err(unsupported())
    }

    pub fn check_exited(&self, _index: u32) -> Option<i32> {
        None
    }
    pub fn count(&self) -> usize {
        0
    }
    pub fn pid_of(&self, _index: u32) -> Option<u32> {
        None
    }
    pub(crate) fn renderer_processes(&self) -> Vec<crate::grid_ipc::RendererProcess> {
        Vec::new()
    }
    pub fn pids(&self) -> Vec<(u32, u32)> {
        Vec::new()
    }
    pub fn request_shutdown(&self) {}
    pub fn shutdown_until(&self, _deadline: std::time::Instant) -> Result<(), String> {
        Ok(())
    }
    pub fn shutdown_all(&self) -> Result<(), String> {
        Ok(())
    }
}
