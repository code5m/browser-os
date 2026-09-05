# A10 · M4 终版安全复核（Batch Re-Review After Fixes + Integration Fix Wave）

> Lane: `A10` — M4 security review（`AI:DEEP / R:xhigh`）
> 时间: 2026-09-06 07:04 CST
> Base: `master` @ `85d2d7b`（领先 origin/master 2 个文档提交；本批未 rebase，因其他 lane 占工作树）
> 批次: **Batch Re-Review After Fixes**（board §Integration Fix Wave → Lane A10）+ **IF-3 处置**
> 范围: **只读复核 + 止损诊断**。**未改任何产品代码、未改任何策略脚本**（IF-3 已由并发 lane 在工作树内修复，本次仅验证其正确性，不重复施加）。
> 前序: `A10-M4-security-review-20260905-2240.md`（G-1~G-12）、`A10-M4-security-recheck-A1A2A6-20260905-2345.md`（R-1~R-11）、`A10-M4-security-review-batch-DB-1-20260905-2319.md`、`A10-M4-security-review-batch-SCHED-1-20260905-2319.md`（均 `??`，待 A0 一并提交）。本文件为**终版裁定**，覆盖全部 M4 落地状态。

---

## 0. 方法声明

- 所有结论基于**源码实证**与**脚本实跑**，非文档互证。
- 实跑（本轮我亲自执行）：
  - `python3 scripts/check-database-policy.py` → `database policy: all invariants hold（ACTIVE=7）` **EXIT=0**
  - `python3 scripts/check-scheduler-policy.py` → `scheduler policy: all invariants hold（ACTIVE=20）` **EXIT=0**
  - `python3 scripts/check-scheduler-ui-policy.py` → `CHECK_SCHEDULER_UI_POLICY_RESULT=PASS（0 违规，14 个码位）` **EXIT=0**
  - `python3 scripts/check-tools-policy.py --self-test` → **`self-test OK … 16 个坏样本全部检出`** **EXIT=0**（早前红灯 IF-3 已消失，见 §1）
  - `cargo check --manifest-path src-tauri/Cargo.toml` → **2 warnings**（`grid_process.rs:76`、`grid_process.rs:103`，均为既有基线）→ 回到基线（见 §5）
- 对照实验：构造 `/tmp/if3probe` 用 `origin/master` 版 ACL（list_tools 与 list_artifact_images 相邻）跑自检 PASS；当前 M4 ACL（中间插 8 条）自检亦 PASS——证实 IF-3 修复已对中间插入稳健。

---

## 1. IF-3 处置（check-tools-policy.py --self-test 红灯）

**根因（确认）**：`scripts/check-tools-policy.py` 的坏样本 #10 原用「交换相邻两行」锚定 `'    "list_tools",\n    "list_artifact_images"`。M4 在二者之间插入 `task_*`×5 + `db_*`×3 共 8 条命令（ACL:109-118），相邻性被破坏 → `.replace()` 成为空操作 → 变异防呆判「变异失配」→ 自检 FAIL。**这是非 M4 脚本（da38a82 落地）的脆弱性，被 M4 改动触发**，与 A11「既有的非 M4 门禁缺陷」判断一致，但触发源是 M4。

**当前状态**：工作树内该脚本已相对 `origin/master` 改动（`git diff origin/master -- scripts/check-tools-policy.py`，+23 行），把坏样本 #10 改为「摘出 `list_tools` 行 → 追加到 `list_artifact_images` 锚点之后」，与两者间插入内容无关（见 `:368-388` 注释明写 IF-3 根因）。自检现已 PASS。修复**正确、稳健，且正是最优选法**——A10 不再另施补丁。

**处置建议（给 A0）**：
1. 该脚本是**受版本控制文件**（`git ls-files` 确认跟踪），修复已在未提交改动中——**集成时必须随 M4 一并提交**，否则红灯会在另一个干净 checkout 复现（origin/master 仍坏）。
2. 不要为过门禁而隐藏该失败；当前修复是「根因修复」而非 waiver，可接受。
3. `scripts/__pycache__/`（含 `check-tools-policy.cpython-312.pyc`、`check-scheduler-policy.cpython-312.pyc`）为运行副产物，确认在 `.gitignore`（或 A0 提交时用 `git add -p` 排除），勿入库。

