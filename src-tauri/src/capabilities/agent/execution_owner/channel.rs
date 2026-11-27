//! The Channel's real entrypoints are an offline fixture and an explicitly
//! approved read-only observation. Both use the existing encrypted Run
//! owner and the same TaskRegistry, not a third runtime or storage engine.
use super::*;
use browseros_agent_foundation::{
    benchmark::is_fixture_id,
    experiment::{ExperimentConfig, ExperimentRouter, RuntimeAssignment},
};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ChannelInput {
    Fixture {
        fixture_id: String,
        assignment: RuntimeAssignment,
    },
    Read {
        command: String,
        args: Value,
        resource: String,
    },
}
pub(crate) enum ChannelStart {
    Fixture {
        fixture_id: String,
        config: ExperimentConfig,
    },
    Read {
        command: String,
        args: Value,
        resource: String,
    },
}
pub(crate) enum ChannelAdmission {
    Execute(ProductionExecution, ChannelInput),
    Recovered(Value),
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(super) fn checked_input(record: &NativeRunReceipt) -> Result<Option<ChannelInput>, String> {
    if record.task.goal != "agent_channel"
        || record.effect.is_some()
        || record
            .approval
            .as_ref()
            .is_some_and(|claim| claim.effect != ToolEffect::ReadOnly)
    {
        return Err("AGENT_RUN_SCOPE_MISMATCH".into());
    }
    let Some(payload) = &record.text else {
        return Ok(None);
    };
    let input: ChannelInput =
        serde_json::from_value(payload.input.clone()).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
    match &input {
        ChannelInput::Fixture {
            fixture_id,
            assignment,
        } => {
            if !record.task.workspace_id.is_nil()
                || !record.task.context.is_empty()
                || record.approval.is_some()
                || assignment.task_id != record.task.id
                || !(1..=32).contains(&assignment.max_turns)
                || !is_fixture_id(fixture_id)
            {
                return Err("AGENT_RUN_SCOPE_MISMATCH".into());
            }
        }
        ChannelInput::Read {
            command,
            args,
            resource,
        } => {
            // A recovered input must still be in the read-only contract.
            // Checking historical approval evidence does not issue a permit.
            let revision =
                browseros_agent_foundation::public_api::channel_read_revision(command, args)
                    .map_err(|_| "AGENT_RUN_SCOPE_MISMATCH")?;
            if let Some(claim) = &record.approval {
                if !claim.accepted
                    || claim.task_id != record.task.id
                    || claim.workspace_id != record.task.workspace_id
                    || claim.session_id != record.task.session_id
                    || claim.tool_name != *command
                    || claim.resource.resource_id != *resource
                    || (claim.resource.resource_revision != "local-main-ui-v1"
                        && claim.resource.resource_revision != revision)
                    || browseros_agent_foundation::approval::approval_argument_digest(
                        command,
                        args,
                        &ToolEffect::ReadOnly,
                    )
                    .ok()
                    .as_deref()
                        != Some(claim.arguments_sha256.as_str())
                {
                    return Err("AGENT_RUN_SCOPE_MISMATCH".into());
                }
            }
            let ctx = ContextRef::checked(
                ContextKind::Resource,
                resource.clone(),
                ContextGrant::ExplicitSelection,
            )
            .map_err(|_| "AGENT_RUN_SCOPE_MISMATCH")?;
            if record.task.workspace_id.is_nil() || record.task.context != vec![ctx] {
                return Err("AGENT_RUN_SCOPE_MISMATCH".into());
            }
        }
    }
    Ok(Some(input))
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn recovered(record: NativeRunReceipt) -> Result<ChannelAdmission, String> {
    if record.phase == "completed" && record.task.status == TaskStatus::Completed {
        return record
            .text
            .and_then(|payload| payload.result)
            .map(ChannelAdmission::Recovered)
            .ok_or_else(|| "AGENT_RUN_RESULT_UNAVAILABLE".into());
    }
    Err("AGENT_RUN_RECOVERY_REQUIRED".into())
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn attach(
    mut owner: DurableRun,
    mut execution: ProductionExecution,
    input: ChannelInput,
) -> Result<ChannelAdmission, String> {
    let value = serde_json::to_value(&input).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
    let encoded = serde_json::to_string(&value).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
    if encoded.len() > 16 * 1024 || crate::security_policy::contains_credential_leak(&encoded) {
        return Err("AGENT_RUN_INPUT_LIMIT".into());
    }
    owner.text = Some(TextRunPayload {
        input: value,
        result: None,
    });
    owner.persist("prepared", execution.record()?)?;
    execution.durable_run = Some(owner);
    Ok(ChannelAdmission::Execute(execution, input))
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn summary(record: &NativeRunReceipt, detail: bool) -> Result<Value, String> {
    let input = checked_input(record)?;
    let mut result = json!({
        "id":record.id,"task_id":record.task.id,"session_id":record.task.session_id,
        "phase":record.phase,"updated_at_ms":record.updated_at_ms,
        "previous_run":record.previous_run,"successor":record.successor.as_ref().map(|next| next.id),
        "can_resume":record.phase == "interrupted" && record.task.status == TaskStatus::Paused && input.is_some(),
        "reserved_input_available":record.successor.as_ref().is_some_and(|next| next.input.is_some()),
        "can_forget":DurableRun::can_compact_payload(record),
        "has_result":record.text.as_ref().and_then(|payload| payload.result.as_ref()).is_some(),
        "forgotten":record.payload_forgotten,
        "kind":input.as_ref().map(|input| match input { ChannelInput::Fixture { .. } => "fixture", ChannelInput::Read { .. } => "read" })
    });
    if detail {
        result["input"] = json!(input);
        result["result"] = json!(record
            .text
            .as_ref()
            .and_then(|payload| payload.result.as_ref()));
    }
    Ok(result)
}

impl ProductionExecution {
    pub(crate) fn begin_channel(
        storage: &Path,
        run: Uuid,
        start: ChannelStart,
    ) -> Result<ChannelAdmission, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let owner = DurableRun::new(storage, run)?;
            if let Some(record) = owner.read_record()? {
                let input = checked_input(&record)?;
                let matches = match (&start, input.as_ref()) {
                    (
                        ChannelStart::Fixture { fixture_id, .. },
                        Some(ChannelInput::Fixture {
                            fixture_id: old, ..
                        }),
                    ) => fixture_id == old,
                    (
                        ChannelStart::Read {
                            command,
                            args,
                            resource,
                        },
                        Some(ChannelInput::Read {
                            command: old,
                            args: saved,
                            resource: bound,
                        }),
                    ) => command == old && args == saved && resource == bound,
                    _ => false,
                };
                if !matches || record.previous_run.is_some() {
                    return Err("AGENT_RUN_INPUT_MISMATCH".into());
                }
                return recovered(owner.recover_record(record)?);
            }
            let (workspace, context) = match &start {
                ChannelStart::Fixture { fixture_id, config } => {
                    if !config.enabled || !is_fixture_id(fixture_id) {
                        return Err("AGENT_EXPERIMENT_FIXTURE_FAILED".into());
                    }
                    (Uuid::nil(), vec![])
                }
                ChannelStart::Read { resource, .. } => (
                    Uuid::new_v4(),
                    vec![ContextRef::checked(
                        ContextKind::Resource,
                        resource.clone(),
                        ContextGrant::ExplicitSelection,
                    )
                    .map_err(|_| "AGENT_EXPERIMENT_RESOURCE_INVALID")?],
                ),
            };
            let execution = Self::begin_scoped("agent_channel", workspace, run, context)?;
            let input = match start {
                ChannelStart::Fixture { fixture_id, config } => {
                    let router = ExperimentRouter::new(true);
                    router
                        .restore_config_for_host(config)
                        .map_err(|_| "AGENT_EXPERIMENT_ROUTE_INVALID")?;
                    let assignment = router
                        .assign(execution.id())
                        .map_err(|_| "AGENT_EXPERIMENT_ROUTE_FAILED")?;
                    ChannelInput::Fixture {
                        fixture_id,
                        assignment,
                    }
                }
                ChannelStart::Read {
                    command,
                    args,
                    resource,
                } => ChannelInput::Read {
                    command,
                    args,
                    resource,
                },
            };
            attach(owner, execution, input)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = (storage, run, start);
            Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into())
        }
    }

    pub(crate) fn resume_channel(
        storage: &Path,
        previous: Uuid,
        run: Uuid,
    ) -> Result<ChannelAdmission, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if previous == run || run.is_nil() {
                return Err("AGENT_RUN_ADVANCE_INVALID".into());
            }
            let predecessor = DurableRun::lookup(storage, previous)?;
            let record = predecessor.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            let input = checked_input(&record)?;
            let mut record = predecessor.recover_record(record)?;
            if record.successor.as_ref().is_some_and(|next| next.id != run) {
                return Err("AGENT_RUN_SUCCESSOR_EXISTS".into());
            }
            let mut child = DurableRun::new(storage, run)?;
            let existing = child.read_record()?;
            if let Some(next) = &record.successor {
                if !next.resume {
                    return Err("AGENT_RUN_SCOPE_MISMATCH".into());
                }
                if let Some(existing) = existing {
                    checked_input(&existing)?;
                    DurableRun::verify_successor(&record, &existing)?;
                    return recovered(child.recover_record(existing)?);
                }
            } else if existing.is_some() {
                return Err("AGENT_RUN_ALREADY_RECORDED".into());
            }
            // An archived predecessor may restore a durable successor, but
            // must never dispatch a missing successor or recover a permit.
            if record.phase != "interrupted" || record.task.status != TaskStatus::Paused {
                return Err("AGENT_RUN_NOT_RESUMABLE".into());
            }
            let input = input.ok_or("AGENT_RUN_RESULT_UNAVAILABLE")?;
            let saved = serde_json::to_value(&input).map_err(|_| "AGENT_RUN_RECEIPT_INVALID")?;
            if let Some(next) = &record.successor {
                if next.input.as_ref() != Some(&saved) {
                    return Err("AGENT_RUN_SCOPE_MISMATCH".into());
                }
            } else {
                record.successor = Some(TextSuccessor {
                    id: run,
                    input: Some(saved),
                    resume: true,
                });
                record.updated_at_ms = unix_ms();
                predecessor.write_record(&record)?;
            }
            let tasks = task_owner();
            let id = record.task.id;
            tasks
                .restore_paused_record(record.task)
                .map_err(|_| "AGENT_TASK_RESTORE_FAILED")?;
            if tasks.start(id).is_err() {
                let _ = tasks.detach_paused_record(id);
                return Err("AGENT_TASK_START_FAILED".into());
            }
            let execution = Self {
                tasks,
                id,
                finished: false,
                durable_run: None,
            };
            child.previous_run = Some(previous);
            child._predecessor = Some(Box::new(predecessor));
            attach(child, execution, input)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = (storage, previous, run);
            Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into())
        }
    }

    pub(crate) fn finish_channel(mut self, result: &Result<Value, String>) -> Result<(), String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if let Ok(value) = result {
                self.checkpoint_local_return(value, 64 * 1024)?;
            }
        }
        self.finish(result.is_ok())
    }

    pub(crate) fn channel_catalog(storage: &Path, after: Option<Uuid>) -> Result<Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let (ids, next) =
                super::super::coding_checkpoint::RunCheckpoint::catalog_page(storage, after)?;
            let mut records = Vec::new();
            let mut unavailable = 0;
            for id in ids {
                let result = (|| -> Result<Option<Value>, String> {
                    let owner = DurableRun::lookup(storage, id)?;
                    let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
                    if record.task.goal != "agent_channel" {
                        return Ok(None);
                    }
                    checked_input(&record)?;
                    Ok(Some(summary(&owner.recover_record(record)?, false)?))
                })();
                match result {
                    Ok(Some(record)) => records.push(record),
                    Ok(None) => {}
                    Err(_) => unavailable += 1,
                }
            }
            Ok(json!({"runs":records,"next_cursor":next,"unavailable":unavailable}))
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = (storage, after);
            Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into())
        }
    }
    pub(crate) fn read_channel(storage: &Path, id: Uuid) -> Result<Value, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            checked_input(&record)?;
            let mut value = summary(&owner.recover_record(record)?, true)?;
            value["archived"] = json!(owner.checkpoint.archive_confirmed()?);
            Ok(value)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = (storage, id);
            Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into())
        }
    }
    pub(crate) fn forget_channel(storage: &Path, id: Uuid) -> Result<bool, String> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let owner = DurableRun::lookup(storage, id)?;
            let record = owner.read_record()?.ok_or("AGENT_RUN_NOT_FOUND")?;
            checked_input(&record)?;
            let record = owner.recover_record(record)?;
            owner.compact_payload(storage, record)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = (storage, id);
            Err("AGENT_PRIVATE_STORAGE_UNAVAILABLE".into())
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests {
    use super::*;
    #[test]
    fn channel_return_checkpoint_recovers_before_terminal_publication() {
        let storage =
            std::env::temp_dir().join(format!("browseros-channel-return-{}", Uuid::new_v4()));
        let run = Uuid::new_v4();
        let config = ExperimentConfig {
            enabled: true,
            ..ExperimentConfig::default()
        };
        let ChannelAdmission::Execute(mut execution, _) = ProductionExecution::begin_channel(
            &storage,
            run,
            ChannelStart::Fixture {
                fixture_id: "tab_summary".into(),
                config: config.clone(),
            },
        )
        .unwrap() else {
            panic!("new fixture")
        };
        let result = json!({"fixture_id":"tab_summary","output":"fixture"});
        execution
            .checkpoint_local_return(&result, 64 * 1024)
            .unwrap();
        drop(execution);
        let ChannelAdmission::Recovered(saved) = ProductionExecution::begin_channel(
            &storage,
            run,
            ChannelStart::Fixture {
                fixture_id: "tab_summary".into(),
                config,
            },
        )
        .unwrap() else {
            panic!("must not replay")
        };
        assert_eq!(saved, result);
        assert_eq!(
            ProductionExecution::read_channel(&storage, run).unwrap()["phase"],
            "completed"
        );
        assert!(ProductionExecution::resume_channel(&storage, run, Uuid::new_v4()).is_err());
        std::fs::remove_dir_all(storage).unwrap();
    }

    #[test]
    fn resume_fixture_pins_route_and_task_without_replaying_a_committed_result() {
        let storage =
            std::env::temp_dir().join(format!("browseros-channel-resume-{}", Uuid::new_v4()));
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let config = ExperimentConfig {
            enabled: true,
            ..ExperimentConfig::default()
        };
        let ChannelAdmission::Execute(execution, input) = ProductionExecution::begin_channel(
            &storage,
            first,
            ChannelStart::Fixture {
                fixture_id: "tab_summary".into(),
                config,
            },
        )
        .unwrap() else {
            panic!("new fixture")
        };
        let task = execution.id();
        drop(execution);
        let ChannelAdmission::Execute(resumed, restored) =
            ProductionExecution::resume_channel(&storage, first, second).unwrap()
        else {
            panic!("resume fixture")
        };
        assert_eq!(resumed.id(), task);
        assert_eq!(input, restored);
        let result = json!({"fixture_id":"tab_summary","output":"fixture"});
        resumed.finish_channel(&Ok(result.clone())).unwrap();
        let ChannelAdmission::Recovered(saved) =
            ProductionExecution::resume_channel(&storage, first, second).unwrap()
        else {
            panic!("recover result without dispatch")
        };
        assert_eq!(saved, result);
        assert!(ProductionExecution::resume_channel(&storage, first, Uuid::new_v4()).is_err());
        assert!(ProductionExecution::forget_channel(&storage, first).unwrap());
        let archived = ProductionExecution::read_channel(&storage, first).unwrap();
        assert_eq!(archived["phase"], "superseded");
        assert_eq!(archived["task_id"], json!(task));
        assert_eq!(archived["successor"], json!(second));
        assert_eq!(archived["archived"], true);
        assert_eq!(archived["can_resume"], false);
        let ChannelAdmission::Recovered(saved) =
            ProductionExecution::resume_channel(&storage, first, second).unwrap()
        else {
            panic!("compacted predecessor may only restore committed successor")
        };
        assert_eq!(saved, result);
        ProductionExecution::forget_channel(&storage, second).unwrap();
        assert!(ProductionExecution::resume_channel(&storage, first, second).is_err());
        std::fs::remove_dir_all(storage).unwrap();
    }
}
