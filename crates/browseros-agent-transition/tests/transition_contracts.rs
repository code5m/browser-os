use std::collections::BTreeMap;

use browseros_agent_transition::apply::{
    ApplyCoordinator, ApplyError, ApplyJournal, ApplyOutcome, ApplyPlan, ApplyStatus, ApplyStep,
    FileEdit, FilePort, FileSnapshot, JournalPort, StepPhase,
};
use browseros_agent_transition::concurrency::{ConcurrencyError, FileLeases, SessionRevisions};
use browseros_agent_transition::lifecycle::{
    CapabilityMachine, CapabilityState, LifecycleAction, LifecycleError, ResourceInspection,
    ResourceLease,
};
use browseros_agent_transition::runtime::{
    CutoverMode, CutoverRouter, Effect, HostDecision, LegacyEntry, Route, RuntimeRequest, Workload,
};

#[derive(Default)]
struct FakeFiles {
    values: BTreeMap<String, FileSnapshot>,
    revision: u64,
    fail_on: Option<String>,
    fail_after_write: Option<String>,
    external_on_failure: Option<(String, String)>,
}

impl FakeFiles {
    fn seed(&mut self, path: &str, content: &str) {
        self.revision += 1;
        self.values.insert(
            path.into(),
            FileSnapshot {
                revision: Some(format!("rev-{}", self.revision)),
                contents: Some(content.into()),
                last_operation_id: None,
            },
        );
    }
    fn content(&self, path: &str) -> Option<&str> {
        self.values.get(path).and_then(|s| s.contents.as_deref())
    }
}

impl FilePort for FakeFiles {
    fn inspect(&mut self, path: &str) -> Result<FileSnapshot, String> {
        Ok(self.values.get(path).cloned().unwrap_or(FileSnapshot {
            revision: None,
            contents: None,
            last_operation_id: None,
        }))
    }

    fn replace(
        &mut self,
        path: &str,
        expected_revision: Option<&str>,
        replacement: Option<&str>,
        operation_id: &str,
    ) -> Result<FileSnapshot, String> {
        let current = self.inspect(path)?;
        if current.revision.as_deref() != expected_revision {
            return Err(format!("file version conflict at {path}"));
        }
        if self.fail_on.as_deref() == Some(path) {
            self.fail_on = None;
            if let Some((external_path, text)) = self.external_on_failure.take() {
                self.seed(&external_path, &text);
            }
            return Err(format!("injected write failure at {path}"));
        }
        self.revision += 1;
        let next = FileSnapshot {
            revision: replacement.map(|_| format!("rev-{}", self.revision)),
            contents: replacement.map(str::to_owned),
            last_operation_id: replacement.map(|_| operation_id.to_owned()),
        };
        if replacement.is_some() {
            self.values.insert(path.into(), next.clone());
        } else {
            self.values.remove(path);
        }
        if self.fail_after_write.as_deref() == Some(path) {
            self.fail_after_write = None;
            return Err(format!("injected post-write failure at {path}"));
        }
        Ok(next)
    }
}

#[derive(Default)]
struct FakeJournals {
    items: BTreeMap<String, ApplyJournal>,
    saves: usize,
    fail_at_save: Option<usize>,
}

impl JournalPort for FakeJournals {
    fn load(&self, id: &str) -> Result<Option<ApplyJournal>, String> {
        Ok(self.items.get(id).cloned())
    }

    fn store(&mut self, journal: &ApplyJournal) -> Result<(), String> {
        self.saves += 1;
        if self.fail_at_save == Some(self.saves) {
            self.fail_at_save = None;
            return Err("injected journal persistence failure".into());
        }
        self.items.insert(journal.apply_id.clone(), journal.clone());
        Ok(())
    }
}

fn plan(files: &mut FakeFiles, id: &str, paths: &[&str]) -> ApplyPlan {
    let edits = paths
        .iter()
        .map(|path| FileEdit {
            path: (*path).into(),
            expected_revision: files.inspect(path).unwrap().revision,
            replacement: format!("replacement-{path}"),
        })
        .collect();
    ApplyPlan {
        apply_id: id.into(),
        edits,
    }
}

