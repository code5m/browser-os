//! Shared native security primitives. No command output is an authority.
//!
//! Windows private storage means the current token user plus LocalSystem.
//! AppContainer storage has a different policy and must not use this API.
#![forbid(unsafe_op_in_unsafe_fn)]
#[cfg(windows)]
pub mod windows;
