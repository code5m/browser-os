//! Explicit adapters for *future* ACP and A2A integration.
//!
//! ACP is intended for editor <-> coding agent sessions, and A2A for remote
//! agent collaboration. This module defines the host-facing seam only: no
//! official SDK is imported and no ACP/A2A wire compatibility is claimed.

use serde::{Deserialize, Serialize};

use crate::agent::AgentRequest;
use crate::events::EventKind;
use crate::model::BoxFuture;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTransport {
    Native,
    Acp,
    A2a,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReceipt {
    pub remote_task_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentBridgeError {
    Unsupported,
    InvalidRequest,
    RemoteFailure(String),
}

/// Future external bridges map ACP/A2A messages into internal task events.
/// The BrowserOS host assigns event IDs and monotonic sequence numbers.
pub trait ExternalAgentPort: Send + Sync {
    fn transport(&self) -> AgentTransport;
    fn submit<'a>(
        &'a self,
        request: AgentRequest,
        on_event: &'a mut (dyn FnMut(EventKind) + Send + 'a),
    ) -> BoxFuture<'a, Result<AgentReceipt, AgentBridgeError>>;
}
