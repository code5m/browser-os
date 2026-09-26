//! Script 能力原生后端（纯校验 / 脱敏 / 内置片段种子，无 Tauri 命令、无执行）。
//!
//! 本目录承载两个 CAPABILITY_NATIVE(script) 的纯函数层：
//! - `scripts.rs`  — 脚本领域纯函数层（M2-3）：id/元数据/正文/参数值校验、脱敏、
//!   持久化协议常量、稳定错误码 `ScriptError`。
//! - `snippets.rs` — 命令片段领域纯函数层（M2-6）：整元素占位 argv 校验、
//!   内置片段种子、稳定错误码 `SnippetError`。
//!
//! 两者共享 `domain` 类型；`snippets` 复用 `scripts` 的字段口径
//! （`validate_param_name` / `validate_script_id` / `MAX_*`），不另设一套，避免漂移。
//!
//! 设计约束（与原文头注释一致）：
//! - 不执行、不安装、不联网、不引桥（`crate::bridge`）。
//! - 执行层在调用方经 `script_runner` 委托，而非在此处直接执行。
//!
//! 迁移自 `src-tauri/src/scripts.rs` 与 `src-tauri/src/snippets.rs`
//! （Native Physical Boundary Matrix Pilot 3，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.3）。
//! 既有 `crate::scripts::` / `crate::snippets::` 调用点经 `main.rs` 顶部 re-export
//! shim 解析，无需逐处改写。
//!
//! `script_runner.rs`（M2-4.c 脚本执行进程组 + 生命周期内核）在 Pilot 10 迁入同目录：
//! 本模块**只做进程/生命周期**，不含 `#[tauri::command]`（命令层在 `bridge.rs`）；
//! 定义 `ScriptProcessTable`（AppState 字段 `script_runs` 的类型）、`RunSnapshot` /
//! `ScriptRunRecord` / `ScriptOutputEvent` / `ScriptFinishedEvent` / `start_run` /
//! `start_command` / `kill_all_running` / `process_group_alive`。既有 `crate::script_runner::`
//! 调用点（bridge 6 / tasks 2 / terminal 1 / scheduler 3）经 `main.rs` 顶部
//! `pub use crate::capabilities::script::script_runner;` shim 解析。
pub mod scripts;
pub mod snippets;
pub mod script_runner;
