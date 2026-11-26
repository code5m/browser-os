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

/// Collect only logical IDs, selection and sleeping flags. No clone of URL,
/// title or other page-controlled metadata is necessary for this projection.
fn metadata_snapshot(state: &crate::bridge::AppState)
    -> Result<(Vec<String>, Option<String>, std::collections::HashSet<String>), String>
{
    let mut ids: Vec<_> = state.tabs.try_lock().map_err(|_| "AGENT_BROWSER_SNAPSHOT_BUSY")?
        .keys().cloned().collect();
    ids.sort_unstable();
    let active = state.active_tab.try_lock().map_err(|_| "AGENT_BROWSER_SNAPSHOT_BUSY")?.clone();
    let sleeping = state.hibernated_tabs.try_lock().map_err(|_| "AGENT_BROWSER_SNAPSHOT_BUSY")?.clone();
    Ok((ids, active, sleeping))
}

pub(crate) fn agent_tab_summary(app: AppHandle) -> Result<Value, String> {
    let state = app.state::<crate::bridge::AppState>();
    if state.shutdown_requested.load(std::sync::atomic::Ordering::Acquire) {
        return Err("AGENT_BROWSER_SNAPSHOT_BUSY".into());
    }
    let metadata = metadata_snapshot(&state)?;
    let (ids, active, hibernated) = &metadata;
    let count = ids.len();
    // Observe only the first 100 logical tabs, not unrelated Grid renderers.
    // Keep native identities in host memory for comparison only.
    let observations: Vec<_> = ids.iter().take(100).map(|id| {
        crate::bridge::isolated_tabs::route(id).map(|index|
            state.grid_manager.tab_renderer_observation(index).ok())
    }).collect();
    let mut stable = metadata_snapshot(&state).is_ok_and(|current| current == metadata);
    let observed_again: Vec<_> = ids.iter().take(100).map(|id| {
        crate::bridge::isolated_tabs::route(id).map(|index|
            state.grid_manager.tab_renderer_observation(index).ok())
    }).collect();
    stable &= metadata_snapshot(&state).is_ok_and(|current| current == metadata)
        && !state.shutdown_requested.load(std::sync::atomic::Ordering::Acquire);
    let renderer_consistent = stable && observations.iter().zip(&observed_again).all(|(first, second)|
        first == second && !matches!(first, Some(None)));
    let items = ids.iter().take(100).enumerate().map(|(index, id)| {
        let first = &observations[index];
        let second = &observed_again[index];
        let isolated = first.is_some();
        let availability = if !stable || first != second {
            "unknown"
        } else {
            match first {
                Some(Some((owner, status))) => match *status {
                    "cleanup_pending" => "cleanup_pending",
                    "ready" if owner.is_some() => "host_ready",
                    "not_ready" if owner.is_some() => "host_not_ready",
                    "absent" if owner.is_none() => tab_availability(true, hibernated.contains(id), None, true),
                    _ => "unknown",
                },
                Some(None) => "unknown",
                None => tab_availability(false, hibernated.contains(id), None, true),
            }
        };
        json!({"id":id,"title":null,"title_redacted":true,
            "backend":if isolated {"isolated"} else {"embedded"},
            "availability":availability,
            "active":stable && active.as_deref() == Some(id.as_str())})
    }).collect::<Vec<_>>();
    Ok(json!({"count":count,"truncated":count > 100,
        "logical_snapshot_consistent":stable,"active_known":stable,
        "renderer_snapshot_consistent":renderer_consistent,"items":items}))
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
