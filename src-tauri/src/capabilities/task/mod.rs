//! Task 能力原生后端（定时任务 + 调度引擎，CAPABILITY_NATIVE(task)）。
//!
//! 本目录承载 M4-6 / M4-7 的定时任务能力原生实现：
//! - `scheduler.rs`：调度循环 / 重试 / 退出收口（task 触发引擎；执行唯一入口委托
//!   `script_runner::start_run` / `start_command`，禁止第二套进程 / spawn 路径）。
//! - `tasks.rs`（后续 Pilot 迁入同目录）：定时任务纯函数（校验 / cron / 持久化）。
//!
//! 命令层（`start_scheduler` / `stop_scheduler` / `fire_task_now` / `create_task` /
//! `update_task` / `delete_task` / `toggle_task` / `list_tasks` / `get_task` /
//! `list_task_runs` / `get_task_run` 等）注册在 `bridge.rs` 经 `generate_handler!`，
//! 命令体内部委托本模块引擎 / 纯函数。本模块本身**不含 `#[tauri::command]`**。
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
pub mod scheduler;