---

## 2. 五问裁定（board §Lane A10 终版必须回答）

### Q1. DB 写闸门是否仍 fail-closed？ — **是 ✅**

证据链（两层闸门）：
- 命令层 `evaluate_db_query_gate`（`security_policy.rs:1138`）→ `require_write_confirmation`（`:1075`）：
  - `SqlRiskClass::Unknown → Err(SqlParseFailed)`（不可解析即拒，不降级放行）
  - `Ddl | Admin → WriteDenied`
  - `Write` + `!allow_write → WriteDenied`
  - `Production → WriteDenied`；`Unknown verdict → ProductionUnknown（拒）`
  - `NonProduction` 仍需 `confirmed` 才放行，否则 `WriteDenied`
- 驱动层第二道（database.rs）：`detect_multiple_statements(sql)` 拒多语句；SQL 超 `DB_MAX_SQL_BYTES` 拒；`conn.prepare(sql)` 单语句执行。
- `db_connect` 默认校验连通性即弃句柄（bridge.rs:5966-5971），写判定与 `cfg.allow_write`（缺省 false）绑定。

唯一**使用面提示**（非缺陷）：`confirm_write` 由前端作为自由布尔传入。当前 A5 DB UI 未接线（见 §4 F-N1），故无 UI 可误传 `true`；留给 A5 上线时确认默认不置 `confirm_write`。

### Q2. 凭据是否缺席于 DTO / 审计 / 前端状态 / 文件 / 日志？ — **是（两处 by-design 瞬时过境，可接受）✅**

- **DTO**：`DbConnectionConfig` 结构性无 password/dsn/connection_string 字段（domain.rs:907）。✅
- **审计**：`db.connect` detail = `conn_id + kind`；`db.query` = `conn_id/rows/truncated/field_truncated`；`db.disconnect` = `conn_id/removed`。**无 SQL 原文、无凭据**（bridge.rs:5980-6085）。✅
- **前端状态**：`DatabasePanel.vue:18` `const password = ref("")` 组件本地瞬时态；`useDatabaseStore.ts:109-125` 注释「绝不写入 form/store/localStorage」，仅在 `connect(password)` 入参瞬时传出。✅
- **文件**：凭据仅落系统密钥库，键 = `db:<conn_id>`（bridge.rs:5974, 6059）；`DbConnectionRegistry` 仅存 config，无 password（bridge.rs:5940-5949）。✅
- **日志/错误串**：`sanitize_message`（database.rs:220-267）把 `password=xxx`/`token:xxx` 形态整体替换为 `***`，并断言于单测（:932-944）；`db_query` 错误回传经 `e.message`（已脱敏）。✅
- **两处 by-design 瞬时过境**（与既有 `configureRepo(token)` 同模式，bridge.ts:96，可接受）：
  1. `db_connect(password: Option<String>)` 命令参数跨 IPC 传入后端（bridge.rs:5963）→ 立即进 Keyring，不持久化、不审计。
  2. Tauri 默认不落命令参数日志；无证据显示有 IPC 参数日志层。
  → 建议 A0 在主文档标注：db 凭据流转与 git 的 repo_id 命名空间隔离（`db:` 前缀），同 `configureRepo` 处置一致。

### Q3. 用户 SQL 路径是否单语句？ — **是 ✅**

- 唯一取数命令 `db_query` → `pool.query` → `database.rs` 用 `conn.prepare(sql)`（单语句，:663）+ `detect_multiple_statements` 前置拒绝（:656）。
- 全仓 grep `simple_query` / `batch_execute` → **0 命中**（database.rs 与 bridge.rs 均无）。
- `execute_batch` 仅出现在 `database.rs` 的 `#[cfg(test)] mod tests`（:750 起，:959/989/1009/1033/1061/1092/1120 全在测试模块内，非生产路径）。
- 第二道 `classify_sql_risk`（security_policy.rs）对多语句整批拒绝。

### Q4. 调度器是否复用 M2-4 执行通道且停机正确？ — **是 ✅**

