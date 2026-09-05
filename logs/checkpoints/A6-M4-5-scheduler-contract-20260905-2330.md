# A6 · M4-5 调度核心契约冻结（TaskDef / 时钟 / 错过执行 / 取消）

> 冻结者：Lane A6（CodeBuddy 会话，Hy4 / 腾讯混元）
> 依据：`PARALLEL_COMMAND_BOARD.md`（2026-09-05 23:00 版，含 Dispatch Waves）
> 开工时间：2026-09-05 23:30 CST
> 基线提交：`f376346`（board 指定 Wave 1 基线）；开工时实际 HEAD = `3544c09`（`docs: add dispatch waves for parallel lanes`），分支 `master`，与 `origin/master` 同步
> Wave 判定：**Wave 1（Start Now）** —— board §Dispatch Waves 明确列出 A6，故直接执行契约冻结
> 性质：**纯契约冻结，未改任何产品代码**（`src/`、`src-tauri/`、`scripts/`、`package*.json` 一律零改动）
> 状态：`CONTRACT_FROZEN`（覆盖 M4-5.a / M4-5.b / M4-5.c），NEXT = `M4-5.d`（需 A0 先裁决 scope，见 §8）与 `M4-6.a`（Lane A7）

---

## 0. Lane 边界与自约束

| 项 | 值 |
|---|---|
| LANE | A6 |
| 职责 | M4-5 调度核心契约（`TaskDef`、时钟语义、错过执行、取消语义） |
| Wave | 1（Start Now，可立即执行） |
| Route | `AI:DEEP / R:high` |
| Allowed Scope（board 原文） | docs、**纯领域类型提案**、policy notes |
| Must Deliver | `TaskDef` contract, clock/missed-run/cancel semantics |
| Merge Order | 2 |
| 禁止 | push；改 Lane 范围外文件；落调度器实现（归 M4-7 / A7）；改产品代码 |

### 0.1 两处主动收窄（超出收窄范围即越权，须 A0 裁决）

1. **不动三份主文档**（`详细设计与实施计划.md` / `后续需求TODO.md` / `AI-模型切换与接手清单.md`）。
   实测开工时 `git status --short --branch` 显示 ` M 详细设计与实施计划.md`（**A1 的展开卡改动已入暂存区但尚未提交**）。
   该文件在 board §File Conflict Map 中属高冲突文件，且 A1（Merge Order 1）排在 A6（Merge Order 2）之前；board 规则为「同一高冲突文件由合并序靠前的 lane 先落地，后者在 A0 集成后 rebase/reapply」。
   → A6 只写本检查点；三份主文档的回写文案在 **§11 以可直接粘贴的成品形式给出**，由 A0 在 A1 落地后套用。
2. **不落 `scripts/check-scheduler-policy.py`、不改 `scripts/pre-merge.sh`**（即 M4-5.d 的实现部分）。
   board 对 A6 的 Allowed Scope 写的是「docs, pure domain type proposal, **policy notes**」，未列 `scripts/`；`scripts/pre-merge.sh` 亦在 File Conflict Map 内。
   → M4-5.d 的**码位设计以 policy notes 形式冻结在 §8**，脚本落地与 pre-merge 接入须 A0 明确授权（建议授权给 A7 或单独开卡，见 §8.3）。

### 0.2 与 A1 展开卡（M4-20260905-2225.md）的关系

A1 的 M4-5 卡是**编号/范围/顺序/红线**的冻结，明确「不做技术选型裁决，选型一律留给 A2/A6 在其契约卡内裁定并写明理由」。
本卡即为 M4-5.a/b/c 的**技术裁定书**，逐条回应 A1 的 F-1/F-4/F-5/F-8/F-9/F-10 与硬停止条件。

---

## 1. 现状实测（本卡现场取证，HEAD `3544c09`，共 18 项）

> 与 `logs/assist/scheduled-task-taskcard-20260902-1146.md`（2026-09-02，`AI:FAST` 预研）的多处结论**已过时**，修正见 §2。

