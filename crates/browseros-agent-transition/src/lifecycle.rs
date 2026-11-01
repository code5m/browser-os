//! Fail-closed lifecycle state machine with explicit lease verification.
//! Never represent a partially cleaned-up capability as ACTIVE via a boolean rollback.
//! No real process, event listener, Tauri state or contribution is changed here.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Disabled,
    Activating,
    Active,
    Suspending,
    Suspended,
    Deactivating,
    Failed,
    Recovering,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleAction {
    Enable,
    Suspend,
    Resume,
    Disable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLease {
    pub owner: String,
    pub resource_id: String,
    /// Generation used to reject a stale resource release/attachment.
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidTransition,
    StaleTicket,
    RevisionOverflow,
    Unverified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionTicket {
    pub generation: u64,
    expected: CapabilityState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceInspection {
    /// Only the trusted host may report this after checking real resources.
    Verified {
        state: CapabilityState,
        leases: Vec<ResourceLease>,
    },
    Uncertain { message: String },
}

#[derive(Debug)]
pub struct CapabilityMachine {
    state: CapabilityState,
    generation: u64,
    leases: Vec<ResourceLease>,
    last_failure: Option<String>,
}

impl Default for CapabilityMachine {
    fn default() -> Self {
        Self {
            state: CapabilityState::Disabled,
            generation: 0,
            leases: vec![],
            last_failure: None,
        }
    }
}

impl CapabilityMachine {
    pub fn state(&self) -> CapabilityState { self.state }
    pub fn generation(&self) -> u64 { self.generation }
    pub fn leases(&self) -> &[ResourceLease] { &self.leases }
    pub fn failure(&self) -> Option<&str> { self.last_failure.as_deref() }

    pub fn begin(&mut self, action: LifecycleAction) -> Result<TransitionTicket, LifecycleError> {
        use CapabilityState as S;
        let pending = match (self.state, action) {
            (S::Disabled, LifecycleAction::Enable) => S::Activating,
            (S::Active, LifecycleAction::Suspend) => S::Suspending,
            (S::Suspended, LifecycleAction::Resume) => S::Activating,
            (S::Suspended, LifecycleAction::Disable) => S::Deactivating,
            _ => return Err(LifecycleError::InvalidTransition),
        };
        self.generation = self.generation.checked_add(1).ok_or(LifecycleError::RevisionOverflow)?;
        self.state = pending;
        Ok(TransitionTicket { generation: self.generation, expected: pending })
    }

    pub fn finish(
        &mut self,
        ticket: &TransitionTicket,
        inspected: ResourceInspection,
    ) -> Result<CapabilityState, LifecycleError> {
        if ticket.generation != self.generation || ticket.expected != self.state {
            return Err(LifecycleError::StaleTicket);
        }
        let expected = match self.state {
            CapabilityState::Activating => CapabilityState::Active,
            CapabilityState::Suspending => CapabilityState::Suspended,
            CapabilityState::Deactivating => CapabilityState::Disabled,
            _ => return Err(LifecycleError::InvalidTransition),
        };
        self.verify(inspected, Some(expected))
    }

    pub fn begin_recovery(&mut self) -> Result<TransitionTicket, LifecycleError> {
        if self.state != CapabilityState::Failed {
            return Err(LifecycleError::InvalidTransition);
        }
        self.generation = self.generation.checked_add(1).ok_or(LifecycleError::RevisionOverflow)?;
        self.state = CapabilityState::Recovering;
        Ok(TransitionTicket { generation: self.generation, expected: CapabilityState::Recovering })
    }

    pub fn finish_recovery(
        &mut self,
        ticket: &TransitionTicket,
        inspected: ResourceInspection,
    ) -> Result<CapabilityState, LifecycleError> {
        if ticket.generation != self.generation || self.state != CapabilityState::Recovering
            || ticket.expected != CapabilityState::Recovering {
            return Err(LifecycleError::StaleTicket);
        }
        self.verify(inspected, None)
    }

    fn verify(
        &mut self,
        inspected: ResourceInspection,
        expected: Option<CapabilityState>,
    ) -> Result<CapabilityState, LifecycleError> {
        match inspected {
            ResourceInspection::Verified { state, leases }
                if matches!(state, CapabilityState::Active | CapabilityState::Suspended | CapabilityState::Disabled)
                    && expected.map_or(true, |value| state == value)
                    && leases.iter().all(|lease| !lease.owner.trim().is_empty()
                        && !lease.resource_id.trim().is_empty()
                        && lease.generation <= self.generation)
                    && !(state == CapabilityState::Disabled && !leases.is_empty()) =>
            {
                self.state = state;
                self.leases = leases;
                self.last_failure = None;
                Ok(state)
            }
            ResourceInspection::Uncertain { message } => {
                self.state = CapabilityState::Failed;
                self.last_failure = Some(message);
                Err(LifecycleError::Unverified)
            }
            ResourceInspection::Verified { .. } => {
                self.state = CapabilityState::Failed;
                self.last_failure = Some("resource state or lease proof mismatched".into());
                Err(LifecycleError::Unverified)
            }
        }
    }
}
