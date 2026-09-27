//! Framework-native service commands (FRAMEWORK_NATIVE_SERVICE).
//!
//! 与业务 capability 无关的原生服务：前端链路追踪（debug_log）、审计读取（audit_log）、
//! 应用启动（launch_app）等。从 `bridge.rs` 迁入（Native Physical Boundary 分解），
//! 命令体只依赖 `crate::workspace` / `crate::security_policy` 等共享基座，
//! 不反向依赖 `crate::bridge`。
pub mod commands;
