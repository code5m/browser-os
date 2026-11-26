//! Stable, read-only Browser Owner API for the Agent Channel host.
//! Only redacted tab metadata leaves this module; no page content or URL.
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

/// A logical tab is not necessarily an executable native page. This is a
/// read-only host observation; "host_ready" does not certify page load or DOM
/// stability, and no URL, native PID, credential or page content is exposed.
fn tab_availability(
    isolated: bool,
    hibernated: bool,
    owner: Option<&crate::grid_ipc::RendererProcess>,
    consistent: bool,
) -> &'static str {
    if !consistent { return "unknown"; }
    if !isolated {
        return if hibernated { "hibernated" } else { "registered" };
    }
    match owner {
        Some(process) if process.ready => "host_ready",
        Some(_) => "host_not_ready",
        None if hibernated => "hibernated",
        None => "recovery_required",
    }
}

pub(crate) fn agent_tab_summary(app: AppHandle) -> Value {
    let state = app.state::<crate::bridge::AppState>();
    // No lock is held while acquiring another owner's snapshot.
    let owners = state.grid_manager.renderer_processes();
    let mut tabs = crate::bridge::tab_list_native(app.clone());
    let hibernated = state.hibernated_tabs.lock().unwrap().clone();
    let active = state.active_tab.lock().unwrap().clone();
    let consistent = state.grid_manager.renderer_processes() == owners;
    tabs.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    let count = tabs.len();
    json!({"count":count,"truncated":count > 100,"renderer_snapshot_consistent":consistent,
        "items":tabs.into_iter().take(100).map(|tab| {
            let route = crate::bridge::isolated_tabs::route(&tab.id);
            let owner = route.and_then(|index| owners.iter().find(|process| process.index == index));
            let availability = if let Some(index) = route {
                // Share the same native observation as the trusted recovery UI.
                // Do not expose PID/epoch or turn cleanup into execution authority.
                match state.grid_manager.tab_renderer_observation(index) {
                    Ok((observed, status)) if consistent
                        && observed == owner.map(|p| (p.pid, p.spawn_epoch)) => {
                        match status {
                            "cleanup_pending" => "cleanup_pending",
                            "ready" if owner.is_some_and(|p| p.ready) => "host_ready",
                            "not_ready" => "host_not_ready",
                            "absent" => tab_availability(true, hibernated.contains(&tab.id), None, true),
                            _ => "unknown",
                        }
                    }
                    _ => "unknown",
                }
            } else {
                tab_availability(false, hibernated.contains(&tab.id), None, consistent)
            };
            let is_active = active.as_deref() == Some(tab.id.as_str());
            // Native logical title can be a raw URL fallback or arbitrary
            // page-supplied text. Keep the field as null in this restricted
            // Agent projection instead of presenting it as redacted metadata.
            json!({"id":tab.id,"title":null,"title_redacted":true,
                "backend":if route.is_some() { "isolated" } else { "embedded" },
                "availability":availability,
                "active":is_active})
        }).collect::<Vec<_>>()})
}

#[cfg(test)]
mod tab_availability_tests {
    use super::tab_availability;
    use crate::grid_ipc::RendererProcess;

    #[test]
    fn logical_metadata_never_implies_a_live_renderer() {
        let mut owner = RendererProcess {
            index: 0x80000001, pid: 42, spawn_epoch: 1,
            ready: false, startup_reserved: true,
        };
        assert_eq!(tab_availability(true, false, None, true), "recovery_required");
        assert_eq!(tab_availability(true, true, None, true), "hibernated");
        assert_eq!(tab_availability(true, false, Some(&owner), true), "host_not_ready");
        owner.ready = true;
        assert_eq!(tab_availability(true, false, Some(&owner), true), "host_ready");
        assert_eq!(tab_availability(true, false, Some(&owner), false), "unknown");
        assert_eq!(tab_availability(false, false, None, true), "registered");
    }
}

/// Metadata-only watchdog observation through the Browser capability.
pub(crate) fn capture_diagnostic() -> Value {
    super::commands::capture_diagnostic()
}

/// Reversible application exit preflight through the existing Browser Owner.
pub(crate) fn prepare_capture_exit(app: &AppHandle) -> Result<(), String> {
    super::commands::prepare_capture_exit(app)
}

// The supervisor may only claim the exact reviewed capture.
pub(crate) use super::commands::{
    claim_capture_domain_recovery, pending_capture_domain_recovery,
    CaptureDomainRecoveryClaim, CaptureDomainRecoveryIntent,
};
