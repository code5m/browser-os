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
use tauri_plugin_browser_tabs::{TabInstance, TabManagerState};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Action {
    New { url: String },
    Close { id: String },
    Activate { id: String },
    Open { id: String, url: String },
    Back { id: String },
    Forward { id: String },
    Reload { id: String },
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
            | Self::Reload { id } => Some(id),
        }
    }
    fn authorize(&self, webview: &Webview) -> Result<(), String> {
        use crate::shared::native_legacy_policy::{bound_text, require_main};
        let cmd = self.command();
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
        }
    }
    fn execute(self, webview: Webview, app: AppHandle) -> Result<Value, String> {
        match self {
            Self::New { url } => serde_json::to_value(tab_new_impl(webview, app, url)?)
                .map_err(|_| "TAB_RECEIPT_SERIALIZATION_FAILED".into()),
            Self::Close { id } => tab_close_impl(webview, app, id).map(|_| Value::Null),
            Self::Activate { id } => tab_activate_impl(webview, app, id).map(|_| Value::Null),
            Self::Open { id, url } => tab_open_impl(webview, app, id, url).map(|_| Value::Null),
            Self::Back { id } => tab_go_back_impl(webview, app, id).map(|_| Value::Null),
            Self::Forward { id } => tab_go_forward_impl(webview, app, id).map(|_| Value::Null),
            Self::Reload { id } => tab_reload_impl(webview, app, id).map(|_| Value::Null),
        }
    }
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
        if state
            .hibernated_tabs
            .try_lock()
            .map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE")?
            .contains(id)
        {
            return Ok(Self::Hibernated);
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
            None
                if self.internal_active
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
pub(crate) struct OwnerView {
    epoch: String,
    next_sequence: String,
    receipt: Option<Receipt>,
    last_ack: Option<String>,
    internal_active: bool,
    active_known: bool,
    active_tab_id: Option<String>,
    domain_recovery_available: bool,
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
            domain_recovery_available: supervisor_ready()
                && state.recovery_target(Instant::now()).is_some(),
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
    if !state.matches_view(&view) || state.recovery_target(Instant::now()).is_none() {
        view.domain_recovery_available = false;
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
    state
        .claim_recovery(intent, Instant::now())
        .then_some(DomainRecoveryClaim)
}

fn matching<'a>(state: &'a mut Owner, ticket: &str) -> Result<&'a mut Slot, String> {
    state
        .slot
        .as_mut()
        .filter(|s| s.receipt.ticket == ticket)
        .ok_or_else(|| "TAB_MUTATION_STALE_TICKET".into())
}
fn completion_unknown(error: &str) -> bool {
    matches!(
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
#[serde(tag = "phase", rename_all = "snake_case")]
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
    Fence {
        epoch: String,
        sequence: String,
    },
    RecoverDomain {
        epoch: String,
        sequence: String,
        ticket: Option<String>,
        confirmed: bool,
    },
}
#[tauri::command]
pub(crate) fn tab_mutation(
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
                            action.execute(webview, worker_app.clone())
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
        Request::RecoverDomain {
            epoch,
            sequence,
            ticket,
            confirmed,
        } => {
            if !confirmed {
                return Err("TAB_DOMAIN_RECOVERY_CONFIRMATION_REQUIRED".into());
            }
            if !supervisor_ready() {
                return Err("TAB_DOMAIN_SUPERVISOR_UNAVAILABLE".into());
            }
            let sequence = sequence
                .parse::<u64>()
                .map_err(|_| "TAB_MUTATION_STALE_SEQUENCE")?;
            owner()?.request_recovery(
                RecoveryTarget {
                    epoch,
                    sequence,
                    ticket,
                },
                Instant::now(),
            )?;
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
pub(super) async fn legacy<T: Send + 'static>(
    webview: Webview,
    command: &'static str,
    operation: impl FnOnce(Webview) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    crate::shared::native_legacy_policy::require_main(&webview, command)?;
    let ticket = uuid::Uuid::new_v4().to_string();
    {
        let mut state = owner()?;
        if state.slot.is_some() || state.internal_active || state.recovery_claimed {
            return Err("TAB_MUTATION_OWNER_BUSY".into());
        }
        state.advance_sequence()?;
        state.slot = Some(Slot {
            receipt: Receipt {
                ticket: ticket.clone(),
                client_id: "legacy".into(),
                command,
                phase: Phase::Running,
                value: None,
                error: None,
            },
            action: None,
            binding: None,
            since: std::time::Instant::now(),
            entered: false,
        });
    }
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
            return Err("TAB_MUTATION_CANCELLED_BEFORE_EXECUTION".into());
        }
        let result = operation(webview);
        if result
            .as_ref()
            .is_err_and(|error| completion_unknown(error))
        {
            unknown(&ticket);
        } else if let Ok(mut state) = owner() {
            if matching(&mut state, &ticket).is_ok() {
                state.slot = None;
            }
        }
        guard.completed = true;
        result
    })
    .await
    .map_err(|_| "TAB_MUTATION_WORKER_UNKNOWN".to_string())?
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
        assert!(!state.claim_recovery(&intent, later + RECOVERY_INTENT_TTL));
        assert!(state.claim_recovery(&intent, later));
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
