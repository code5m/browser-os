# A7 · M4-6 / M4-7 调度器后端 — 只读预研（Wave 2 阻塞期）

> Lane：**A7**（M4-6 调度命令与持久化 / M4-7 安全触发与退出，Merge Order 5）
> 路由：`AI:DEEP / R:xhigh`
> 预研者：CodeBuddy 会话（Hy4 / 腾讯混元）
> 时间：2026-09-05 23:15 CST
> 基线：`master` @ `3544c09`（`WORKSPACE_ID=BACKV3_MAIN`，`pwd` = `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`）
> 状态：**BLOCKED（Wave 2 前置未完成）** —— 本次**零产品代码改动**，仅产出本只读预研
> 依据：`PARALLEL_COMMAND_BOARD.md`（23:00 版 §Dispatch Waves / §Startup Gate L33）、`logs/checkpoints/M4-20260905-2225.md`（A1 展开卡）

---

## 0. 本文件边界与合规声明

| 项 | 值 |
|---|---|
| Wave | **Wave 2**（「Start after A6 freezes `TaskDef`, trigger kind, missed-run policy, and cancellation semantics」） |
| 前置是否满足 | **否** —— `logs/checkpoints/` 下只有 A1 的 `M4-20260905-2225.md`，**无 `M4-5.a/b/c/d` 任何契约检查点** |
| 本次动作 | 只读：`cat` / `grep` / `ls` / 既有夹具 `--self-test`。**未新增或修改 `src/` `src-tauri/` `scripts/` 任何文件** |
| 未做 | 未 commit、未 push、未改任何 Lane 范围外文件、未替 A6 冻结契约 |
| 依据（Board L33） | 「If the only blocker is an unfinished predecessor lane, do not edit product code. Produce a read-only assist note under `logs/assist/`」 |

**工作树现状（含其它 Lane 的并发改动，A7 一律不触碰）**：

```text
## master...origin/master
 M  详细设计与实施计划.md                                ← Lane A1
??  logs/assist/A10-M4-security-review-20260905-2240.md   ← Lane A10
??  logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md ← Lane A11
??  logs/checkpoints/M4-20260905-2225.md                  ← Lane A1
??  logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md ← Lane A11
```

---

## 1. 解除阻塞条件（Exact Unblock Condition）

A7 认领 `M4-6.a` 的**充要条件**（逐条可机器核对）：

1. **A6 交付 `logs/checkpoints/M4-5.d-<ts>.md`**（M4-5.a/b/c/d 四张卡全 PASS），且冻结裁定书覆盖：
   - `TaskDef` 字段逐项（含**默认 enabled 语义** —— 见 §3 C-4）；
   - `TaskKind` 取值（A1 F-5 定为 `Script` / `Command`，不做 `Tool`）；
   - 持久化形态与路径（A1 F-4：`tasks.json`，**非 YAML**）；
   - 容量上限具体数值（对齐 `snippets.rs:31` `MAX_SNIPPETS=200` 口径）；
   - **错过执行策略落为 `TaskDef` 字段**（F9，skip / catch-up，不得为运行时常量）；
   - 取消语义、系统时间回拨/跳变行为；
   - 时钟可注入 `Clock` 契约与 cron 方言归属（自写 vs 引 crate）。
2. **A6 交付 `scripts/check-scheduler-policy.py`**（M4-5.d），含默认码 + pending 码位 + 变异防呆自检，供 M4-6/M4-7 把 pending 转默认。
3. **A2 的 `M4-1.a` tokio 裁定已落地**（跨链依赖，见 §4 R-1 —— 这是 A1 展开卡未显式列入依赖图、但 M4-7.a `FORBID` 明确要求的硬约束）。
4. 三份主文档顶部 `NEXT=` 已回填到 `M4-6.a`（由 A1/A6 回写，非 A7 职责）。

满足 1+2+3 后，A7 方可写 `src-tauri/src/tasks.rs`；`M4-7.*` 还需 `M4-6.c` PASS。

---

## 2. 起始面实测（一手证据，逐条可复跑）

