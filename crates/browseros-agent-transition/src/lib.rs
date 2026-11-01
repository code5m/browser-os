//! Migration-only contracts. Unregistered, no Tauri integration or side effects.
//! Original BrowserOS and browseros-agent-channel files remain untouched.
//! SPDX-License-Identifier: MulanPSL-2.0
#![forbid(unsafe_code)]

pub mod apply;
pub mod concurrency;
pub mod lifecycle;
pub mod runtime;

pub const TRANSITION_CONTRACT_VERSION: u16 = 1;