| # | 实测项 | 命令/位置 | 结果 |
|---|---|---|---|
| 1 | 执行通道入口 | `src-tauri/src/script_runner.rs` | `start_run(table, meta, script_abs_path, values, roots, home_dir, app, records_file) -> Result<String, RunError>`；`start_command(table, snippet, values, roots, home_dir, app, records_file) -> Result<String, RunError>`；返回 `run_id` |
| 2 | 进程表 | 同上 `ScriptProcessTable` | `running_count()` / `has_running_script(script_id)` / `snapshot(run_id) -> Option<RunSnapshot>` / `cancel(run_id) -> Result<(), RunError>`；实例在 `AppState.script_runs: Arc<ScriptProcessTable>` |
| 3 | 互斥键的**真实粒度** | `start_argv_run(table, &meta.id, …)`、`start_command` 传 `&snippet.id` | 并发互斥键是 **script_id / snippet_id**，**不是 task_id**；`RunError::AlreadyRunning{script_id}` |
| 4 | 运行期错误码 | `RunError::code()` | `SCRIPT_ALREADY_RUNNING` / `TOO_MANY_RUNS` / `TIMEOUT_TOO_LARGE` / `SCRIPT_PATH_REJECTED` / `CWD_REJECTED` / `UNSUPPORTED_PLATFORM` / `SPAWN_FAILED` / `UNKNOWN_RUN` + `ScriptError` 的定义期码 |
| 5 | 状态枚举 | `domain.rs` `RunStatus` | `Running` / `Succeeded` / `Failed` / `Cancelled` / `Timeout`，`is_terminal()`；**终态不可互转** |
| 6 | 既有上限 | `script_runner.rs` 常量 | `DEFAULT_TIMEOUT_SECS=60`、`MAX_TIMEOUT_SECS=600`、`HARD_GRACE_SECS=5`、`CANCEL_GRACE_SECS=5`、`MAX_CONCURRENT_RUNS=8`、`MAX_TABLE_ENTRIES=200`、`MAX_RUN_RECORDS=200`、`SCRIPT_RUN_TAIL_BYTES=256KiB` |
| 7 | 运行记录落盘 | `save_run_records` / `persist_run_record` | 自动写 `data_dir/script-runs.json`，原子写，FIFO 保留 200 条 |
| 8 | 原子写原语 | `session.rs:30` `atomic_write` | `create_dir_all(parent)` + `.json.tmp` 写 + `rename`；被 `save_scripts_at` / `save_snippets_at` / `save_run_records` 复用 |
| 9 | 持久化文件位 | `workspace.rs` | `data_dir(app)` 下 `scripts.json` / `snippets.json` / `script-runs.json` / `repos.json` / `bookmarks.json` / `audit.json`；**无 YAML、无 tasks.json** |
| 10 | 损坏时行为 | `load_scripts_at` / `load_snippets_at` | `fs::read_to_string().ok().and_then(serde_json::from_str().ok).unwrap_or_default()` → **解析失败即静默丢弃整个文件**（无备份、无日志） |
| 11 | 容量常量 | `scripts.rs` / `snippets.rs` | `MAX_SCRIPTS=200`、`MAX_SNIPPETS=200`、`MAX_ARGV_ELEMENTS=64`、`MAX_NAME_BYTES=128` |
| 12 | 生命周期 | `shutdown.rs` + `bridge.rs:778` `register_shutdown_tasks` | `ShutdownCoordinator` 幂等/失败隔离/**不阻塞重入**；已注册 6 个任务，顺序 `stop-background-workers → flush-sessions → close-tabs → kill-terminals → shutdown-grid → kill-running-scripts` |
| 13 | 全局停止位 | `AppState.shutdown_requested: Arc<AtomicBool>` | 由 `stop-background-workers`（索引 0）置位，供常驻线程轮询 |
| 14 | 既有常驻线程范式 | `start_layout_enforcer`（400ms）/ `start_hibernation_sweeper`（**60s**）/ `start_grid_load_retry` | 均为 `std::thread::spawn` + `thread::sleep(固定)` + 循环内查 `shutdown_requested`。**60s 固定 sleep 意味着停止信号最长 60s 才被观测** |
| 15 | 审计 | `workspace::log_audit` | `Vec<AuditEntry>`，`>1000` 时 `drain(0..len-1000)`；`fs::write` 非原子；既有事件名 `script.run.start` / `script.run.cancel` / `script.run.status` / `script.runs.list` / `script.validate.reject` / `cmd.run.start` / `cmd.validate.reject` … |
| 16 | 审计脱敏单测 | `bridge.rs` 测试（source 扫描） | 断言审计格式串窗口内**不得出现** `values` / `argv` / `{PATTERN}`；新增格式串必须同样干净 |
| 17 | 依赖现状 | `Cargo.toml` / `Cargo.lock` | `chrono 0.4(serde)` / `uuid 1(v4)` / `serde` / `serde_json` 齐备；`tokio` **仅传递依赖（不可 `use`）**；`chrono-tz`、`cron`、`tokio-cron-scheduler` **在 Cargo.lock 中零命中** |
| 18 | 调度相关现状 | 全仓 grep `scheduler|TaskDef|tasks.json|task_run_now` | `src-tauri/**` **零命中**；`src/**` 仅 `useBrowserHost.ts` / `useBrowserStore.ts` 命中无关词（`task_list` 等前端局部命名）。ACL 末条为 `list_artifact_images`（第 110 行），共 **108** 条条目 |

---

## 2. 对既有草案与需求原文的事实修正（7 条）

> 主要修正对象：`logs/assist/scheduled-task-taskcard-20260902-1146.md`（下文称「旧草案」）。未修正即施工 = 按过时前提写代码（同 M2-3 / M2-5 / M3 的三次事实修正性质）。

| 编号 | 旧口径 | 实测 | 本卡裁决 |
|---|---|---|---|
| **F-A6-1** | 旧草案 §2「`ShutdownCoordinator` 无 → 阻塞」；「`run_script` 未实现 → 阻塞」 | 二者均已交付（实测 1/12），旧草案的两条硬阻塞**已解除** | M4-5 契约按「依赖已就绪」冻结；M4-7 只做接入，不得重造 |
| **F-A6-2** | 旧草案 §4.1 `timezone: String // IANA，如 Asia/Shanghai` | `chrono-tz` 在 `Cargo.lock` **零命中**；无 IANA 求值能力 | **`TaskDef` 不引入 `timezone` 字段**。cron 按 `chrono::Local`（本机时区）解释，存储与比较一律 UTC。理由：无依赖支撑的字段即 no-op 误导字段（同 `ScriptParam.raw`，见 `M2-4.b-VERDICT §3.1` 口径）。若将来要支持任意时区，须先引入 `chrono-tz` 并单开卡 |
| **F-A6-3** | 旧草案 §4.4「cron 语法：仅 5 段」但未交代与种子工具的关系 | `src-tauri/src/tools/cron-tool.html:627` 明确支持 **6 位（秒 分 时 日 月 周）或 5 位（分 时 日 月 周）** | 后端**只接受 5 段**；`cron-tool.html` 的 6 位模式与后端不兼容 → 登记 **O-A6-4**，由 M4-8 UI 显式标注/禁用 |
| **F-A6-4** | 旧草案 §4.3 幂等键 = `<task_id>:<计划触发时刻>`，**存 `HashSet` + `task-runs.json` 判重** | `task-runs.json` 若有容量上限（旧草案自己定 500 条环形裁剪），**裁剪后旧键丢失 = 重启后重复执行** | **判重真相源改为 `tasks.json` 内的 `last_fired_at`**（与 `next_run_at` 同一次原子写）；`task-runs.json` 只做 UI 历史与排障，**不参与判重**。同时**取消 `idempotency_key` 字段**（无消费者即不定义，同 `raw` 口径） |
| **F-A6-5** | 旧草案 §4.4「tick = 1 秒；sleep(1s)」、§4.6「CatchUp 上限 3（硬编码）」 | 既有 `start_hibernation_sweeper` 用固定 60s sleep，停止信号最长 60s 才被观测；A1 的 F9 要求错过执行策略**落为字段**不能靠常量 | tick **可中断等待**（stop 延迟 ≤ 1s，见 §4.3）；`catch_up_limit` 成为 `TaskDef` 字段（默认 3，硬上限 10） |
| **F-A6-6** | 需求 #11 原文「调度表达式（cron 或 **every Ns**/min/h）」 | 无约束的秒级周期 × 休眠/错过补偿 = 触发风暴（关机 1 小时 → 数百次补跑） | **`Interval.every_secs` 下界 60**（`SCHED_MIN_INTERVAL_SECS`），上界 30 天。这是对需求原文的**保守收窄**，可回退，放宽须单开卡（**O-A6-5**） |
| **F-A6-7** | 旧草案 §5 列 7 条 `task_*` 命令（含 `task_cancel` / `task_history`） | A1 展开卡（Merge Order 1，先于本卡）已冻结为 **5 条**：`task_list` / `task_add` / `task_update` / `task_remove` / `task_run_now` | 本卡 **沿用 A1 的 5 条命名**，`task_cancel` / `task_history` **不新增**（取消复用既有 `cancel_script(run_id)`；历史读 `task-runs.json` 由 `task_list` 附带返回）。A6 不改动 A1 已冻结的编号与命名 |

---

## 3. M4-5.a `TaskDef` 契约冻结

> 全部为**纯领域类型提案**；由 A7 在 M4-6.a 落地到 `domain.rs` / `tasks.rs`。A6 不写产品代码。

### 3.1 类型提案（Rust，落 `src-tauri/src/domain.rs`）

