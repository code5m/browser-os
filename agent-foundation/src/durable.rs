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
        tx.commit().map_err(storage)?;
        Ok(committed)
    }

    pub fn load_tasks(&self) -> Result<Vec<TaskRecord>> {
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
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
        // Do not steal a live worker's task on a second process attach.
        // An expired lease can be recovered, but never resumed automatically.
        let live: i64 = {
            let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
            connection
                .query_row(
                    "SELECT COUNT(*) FROM foundation_leases WHERE expires_ms>?1",
                    params![epoch_ms()?],
                    |row| row.get(0),
                )
                .map_err(storage)?
        };
        if live != 0 {
            return Err(FoundationError::Storage(
                "another worker has an active task lease".into(),
            ));
        }
        let tasks = self.load_tasks()?;
        let mut recovered = Vec::with_capacity(tasks.len());
        for mut task in tasks {
            let previous = task.status;
            if matches!(previous, TaskStatus::Running | TaskStatus::AwaitingApproval) {
                let old = task.revision;
                task.revision += 1;
                task.status = TaskStatus::Paused;
                task.pending = None;
                self.commit(
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
        if key.is_empty() || key.len() > 256 || effect == ToolEffect::ReadOnly {
            return Err(FoundationError::InvalidInput("effect intent"));
        }
        let connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        connection.execute(
            "INSERT INTO foundation_effects(call_id,task_id,effect_key,effect,tool,state,started_ms)
             VALUES(?1,?2,?3,?4,?5,'unknown',?6)",
            params![call_id.to_string(),task_id.to_string(),key,encode(&effect)?,tool_name,epoch_ms()?]
        ).map_err(storage)?;
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
            "UPDATE foundation_effects SET state=?1,evidence=?2 WHERE call_id=?3 AND state='unknown'",
            params![state,external_evidence,call_id.to_string()]
        ).map_err(storage)?;
        if changed != 1 {
            return Err(FoundationError::Storage(
                "effect not in unknown state".into(),
            ));
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

    /// Cross-process exclusion for the existing 120-second runtime limit;
    /// stale leases only expire after 180 seconds. Never use as a fencing
    /// mechanism for remote side effects (their idempotency belongs upstream).
    pub fn acquire_lease(&self, task_id: Uuid, revision: u64) -> Result<Uuid> {
        let mut connection = self.connection.lock().unwrap_or_else(|e| e.into_inner());
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let stored: Option<i64> = tx
            .query_row(
                "SELECT revision FROM foundation_tasks WHERE id=?1",
                params![task_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        if stored != Some(to_i64(revision)?) {
            return Err(FoundationError::InvalidTransition);
        }
        let token = Uuid::new_v4();
        let now = epoch_ms()?;
        let changed = tx
            .execute(
                "INSERT INTO foundation_leases(task_id,token,expires_ms) VALUES(?1,?2,?3)
             ON CONFLICT(task_id) DO UPDATE SET token=excluded.token,expires_ms=excluded.expires_ms
             WHERE foundation_leases.expires_ms<=?4",
                params![task_id.to_string(), token.to_string(), now + 180_000, now],
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
    fn outbox_delivery_requires_existing_undelivered_record() {
        let db = SqliteTaskStore::in_memory().unwrap();
        assert!(db.mark_outbox_delivered(Uuid::new_v4()).is_err());
    }
}
