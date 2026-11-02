//! Deterministic no-network/side-effect Agent Runtime A/B suite.
//! Samples measure orchestration overhead only; never model quality or GUI.
use crate::{
    engine::{AgentEngine, TurnCompletion, TurnLimits},
    event::EventJournal,
    experiment::{BenchmarkSample, EngineKind},
    model::{ModelEvent, ModelFuture, ModelProvider, ModelRequest},
    task::TaskRegistry,
    tool::ToolRegistry,
    FoundationError, Result,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

#[cfg(feature = "experimental-rig-run")]
use crate::rig_driver::{RigCall, RigDriverStep, RigModelBackend, RigMultiTurnDriver};
#[cfg(feature = "experimental-rig-run")]
use rig_core::{
    completion::{CompletionRequest, CompletionResponse, Usage},
    message::AssistantContent,
};

const SUITE: [(&str, &str); 3] = [
    ("tab_summary", "Summarize the tabs in the mock context."),
    (
        "git_review",
        "Summarize the changed files in a mock repository.",
    ),
    (
        "database_report",
        "Explain an example mock database result.",
    ),
];
const ANSWER: &str = "Synthetic fixture completed; no native resources accessed.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub kind: String,
    pub repeat_count: u8,
    pub suite_id: String,
    pub samples: Vec<BenchmarkSample>,
    pub disclaimer: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutedFixtureResult {
    pub fixture_id: String,
    pub sample: BenchmarkSample,
    pub output: String,
}

struct ScriptedNative;
impl ModelProvider for ScriptedNative {
    fn stream<'a>(&'a self, request: ModelRequest) -> ModelFuture<'a> {
        Box::pin(async move {
            request.validate()?;
            let (tx, rx) = mpsc::channel(8);
            for event in [
                ModelEvent::TextDelta {
                    text: ANSWER.into(),
                },
                ModelEvent::Usage {
                    input_tokens: 12,
                    output_tokens: 14,
                },
                ModelEvent::Finished,
            ] {
                tx.send(event)
                    .await
                    .map_err(|_| FoundationError::Cancelled)?;
            }
            Ok(rx)
        })
    }
}
#[cfg(feature = "experimental-rig-run")]
struct ScriptedRig;
#[cfg(feature = "experimental-rig-run")]
impl RigModelBackend for ScriptedRig {
    fn complete<'a>(&'a self, _request: CompletionRequest) -> RigCall<'a> {
        Box::pin(async {
            let usage = Usage {
                input_tokens: Some(12),
                output_tokens: Some(14),
                ..Usage::default()
            };
            Ok(CompletionResponse::new(
                vec![AssistantContent::text(ANSWER)],
                usage,
                "benchmark-mock",
                serde_json::json!({}),
            ))
        })
    }
}

/// New task in either engine. No real model credentials, external effect, or
/// browser state are accessed. Always use the identical fixed fixture catalog.
pub async fn run_routed_fixture(
    kind: EngineKind,
    task_id: Uuid,
    fixture_id: &str,
    max_turns: u8,
) -> Result<RoutedFixtureResult> {
    if !(1..=32).contains(&max_turns) {
        return Err(FoundationError::InvalidInput("benchmark max turns"));
    }
    if task_id.is_nil() {
        return Err(FoundationError::InvalidInput("benchmark task id"));
    }
    let prompt = SUITE
        .iter()
        .find(|(id, _)| *id == fixture_id)
        .map(|(_, prompt)| *prompt)
        .ok_or(FoundationError::InvalidInput("benchmark fixture id"))?;
    let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
    let task = tasks.create(Uuid::new_v4(), prompt, vec![])?;
    tasks.start(task.id)?;
    run_routed_fixture_owned_with_id(kind, task.id, task_id, fixture_id, max_turns, &tasks).await
}

/// Run a real Foundation task ID in the supplied production TaskRegistry.
/// Unlike the standalone A/B helper, this never creates a second Task Owner.
pub async fn run_routed_fixture_owned(
    kind: EngineKind,
    task_id: Uuid,
    fixture_id: &str,
    max_turns: u8,
    tasks: &TaskRegistry,
) -> Result<RoutedFixtureResult> {
    run_routed_fixture_owned_with_id(kind, task_id, task_id, fixture_id, max_turns, tasks).await
}

