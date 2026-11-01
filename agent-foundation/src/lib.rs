//! Standalone Agent Channel prototype. Never calls BrowserOS/Tauri/native commands.
//! Protocol internals are independent of model vendors and editor implementation.
#![forbid(unsafe_code)]

#[cfg(any(
    feature = "adapter-rig",
    feature = "adapter-rmcp",
    feature = "adapter-acp"
))]
pub mod adapters;
pub mod approval;
pub mod benchmark;
pub mod context;
#[cfg(feature = "durable-sqlite")]
pub mod durable;
pub mod engine;
pub mod event;
pub mod experiment;
pub mod model;
pub mod protocol;
pub mod public_api;
#[cfg(feature = "experimental-rig-run")]
pub mod rig_driver;
#[cfg(feature = "experimental-rig-run")]
pub mod rig_run;
pub mod task;
pub mod tool;

use std::fmt;

pub type Result<T> = std::result::Result<T, FoundationError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundationError {
    InvalidInput(&'static str),
    Storage(String),
    NotFound,
    InvalidTransition,
    ApprovalMismatch,
    PermissionDenied,
    Cancelled,
    Timeout,
    ReplayGap,
    Provider(String),
    Tool(String),
    Serialization(String),
}

impl fmt::Display for FoundationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "storage: {error}"),
            Self::InvalidInput(reason) => write!(f, "invalid input: {reason}"),
            Self::NotFound => write!(f, "resource not found"),
            Self::InvalidTransition => write!(f, "invalid task transition"),
            Self::ApprovalMismatch => write!(f, "approval does not match pending call"),
            Self::PermissionDenied => write!(f, "operation denied"),
            Self::Cancelled => write!(f, "operation cancelled"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::ReplayGap => write!(f, "event replay gap; request task snapshot"),
            Self::Provider(error) => write!(f, "model provider: {error}"),
            Self::Tool(error) => write!(f, "tool: {error}"),
            Self::Serialization(error) => write!(f, "serialization: {error}"),
        }
    }
}
impl std::error::Error for FoundationError {}
