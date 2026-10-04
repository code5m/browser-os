//! Task 能力原生后端（定时任务 + 调度引擎，CAPABILITY_NATIVE(task)）。
//!
//! 本目录承载 M4-6 / M4-7 的定时任务能力原生实现：
//! - `scheduler.rs`：调度循环 / 重试 / 退出收口（task 触发引擎；执行唯一入口委托
//!   `script_runner::start_run` / `start_command`，禁止第二套进程 / spawn 路径）。
//! - `tasks.rs`（后续 Pilot 迁入同目录）：定时任务纯函数（校验 / cron / 持久化）。
//!
//! 命令层：本目录现含两个子模块——
//! - `commands.rs`：`task_list` / `task_add` / `task_update` / `task_remove` /
//!   `task_run_now` 五个命令及其私有 helper（CAPABILITY_NATIVE_ADAPTER），
//!   迁移自 `bridge.rs`（native-physical-batch-task）。
//! - `scheduler.rs`：调度引擎（`start_scheduler` / `stop_scheduler` / `fire_now` 等
//!   命令仍注册在 `bridge.rs`，命令体委托本引擎）。
//! 命令体内部委托本模块引擎 / 纯函数；能力目录不含散落的 `#[tauri::command]` 于
//! `scheduler.rs` 之外。
//!
//! 迁移自 `src-tauri/src/scheduler.rs`
//! （Native Physical Boundary Matrix Pilot 9，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.11）。
//! 既有 `crate::scheduler::` 调用点（bridge.rs ×3：`request_stop` / `cancel_in_flight` /
//! `fire_now`；main.rs ×1：`start`）经 `main.rs` 顶部 re-export shim 解析，无需逐处改写。
//!
//! 已知债务（AppState 阶段处理，非本批回归）：本模块 `use crate::bridge::AppState;`
//! （scheduler.rs line 28 / 648），耦合 bridge hub 的全局 AppState；属矩阵 §9
//! AppState Field Ownership Matrix 待下沉项。
pub mod commands;
pub mod scheduler;
