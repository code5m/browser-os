//! Explicitly saved, read-only observations of native transfers. This is
//! not a live transfer registry or a grant to reopen a path or resume a job.
use crate::capabilities::workspace::safe_paths;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};
use uuid::Uuid;

const FILE: &str = "native-download-observations.json";
const LIMIT: usize = 256;
const MAX_BYTES: usize = 512 * 1024;
static GATE: Mutex<()> = Mutex::new(());

pub(super) fn claim() -> Result<MutexGuard<'static, ()>, String> {
    GATE.try_lock().map_err(|_| "DOWNLOAD_HISTORY_BUSY".into())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    session: Uuid,
    id: String,
    pid: Option<u32>,
    spawn_epoch: Option<String>,
    transfer_id: String,
    name: Option<String>,
    state: String,
    received_bytes: Option<String>,
    total_bytes: Option<String>,
    observed_at_ms: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    version: u32,
    rows: Vec<Row>,
}
fn number(value: &str, nonzero: bool) -> bool {
    value
        .parse::<u64>()
        .is_ok_and(|v| (!nonzero || v > 0) && v.to_string() == value)
}
fn transfer(value: &str) -> bool {
    value
        .split_once(':')
        .is_some_and(|(a, b)| number(a, true) && number(b, true))
}
fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.chars().count() <= 180
        && !value.chars().any(|c| {
            c.is_control()
                || matches!(c, '/' | '\\' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}
impl Row {
    fn valid(&self) -> bool {
        self.session.get_version_num() == 4
            && super::is_tab_label(&self.id)
            && self.id.len() <= 64
            && (self.pid.is_none() && self.spawn_epoch.is_none()
                || self.pid.is_some_and(|v| v > 0)
                    && self.spawn_epoch.as_deref().is_some_and(|v| number(v, true)))
            && transfer(&self.transfer_id)
            && self.name.as_deref().is_none_or(safe_name)
            && matches!(
                self.state.as_str(),
                "in_progress"
                    | "paused"
                    | "interrupted"
                    | "completed"
                    | "cancelled"
                    | "failed"
                    | "unknown"
            )
            && [&self.received_bytes, &self.total_bytes]
                .into_iter()
                .all(|v| v.as_deref().is_none_or(|v| number(v, false)))
    }
    fn same_owner(&self, other: &Self) -> bool {
        self.session == other.session
            && self.id == other.id
            && self.pid == other.pid
            && self.spawn_epoch == other.spawn_epoch
            && self.transfer_id == other.transfer_id
    }
}
fn read(root: &Path) -> Result<(History, Option<safe_paths::FileSnapshot>), String> {
    let snapshot = safe_paths::read_versioned(&root.join(FILE), &[root.to_path_buf()], MAX_BYTES)?;
    let history = match snapshot.as_ref() {
        Some(value) => serde_json::from_slice::<History>(&value.bytes)
            .map_err(|_| "DOWNLOAD_HISTORY_INVALID")?,
        None => History {
            version: 1,
            rows: Vec::new(),
        },
    };
    if history.version != 1
        || history.rows.len() > LIMIT
        || history.rows.iter().any(|row| !row.valid())
    {
        return Err("DOWNLOAD_HISTORY_INVALID".into());
    }
    for (i, row) in history.rows.iter().enumerate() {
        if history.rows[..i].iter().any(|other| row.same_owner(other)) {
            return Err("DOWNLOAD_HISTORY_INVALID".into());
        }
    }
    Ok((history, snapshot))
}
fn publish(
    root: &Path,
    identity: &str,
    previous: Option<&safe_paths::FileSnapshot>,
    history: &History,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(history).map_err(|_| "DOWNLOAD_HISTORY_INVALID")?;
    safe_paths::write_versioned_approved(
        &root.join(FILE),
        &[root.to_path_buf()],
        root,
        identity,
        previous,
        &bytes,
        MAX_BYTES,
        |_, _| Ok(()),
    )?;
    let (_, saved) = read(root)?;
    if saved.is_none_or(|v| v.bytes != bytes) {
        return Err("DOWNLOAD_HISTORY_PUBLICATION_UNCONFIRMED".into());
    }
    Ok(())
}
pub(super) fn inspect(root: &Path, _guard: &MutexGuard<'static, ()>) -> Result<Value, String> {
    let (history, _) = read(root)?;
    Ok(json!({"coverage":"explicit_observations","limit":LIMIT,"rows":history.rows}))
}
pub(super) fn clear(root: &Path, _guard: &MutexGuard<'static, ()>) -> Result<Value, String> {
    let identity =
        safe_paths::directory_identity(root).map_err(|_| "DOWNLOAD_HISTORY_DIRECTORY")?;
    let (_, previous) = read(root)?;
    let history = History {
        version: 1,
        rows: Vec::new(),
    };
    publish(root, &identity, previous.as_ref(), &history)?;
    Ok(json!({"coverage":"explicit_observations","limit":LIMIT,"rows":[]}))
}
pub(super) fn remember(
    root: &Path,
    _guard: &MutexGuard<'static, ()>,
    session: Uuid,
    id: &str,
    owner: Option<(u32, u64)>,
    tasks: &Value,
) -> Result<(), String> {
    let instance = tasks["native_instance"]
        .as_str()
        .filter(|v| number(v, true))
        .ok_or("DOWNLOAD_RESULT_INVALID")?;
    let items = tasks["items"]
        .as_array()
        .filter(|v| v.len() <= 64)
        .ok_or("DOWNLOAD_RESULT_INVALID")?;
    if tasks["coverage"] != "since_registration" {
        return Err("DOWNLOAD_RESULT_INVALID".into());
    }
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or(0);
    let mut rows = Vec::<Row>::new();
    // Only typed, redacted metadata crosses the persistence boundary.
    let optional_string = |item: &Value, key: &str| -> Result<Option<String>, String> {
        match item.get(key) {
            Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            _ => Err("DOWNLOAD_RESULT_INVALID".into()),
        }
    };
    for item in items {
        let transfer_id = item["transfer_id"]
            .as_str()
            .filter(|v| v.split_once(':').is_some_and(|(a, _)| a == instance))
            .ok_or("DOWNLOAD_RESULT_INVALID")?
            .to_owned();
        let row = Row {
            session,
            id: id.to_owned(),
            pid: owner.map(|v| v.0),
            spawn_epoch: owner.map(|v| v.1.to_string()),
            transfer_id,
            name: optional_string(item, "name")?,
            state: item["state"]
                .as_str()
                .ok_or("DOWNLOAD_RESULT_INVALID")?
                .to_owned(),
            received_bytes: optional_string(item, "received_bytes")?,
            total_bytes: optional_string(item, "total_bytes")?,
            observed_at_ms,
        };
        if !row.valid() || rows.iter().any(|other| row.same_owner(other)) {
            return Err("DOWNLOAD_RESULT_INVALID".into());
        }
        rows.push(row);
    }
    let identity =
        safe_paths::directory_identity(root).map_err(|_| "DOWNLOAD_HISTORY_DIRECTORY")?;
    let (mut history, previous) = read(root)?;
    for row in rows {
        history.rows.retain(|old| !row.same_owner(old));
        history.rows.insert(0, row);
    }
    history.rows.truncate(LIMIT);
    publish(root, &identity, previous.as_ref(), &history)
}
