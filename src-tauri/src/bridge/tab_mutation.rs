//! Native receipt ownership for main-window tab mutations (ADR D01).
//! A receipt describes the host mutation, not the renderer's navigation/paint.
//! A lost IPC reply, cancelled waiter or elapsed deadline cannot release a live
//! worker. Prepared work has no effects; running cancellation is advisory.
use super::tab_runtime::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Webview};

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
            Self::New { .. } => "tab_new", Self::Close { .. } => "tab_close",
            Self::Activate { .. } => "tab_activate", Self::Open { .. } => "tab_open",
            Self::Back { .. } => "tab_go_back", Self::Forward { .. } => "tab_go_forward",
            Self::Reload { .. } => "tab_reload",
        }
    }
    fn authorize(&self, webview: &Webview) -> Result<(), String> {
        use crate::shared::native_legacy_policy::{bound_text, require_main};
        let cmd = self.command();
        require_main(webview, cmd)?;
        match self {
            Self::New { url } => bound_text(cmd, "url", url),
            Self::Open { id, url } => { bound_text(cmd, "id", id)?; bound_text(cmd, "url", url) }
            Self::Close { id } | Self::Activate { id } | Self::Back { id }
                | Self::Forward { id } | Self::Reload { id } => bound_text(cmd, "id", id),
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
enum Phase { Prepared, Running, CancelRequested, Succeeded, Failed, Cancelled, Unknown }
impl Phase {
    fn terminal(self) -> bool { matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled) }
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
struct Slot { receipt: Receipt, action: Option<Action>, since: std::time::Instant }
#[derive(Default)]
struct Owner { slot: Option<Slot>, last_ack: Option<String> }
static OWNER: LazyLock<Mutex<Owner>> = LazyLock::new(|| Mutex::new(Owner::default()));

fn owner() -> Result<std::sync::MutexGuard<'static, Owner>, String> {
    OWNER.lock().map_err(|_| "TAB_MUTATION_OWNER_UNKNOWN".into())
}
fn matching<'a>(state: &'a mut Owner, ticket: &str) -> Result<&'a mut Slot, String> {
    state.slot.as_mut().filter(|s| s.receipt.ticket == ticket)
        .ok_or_else(|| "TAB_MUTATION_STALE_TICKET".into())
}
fn finish(ticket: &str, result: Result<Value, String>) {
    if let Ok(mut state) = owner() {
        if let Ok(slot) = matching(&mut state, ticket) {
            if !matches!(slot.receipt.phase, Phase::Running | Phase::CancelRequested) { return; }
            match result {
                Ok(value) => { slot.receipt.phase = Phase::Succeeded; slot.receipt.value = Some(value); }
                Err(error) => { slot.receipt.phase = Phase::Failed; slot.receipt.error = Some(error); }
            }
        }
    }
}
fn unknown(ticket: &str) {
    if let Ok(mut state) = owner() {
        if let Ok(slot) = matching(&mut state, ticket) {
            if !slot.receipt.phase.terminal() { slot.receipt.phase = Phase::Unknown; }
        }
    }
}
struct WorkerGuard { ticket: String, completed: bool }
impl Drop for WorkerGuard {
    fn drop(&mut self) { if !self.completed { unknown(&self.ticket); } }
}