```rust
/// 触发方式（首期两种，均 ≥ 1 分钟粒度，见 F-A6-6）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskTrigger {
    /// 标准 5 段 cron：`分 时 日 月 周`（方言见 §4.4）。
    Cron { expr: String },
    /// 固定间隔（秒）。合法区间 [60, 30*24*3600]。
    Interval { every_secs: u64 },
}

/// 执行体类型。F-5：首期仅 Script / Command，**不做 Tool**（工具无 headless 执行入口）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind { Script, Command }

/// 错过执行策略。F9 硬性要求：必须是 `TaskDef` 字段，不得退化为运行时常量。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    /// 默认：错过的触发点不补跑，`next_run_at` 直接推到当前之后的第一个未来触发点。
    Skip,
    /// 补跑一次（用**最新**那个错过的触发点），不论错过了多少个。
    RunOnce,
    /// 按 `catch_up_limit` 上限补跑，超出部分记 `over_limit` 后丢弃。
    CatchUp,
}

/// 触发来源（写 `task-runs.json`，用于 UI 区分「定时 / 手工 / 补跑 / 重试」）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunTrigger { Scheduled, Manual, CatchUp, Retry }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetryBackoff { Fixed, Exponential }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetryPolicy {
    /// 含首次；1 = 不重试（默认）。硬上限 5。
    pub max_attempts: u32,
    pub backoff: RetryBackoff,
    pub base_delay_secs: u64,
    pub max_delay_secs: u64,
}

/// 定时任务定义（持久化 `data_dir/tasks.json`，`Vec<TaskDef>`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskDef {
    /// uuid v4
    pub id: String,
    /// 显示名，≤ 128 字节（对齐 `scripts::MAX_NAME_BYTES`）
    pub name: String,
    pub kind: TaskKind,
    /// 目标 id：`ScriptMeta.id`（Script）或 `CommandSnippet.id`（Command）。创建时校验存在性。
    pub target_id: String,
    /// 参数值。**不得包含** `ScriptParam.secret == true` 的参数值（定义期拒绝，见 §3.4 R-3）。
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
    /// 默认 **false**（裁定 R-A6-1，见 §3.5）；存量文件缺该字段时同样按 false 处理
    #[serde(default = "default_task_enabled")]
    pub enabled: bool,
    pub trigger: TaskTrigger,
    /// 错过执行策略，默认 `Skip`
    #[serde(default)]
    pub missed_run_policy: MissedRunPolicy,
    /// `CatchUp` 补跑上限，默认 3，硬上限 10
    #[serde(default = "default_catch_up_limit")]
    pub catch_up_limit: u32,
    /// 迟到超过该秒数才判定为「错过」，默认 60
    #[serde(default = "default_misfire_grace_secs")]
    pub misfire_grace_secs: u64,
    /// 重试策略，默认「不重试」
    #[serde(default)]
    pub retry: RetryPolicy,
    /// 0 = 沿用 `script_runner::DEFAULT_TIMEOUT_SECS`(60)；上限 600
    #[serde(default)]
    pub timeout_secs: u32,
    /// **判重真相源**：上一次「已触发」的计划触发时刻（不是完成时刻）
    #[serde(default)]
    pub last_fired_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 下一次计划触发时刻（永远指向未来）；由调度器计算并落盘
    #[serde(default)]
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 定时任务运行记录（持久化 `data_dir/task-runs.json`，**仅历史与排障，不参与判重**，见 F-A6-4）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskRunRecord {
    pub task_id: String,
    /// 与 `ScriptRunRecord.run_id` 同值（join key；输出尾存仍在 `script-runs.json`）
    pub run_id: String,
    pub trigger: TaskRunTrigger,
    /// 计划触发时刻；重试沿用同一值，只递增 `attempt`
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
    pub attempt: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    /// 稳定错误码（如 `SCRIPT_ALREADY_RUNNING`、`TASK_TARGET_NOT_FOUND`）
    pub error_code: Option<String>,
}
```

### 3.2 字段语义裁决（易错点逐条定死）

| 字段 | 语义 | 为什么这样定 |
|---|---|---|
| `last_fired_at` | **已触发**的计划时刻，不是「完成时刻」 | 崩溃恢复判重看的是「这个触发点是否已经发过」，不是「是否跑完」。用完成时刻会在「触发后立刻崩溃」时重复执行 |
| `next_run_at` | 永远 **> now**；缺失或与 `last_fired_at` 冲突时由调度器重算 | 防止 UI 展示一个过去时间；同时让「启动时扫描」逻辑唯一 |
| `last_fired_at` 与 `next_run_at` | **同一次 `atomic_write`** 落盘 | 二者不一致 = 重复执行或漏执行（实测 `session::atomic_write` 为 tmp+rename，单次调用即原子） |
| `catch_up_limit` | 仅 `CatchUp` 策略下生效；`Skip`/`RunOnce` 忽略 | 避免「字段看似生效实则 no-op」；UI 须按策略置灰 |
| `timeout_secs` | 0 = 60（与 `ScriptMeta.timeout_secs` 同口径），上限 600 | 直接复用 `script_runner::effective_timeout` 的既有语义，不新造一套 |
| `params` | 只存**非 secret** 参数值 | M4 红线：凭据不得进普通文件（见 §3.4 R-3） |
| `retry` | 默认 `max_attempts = 1`（不重试） | 定时任务的默认行为必须是「跑一次就完」，重试是显式选择 |

### 3.3 定义期校验规则（落 `tasks.rs` 纯函数层，沿用 `snippets.rs` / `scripts.rs` 范式）

| 规则 | 判定 | 错误码 |
|---|---|---|
| R-1 | `id` 为 uuid v4；`name` 非空且 ≤ 128 字节；`target_id` 非空且存在（Script 查 `scripts.json`，Command 查 `snippets.json`） | `TASK_NAME_TOO_LONG` / `TASK_TARGET_NOT_FOUND` |
| R-2 | `trigger` 合法（cron 5 段且字段在值域内 / interval ∈ [60, 2592000]） | `TASK_INVALID_CRON` / `TASK_INTERVAL_OUT_OF_RANGE` |
| R-3 | `params` 中**禁止**出现目标 `ScriptParam.secret == true` 的参数值；若目标存在必填 secret 参数，则该任务**直接拒绝创建** | `TASK_SECRET_PARAM_FORBIDDEN` |
| R-4 | `params` 键集必须与目标参数名一致（缺必填 / 多未知键均拒绝），值过 `security_policy::check_text_field` | `TASK_PARAM_INVALID` |
| R-5 | `catch_up_limit ∈ [1, 10]`；`timeout_secs ∈ [0, 600]`；`retry.max_attempts ∈ [1, 5]`；`retry.max_delay_secs ≤ 600` | `TASK_PARAM_INVALID` |
| R-6 | 任务总数 ≤ `SCHED_MAX_TASKS = 200`（对齐 `MAX_SCRIPTS` / `MAX_SNIPPETS`）；`params` 条目 ≤ `MAX_PARAMS = 20` | `TASK_LIMIT_REACHED` |
| R-7 | Command 类任务若目标片段 `dangerous == true`，创建时允许但**必须**带 `dangerous` 透传标记，供 M4-8 UI 二次确认 | —（UI 职责，字段复用既有 `dangerous`） |

> 错误码为**稳定字符串**（沿用 `scripts.rs` 20 码范式），首期 10 个：`TASK_NOT_FOUND` / `TASK_INVALID_CRON` / `TASK_INTERVAL_OUT_OF_RANGE` / `TASK_TARGET_NOT_FOUND` / `TASK_SECRET_PARAM_FORBIDDEN` / `TASK_PARAM_INVALID` / `TASK_LIMIT_REACHED` / `TASK_ALREADY_RUNNING` / `TASK_PERSIST_FAILED` / `TASK_NAME_TOO_LONG`。

### 3.4 持久化（F-4 / F-10）

| 项 | 冻结值 |
|---|---|
| 文件 | `data_dir(app)/tasks.json`（`Vec<TaskDef>`）；历史 `data_dir(app)/task-runs.json`（`Vec<TaskRunRecord>`，环形保留 **500** 条） |
| 格式 | **JSON**（不引入 YAML —— 实测全仓持久化均为 JSON，F-4 已裁） |
| 写 | `session::atomic_write`（tmp + rename），**与既有 scripts/snippets 同一原语**，不新造第二条原子写路径 |
| 新增字段 | 一律 `#[serde(default)]`。理由：实测 `load_scripts_at` 用 `unwrap_or_default()`，缺 `default` 会让缺字段的历史文件被**整份静默丢弃**（`domain.rs` 顶部注释已记此坑） |
| 容量 | `SCHED_MAX_TASKS = 200`；超出拒绝新增（与 `MAX_SCRIPTS` 同为拒绝，不是静默裁剪） |
| 损坏 | A1 F10 定的是「损坏跳过」。本卡**追加要求**：A7 必须把损坏文件 `rename` 为 `tasks.json.corrupt` 并 `eprintln!`，不得静默清空（实测既有 `load_scripts_at` 就是静默丢弃，等于定时任务全丢且无痕）→ 登记 **O-A6-7** |
| 删除 | 幂等（`task_remove` 对不存在 id 返回 `TASK_NOT_FOUND` 而非崩溃） |

