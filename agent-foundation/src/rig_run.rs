//! Optional Rig AgentRun state-machine pilot, NOT a second Task/permission owner.
//! The host retains TaskRegistry, signed approval and durable effect ledger.
//! Driver MUST suspend on CallTools, create proposals and await Host approval.
use crate::{FoundationError, Result};
use rig_agent::run::{AgentRun, AgentRunStep};
use rig_core::completion::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RigStepKind {
    Model,
    AwaitHostToolApproval,
    Done,
}

pub struct RigRunPilot {
    run: AgentRun,
}
impl RigRunPilot {
    pub fn new(prompt: &str, max_turns: u8) -> Result<Self> {
        if prompt.is_empty() || prompt.len() > 32_768 || !(1..=32).contains(&max_turns) {
            return Err(FoundationError::InvalidInput("rig agent run configuration"));
        }
        Ok(Self {
            run: AgentRun::new(Message::user(prompt)).max_turns(usize::from(max_turns)),
        })
    }
    pub fn next_step(&mut self) -> Result<RigStepKind> {
        let step = self
            .run
            .next_step()
            .map_err(|_| FoundationError::Provider("RIG_RUN_TRANSITION".into()))?;
        match step {
            AgentRunStep::CallModel { .. } => Ok(RigStepKind::Model),
            AgentRunStep::CallTools { .. } => Ok(RigStepKind::AwaitHostToolApproval),
            AgentRunStep::Done(_) => Ok(RigStepKind::Done),
        }
    }
    pub(crate) fn state(&mut self) -> &mut AgentRun {
        &mut self.run
    }

    pub fn snapshot(&self) -> Result<String> {
        serde_json::to_string(&self.run).map_err(|e| FoundationError::Serialization(e.to_string()))
    }
    pub fn restore(text: &str) -> Result<Self> {
        if text.len() > 1_048_576 {
            return Err(FoundationError::InvalidInput("rig run snapshot"));
        }
        let run = serde_json::from_str(text)
            .map_err(|_| FoundationError::InvalidInput("rig run snapshot"))?;
        Ok(Self { run })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_rig_state_machine_roundtrip_and_bounded_turns() {
        let mut a = RigRunPilot::new("review", 3).unwrap();
        assert_eq!(a.next_step().unwrap(), RigStepKind::Model);
        let saved = a.snapshot().unwrap();
        let b = RigRunPilot::restore(&saved).unwrap();
        assert_eq!(b.snapshot().unwrap(), saved);
        assert!(RigRunPilot::new("review", 0).is_err());
    }
}