#[test]
fn compatibility_router_is_off_by_default_and_never_replays_writes() {
    let mut routes = CutoverRouter::default();
    let entry = LegacyEntry::parse("coding_service").unwrap();
    assert_eq!(
        LegacyEntry::parse("agent_chat"),
        Some(LegacyEntry::AgentChat)
    );
    assert!(LegacyEntry::parse("not_registered").is_none());
    assert_eq!(entry.workload(), Workload::Coding);
    assert_eq!(
        routes.route(entry, Effect::Write, HostDecision::ExplicitOptIn),
        Route::ExistingUnchanged
    );
    assert_eq!(
        routes.route(LegacyEntry::AgentChat, Effect::Infer, HostDecision::No),
        Route::ExistingUnchanged
    );
    routes.set_policy(entry, CutoverMode::ShadowReads);
    assert_eq!(
        routes.route(entry, Effect::Read, HostDecision::No),
        Route::ShadowReadOnly
    );
    assert_eq!(
        routes.route(entry, Effect::Infer, HostDecision::No),
        Route::ExistingUnchanged
    );
    assert_eq!(
        routes.route(entry, Effect::Write, HostDecision::No),
        Route::ExistingUnchanged
    );
    routes.set_policy(entry, CutoverMode::OptIn);
    assert_eq!(
        routes.route(entry, Effect::Write, HostDecision::No),
        Route::ExistingUnchanged
    );
    assert_eq!(
        routes.route(entry, Effect::Write, HostDecision::ExplicitOptIn),
        Route::CandidateOnly
    );
}

#[test]
fn unified_request_never_embeds_unapproved_context_content() {
    let request = RuntimeRequest {
        task_id: "task".into(),
        session_id: "session".into(),
        workload: Workload::Database,
        effect: Effect::Read,
        instruction: "explain the selected query".into(),
        contexts: vec![],
        requested_tool: Some("db.explain".into()),
    };
    assert!(request.validate().is_ok());
    let mut invalid = request;
    invalid.requested_tool = Some(" ".into());
    assert!(invalid.validate().is_err());
}

#[test]
fn session_revisions_allow_parallel_model_calls_but_conflict_on_commit() {
    let revisions = SessionRevisions::default();
    let left = revisions.begin("workspace", "session-a").unwrap();
    let stale = revisions.begin("workspace", "session-a").unwrap();
    let other = revisions.begin("workspace", "session-b").unwrap();
    assert_eq!(revisions.commit(&left), Ok(1));
    assert_eq!(revisions.commit(&stale), Err(ConcurrencyError::Conflict));
    assert_eq!(revisions.commit(&other), Ok(1));
}

#[test]
fn file_lease_is_all_or_nothing_and_released_on_drop() {
    let leases = FileLeases::default();
    let held = leases
        .acquire("workspace", &["a.rs".into(), "b.rs".into()])
        .ok()
        .unwrap();
    assert!(matches!(
        leases.acquire("workspace", &["b.rs".into(), "c.rs".into()]),
        Err(ConcurrencyError::Conflict)
    ));
    assert!(leases.acquire("other-workspace", &["a.rs".into()]).is_ok());
    assert!(matches!(
        leases.acquire("workspace", &["../secret".into()]),
        Err(ConcurrencyError::InvalidKey)
    ));
    drop(held);
    assert!(leases
        .acquire("workspace", &["b.rs".into(), "c.rs".into()])
        .is_ok());
}

#[test]
fn multi_file_failure_rolls_back_previous_files() {
    let mut files = FakeFiles::default();
    files.seed("a.rs", "before-a");
    files.seed("b.rs", "before-b");
    files.seed("c.rs", "before-c");
    let plan = plan(&mut files, "apply-three", &["a.rs", "b.rs", "c.rs"]);
    files.fail_on = Some("c.rs".into());
    let mut journals = FakeJournals::default();
    assert!(matches!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan.clone()),
        Err(ApplyError::Aborted(_))
    ));
    assert_eq!(files.content("a.rs"), Some("before-a"));
    assert_eq!(files.content("b.rs"), Some("before-b"));
    assert_eq!(files.content("c.rs"), Some("before-c"));
    assert_eq!(
        journals.load("apply-three").unwrap().unwrap().status,
        ApplyStatus::Compensated
    );
    assert_eq!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan).unwrap(),
        ApplyOutcome::AlreadyCompensated
    );
}

#[test]
fn interrupted_unknown_write_is_recovered_only_with_operation_proof() {
    let mut files = FakeFiles::default();
    files.seed("file.rs", "original");
    let mut interrupted_plan = plan(&mut files, "interrupted", &["file.rs"]);
    let edit = interrupted_plan.edits.remove(0);
    let before = files.inspect("file.rs").unwrap();
    let op = "apply:interrupted:0";
    let mut journals = FakeJournals::default();
    journals
        .store(&ApplyJournal {
            apply_id: "interrupted".into(),
            status: ApplyStatus::Applying,
            steps: vec![ApplyStep {
                edit: edit.clone(),
                before: before.clone(),
                operation_id: op.into(),
                phase: StepPhase::MayHaveWritten,
                after: None,
            }],
        })
        .unwrap();
    files
        .replace(
            "file.rs",
            before.revision.as_deref(),
            Some(&edit.replacement),
            op,
        )
        .unwrap();
    assert_eq!(
        ApplyCoordinator::recover(&mut files, &mut journals, "interrupted").unwrap(),
        ApplyOutcome::Compensated
    );
    assert_eq!(files.content("file.rs"), Some("original"));
}