- **单执行路径**：`scheduler.rs:7-8` 文档声明禁第二进程路径；实际触发 `crate::script_runner::start_run`（:639）/ `start_command`（:665）。全仓 grep `std::process::Command`/`Command::new`/`sh -c`/`bash -c` → 仅在注释（:8），运行代码零独立 spawn。
- **取消复用既有通道**：`cancel_in_flight`（scheduler.rs:737-754）调用 `script_runner` 的 `ScriptProcessTable::cancel`，无第二取消实现。
- **停机顺序正确**（F7）：`bridge::register_shutdown_tasks` 在 `stop-background-workers`（索引 0）之后立即注册 `stop-scheduler`（:797），**早于** `kill-running-scripts`（索引 5），与 A6 §5.5 裁定一致。
- **审计 detail 脱敏**（scheduler.rs:758-759）：仅含 `task_id/run_id/reason/status/error_code`，无参数/命令/脚本/凭据。

### Q5. 新增警告 / 策略例外是否可接受？ — **是 ✅（IF-4 已回落基线）**

- 本轮 `cargo check` 仅 **2 warnings**，均在 `grid_process.rs`（:76 `index`/`comms` never read；:103 `new` never used）——**正是 M0 基线 2 条**，说明 IF-4（曾观测 5 条新增死代码）已被并发 lane 在 fix-wave 中消解。
- 无新策略例外、无 `tokio`/`sqlx`/`diesel`/YAML/代码生成引入（Cargo.toml 仅 rusqlite/mysql/postgres，与冻结裁定一致）。
- 备注：早前观测的 5 条（DbSslMode unused import、QueryCancel 的 cancel/close/new）现已消失——说明 A3/A4 已完成 IF-4 清理，无需 A10 再动。

---

## 3. 仍存 HIGH 实债（需 A0 在 push 前裁决）

### R-1（高·仍 live）审计冲刷：`task.run.start` / `task.run.finish` 写入 `audit.json`（cap 1000 FIFO）

- 证据：`scheduler.rs:760-774` `record_run_start` / `record_run_finish` 调 `crate::workspace::log_audit(app, "task.run.start"/"task.run.finish", ...)`；`workspace.rs` 落盘 `audit.json` 且 cap 1000 FIFO。
- 量级：单任务每 60s = 1440 次/天 → 日增 2880 条 audit → 约 8.3h 冲刷一轮全局 `audit.json`，**手工操作审计被挤出**。明细已完整存于 `task-runs.json`（cap 500），重复存储且有害。
- 现状：A6 接受该建议并加 `SCHED_AUDIT_PER_RUN_EVENT` 码位，但**契约 §7 尚未修订**；A7 已按未修订 §7 落地为真实路径 → 已成为实债。
- **A0 必办**：① 修订契约 §7，把 `task.run.start/finish` 移出 `audit.json` 事件表（明细留 `task-runs.json`）；② 指派 A7 改 `record_run_start/finish` 写 `task-runs.json`；③ 同步把 `SCHED_AUDIT_PER_RUN_EVENT` 语义由「漏记」改为「误写进 audit.json」。在修订前**不建议 push**（违反 M4 可审计性红线）。
- 其余 R-2~R-11（A10 上轮提出）状态：R-6（M4-1.c STOPPED 裁定）已由 A2 收口（见 A2 patch，工作树 `M4-1.c-*.md` 已重写为完整契约）；R-3（dangerous 片段无人值守放大）、R-4（事后改标 secret 不复检）、R-5（未标 secret 明文落盘）仍属契约层已知残余风险，未变；R-7/R-8 已由 A3/A4 落地常量与脚本而关闭；R-9（ACL 计数）由 `DB_ACL_ORDER`/`SCHED_ACL_ORDER` 按锚点判定，已被两夹具采纳；R-10 关闭；R-11 由 A7 冻结重试常量（未独立验证，建议 A11 复测）。

---

## 4. 本轮新增集成面发现（非安全红线，供 A0）

### F-N1（中）`db_*` 命令前端桥接缺失 → 命令不可达 UI

