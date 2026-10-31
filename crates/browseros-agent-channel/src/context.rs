//! References to contexts selected or authorized by the user.
//!
//! A reference is metadata only: resolving it is the responsibility of a
//! future host adapter that must independently enforce permissions.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    BrowserTab,
    EditorSelection,
    Repository,
    Terminal,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextAccess {
    UserSelected,
    ExplicitlyApproved,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextRef {
    pub kind: ContextKind,
    pub resource_id: String,
    pub label: Option<String>,
    pub access: ContextAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextError {
    EmptyResource,
}

impl ContextRef {
    pub fn new(
        kind: ContextKind,
        resource_id: impl Into<String>,
        access: ContextAccess,
    ) -> Result<Self, ContextError> {
        let resource_id = resource_id.into();
        if resource_id.trim().is_empty() {
            return Err(ContextError::EmptyResource);
        }
        Ok(Self {
            kind,
            resource_id,
            label: None,
            access,
        })
    }
}