#[derive(Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub(crate) enum Request {
    Prepare { client_id: String, action: Action },
    Start { ticket: String },
    Poll { ticket: String },
    Cancel { ticket: String },
    Ack { ticket: String },
    Inspect,
}
#[tauri::command]
pub(crate) fn tab_mutation(webview: Webview, app: AppHandle, request: Request) -> Result<Option<Receipt>, String> {
    if webview.label() != "main" { return Err("NATIVE_SOURCE_DENIED".into()); }
    match request {
        Request::Prepare { client_id, action } => {
            action.authorize(&webview)?;
            if uuid::Uuid::parse_str(&client_id).is_err() { return Err("TAB_MUTATION_INVALID_ID".into()); }
            let mut state = owner()?;
            if let Some(slot) = &state.slot {
                if slot.receipt.client_id == client_id && slot.action.as_ref() == Some(&action) {
                    return Ok(Some(slot.receipt.clone()));
                }
                return Err("TAB_MUTATION_OWNER_BUSY".into());
            }
            let receipt = Receipt { ticket: uuid::Uuid::new_v4().to_string(), client_id,
                command: action.command(), phase: Phase::Prepared, value: None, error: None };
            state.slot = Some(Slot { receipt: receipt.clone(), action: Some(action), since: std::time::Instant::now() });
            Ok(Some(receipt))
        }
        Request::Start { ticket } => {
            let action = {
                let mut state = owner()?;
                let slot = matching(&mut state, &ticket)?;
                if slot.receipt.phase != Phase::Prepared { return Ok(Some(slot.receipt.clone())); }
                let action = slot.action.clone().ok_or("TAB_MUTATION_INVALID_ACTION")?;
                slot.receipt.phase = Phase::Running;
                action
            };
            let worker_ticket = ticket.clone();
            let spawned = std::thread::Builder::new().name("browseros-tab-owner".into()).spawn(move || {
                let mut guard = WorkerGuard { ticket: worker_ticket.clone(), completed: false };
                // Only cancel before crossing the native dispatch boundary.
                {
                    let Ok(mut state) = owner() else { return; };
                    let Ok(slot) = matching(&mut state, &worker_ticket) else { return; };
                    if slot.receipt.phase == Phase::CancelRequested {
                        slot.receipt.phase = Phase::Cancelled;
                        guard.completed = true;
                        return;
                    }
                }
                let result = action.execute(webview, app);
                finish(&worker_ticket, result);
                guard.completed = true;
            });
            if spawned.is_err() { finish(&ticket, Err("TAB_MUTATION_WORKER_START_FAILED".into())); }
            let mut state = owner()?;
            Ok(Some(matching(&mut state, &ticket)?.receipt.clone()))
        }
        Request::Poll { ticket } => {
            let mut state = owner()?;
            Ok(Some(matching(&mut state, &ticket)?.receipt.clone()))
        }
        Request::Cancel { ticket } => {
            let mut state = owner()?;
            let slot = matching(&mut state, &ticket)?;
            slot.receipt.phase = match slot.receipt.phase {
                Phase::Prepared => Phase::Cancelled,
                Phase::Running => Phase::CancelRequested,
                other => other,
            };
            Ok(Some(slot.receipt.clone()))
        }
        Request::Ack { ticket } => {
            let mut state = owner()?;
            if state.last_ack.as_deref() == Some(ticket.as_str()) { return Ok(None); }
            if !matching(&mut state, &ticket)?.receipt.phase.terminal() {
                return Err("TAB_MUTATION_NOT_TERMINAL".into());
            }
            state.slot = None;
            state.last_ack = Some(ticket);
            Ok(None)
        }
        Request::Inspect => Ok(owner()?.slot.as_ref().map(|s| s.receipt.clone())),
    }
}

// Keep the old trusted command contract for compatibility without giving it
// a second admission lane. A legacy command does not expose replayable results.
pub(super) async fn legacy<T: Send + 'static>(
    webview: Webview, command: &'static str,
    operation: impl FnOnce(Webview) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    crate::shared::native_legacy_policy::require_main(&webview, command)?;
    let ticket = uuid::Uuid::new_v4().to_string();
    {
        let mut state = owner()?;
        if state.slot.is_some() { return Err("TAB_MUTATION_OWNER_BUSY".into()); }
        state.slot = Some(Slot {
            receipt: Receipt { ticket: ticket.clone(), client_id: "legacy".into(), command,
                phase: Phase::Running, value: None, error: None },
            action: None, since: std::time::Instant::now(),
        });
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = WorkerGuard { ticket: ticket.clone(), completed: false };
        let result = operation(webview);
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
            "phase": s.receipt.phase, "age_ms": s.since.elapsed().as_millis() as u64
        })).unwrap_or(Value::Null),
        Err(_) => serde_json::json!({"phase":"owner_unavailable"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_request_and_unknown_are_not_terminal_receipts() {
        for phase in [Phase::Prepared, Phase::Running, Phase::CancelRequested, Phase::Unknown] {
            assert!(!phase.terminal());
        }
        for phase in [Phase::Succeeded, Phase::Failed, Phase::Cancelled] {
            assert!(phase.terminal());
        }
    }
}
