//! Native receipt ownership for main-window tab mutations (ADR D01).
//! A receipt describes the host mutation, not the renderer's navigation/paint.
//! A lost IPC reply, cancelled waiter or elapsed deadline cannot release a live
//! worker. Prepared work has no effects; running cancellation is advisory.
use super::tab_runtime::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Webview};
use tauri_plugin_browser_tabs::{CaptureDocument, TabInstance, TabManagerState};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Action {
    New { url: String },
    Close { id: String },
    Activate { id: String },
    Open { id: String, url: String },
    Back { id: String },
    Forward { id: String },
    Reload { id: String },
    RetireRenderer { id: String, pid: u32, spawn_epoch: String, confirmed: bool },
    ReopenRenderer { id: String, url: String, confirmed: bool,
        review_owner_epoch: String, review_owner_sequence: String, absence_epoch: String },
    ParkRenderer { id: String, url: String, pid: u32, spawn_epoch: String,
        renderer_generation: String, navigation_revision: String, confirmed: bool },
}
impl Action {
    fn command(&self) -> &'static str {
        match self {
            Self::New { .. } => "tab_new",
            Self::Close { .. } => "tab_close",
            Self::Activate { .. } => "tab_activate",
            Self::Open { .. } => "tab_open",
            Self::Back { .. } => "tab_go_back",
            Self::Forward { .. } => "tab_go_forward",
            Self::Reload { .. } => "tab_reload",
            Self::RetireRenderer { .. } => "tab_renderer_retire",
            Self::ReopenRenderer { .. } => "tab_open",
            Self::ParkRenderer { .. } => "tab_renderer_park",
        }
    }
    fn target(&self) -> Option<&str> {
        match self {
            Self::New { .. } => None,
            Self::Close { id }
            | Self::Activate { id }
            | Self::Open { id, .. }
            | Self::Back { id }
            | Self::Forward { id }
            | Self::Reload { id }
            | Self::RetireRenderer { id, .. }
            | Self::ReopenRenderer { id, .. }
            | Self::ParkRenderer { id, .. } => Some(id),
        }
    }
    fn authorize(&self, webview: &Webview) -> Result<(), String> {
        use crate::shared::native_legacy_policy::{bound_text, require_main};
        // Retiring one native renderer is the same destructive privilege
        // as closing it, but retains the logical tab. No new IPC permission.
        let cmd = if matches!(self, Self::RetireRenderer { .. } | Self::ParkRenderer { .. }) { "tab_close" } else { self.command() };
        require_main(webview, cmd)?;
        match self {
            Self::New { url } => bound_text(cmd, "url", url),
            Self::Open { id, url } => {
                bound_text(cmd, "id", id)?;
                bound_text(cmd, "url", url)
            }
            Self::Close { id }
            | Self::Activate { id }
            | Self::Back { id }
            | Self::Forward { id }
            | Self::Reload { id } => bound_text(cmd, "id", id),
            Self::ReopenRenderer { id, url, confirmed, review_owner_epoch, review_owner_sequence, absence_epoch } => {
                bound_text(cmd, "id", id)?;
                bound_text(cmd, "url", url)?;
                if !confirmed || super::isolated_tabs::route(id).is_none()
                    || !crate::grid_ipc::explicit_reopen_address(url)
                    || review_owner_epoch.len() != 36
                    || uuid::Uuid::parse_str(review_owner_epoch).is_err()
                    || review_owner_sequence.parse::<u64>().ok().filter(|n| *n > 0)
                        .is_none_or(|n| n.to_string() != *review_owner_sequence)
                    || absence_epoch.parse::<u64>().ok()
                        .is_none_or(|n| n.to_string() != *absence_epoch) {
                    return Err("TAB_RENDERER_REOPEN_INVALID".into());
                }
                Ok(())
            }
            Self::ParkRenderer { id, url, pid, spawn_epoch, renderer_generation, navigation_revision, confirmed } => {
                bound_text(cmd, "id", id)?;
                bound_text(cmd, "url", url)?;
                if !confirmed || *pid == 0 || !crate::grid_ipc::restorable_address(url)
                    || spawn_epoch.parse::<u64>().ok().filter(|n| *n > 0)
                        .is_none_or(|n| n.to_string() != *spawn_epoch)
                    || [renderer_generation, navigation_revision].iter().any(|value|
                        value.parse::<u64>().ok().filter(|n| *n > 0)
                            .is_none_or(|n| n.to_string().as_str() != value.as_str()))
                    || super::isolated_tabs::route(id).is_none() {
                    return Err("TAB_RENDERER_PARK_INVALID".into());
                }
                Ok(())
            }
            Self::RetireRenderer { id, pid, spawn_epoch, confirmed } => {
                bound_text(cmd, "id", id)?;
                if !confirmed || *pid == 0 || spawn_epoch.parse::<u64>().ok().filter(|n| *n > 0)
                    .is_none_or(|n| n.to_string() != *spawn_epoch)
                    || super::isolated_tabs::route(id).is_none()
                {
                    return Err("TAB_RENDERER_RECOVERY_INVALID_TARGET".into());
                }
                Ok(())
            }
        }
    }
    fn is_document_mutation(&self) -> bool {
        matches!(self, Self::Open { .. } | Self::Back { .. } | Self::Forward { .. } | Self::Reload { .. } | Self::ParkRenderer { .. })
    }

    fn execute(self, webview: Webview, app: AppHandle, binding: &Binding) -> Result<Value, String> {
        self.authorize(&webview)?;
        let state = app.state::<crate::AppState>();
        if state.shutdown_requested.load(std::sync::atomic::Ordering::Acquire) {
            return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
        }
        if let Self::Close { ref id } = self {
            let admitted = match binding {
                Binding::Live(instance) => super::TabCloseAdmission::Embedded(instance),
                Binding::Isolated(owner) => super::TabCloseAdmission::Isolated(*owner),
                Binding::Hibernated => super::TabCloseAdmission::Hibernated,
                _ => return Err("TAB_MUTATION_INSTANCE_STALE".into()),
            };
            return super::close_tab_admitted(&app, id, Some(admitted)).map(|_| Value::Null);
        }
        if let Binding::LiveDocument(document) = binding {
            let (id, js) = match self {
                Self::Open { id, url } => return tab_open_document(&app, &id, &url, document).map(|_| Value::Null),
                Self::Back { id } => (id, "if (history.length > 1) { history.back(); }"),
                Self::Forward { id } => (id, "history.forward();"),
                Self::Reload { id } => (id, "location.reload();"),
                _ => return Err("TAB_MUTATION_INSTANCE_STALE".into()),
            };
            app.state::<TabManagerState>().eval_navigation_document(&id, document, js)
                .map_err(|error| error.to_string())?;
            state.hibernated_scroll.lock().unwrap().remove(&id);
            return Ok(Value::Null);
        }
        if let Binding::Isolated(expected) = binding {
            if let Self::Activate { ref id } = self {
                return tab_activate_admitted(webview, app.clone(), id.clone(), Some(*expected)).map(|_| Value::Null);
            }
        }
        if let Binding::IsolatedDocument(document) = binding {
            return match self {
                Self::Open { id, url } => tab_open_admitted(webview, app.clone(), id, url, Some(*document)).map(|_| Value::Null),
                Self::Back { id } => super::isolated_tabs::eval_bound(&app, &id, "if (history.length > 1) { history.back(); }", *document).map(|_| Value::Null),
                Self::Forward { id } => super::isolated_tabs::eval_bound(&app, &id, "history.forward();", *document).map(|_| Value::Null),
                Self::Reload { id } => super::isolated_tabs::eval_bound(&app, &id, "location.reload();", *document).map(|_| Value::Null),
                Self::ParkRenderer { id, url, .. } => {
                    let _transition = reserve_tab_transition(&id)?;
                    super::isolated_tabs::park_admitted(&app, &id, &url, *document).map(|_| Value::Null)
                }
                _ => Err("TAB_MUTATION_INSTANCE_STALE".into()),
            };
        }
        drop(state);
        match self {
            Self::New { url } => serde_json::to_value(tab_new_impl(webview, app, url)?)
                .map_err(|_| "TAB_RECEIPT_SERIALIZATION_FAILED".into()),
            Self::Close { id } => tab_close_impl(webview, app, id).map(|_| Value::Null),
            Self::Activate { id } => tab_activate_impl(webview, app, id).map(|_| Value::Null),
            Self::Open { id, url } => tab_open_impl(webview, app, id, url).map(|_| Value::Null),
            Self::Back { id } => tab_go_back_impl(webview, app, id).map(|_| Value::Null),
            Self::Forward { id } => tab_go_forward_impl(webview, app, id).map(|_| Value::Null),
            Self::Reload { id } => tab_reload_impl(webview, app, id).map(|_| Value::Null),
            Self::ParkRenderer { .. } => Err("TAB_RENDERER_PARK_STALE".into()),
            Self::ReopenRenderer { id, url, .. } => {
                let Binding::IsolatedAbsent(epoch) = binding else {
                    return Err("TAB_RENDERER_REOPEN_STALE".into());
                };
                let _transition = reserve_tab_transition(&id)?;
                serde_json::to_value(super::isolated_tabs::reopen_absent(&app, &id, &url, *epoch)?)
                    .map_err(|_| "TAB_RENDERER_REOPEN_PARTIAL: receipt serialization".into())
            }
            Self::RetireRenderer { id, pid, spawn_epoch, .. } => {
                let index = super::isolated_tabs::route(&id).ok_or("TAB_RENDERER_ROUTE_INVALID")?;
                let epoch = spawn_epoch.parse::<u64>().map_err(|_| "TAB_RENDERER_RECOVERY_INVALID_TARGET")?;
                let _transition = reserve_tab_transition(&id)?;
                let state = app.state::<crate::AppState>();
                state.grid_manager.retire_bound_tab_renderer(index, (pid, epoch))
                    .map_err(|error| if error.contains("CLEANUP_PENDING") {
                        format!("GRID_REQUEST_OUTCOME_UNKNOWN: {error}")
                    } else { error })?;
                state.hibernated_tabs.lock().unwrap().remove(&id);
                state.hibernated_scroll.lock().unwrap().remove(&id);
                Ok(Value::Null)
            }
        }
    }
}

