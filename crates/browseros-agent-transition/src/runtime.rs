//! One host-owned execution port for all agent surfaces, with a safe legacy router.
//! This module NEVER invokes an existing command or enables legacy Agent backends.
use std::collections::BTreeMap;

use browseros_agent_channel::context::ContextRef;
use browseros_agent_channel::events::EventKind;
use browseros_agent_channel::model::BoxFuture;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workload {
    Coding,
    Browser,
    Database,
    GeneralAgent,
    Skill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Read,
    Infer,
    Write,
    Execute,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRequest {
    pub task_id: String,
    pub session_id: String,
    pub workload: Workload,
    pub effect: Effect,
    pub instruction: String,
    /// Metadata only: no implicit page, file or account access.
    pub contexts: Vec<ContextRef>,
    /// Tool names are resolved by the host's ToolCatalog; not trusted grants.
    pub requested_tool: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidRequest,
    Unsupported,
    Unauthorized,
    Provider(String),
}

impl RuntimeRequest {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.task_id.trim().is_empty()
            || self.session_id.trim().is_empty()
            || self.instruction.trim().is_empty()
            || self
                .contexts
                .iter()
                .any(|context| context.resource_id.trim().is_empty())
            || self
                .requested_tool
                .as_ref()
                .is_some_and(|tool| tool.trim().is_empty())
        {
            return Err(RuntimeError::InvalidRequest);
        }
        Ok(())
    }
}

/// A single, typed interface. Later adapters can reuse the existing Coding Agent
/// without creating another model loop, tool executor, credential store or IPC.
pub trait AgentRuntimePort: Send + Sync {
    fn submit<'a>(
        &'a self,
        request: RuntimeRequest,
        emit: &'a mut (dyn FnMut(EventKind) + Send + 'a),
    ) -> BoxFuture<'a, Result<(), RuntimeError>>;

    fn cancel<'a>(&'a self, task_id: &'a str) -> BoxFuture<'a, Result<(), RuntimeError>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegacyEntry {
    CodingService,
    AgentChat,
    AgentRun,
    AgentInstall,
}

impl LegacyEntry {
    /// Exact existing command names; this does not register missing commands.
    pub fn parse(command: &str) -> Option<Self> {
        match command {
            "coding_service" => Some(Self::CodingService),
            "agent_chat" => Some(Self::AgentChat),
            "agent_run" => Some(Self::AgentRun),
            "agent_install" => Some(Self::AgentInstall),
            _ => None,
        }
    }

    pub fn workload(self) -> Workload {
        match self {
            Self::CodingService => Workload::Coding,
            Self::AgentInstall | Self::AgentRun | Self::AgentChat => Workload::GeneralAgent,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoverMode {
    /// Preserve the current behavior, including disabled legacy backends.
    ExistingOnly,
    /// Observe/cross-check reads only; no duplicate model calls or mutations.
    ShadowReads,
    /// Host must explicitly opt in for each entry point and authorize effects.
    OptIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostDecision {
    No,
    ExplicitOptIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    ExistingUnchanged,
    ShadowReadOnly,
    /// Run only the candidate; never silently fall back after write/execute.
    CandidateOnly,
}

#[derive(Default)]
pub struct CutoverRouter {
    policies: BTreeMap<LegacyEntry, CutoverMode>,
}

impl CutoverRouter {
    pub fn set_policy(&mut self, entry: LegacyEntry, policy: CutoverMode) {
        self.policies.insert(entry, policy);
    }

    /// This is routing intent, NOT a permission decision. Production host must
    /// check source identity, user consent, tool scope and capability ownership.
    pub fn route(&self, entry: LegacyEntry, effect: Effect, decision: HostDecision) -> Route {
        match self
            .policies
            .get(&entry)
            .copied()
            .unwrap_or(CutoverMode::ExistingOnly)
        {
            CutoverMode::ExistingOnly => Route::ExistingUnchanged,
            CutoverMode::ShadowReads if effect == Effect::Read => Route::ShadowReadOnly,
            CutoverMode::ShadowReads => Route::ExistingUnchanged,
            CutoverMode::OptIn if decision == HostDecision::ExplicitOptIn => Route::CandidateOnly,
            CutoverMode::OptIn => Route::ExistingUnchanged,
        }
    }
}