| # | 实测项 | 结果 | 证据/复跑命令 |
|---|---|---|---|
| 1 | `src-tauri/src/` 下 scheduler / tasks 文件 | **零** 命中 | `ls src-tauri/src/ \| grep -Ei "sched\|task"` → 空 |
| 2 | `#[tauri::command]` 数量 | **105**（`bridge.rs`） | `grep -c "#\[tauri::command\]" src-tauri/src/bridge.rs` |
| 3 | ACL 规模与插入锚点 | `default-commands.toml` **111 行**；末条 `list_artifact_images` 在**第 110 行**；`task_*` 五命令**须插在其之前**（A1 实测项 6，三度踩坑） | `tail -8 src-tauri/permissions/default-commands.toml` |
| 4 | `check-scheduler-policy.py` | **不存在**（由 A6 在 M4-5.d 建立） | `ls scripts/ \| grep -E "^check-"` |
| 5 | 执行通道（F6 唯一入口） | `script_runner.rs:843 start_run`、`882 start_command`、`909 kill_all_running`、`224 ScriptProcessTable`、`282 cancel`、`330 effective_timeout` | `grep -n "pub fn start_run\|pub fn start_command" src-tauri/src/script_runner.rs` |
| 6 | `start_run` / `start_command` 签名 | 均需 `table: &Arc<ScriptProcessTable>`、`roots: &[PathBuf]`、`home_dir`、`app: Option<AppHandle>`、`records_file: Option<PathBuf>` —— **调度触发可完整复用，无需扩参** | `script_runner.rs:843-890` |
| 7 | 关机注册顺序（生产） | 6 任务：`stop-background-workers`(1) → `flush-sessions`(2) → `close-tabs`(3) → `kill-terminals`(4) → `shutdown-grid`(5) → `kill-running-scripts`(6) | `bridge.rs:778-885` |
| 8 | **shutdown.rs 测试的硬编码面** | `register_realistic_tasks` 的名字数组**只有 4 个**：`stop-background-workers`/`close-tabs`/`kill-terminals`/`shutdown-grid`；且 `assert_eq!(report.executed(), 4)`、`names` 顺序断言同为这 4 个 —— **不含 `flush-sessions` 与 `kill-running-scripts`** | `shutdown.rs:446-457`、`480`、`656-665` |
| 9 | 持久化范式 | `session.rs:30 atomic_write`（写 `.tmp` + rename）、`prune_sessions`（按 `updated_at` 从旧到新）、T-sp-8 清理孤儿 `.json.tmp` | `grep -n "pub fn atomic_write\|prune_sessions" src-tauri/src/session.rs` |
| 10 | 领域校验范式 | `snippets.rs:31 MAX_SNIPPETS=200`、`142 validate_snippet`（id / 字段长度 / params / argv 上限）、稳定错误码枚举 | `src-tauri/src/snippets.rs:31-160` |
| 11 | 审计入口 | `workspace.rs:381 log_audit(app: &AppHandle, action: &str, detail: String)`，**非原子写**、上限 1000 条 | `grep -n "pub fn log_audit" src-tauri/src/workspace.rs` |
| 12 | 夹具基线 | `check-lifecycle-contract.py`：自检 `ALL_PASS`、默认 `PASS`，且**存在 `--expect-current-gaps`**（第 236 行，供 M4-7.c 扩不变量时用 pending 机制）；`check-script-exec-policy.py`：自检 `ALL_PASS`（23 坏 + 1 好） | `python3 scripts/check-lifecycle-contract.py --self-test` |
| 13 | pre-merge 接入位 | `pre-merge.sh` **498 行**；末个功能夹具项为 `M3.c 终端体验项 UI 逻辑`（第 341 行），其后第 345 行即 `git diff --check` → **M4 新夹具须插在 341 与 345 之间**；`git diff --check` 之后不得再插 | `grep -n "pm_log" scripts/pre-merge.sh \| tail` |
| 14 | 债务基线 | D23/D24/D25/D26 挂账中；**M4 新增债务从 D27 起编号**，不得顺手关闭 | A1 展开卡 §4 F12 |

---

## 3. 跨 Lane 口径冲突（A7 开工前须裁定，勿按字面施工）

> 冲突来源：A10 安全评审（22:40）与 A1 展开卡（22:25）在 A10 自述「已按 A1 实测修订」的前提下，仍有 3 处字面未对齐；A7 若照抄会直接违反 F 条款。

| 编号 | 冲突点 | A1 展开卡口径 | A10 安全评审口径 | A7 处置建议 |
|---|---|---|---|---|
| **C-1** | 持久化格式 | **F-4：统一 `tasks.json`，禁 YAML**（既有持久化全为 JSON，无 YAML 解析器） | G-8 标题与正文仍写 `workspace/tasks.yaml` | **以 A1 F-4 为准**。建议 A0 订正 A10 G-8 标题，避免后卡误读引入第二套序列化 |
| **C-2** | 关机任务注册位置 | **F7：索引须小于 `kill-running-scripts`**（即 ≤5） | **G-9：须为首个任务，或紧随 `stop-background-workers`**（即索引 1） | **取严者（G-9，索引 1）**，理由：A10 指出定时器若在进程回收后触发会拉起子进程，正是门禁要防的场景。插入点不得破坏 `flush-sessions` 早于 `close-tabs` 的既有约定（`bridge.rs:792` 注释） |
| **C-3** | 审计标记 | F5：detail 只含 `op/conn_id/task_id/计数/风险等级`，**不含命令正文/参数值/凭据/连接串** | G-7.3：审计须带 **`origin=scheduler`** 以区分人工执行 | **两者叠加**：detail 在 F5 白名单字段基础上增加 `origin` 取值。需 A6 在 M4-5 冻结 `origin` 枚举（`manual` / `scheduler`），否则 A7 自行命名会与 A8 UI 展示口径漂移 |
| **C-4** | 新建任务默认态 | M4-6 卡正文未提 | G-7.5：**新建任务默认 `enabled=false`**，启用为显式动作 | 需 A6 在 `TaskDef` 冻结里明确 `enabled` 默认值与 `#[serde(default)]` 语义（存量文件无该字段时的兼容行为） |

