# A9 M5 定时任务驱动图谱更新接缝（Batch Implementation Dispatch 交付）

> 生成：2026-09-06 07:00 CST · Lane A9（M5 prework only · docs only）
> 性质：M4-7/A7 调度器 → M5 图谱维护任务的**接缝契约草案**。零产品代码。
> 锚定事实：来自对当前工作树 A7 已落地代码的只读核对（§0）。
> 配套：`A9-M5-graph-store-contract-20260906-0700.md`（§3 实时增量/周期/补采三层）· `A9-M5-A13plus-cards-20260906-0010.md`（A17 卡）。

---

## 0. 事实基线（2026-09-06 07:00 实测，只读核对）

| 项 | 已落地事实（文件:行） | 对图谱维护接缝的意义 |
|---|---|---|
| 注册序 | `src-tauri/src/bridge.rs:778` `register_shutdown_tasks`；`:797` `stop-scheduler` 紧随 `stop-background-workers` 之后注册（索引 1），**早于** `kill-running-scripts`（注释明述 F7） | 图谱维护任务随 `stop-scheduler` 收口；图谱自身 flush 注册位见 §6 G-D6 |
| 触发执行 | `src-tauri/src/scheduler.rs:600` `fire()` 经 `script_runner::start_run` / `start_command`（`:660`/`:689`）；**无** `std::process::Command` 第二路径 | 图谱维护任务**只能**是 Script/Command 执行体（见 §2 硬约束） |
| 非重叠 | `scheduler.rs:456` `is_occupied` + InFlight 表（`:217`）；同任务单实例，下一 tick 按 reentrant 跳过 | 超长 rebuild 不被下一 tick 顶掉 |
| 手动触发 | `scheduler.rs:717` `fire_now(app, id)`（`trigger=Manual`，**不推进** `last_fired_at`/`next_run_at`） | 运维手动 `graph_rebuild` 不影响计划 |
| 取消在飞 | `scheduler.rs:737` `cancel_in_flight(app, id) -> usize` | rebuild 中途停 |
| 任务类型 | `domain.rs:1317` `TaskKind{Script, Command}`（**仅**二者；F5 明确不做 Tool） | 图谱维护必须落成 Script/Command 片段，**不得**新增 `TaskKind::Builtin`（§2 硬约束） |
| 触发定义 | `domain.rs:1306` `TaskTrigger::Cron{expr}`（5 段）/ `Interval{every_secs}`；`:1299` `TASK_INTERVAL_MIN_SECS=60`；`:1301` 上界 2592000 | 图谱任务 interval 下界 60s |
| 错过策略 | `domain.rs:1335` `MissedRunPolicy{Skip(默认),RunOnce,CatchUp}`；`:1462` `misfire_grace_secs` 默认 60 | 维护任务默认 `Skip` |
| 重试 | `domain.rs:1399` `RetryPolicy` 默认 `max_attempts=1`（不重试）；`:1288` `SCHED_MAX_ATTEMPTS=5`；`:1292` `SCHED_MAX_DELAY_SECS=600` | 维护任务默认不重试 |
| 默认启用 | `domain.rs:1451` `enabled` 默认 **false**（裁定 R-A6-1） | 图谱维护任务默认 `enabled=false`，须显式开启 |
| 判重真相源 | `domain.rs:1471` `last_fired_at`（计划触发时刻，非完成）；`task-runs.json` 仅历史不参与判重 | 维护任务判重同 A6 口径 |
| 容量 | `domain.rs:1282` `SCHED_MAX_TASKS=200`；`:1284` `SCHED_MAX_HISTORY=500`；`:1286` `SCHED_MAX_SLOT_SCAN=512` | 维护任务计入总任务上限 |
| 参数 | `domain.rs:1297` `TASK_MAX_PARAMS=20`；`:1449` `params` 不得含 `secret=true` 参数（定义期拒绝 `TASK_SECRET_PARAM_FORBIDDEN`） | 维护任务参数须非 secret |
| 运行记录 | `domain.rs:1497` `TaskRunRecord{run_id, trigger, scheduled_at, attempt, status, exit_code, error_code}` | join `script-runs.json` 同 `run_id` |
| 自动禁用 | `scheduler.rs:800` `record_auto_disabled(app, task_id, error_code)` | 维护任务连续失败→自动 disable，不阻塞主功能 |
| 时钟 | `scheduler.rs:48` `Clock` trait（injectable `now_utc`/`sleep_until_or_stop`）；`:37` `TICK_SECS=1` | 测试可注入时钟验证周期 |
| 审计 | `src-tauri/src/workspace.rs:386` `log_audit(app, action, detail)`；tick 内禁写审计（坑位⑤） | 维护任务审计低频 |