// A model may propose only these typed operations. No script, DOM write,
// arbitrary native command, recovery permission or hidden file navigation.
fn agent_action(input: &Value) -> Result<Action, String> {
    let action: Action = serde_json::from_value(input.clone())
        .map_err(|_| "AGENT_BROWSER_ARGUMENT_INVALID")?;
    match &action {
        Action::New { url } | Action::Open { url, .. } => {
            let parsed = url::Url::parse(url).map_err(|_| "AGENT_BROWSER_ADDRESS_DENIED")?;
            if url.len() > 8192 || !matches!(parsed.scheme(), "http" | "https")
                || parsed.host().is_none() || !parsed.username().is_empty() || parsed.password().is_some()
                || url.chars().any(char::is_control) {
                return Err("AGENT_BROWSER_ADDRESS_DENIED".into());
            }
        }
        Action::Close { .. } | Action::Back { .. } | Action::Forward { .. } | Action::Reload { .. } => {}
        _ => return Err("AGENT_BROWSER_ACTION_DENIED".into()),
    }
    if action.target().is_some_and(|id| !super::is_tab_label(id)) {
        return Err("AGENT_BROWSER_ARGUMENT_INVALID".into());
    }
    Ok(action)
}
pub(crate) fn validate_agent_browser_action(input: &Value) -> Result<(), String> {
    agent_action(input).map(|_| ())
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Prepared,
    Running,
    CancelRequested,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}
