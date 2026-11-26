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

    pub(crate) fn grid_route_stamps(&self, _indices: &[u32]) -> Result<Vec<crate::grid_ipc::GridRouteStamp>, String> { Ok(Vec::new()) }
    pub(crate) fn close_grid_targets(&self, targets: &[crate::grid_ipc::GridRouteStamp]) -> Result<(), String> {
        if targets.is_empty() { Ok(()) } else { Err(unsupported()) }
    }
    pub(crate) fn close_grid_targets_checked(&self, targets: &[crate::grid_ipc::GridRouteStamp], retained: &[crate::grid_ipc::GridRouteStamp]) -> Result<(), String> {
        if targets.is_empty() && retained.is_empty() { Ok(()) } else { Err(unsupported()) }
    }
    pub(crate) fn prepare_grid_slot(&self, _target: crate::grid_ipc::GridRouteStamp) -> Result<crate::grid_ipc::GridPreparedSlot, String> { Err(unsupported()) }
    pub(crate) fn grid_binding_for_stamp(&self, _stamp: crate::grid_ipc::GridRouteStamp) -> Result<crate::grid_ipc::RendererBinding, String> { Err(unsupported()) }
    pub(crate) fn rollback_grid_creation(&self, _prepared: &crate::grid_ipc::GridPreparedSlot) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_create_bound(&self, _binding: crate::grid_ipc::RendererBinding, _url: &str) -> Result<(), String> { Err(unsupported()) }

    pub fn get_or_spawn(&self, _index: u32) -> Result<bool, String> {
        Err(unsupported())
    }

    pub(crate) fn tab_renderer_owner(&self, _index: u32) -> Result<Option<(u32, u64)>, String> {
        Err(unsupported())
    }
    pub(crate) fn select_tab_renderer(&self, _index: u32, _others: &[u32]) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn select_tab_renderer_bound(&self, _target: crate::grid_ipc::RendererBinding, _others: &[u32]) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn close_tab_renderer_bound(&self, _index: u32, _release_listener: bool, _expected: Option<(u32, u64)>) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn publish_tab_renderer_info(&self, _index: u32, _info: crate::domain::TabInfo)
        -> Result<crate::domain::TabInfo, String> {
        Err(unsupported())
    }
    pub(crate) fn publish_reopened_tab(&self, _binding: crate::grid_ipc::RendererBinding)
        -> Result<crate::domain::TabInfo, String> { Err(unsupported()) }
    pub(crate) fn navigate_tab_renderer(&self, _index: u32, _url: &str) -> Result<(), String> {
        Err(unsupported())
    }
    pub(crate) fn tab_renderer_observation(&self, _index: u32) -> Result<(Option<(u32, u64)>, &'static str), String> { Err(unsupported()) }
    pub(crate) fn tab_renderer_inspection(&self, _index: u32) -> Result<(Option<(u32, u64)>, &'static str, Option<serde_json::Value>), String> { Err(unsupported()) }
    pub(crate) fn tab_download_inventory(&self) -> Result<serde_json::Value, String> { Err(unsupported()) }
    pub(crate) fn sample_tab_downloads(&self, _binding: crate::grid_ipc::RendererBinding, _timeout_ms: u64)
        -> Result<bool, String> { Err(unsupported()) }
    pub(crate) fn refresh_tab_downloads(&self, _binding: crate::grid_ipc::RendererBinding, _timeout_ms: u64)
        -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn tab_renderer_download_observation(&self, _index: u32, _expected: (u32, u64))
        -> Result<Option<serde_json::Value>, String> { Err(unsupported()) }
    pub(crate) fn hide_grid_renderers(&self) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn hide_grid_route(&self, _stamp: crate::grid_ipc::GridRouteStamp) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn capture_window_route(&self, _index: u32) -> Result<Option<crate::grid_ipc::RendererBinding>, String> { Err(unsupported()) }
    pub(crate) fn position_tab_renderer_bound(&self, _index: u32, _target: Option<crate::grid_ipc::RendererBinding>, _css: (f64, f64, f64, f64)) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn hide_tab_renderer_bound(&self, _index: u32, _target: Option<crate::grid_ipc::RendererBinding>) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn navigate_tab_renderer_bound(&self, _binding: crate::grid_ipc::RendererBinding, _url: &str) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn renderer_binding(&self, _index: u32) -> Result<crate::grid_ipc::RendererBinding, String> { Err(unsupported()) }
    pub(crate) fn grid_eval_binding(&self, _index: u32) -> Result<crate::grid_ipc::RendererBinding, String> { Err(unsupported()) }
    pub(crate) fn grid_live_binding(&self, _index: u32) -> Result<Option<crate::grid_ipc::RendererBinding>, String> { Err(unsupported()) }
    pub(crate) fn grid_eval_bound(&self, _binding: crate::grid_ipc::RendererBinding, _js: String) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn publish_grid_zoom(&self, _stamp: crate::grid_ipc::GridRouteStamp, _zoom: f64) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_zoom_bound(&self, _binding: crate::grid_ipc::RendererBinding, _zoom: f64) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_navigate_bound(&self, _binding: crate::grid_ipc::RendererBinding, _url: &str) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_position_bound(&self, _binding: crate::grid_ipc::RendererBinding, _css: (f64, f64, f64, f64), _rect: crate::grid_ipc::IpcRect) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn grid_read_replies_bound(&self, _binding: crate::grid_ipc::RendererBinding) -> Result<String, String> { Err(unsupported()) }
    pub(crate) fn renderer_request_data(&self, _binding: crate::grid_ipc::RendererBinding, _cmd: GridCmd, _timeout_ms: u64) -> Result<Option<String>, String> { Err(unsupported()) }
    pub(crate) fn retire_bound_tab_renderer(&self, _index: u32, _expected: (u32, u64)) -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn park_tab_renderer(&self, _binding: crate::grid_ipc::RendererBinding, _address: &str)
        -> Result<(), String> { Err(unsupported()) }
    pub(crate) fn hibernate_blank_tab_renderer(&self, _index: u32, _idle_for: std::time::Duration) -> Result<bool, String> {
        Err(unsupported())
    }
    pub(crate) fn tab_renderer_absence_epoch(&self, _index: u32) -> Result<u64, String> { Err(unsupported()) }
    pub(crate) fn start_tab_renderer_at_absence(&self, _index: u32, _url: &str, _expected: u64) -> Result<(), String> { Err(unsupported()) }
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

    pub(crate) fn try_grid_is_hibernated(&self, _index: u32) -> Result<bool, String> { Err(unsupported()) }
    pub fn is_hibernated(&self, _index: u32) -> bool {
        false
    }
    pub fn hibernate_child(&self, _index: u32) -> bool {
        false
    }

    pub fn request(&self, _index: u32, _cmd: GridCmd, _timeout_ms: u64) -> Result<(), String> {
        Err(unsupported())
    }

    pub(crate) fn retry_failed_navigation(&self, _binding: crate::grid_ipc::RendererBinding, _url: &str) -> Result<(), String> {
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
    pub(crate) fn grid_route_statuses(&self) -> Option<Vec<crate::grid_ipc::GridRouteStatus>> { None }
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