---

## 1. 三层触发模型（与 M5-8 §3 对齐）

| 触发 | 机制 | 依赖的 M4 产物 |
|---|---|---|
| 实时增量 | 上游写后发 `GraphEvent`（app 内事件，不经 scheduler） | A4 写路径钩子（图模块自行消费 `graph_event`） |
| 周期校验/重建 | 注册内部维护任务（interval≥60s），周期执行 `graph_rebuild`/`graph_verify`/`graph_prune` | A7 `task_add` + `script_runner`（复用，无第二执行路径） |
| 溯源补采 | `task-runs.json` 历史 → `Task→Script/Artifact` 边 | A7 运行记录 |

---

## 2. 硬约束（A7 契约不可漂移）

1. **执行体只能是 Script/Command**：`TaskKind` 仅有 `Script|Command`（`domain.rs:1317`）。图谱维护必须落地为 `workspace/scripts/` 维护脚本或 `snippets` 命令片段；**严禁**为图谱新增 `TaskKind::Builtin` 或 `TaskKind::GraphOp`（会破坏 A6 冻结的枚举穷尽）。
2. **无第二执行路径**：`fire()` 已复用 `script_runner::start_run/start_command`，图谱维护任务**天然**走同通道，不新开进程/线程执行。
3. **参数非 secret**：维护任务 `params` 不得含 `secret=true` 值（定义期拒绝 `TASK_SECRET_PARAM_FORBIDDEN`）。
4. **默认 `enabled=false`**：新建维护任务 `enabled` 缺省 false（R-A6-1），需用户显式开启。

---

## 3. 内部维护任务注册参数草案

> 以下为 M5-8 实施期调用 `task_add` 的参数建议（非命令，是图模块的"自举"任务）。

### 3.1 `graph-verify`（周期校验一致性）

```json
{
  "name": "graph-consistency-verify",
  "kind": "command",
  "target_id": "<snippet:graph_verify>",
  "params": {},
  "enabled": false,
  "trigger": { "interval": { "every_secs": 3600 } },
  "missed_run_policy": "skip",
  "retry": { "max_attempts": 1 },
  "timeout_secs": 120
}
```

### 3.2 `graph-rebuild`（周期重建/重放事件）

```json
{
  "name": "graph-rebuild",
  "kind": "command",
  "target_id": "<snippet:graph_rebuild>",
  "params": {},
  "enabled": false,
  "trigger": { "cron": { "expr": "17 3 * * *" } },  // 每日 03:17，避开整点
  "missed_run_policy": "skip",
  "retry": { "max_attempts": 1 },
  "timeout_secs": 300
}
```

### 3.3 `graph-prune`（周期裁剪过期事件）

```json
{
  "name": "graph-prune",
  "kind": "command",
  "target_id": "<snippet:graph_prune>",
  "params": {},
  "enabled": false,
  "trigger": { "interval": { "every_secs": 86400 } },
  "missed_run_policy": "skip",
  "retry": { "max_attempts": 1 },
  "timeout_secs": 120
}
```