impl Phase {
    fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Receipt {
    ticket: String,
    client_id: String,
    command: &'static str,
    phase: Phase,
    value: Option<Value>,
    error: Option<String>,
}
#[derive(Clone)]
enum Binding {
    New,
    Live(TabInstance),
    LiveDocument(CaptureDocument),
    Isolated(Option<(u32, u64)>),
    IsolatedAbsent(u64),
    IsolatedDocument(crate::grid_ipc::RendererBinding),
    Hibernated,
}
impl Binding {
    fn capture(app: &AppHandle, action: &Action) -> Result<Self, String> {
        let Some(id) = action.target() else {
            return Ok(Self::New);
        };
        let state = app.state::<crate::AppState>();
        if !state
            .tabs
            .try_lock()
            .map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE")?
            .contains_key(id)
        {
            return Err("TAB_NOT_FOUND".into());
        }
        if let Some(index) = super::isolated_tabs::route(id) {
            let owner = state.grid_manager.tab_renderer_owner(index)?;
            if let Action::RetireRenderer { pid, spawn_epoch, .. } = action {
                let expected = spawn_epoch.parse::<u64>().map_err(|_| "TAB_RENDERER_RECOVERY_INVALID_TARGET")?;
                if owner != Some((*pid, expected)) {
                    return Err("TAB_RENDERER_RECOVERY_STALE".into());
                }
            }
            if let Action::ParkRenderer { pid, spawn_epoch, url, renderer_generation, navigation_revision, .. } = action {
                let epoch = spawn_epoch.parse::<u64>().map_err(|_| "TAB_RENDERER_PARK_INVALID")?;
                if owner != Some((*pid, epoch))
                    || state.active_tab.try_lock().map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE")?.as_deref() == Some(id)
                    || !state.tabs.try_lock().map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE")?
                        .get(id).is_some_and(|tab| tab.url == *url) {
                    return Err("TAB_RENDERER_PARK_STALE".into());
                }
                let binding = state.grid_manager.renderer_binding(index)?;
                if binding.owner != (*pid, epoch)
                    || binding.generation.to_string() != *renderer_generation
                    || binding.navigation_revision.to_string() != *navigation_revision {
                    return Err("TAB_RENDERER_PARK_STALE".into());
                }
                // Never take a second binding after checking the reviewed one.
                return Ok(Self::IsolatedDocument(binding));
            }
            if let Action::ReopenRenderer { review_owner_epoch, review_owner_sequence, absence_epoch, .. } = action {
                let epoch = absence_epoch.parse::<u64>().map_err(|_| "TAB_RENDERER_REOPEN_INVALID")?;
                // The original user-visible absence review, not a fresh
                // inspect issued by the mutation transport, grants admission.
                {
                    let control = self::owner()?;
                    if control.epoch != *review_owner_epoch || control.next_sequence.to_string() != *review_owner_sequence {
                        return Err("TAB_RENDERER_REOPEN_STALE".into());
                    }
                }
                if owner.is_some()
                    || state.grid_manager.tab_renderer_absence_epoch(index)? != epoch
                    || state.hibernated_tabs.try_lock().map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE")?.contains(id) {
                    return Err("TAB_RENDERER_REOPEN_STALE".into());
                }
                return Ok(Self::IsolatedAbsent(epoch));
            }
            if owner.is_none() && !matches!(action, Action::Activate { .. } | Action::Close { .. }) {
                return Err("TAB_RENDERER_RECOVERY_REQUIRED".into());
            }
            if action.is_document_mutation() {
                let binding = state.grid_manager.renderer_binding(index)?;
                if Some(binding.owner) != owner {
                    return Err("TAB_MUTATION_INSTANCE_STALE".into());
                }
                return Ok(Self::IsolatedDocument(binding));
            }
            return Ok(Self::Isolated(owner));
        }
        if state
            .hibernated_tabs
            .try_lock()
            .map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE")?
            .contains(id)
        {
            if action.is_document_mutation() {
                return Err("TAB_RENDERER_HIBERNATED_ACTIVATE_FIRST".into());
            }
            return Ok(Self::Hibernated);
        }
        if action.is_document_mutation() {
            return app.state::<TabManagerState>().capture_document(&id.to_owned())
                .map(Self::LiveDocument)
                .map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE".into());
        }
        app.state::<TabManagerState>()
            .instance(&id.to_owned())
            .map(Self::Live)
            .map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE".into())
    }
    fn current(&self, app: &AppHandle, action: &Action) -> bool {
        match (self, action.target()) {
            (Self::New, None) => true,
            (Self::Live(instance), Some(id)) => app
                .state::<TabManagerState>()
                .is_current_instance(&id.to_owned(), instance),
            (Self::LiveDocument(document), Some(id)) => app
                .state::<TabManagerState>()
                .is_current_capture_document(&id.to_owned(), document),
            (Self::Isolated(expected), Some(id)) => {
                super::isolated_tabs::route(id).is_some_and(|index| {
                    let state = app.state::<crate::AppState>();
                    let exists = state.tabs.lock().unwrap().contains_key(id);
                    exists && state.grid_manager.tab_renderer_owner(index).is_ok_and(|owner| owner == *expected)
                })
            }
            (Self::IsolatedAbsent(epoch), Some(id)) => {
                super::isolated_tabs::route(id).is_some_and(|index| {
                    let state = app.state::<crate::AppState>();
                    let exists = state.tabs.lock().unwrap().contains_key(id);
                    exists && !state.hibernated_tabs.lock().unwrap().contains(id)
                        && state.grid_manager.tab_renderer_absence_epoch(index).is_ok_and(|current| current == *epoch)
                })
            }
            (Self::IsolatedDocument(binding), Some(id)) => {
                super::isolated_tabs::route(id).is_some_and(|index| {
                    let state = app.state::<crate::AppState>();
                    let exists = state.tabs.lock().unwrap().contains_key(id);
                    exists && state.grid_manager.renderer_binding(index).is_ok_and(|current| current == *binding)
                })
            }
            (Self::Hibernated, Some(id)) => app
                .state::<crate::AppState>()
                .hibernated_tabs
                .lock()
                .is_ok_and(|tabs| tabs.contains(id)),
            _ => false,
        }
    }
}
struct Slot {
    receipt: Receipt,
    action: Option<Action>,
    binding: Option<Binding>,
    since: std::time::Instant,
    entered: bool,
    // Agent-owned prepared work may enter only after the Coding durable
    // effect fence. Main-window Start must not bypass that owner.
    agent_owned: bool,
}
impl Slot {
    /// A queued worker is revocable only under this same Owner lock.
    /// Once it has entered, cancellation is a request, never a receipt.
    fn enter(&mut self) -> bool {
        if self.receipt.phase != Phase::Running || self.entered {
            return false;
        }
        self.entered = true;
        self.since = Instant::now();
        true
    }
    fn cancel(&mut self) {
        match self.receipt.phase {
            Phase::Prepared => self.receipt.phase = Phase::Cancelled,
            Phase::Running if !self.entered => {
                self.receipt.phase = Phase::Cancelled;
                self.receipt.error = Some("TAB_MUTATION_CANCELLED_BEFORE_EXECUTION".into());
            }
            Phase::Running => self.receipt.phase = Phase::CancelRequested,
            _ => {}
        }
    }
}
// A stuck mutation with a responsive shell is not a GTK heartbeat failure.
// Only an explicit, short-lived request bound to the current native Owner
// may authorize the existing supervisor to retire the whole process domain.
const DOMAIN_STALL: Duration = Duration::from_secs(60);
const RECOVERY_INTENT_TTL: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryTarget {
    epoch: String,
    sequence: u64,
    ticket: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DomainRecoveryIntent {
    target: RecoveryTarget,
    issued: Instant,
}

struct Owner {
    slot: Option<Slot>,
    last_ack: Option<String>,
    epoch: String,
    next_sequence: u64,
    internal_active: bool,
    internal_started: Option<Instant>,
    recovery_intent: Option<DomainRecoveryIntent>,
    recovery_claimed: bool,
}
impl Default for Owner {
    fn default() -> Self {
        Self {
            slot: None,
            last_ack: None,
            epoch: uuid::Uuid::new_v4().to_string(),
            next_sequence: 1,
            internal_active: false,
            internal_started: None,
            recovery_intent: None,
            recovery_claimed: false,
        }
    }
}
impl Owner {
    fn advance_sequence(&mut self) -> Result<(), String> {
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or("TAB_MUTATION_SEQUENCE_EXHAUSTED")?;
        Ok(())
    }

    fn admit_background(&mut self) -> Result<(), String> {
        self.expire_prepared();
        if self.slot.is_some() || self.internal_active || self.recovery_claimed {
            return Err("TAB_MUTATION_OWNER_BUSY".into());
        }
        // A background operation changes the same native tab state as a
        // user mutation. Even a completed operation between two inspect
        // calls must invalidate an older reconciliation snapshot.
        self.advance_sequence()?;
        self.internal_active = true;
        self.internal_started = Some(Instant::now());
        Ok(())
    }

    fn recovery_target(&self, now: Instant) -> Option<RecoveryTarget> {
        if self.recovery_claimed {
            return None;
        }
        let ticket = match &self.slot {
            Some(slot)
                if slot.receipt.phase == Phase::Unknown
                    || (slot.entered
                        && matches!(slot.receipt.phase, Phase::Running | Phase::CancelRequested)
                        && now.saturating_duration_since(slot.since) >= DOMAIN_STALL) =>
            {
                Some(slot.receipt.ticket.clone())
            }
            None if self.internal_active
                && self.internal_started.is_some_and(|started| {
                    now.saturating_duration_since(started) >= DOMAIN_STALL
                }) =>
            {
                None
            }
            _ => return None,
        };
        Some(RecoveryTarget {
            epoch: self.epoch.clone(),
            sequence: self.next_sequence,
            ticket,
        })
    }

    fn local_renderer_recovery(&self) -> Option<(u32, (u32, u64))> {
        let slot = self.slot.as_ref()?;
        if self.recovery_claimed || slot.receipt.phase != Phase::Unknown {
            return None;
        }
        let expected = match slot.binding.as_ref()? {
            Binding::Isolated(Some(expected)) => *expected,
            Binding::IsolatedDocument(binding) => binding.owner,
            _ => return None,
        };
        let id = slot.action.as_ref()?.target()?;
        Some((super::isolated_tabs::route(id)?, expected))
    }

    fn valid_intent(&self, intent: &DomainRecoveryIntent, now: Instant) -> bool {
        now.saturating_duration_since(intent.issued) < RECOVERY_INTENT_TTL
            && self.recovery_target(now).as_ref() == Some(&intent.target)
    }

    fn request_recovery(&mut self, target: RecoveryTarget, now: Instant) -> Result<(), String> {
        if self.recovery_target(now).as_ref() != Some(&target) {
            return Err("TAB_DOMAIN_RECOVERY_NOT_ELIGIBLE".into());
        }
        // Do not refresh an existing authorization on a duplicated IPC.
        if self
            .recovery_intent
            .as_ref()
            .is_some_and(|intent| self.valid_intent(intent, now))
        {
            return Ok(());
        }
        self.recovery_intent = Some(DomainRecoveryIntent {
            target,
            issued: now,
        });
        Ok(())
    }

    fn claim_recovery(&mut self, intent: &DomainRecoveryIntent, now: Instant) -> bool {
        if self.recovery_intent.as_ref() != Some(intent) || !self.valid_intent(intent, now) {
            return false;
        }
        self.recovery_intent = None;
        self.recovery_claimed = true;
        true
    }

    fn matches_view(&self, view: &OwnerView) -> bool {
        self.epoch == view.epoch
            && self.next_sequence.to_string() == view.next_sequence
            && self.internal_active == view.internal_active
            && self.last_ack == view.last_ack
            && self
                .slot
                .as_ref()
                .map(|s| (&s.receipt.ticket, s.receipt.phase))
                == view.receipt.as_ref().map(|r| (&r.ticket, r.phase))
    }

    fn expire_prepared(&mut self) {
        if let Some(slot) = &mut self.slot {
            if slot.since.elapsed() >= std::time::Duration::from_secs(30)
                && (slot.receipt.phase == Phase::Prepared
                    || (slot.receipt.phase == Phase::Running && !slot.entered))
            {
                let prepared = slot.receipt.phase == Phase::Prepared;
                slot.cancel();
                slot.receipt.error = Some(
                    if prepared {
                        "TAB_MUTATION_PREPARE_EXPIRED"
                    } else {
                        "TAB_MUTATION_WORKER_QUEUE_EXPIRED"
                    }
                    .into(),
                );
            }
        }
    }
}
/// Only background lifecycle changes acquire this guard; routine geometry
/// updates keep the existing per-resource layout reservation. Never wait on
/// the owner while holding a native lifecycle or GTK lock.
pub(super) struct InternalMutation;
impl Drop for InternalMutation {
    fn drop(&mut self) {
        if let Ok(mut state) = owner() {
            state.internal_active = false;
            state.internal_started = None;
        }
    }
}
pub(super) fn reserve_internal() -> Result<InternalMutation, String> {
    let mut state = owner()?;
    state.admit_background()?;
    Ok(InternalMutation)
}
#[derive(Clone, Debug, Serialize)]
struct RendererRecoveryTarget {
    id: String,
    pid: u32,
    spawn_epoch: String,
}

#[derive(Clone, Debug, Serialize)]
struct RendererParkTarget {
    id: String,
    pid: u32,
    spawn_epoch: String,
    renderer_generation: String,
    navigation_revision: String,
}

#[derive(Clone, Debug, Serialize)]
struct RendererReopenTarget {
    id: String,
    review_owner_epoch: String,
    review_owner_sequence: String,
    absence_epoch: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OwnerView {
    epoch: String,
    next_sequence: String,
    receipt: Option<Receipt>,
    last_ack: Option<String>,
    internal_active: bool,
    active_known: bool,
    active_tab_id: Option<String>,
    domain_recovery_available: bool,
    recovery_scope: Option<&'static str>,
    renderer_recovery_target: Option<RendererRecoveryTarget>,
    renderer_park_target: Option<RendererParkTarget>,
    renderer_reopen_target: Option<RendererReopenTarget>,
    isolated_renderer_mode: bool,
    renderer_state: Option<&'static str>,
    // Advisory native counts, not per-transfer state or filesystem access.
    renderer_download: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    download_inventory: Option<Value>,
}
fn snapshot(app: &AppHandle) -> Result<OwnerView, String> {
    let mut view = {
        let mut state = owner()?;
        state.expire_prepared();
        OwnerView {
            epoch: state.epoch.clone(),
            next_sequence: state.next_sequence.to_string(),
            receipt: state.slot.as_ref().map(|s| s.receipt.clone()),
            last_ack: state.last_ack.clone(),
            internal_active: state.internal_active,
            active_known: false,
            active_tab_id: None,
            domain_recovery_available: state.local_renderer_recovery().is_some()
                || (supervisor_ready() && state.recovery_target(Instant::now()).is_some()),
            recovery_scope: if state.local_renderer_recovery().is_some() { Some("renderer") }
                else if supervisor_ready() && state.recovery_target(Instant::now()).is_some() { Some("application") }
                else { None },
            renderer_recovery_target: None,
            renderer_park_target: None,
            renderer_reopen_target: None,
            isolated_renderer_mode: super::isolated_tabs::mode_enabled(),
            renderer_state: None,
            renderer_download: None,
            download_inventory: None,
        }
    };
    // Do not hold the Owner mutex while observing app metadata.
    if let Ok(active) = app.state::<crate::AppState>().active_tab.try_lock() {
        view.active_known = true;
        view.active_tab_id = active.clone();
    }
    // Validate the split snapshot after releasing the app metadata lock.
    // A native admission or phase transition during collection cannot be
    // represented as a consistent active-tab observation.
    let state = owner()?;
    if view.internal_active || !state.matches_view(&view) {
        view.active_known = false;
        view.active_tab_id = None;
    }
    if !state.matches_view(&view)
        || (state.local_renderer_recovery().is_none()
            && (!supervisor_ready() || state.recovery_target(Instant::now()).is_none()))
    {
        view.domain_recovery_available = false;
        view.recovery_scope = None;
    }
    Ok(view)
}
fn notify(app: &AppHandle, ticket: &str) {
    let receipt = owner().ok().and_then(|state| {
        state
            .slot
            .as_ref()
            .filter(|slot| slot.receipt.ticket == ticket)
            .map(|slot| slot.receipt.clone())
    });
    if let Some(receipt) = receipt {
        let _ = app.emit_to("main", "browser-tab-mutation-receipt", receipt);
    }
}
static OWNER: LazyLock<Mutex<Owner>> = LazyLock::new(|| Mutex::new(Owner::default()));

fn owner() -> Result<std::sync::MutexGuard<'static, Owner>, String> {
    OWNER
        .lock()
        .map_err(|_| "TAB_MUTATION_OWNER_UNKNOWN".into())
}
fn recovery_scope_matches(requested: Option<&str>, renderer_only: bool) -> bool {
    // Historical clients only offered a whole-app confirmation. Require a
    // fresh explicit scope for a newly available renderer-only recovery.
    requested.unwrap_or("application") == if renderer_only { "renderer" } else { "application" }
}

fn supervisor_ready() -> bool {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        crate::native_supervisor::domain_recovery_ready()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

pub(crate) fn pending_domain_recovery() -> Option<DomainRecoveryIntent> {
    let mut state = OWNER.try_lock().ok()?;
    let intent = state.recovery_intent.clone()?;
    if state.valid_intent(&intent, Instant::now()) {
        Some(intent)
    } else {
        state.recovery_intent = None;
        None
    }
}

/// While the supervisor holds this claim, no replacement tab operation can
/// enter. A terminal old worker may still settle. On transport loss the
/// guard releases admission, but never fabricates a completion receipt.
pub(crate) struct DomainRecoveryClaim;

impl Drop for DomainRecoveryClaim {
    fn drop(&mut self) {
        if let Ok(mut state) = owner() {
            state.recovery_claimed = false;
        }
    }
}

pub(crate) fn claim_domain_recovery(intent: &DomainRecoveryIntent) -> Option<DomainRecoveryClaim> {
    let mut state = OWNER.try_lock().ok()?;
    // A refused claim must not construct a Drop guard while OWNER is held.
    if !state.claim_recovery(intent, Instant::now()) {
        return None;
    }
    drop(state);
    Some(DomainRecoveryClaim)
}

// One ephemeral native review, never a serialized or model-supplied grant.
// Exact encrypted Coding proposal identity is rechecked at consumption.
struct AgentBrowserReview {
    token: uuid::Uuid,
    operation: uuid::Uuid,
    workspace: String,
    action: Action,
    binding: Binding,
    close_document: Option<Binding>,
    epoch: String,
    sequence: u64,
    expires: Instant,
}
static AGENT_BROWSER_REVIEW: Mutex<Option<AgentBrowserReview>> = Mutex::new(None);

pub(crate) fn review_agent_browser_action(
    app: &AppHandle, webview: &Webview, workspace: String, operation: uuid::Uuid,
    input: &Value,
) -> Result<Value, String> {
    let action = agent_action(input)?;
    action.authorize(webview)?;
    let (epoch, sequence) = {
        let mut state = owner()?;
        state.expire_prepared();
        if state.slot.is_some() || state.internal_active || state.recovery_claimed {
            return Err("TAB_MUTATION_OWNER_BUSY".into());
        }
        (state.epoch.clone(), state.next_sequence)
    };
    let binding = Binding::capture(app, &action)?;
    // Closing must also bind the live reviewed document. Do not grant
    // closing a sleeping/absent tab or a replacement document.
    let close_document = if let Action::Close { id } = &action {
        Some(Binding::capture(app, &Action::Reload { id: id.clone() })?)
    } else { None };
    let origin = action.target().and_then(|id| {
        let state = app.state::<crate::AppState>();
        let tabs = state.tabs.try_lock().ok()?;
        let address = &tabs.get(id)?.url;
        let url = url::Url::parse(address).ok()?;
        Some(url.origin().ascii_serialization())
    });
    if !binding.current(app, &action)
        || close_document.as_ref().is_some_and(|document| !document.current(app, &action)) {
        return Err("AGENT_BROWSER_REVIEW_STALE".into());
    }
    {
        let state = owner()?;
        if state.epoch != epoch || state.next_sequence != sequence
            || state.slot.is_some() || state.internal_active || state.recovery_claimed {
            return Err("AGENT_BROWSER_REVIEW_STALE".into());
        }
    }
    let token = uuid::Uuid::new_v4();
    let command = action.command();
    *AGENT_BROWSER_REVIEW.lock().map_err(|_| "AGENT_BROWSER_REVIEW_BUSY")? =
        Some(AgentBrowserReview {
            token, operation, workspace, action, binding, close_document, epoch, sequence,
            expires: Instant::now() + Duration::from_secs(90),
        });
    Ok(serde_json::json!({"token":token,"operation_id":operation,"command":command,
        "input":input,"observed_origin":origin,"expires_seconds":90,
        "scope":"native_host_dispatch_only"}))
}

fn cancel_agent_prepared(webview: &Webview, app: &AppHandle, ticket: &str) {
    // No native operation entered. Only a terminal cancelled slot may be
    // acknowledged; an unexpected phase remains for normal recovery.
    let _ = tab_mutation_control(webview.clone(), app.clone(), Request::Cancel { ticket: ticket.into() });
    let _ = tab_mutation_control(webview.clone(), app.clone(), Request::Ack { ticket: ticket.into() });
}

/// Called only by the durable Coding execution adapter, on its owned worker.
/// Browser Owner and Coding Owner both retain the effect until settlement.
pub(crate) fn execute_agent_browser_action(
    app: &AppHandle, webview: &Webview, workspace: &str, operation: uuid::Uuid,
    input: &Value, review_token: &str, cancelled: &dyn Fn() -> bool,
    admit: impl FnOnce() -> Result<(), String>,
) -> Result<Value, String> {
    let action = agent_action(input)?;
    action.authorize(webview)?;
    let token = uuid::Uuid::parse_str(review_token).map_err(|_| "AGENT_BROWSER_REVIEW_REQUIRED")?;
    let review = {
        let mut slot = AGENT_BROWSER_REVIEW.lock().map_err(|_| "AGENT_BROWSER_REVIEW_BUSY")?;
        if !slot.as_ref().is_some_and(|r| r.token == token && r.operation == operation
            && r.workspace == workspace && r.action == action && Instant::now() < r.expires) {
            return Err("AGENT_BROWSER_REVIEW_STALE".into());
        }
        slot.take().ok_or("AGENT_BROWSER_REVIEW_STALE")?
    };
    let current = || !cancelled() && Instant::now() < review.expires
        && review.binding.current(app, &action)
        && review.close_document.as_ref().is_none_or(|document| document.current(app, &action));
    if !current() { return Err("AGENT_BROWSER_REVIEW_STALE".into()); }
    let ticket = uuid::Uuid::new_v4().to_string();
    {
        let mut state = owner()?;
        state.expire_prepared();
        if state.epoch != review.epoch || state.next_sequence != review.sequence
            || state.slot.is_some() || state.internal_active || state.recovery_claimed {
            return Err("AGENT_BROWSER_REVIEW_STALE".into());
        }
        state.advance_sequence()?;
        state.slot = Some(Slot {
            receipt: Receipt { ticket: ticket.clone(), client_id: operation.to_string(),
                command: action.command(), phase: Phase::Prepared, value: None, error: None },
            action: Some(action.clone()), binding: Some(review.binding.clone()),
            since: Instant::now(), entered: false,
            agent_owned: true,
        });
    }
    if !current() {
        cancel_agent_prepared(webview, app, &ticket);
        return Err("AGENT_BROWSER_REVIEW_STALE".into());
    }
    // Write Coding's authenticated dispatch intent before the first effect.
    if let Err(error) = admit() {
        cancel_agent_prepared(webview, app, &ticket);
        return Err(error);
    }
    if !current() {
        cancel_agent_prepared(webview, app, &ticket);
        return Err("AGENT_BROWSER_REVIEW_STALE".into());
    }
    let entered = {
        let mut state = owner()?;
        state.expire_prepared();
        if state.recovery_claimed { false } else {
            let slot = matching(&mut state, &ticket)?;
            if slot.receipt.phase != Phase::Prepared { false } else {
                slot.receipt.phase = Phase::Running;
                slot.enter()
            }
        }
    };
    if !entered {
        cancel_agent_prepared(webview, app, &ticket);
        return Err("AGENT_BROWSER_REVIEW_STALE".into());
    }
    let mut guard = WorkerGuard { ticket: ticket.clone(), completed: false };
    let command = action.command();
    let id = action.target().map(str::to_owned);
    let result = if current() {
        action.execute(webview.clone(), app.clone(), &review.binding)
    } else {
        Err("TAB_MUTATION_INSTANCE_STALE".into())
    };
    // Any failure after entry is conservatively unknown. Do not release a
    // potentially affected renderer or allow the Agent to retry the action.
    match result {
        Ok(value) => {
            let created_id = value.get("id").and_then(Value::as_str).map(str::to_owned);
            finish(&ticket, Ok(value));
            guard.completed = true;
            notify(app, &ticket);
            // The main browser store reconciles/ACKs the terminal receipt.
            // The model receives only a host dispatch observation.
            Ok(serde_json::json!({"status":"succeeded","result":{
                "command":command,"tab_id":id.or(created_id),
                "host_dispatched":true,"page_or_remote_effect_confirmed":false}}))
        }
        Err(_) => {
            unknown(&ticket);
            guard.completed = true;
            notify(app, &ticket);
            Err("AGENT_BROWSER_NATIVE_OUTCOME_UNKNOWN".into())
        }
    }
}

fn matching<'a>(state: &'a mut Owner, ticket: &str) -> Result<&'a mut Slot, String> {
    state
        .slot
        .as_mut()
        .filter(|s| s.receipt.ticket == ticket)
        .ok_or_else(|| "TAB_MUTATION_STALE_TICKET".into())
}
fn completion_unknown(error: &str) -> bool {
    error.contains("GRID_REQUEST_OUTCOME_UNKNOWN") || error.contains("TAB_RENDERER_CLEANUP_PENDING") || matches!(
        error,
        "TAB_CREATE_CALLBACK_LOST" | "TAB_ACTIVATION_CALLBACK_LOST"
    )
}
fn finish(ticket: &str, result: Result<Value, String>) {
    if let Ok(mut state) = owner() {
        if let Ok(slot) = matching(&mut state, ticket) {
            if !matches!(slot.receipt.phase, Phase::Running | Phase::CancelRequested) {
                return;
            }
            match result {
                Ok(value) => {
                    slot.receipt.phase = Phase::Succeeded;
                    slot.receipt.value = Some(value);
                }
                Err(error) => {
                    slot.receipt.phase = if completion_unknown(&error) {
                        Phase::Unknown
                    } else {
                        Phase::Failed
                    };
                    slot.receipt.error = Some(error);
                }
            }
        }
    }
}
fn unknown(ticket: &str) {
    if let Ok(mut state) = owner() {
        if let Ok(slot) = matching(&mut state, ticket) {
            if !slot.receipt.phase.terminal() {
                slot.receipt.phase = Phase::Unknown;
            }
        }
    }
}
struct WorkerGuard {
    ticket: String,
    completed: bool,
}
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if !self.completed {
            unknown(&self.ticket);
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Request {
    Prepare {
        client_id: String,
        epoch: String,
        sequence: String,
        action: Action,
    },
    Start {
        ticket: String,
    },
    Poll {
        ticket: String,
    },
    Cancel {
        ticket: String,
    },
    Ack {
        ticket: String,
    },
    Inspect,
    InspectRenderer { id: String },
    InspectDownloads,
    SampleDownload { id: String, pid: u32, spawn_epoch: String },
    Fence {
        epoch: String,
        sequence: String,
    },
    RecoverDomain {
        epoch: String,
        sequence: String,
        ticket: Option<String>,
        confirmed: bool,
        #[serde(default)]
        scope: Option<String>,
    },
}
static DOWNLOAD_SAMPLE_LANE: crate::shared::blocking::BlockingLane =
    crate::shared::blocking::BlockingLane::new(2, "TAB_DOWNLOAD_SAMPLE_BUSY", "TAB_DOWNLOAD_SAMPLE_WORKER_FAILED");

#[tauri::command]
pub(crate) async fn tab_mutation(
    webview: Webview,
    app: AppHandle,
    request: Request,
) -> Result<OwnerView, String> {
    match request {
        Request::SampleDownload { id, pid, spawn_epoch } => {
            crate::shared::native_legacy_policy::require_main(&webview, "tab_list")?;
            crate::shared::native_legacy_policy::bound_text("tab_list", "id", &id)?;
            let index = super::isolated_tabs::route(&id).ok_or("TAB_RENDERER_ROUTE_INVALID")?;
            let epoch = spawn_epoch.parse::<u64>().ok().filter(|v| *v > 0 && v.to_string() == spawn_epoch)
                .ok_or("TAB_DOWNLOAD_SAMPLE_STALE")?;
            if pid == 0 { return Err("TAB_DOWNLOAD_SAMPLE_STALE".into()); }
            let binding = app.state::<crate::AppState>().grid_manager.renderer_binding(index)?;
            if binding.owner != (pid, epoch) { return Err("TAB_DOWNLOAD_SAMPLE_STALE".into()); }
            DOWNLOAD_SAMPLE_LANE.run(move || {
                let state = app.state::<crate::AppState>();
                if state.shutdown_requested.load(std::sync::atomic::Ordering::Acquire)
                    || !state.tabs.try_lock().map_err(|_| "TAB_DOWNLOAD_SAMPLE_BUSY")?.contains_key(&id) {
                    return Err("TAB_DOWNLOAD_SAMPLE_STALE".into());
                }
                if !state.grid_manager.sample_tab_downloads(binding, 800)? {
                    return Err("TAB_DOWNLOAD_SAMPLE_UNCONFIRMED".into());
                }
                let inventory = state.grid_manager.tab_download_inventory()?;
                let current = inventory["live"].as_array().is_some_and(|rows| rows.iter().any(|row|
                    row["id"].as_str() == Some(id.as_str())
                    && row["pid"].as_u64() == Some(u64::from(pid))
                    && row["spawn_epoch"].as_str() == Some(spawn_epoch.as_str())
                    && row["renderer_generation"].as_str() == Some(binding.generation.to_string().as_str())
                    && row["connection_current"].as_bool() == Some(true)));
                if !current { return Err("TAB_DOWNLOAD_SAMPLE_STALE".into()); }
                let mut view = snapshot(&app)?;
                view.download_inventory = Some(inventory);
                Ok(view)
            }).await
        }
        other => tab_mutation_control(webview, app, other),
    }
}

fn tab_mutation_control(
    webview: Webview,
    app: AppHandle,
    request: Request,
) -> Result<OwnerView, String> {
    if webview.label() != "main" {
        return Err("NATIVE_SOURCE_DENIED".into());
    }
    match request {
        Request::Prepare {
            client_id,
            epoch,
            sequence,
            action,
        } => {
            action.authorize(&webview)?;
            if client_id.len() != 36 || uuid::Uuid::parse_str(&client_id).is_err() {
                return Err("TAB_MUTATION_INVALID_ID".into());
            }
            // A reservation is held across instance capture, but the mutex
            // is not. Background lifecycle changes cannot cross this gap.
            let reserve = {
                let mut state = owner()?;
                state.expire_prepared();
                if state.recovery_claimed {
                    return Err("TAB_MUTATION_OWNER_BUSY".into());
                }
                if epoch != state.epoch {
                    return Err("TAB_MUTATION_STALE_EPOCH".into());
                }
                if let Some(slot) = &state.slot {
                    if slot.receipt.client_id != client_id || slot.action.as_ref() != Some(&action)
                    {
                        return Err("TAB_MUTATION_OWNER_BUSY".into());
                    }
                    None
                } else {
                    if state.internal_active {
                        return Err("TAB_MUTATION_OWNER_BUSY".into());
                    }
                    if sequence != state.next_sequence.to_string() {
                        return Err("TAB_MUTATION_STALE_SEQUENCE".into());
                    }
                    state.internal_active = true;
                    state.internal_started = Some(Instant::now());
                    Some(InternalMutation)
                }
            };
            if let Some(reservation) = reserve {
                let binding = Binding::capture(&app, &action)?;
                {
                    let mut state = owner()?;
                    if state.slot.is_some()
                        || !state.internal_active
                        || state.recovery_claimed
                        || epoch != state.epoch
                        || sequence != state.next_sequence.to_string()
                    {
                        return Err("TAB_MUTATION_STALE_SEQUENCE".into());
                    }
                    state.advance_sequence()?;
                    let receipt = Receipt {
                        ticket: uuid::Uuid::new_v4().to_string(),
                        client_id,
                        command: action.command(),
                        phase: Phase::Prepared,
                        value: None,
                        error: None,
                    };
                    state.slot = Some(Slot {
                        receipt,
                        action: Some(action),
                        binding: Some(binding),
                        since: std::time::Instant::now(),
                        entered: false,
            agent_owned: false,
                    });
                }
                drop(reservation);
            }
        }
        Request::Start { ticket } => {
            let work = {
                let mut state = owner()?;
                state.expire_prepared();
                if state.recovery_claimed {
                    return Err("TAB_MUTATION_OWNER_BUSY".into());
                }
                let slot = matching(&mut state, &ticket)?;
                if slot.agent_owned {
                    return Err("AGENT_BROWSER_NATIVE_OWNER_REQUIRED".into());
                }
                if slot.receipt.phase != Phase::Prepared {
                    None
                } else {
                    let action = slot.action.clone().ok_or("TAB_MUTATION_INVALID_ACTION")?;
                    let binding = slot.binding.clone().ok_or("TAB_MUTATION_INVALID_ACTION")?;
                    slot.receipt.phase = Phase::Running;
                    slot.since = std::time::Instant::now();
                    Some((action, binding))
                }
            };
            if let Some((action, binding)) = work {
                let worker_ticket = ticket.clone();
                let worker_app = app.clone();
                let spawned = std::thread::Builder::new()
                    .name("browseros-tab-owner".into())
                    .spawn(move || {
                        let mut guard = WorkerGuard {
                            ticket: worker_ticket.clone(),
                            completed: false,
                        };
                        let entered = {
                            let Ok(mut state) = owner() else {
                                return;
                            };
                            state.expire_prepared();
                            let Ok(slot) = matching(&mut state, &worker_ticket) else {
                                guard.completed = true;
                                return;
                            };
                            slot.enter()
                        };
                        if !entered {
                            guard.completed = true;
                            notify(&worker_app, &worker_ticket);
                            return;
                        }
                        // The existing host Owner excludes background hibernation,
                        // direct tab mutation and positioning-triggered recreation.
                        let result = if binding.current(&worker_app, &action) {
                            action.execute(webview, worker_app.clone(), &binding)
                        } else {
                            Err("TAB_MUTATION_INSTANCE_STALE".into())
                        };
                        finish(&worker_ticket, result);
                        guard.completed = true;
                        notify(&worker_app, &worker_ticket);
                    });
                if spawned.is_err() {
                    finish(&ticket, Err("TAB_MUTATION_WORKER_START_FAILED".into()));
                    notify(&app, &ticket);
                }
            }
        }
        Request::Poll { ticket } => {
            let mut state = owner()?;
            state.expire_prepared();
            matching(&mut state, &ticket)?;
        }
        Request::Cancel { ticket } => {
            {
                let mut state = owner()?;
                state.expire_prepared();
                let slot = matching(&mut state, &ticket)?;
                slot.cancel();
            }
            notify(&app, &ticket);
        }
        Request::Ack { ticket } => {
            let mut state = owner()?;
            if state.last_ack.as_deref() != Some(ticket.as_str()) {
                state.expire_prepared();
                if !matching(&mut state, &ticket)?.receipt.phase.terminal() {
                    return Err("TAB_MUTATION_NOT_TERMINAL".into());
                }
                // Retire any prepare sequence observed before the old
                // terminal slot was released. A duplicate ACK is a no-op.
                state.advance_sequence()?;
                state.slot = None;
                state.last_ack = Some(ticket);
            }
        }
        Request::Inspect => {}
        Request::SampleDownload { .. } => return Err("TAB_DOWNLOAD_SAMPLE_INVALID".into()),
        Request::InspectDownloads => {
            crate::shared::native_legacy_policy::require_main(&webview, "tab_list")?;
            if !super::isolated_tabs::mode_enabled() {
                return Err("TAB_RENDERER_ROUTE_INVALID".into());
            }
            let mut view = snapshot(&app)?;
            view.download_inventory = Some(app.state::<crate::AppState>()
                .grid_manager.tab_download_inventory()?);
            return Ok(view);
        }
        Request::InspectRenderer { id } => {
            crate::shared::native_legacy_policy::require_main(&webview, "tab_close")?;
            crate::shared::native_legacy_policy::bound_text("tab_close", "id", &id)?;
            let index = super::isolated_tabs::route(&id).ok_or("TAB_RENDERER_ROUTE_INVALID")?;
            let state = app.state::<crate::AppState>();
            if !state.tabs.try_lock().map_err(|_| "TAB_RENDERER_OBSERVATION_BUSY")?.contains_key(&id) {
                return Err("TAB_NOT_FOUND".into());
            }
            let mut view = snapshot(&app)?;
            let (observed_owner, status, download) = state.grid_manager.tab_renderer_inspection(index)?;
            let hibernated = state.hibernated_tabs.try_lock()
                .map_err(|_| "TAB_RENDERER_OBSERVATION_BUSY")?.contains(&id);
            // A logical close or a competing Owner admission can cross the
            // split observation. Return busy rather than an actionable target.
            if !state.tabs.try_lock().map_err(|_| "TAB_RENDERER_OBSERVATION_BUSY")?.contains_key(&id)
                || !owner()?.matches_view(&view) {
                return Err("TAB_RENDERER_OBSERVATION_BUSY".into());
            }
            view.renderer_state = Some(if status == "absent" && hibernated {
                "hibernated"
            } else { status });
            if status == "absent" && observed_owner.is_none() && !hibernated
                && view.receipt.is_none() && !view.internal_active {
                if let Ok(epoch) = state.grid_manager.tab_renderer_absence_epoch(index) {
                    if owner()?.matches_view(&view) {
                        view.renderer_reopen_target = Some(RendererReopenTarget {
                            id: id.clone(), review_owner_epoch: view.epoch.clone(),
                            review_owner_sequence: view.next_sequence.clone(), absence_epoch: epoch.to_string(),
                        });
                    }
                }
            }
            if let Some((pid, epoch)) = observed_owner {
                view.renderer_download = download;
                if status == "ready" && view.active_known
                    && view.active_tab_id.as_deref() != Some(id.as_str()) {
                    if let Ok(binding) = state.grid_manager.renderer_binding(index) {
                        if binding.owner == (pid, epoch) {
                            view.renderer_park_target = Some(RendererParkTarget {
                                id: id.clone(), pid, spawn_epoch: epoch.to_string(),
                                renderer_generation: binding.generation.to_string(),
                                navigation_revision: binding.navigation_revision.to_string(),
                            });
                        }
                    }
                }
                view.renderer_recovery_target = Some(RendererRecoveryTarget {
                    id, pid, spawn_epoch: epoch.to_string(),
                });
            }
            return Ok(view);
        }
        Request::RecoverDomain {
            epoch,
            sequence,
            ticket,
            confirmed,
            scope,
        } => {
            if !confirmed {
                return Err("TAB_DOMAIN_RECOVERY_CONFIRMATION_REQUIRED".into());
            }
            let sequence = sequence.parse::<u64>().map_err(|_| "TAB_MUTATION_STALE_SEQUENCE")?;
            let target = RecoveryTarget { epoch, sequence, ticket };
            let local = {
                let mut state = owner()?;
                if state.recovery_target(Instant::now()).as_ref() != Some(&target) {
                    return Err("TAB_DOMAIN_RECOVERY_NOT_ELIGIBLE".into());
                }
                let renderer = state.local_renderer_recovery();
                // The user confirmed an exact recovery scope, not permission
                // for a renderer-only recovery to escalate to application exit.
                if !recovery_scope_matches(scope.as_deref(), renderer.is_some()) {
                    return Err("TAB_DOMAIN_RECOVERY_SCOPE_CHANGED".into());
                }
                if let Some((index, expected)) = renderer {
                    let ticket = target.ticket.clone().ok_or("TAB_MUTATION_INVALID_ACTION")?;
                    state.recovery_intent = None;
                    state.recovery_claimed = true;
                    Some((index, expected, ticket))
                } else {
                    if !supervisor_ready() {
                        return Err("TAB_DOMAIN_SUPERVISOR_UNAVAILABLE".into());
                    }
                    state.request_recovery(target, Instant::now())?;
                    None
                }
            };
            if let Some((index, expected, ticket)) = local {
                let worker_app = app.clone();
                let started = std::thread::Builder::new().name("tab-renderer-recovery".into()).spawn(move || {
                    let result = worker_app.state::<crate::AppState>().grid_manager.retire_bound_tab_renderer(index, expected);
                    if let Ok(mut state) = owner() {
                        state.recovery_claimed = false;
                        if let Ok(slot) = matching(&mut state, &ticket) {
                            if slot.receipt.phase == Phase::Unknown {
                                match result {
                                    Ok(()) => {
                                        slot.receipt.phase = Phase::Failed;
                                        slot.receipt.error = Some("TAB_RENDERER_RETIRED_REACTIVATE: original effect was unknown".into());
                                    }
                                    Err(_) => slot.receipt.error = Some("TAB_RENDERER_CLEANUP_PENDING".into()),
                                }
                            }
                        }
                    }
                    notify(&worker_app, &ticket);
                });
                if started.is_err() {
                    owner()?.recovery_claimed = false;
                    return Err("TAB_RENDERER_RECOVERY_WORKER_FAILED".into());
                }
            }
        }
        Request::Fence { epoch, sequence } => {
            let mut state = owner()?;
            if state.slot.is_some() || state.internal_active || state.recovery_claimed {
                return Err("TAB_MUTATION_OWNER_BUSY".into());
            }
            if epoch != state.epoch {
                return Err("TAB_MUTATION_STALE_EPOCH".into());
            }
            if sequence != state.next_sequence.to_string() {
                return Err("TAB_MUTATION_STALE_SEQUENCE".into());
            }
            // A lost prepare may still be queued in IPC. Retire its admission
            // sequence before declaring an empty owner reconciled.
            state.advance_sequence()?;
        }
    }
    snapshot(&app)
}

// Keep the old trusted command contract for compatibility without giving it
// a second admission lane. A legacy command does not expose replayable results.
pub(super) async fn legacy_action<T: serde::de::DeserializeOwned + Send + 'static>(
    webview: Webview, app: AppHandle, action: Action,
) -> Result<T, String> {
    action.authorize(&webview)?;
    let ticket = uuid::Uuid::new_v4().to_string();
    // As with Prepare, reserve admission across native identity capture,
    // without holding the Owner mutex across native locks.
    let reservation = {
        let mut state = owner()?;
        state.expire_prepared();
        if state.slot.is_some() || state.internal_active || state.recovery_claimed {
            return Err("TAB_MUTATION_OWNER_BUSY".into());
        }
        state.internal_active = true;
        state.internal_started = Some(Instant::now());
        Some(InternalMutation)
    };
    let binding = Binding::capture(&app, &action)?;
    {
        let mut state = owner()?;
        if state.slot.is_some() || !state.internal_active || state.recovery_claimed {
            return Err("TAB_MUTATION_OWNER_BUSY".into());
        }
        state.advance_sequence()?;
        state.slot = Some(Slot {
            receipt: Receipt {
                ticket: ticket.clone(),
                client_id: "legacy".into(),
                command: action.command(),
                phase: Phase::Running,
                value: None,
                error: None,
            },
            action: Some(action.clone()),
            binding: Some(binding.clone()),
            since: std::time::Instant::now(),
            entered: false,
            agent_owned: false,
        });
    }
    drop(reservation);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = WorkerGuard {
            ticket: ticket.clone(),
            completed: false,
        };
        let entered = {
            let mut state = owner()?;
            state.expire_prepared();
            matching(&mut state, &ticket).is_ok_and(Slot::enter)
        };
        if !entered {
            guard.completed = true;
            notify(&app, &ticket);
            return Err("TAB_MUTATION_CANCELLED_BEFORE_EXECUTION".into());
        }
        let result = if binding.current(&app, &action) {
            action.execute(webview, app.clone(), &binding)
                .and_then(|value| serde_json::from_value::<T>(value)
                    .map_err(|_| "GRID_REQUEST_OUTCOME_UNKNOWN: legacy receipt decoding rejected".into()))
        } else {
            Err("TAB_MUTATION_INSTANCE_STALE".into())
        };
        if let Err(error) = &result {
            if completion_unknown(error) || error.contains("TAB_CREATED_ACTIVATION_PARTIAL")
                || error.contains("TAB_RENDERER_ACTIVATION_PARTIAL")
                || error.contains("TAB_RENDERER_REOPEN_PARTIAL") {
                finish(&ticket, Err(error.clone()));
                notify(&app, &ticket);
                guard.completed = true;
                return result;
            }
        }
        if let Ok(mut state) = owner() {
            if matching(&mut state, &ticket).is_ok() { state.slot = None; }
        }
        guard.completed = true;
        result
    }).await.map_err(|_| "TAB_MUTATION_WORKER_UNKNOWN".to_string())?
}