---

## 4. 风险（A7 实现期，登记不代为裁决）

| 编号 | 风险 | 影响 | 建议缓解 |
|---|---|---|---|
| **R-1** | **跨链阻塞：M4-7.a 依赖 A2 的 tokio 裁定**。A1 展开卡 §3.3 称「库链与调度链互不阻塞，可完全并行」，但 M4-7.a 的 `FORBID` 明写「**不得绕过 M4-1.a 关于 tokio 的裁定**」，而 A1 实测项 3 确认 `tokio` 仅为传递依赖（1.53.1）、当前**不可 `use`** | 这是**依赖图未表达的第四条依赖**，会使 M4-7.a 在 A2 未交付时开工即违规 | A7 开工前确认 `logs/checkpoints/M4-1.a-<ts>.md` 存在；若选 (b) `std::thread` + `Condvar` 可中断 sleep，则调度器须按该形态设计，不得引 tokio runtime。**建议 A0 在依赖图补这条边** |
| **R-2** | **F7 的机器可检性存在缺口**：`shutdown.rs` 现有测试的名字数组只有 4 个且**不含 `kill-running-scripts`**（实测项 8），无法证明「stop-scheduler 索引 < kill-running-scripts 索引」 | 按 A10 G-9 的说法「同步更新 `shutdown.rs` 的顺序期望」并不足以实现 F7 断言 | A7 须**新建**生产注册顺序单测（遍历 `register_shutdown_tasks` 产出，断言 `stop-scheduler` index < `kill-running-scripts` index），而非改既有 4 名字数组；另在 `check-lifecycle-contract.py` 用 `--expect-current-gaps` 机制扩一条不变量 |
| **R-3** | 重入互斥与错过执行（M4-7.b）依赖 M4-5.c 的**字段**冻结（F9） | 若 A6 只给运行时常量，F9 不可检 | A7 在 M4-6.a 落 `TaskDef` 时若发现该字段缺失 → **停止并回写 BLOCKED**，不得自行添字段 |
| **R-4** | 审计脱敏：任务审计 detail 易误带命令正文/参数值（F5）；`start_run` / `start_command` 的 `values: &HashMap<String,String>` 是参数值载体 | 违反 F5 与 G-7 | 沿用既有「审计格式串单测」范式（A1 展开卡 M4-3.a 引用的 `script_run_audit_format_strings_do_not_include_values_or_body`），在 M4-6.b 同步加一条 task 版 |
| **R-5** | M4-7.d 真实进程取证（`/proc` 验证进程组消失、<15s） | 时间预算与稳定性 | 沿用 M2-4 B7/B8/B9/B11/B12 与 M3.a 的取证口径；无 GUI 通道，退出后无 timer/子进程残留须以 `/proc` + 线程计数取证 |
| **R-6** | 并发 Lane 改动：A1 已改 `详细设计与实施计划.md`（staged），A10/A11 已落 `logs/`。A7 在 A0 集成前若直接改 `bridge.rs` / `domain.rs` / ACL（均属**高冲突文件**） | 与 A3（M4-2/M4-3，同改 `domain.rs` / `bridge.rs` / ACL）撞车 | A7 严格按 Merge Order 5 交付**补丁 + 检查点**；`domain.rs` 只落 `TaskDef` 一处；ACL 五条 `task_*` 严格插在 `list_artifact_images`（第 110 行）**之前** |
| **R-7** | 债务纪律 | — | M4 新增债务从 **D27** 起编号；不得顺手关闭 D23/D24/D25/D26（F12） |

---

## 5. 预期改动面（Likely Files，待 A6 冻结后确认）