> `target_id` 指向的内置 snippet 由 A17 在 `snippets.rs` 种子（或图模块自带维护脚本）；其 `enabled` 须 false（与任务口径一致）。**不**写任何凭据到 params（K3）。

---

## 4. 复用规则（与 A6 冻结口径一致）

- 判重/触发以 `tasks.json` 为准；图谱任务同样 `enabled=false` 默认 + `last_fired_at` 判重。
- 非重叠：单任务单实例（超长 rebuild 不能被下一 tick 顶掉），复用 InFlight（`scheduler.rs:217`）。
- 审计低频：rebuild 开始/结束、prune 记 `audit.json`；**tick 内禁写审计**（坑位⑤）；单次 `graph_query` 不记审计（见 contract 文档 §9）。
- 退出：scheduler 停 → 图谱维护任务随 `stop-scheduler` 收口（`stop-scheduler` 索引 1，先于 `kill-running-scripts`）。

---

## 5. 失败/降级

- 维护任务执行失败：经 `RunError::SpawnFailed`/`UnsupportedPlatform` → `Retryable`，但默认 `max_attempts=1` 不重试；连续失败由 `record_auto_disabled` 自动禁用，**不阻塞**主应用与图谱查询（只读路径独立）。
- rebuild 中应用退出：随 `stop-scheduler` 收口；重启后 `graph_meta.last_event_seq` 游标保证增量重放幂等（contract 文档 §5）。

---

## 6. 图谱自身关闭序（G-D6，交 A0 裁定）

`scheduler.rs` 的 `stop-scheduler`（索引 1）只置停止位 + 通知条件变量，**不 join 线程、不取消在飞**（M0-2.b）。在飞维护任务由 `kill-running-scripts` 收口。

图谱模块自身的 flush（待写 `graph.db` 脏页）须注册在 `ShutdownCoordinator`：
- **建议**：注册在 `flush-sessions` 之后、`close-tabs` 之前（沿用 M1-9 顺序口吻——先确定数据落盘，再销毁 webview）；命名为 `flush-graph`。
- **理由**：图谱维护任务是被 `stop-scheduler` 起的子进程脚本，已随 `kill-running-scripts` 收口；图谱自身的索引写盘应在 webview 销毁前完成。
- **边界**：若图谱在上一 tick 的 rebuild 写出尚未 flush，需在 `flush-graph` 显式 `conn.close()`（rusqlite Drop 已关闭，但应在审计里记 `graph_flush`）。

G-D6 交 A0 在 M5-8 实施期与 A7 协调注册序，并补 single test 断言 `flush-graph` 在 `kill-running-scripts` 之后（仿 `stop-scheduler` 索引断言）。

---

## 7. 测试清单（供 A17 实施期）

- `T-graph-feed1`：`task_add` 维护任务 `enabled=false` 默认（与 R-A6-1 一致）。
- `T-graph-feed2`：维护任务 interval=60 边界合法，interval<60 拒绝 `TASK_INTERVAL_OUT_OF_RANGE`。
- `T-graph-feed3`：维护任务 params 含 `secret=true` → 定义期拒绝 `TASK_SECRET_PARAM_FORBIDDEN`。
- `T-graph-feed4`：超长 rebuild 运行期间，下一 tick 同任务 `reentrant` 跳过（非重叠）。
- `T-graph-feed5`：`stop-scheduler` 触发后，在飞维护任务随 `kill-running-scripts` 收口（不残留进程）。
- `T-graph-feed6`：`flush-graph` 注册序在 `kill-running-scripts` 之后（G-D6）。
- `T-graph-feed7`：维护任务连续失败 → 自动 `disabled`，主应用与 `graph_query` 仍可服务。

---

## 8. FORBID 遵守记录

- 未写产品代码；未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh` 及三份主文档。
- 未移动 `NEXT`。
- 未提交、未 push；与工作树中 A7/A3/A6 未提交产物无交集。
- 「已冻结」陈述以 file:line 标注；「提案/开放项」明确区分。