async fn run_routed_fixture_owned_with_id(
    kind: EngineKind,
    actual_task_id: Uuid,
    sample_id: Uuid,
    fixture_id: &str,
    max_turns: u8,
    tasks: &TaskRegistry,
) -> Result<RoutedFixtureResult> {
    if !(1..=32).contains(&max_turns) || actual_task_id.is_nil() || sample_id.is_nil() {
        return Err(FoundationError::InvalidInput("benchmark run parameters"));
    }
    let prompt = SUITE
        .iter()
        .find(|(id, _)| *id == fixture_id)
        .map(|(_, text)| *text)
        .ok_or(FoundationError::InvalidInput("benchmark fixture id"))?;
    if tasks.get(actual_task_id)?.status != crate::task::TaskStatus::Running {
        return Err(FoundationError::InvalidTransition);
    }
    let registry = ToolRegistry::default();
    let start = Instant::now();
    let answer = match kind {
        EngineKind::NativeTurn => {
            let provider = ScriptedNative;
            let engine = AgentEngine {
                provider: &provider,
                tools: &registry,
                tasks,
                limits: TurnLimits::default(),
            };
            let result = engine
                .drive_turn(actual_task_id, "benchmark-mock", prompt)
                .await?;
            if result.completion != TurnCompletion::Completed {
                return Err(FoundationError::Provider(
                    "BENCHMARK_NATIVE_INCOMPLETE".into(),
                ));
            }
            result.text
        }
        EngineKind::RigAgentRun => {
            #[cfg(feature = "experimental-rig-run")]
            {
                let mut driver = RigMultiTurnDriver::new(prompt, max_turns)?;
                let result = driver
                    .drive(
                        actual_task_id,
                        "benchmark-mock",
                        &ScriptedRig,
                        tasks,
                        &registry,
                        &[],
                    )
                    .await?;
                match result {
                    RigDriverStep::Completed { text, .. } => text,
                    RigDriverStep::NeedsApproval { .. } => {
                        return Err(FoundationError::PermissionDenied)
                    }
                }
            }
            #[cfg(not(feature = "experimental-rig-run"))]
            {
                return Err(FoundationError::PermissionDenied);
            }
        }
    };
    if answer != ANSWER {
        return Err(FoundationError::Provider(
            "BENCHMARK_OUTPUT_MISMATCH".into(),
        ));
    }
    let elapsed = start.elapsed();
    let mut sample = BenchmarkSample::from_elapsed(sample_id, kind, elapsed, elapsed, true);
    sample.input_tokens = Some(12);
    sample.output_tokens = Some(14);
    Ok(RoutedFixtureResult {
        fixture_id: fixture_id.into(),
        sample,
        output: answer,
    })
}

pub async fn run_benchmark_suite(repeats: u8) -> Result<BenchmarkReport> {
    if !(1..=10).contains(&repeats) {
        return Err(FoundationError::InvalidInput("benchmark repeats"));
    }
    let mut samples = Vec::with_capacity(usize::from(repeats) * SUITE.len() * 2);
    for _ in 0..repeats {
        for (fixture_id, _) in SUITE {
            samples.push(
                run_routed_fixture(EngineKind::NativeTurn, Uuid::new_v4(), fixture_id, 3)
                    .await?
                    .sample,
            );
            #[cfg(feature = "experimental-rig-run")]
            samples.push(
                run_routed_fixture(EngineKind::RigAgentRun, Uuid::new_v4(), fixture_id, 3)
                    .await?
                    .sample,
            );
        }
    }
    Ok(BenchmarkReport{
        kind:"synthetic_orchestration_only".into(),
        repeat_count:repeats,
        suite_id:"browseros-agent-v1-fixed-fixtures".into(),
        samples,
        disclaimer:"Official Rig AgentRun and BrowserOS NativeTurn are measured with fixed mock replies, no network or native side effects. This does not evaluate quality, GUI or real provider latency.".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn owned_fixture_uses_the_actual_caller_task_without_creating_another() {
        let tasks = TaskRegistry::new(Arc::new(EventJournal::new(32)));
        let t = tasks
            .create_scoped(Uuid::new_v4(), Uuid::new_v4(), "agent_channel", vec![])
            .unwrap();
        tasks.start(t.id).unwrap();
        let result =
            run_routed_fixture_owned(EngineKind::NativeTurn, t.id, "tab_summary", 3, &tasks)
                .await
                .unwrap();
        assert_eq!(result.sample.task_id, t.id);
        assert_eq!(
            tasks.get(t.id).unwrap().status,
            crate::task::TaskStatus::Completed
        );
        assert!(
            run_routed_fixture_owned(EngineKind::NativeTurn, t.id, "tab_summary", 3, &tasks)
                .await
                .is_err()
        );
        tasks.retire_terminal(t.id).unwrap();
        assert_eq!(tasks.export_snapshot().unwrap(), "[]");
    }
    #[tokio::test]
    async fn fixed_suite_ab_and_individual_routes() {
        let report = run_benchmark_suite(2).await.unwrap();
        #[cfg(feature = "experimental-rig-run")]
        assert_eq!(report.samples.len(), 12);
        #[cfg(not(feature = "experimental-rig-run"))]
        assert_eq!(report.samples.len(), 6);
        assert!(report
            .samples
            .iter()
            .all(|sample| sample.completed && sample.input_tokens == Some(12)));
        assert!(run_benchmark_suite(0).await.is_err());
        assert!(
            run_routed_fixture(EngineKind::NativeTurn, Uuid::new_v4(), "../invalid", 3)
                .await
                .is_err()
        );
        assert!(
            run_routed_fixture(EngineKind::NativeTurn, Uuid::new_v4(), "git_review", 3)
                .await
                .is_ok()
        );
    }
}
