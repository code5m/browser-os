//! Optional SQLite transactional task/event/outbox store.
//! All operations fail closed on conflict. The host must provide file access
//! controls and encryption-at-rest before persisting real user goals/context.
use crate::{
    event::{EventKind, EventRecord},
    task::{TaskRecord, TaskStatus},
    tool::ToolEffect,
    FoundationError, Result,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

fn storage(error: impl std::fmt::Display) -> FoundationError {
    FoundationError::Storage(error.to_string())
}
fn encode<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(storage)
}
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(storage)
}
fn epoch_ms() -> Result<i64> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(storage)?
        .as_millis();
    i64::try_from(millis).map_err(storage)
}
fn to_i64(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(storage)
}

pub struct SqliteTaskStore {
    connection: Mutex<Connection>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectReceipt {
    pub task_id: Uuid,
    pub call_id: Uuid,
    pub key: String,
    pub effect: ToolEffect,
    pub tool_name: String,
    pub state: EffectState,
    pub evidence: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectState {
    /// External side effect may have run; no retry is safe without reconciliation.
    Unknown,
    /// The adapter received an explicit success response (not a rollback guarantee).
    Confirmed,
    /// A separately verified external outcome was recorded by the trusted host.
    ReconciledSucceeded,
    ReconciledNotApplied,
}

impl SqliteTaskStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path).map_err(storage)?;
        Self::initialize(connection)
    }
    pub fn in_memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory().map_err(storage)?)
    }
    fn initialize(connection: Connection) -> Result<Self> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(storage)?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS foundation_schema (
                id INTEGER PRIMARY KEY CHECK (id=1), version INTEGER NOT NULL
             );
             INSERT OR IGNORE INTO foundation_schema VALUES (1,1);
             CREATE TABLE IF NOT EXISTS foundation_tasks (
                id TEXT PRIMARY KEY, revision INTEGER NOT NULL, body TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS foundation_events (
                task_id TEXT NOT NULL, seq INTEGER NOT NULL, event_id TEXT NOT NULL UNIQUE,
                body TEXT NOT NULL, PRIMARY KEY(task_id,seq)
             );
             CREATE TABLE IF NOT EXISTS foundation_outbox (
                event_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                seq INTEGER NOT NULL, body TEXT NOT NULL, delivered_ms INTEGER
             );
             CREATE INDEX IF NOT EXISTS foundation_outbox_pending ON foundation_outbox(delivered_ms,task_id,seq);
             CREATE TABLE IF NOT EXISTS foundation_effects (
                call_id TEXT PRIMARY KEY, task_id TEXT NOT NULL, effect_key TEXT NOT NULL UNIQUE,
                effect TEXT NOT NULL, tool TEXT NOT NULL, state TEXT NOT NULL, evidence TEXT,
                started_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS foundation_effect_authorities (
                call_id TEXT PRIMARY KEY REFERENCES foundation_effects(call_id),
                authority TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS foundation_leases (
                task_id TEXT PRIMARY KEY, token TEXT NOT NULL, expires_ms INTEGER NOT NULL
             );"
        ).map_err(storage)?;
        let version: i64 = connection
            .query_row(
                "SELECT version FROM foundation_schema WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if version != 1 {
            return Err(FoundationError::Storage(
                "unsupported storage schema".into(),
            ));
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Update exactly one task revision and emit all metadata-only events with
    /// matching outbox entries in ONE transaction. Committed events are returned
    /// for best-effort live broadcast after the caller updates its memory view.
    pub fn commit(
        &self,
        task: &TaskRecord,
        expected_revision: Option<u64>,
        event_kinds: &[EventKind],
    ) -> Result<Vec<EventRecord>> {
        let mut connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let committed = Self::commit_in_transaction(&tx, task, expected_revision, event_kinds)?;
        tx.commit().map_err(storage)?;
        Ok(committed)
    }

    fn commit_in_transaction(
        tx: &rusqlite::Transaction<'_>,
        task: &TaskRecord,
        expected_revision: Option<u64>,
        event_kinds: &[EventKind],
    ) -> Result<Vec<EventRecord>> {
        // A recovered or previously paused task cannot become executable
        // while an earlier external effect is still unaccounted for.
        if event_kinds.iter().any(|kind| {
            matches!(
                kind,
                EventKind::StateChanged {
                    to: TaskStatus::Running,
                    ..
                }
            )
        }) {
            Self::ensure_no_unknown_effect(tx, task.id)?;
        }
        let id = task.id.to_string();
        let serialized = encode(task)?;
        match expected_revision {
            None => {
                tx.execute(
                    "INSERT INTO foundation_tasks(id,revision,body) VALUES(?1,?2,?3)",
                    params![id, to_i64(task.revision)?, serialized],
                )
                .map_err(storage)?;
            }
            Some(expected) => {
                let updated = tx.execute(
                    "UPDATE foundation_tasks SET revision=?1, body=?2 WHERE id=?3 AND revision=?4",
                    params![to_i64(task.revision)?,serialized,id,to_i64(expected)?]
                ).map_err(storage)?;
                if updated != 1 {
                    return Err(FoundationError::Storage("task revision conflict".into()));
                }
            }
        }
        let mut seq: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(seq),0) FROM foundation_events WHERE task_id=?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let mut committed = Vec::with_capacity(event_kinds.len());
        for kind in event_kinds {
            seq = seq
                .checked_add(1)
                .ok_or_else(|| FoundationError::Storage("event sequence overflow".into()))?;
            let event = EventRecord {
                event_id: Uuid::new_v4(),
                task_id: task.id,
                sequence: u64::try_from(seq).map_err(storage)?,
                kind: kind.clone(),
            };
            let body = encode(&event)?;
            tx.execute(
                "INSERT INTO foundation_events(task_id,seq,event_id,body) VALUES(?1,?2,?3,?4)",
                params![id, seq, event.event_id.to_string(), body],
            )
            .map_err(storage)?;
            tx.execute(
                "INSERT INTO foundation_outbox(event_id,task_id,seq,body) VALUES(?1,?2,?3,?4)",
                params![event.event_id.to_string(), id, seq, encode(&event)?],
            )
            .map_err(storage)?;
            committed.push(event);
        }
        Ok(committed)
    }

    pub fn load_tasks(&self) -> Result<Vec<TaskRecord>> {
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        Self::load_tasks_from(&connection)
    }

    fn load_tasks_from(connection: &Connection) -> Result<Vec<TaskRecord>> {
        let mut stmt = connection
            .prepare("SELECT body FROM foundation_tasks ORDER BY id")
            .map_err(storage)?;
        let jsons = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        jsons
            .map(|x| decode::<TaskRecord>(&x.map_err(storage)?))
            .collect()
    }

    /// Explicitly pause interrupted work. All pending approvals are invalidated;
    /// existing effect receipts remain Unknown pending external reconciliation.
    pub fn recover_paused(&self) -> Result<Vec<TaskRecord>> {
        // A wall-clock deadline is not proof that a worker or its native
        // side effect stopped. Even expired leases must be reconciled by a
        // trusted owner before crash recovery can take over.
        // Keep lease inspection, snapshot reads and all recovery publications
        // in one write transaction. A second connection cannot acquire a lease
        // between the check and pause, and a failed recovery leaves no partial
        // task/event/outbox updates.
        let mut connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let leases: i64 = tx
            .query_row("SELECT COUNT(*) FROM foundation_leases", [], |row| {
                row.get(0)
            })
            .map_err(storage)?;
        if leases != 0 {
            return Err(FoundationError::Storage(
                "unreleased task lease requires owner reconciliation".into(),
            ));
        }
        let tasks = Self::load_tasks_from(&tx)?;
        let mut recovered = Vec::with_capacity(tasks.len());
        for mut task in tasks {
            let previous = task.status;
            if matches!(previous, TaskStatus::Running | TaskStatus::AwaitingApproval) {
                let old = task.revision;
                task.revision = old
                    .checked_add(1)
                    .ok_or_else(|| FoundationError::Storage("task revision overflow".into()))?;
                task.status = TaskStatus::Paused;
                task.pending = None;
                Self::commit_in_transaction(
                    &tx,
                    &task,
                    Some(old),
                    &[
                        EventKind::Interrupted,
                        EventKind::StateChanged {
                            from: previous,
                            to: TaskStatus::Paused,
                        },
                    ],
                )?;
            } else if task.pending.is_some() {
                return Err(FoundationError::Storage(
                    "invalid persisted pending operation".into(),
                ));
            }
            recovered.push(task);
        }
        tx.commit().map_err(storage)?;
        Ok(recovered)
    }

    pub fn events_since(&self, task_id: Uuid, after: u64, max: usize) -> Result<Vec<EventRecord>> {
        if !(1..=1024).contains(&max) {
            return Err(FoundationError::InvalidInput("event page size"));
        }
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = connection.prepare(
            "SELECT body FROM foundation_events WHERE task_id=?1 AND seq>?2 ORDER BY seq LIMIT ?3"
        ).map_err(storage)?;
        let rows = stmt
            .query_map(
                params![
                    task_id.to_string(),
                    to_i64(after)?,
                    i64::try_from(max).map_err(storage)?
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?;
        rows.map(|x| decode::<EventRecord>(&x.map_err(storage)?))
            .collect()
    }

    pub fn recent_events(&self, task_id: Uuid, max: usize) -> Result<Vec<EventRecord>> {
        if !(1..=1024).contains(&max) {
            return Err(FoundationError::InvalidInput("event page size"));
        }
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = connection
            .prepare(
                "SELECT body FROM foundation_events WHERE task_id=?1 ORDER BY seq DESC LIMIT ?2",
            )
            .map_err(storage)?;
        let rows = stmt
            .query_map(
                params![task_id.to_string(), i64::try_from(max).map_err(storage)?],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?;
        let mut result: Vec<EventRecord> = rows
            .map(|x| decode::<EventRecord>(&x.map_err(storage)?))
            .collect::<Result<_>>()?;
        result.reverse();
        Ok(result)
    }

    pub fn pending_outbox(&self, max: usize) -> Result<Vec<EventRecord>> {
        if !(1..=1024).contains(&max) {
            return Err(FoundationError::InvalidInput("outbox page size"));
        }
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = connection.prepare(
            "SELECT body FROM foundation_outbox WHERE delivered_ms IS NULL ORDER BY rowid LIMIT ?1"
        ).map_err(storage)?;
        let rows = stmt
            .query_map(params![i64::try_from(max).map_err(storage)?], |row| {
                row.get::<_, String>(0)
            })
            .map_err(storage)?;
        rows.map(|x| decode::<EventRecord>(&x.map_err(storage)?))
            .collect()
    }

    pub fn mark_outbox_delivered(&self, event_id: Uuid) -> Result<()> {
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let changed = connection.execute(
            "UPDATE foundation_outbox SET delivered_ms=?1 WHERE event_id=?2 AND delivered_ms IS NULL",
            params![epoch_ms()?,event_id.to_string()]
        ).map_err(storage)?;
        if changed != 1 {
            return Err(FoundationError::Storage(
                "outbox item missing or delivered".into(),
            ));
        }
        Ok(())
    }

    /// Record external call intent DURABLY before dispatch. Every subsequent
    /// timeout, cancellation or crash leaves an Unknown receipt, never retries.
    pub fn begin_effect(
        &self,
        task_id: Uuid,
        call_id: Uuid,
        key: &str,
        effect: ToolEffect,
        tool_name: &str,
    ) -> Result<()> {
        self.begin_effect_checked(task_id, call_id, key, effect, tool_name, None, None)
    }

    pub(crate) fn begin_effect_at_revision(
        &self,
        task_id: Uuid,
        call_id: Uuid,
        key: &str,
        effect: ToolEffect,
        tool_name: &str,
        revision: u64,
    ) -> Result<()> {
        self.begin_effect_checked(task_id, call_id, key, effect, tool_name, Some(revision), None)
    }

    pub(crate) fn begin_provider_effect_at_revision(
        &self,
        task_id: Uuid,
        call_id: Uuid,
        key: &str,
        tool_name: &str,
        revision: u64,
        authority: &str,
    ) -> Result<()> {
        if authority.len() != 64 || !authority.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(FoundationError::InvalidInput("provider authority digest"));
        }
        self.begin_effect_checked(
            task_id,
            call_id,
            key,
            ToolEffect::ExternalSideEffect,
            tool_name,
            Some(revision),
            Some(authority),
        )
    }

    fn ensure_no_unknown_effect(connection: &Connection, task_id: Uuid) -> Result<()> {
        let unknown: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM foundation_effects WHERE task_id=?1 AND state='unknown')",
            params![task_id.to_string()],
            |row| row.get(0),
        ).map_err(storage)?;
        if unknown {
            return Err(FoundationError::Storage("unreconciled task effect".into()));
        }
        Ok(())
    }

    fn require_running_revision(
        connection: &Connection,
        task_id: Uuid,
        revision: u64,
    ) -> Result<()> {
        let body: Option<String> = connection
            .query_row(
                "SELECT body FROM foundation_tasks WHERE id=?1 AND revision=?2",
                params![task_id.to_string(), to_i64(revision)?],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        let task: TaskRecord = decode(&body.ok_or(FoundationError::InvalidTransition)?)?;
        if task.id != task_id || task.revision != revision || task.status != TaskStatus::Running {
            return Err(FoundationError::InvalidTransition);
        }
        Ok(())
    }

    fn begin_effect_checked(
        &self,
        task_id: Uuid,
        call_id: Uuid,
        key: &str,
        effect: ToolEffect,
        tool_name: &str,
        revision: Option<u64>,
        authority: Option<&str>,
    ) -> Result<()> {
        if key.is_empty() || key.len() > 256 || effect == ToolEffect::ReadOnly {
            return Err(FoundationError::InvalidInput("effect intent"));
        }
        let mut connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if let Some(revision) = revision {
            Self::require_running_revision(&tx, task_id, revision)?;
        }
        Self::ensure_no_unknown_effect(&tx, task_id)?;
        tx.execute(
            "INSERT INTO foundation_effects(call_id,task_id,effect_key,effect,tool,state,started_ms)
             VALUES(?1,?2,?3,?4,?5,'unknown',?6)",
            params![call_id.to_string(),task_id.to_string(),key,encode(&effect)?,tool_name,epoch_ms()?]
        ).map_err(storage)?;
        if let Some(authority) = authority {
            tx.execute(
                "INSERT INTO foundation_effect_authorities(call_id,authority) VALUES(?1,?2)",
                params![call_id.to_string(), authority],
            )
            .map_err(storage)?;
        }
        tx.commit().map_err(storage)?;
        Ok(())
    }

    pub fn confirm_effect(&self, call_id: Uuid) -> Result<()> {
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let changed = connection.execute(
            "UPDATE foundation_effects SET state='confirmed' WHERE call_id=?1 AND state='unknown'",
            params![call_id.to_string()]
        ).map_err(storage)?;
        if changed != 1 {
            return Err(FoundationError::Storage(
                "effect not in unknown state".into(),
            ));
        }
        Ok(())
    }

    pub fn reconcile_effect(
        &self,
        call_id: Uuid,
        applied: bool,
        external_evidence: &str,
    ) -> Result<()> {
        if external_evidence.trim().is_empty() || external_evidence.len() > 4096 {
            return Err(FoundationError::InvalidInput("reconciliation evidence"));
        }
        let state = if applied {
            "reconciled_succeeded"
        } else {
            "reconciled_not_applied"
        };
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let changed = connection.execute(
            "UPDATE foundation_effects SET state=?1,evidence=?2 WHERE call_id=?3 AND state='unknown'
             AND NOT EXISTS(SELECT 1 FROM foundation_leases WHERE task_id=foundation_effects.task_id)",
            params![state,external_evidence,call_id.to_string()]
        ).map_err(storage)?;
        if changed != 1 {
            return Err(FoundationError::Storage(
                "effect not in unknown state".into(),
            ));
        }
        Ok(())
    }

    /// Return only an originally bound, unresolved external operation whose
    /// execution lease has retired. An expired lease is not retirement.
    pub(crate) fn provider_query(
        &self,
        call_id: Uuid,
    ) -> Result<Option<(String, String, String)>> {
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        connection.query_row(
            "SELECT e.tool,e.effect_key,a.authority FROM foundation_effects e
             JOIN foundation_effect_authorities a ON a.call_id=e.call_id
             WHERE e.call_id=?1 AND e.state='unknown' AND e.effect=?2
             AND NOT EXISTS(SELECT 1 FROM foundation_leases l WHERE l.task_id=e.task_id)",
            params![call_id.to_string(), encode(&ToolEffect::ExternalSideEffect)?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional().map_err(storage)
    }

    /// One compare-and-set across the original provider binding, exact
    /// idempotency key, unknown state, and absence of a live execution owner.
    pub(crate) fn reconcile_provider_effect(
        &self,
        call_id: Uuid,
        tool: &str,
        key: &str,
        authority: &str,
        applied: bool,
        evidence_hash: [u8; 32],
    ) -> Result<()> {
        let state = if applied { "reconciled_succeeded" } else { "reconciled_not_applied" };
        let evidence = format!("provider-sha256:{}", hex::encode(evidence_hash));
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let changed = connection.execute(
            "UPDATE foundation_effects SET state=?1,evidence=?2
             WHERE call_id=?3 AND tool=?4 AND effect_key=?5 AND state='unknown' AND effect=?6
             AND EXISTS(SELECT 1 FROM foundation_effect_authorities a
                        WHERE a.call_id=foundation_effects.call_id AND a.authority=?7)
             AND NOT EXISTS(SELECT 1 FROM foundation_leases l WHERE l.task_id=foundation_effects.task_id)",
            params![state,evidence,call_id.to_string(),tool,key,encode(&ToolEffect::ExternalSideEffect)?,authority],
        ).map_err(storage)?;
        if changed != 1 {
            return Err(FoundationError::InvalidTransition);
        }
        Ok(())
    }

    pub fn unknown_effects(&self) -> Result<Vec<EffectReceipt>> {
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = connection.prepare(
            "SELECT task_id,call_id,effect_key,effect,tool,state,evidence FROM foundation_effects WHERE state='unknown' ORDER BY started_ms"
        ).map_err(storage)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(storage)?;
        rows.map(|x| {
            let (task, call, key, effect, tool, state, evidence) = x.map_err(storage)?;
            Ok(EffectReceipt {
                task_id: Uuid::parse_str(&task).map_err(storage)?,
                call_id: Uuid::parse_str(&call).map_err(storage)?,
                key,
                effect: decode(&effect)?,
                tool_name: tool,
                state: match state.as_str() {
                    "unknown" => EffectState::Unknown,
                    _ => return Err(FoundationError::Storage("unknown effect state".into())),
                },
                evidence,
            })
        })
        .collect()
    }

    /// Cross-process admission. The stored deadline is diagnostic only:
    /// expiration is not proof that the previous execution has stopped.
    /// Until the trusted host supplies a protected crash-recovery boundary,
    /// orphaned leases fail closed rather than allowing overlapping workers.
    pub fn acquire_lease(&self, task_id: Uuid, revision: u64) -> Result<Uuid> {
        let mut connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        Self::require_running_revision(&tx, task_id, revision)?;
        Self::ensure_no_unknown_effect(&tx, task_id)?;
        let token = Uuid::new_v4();
        let now = epoch_ms()?;
        let expires = now
            .checked_add(180_000)
            .ok_or_else(|| FoundationError::Storage("lease deadline overflow".into()))?;
        let changed = tx
            .execute(
                "INSERT INTO foundation_leases(task_id,token,expires_ms) VALUES(?1,?2,?3)
                 ON CONFLICT(task_id) DO NOTHING",
                params![task_id.to_string(), token.to_string(), expires],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(FoundationError::InvalidTransition);
        }
        tx.commit().map_err(storage)?;
        Ok(token)
    }

    pub fn release_lease(&self, task_id: Uuid, token: Uuid) {
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let _ = connection.execute(
            "DELETE FROM foundation_leases WHERE task_id=?1 AND token=?2",
            params![task_id.to_string(), token.to_string()],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event::EventKind, task::TaskRegistry};
    use std::sync::Arc;

    #[test]
    fn atomic_commit_outbox_and_crash_recovery() {
        let store = Arc::new(SqliteTaskStore::in_memory().unwrap());
        let registry =
            TaskRegistry::new_durable(store.clone(), Arc::new(crate::event::EventJournal::new(32)))
                .unwrap();
        let task = registry
            .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "Goal", vec![])
            .unwrap();
        registry.start(task.id).unwrap();
        let events = store.events_since(task.id, 0, 100).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, EventKind::TaskCreated);
        assert_eq!(store.pending_outbox(100).unwrap().len(), 2);
        let duplicate = store.commit(
            &registry.get(task.id).unwrap(),
            Some(0),
            &[EventKind::Interrupted],
        );
        assert!(duplicate.is_err());
        assert_eq!(store.events_since(task.id, 0, 100).unwrap().len(), 2);
        let recovered =
            TaskRegistry::new_durable(store.clone(), Arc::new(crate::event::EventJournal::new(32)))
                .unwrap();
        assert_eq!(recovered.get(task.id).unwrap().status, TaskStatus::Paused);
        assert_eq!(store.events_since(task.id, 0, 100).unwrap().len(), 4);
    }

    #[test]
    fn effect_intent_is_never_replayed_or_silently_retried() {
        let db = SqliteTaskStore::in_memory().unwrap();
        let task = Uuid::new_v4();
        let call = Uuid::new_v4();
        db.begin_effect(
            task,
            call,
            &format!("effect:{call}"),
            ToolEffect::ExternalSideEffect,
            "test.send",
        )
        .unwrap();
        assert_eq!(db.unknown_effects().unwrap().len(), 1);
        assert!(db
            .begin_effect(
                task,
                call,
                &format!("effect:{call}"),
                ToolEffect::ExternalSideEffect,
                "test.send"
            )
            .is_err());
        db.reconcile_effect(call, false, "provider checked transaction 123")
            .unwrap();
        assert_eq!(db.unknown_effects().unwrap().len(), 0);
        assert!(db.confirm_effect(call).is_err());
    }

    #[test]
    fn injected_outbox_insert_failure_rolls_back_task_and_events() {
        let db = SqliteTaskStore::in_memory().unwrap();
        let task = TaskRecord {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            goal: "rollback".into(),
            context: vec![],
            status: TaskStatus::Ready,
            revision: 0,
            pending: None,
        };
        db.commit(&task, None, &[EventKind::TaskCreated]).unwrap();
        db.connection
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .execute_batch(
                "CREATE TRIGGER fail_outbox BEFORE INSERT ON foundation_outbox
                BEGIN SELECT RAISE(ABORT,'simulated outbox write failure'); END;",
            )
            .unwrap();
        let mut updated = task.clone();
        updated.status = TaskStatus::Running;
        updated.revision += 1;
        let result = db.commit(
            &updated,
            Some(0),
            &[EventKind::StateChanged {
                from: TaskStatus::Ready,
                to: TaskStatus::Running,
            }],
        );
        assert!(result.is_err());
        assert_eq!(db.load_tasks().unwrap()[0].revision, 0);
        assert_eq!(db.events_since(task.id, 0, 100).unwrap().len(), 1);
        assert_eq!(db.pending_outbox(100).unwrap().len(), 1);
    }

    #[test]
    fn active_cross_process_execution_lease_blocks_recovery() {
        let db = Arc::new(SqliteTaskStore::in_memory().unwrap());
        let tasks =
            TaskRegistry::new_durable(db.clone(), Arc::new(crate::event::EventJournal::new(32)))
                .unwrap();
        let task = tasks
            .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "lease", vec![])
            .unwrap();
        tasks.start(task.id).unwrap();
        let lease = db
            .acquire_lease(task.id, tasks.get(task.id).unwrap().revision)
            .unwrap();
        assert!(db.recover_paused().is_err());
        db.release_lease(task.id, lease);
        assert_eq!(db.recover_paused().unwrap()[0].status, TaskStatus::Paused);
    }

    #[test]
    fn recovery_rolls_back_every_task_if_later_publication_fails() {
        let db = SqliteTaskStore::in_memory().unwrap();
        for id in [Uuid::from_u128(1), Uuid::from_u128(2)] {
            let task = TaskRecord {
                id,
                session_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                goal: "interrupted".into(),
                context: vec![],
                status: TaskStatus::Running,
                revision: 1,
                pending: None,
            };
            db.commit(&task, None, &[EventKind::TaskCreated]).unwrap();
        }
        db.connection
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .execute_batch(
                "CREATE TRIGGER fail_second_recovery BEFORE UPDATE ON foundation_tasks
             WHEN OLD.id='00000000-0000-0000-0000-000000000002'
             BEGIN SELECT RAISE(ABORT,'simulated second recovery failure'); END;",
            )
            .unwrap();
        assert!(db.recover_paused().is_err());
        for task in db.load_tasks().unwrap() {
            assert_eq!(task.status, TaskStatus::Running);
            assert_eq!(task.revision, 1);
            assert_eq!(db.events_since(task.id, 0, 100).unwrap().len(), 1);
        }
        assert_eq!(db.pending_outbox(100).unwrap().len(), 2);
    }

    #[test]
    fn expired_lease_cannot_be_stolen_or_recovered_by_time_alone() {
        let db = Arc::new(SqliteTaskStore::in_memory().unwrap());
        let tasks =
            TaskRegistry::new_durable(db.clone(), Arc::new(crate::event::EventJournal::new(32)))
                .unwrap();
        let task = tasks
            .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "lease", vec![])
            .unwrap();
        tasks.start(task.id).unwrap();
        let revision = tasks.get(task.id).unwrap().revision;
        let lease = db.acquire_lease(task.id, revision).unwrap();
        db.connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE foundation_leases SET expires_ms=0 WHERE task_id=?1",
                params![task.id.to_string()],
            )
            .unwrap();
        assert!(db.acquire_lease(task.id, revision).is_err());
        assert!(db.recover_paused().is_err());
        db.release_lease(task.id, lease);
        assert_eq!(db.recover_paused().unwrap()[0].status, TaskStatus::Paused);
    }

    #[test]
    fn unknown_effect_blocks_new_execution_until_reconciled() {
        let db = Arc::new(SqliteTaskStore::in_memory().unwrap());
        let tasks =
            TaskRegistry::new_durable(db.clone(), Arc::new(crate::event::EventJournal::new(32)))
                .unwrap();
        let task = tasks
            .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "effect", vec![])
            .unwrap();
        tasks.start(task.id).unwrap();
        let revision = tasks.get(task.id).unwrap().revision;
        let call = Uuid::new_v4();
        db.begin_effect_at_revision(
            task.id,
            call,
            "first",
            ToolEffect::ExternalSideEffect,
            "test.send",
            revision,
        )
        .unwrap();
        assert!(db
            .begin_effect_at_revision(
                task.id,
                Uuid::new_v4(),
                "second",
                ToolEffect::ExternalSideEffect,
                "test.send",
                revision
            )
            .is_err());
        assert!(db.acquire_lease(task.id, revision).is_err());
        tasks.pause(task.id).unwrap();
        assert!(tasks.start(task.id).is_err());
        db.reconcile_effect(call, false, "trusted provider confirms not applied")
            .unwrap();
        tasks.start(task.id).unwrap();
        let new_revision = tasks.get(task.id).unwrap().revision;
        assert!(db
            .begin_effect_at_revision(
                task.id,
                Uuid::new_v4(),
                "stale",
                ToolEffect::ExternalSideEffect,
                "test.send",
                revision
            )
            .is_err());
        let lease = db.acquire_lease(task.id, new_revision).unwrap();
        db.release_lease(task.id, lease);
    }

    #[test]
    fn outbox_delivery_requires_existing_undelivered_record() {
        let db = SqliteTaskStore::in_memory().unwrap();
        assert!(db.mark_outbox_delivered(Uuid::new_v4()).is_err());
    }
}