- Rust 命令 `db_connect/db_query/db_disconnect` 已落地（bridge.rs:5959/5990/6047）、`main.rs:1421-1423` 注册、`default-commands.toml` ACL 允许（:115-117）。
- 但 `src/bridge.ts` 与 `src/types.ts` **无** `db*` 封装（grep 0 命中）；`DatabasePanel.vue` + `useDatabaseStore.ts` 采用「可选成员探测」模式（store 注释明写「A4 尚未落地 db_* 命令封装…保持禁用态」），故面板对真实后端**一律不发 IPC**。
- 结论：db 安全面当前**完全不可由 UI 触发**（仅 ACL+源码层存在），既非漏洞也非功能——属 A5/A4 的接线欠账。A0 集成时应知：要么 A5 补 `bridge.ts`/`types.ts` 封装并接线，要么显式登记为「DB UI 首期不启用」。安全评审不因此降级。

### F-N2（低）IF-2 构建体积门禁仍属 A5/A8 决策项

- DatabasePanel 已用 `defineAsyncComponent(() => import(...))` 懒加载（MainArea.vue:29），利于控体积。
- 体积 +14.6% 余量仅 0.4pt 的事实未变；`rusqlite`(bundled) 在功能未接线前已吃掉预算。是否上调阈值 / 改非 bundled / 延后 `mysql`/`postgres` 由 A0 决策（board IF-2 归属）。A10 不擅改。

---

## 5. 红灯总览（Integration Fix Wave 复核）

| ID | 项 | 状态 | A10 裁定 |
|---|---|---|---|
| IF-1 | cargo fmt main | 待 A0 全局 | 本批未跑 fmt（非 A10 权限）；建议 A0 收口前统一 `cargo fmt` |
| IF-2 | 前端体积 +14.6% | A5/A8 决策 | 懒加载已做；阈值决策归 A0 |
| IF-3 | check-tools-policy 自检 | **已修复（工作树）** | 根因确认，修复稳健，须随 M4 提交 |
| IF-4 | 警告 2→5 | **已回落基线 2** | 并发 lane 清理完成，可接受 |
| IF-5 | A11 报告过期 | 归 A11 | A11 须在全部 fix 后出终版 |

---

## 6. Lane Output Template

```
LANE=A10
STATUS=PASS_WITH_DEBT
BASE=85d2d7b (master, ahead origin/master by 2 doc commits)
HEAD=logs/assist/A10-M4-security-review-batch-final-20260906-0704.md (+ 4 前序 A10 文档，均 ?? 待 A0 提交)
FILES= logs/assist/A10-*.md (本批仅新增文档；未改产品代码/策略脚本)
VERIFY=
  check-database-policy.py        -> PASS (ACTIVE=7)
  check-scheduler-policy.py       -> PASS (ACTIVE=20)
  check-scheduler-ui-policy.py    -> PASS (14 码位)
  check-tools-policy.py --self-test -> PASS (16 坏样本全检出；IF-3 已消)
  cargo check (src-tauri)         -> 2 warnings (均 grid_process.rs 基线)
CHECKPOINT=logs/assist/A10-M4-security-review-batch-final-20260906-0704.md
MERGE_NOTES=
  1. IF-3 修复在 scripts/check-tools-policy.py 工作树改动中，须随 M4 提交（否则干净 checkout 复现红灯）。
  2. R-1 审计冲刷仍为 HIGH 实债：推送前必须修订契约 §7 并指派 A7 改代码。
  3. F-N1：db_* 命令无前端桥接，UI 不可达，建议 A0 决定接线或显式禁用。
  4. 工作树含大量其他 lane 未提交改动（database.rs/scheduler.rs/tasks.rs 等 ??），A0 集成按各自 lane 收口顺序整理，勿整体 stage。
NEXT=A0 集成（IF-1 全局 fmt；R-1 裁决；F-N1 决策；统一提交 M4 批次）
```

---

## 7. 声明（避免误读）

- 本轮**零产品代码改动、零策略脚本改动**（IF-3 修复由并发 lane 落于工作树，A10 仅验证其正确性，未重复施加——符合 board「A10 仅在 A0 指派时改策略脚本」）。
- 未 rebase / 未 commit / 未 push（board Merge Rule：仅 A0 可向 master 提交推送）。
- 结论全部源码实证 + 脚本实跑，非文档互证。前序 G-1~G-12 / R-1~R-11 仍有效，本文件为终版增量裁定，覆盖全部 M4 落地后的真实状态。
