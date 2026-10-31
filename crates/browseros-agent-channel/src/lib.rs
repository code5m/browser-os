//! Isolated protocol primitives for a future BrowserOS Agent Channel.
//!
//! This crate is intentionally not a member of the Tauri package, the npm
//! workspaces, or any production execution path. No model, network, filesystem,
//! terminal, or browser access is performed by this crate.
//!
//! SPDX-License-Identifier: MulanPSL-2.0

#![forbid(unsafe_code)]

pub mod agent;
pub mod context;
pub mod events;
pub mod interop;
pub mod model;
pub mod tools;

/// Version of BrowserOS's *internal* Agent Channel contract, not the version
/// of MCP, ACP, A2A, CloudEvents, or any model provider's protocol.
pub const CONTRACT_VERSION: u16 = 1;