### 3.5 裁定 R-A6-1：新建任务默认 `enabled = false`

> **冲突来源**：A1 展开卡未提；A10 独立安全评审 `logs/assist/A10-M4-security-review-20260905-2240.md` §G-7.5 建议「新建任务默认 `enabled=false`，启用为显式动作」；A7 预研 `logs/assist/A7-M4-6-7-scheduler-backend-prework-20260905-2315.md` §3 **C-4** 点名要求「A6 在 `TaskDef` 冻结里明确 `enabled` 默认值与 `#[serde(default)]` 语义」。
> 既有领域范式实测：`ScriptMeta.enabled` 与 `CommandSnippet.enabled` 均 `#[serde(default = "default_script_enabled")]` 且该函数返回 **true**（`domain.rs:682`）。

**裁定：采纳更严口径 —— `TaskDef.enabled` 默认 `false`（`default_task_enabled() -> false`）。**

| 理由 | 说明 |
|---|---|
| R1 风险量级不对称（最高权重） | 脚本/片段的 `enabled` 只决定「是否出现在可运行列表」，仍需人工点击才会执行；定时任务的 `enabled` 决定「是否**无人值守自动执行**」，且可能执行 `dangerous == true` 的命令片段（实测 `CommandSnippet.dangerous`）。二者同名不同险，不能照搬默认 |
| R2 fail-closed 同源 | A1 F1 冻结「数据库写默认拒绝」，本卡冻结「自动执行默认关闭」，同属 M4 护栏的最小权限口径 |
| R3 独立评审优先 | A10 是独立于实现链的安全评审 lane；本项目历史对安全侧从严口径一贯采纳（如 `M2-6-fix1` P1-2/P1-3） |
| R4 代价可忽略 | 用户创建后需点一次「启用」；M4-8 可提供「创建并启用」显式二次动作 |

**配套语义（必须同时落地，否则裁定落空）**

1. `default_task_enabled()` 返回 `false`；**存量 `tasks.json` 缺 `enabled` 字段时同样按 `false`**（即「缺失 = 不自动执行」，fail-closed 方向正确）。
2. **创建时即计算并持久化 `next_run_at`**（但不触发），保证 UI 能展示「下次执行时间」；`enabled=false` 期间不扫描、不补跑、不改 `last_fired_at`。
3. 重新启用时 `next_run_at` 从 `now` 重算，**不补启用期间错过的触发点**（§5.1 不变量 4）。
4. UI（M4-8）必须以显式开关承载该语义，且默认态为关；禁止「创建即静默启用」。
5. 机器可检：Rust 单测断言 `TaskDef::default_enabled_is_false()`；`SCHED_ENABLED_DEFAULT_TRUE` 码位（见 §8.1 追加第 9 条）。

> **一致性说明**：本裁定**不推翻**既有 `ScriptMeta.enabled` / `CommandSnippet.enabled` 默认 true —— 那是「列表可见性」，与「自动执行」不同险，A6 不越权改动 M2 既有契约。

---

## 4. M4-5.b 时钟语义与可注入 `Clock`

### 4.1 `Clock` 契约（F8）

```rust
/// 可注入时钟。**调度判定逻辑只经本 trait 取时间。**
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> chrono::DateTime<chrono::Utc>;
    /// 可中断等待。返回 true = 到期；false = 被停止信号唤醒（应立即退出循环）。
    /// `cap`：单次等待硬上界，实现必须把 `deadline - now` 截断到 `cap` 以内。
    fn sleep_until_or_stop(&self, deadline: std::time::Instant, stop: &AtomicBool, cap: Duration) -> bool;
}
pub struct SystemClock;   // 唯一允许读系统时钟的实现
```

**机器可检方式**（`SCHED_CLOCK_NOT_INJECTABLE`）：`scheduler.rs` 与 `tasks.rs` 中的**判定函数**（`next_fire_after` / `collect_missed_slots` / `is_missed` / `should_fire` / `validate_*`）函数体内不得出现 `Utc::now()` / `Local::now()` / `Instant::now()` / `SystemTime::now()`；允许出现的位置仅限 `SystemClock` 实现与调度主循环。

### 4.2 时间基准与时区（F-A6-2）

1. 内部计算与持久化一律 `DateTime<Utc>`（RFC 3339，与 `ScriptMeta.created_at` 同口径）。
2. cron 的「分/时/日/月/周」按 **`chrono::Local` 的本机本地时区**解释（实测 `chrono 0.4` 默认带 `clock` feature，`Local` 可用）。
3. **不引入 `timezone` 字段**：`chrono-tz` 在 `Cargo.lock` 零命中，无法按 IANA 求值；加字段即 no-op 误导字段。
4. `Interval` 不受时区/DST 影响（纯 UTC 秒数累加）。

### 4.3 等待与停止语义

| 项 | 冻结值 | 依据 |
|---|---|---|
| 单次等待上界 `cap` | **1 秒** | 既有 `start_hibernation_sweeper` 睡 60s，停止信号最长 60s 才被观测（实测 14）；调度器不得沿用固定长 sleep |
| 可中断性 | 必须（`Condvar::wait_timeout`，或 A2 若选 tokio 则用 `tokio::time::sleep` + 取消 token） | 停止延迟上界 ≤ 1s |
| 实现形态 | **已确定**：A2 在 `logs/checkpoints/M4-1.a-20260905-2245.md §3` 裁定 **F-1 选 (b)** —— 应用层保持 `std::thread` + 可中断等待（`Condvar` / `AtomicBool`），**不把 `tokio` 提升为直接依赖、不自建 async runtime**。调度器必须按该形态实现（`Condvar::wait_timeout` + stop `AtomicBool`），**不得引 tokio runtime** | A2 裁定（M4-1.a §3，理由 R1/R4：全仓零 async 业务代码 + M0-4 依赖清理口径）；board 明令 A7「不得绕过 M4-1.a 关于 tokio 的裁定」 |
| 精度 | cron 最小粒度 1 分钟，不做秒级对齐；实际触发允许 ≤ 1s 漂移 | 与 `cap=1s` 自洽，避免为亚秒精度做无收益优化 |

### 4.4 cron 方言（F-A6-3）

| 项 | 冻结值 |
|---|---|
| 段数 | **5 段**：`分 时 日 月 周` |
| 值域 | 分 0–59 / 时 0–23 / 日 1–31 / 月 1–12 / 周 0–7（0 与 7 均为周日） |
| 语法 | `*`、`*/n`、`a-b`、`a,b`、字面量 |
| **不支持** | 秒字段、年字段、`@reboot` / `@daily` / `@yearly` 等宏、`L` / `W` / `#` / `?`、步长嵌套 |
| 非法输入 | **定义期拒绝**（`TASK_INVALID_CRON`），**不得静默降级/截断/按默认值解析** |
| 解析实现 | **自研纯函数**（约 150–200 行，落 `tasks.rs`，零新依赖）。不引入 `cron` crate：M0-4 依赖清理口径 + 该 crate 的语义面大于本项目所需 |
| 与种子工具的关系 | `cron-tool.html`（M2-7）同时支持 6 位（秒 分 时 日 月 周）与 5 位；**后端只接受 5 位** → **O-A6-4**，M4-8 UI 须显式标注并禁用 6 位模式 |