pub(crate) fn tab_mutation_diagnostic() -> Value {
    match OWNER.try_lock() {
        Ok(state) => state.slot.as_ref().map(|s| serde_json::json!({
            "ticket": s.receipt.ticket, "command": s.receipt.command,
            "phase": s.receipt.phase, "entered": s.entered, "age_ms": s.since.elapsed().as_millis() as u64
        })).unwrap_or(Value::Null),
        Err(_) => serde_json::json!({"phase":"owner_unavailable"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_scope_cannot_change_after_confirmation() {
        assert!(recovery_scope_matches(Some("renderer"), true));
        assert!(recovery_scope_matches(Some("application"), false));
        assert!(!recovery_scope_matches(Some("renderer"), false));
        assert!(!recovery_scope_matches(Some("application"), true));
        assert!(recovery_scope_matches(None, false));
        assert!(!recovery_scope_matches(None, true));
        assert!(!recovery_scope_matches(Some("all"), false));
    }

    #[test]
    fn expired_preparation_never_admits_a_late_start() {
        let mut state = Owner::default();
        let old_sequence = state.next_sequence;
        state.next_sequence += 1;
        let action = Action::New {
            url: "about:blank".into(),
        };
        state.slot = Some(Slot {
            receipt: Receipt {
                ticket: "fixture".into(),
                client_id: "fixture".into(),
                command: action.command(),
                phase: Phase::Prepared,
                value: None,
                error: None,
            },
            action: Some(action),
            binding: Some(Binding::New),
            since: std::time::Instant::now() - std::time::Duration::from_secs(31),
            entered: false,
            agent_owned: false,
        });
        state.expire_prepared();
        assert_eq!(state.slot.as_ref().unwrap().receipt.phase, Phase::Cancelled);
        assert_ne!(old_sequence, state.next_sequence);
    }

    #[test]
    fn queued_worker_revocation_is_terminal_and_late_entry_is_impossible() {
        let mut slot = Slot {
            receipt: Receipt {
                ticket: "fixture".into(),
                client_id: "fixture".into(),
                command: "tab_new",
                phase: Phase::Running,
                value: None,
                error: None,
            },
            action: None,
            binding: None,
            since: std::time::Instant::now(),
            entered: false,
            agent_owned: false,
        };
        slot.cancel();
        assert_eq!(slot.receipt.phase, Phase::Cancelled);
        assert!(!slot.enter());
        slot.receipt.phase = Phase::Running;
        assert!(slot.enter());
        slot.cancel();
        assert_eq!(slot.receipt.phase, Phase::CancelRequested);
        assert!(!slot.receipt.phase.terminal());
    }

    #[test]
    fn completed_background_mutation_invalidates_empty_snapshot() {
        let mut state = Owner::default();
        let view = OwnerView {
            epoch: state.epoch.clone(),
            next_sequence: state.next_sequence.to_string(),
            receipt: None,
            last_ack: None,
            internal_active: false,
            active_known: true,
            active_tab_id: None,
            domain_recovery_available: false,
            recovery_scope: None,
            renderer_recovery_target: None,
            renderer_park_target: None,
            renderer_reopen_target: None,
            isolated_renderer_mode: false,
            renderer_state: None,
            renderer_download: None,
            download_inventory: None,
        };
        assert!(state.matches_view(&view));
        state.admit_background().unwrap();
        assert!(!state.matches_view(&view));
        assert!(state.admit_background().is_err());
        state.internal_active = false;
        assert!(!state.matches_view(&view));
    }

    #[test]
    fn explicit_domain_recovery_requires_the_same_stalled_owner() {
        let now = Instant::now();
        let mut state = Owner::default();
        state.admit_background().unwrap();
        assert!(state.recovery_target(now).is_none());
        let later = now + DOMAIN_STALL + Duration::from_secs(1);
        let target = state.recovery_target(later).unwrap();
        state.request_recovery(target.clone(), later).unwrap();
        let intent = state.recovery_intent.clone().unwrap();
        state
            .request_recovery(target, later + Duration::from_secs(1))
            .unwrap();
        assert_eq!(state.recovery_intent.as_ref(), Some(&intent));
        let expired = later + RECOVERY_INTENT_TTL;
        assert!(!state.claim_recovery(&intent, expired));
        let current = state.recovery_target(expired).unwrap();
        state.request_recovery(current, expired).unwrap();
        let fresh = state.recovery_intent.clone().unwrap();
        assert!(!state.claim_recovery(&intent, expired));
        assert!(state.claim_recovery(&fresh, expired));
        assert!(state.recovery_target(later).is_none());
        state.internal_active = false;
        state.internal_started = None;
        assert!(state.admit_background().is_err());
        state.recovery_claimed = false;
        state.admit_background().unwrap();
        assert!(!state.claim_recovery(&intent, later));
    }

    #[test]
    fn prepared_and_terminal_receipts_cannot_authorize_domain_recovery() {
        let now = Instant::now();
        let mut state = Owner::default();
        state.slot = Some(Slot {
            receipt: Receipt {
                ticket: "old".into(),
                client_id: "fixture".into(),
                command: "tab_open",
                phase: Phase::Prepared,
                value: None,
                error: None,
            },
            action: None,
            binding: None,
            since: now,
            entered: false,
            agent_owned: false,
        });
        let later = now + DOMAIN_STALL + Duration::from_secs(1);
        assert!(state.recovery_target(later).is_none());
        state.slot.as_mut().unwrap().receipt.phase = Phase::Running;
        assert!(state.recovery_target(later).is_none());
        assert!(state.slot.as_mut().unwrap().enter());
        let entered = state.slot.as_ref().unwrap().since;
        assert!(state.recovery_target(entered).is_none());
        let stalled = entered + DOMAIN_STALL;
        let target = state.recovery_target(stalled).unwrap();
        state.request_recovery(target, stalled).unwrap();
        let intent = state.recovery_intent.clone().unwrap();
        state.slot.as_mut().unwrap().receipt.phase = Phase::Succeeded;
        assert!(!state.claim_recovery(&intent, stalled));
        state.slot.as_mut().unwrap().receipt.phase = Phase::Unknown;
        state.slot.as_mut().unwrap().receipt.ticket = "new".into();
        assert!(!state.claim_recovery(&intent, stalled));
    }

    #[test]
    fn cancellation_request_and_unknown_are_not_terminal_receipts() {
        for phase in [
            Phase::Prepared,
            Phase::Running,
            Phase::CancelRequested,
            Phase::Unknown,
        ] {
            assert!(!phase.terminal());
        }
        for phase in [Phase::Succeeded, Phase::Failed, Phase::Cancelled] {
            assert!(phase.terminal());
        }
    }
}