#[test]
fn post_write_error_is_compensated_not_misreported_as_success() {
    let mut files = FakeFiles::default();
    files.seed("a.rs", "before");
    let plan = plan(&mut files, "post-write-error", &["a.rs"]);
    files.fail_after_write = Some("a.rs".into());
    let mut journals = FakeJournals::default();
    assert!(matches!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan),
        Err(ApplyError::Aborted(_))
    ));
    assert_eq!(files.content("a.rs"), Some("before"));
}

#[test]
fn external_change_during_failure_is_never_overwritten_by_rollback() {
    let mut files = FakeFiles::default();
    files.seed("a.rs", "original");
    files.seed("b.rs", "original-b");
    let plan = plan(&mut files, "concurrent-edit", &["a.rs", "b.rs"]);
    files.fail_on = Some("b.rs".into());
    files.external_on_failure = Some(("a.rs".into(), "someone-else-edited".into()));
    let mut journals = FakeJournals::default();
    assert!(matches!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan),
        Err(ApplyError::RecoveryRequired(_))
    ));
    assert_eq!(files.content("a.rs"), Some("someone-else-edited"));
    assert_eq!(
        journals.load("concurrent-edit").unwrap().unwrap().status,
        ApplyStatus::RecoveryRequired
    );
}

#[test]
fn journal_failure_before_first_write_prevents_any_side_effect() {
    let mut files = FakeFiles::default();
    files.seed("a.rs", "original");
    let plan = plan(&mut files, "journal-fail", &["a.rs"]);
    let mut journals = FakeJournals {
        fail_at_save: Some(1),
        ..FakeJournals::default()
    };
    assert!(matches!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan),
        Err(ApplyError::Journal(_))
    ));
    assert_eq!(files.content("a.rs"), Some("original"));
}

#[test]
fn idempotent_apply_detects_reused_id_with_different_payload() {
    let mut files = FakeFiles::default();
    files.seed("a.rs", "old");
    let plan = plan(&mut files, "same-id", &["a.rs"]);
    let mut journals = FakeJournals::default();
    assert_eq!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan.clone()).unwrap(),
        ApplyOutcome::Applied
    );
    assert_eq!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan.clone()).unwrap(),
        ApplyOutcome::AlreadyApplied
    );
    let mut altered = plan;
    altered.edits[0].replacement.push_str(" changed");
    assert!(matches!(
        ApplyCoordinator::apply(&mut files, &mut journals, altered),
        Err(ApplyError::DuplicateId)
    ));
}

#[test]
fn hotplug_failure_keeps_failed_state_until_host_verifies_recovery() {
    let mut lifecycle = CapabilityMachine::default();
    let token = lifecycle.begin(LifecycleAction::Enable).unwrap();
    assert_eq!(lifecycle.state(), CapabilityState::Activating);
    assert_eq!(
        lifecycle.finish(
            &token,
            ResourceInspection::Verified {
                state: CapabilityState::Active,
                leases: vec![ResourceLease {
                    owner: "coding".into(),
                    resource_id: "lsp".into(),
                    generation: 1
                }],
            }
        ),
        Ok(CapabilityState::Active)
    );
    let suspend = lifecycle.begin(LifecycleAction::Suspend).unwrap();
    assert_eq!(
        lifecycle.finish(
            &suspend,
            ResourceInspection::Uncertain {
                message: "child exited but listener still registered".into(),
            }
        ),
        Err(LifecycleError::Unverified)
    );
    assert_eq!(lifecycle.state(), CapabilityState::Failed);
    assert_eq!(lifecycle.leases().len(), 1);
    assert_eq!(
        lifecycle.begin(LifecycleAction::Resume),
        Err(LifecycleError::InvalidTransition)
    );
    let recovering = lifecycle.begin_recovery().unwrap();
    assert_eq!(
        lifecycle.finish_recovery(
            &recovering,
            ResourceInspection::Verified {
                state: CapabilityState::Suspended,
                leases: vec![],
            }
        ),
        Ok(CapabilityState::Suspended)
    );
    let disable = lifecycle.begin(LifecycleAction::Disable).unwrap();
    assert_eq!(
        lifecycle.finish(
            &disable,
            ResourceInspection::Verified {
                state: CapabilityState::Disabled,
                leases: vec![],
            }
        ),
        Ok(CapabilityState::Disabled)
    );
}