| 文件 | 动作 | 落点/锚点 |
|---|---|---|
| `src-tauri/src/tasks.rs` | **新增**（M4-6.a 纯函数层） | 照 `snippets.rs`（校验 + 稳定错误码）与 `session.rs::atomic_write`（`.tmp` + rename）范式；容量上限对齐 `MAX_SNIPPETS=200` 口径 |
| `src-tauri/src/domain.rs` | 改（仅 `TaskDef` + 常量） | 新增字段一律 `#[serde(default)]`；**不得触碰 M4-1.b 的 db 类型提案区**（与 A2/A3/A4 高冲突） |
| `src-tauri/src/scheduler*.rs` | **新增**（M4-7.a） | **禁** `std::process::Command` / `sh -c` / `bash -c`（F6）；判定函数禁裸 `SystemTime::now()` / `Instant`（F8） |
| `src-tauri/src/bridge.rs` | 改（M4-6.b 五命令 + M4-7.c 关机注册） | 命令过 `check_invocation_source`；关机任务插在 `bridge.rs:783` 之后（索引 1，见 C-2） |
| `src-tauri/src/main.rs` | 改（`mod` + handler 注册） | 一行 `mod` + `invoke_handler` 追加 |
| `src-tauri/permissions/default-commands.toml` | 改（5 条 `task_*`） | **必须插在第 110 行 `list_artifact_images` 之前** |
| `src/types.ts` / `src/bridge.ts` | 改（TS 镜像 + 封装） | 沿用既有镜像范式；组件不得直接 invoke |
| `scripts/check-scheduler-policy.py` | 扩（把 A6 的 pending 码转默认） | 新增 `SCHED_SECOND_EXEC_PATH` / `SCHED_CLOCK_NOT_INJECTABLE` / `SCHED_MISSED_POLICY_FIELD` / `SCHED_PERSIST_NOT_ATOMIC` 等默认码 |
| `scripts/check-lifecycle-contract.py` | 扩（M4-7.c 一条不变量） | 用 `--expect-current-gaps` pending 机制，避免未落地即染红 |
| `scripts/pre-merge.sh` | 改（接入新夹具） | 插在第 341 行与第 345 行之间；项号**开工时现场复核**（A1 文档口径为「第 22 项起」） |
| `src-tauri/src/shutdown.rs`（测试模块） | **不改** 既有 4 名字数组 | 改它不等于实现 F7；另建生产注册顺序单测（见 R-2） |

---

## 6. 建议的三处取证清单（与 A11 验证矩阵对齐，未开工故未执行）

A11 `logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md:67` 给 A7 的定向命令：

```text
cargo test scheduler
python3 scripts/check-scheduler-policy.py --self-test   # 1 好 + N 坏，双向自检 + 变异防呆
python3 scripts/check-scheduler-policy.py               # 默认零违规
python3 scripts/check-lifecycle-contract.py --self-test --expect-current-gaps + 默认
python3 scripts/check-script-exec-policy.py             # 证明未新增第二执行路径
```

以及门禁映射：G5（`grep -rn "std::process::Command" src-tauri/src/scheduler*.rs` 应为空）/ G6（退出经既有协调器）/ G7（新命令三处同步）/ G10（休眠·重启·错过执行·重复触发·系统时间变化·任务损坏·不并发重入）/ T0-10（`check-lifecycle-contract.py --self-test`）。

---

## 7. A7 自检记录（本次预研，只读）

```text
cat .workspace-identity                     → WORKSPACE_ID=BACKV3_MAIN
pwd                                         → /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --short --branch                 → ## master...origin/master（含 A1/A10/A11 并发改动，A7 未触碰）
git log --oneline -12                       → HEAD=3544c09
ls logs/checkpoints/ | grep M4              → 仅 M4-20260905-2225.md、M4-A11-verification-matrix-*.md（**无 M4-5.*，契约未冻结**）
ls src-tauri/src/ | grep -Ei "sched|task"   → 空（零实现）
grep -c "#\[tauri::command\]" bridge.rs     → 105
tail -8 permissions/default-commands.toml   → 末条 list_artifact_images（第 110 行 / 共 111 行）
python3 scripts/check-lifecycle-contract.py --self-test → SELF_TEST_RESULT=ALL_PASS
python3 scripts/check-lifecycle-contract.py            → LIFECYCLE_CONTRACT_RESULT=PASS
python3 scripts/check-script-exec-policy.py --self-test → SELF_TEST_RESULT=ALL_PASS（23 坏 + 1 好）
grep -n "expect-current-gaps" scripts/check-lifecycle-contract.py → 236（pending 机制可用）
wc -l scripts/pre-merge.sh                  → 498（末个夹具项 341 行，其后 345 行为 git diff --check）
```

**结论**：目录 ✅ / 分支 ✅ / 工作树（他人改动，A7 未触碰）✅ / **NEXT 与 Wave 2 前置 ❌** → 按 Board 规则，A7 本轮**不写产品代码**，仅交付本只读预研。
