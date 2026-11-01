//! Official SDK types at the external boundaries; no SDK owns BrowserOS permissions.
//! Feature-gated and no actual browser/native tool access.
#[cfg(feature = "adapter-acp")]
pub mod acp;
#[cfg(feature = "adapter-rig")]
pub mod rig;
#[cfg(feature = "adapter-rmcp")]
pub mod rmcp;