### 4.5 系统时间变化行为

| 场景 | 判定 | 行为 |
|---|---|---|
| **回拨**（`now < last_fired_at`，NTP 校正/手动改表） | `now` 比已触发时刻还早 | **不追补、不倒退**：`last_fired_at` 保持不变，`next_run_at` 从 `now` 重新计算，写审计 `task.clock.rewind`，本 tick 不触发 |
| **前跳**（`now` 远晚于 `next_run_at`） | 走错过执行流程（§5.2），受 `catch_up_limit` 约束 | 见 §5.2 |
| **DST 前进**（本地时刻不存在，如 02:30 被跳过） | 该触发点在本机时区不存在 | 跳过该点，记 `task.run.missed(reason=nonexistent_local_time)`，不补跑 |
| **DST 回拨**（本地 1 小时重复） | 同一本地时刻出现两次 | 只触发第一次（由 `last_fired_at` 单调推进保证），第二次判为已触发 |
| **休眠/挂起** | 唤醒后 `now - next_run_at` 可能达数小时 | 同「前跳」，走错过执行；**不做**「按挂起时长折算」的特殊逻辑 |

---

## 5. M4-5.c 错过执行 / 重入 / 取消语义

### 5.1 触发判定主流程（每次 tick 与启动时各执行一次）

```
anchor = max(last_fired_at, created_at)          # 无 last_fired_at 时用 created_at，避免「建任务即补跑历史」
slots  = 生成 (anchor, now] 内的全部计划触发点，扫描上限 SCHED_MAX_SLOT_SCAN = 512
若 slots 为空：跳到「推进 next_run_at」

fired = 0
for slot in slots:
    if stop 信号: break
    if slot <= last_fired_at: continue            # 已触发过（崩溃恢复判重，真相源 = last_fired_at）
    lateness = now - slot
    if lateness <= misfire_grace_secs:
        触发(slot, trigger=Scheduled)             # 正常触发，不算错过
    else:
        match missed_run_policy:
            Skip    -> 记 task.run.missed(skipped)；不触发
            RunOnce -> 尚未补跑则 触发(最新 slot, trigger=CatchUp)，fired_once = true；其余记 skipped
            CatchUp -> if fired < catch_up_limit { 触发(slot, trigger=CatchUp); fired += 1 }
                       else { 记 task.run.missed(over_limit) }
    每次触发前依次检查：① stop ② 本任务 in_flight ③ 全局并发
    触发成功后：last_fired_at = max(last_fired_at, slot) 并**原子落盘**（与 next_run_at 同一次写）

next_run_at = 严格大于 now 的第一个未来触发点；原子落盘
```

**关键不变量**

1. `last_fired_at` 单调递增（任何路径都不回改，含时钟回拨）。
2. `next_run_at` 永远 > `now`。
3. `last_fired_at` 与 `next_run_at` **同一次 `atomic_write`**（崩溃恢复不重复、不漏）。
4. `enabled == false` 的任务：不扫描、不补跑、不改 `last_fired_at`；重新启用时 `next_run_at` 从 `now` 重算（**不补启用期间错过的**）。

### 5.2 跳过 / 失败分类（决定「是否消耗重试配额」）

| 触发前/后返回 | 分类 | 行为 | 消耗 retry 配额 |
|---|---|---|---|
| 本任务 `in_flight` 非空 | reentrant（F9：同任务不并发） | 记 `task.run.skipped(reason=reentrant)`，**不排队** | ❌ |
| `RunError::AlreadyRunning` | 目标脚本/片段正被占用（含用户手工执行，或**另一任务引用同一脚本** —— 实测互斥键是 script_id 而非 task_id） | 记 `task.run.skipped(reason=target_busy)` | ❌ |
| `RunError::TooManyRuns` | 达 `MAX_CONCURRENT_RUNS = 8` | 记 `task.run.skipped(reason=global_limit)`，**不排队**（排队会积压，且重试会加剧拥塞） | ❌ |
| `RunError::InvalidParam` / `ScriptPathRejected` / `CwdRejected` / `TimeoutTooLarge` / `TASK_TARGET_NOT_FOUND` | 持久性错误（定义或文件已失效） | 记 `task.run.reject` + **自动 `enabled = false`** + 审计 `task.auto_disabled` | ❌ |
| `SpawnFailed` / `UnsupportedPlatform` | 可重试错误 | 走 `retry`（§5.3） | ✅ |
| 进程退出码非 0 / 超时 | 运行失败 | 走 `retry`（§5.3） | ✅ |
| 用户取消（`Cancelled`） | 显式意图 | **不重试**，终态 `Cancelled` | ❌ |

> 自动禁用（第 4 行）的理由：与 `script.validate.reject` / `cmd.validate.reject` 同口径（M2-6-fix1 P1-3），避免一个失效任务每分钟刷一条同样的失败。

### 5.3 重试语义

```
attempt = 1 → 失败 → 等待 delay → attempt = 2 → … → attempt = max_attempts → 终态 Failed
delay(Exponential) = min(base_delay_secs * 2^(attempt-1), max_delay_secs)
delay(Fixed)       = min(base_delay_secs, max_delay_secs)
```

| 规则 | 冻结值 |
|---|---|
| 默认 | `max_attempts = 1`（不重试） |
| 上限 | `max_attempts ≤ 5`；`max_delay_secs ≤ 600`；重试**总时长**不超过 `timeout_secs * max_attempts + Σdelay`（由 M4-7 落一个硬上界常量并测试） |
| 幂等 | 重试沿用同一 `scheduled_at`，只递增 `attempt`；`trigger = Retry` |
| 抖动 | 加 ±10% 抖动（避免多任务同时重试造成尖峰）—— 由 M4-7 实现，抖动**不得**使 `delay` 超过 `max_delay_secs` |
| 不重试 | `Cancelled`（用户取消）、`Skipped`（§5.2 前三类）、持久性错误 |
| 跨重启 | 重试态持久化在 `task-runs.json`；重启后未完成重试的任务按 `missed_run_policy` 处理（**不**沿用过期的重试计时） |

> **范围裁定（防止 no-op 字段）**：`retry` 是 **M4-7 必做项**，不是可选项 —— M4-8 UI 需求含「失败重试状态」。若 M4-7 无法交付重试，必须**删除 `retry` 字段**，不得保留为「看似可配实则无效」的字段（`ScriptParam.raw` 教训）。

### 5.4 取消语义矩阵

| 操作 | 行为 |
|---|---|
| 取消某次运行 | 复用既有 **`ScriptProcessTable::cancel(run_id)`**（与 `cancel_script` 命令同一入口）；取消宽限 `CANCEL_GRACE_SECS = 5` 后 SIGKILL。**不新增 `task_cancel` 命令**（F-A6-7） |
| 取消后 | 终态 `Cancelled`，**不重试**，写 `script.run.cancel`（既有事件名，沿用） |
| `enabled = false` | 停止后续触发；**in-flight 运行不受影响**（让它跑完） |
| `task_remove` | 先取消该任务的 in-flight 运行（若有），再删持久化条目；`task-runs.json` 中的历史**保留**（审计与排障需要），不随任务删除 |
| `task_update` 改 trigger | 重算 `next_run_at`；in-flight 不受影响 |
| `task_run_now` | 立即触发一次，`trigger = Manual`，`scheduled_at = now`；**不推进** `last_fired_at` / `next_run_at`（不干扰计划）；同样受「同任务 in_flight」与全局并发约束，冲突时返回 `TASK_ALREADY_RUNNING` |
| **应用退出** | `stop-scheduler`（见 §5.5）只停止「触发」，**不取消 in-flight 运行**；in-flight 由既有的 `kill-running-scripts` 收口 —— 避免两条 kill 路径 |

