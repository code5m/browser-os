//! Per-task runtime selection and dynamic benchmark toggles.
//! Both strategies share exactly ONE task/approval/storage owner. No mid-task migration.
use crate::{FoundationError, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::RwLock, time::Duration};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    NativeTurn,
    RigAgentRun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingMode {
    Native,
    Rig,
    Canary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentConfig {
    pub revision: u64,
    pub enabled: bool,
    pub mode: RoutingMode,
    pub canary_percent: u8,
    pub emergency_fallback_to_native: bool,
    pub benchmark_enabled: bool,
    pub max_turns: u8,
}
impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            revision: 0,
            enabled: false,
            mode: RoutingMode::Native,
            canary_percent: 0,
            emergency_fallback_to_native: true,
            benchmark_enabled: false,
            max_turns: 1,
        }
    }
}
impl ExperimentConfig {
    pub fn validate(&self) -> Result<()> {
        if self.canary_percent > 100 || !(1..=32).contains(&self.max_turns) {
            return Err(FoundationError::InvalidInput(
                "runtime experiment configuration",
            ));
        }
        if !self.enabled && self.mode != RoutingMode::Native {
            return Err(FoundationError::PermissionDenied);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeAssignment {
    pub task_id: Uuid,
    pub kind: EngineKind,
    pub configuration_revision: u64,
    pub benchmark_enabled: bool,
    pub max_turns: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BenchmarkSample {
    pub task_id: Uuid,
    pub engine: EngineKind,
    pub total_duration_ms: u64,
    pub model_duration_ms: u64,
    /// Microseconds avoid zero-duration readings in short offline fixtures.
    pub total_duration_us: u64,
    pub model_duration_us: u64,
    pub first_token_ms: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub tool_approval_wait_ms: Option<u64>,
    pub tool_calls: u32,
    pub completed: bool,
    pub error_code: Option<String>,
}
impl BenchmarkSample {
    pub fn from_elapsed(
        task_id: Uuid,
        engine: EngineKind,
        total: Duration,
        model: Duration,
        completed: bool,
    ) -> Self {
        Self {
            task_id,
            engine,
            total_duration_ms: total.as_millis().min(u128::from(u64::MAX)) as u64,
            model_duration_ms: model.as_millis().min(u128::from(u64::MAX)) as u64,
            total_duration_us: total.as_micros().min(u128::from(u64::MAX)) as u64,
            model_duration_us: model.as_micros().min(u128::from(u64::MAX)) as u64,
            first_token_ms: None,
            input_tokens: None,
            output_tokens: None,
            tool_approval_wait_ms: None,
            tool_calls: 0,
            completed,
            error_code: None,
        }
    }
}

/// Thread-safe experiment control plane. Config updates only affect NEW tasks.
/// Assignment is sticky for a task even after cancellation/restart.
pub struct ExperimentRouter {
    config: RwLock<ExperimentConfig>,
    assignments: RwLock<HashMap<Uuid, RuntimeAssignment>>,
    rig_available: bool,
}
impl ExperimentRouter {
    pub fn new(rig_available: bool) -> Self {
        Self {
            config: RwLock::new(ExperimentConfig::default()),
            assignments: RwLock::new(HashMap::new()),
            rig_available,
        }
    }
    /// The trusted desktop host reloads a previously atomically persisted
    /// configuration before accepting new tasks. No existing assignment may
    /// be silently reconfigured.
    pub fn restore_config_for_host(&self, saved: ExperimentConfig) -> Result<()> {
        saved.validate()?;
        if saved.enabled
            && (saved.mode == RoutingMode::Rig
                || (saved.mode == RoutingMode::Canary && saved.canary_percent > 0))
            && !self.rig_available
        {
            return Err(FoundationError::PermissionDenied);
        }
        let mut current = self.config.write().unwrap_or_else(|e| e.into_inner());
        if current.revision != 0
            || !self
                .assignments
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .is_empty()
        {
            return Err(FoundationError::InvalidTransition);
        }
        *current = saved;
        Ok(())
    }

    pub fn config(&self) -> ExperimentConfig {
        self.config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn update(&self, next: ExperimentConfig, expected_revision: u64) -> Result<()> {
        next.validate()?;
        let mut config = self.config.write().unwrap_or_else(|e| e.into_inner());
        if config.revision != expected_revision || next.revision != expected_revision + 1 {
            return Err(FoundationError::InvalidTransition);
        }
        if next.enabled
            && (next.mode == RoutingMode::Rig
                || (next.mode == RoutingMode::Canary && next.canary_percent > 0))
            && !self.rig_available
        {
            return Err(FoundationError::PermissionDenied);
        }
        *config = next;
        Ok(())
    }
    pub fn assign(&self, task_id: Uuid) -> Result<RuntimeAssignment> {
        if task_id.is_nil() {
            return Err(FoundationError::InvalidInput("task id"));
        }
        // Keep lock ordering consistent with restore_config_for_host:
        // always acquire config BEFORE assignments, avoiding an ABBA deadlock.
        let cfg = self.config();
        let mut assigned = self.assignments.write().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = assigned.get(&task_id) {
            return Ok(existing.clone());
        }
        let choice = if !cfg.enabled {
            EngineKind::NativeTurn
        } else {
            match cfg.mode {
                RoutingMode::Native => EngineKind::NativeTurn,
                RoutingMode::Rig => EngineKind::RigAgentRun,
                RoutingMode::Canary => {
                    // Deterministic allocation: no runtime randomness, stable across reboots
                    // as long as the assignment is persisted by the host.
                    let bucket = task_id.as_u128() % 100;
                    if bucket < u128::from(cfg.canary_percent) {
                        EngineKind::RigAgentRun
                    } else {
                        EngineKind::NativeTurn
                    }
                }
            }
        };
        if choice == EngineKind::RigAgentRun && !self.rig_available {
            return Err(FoundationError::PermissionDenied);
        }
        let assignment = RuntimeAssignment {
            task_id,
            kind: choice,
            configuration_revision: cfg.revision,
            benchmark_enabled: cfg.benchmark_enabled,
            max_turns: cfg.max_turns,
        };
        assigned.insert(task_id, assignment.clone());
        Ok(assignment)
    }
    pub fn restore(&self, assignment: RuntimeAssignment) -> Result<()> {
        if assignment.task_id.is_nil()
            || assignment.max_turns == 0
            || (assignment.kind == EngineKind::RigAgentRun && !self.rig_available)
        {
            return Err(FoundationError::PermissionDenied);
        }
        let mut assigned = self.assignments.write().unwrap_or_else(|e| e.into_inner());
        if assigned
            .get(&assignment.task_id)
            .is_some_and(|old| old != &assignment)
        {
            return Err(FoundationError::InvalidTransition);
        }
        assigned.insert(assignment.task_id, assignment);
        Ok(())
    }
    pub fn release(&self, task_id: Uuid) -> Option<RuntimeAssignment> {
        self.assignments
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&task_id)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_off_rig_requires_capability() {
        let router = ExperimentRouter::new(false);
        assert_eq!(
            router.assign(Uuid::new_v4()).unwrap().kind,
            EngineKind::NativeTurn
        );
        let blocked = ExperimentConfig {
            revision: 1,
            enabled: true,
            mode: RoutingMode::Rig,
            ..ExperimentConfig::default()
        };
        assert!(router.update(blocked, 0).is_err());
    }
    #[test]
    fn running_tasks_never_migrate_and_config_is_cas() {
        let router = ExperimentRouter::new(true);
        let id = Uuid::new_v4();
        let original = router.assign(id).unwrap();
        let rig = ExperimentConfig {
            revision: 1,
            enabled: true,
            mode: RoutingMode::Rig,
            benchmark_enabled: true,
            ..ExperimentConfig::default()
        };
        router.update(rig.clone(), 0).unwrap();
        assert!(router.update(rig, 0).is_err());
        assert_eq!(router.assign(id).unwrap(), original);
        assert_eq!(
            router.assign(Uuid::new_v4()).unwrap().kind,
            EngineKind::RigAgentRun
        );
        let saved = router.assign(id).unwrap();
        let other = ExperimentRouter::new(true);
        other.restore(saved.clone()).unwrap();
        assert_eq!(other.assign(id).unwrap(), saved);
    }
    #[test]
    fn canary_and_emergency_off() {
        let router = ExperimentRouter::new(true);
        router
            .update(
                ExperimentConfig {
                    revision: 1,
                    enabled: true,
                    mode: RoutingMode::Canary,
                    canary_percent: 100,
                    ..ExperimentConfig::default()
                },
                0,
            )
            .unwrap();
        assert_eq!(
            router.assign(Uuid::new_v4()).unwrap().kind,
            EngineKind::RigAgentRun
        );
        router
            .update(
                ExperimentConfig {
                    revision: 2,
                    ..ExperimentConfig::default()
                },
                1,
            )
            .unwrap();
        assert_eq!(
            router.assign(Uuid::new_v4()).unwrap().kind,
            EngineKind::NativeTurn
        );
    }
}
