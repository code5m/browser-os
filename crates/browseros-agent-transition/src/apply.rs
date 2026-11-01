//! Compensating multi-file apply (a saga, NOT an atomic filesystem transaction).
//! All side effects, durable journals, fsync and CAS checks are host-owned.
//! An uncertain write is never blindly retried or overwritten.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_FILES: usize = 32;
const MAX_CONTENT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEdit {
    pub path: String,
    /// Opaque, host-verified revision. None means that the file must not exist.
    pub expected_revision: Option<String>,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPlan {
    pub apply_id: String,
    pub edits: Vec<FileEdit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub revision: Option<String>,
    pub contents: Option<String>,
    /// Host-provided provenance; needed to resolve interrupted writes safely.
    pub last_operation_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepPhase {
    Prepared,
    MayHaveWritten,
    Applied,
    Restored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyStatus {
    Prepared,
    Applying,
    Applied,
    Compensating,
    Compensated,
    RecoveryRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyStep {
    pub edit: FileEdit,
    pub before: FileSnapshot,
    /// Unique operation identity MUST be recorded by the host write adapter.
    pub operation_id: String,
    pub phase: StepPhase,
    pub after: Option<FileSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyJournal {
    pub apply_id: String,
    pub status: ApplyStatus,
    pub steps: Vec<ApplyStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    AlreadyApplied,
    Compensated,
    AlreadyCompensated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyError {
    InvalidPlan,
    DuplicateId,
    Conflict(String),
    MissingJournal,
    Journal(String),
    File(String),
    PendingRecovery,
    Aborted(String),
    RecoveryRequired(String),
}

/// A real adapter MUST implement symlink-safe path resolution, hash/generation
/// compare-and-swap, ownership checks and durable per-operation provenance.
/// replace() may fail AFTER a physical write, so recovery must inspect state.
pub trait FilePort {
    fn inspect(&mut self, path: &str) -> Result<FileSnapshot, String>;
    fn replace(
        &mut self,
        path: &str,
        expected_revision: Option<&str>,
        replacement: Option<&str>,
        operation_id: &str,
    ) -> Result<FileSnapshot, String>;
}

/// store() MUST atomically persist + fsync the journal before returning Ok.
/// Journals include original file contents and must be access-controlled.
pub trait JournalPort {
    fn load(&self, apply_id: &str) -> Result<Option<ApplyJournal>, String>;
    fn store(&mut self, journal: &ApplyJournal) -> Result<(), String>;
}

pub struct ApplyCoordinator;

impl ApplyCoordinator {
    pub fn apply(
        files: &mut impl FilePort,
        journals: &mut impl JournalPort,
        plan: ApplyPlan,
    ) -> Result<ApplyOutcome, ApplyError> {
        validate(&plan)?;
        if let Some(existing) = journals.load(&plan.apply_id).map_err(ApplyError::Journal)? {
            let same_plan = existing.steps.len() == plan.edits.len()
                && existing
                    .steps
                    .iter()
                    .zip(&plan.edits)
                    .all(|(step, edit)| step.edit == *edit);
            if !same_plan {
                return Err(ApplyError::DuplicateId);
            }
            return match existing.status {
                ApplyStatus::Applied => Ok(ApplyOutcome::AlreadyApplied),
                ApplyStatus::Compensated => Ok(ApplyOutcome::AlreadyCompensated),
                _ => Err(ApplyError::PendingRecovery),
            };
        }

        // Preflight ALL files, including originals needed for compensation.
        let mut steps = Vec::with_capacity(plan.edits.len());
        for (index, edit) in plan.edits.into_iter().enumerate() {
            let before = files.inspect(&edit.path).map_err(ApplyError::File)?;
            if before.revision != edit.expected_revision {
                return Err(ApplyError::Conflict(edit.path));
            }
            steps.push(ApplyStep {
                operation_id: format!("apply:{}:{}", plan.apply_id, index),
                edit,
                before,
                phase: StepPhase::Prepared,
                after: None,
            });
        }
        let mut journal = ApplyJournal {
            apply_id: plan.apply_id,
            status: ApplyStatus::Prepared,
            steps,
        };
        journals.store(&journal).map_err(ApplyError::Journal)?;

        for index in 0..journal.steps.len() {
            // Write-ahead mark BEFORE the file operation; if process crashes,
            // recovery can distinguish no write / our write / external change.
            journal.status = ApplyStatus::Applying;
            journal.steps[index].phase = StepPhase::MayHaveWritten;
            if let Err(error) = journals.store(&journal) {
                return Err(abort(files, journals, &mut journal, error));
            }
            let step = &journal.steps[index];
            let result = files.replace(
                &step.edit.path,
                step.before.revision.as_deref(),
                Some(step.edit.replacement.as_str()),
                &step.operation_id,
            );
            match result {
                Ok(after)
                    if after.contents.as_deref() == Some(step.edit.replacement.as_str())
                        && after.last_operation_id.as_deref()
                            == Some(step.operation_id.as_str()) =>
                {
                    journal.steps[index].after = Some(after);
                    journal.steps[index].phase = StepPhase::Applied;
                    if let Err(error) = journals.store(&journal) {
                        return Err(abort(files, journals, &mut journal, error));
                    }
                }
                Ok(_) => {
                    return Err(abort(
                        files,
                        journals,
                        &mut journal,
                        "write proof missing".into(),
                    ));
                }
                Err(error) => return Err(abort(files, journals, &mut journal, error)),
            }
        }
        journal.status = ApplyStatus::Applied;
        if let Err(error) = journals.store(&journal) {
            return Err(abort(files, journals, &mut journal, error));
        }
        Ok(ApplyOutcome::Applied)
    }

    /// Recover an interrupted or failed apply by conditional compensation.
    /// Applied journal entries are never automatically undone on re-open.
    pub fn recover(
        files: &mut impl FilePort,
        journals: &mut impl JournalPort,
        apply_id: &str,
    ) -> Result<ApplyOutcome, ApplyError> {
        let mut journal = journals
            .load(apply_id)
            .map_err(ApplyError::Journal)?
            .ok_or(ApplyError::MissingJournal)?;
        match journal.status {
            ApplyStatus::Applied => return Ok(ApplyOutcome::AlreadyApplied),
            ApplyStatus::Compensated => return Ok(ApplyOutcome::AlreadyCompensated),
            _ => {}
        }
        compensate(files, journals, &mut journal)?;
        Ok(ApplyOutcome::Compensated)
    }
}

fn validate(plan: &ApplyPlan) -> Result<(), ApplyError> {
    if plan.apply_id.trim().is_empty()
        || plan.apply_id.len() > 128
        || plan.edits.is_empty()
        || plan.edits.len() > MAX_FILES
    {
        return Err(ApplyError::InvalidPlan);
    }
    let mut names = BTreeSet::new();
    for edit in &plan.edits {
        let path = edit.path.as_str();
        if path.is_empty()
            || path.starts_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|part| part == ".." || part == "." || part.is_empty())
            || edit.replacement.len() > MAX_CONTENT_BYTES
            || !names.insert(path)
        {
            return Err(ApplyError::InvalidPlan);
        }
    }
    Ok(())
}

fn abort(
    files: &mut impl FilePort,
    journals: &mut impl JournalPort,
    journal: &mut ApplyJournal,
    cause: String,
) -> ApplyError {
    match compensate(files, journals, journal) {
        Ok(()) => ApplyError::Aborted(cause),
        Err(error) => ApplyError::RecoveryRequired(format!("{cause}; {error:?}")),
    }
}

fn fail_recovery(
    journals: &mut impl JournalPort,
    journal: &mut ApplyJournal,
    reason: String,
) -> ApplyError {
    journal.status = ApplyStatus::RecoveryRequired;
    if let Err(error) = journals.store(journal) {
        return ApplyError::RecoveryRequired(format!("{reason}; journal: {error}"));
    }
    ApplyError::RecoveryRequired(reason)
}

fn compensate(
    files: &mut impl FilePort,
    journals: &mut impl JournalPort,
    journal: &mut ApplyJournal,
) -> Result<(), ApplyError> {
    journal.status = ApplyStatus::Compensating;
    journals.store(journal).map_err(|error| {
        fail_recovery(
            journals,
            journal,
            format!("cannot mark compensation: {error}"),
        )
    })?;
    for index in (0..journal.steps.len()).rev() {
        let step = &journal.steps[index];
        if step.phase == StepPhase::Prepared || step.phase == StepPhase::Restored {
            continue;
        }
        let actual = match files.inspect(&step.edit.path) {
            Ok(snapshot) => snapshot,
            Err(error) => return Err(fail_recovery(journals, journal, error)),
        };
        let step = &journal.steps[index];
        let rollback_operation = format!("rollback:{}:{}", journal.apply_id, index);
        // The rollback itself might have succeeded before its adapter failed.
        // Recognize only OUR rollback provenance, never matching bytes alone.
        let already_restored = actual == step.before
            || (actual.contents == step.before.contents
                && actual.last_operation_id.as_deref() == Some(rollback_operation.as_str()));
        if already_restored {
            journal.steps[index].phase = StepPhase::Restored;
            if let Err(error) = journals.store(journal) {
                return Err(fail_recovery(journals, journal, error));
            }
            continue;
        }
        // For an interrupted write, a matching byte string alone is NOT enough:
        // demand the exact durable operation identity to avoid clobbering edits.
        let our_write = actual.last_operation_id.as_deref() == Some(step.operation_id.as_str())
            && actual.contents.as_deref() == Some(step.edit.replacement.as_str())
            && (step.phase == StepPhase::MayHaveWritten || step.after.as_ref() == Some(&actual));
        if !our_write {
            let reason = format!(
                "external modification or ambiguous write at {}",
                step.edit.path
            );
            return Err(fail_recovery(journals, journal, reason));
        }
        let restored = files.replace(
            &step.edit.path,
            actual.revision.as_deref(),
            step.before.contents.as_deref(),
            &rollback_operation,
        );
        match restored {
            Ok(snapshot)
                if snapshot.contents == step.before.contents
                    && (step.before.contents.is_none()
                        || snapshot.last_operation_id.as_deref()
                            == Some(rollback_operation.as_str())) => {}
            Ok(_) => {
                return Err(fail_recovery(
                    journals,
                    journal,
                    "rollback result could not be verified".into(),
                ))
            }
            Err(error) => return Err(fail_recovery(journals, journal, error)),
        }
        journal.steps[index].phase = StepPhase::Restored;
        if let Err(error) = journals.store(journal) {
            return Err(fail_recovery(journals, journal, error));
        }
    }
    journal.status = ApplyStatus::Compensated;
    journals
        .store(journal)
        .map_err(|error| ApplyError::RecoveryRequired(error))
}
