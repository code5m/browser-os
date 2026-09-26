//! Agent 能力原生后端（纯解析 / 校验 + 记忆 KV 契约层，无 Tauri 命令）。
//!
//! 本目录承载两个纯模块：
//! - `agent.rs` — `AgentDef` 外部 JSON 解析与语义校验（M5-4/5，Lane A5, W4）。
//! - `agent_memory.rs` — Agent 记忆 KV 契约层（M5-3，纯存储 + 纯策略，无进程 / 监听 / 网络）。
//!
//! 设计约束（与两文件头注释一致）：
//! - 不执行、不联网、不引桥（`crate::bridge`）。
//! - `agent_memory.rs` 经 `mvp_core::core::seam::PathResolver` 解析持久化目录；
//!   容量 / 隐私 / 审计不变量由 `scripts/check-agent-memory-policy.py` 钉死。
//! - 迁移自 `src-tauri/src/agent.rs` / `src-tauri/src/agent_memory.rs`
//!   （Native Physical Boundary Matrix Pilot，见
//!   `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §6.1）。
pub mod agent;
pub mod agent_memory;
