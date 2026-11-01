//! Host-authorized tool calls and a one-shot in-memory approval gate.
//!
//! RMCP can later implement ToolPort. No MCP server, file/process access, or
//! global tool registration is provided. The host MUST mint Approval decisions
//! from a trusted UI (not from model text), and MUST re-check real resource
//! ownership/permissions immediately before side effects.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::BoxFuture;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRisk {
    Read,
    Write,
    Execute,
    External,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub risk: ToolRisk,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub request_id: String,
    pub name: String,
    /// Host-defined exact scope, e.g. a selected repository or browser tab.
    pub scope: String,
    pub arguments: Value,
}

/// A trusted host must construct this after a user approval; untrusted model
/// output must never be accepted as an Approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Approval {
    Deny,
    AllowOnce {
        request_id: String,
        name: String,
        scope: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolError {
    InvalidDefinition,
    DuplicateTool,
    UnknownTool,
    InvalidCall,
    Denied,
    ApprovalMismatch,
    ApprovalUsed,
    ExecutorFailed,
}

#[derive(Debug, PartialEq)]
pub struct AuthorizedToolCall {
    call: ToolCall,
}

impl AuthorizedToolCall {
    pub fn call(&self) -> &ToolCall {
        &self.call
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub request_id: String,
    pub output: Value,
}

#[derive(Default)]
pub struct ToolCatalog {
    tools: BTreeMap<String, ToolSpec>,
    consumed_request_ids: BTreeSet<String>,
}

impl ToolCatalog {
    pub fn register(&mut self, spec: ToolSpec) -> Result<(), ToolError> {
        if spec.name.trim().is_empty() || !spec.input_schema.is_object() {
            return Err(ToolError::InvalidDefinition);
        }
        if self.tools.contains_key(&spec.name) {
            return Err(ToolError::DuplicateTool);
        }
        self.tools.insert(spec.name.clone(), spec);
        Ok(())
    }

    pub fn specs(&self) -> impl Iterator<Item = &ToolSpec> {
        self.tools.values()
    }

    /// Default deny, including read-only tools. A grant must match the exact
    /// request id, tool name, and scope, and cannot be replayed in this catalog.
    /// Durable replay protection is a future host responsibility.
    pub fn authorize(
        &mut self,
        call: ToolCall,
        approval: Approval,
    ) -> Result<AuthorizedToolCall, ToolError> {
        if call.request_id.trim().is_empty()
            || call.name.trim().is_empty()
            || call.scope.trim().is_empty()
            || !call.arguments.is_object()
        {
            return Err(ToolError::InvalidCall);
        }
        if !self.tools.contains_key(&call.name) {
            return Err(ToolError::UnknownTool);
        }
        match approval {
            Approval::Deny => return Err(ToolError::Denied),
            Approval::AllowOnce {
                request_id,
                name,
                scope,
            } => {
                if request_id != call.request_id || name != call.name || scope != call.scope {
                    return Err(ToolError::ApprovalMismatch);
                }
            }
        }
        if !self.consumed_request_ids.insert(call.request_id.clone()) {
            return Err(ToolError::ApprovalUsed);
        }
        Ok(AuthorizedToolCall { call })
    }
}

/// Implemented by a future host-owned tool adapter, not by this crate.
/// Only AuthorizedToolCall is accepted so models cannot directly invoke it.
pub trait ToolPort: Send + Sync {
    fn execute<'a>(
        &'a self,
        call: AuthorizedToolCall,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>>;
}
