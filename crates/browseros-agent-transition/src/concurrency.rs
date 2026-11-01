//! Short-lived in-process revision checks and exclusive file leases.
//! Model/provider requests MUST run without holding these guards.
//! Cross-process durability and canonical file ownership belong to the host.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConcurrencyError {
    InvalidKey,
    Conflict,
    RevisionOverflow,
    Poisoned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionTicket {
    pub workspace: String,
    pub session_id: String,
    pub expected_revision: u64,
}

#[derive(Default)]
pub struct SessionRevisions {
    revisions: Mutex<BTreeMap<(String, String), u64>>,
}

impl SessionRevisions {
    /// No long-running lock escapes this method; inference may run in parallel.
    pub fn begin(&self, workspace: &str, session_id: &str) -> Result<SessionTicket, ConcurrencyError> {
        if workspace.trim().is_empty() || session_id.trim().is_empty() {
            return Err(ConcurrencyError::InvalidKey);
        }
        let revisions = self.revisions.lock().map_err(|_| ConcurrencyError::Poisoned)?;
        Ok(SessionTicket {
            workspace: workspace.into(),
            session_id: session_id.into(),
            expected_revision: *revisions
                .get(&(workspace.into(), session_id.into()))
                .unwrap_or(&0),
        })
    }

    /// CAS only at session state commit, not while invoking the model.
    pub fn commit(&self, ticket: &SessionTicket) -> Result<u64, ConcurrencyError> {
        let mut revisions = self.revisions.lock().map_err(|_| ConcurrencyError::Poisoned)?;
        let key = (ticket.workspace.clone(), ticket.session_id.clone());
        let current = *revisions.get(&key).unwrap_or(&0);
        if current != ticket.expected_revision {
            return Err(ConcurrencyError::Conflict);
        }
        let next = current.checked_add(1).ok_or(ConcurrencyError::RevisionOverflow)?;
        revisions.insert(key, next);
        Ok(next)
    }
}

#[derive(Clone, Default)]
pub struct FileLeases {
    active: Arc<Mutex<BTreeSet<(String, String)>>>,
}

pub struct FileLease {
    active: Arc<Mutex<BTreeSet<(String, String)>>>,
    keys: Vec<(String, String)>,
}

impl FileLeases {
    /// Acquire all paths or none. Call with host-validated canonical paths.
    pub fn acquire(&self, workspace: &str, paths: &[String]) -> Result<FileLease, ConcurrencyError> {
        if workspace.trim().is_empty() || paths.is_empty() {
            return Err(ConcurrencyError::InvalidKey);
        }
        let mut keys = BTreeSet::new();
        for path in paths {
            if path.trim().is_empty()
                || path.starts_with('/')
                || path.contains('\\')
                || path.split('/').any(|segment| segment == ".." || segment == ".")
                || !keys.insert((workspace.to_owned(), path.clone()))
            {
                return Err(ConcurrencyError::InvalidKey);
            }
        }
        let mut active = self.active.lock().map_err(|_| ConcurrencyError::Poisoned)?;
        if keys.iter().any(|key| active.contains(key)) {
            return Err(ConcurrencyError::Conflict);
        }
        active.extend(keys.iter().cloned());
        Ok(FileLease {
            active: Arc::clone(&self.active),
            keys: keys.into_iter().collect(),
        })
    }
}

impl Drop for FileLease {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.lock() {
            for key in &self.keys {
                active.remove(key);
            }
        }
    }
}