### 5.5 退出收口（F7）

| 项 | 冻结值 |
|---|---|
| 注册位置 | `bridge::register_shutdown_tasks` 中，**紧随 `stop-background-workers` 之后、`flush-sessions` 之前**（插入后索引 = 1） |
| 与 `kill-running-scripts` 的顺序 | 必须满足 `index(stop-scheduler) < index(kill-running-scripts)`（F7）；按上述插入位置，索引 1 < 6 ✅ |
| 任务行为 | 置 stop `AtomicBool` + 通知 `Condvar`；**不 join 线程、不取消 in-flight 运行**（M0-2.b：清理任务不得持业务锁阻塞） |
| 调度线程 | 每轮循环先查 `AppState.shutdown_requested`（索引 0 置位，既有全局停止位）+ 自身 stop 位；**并在调用 `start_run` / `start_command` 之前最后一次查 stop**（TOCTOU 最小化） |
| 现有任务顺序 | **不得改动**既有 6 个任务的相对顺序 |
| 残留风险 | stop 置位与 `kill-running-scripts` 之间存在微秒级窗口，理论上可产生「kill 之后才启动」的孤儿进程 → **O-A6-6**：M4-7.d 必须以真实进程取证（`/proc` 验证进程组消失，<15s，同 M2-4 B7~B12 与 M3.a 口径）；若取证失败，收紧为「`stop-scheduler` 获取调度器状态锁后再置位」 |

---

## 6. 命令 DTO 提案（A7 实现 / A8 消费）

> A6 只提案；命令骨架与 ACL 归 M4-6.b（A7）。命名沿用 A1 已冻结的 5 条，A6 不改名。

| 命令 | 入参 | 返回 | 来源校验 / ACL / 审计 |
|---|---|---|---|
| `task_list` | — | `Vec<TaskDef>`（**不含** secret 参数值）+ 可选最近 `Vec<TaskRunRecord>` | `check_invocation_source(webview, "task_list", None, &app)`；审计 `task.runs.list`（仅 `count`） |
| `task_add` | `TaskDef`（不含 id/时间戳，服务端生成） | `TaskDef` | 同上，`task.add`（`id` / `kind` / `trigger` 摘要，**不含参数值**） |
| `task_update` | `TaskDef`（按 id 全量替换） | `TaskDef` | 同上，`task.update` |
| `task_remove` | `id: String` | `bool`（幂等） | 同上，`task.remove` |
| `task_run_now` | `id: String` | `RunSnapshot`（复用既有类型） | 同上，**`task.run.manual`**（`task_id` / `run_id` / `trigger=manual`；修订 A-1：不得用 `task.run.start`，见 §7） |

**约束（沿用 F5 / M2-4.c 口径）**

1. 五条命令**全部**过 `check_invocation_source`；**远程 webview 一律拒绝**（与 `run_command` / `run_script` 同口径，实测 M2-6-fix1 P1-1 已为 `run_command` 建立该单测范式）。
2. ACL 新条目必须**插在 `list_artifact_images` 之前**（实测其为末条，第 110 行；`check-image-policy.py` 等夹具按末行定位，M2-1 / M2-2.b / M2-3.b 三度踩坑）。新增后条目数 108 → 113。
3. `main.rs` 的 `invoke_handler` 必须注册；否则命令静默不可用（既有坑）。
4. 审计 `detail` 只含 `task_id` / `run_id` / `count` / `status` / `reason` / `error_code`；**不得含**参数值、命令正文、脚本正文、输出。注意 `bridge.rs` 既有测试会扫描审计格式串窗口内是否出现 `values` / `argv` / `{PATTERN}`，新增格式串必须同样干净。
5. **tick 内禁止 `log_audit`**（见 §7）。

---

## 7. 审计与红线

| 项 | 冻结值 |
|---|---|
| 事件名（低频、动作级，进 `audit.json`） | `task.add` / `task.update` / `task.remove` / `task.enable` / **`task.run.manual`** / `task.run.skipped` / `task.run.missed` / `task.run.reject` / `task.auto_disabled` / `task.clock.rewind` / `task.runs.list` |
| **修订 A-1（A10 R-1，已生效）** | `task.run.start` / `task.run.finish` **移出** `audit.json`：最小间隔 60s 的任务单任务即 **1440 次/天**，会把 cap 1000 的审计环形缓冲整体冲掉（A10 回查 `workspace.rs:388` 实证）。执行明细一律落 `task-runs.json`；手工触发改记低频的 `task.run.manual`。守护码位 `SCHED_AUDIT_PER_RUN_EVENT`（ACTIVE，见 `M4-5.d` 检查点 §2.1） |
| **每次 tick** | ❌ **禁止** `log_audit`。实测 `log_audit` 上限 1000 条 FIFO；每秒 1 条会在 ~17 分钟内把全部审计冲掉（旧草案已识别同款风险 K5） |
| 执行明细 | 复用既有 `script-runs.json`（`script_runner` 自动落盘，cap 200，输出尾存 256 KiB）；调度侧 `task-runs.json`（cap 500）**仅 UI 历史与排障，不参与判重** |
| 脱敏 | `detail` 不含参数值 / 命令正文 / 输出 / 任何凭据；`ScriptParam.secret` 参数值**根本不落盘**（§3.3 R-3） |
| 执行通道 | **唯一入口** `script_runner::start_run` / `start_command`（F6）；`scheduler.rs` / `tasks.rs` 禁止出现 `std::process::Command` / `sh -c` / `bash -c` |
| 凭据 | 任务不持有凭据；相关凭据一律走 `KeyringStore`，不进 `tasks.json` / 日志 / 前端 state / 审计 / 检查点 |

---

## 8. M4-5.d 夹具码位（**已落地，见 `M4-5.d` 检查点**）

> **本节的码位清单已被下游检查点取代**：`logs/checkpoints/M4-5.d-20260905-2355.md` §2.1 是
> **唯一权威集合**（ACTIVE 14 / PENDING 6），脚本 `scripts/check-scheduler-policy.py` 与之逐字一致。
> 取代原因有三，均记录在该检查点：
> ① board 23:55 的 `Batch Implementation Dispatch` 要求强制「无 `tokio_cron_scheduler`、无 `tokio`
> 直接提升、退出顺序可检」三项，原 9 码不含；
> ② 退出收口两项（`SCHED_SHUTDOWN_NOT_REGISTERED` / `SCHED_SHUTDOWN_ORDER`）由 pending 转 ACTIVE；
> ③ A10 `R-1` 触发契约 §7 修订，新增 `SCHED_AUDIT_PER_RUN_EVENT`。
> 本节保留为**历史提案痕迹**，不再作为施工依据。

### 8.1 初版提案（9 条，已被取代）

~~`SCHED_ENABLED_DEFAULT_TRUE`、`SCHED_SECOND_EXEC_PATH`、`SCHED_CLOCK_NOT_INJECTABLE`、
`SCHED_MISSED_POLICY_FIELD`、`SCHED_PERSIST_NOT_ATOMIC`、`SCHED_AUDIT_IN_TICK`、
`SCHED_CRON_MACRO_SUPPORT`、`SCHED_TASK_DEF_NO_DEFAULT`、`SCHED_SECRET_PARAM_PERSISTED`。~~