#[test]
fn hotplug_rejects_stale_and_mismatched_resource_evidence() {
    let mut lifecycle = CapabilityMachine::default();
    let token = lifecycle.begin(LifecycleAction::Enable).unwrap();
    assert_eq!(
        lifecycle.finish(
            &token,
            ResourceInspection::Verified {
                state: CapabilityState::Disabled,
                leases: vec![],
            }
        ),
        Err(LifecycleError::Unverified)
    );
    assert_eq!(lifecycle.state(), CapabilityState::Failed);
    let retry = lifecycle.begin_recovery().unwrap();
    assert_eq!(
        lifecycle.finish(
            &token,
            ResourceInspection::Uncertain {
                message: "stale".into(),
            }
        ),
        Err(LifecycleError::StaleTicket)
    );
    assert_eq!(
        lifecycle.finish_recovery(
            &retry,
            ResourceInspection::Verified {
                state: CapabilityState::Active,
                leases: vec![],
            }
        ),
        Ok(CapabilityState::Active)
    );
}

#[test]
fn journal_failure_after_a_real_write_still_compensates() {
    let mut files = FakeFiles::default();
    files.seed("file.rs", "before");
    let plan = plan(&mut files, "journal-late", &["file.rs"]);
    let mut journals = FakeJournals {
        // 1: PREPARED, 2: MAY_HAVE_WRITTEN, 3: APPLIED_STEP.
        fail_at_save: Some(3),
        ..FakeJournals::default()
    };
    assert!(matches!(
        ApplyCoordinator::apply(&mut files, &mut journals, plan),
        Err(ApplyError::Aborted(_))
    ));
    assert_eq!(files.content("file.rs"), Some("before"));
    assert_eq!(
        journals.load("journal-late").unwrap().unwrap().status,
        ApplyStatus::Compensated
    );
}

#[test]
fn crashed_rollback_recovers_from_its_own_operation_provenance() {
    let mut files = FakeFiles::default();
    files.seed("file.rs", "original");
    let mut candidate = plan(&mut files, "rollback-interrupted", &["file.rs"]);
    let edit = candidate.edits.remove(0);
    let before = files.inspect("file.rs").unwrap();
    let op_id = "apply:rollback-interrupted:0";
    let after = files
        .replace(
            "file.rs",
            before.revision.as_deref(),
            Some(&edit.replacement),
            op_id,
        )
        .unwrap();
    let mut journals = FakeJournals::default();
    journals
        .store(&ApplyJournal {
            apply_id: "rollback-interrupted".into(),
            status: ApplyStatus::Applying,
            steps: vec![ApplyStep {
                edit,
                before,
                operation_id: op_id.into(),
                phase: StepPhase::Applied,
                after: Some(after),
            }],
        })
        .unwrap();
    files.fail_after_write = Some("file.rs".into());
    assert!(matches!(
        ApplyCoordinator::recover(&mut files, &mut journals, "rollback-interrupted"),
        Err(ApplyError::RecoveryRequired(_))
    ));
    assert_eq!(files.content("file.rs"), Some("original"));
    assert_eq!(
        ApplyCoordinator::recover(&mut files, &mut journals, "rollback-interrupted").unwrap(),
        ApplyOutcome::Compensated
    );
}

#[test]
fn equal_bytes_without_trusted_write_provenance_do_not_authorize_rollback() {
    let mut files = FakeFiles::default();
    files.seed("file.rs", "original");
    let mut candidate = plan(&mut files, "unproven-write", &["file.rs"]);
    let edit = candidate.edits.remove(0);
    let before = files.inspect("file.rs").unwrap();
    let mut journals = FakeJournals::default();
    journals
        .store(&ApplyJournal {
            apply_id: "unproven-write".into(),
            status: ApplyStatus::Applying,
            steps: vec![ApplyStep {
                edit: edit.clone(),
                before,
                operation_id: "apply:unproven-write:0".into(),
                phase: StepPhase::MayHaveWritten,
                after: None,
            }],
        })
        .unwrap();
    // Same text, but the host reports a different writer.
    files.seed("file.rs", &edit.replacement);
    assert!(matches!(
        ApplyCoordinator::recover(&mut files, &mut journals, "unproven-write"),
        Err(ApplyError::RecoveryRequired(_))
    ));
    assert_eq!(files.content("file.rs"), Some(edit.replacement.as_str()));
}
