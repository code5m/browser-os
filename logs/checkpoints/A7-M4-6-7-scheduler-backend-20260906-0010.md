# Lane A7 — M4-6 / M4-7 调度后端完整代码包（交付检查点）

> 状态：**START → DONE**（代码包已落地、机器门禁全绿、单测全绿；前端镜像归 A8，未做）
> 时间：2026-09-06 00:10 CST
> 依赖的冻结契约：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`（Lane A6，已集成）
> 机器门禁：`scripts/check-scheduler-policy.py`（ACTIVE=23，无 pending）

## 1. 范围与边界（严格按 `PARALLEL_COMMAND_BOARD.md` Batch §A7）

- **Allowed Scope**：`src-tauri/src/{domain,scheduler,tasks,bridge,workspace,main}.rs`、`src-tauri/permissions/default-commands.toml`、Rust tests。
- **不在本 lane 范围（属 A8 前端 lane）**：`src/types.ts`、`src/bridge.ts`、`src/components/**`、`src/stores/**`、`scripts/check-scheduler-ui-logic.mjs`、`scripts/check-scheduler-ui-policy.py` 的前端部分。
- **未 push**（规则），交 A0 集成 master。
- 工作树含 A3/A4/A6/A11 的并发改动（database.rs 等），pull 不可行 → 按 Batch Rule 3 走补丁交付。

## 2. 交付文件与改动

| 文件 | 改动 | 说明 |
|---|---|---|
| `src-tauri/src/domain.rs` | 追加 | `TaskDef`/`TaskRunRecord`/`TaskTrigger`/`TaskKind`/`MissedRunPolicy`/`RetryPolicy`/`TaskRunTrigger` 及常量（§3 字段、R-A6-1 默认 `enabled=false`、`missed_run_policy` 为字段、新增字段均 `serde(default)`） |
| `src-tauri/src/tasks.rs` | **新建** | 纯函数层：cron 5 段解析、判定函数（`next_fire_after`/`collect_missed_slots`/`is_missed`/`should_fire`/`validate_trigger`/`validate_task`/`validate_params`）、重试退避+抖动、原子持久化（`tasks.json`/`task-runs.json`，损坏备份 `.corrupt`）、23 个单测 |
| `src-tauri/src/scheduler.rs` | **新建** | `Clock` trait + `SystemClock`、可中断 sleep（停止响应 ≤50ms）、`plan_slots` 错过的执行纯决策、`SchedulerHandle`、`stop-scheduler` 钩子、主循环 `run_loop`/`tick`、触发 `fire`（复用 `script_runner::start_run`/`start_command`）、回收/重试/自动禁用、命令入口 `fire_now`/`cancel_in_flight`、审计下沉 `record_*`、8 个单测 |
| `src-tauri/src/bridge.rs` | 改 | 注册 `stop-scheduler`（索引 1，早于 `kill-running-scripts`）；新增 5 命令 `task_list/add/update/remove/run_now`（全部过 `check_invocation_source`、ACL、审计脱敏） |
| `src-tauri/src/main.rs` | 改 | `mod scheduler; mod tasks;`；invoke_handler 注册 5 命令；`setup` 中 `scheduler::start` |
| `src-tauri/src/workspace.rs` | 改 | `data_dir` 由 `fn` 升 `pub fn`（供 `tasks.rs` 复用同一数据目录，避免第二条持久化路径） |
| `src-tauri/permissions/default-commands.toml` | 改 | 5 命令插在 `list_artifact_images` 锚点之前 |

## 3. 机器门禁结果

- `python3 scripts/check-scheduler-policy.py` → **scheduler policy: all invariants hold（ACTIVE=23）** ✅
  - F6 第二执行路径禁止 ✅（scheduler/tasks 无 `std::process::Command`/`sh -c`/`bash -c`）
  - F8 判定函数无裸时钟 ✅（时间全部入参传入）
  - F9 错过策略=字段 ✅；F10 原子写 ✅；§7 tick 禁审计 ✅（tick/run_loop 体内无 `log_audit`）
  - §4.4 cron 宏拒绝 ✅；§3.4 新增字段 default ✅；§3.3 R-3 secret 禁止落盘 ✅
  - §3.5 R-A6-1 默认 `enabled=false`，且 `default_task_enabled` 体内无 `true` 字面量 ✅
  - A10 R-1 每次执行 start/finish 不进 audit ✅（仅动作级事件写；**Integration Fix Wave 已补 gate 盲区**：扫描从仅 `bridge.rs` 扩到 `bridge.rs`+`scheduler.rs`，并落地 scheduler.rs 真改——见 §8）
  - F7 退出序：scheduler 存在即按序注册 `stop-scheduler` < `kill-running-scripts` ✅
  - 命令三处同步（ACL+main.rs+来源校验）、ACL 顺序、审计不泄露 params ✅；重试/补跑硬上界 ✅；判重不依赖历史 ✅
- `python3 scripts/check-scheduler-policy.py --self-test` → 合成合规样本零违规 + 坏样本全检出 ✅
- `cargo check --locked` → 我的文件 0 错误、0 新增 warning（database.rs 的 41 dead_code 警告属 A3 lane，非本包）✅
- `cargo test --locked` → **329 passed / 0 failed**（A1 基线 232 不回归；A7 新增 23 + Integration Fix Wave 新增 1 计入）✅
  - A7 子集：`task_domain_tests` 15 passed（c1–c4、c13、T-task-1–5、R-A6-1 默认 false）、`scheduler_tests` 8 passed（c5 Skip / c6 RunOnce 最新 / c7 CatchUp 上界 / c10 同任务不重入 / 判重过滤 / on-time 单次 / 可中断 sleep / 持久性错误分类）

## 4. 已登记的设计偏离（交 A0 裁定）

- **O-A7-1**（dom/dow 取交集）：cron `day-of-month` 与 `day-of-week` 同时非通配时取**交集（AND）**，而非 Vixie 标准的并集（OR）。取交集 = 触发更少 = fail-closed，且语义简单可测。契约 §4.4 未规定该项，已在 `CronExpr::matches_naive` 注释登记。
- **O-A7-2**（CatchUp 同任务不并发张力）：契约 §5.1 允许 `CatchUp` 一轮补跑多个 slot，但 §5.2 同时规定「同任务 in_flight 非空则跳过、且不排队」；在「F9 同任务不并发」前提下二者不可兼得。`plan_slots` 按契约产出**意图**（`fire` 至多 `catch_up_limit` 个），但执行层每次真正触发后即占 `in_flight`，超出的 slot 在下一轮按 `reentrant` 跳过（不消耗重试配额）。即实际每轮至多补 1 次。c7 单测按「意图层」校验（`catch_up_limit=3 → fire.len()==3`），已在函数注释与本节登记。

## 5. 未做 / 交下游

- **前端 UI 与 TS 镜像**（types.ts / bridge.ts / store / 组件 / `check-scheduler-ui-*.{mjs,py}`）属 **A8 lane**，本包未触。（A8 的 `Must verify` 依赖本包的 `task_*` 命令 DTO，现已稳定。）
- `stop-scheduler` 注册顺序的 Rust 集成测试依赖 `ShutdownCoordinator` 公开顺序 API + Tauri App 构建，跨模块重量大；其顺序正确性已由 `check-scheduler-policy.py` 的 F7 机器门禁（`stop-scheduler` < `kill-running-scripts` 文本顺序）等价证明。

## 6. `pre-merge.sh` 整体结果归因

`bash scripts/pre-merge.sh` → `PRE_MERGE_RESULT=FAIL`，但 FAIL 项**均不属 A7**：

| FAIL 项 | 归因 lane | 与本包关系 |
|---|---|---|
| M0-4.c build metrics regression（cargo warning 增多） | **A3**（database.rs 41 个 dead_code 警告 + grid_process.rs） | 本包 0 新增 warning |
| `check-tools-policy.py --self-test` | 工具 lane（M2-7，非 M4） | 无关 |
| `check-database-policy.py --self-test` / `--expect-pending` | **A4**（数据库安全闸门未合入，pending 未转） | 无关 |

A7 自身的 `M4-5.d 调度契约不变量夹具` 在该运行中**无 FAIL**。

## 7. 不 push 声明

按规则，**未 push**。代码包留于本地工作树，交 A0 在 master 同步后集成（rebase / 补丁合并），由 A0 统一 push。

## 8. Integration Fix Wave 修复项（A10 复查 R-1 / R-4 / R-11）

> 时间：2026-09-06（续做）
> 触发：A10 批量复查把 R-1 / R-4 转本 lane 修复，R-11 冻结常量，R-2 评估（冻结契约下不单方面改行为）。

### 8.1 R-1（高）— 每轮执行 start/finish 审计落 task-runs.json（已真改 + 堵 gate 盲区）

- **真改（scheduler.rs）**：
  - `record_run_start`：仅 `trigger == "manual"` 时写 `task.run.manual`（手工触发是低频用户动作，契约 §6）；
    `scheduled`/`catchup`/`retry` 触发**不再写任何 audit**（明细在 `reap_finished` 的 `append_task_run` 落 `task-runs.json`）。
  - **删除 `record_run_finish`**（连同唯一消费者 `status_str` 一起移除，避免 dead_code warning）→ `reap_finished` 仅保留 `append_task_run`。
  - 效果：最小间隔 60s 的任务单任务 1440 次/天不再冲掉 `audit.json` 的 cap 1000 环形缓冲。
- **堵 gate 盲区（check-scheduler-policy.py）**：`SCHED_AUDIT_PER_RUN_EVENT` 原只扫 `bridge.rs`，scheduler.rs 回退写 `task.run.start/finish` 会**静默失效**。
  改为同时扫 `("bridge", bridge)` 与 `("scheduler", sched)`；并保留 `bridge_rs` 真实仓库变异样本，真实仓库仍零违规（good 样本 A/B 零违规）。

### 8.2 R-4（中）— fire 路径复跑 R-3 secret/参数定义期校验（自动禁用 + 审计）

- **真改（scheduler.rs `fire`）**：Script / Command 两个分支在 `start_run`/`start_command` **之前**加
  `if let Err(e) = tasks::validate_params(&target.params, &task.params) { return FireOutcome::Reject(e.code()); }`。
- 命中即走 §5.2 持久性错误处置（`enabled=false` + 审计），与 `task_update`/`task_add` 定义期拒绝口径一致。
- 失败闭环语义：目标脚本事后把某参数改标 secret / 新增必填参数，既有任务持明文值或缺值会在真正执行前被拒并自动禁用——无需等新任务创建。

### 8.3 R-11（低）— 冻结重试总时长硬上界常量 + 单测

- **真改（domain.rs）**：新增 `SCHED_RETRY_TOTAL_BUDGET_SECS: u64 = 5400`（90 分钟，自最初触发点 `scheduled_at` 起算）。
- **真改（scheduler.rs `reap_finished`）**：重试 push 前经纯函数 `retry_within_budget(scheduled_at, due_at, budget)` 判定；
  超出则放弃后续重试，失败任务保持终态（不自动禁用——属可重试错误，非持久性错误）。
- 新增单测 `retry_total_duration_is_bounded`（边界 + 越界 + 更小预算三态），已计入 329 passed。

### 8.4 R-2（中高）— 调度记录配额评估（**不单边改行为**，交 A0/A11）

- 契约 §7（Lane A6 冻结）明确**复用** `script-runs.json`（`MAX_RUN_RECORDS=200` FIFO）承载调度执行明细。
- 评估结论：调度密集时，调度 run 会按 FIFO 挤出**手工** run 历史（同一环形缓冲）。这是契约冻结口径下的已知权衡，**本 lane 不单方面改**。
- 建议（待 A0 裁决）：评估为调度 run 启用**独立文件**（如 `task-runs.json` 已存在、可独立限容）或独立配额，以隔离调度/手工历史。
  已登记至 §6 之外的 A0/A11 验证清单，**未落地代码改动**。

### 8.5 复跑结果（本轮末态）

| 验证 | 结果 |
|---|---|
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | 干净（修复 IF-1 fmt 门禁） |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **329 passed / 0 failed**（scheduler/task 全绿，含 R-11 新测） |
| `python3 scripts/check-scheduler-policy.py --self-test` | PASS：2 好样本零违规 + 23 合成坏样本 + 23 真实仓库变异全检出 |
| `python3 scripts/check-scheduler-policy.py` | all invariants hold（ACTIVE=23） |
| `git diff --check` | 无空白错误 |

- 仍 **不 push**；代码包与本文档留本地工作树，交 A0 集成 master。
- 与 A3/A4/A6/A11 的并发改动同在树中，按 Batch Rule 3 走补丁交付，不 rebase 上游。
