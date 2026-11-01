//! Context is explicitly scoped and reference-only. This crate does not read it.
use crate::{FoundationError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    BrowserTab,
    EditorSelection,
    WorkspaceFile,
    Project,
    TerminalSelection,
    Resource,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextGrant {
    ExplicitSelection,
    ApprovedScope,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRef {
    pub kind: ContextKind,
    /// Opaque identifier produced by a trusted host; never dereference arbitrary URLs here.
    pub resource_id: String,
    pub grant: ContextGrant,
}

impl ContextRef {
    pub fn checked(kind: ContextKind, resource_id: impl Into<String>, grant: ContextGrant) -> Result<Self> {
        let resource_id = resource_id.into();
        if resource_id.trim().is_empty()
            || resource_id.len() > 4096
            || resource_id.chars().any(char::is_control)
        {
            return Err(FoundationError::InvalidInput("context resource_id"));
        }
        Ok(Self { kind, resource_id, grant })
    }
}

/// This is not a security check: the host must resolve the opaque id and verify consent
/// against the current principal, project scope, and resource revision.
pub fn validate_context_list(context: &[ContextRef]) -> Result<()> {
    if context.len() > 32 {
        return Err(FoundationError::InvalidInput("too many context references"));
    }
    for item in context {
        ContextRef::checked(item.kind.clone(), &item.resource_id, item.grant.clone())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_and_control_characters() {
        assert!(ContextRef::checked(ContextKind::BrowserTab, "", ContextGrant::ExplicitSelection).is_err());
        assert!(ContextRef::checked(ContextKind::BrowserTab, "tab\n1", ContextGrant::ApprovedScope).is_err());
    }
}
