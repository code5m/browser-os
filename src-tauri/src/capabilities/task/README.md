# Capability Native Module: `task` (Rust)

> 本目录 = `src-tauri/src/capabilities/task/`（Native Physical Boundary Matrix Pilot 9 起）。
> 分类：**CAPABILITY_NATIVE(task)**（矩阵 §2.3；target 同目录）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.11。
> 同级 TS 能力：`src/capabilities/task/`（成熟度 C1，命令接线仍经 `bridge.ts`）。

## 1. DDD 职责（Domain Responsibility）

定时任务能力的原生实现，目前迁入 `scheduler.rs`（M4-7），`tasks.rs`（M4-6 纯函数）待后续 Pilot 同目录迁入。

- **`scheduler.rs`（已迁入）** —— 安全触发与退出：调度循环（`tick` / `run_loop`）、重试（`retry_within_budget`）、错过补跑（`collect_missed_slots` / `is_missed` / `should_fire`）、退出收口（`stop-scheduler` 注册序）、审计下沉（`record_run_start` / `record_run_finish` 低频动作级，禁止 tick 内逐轮写 audit）。仅做触发，执行唯一入口 = `script_runner::start_run` / `start_command`（F6 禁止第二套进程 / spawn 路径）。
- **`tasks.rs`（待迁入）** —— 定时任务纯函数：参数校验（`validate_params`）、cron 纯函数（`next_fire_after` 等时间入参纯函数，可测性等价且更强）、`tasks.json` 原子写持久化（`crate::session::atomic_write`）。不含调度循环。

## 2. 边界（Boundary / Non-Responsibility）

- 本目录模块**不含任何 `#[tauri::command]`**：task 命令体（`start_scheduler` / `stop_scheduler` / `fire_task_now` / `create_task` / `update_task` / `delete_task` / `toggle_task` / `list_tasks` / `get_task` / `list_task_runs` / `get_task_run`）注册在 `bridge.rs`（经 `generate_handler!`），命令体委托本目录引擎 / 纯函数。
- **不执行脚本**：scheduler 只触发，执行一律委托 `script_runner`（F6 硬约束，由 `check-scheduler-policy.py` 机器守护）。
- **不引入 `tokio-cron-scheduler` / `cron` crate**：cron 方言自研纯函数，零新依赖（契约 §4.4）。
- **tick / 主循环内不得逐轮写 `audit.json`**（F7 / §7）：审计上限 1000 FIFO，每秒 1 条约 17 分钟冲光；审计仅在下层 `record_*` 低频动作级事件。

## 3. Commands

**本目录无命令。** task 命令体归属 task 能力，注册在 `bridge.rs`（经 `generate_handler!`），命令体调用本目录 `scheduler` / `tasks` 引擎 / 纯函数。命令体在 **bridge.rs 逐 command 分解阶段**迁移至 `capabilities/task/commands.rs`。

## 4. Resources

- 文件系统：`tasks.json` / `task-runs.json`（原子写，走 `crate::session::atomic_write`）；判重真相源 = `tasks.json.last_fired_at`，`task-runs.json` 只做历史不参与判重。
- 后台线程：调度主循环（`SchedulerHandle` 由 `OnceLock` 保证全进程仅一次启动），由 `stop-scheduler` 关机任务收口（注册序 < `kill-running-scripts`）。
- 无 WebView / PTY / socket / 子进程（执行委托 script_runner）/ 定时器（调度自带循环）。

## 5. 生命周期（Lifecycle）

- 启动：main.rs `setup` 调 `crate::scheduler::start(app.handle())`（全进程仅一次）。
- 退出：`stop-scheduler` 关机任务（索引 < `kill-running-scripts`）置停止信号 → 主循环 ≤1s 响应退出。
- 运行态：调度主循环常驻；`task-runs.json` 追加运行记录。

## 6. 依赖（Dependencies）