### 8.2 初版 pending（8 条，已被取代）

~~`SCHED_CMD_NOT_REGISTERED`、`SCHED_ACL_ORDER`、`SCHED_AUDIT_LEAKS_PARAMS`、
`SCHED_SHUTDOWN_NOT_REGISTERED`、`SCHED_SHUTDOWN_ORDER`、`SCHED_RETRY_UNBOUNDED`、
`SCHED_CATCHUP_UNBOUNDED`、`SCHED_HISTORY_AS_IDEMPOTENCY`。~~

### 8.3 落地要求（无论授权给谁）

1. 沿用既有夹具三约定：① `--self-test` **双向自检**（1 好样本零违规 + N 坏样本各自命中对应码）；② 坏样本**变异防呆**（变异未真正改动内容按漏检计）；③ `PENDING_CODES` 与 `--expect-pending` 双向一致，转默认时同步更新。
2. **pre-merge 项号须现场复核**：实测末条夹具为 `check-command-ui-logic.mjs`（M2-6.d，`scripts/pre-merge.sh:341`），新增夹具须插在 `git diff --check`（第 345 行）**之前**；A1 文档口径记为「第 22 项起」，以运行时实际项号为准。
3. 建议授权对象：**A7**（M4-6/M4-7 实现者，需要这些码位守护自己的实现）或单开 `M4-5.d` 卡；不建议由 A6 越权执行。

---

## 9. 测试矩阵落地映射（承接 A1 分配的 `T-sched-c1~c14`）

> 全部为**假时钟**下的纯函数/Rust 单测；真实进程取证归 M4-7.d（`T-trig-*`）。

| ID | 断言 |
|---|---|
| c1 | 合法 5 段表达式解析：`*/15 * * * *`、`0 9 * * 1-5`、`0 0 1,15 * *`、`0 0 * * 0` 与 `0 0 * * 7` 等价（周日） |
| c2 | 非法表达式全部拒绝：6 段、`@daily`、分值 60、空串、月 0、`*/0` → `TASK_INVALID_CRON` |
| c3 | `next_fire_after` 单调递增且**严格大于** `now`；连续调用 100 次不出现重复值 |
| c4 | `Interval.every_secs` 边界：59 拒绝、60 通过、2592000 通过、2592001 拒绝 |
| c5 | 假时钟跳过 5 个触发点 + `Skip` → 不补跑，`next_run_at` 指向未来，`task.run.missed(skipped)` × 5 |
| c6 | `RunOnce`：错过 5 次 → 只补跑 1 次，且用**最新** slot |
| c7 | `CatchUp`：错过 10 次、`catch_up_limit = 3` → 只补 3 次，其余记 `over_limit`；`catch_up_limit = 11` 定义期拒绝 |
| c8 | 时钟回拨（`now < last_fired_at`）→ 不追补、`last_fired_at` 不回改、`next_run_at` 从 `now` 重算、`task.clock.rewind` 已记 |
| c9 | 时钟前跳 2 小时 → 按策略处理，扫描上限 512 生效，不雪崩 |
| c10 | 同任务并发触发（tick + `task_run_now` 同时）→ 只执行一次，另一次记 `skipped(reentrant)` |
| c11 | 目标脚本正被手工执行（`AlreadyRunning`）→ skip，**不消耗** retry 配额 |
| c12 | 取消：`cancel(run_id)` → `Cancelled`，**不产生** attempt=2 |
| c13 | 重试：`Failed` + `max_attempts=3` + Exponential → 共 3 次，`scheduled_at` 不变、attempt 递增、delay clamp 到 `max_delay_secs` |
| c14 | 崩溃恢复：`last_fired_at` 已落盘但进程被杀 → 重启后**不重复**执行同一 slot（§5.1 不变量 1/3） |

---

## 10. 挂账与风险（A6 登记，不代为裁决）

| 编号 | 风险 / 待决 | 归属 |
|---|---|---|
| ~~O-A6-1~~ | **已解除**：`tokio` 裁定由 **A2（M4-1.a §3）**完成，选 (b) —— 应用层 `std::thread` + `Condvar`/`AtomicBool`，不引 tokio 直接依赖、不自建 runtime。本契约 §4.3 已按该形态锁定；调度器实现**不得**引 tokio runtime | ✅ 已闭环（A2） |
| **O-A6-2** | 判重真相源已裁定为 `last_fired_at`（非历史幂等键）。若 A7 改用历史判重，必须同时解决环形裁剪导致旧键丢失后的重复执行 | A7 |
| **O-A6-3** | `ScriptRunRecord` 无 `kind` 判别（M2-6 债务 **D21**）+ 缺 `cmd.run.cancel` / `cmd.run.finish`（**D22**）→ 定时运行的记录在 `script-runs.json` 中**无法与手工运行区分**。A6 裁定：**M4 不改 M2-4 既有记录结构**（避免跨里程碑改动），改用 `task-runs.json` 以 `run_id` 关联；D21/D22 **保持挂账**，是否单开卡由 A0 定 | A0 |
| **O-A6-4** | `cron-tool.html`（M2-7 种子）支持 6 位（秒 分 时 日 月 周），后端只接受 5 位 → M4-8 UI 必须显式标注并禁用 6 位模式，否则用户生成的表达式会在后端被拒 | A8 |
| **O-A6-5** | `Interval.every_secs` 下界 60 是对需求 #11 原文「every Ns」的保守收窄；放宽须单开卡（含休眠/补跑风暴的复核） | A0 / 需求方 |
| **O-A6-6** | `stop-scheduler` 与 `kill-running-scripts` 之间的微秒级窗口可能产生「kill 之后启动」的孤儿进程；M4-7.d 须以 `/proc` 真实取证，失败则收紧为持锁 | A7 |
| **O-A6-7** | 损坏 `tasks.json` 若沿用既有「静默 `unwrap_or_default()`」= 定时任务全丢且无痕；要求 A7 至少补 `.corrupt` 备份 + 日志 | A7 |
| **O-A6-8** | 三份主文档回写被 A6 **主动推迟**（A1 的 `详细设计与实施计划.md` 改动已入暂存但未提交，属高冲突文件且 A1 合并序在前）→ A0 在 A1 落地后套用 §11 文案 | A0 |
| **O-A6-9** | 进程表互斥键是 `script_id` / `snippet_id`（**非 task_id**）→ 两个任务引用同一脚本会互相 skip；UI 必须能展示「跳过原因」（`task.run.skipped` 的 `reason`） | A8 |
| **O-A6-10** | `retry` 若 M4-7 未实现，必须删除字段而非留作 no-op（`ScriptParam.raw` 教训） | A7 |
| **O-A6-11** | A7 §3 **C-1** 指出：`logs/assist/A10-M4-security-review-20260905-2240.md` §**G-8 标题仍写 `workspace/tasks.yaml`**，与 A1 F-4（禁 YAML，统一 `tasks.json`）冲突。本卡 §3.4 与 A7 一致取 **`tasks.json`**；建议 A0 订正 A10 的 G-8 标题，避免后卡按字面引入第二套序列化 | A0 |

---

## 11. A0 集成回写文案（A1 落地后套用，**A6 本人不改这三份文件**）

### 11.1 `详细设计与实施计划.md` §6.1 追加

