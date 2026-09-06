# A6 — M4 调度策略夹具修复包（Integration Fix Wave / Batch Implementation Dispatch）

- **Lane**: A6
- **检查点时间**: 2026-09-06 13:41 CST
- **Dispatch 来源**: `PARALLEL_COMMAND_BOARD.md` §Batch Implementation Dispatch → Integration Fix Wave → Lane A6: Scheduler Policy Fix Package（行 266-275）
- **目标**: 让 `scripts/check-scheduler-policy.py` 的 active 码位集合与 A7 已落地的调度实现保持一致、对真实仓库无假阳性；产出自测 + checkpoint，零产品代码改动（仅策略脚本注释订正 + 本检查点）。
- **Base**: `854bc40 feat(M5): add core boundary gate`（本地领先 origin/master 4）
- **HEAD / Files**:
  - `scripts/check-scheduler-policy.py`（注释订正：过时 `PENDING` 段标签 → `ACTIVE`；行为/码位不变）
  - `logs/checkpoints/A6-M4-scheduler-policy-fix-20260906-1341.md`（本文件）

## 1. 调度实现落地事实（A7，已核实）

A7 的调度后端已在仓库中真实落地，本包据此确认策略脚本与其一致：

| 产物 | 路径 | 关键证据 |
|---|---|---|
| 调度决策 | `src-tauri/src/scheduler.rs` | 仅触发，执行入口 `crate::script_runner::start_run`（行 651）；`last_fired_at` 判重真相源（行 118/364/413）；`catch_up_limit`（行 117/182/388）；`SCHED_MAX_ATTEMPTS` 重试硬上界（行 30/544）；`SKIP_REENTRANT` 占用跳过（M4-5.e 裁定 R-A6-3） |
| 任务持久化 / 校验 | `src-tauri/src/tasks.rs` | `save_tasks_at` 走 `session::atomic_write`（F10）；`TASK_SECRET_PARAM_FORBIDDEN` 定义期拒绝（R-3）；`matches_naive` cron dom/dow 交集（R-A6-2） |
| 五命令 IPC 面 | `src-tauri/src/bridge.rs`（行 952-1097） | `task_list/add/update/remove/run_now` 均过 `check_invocation_source` |
| 命令注册 | `src-tauri/src/main.rs`（行 1422-1426） | 五命令 `bridge::task_*` 全部注册 |
| ACL | `src-tauri/permissions/default-commands.toml`（行 110-114） | 五命令均在 `list_artifact_images` 之前 |
| 退出收口 | `src-tauri/src/shutdown.rs` + `bridge.rs` | `stop-scheduler` 注册于 `kill-running-scripts` 之前（F7） |

## 2. 策略脚本一致性结论（核心交付）

`scripts/check-scheduler-policy.py` 的 `ACTIVE_CODES`（行 105-133）已包含 **23 个 ACTIVE 码位**，其中 M4-5.d 收尾时由 Lane A7 落地并提升的 6 个命令/边界码位（`SCHED_CMD_NOT_REGISTERED` / `SCHED_ACL_ORDER` / `SCHED_AUDIT_LEAKS_PARAMS` / `SCHED_RETRY_UNBOUNDED` / `SCHED_CATCHUP_UNBOUNDED` / `SCHED_HISTORY_AS_IDEMPOTENCY`）**确为 ACTIVE**，`PENDING_CODES = ()`。

> 修复前隐患：脚本第 497 行起的责任段注释仍标 `# ================= PENDING：M4-6 / M4-7 职责 =================` 及 `PENDING 1/2/3/4/7/8)` 子标签，与实际（码位已 ACTIVE、PENDING 集合已空）不符，会误导维护者以为这些门禁未生效。本包仅将其订正为 `ACTIVE`（段头 + 子标签 15-20），**未改动任何检测逻辑、码位清单或自测夹具**。

## 3. 自测结果（机器证据）

```text
# --self-test
SCHED_SELF_TEST_RESULT=PASS: 2 好样本零违规（真实仓库 + 合成参考实现）
  + 23 个合成坏样本全检出 + 23 个真实仓库变异全检出（含变异防呆）；ACTIVE=23 PENDING=0

# default（真实仓库）
scheduler policy: all invariants hold（ACTIVE=23）

# --expect-pending
SCHED_PENDING_RESULT=NONE（0 个 pending 码位均未实现）
```

- 真实仓库变异覆盖：每个 ACTIVE 码位均至少一个来自 A7 真实实现形状的变异被检出（如 `task_list` 撤出 ACL → `SCHED_CMD_NOT_REGISTERED`；`task_list` 移到 `list_artifact_images` 之后 → `SCHED_ACL_ORDER`；`task.add` 审计 detail 泄露参数 → `SCHED_AUDIT_LEAKS_PARAMS`；`max_attempts` 改名 → `SCHED_RETRY_UNBOUNDED`；`catch_up_limit` 改名 → `SCHED_CATCHUP_UNBOUNDED`；`last_fired_at` 改名 → `SCHED_DEDUP_NOT_LAST_FIRED`），证明门禁对真实代码形状有效、无假通过。
- 无假阳性：真实仓库 default 模式 0 违规（`tasks.rs` 仅 `std::process::id()` 读进程号，非 spawn，不命中 `EXEC_FORBIDDEN`；`scheduler.rs` 复用 `start_run`，无第二执行路径）。

## 4. pre-merge 关联

- `scripts/pre-merge.sh` 中 `M4-5.d 调度契约不变量夹具` 子项调用本脚本，随本包注释订正仍 PASS。
- **整体 `PRE_MERGE_RESULT=FAIL`，但失败来源与 A6 无关**：本轮工作树含其他 Lane 的 W2 在途（未提交）改动，失败项为 `cargo fmt main`（A3/A4/A7 Rust 格式）、`check-script-exec-policy.py --self-test` 与默认模式（M2 脚本执行通道策略，另一脚本、另一 Lane）。A6 调度策略包零贡献失败。

## 5. 范围与边界

- **零产品代码改动**：仅策略脚本注释 + 本 checkpoint；未触碰 `src/`、`src-tauri/`、`default-commands.toml`、三份主文档、ACL。
- **未抢占其他 Lane**：工作树中 A1/A2/A3/A4/A5/A7/A8/A9/A10/A11 的在途文件（M5-1.b、M5 各卡、DB/security_policy 等）一律未改动、未回退。
- **M5-6 Agent/Skill UI 仍归 A6 prework-only**：本包为 M4 调度策略修复（Integration Fix Wave），与 M5-6 UI 无关；M5-6 实现仍 BLOCKED（W1 硬停止：仅 A2 可写产品代码，M5-4/M5-5 未落地），归 A19 批。

## 6. STATUS / NEXT

- `STATUS=PASS`：A6 调度策略修复包交付；脚本与 A7 实现一致、三模式全绿、注释订正消除误导。
- `NEXT`：等待其他 Lane 修复 `cargo fmt` 与 `check-script-exec-policy` 后整体 pre-merge 转 ALL_PASS；A6 转回 M5-6 UI prework 待命（M5-4/M5-5 落地后由 A19 取 `logs/assist/A6-M5-agent-ui-20260906-1200.md` 开工）。
- 本包已本地提交（A6 lane 包），**未 push**（仅 A0 push）。