- `crate::domain`：`TaskDef` / `TaskRunRecord` / `TaskRunTrigger` / `RunStatus` / `MissedRunPolicy` / `SCHED_MAX_ATTEMPTS` / `SCHED_RETRY_TOTAL_BUDGET_SECS`（类型真源，SHARED）。
- `crate::script_runner`：`RunError` / `RunSnapshot` / `ScriptProcessTable`（执行委托，SHARED_NATIVE_INFRASTRUCTURE）。
- `crate::tasks`：定时任务纯函数（同能力，待迁入同目录后同包引用）。
- `crate::session`：`atomic_write`（原子写原语，SHARED）。
- 标准库 + `chrono` + `tauri`（`AppHandle` / `Manager`）。
- **已知债务（AppState 阶段）**：`scheduler.rs` `use crate::bridge::AppState;`（line 28 / 648）—— 耦合 bridge hub 全局 AppState，属矩阵 §9 待下沉项。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge` 之外的跨能力 hub 直调（`bridge::AppState` 属待下沉债务）。
- 禁止第二套进程 / spawn 路径执行脚本（F6）。
- 禁止引入 `tokio-cron-scheduler` / `cron` crate。
- 禁止 tick / 主循环内逐轮写 `audit.json`（F7 / §7）。

## 8. Public / Native Contract

- `scheduler`：`start` / `request_stop` / `cancel_in_flight` / `fire_now` / `reap_finished` / `record_run_start` / `record_run_finished` / `validate_params`（fire 路径复用）/ `Clock` trait / `SystemClock` / `TICK_SECS` / `STOP_POLL_MS` / `SKIP_STOPPING`。
- `tasks`（待迁入）：`validate_params` / cron 纯函数 / `TaskDef` 持久化辅助。
- 既有 `crate::scheduler::` 调用点（bridge.rs ×3 + main.rs ×1）经 `main.rs` 顶部 `pub use crate::capabilities::task::scheduler;` re-export shim 解析，**未逐处改写**。
- TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。

## 9. Security / ACL

- task 命令在 `permissions/default-commands.toml` 放行（与 `bridge.ts` / `src/types.ts` 镜像一致）。
- 审计 FIFO 上限 1000；tick 内禁写（防冲光）。
- 门禁真源：`scripts/check-scheduler-policy.py`（扫 bridge.rs + 递归 glob `scheduler*.rs` / `tasks*.rs`；ACTIVE 码含 SCHED_*）。

## 10. Tests

- `scheduler.rs` 内 `#[cfg(test)] mod`（line ~861 起：错过补跑 / 重试预算 / 取消 / 时间纯函数）。
- 运行：`cd src-tauri && cargo test capabilities::task`（或 `cargo test` 全量）。

## 11. Source of Truth

- 调度引擎：`src-tauri/src/capabilities/task/scheduler.rs`（本目录）。
- 定时任务纯函数：`src-tauri/src/tasks.rs`（待迁入同目录）。
- 类型真源：`src-tauri/src/domain.rs`。
- 命令体（暂留）：`src-tauri/src/bridge.rs`。
- 前端接线：`src/bridge.ts`（`startScheduler` / `stopScheduler` / `fireTaskNow` 等）+ `src/types.ts` + `src/stores/useTaskStore.ts` + `workspace/SchedulerView.vue`。
- 门禁真源：`scripts/check-scheduler-policy.py`。

## 12. Known Debt

- `scheduler.rs` 耦合 `crate::bridge::AppState`（F7 审计下沉依赖全局状态）；AppState 阶段下沉到 task 能力本地状态。
- `tasks.rs` 尚未迁入（待后续 Pilot）；迁入后 `use crate::tasks;` 顶层引用转 shim。
- 命令体仍在 `bridge.rs`（LEGACY_MIXED_MODULE 残核）；属矩阵 §8 末段 bridge 分解计划。

## 13. Extraction Readiness

- **高。** 纯引擎 / 纯函数、零命令、跨模块依赖仅 SHARED（`domain` / `script_runner` / `session`）+ 标准库；唯一的 hub 耦合（`bridge::AppState`）为已知债务，待 AppState 阶段下沉。
- 达到阈值后可随 `task` 能力提级为 crate `mvp-task-rust`（不改领域语义）。
- 同能力下一个同批候选：`tasks.rs` 迁入 `capabilities/task/tasks.rs`（零命令纯函数，低风险）。