```markdown
> **M4-5 契约已冻结（Lane A6，`AI:DEEP/R:high`，CodeBuddy Hy4）**：`TaskDef` / 时钟 / 错过执行 / 取消语义
> 见 `logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`。
> 关键裁定：① `TaskDef` **不引入 `timezone` 字段**（`chrono-tz` 零依赖，按 `chrono::Local` 求值）；
> ② cron 仅 5 段，秒级与宏一律定义期拒绝；③ `Interval.every_secs` 下界 60（对需求原文「every Ns」的保守收窄）；
> ④ 判重真相源 = `tasks.json` 的 `last_fired_at`（`task-runs.json` 只做历史，不参与判重）；
> ⑤ 取消复用 `ScriptProcessTable::cancel`，**不新增 `task_cancel` 命令**；
> ⑥ `stop-scheduler` 注册在 `stop-background-workers` 之后、`flush-sessions` 之前；
> ⑦ **新建任务默认 `enabled=false`**（裁定 R-A6-1：自动执行默认关闭，采纳 A10 G-7.5 更严口径）；
> ⑧ 执行形态按 A2 `M4-1.a` F-1 裁定 (b)：`std::thread` + `Condvar` 可中断等待，不引 tokio runtime。
```

### 11.2 `后续需求TODO.md` 当前执行结论追加

```markdown
> **M4-5 调度契约已冻结（Lane A6，Wave 1，纯契约、产品代码零改动）**，证据
> `logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`；NEXT=`M4-5.d`（夹具骨架，需 A0 授权）+ `M4-6.a`（Lane A7）。
> 与调度链相关的既有债务 **D21**（运行记录无 `kind` 判别）/ **D22**（缺 `cmd.run.cancel` / `cmd.run.finish`）保持挂账，M4 不代关。
```

### 11.3 `AI-模型切换与接手清单.md` 追加

```text
> M4-5 调度核心契约已冻结（Lane A6 / `AI:DEEP` / R:high，CodeBuddy Hy4；与 A1 展开者同模型，独立性弱于跨模型裁定，可被 A0/强模型复核推翻）。
```

---

## 12. A6 自验证（实测输出）

```text
开工前：
  cat .workspace-identity  → WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master
  pwd                      → /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
  git status --short --branch
      ## master...origin/master
       M 详细设计与实施计划.md                                ← A1 产出（本卡不碰）
      ?? logs/assist/A10-M4-security-review-20260905-2240.md  ← A10 产出
      ?? logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md
      ?? logs/checkpoints/M4-20260905-2225.md                 ← A1 产出
      ?? logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md
  git log --oneline -8     → HEAD=3544c09（基线 f376346 + 3544c09 波次说明）
  Wave 判定                → PARALLEL_COMMAND_BOARD.md §Dispatch Waves 列 A6 于 Wave 1 → 直接执行

交付前：
  python3 scripts/check-plan-routing.py  → check-plan-routing: ok (50 WBS rows)      [exit 0]
  git diff --check                       → 无输出                                     [exit 0]
  bash scripts/pre-merge.sh              → PRE_MERGE_RESULT=ALL_PASS                  [exit 0]
  git diff --stat -- src src-tauri scripts package.json package-lock.json
      → src-tauri/src/domain.rs | 353 +++++++++     ← **A2（M4-1.b 类型提案）产出，非本卡**
  git status --short --branch            → 本卡唯一新增 = logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md

并发 Lane 现状（交付时工作树，均非 A6 产出，A6 一律未触碰）：
   M 详细设计与实施计划.md / AI-模型切换与接手清单.md / 后续需求TODO.md   ← A1 等
   M src-tauri/src/domain.rs                                            ← A2（+353）
   ?? logs/checkpoints/M4-1.a-20260905-2245.md（A2 裁定书，本卡已引用）
   ?? logs/assist/A3|A4|A5|A7|A8|A9|A10|A11 …                            ← Wave 2/3 只读预研
   ?? logs/checkpoints/M4-20260905-2225.{md,patch}                      ← A1

产品代码改动（A6 本人）：0
提交与推送：无（按 board Merge Rule，只有 Lane A0 向 master 提交并推送）
```

---

## 13. 交付形态

- 本检查点（唯一新增文件）。
- **不提交、不 push**：以「检查点文档（无产品代码改动）」形式交付，符合 board Merge Rule 第三项。
- A0 集成时只需 `git add logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`，并套用 §11 的三段回写文案。

---

## 14. 下游解锁声明 —— 逐条对照 A7 的「解除阻塞条件」

> A7（`logs/assist/A7-M4-6-7-scheduler-backend-prework-20260905-2315.md` §1）自列 Wave 2 解锁条件。本卡逐条声明满足情况，供 A7 / A0 核对后放行 M4-6.a / M4-7.a。

| # | A7 解锁条件原文 | 满足 | 本卡落点 |
|---|---|---|---|
| 1 | `TaskDef` 字段逐项（**含默认 `enabled` 语义**） | ✅ | §3.1 类型提案 + §3.2 字段语义表 + **§3.5 裁定 R-A6-1（默认 `enabled=false`）** |
| 2 | 错过执行策略**落为 `TaskDef` 字段**（F9，不得为运行时常量） | ✅ | §3.1 `MissedRunPolicy` + `catch_up_limit` / `misfire_grace_secs`；守护码位 `SCHED_MISSED_POLICY_FIELD`（§8.1） |
| 3 | 取消语义 | ✅ | §5.4 取消矩阵（复用 `ScriptProcessTable::cancel`、**不新增 `task_cancel`**） |
| 4 | 系统时间回拨 / 跳变行为 | ✅ | §4.5（回拨不追补不倒退 / 前跳走错过执行 / DST 不存在时刻跳过、重复时刻只触发一次 / 休眠同前跳） |
| 5 | 时钟可注入 `Clock` 契约 | ✅ | §4.1 trait 提案 + 机器可检方式（`SCHED_CLOCK_NOT_INJECTABLE`） |
| 6 | cron 方言归属（自写 vs 引 crate） | ✅ | §4.4：**自研 5 段纯函数**，零新依赖，不支持宏/秒/年，非法一律定义期拒绝 |
| 7 | A2 的 `M4-1.a` tokio 裁定已落地（A7 §4 R-1 跨链阻塞） | ✅ | A2 已落 `M4-1.a-20260905-2245.md`，**选 (b) 不引 tokio**；本卡 §4.3 已按该形态锁定 |
| 8 | （A7 §3 C-4 点名）`enabled` 默认值与 `#[serde(default)]` 语义 | ✅ | §3.5：`default_task_enabled() -> false`，缺字段亦按 false（fail-closed 方向） |
| 9 | （A7 §3 C-1 点名）持久化格式统一 | ✅ | §3.4：`tasks.json` + `session::atomic_write`，禁 YAML |
| 10 | （A7 §5）`domain.rs` 只落 `TaskDef`，不碰 A2 的 db 类型提案区 | ✅ | A6 **未改任何产品代码**，`domain.rs` 仅作类型提案写入本文档，由 A7 落地时自行隔离落点 |

**A7 可据此从 Wave 2 解锁**（board §Dispatch Waves：「A7: Start after A6 freezes `TaskDef`, trigger kind, missed-run policy, and cancellation semantics」—— 四项全部满足）。
仍需注意：A7 落地时若发现本契约某条无法在 M4-6/M4-7 内实现，**按 A7 §4 R-3 的既定动作回写 BLOCKED，不得自行改字段**。
