# A6 · M5-W18-R2B 资源生命周期与恢复合同（Resource Lifecycle & Recovery Contract）

> Lane: A6 · Role: RESEARCH ONLY（只产出报告/checkpoint/候选实现卡，不写产品代码、不 push）
> DISPATCH: `M5-W18-R2B`（任务卡 §7「资源生命周期与恢复合同」）· `W19=CLOSED`
> WORKDIR: `/home/ainfinit/.codex/worktrees/m5-w18-a6/mvp-browser-os-v3` · BRANCH: `codex/m5-w18-a6`
> BASE: `434e63f`（rebase 后的 origin/master 顶）· HEAD: 本包提交后填（见 §7）
> 参考源：当前产品（worktree `m5-w18-a6`，HEAD `38faa2d`）；dbx 快照 `/home/ainfinit/Documents/极智简单/V3/research/dbx-src`（Apache-2.0，Cargo.lock SHA-256 `c0a7be12…`）；A4 R1 源映射 `logs/research/M5-W18/A4-dbx-backend-architecture-map.md`（SHA `971378c`）；蓝图 `WORKBENCH_BLUEPRINT-20260908.md`、`PARALLEL_COMMAND_BOARD.md`、`A0-M5-W18-R2B-dispatch-20260908.md`。

---

## 0. 修订日志与旧成果复用（任务卡 §1.1 / 通用完成条件）

**复用旧成果（不重写整篇）**：
- `A6-security-lifecycle-audit.md`（当前 HEAD `38faa2d`，原 R2 证据闭环）已覆盖：凭据 source-to-sink 全链路（form→JS→Tauri→Rust→keyring/SQLite→log/audit/shutdown）、dbx 真实活路径（SQLite 明文 `connection_secrets` 表 + 内存 `SessionCredentialStore`）、取消/超时/写确认三条竞态时间线与 fail-closed 测试。本包在其之上补 R2B 专属的「生命周期与恢复合同」，不重复。
- A4 R1 `A4-dbx-backend-architecture-map.md`（`971378c`）的 dbx 函数级映射（connection / schema / query_execution / query_cancel / history / csv_export）作为「A4请求合同」输入端引用；A4 的 R2B 正式契约截至本包撰写时尚未提交（详见 §6 OPEN_DECISIONS），本包按 R2B §13「A4/A9 先补 R2，其他 lane 不必等待」独立成稿。

**本包相对 R2 的新增纠正/强调**：
- 明确了「瞬时经 JS 参数」不是安全缺陷，而是**受控过渡态**：要求不误持久化、不泄漏，且**不得虚称"从未进内存"**——此点已被蓝图书写为永久防回退项 #5（`WORKBENCH_BLUEPRINT-20260908.md` 第 130 行），本包将其纳入故障注入项 F-CRED-1。
- 把"查询状态转移"从 R2 的"竞态时间线"升级为**显式状态机**（本包 §2），区分一次完成 / 取消幂等 / 旧结果丢弃 / 文档切换。
- 首次把**调度器的先落盘后启动 / load-modify-save 串行 / 崩溃恢复测试**逐行定位（§3），并明确 UI 重构不得破坏这些保证。
- 首次给出**工作台布局/文档恢复的最小持久化增量设计**与 **S0/S5 故障注入表**（§4、§5）。

---

## 1. 证据标签

- `CURRENT_PRODUCT`（已读源码，行号核实）：`src-tauri/src/scheduler.rs`、`tasks.rs`、`shutdown.rs`、`session.rs`、`database.rs`；`src/stores/useLayoutStore.ts`、`useDatabaseStore.ts`；`src/App.vue`。
- `REFERENCE_SOURCE`：dbx 快照（Apache-2.0）；A4 R1 映射（`971378c`）。
- `OFFICIAL_DOC`：`WORKBENCH_BLUEPRINT-20260908.md`、`PARALLEL_COMMAND_BOARD.md`、`A0-M5-W18-R2B-dispatch-20260908.md`。
- `OBSERVED_BEHAVIOR`：grep + 局部 read 得到的调度/关闭/原子写行为（行号见各 §）。
- `INFERENCE`：前端布局/文档恢复当前**未持久化**（grep 无 localStorage/atomic_write 落点）→ 属设计目标。
- `DESIGN_DECISION`：§3/§4 中标注的取舍，均 `PROPOSED_NOT_AUTHORIZED`。

---

## 2. 查询状态转移（完成 / 取消 / 超时 / 断线 / 关闭 / 文档切换）

> 来源：`database.rs` 的 `QueryCancel`(L168) / `QueryDeadline` / `QueryLimiter`(L578)，`bridge.rs` 的 `db_query`/`db_connect`/`db_disconnect`，R2 审计 §4 竞态时间线。

### 2.1 状态机（单次查询）

```
            ┌─────────────┐
            │  IDLE        │  (文档打开, 未执行)
            └──────┬──────┘
                   │ db_query(sql, confirm_write, timeout)
                   ▼
            ┌─────────────┐
   ┌─────── │ GATE        │ (evaluate_db_query_gate: 写确认/生产判定) ──fail──► REJECTED(终态, 旧结果保留)
   │        └──────┬──────┘
   │               │ pass
   │               ▼
   │        ┌─────────────┐
   │        │ RUNNING     │ ←── 每 64 行检查 QueryCancel.is_cancelled (L184-192)
   │        │  (QueryDeadline 计时)                │
   │        └───┬─────┬──────┬────────┬────────┘
   │   cancel   │     │ timeout│       │ disconnect/doc-switch
   │     ┌──────▼ │     ▼      │       ▼
   │     │ ┌────────┐ ┌────────┐ │  ┌────────────┐
   │     │ │CANCELLED│ │TIMEOUT│ │  │ ABORTED     │ (资源释放, 旧结果丢弃)
   │     │ └───┬────┘ └───┬────┘ │  └────────────┘
   │     │     │ 一次完成  │     │
   │     │     ▼          ▼     │
   │     │  ┌─────────────────┐ │
   └─────┴──│ COMPLETED       │◄┘
            │ (truncated/field_truncated 标记)
            └─────────────────┘
```

### 2.2 不变量（每条对应真实实现或待实现测试）

- **一次完成（at-most-once 执行语义）**：`db_query` 在 `bridge.rs` 内为同步命令体，连接随作用域 drop（R2 审计 1.5）；结果一旦写回即 COMPLETED，不二次触发。
- **取消幂等**：当前 `QueryCancel::cancel` 标 `#[allow(dead_code)]`（`database.rs:187`）——**无 `db_cancel` 命令触发**，故取消竞态当前不可达（无外部触发点）。幂等性待 `db_cancel` 命令落地后由 R2 审计 §4.2 的 fail-closed 测试覆盖（G3）。
- **旧结果丢弃**：切换 SQL 文档 / 重新执行时，前端 `useDatabaseStore` 的 `result.value = null`（`useDatabaseStore.ts:105/121/185`）清空旧结果；后端每次 `db_query` 新建 `DbPool`（bridge.rs:6069），无跨请求结果串扰。
- **断线/关闭 ≠ 取消任务/销毁会话**：蓝图第 32/82 行明确"关闭视图、取消任务、销毁会话须是不同语义"。当前 `db_disconnect` 仅删登记 config + 清 keyring（`bridge.rs:6100-6112`）；`ShutdownCoordinator` 负责进程级协调（§3.3）。
- **文档切换**：J2 要求"至少两份 SQL 文档分别保留文本、连接、执行状态和结果"——当前 `useDatabaseStore` 仅持有单 `result` ref（in-memory），**无多文档独立结果**；属设计目标，见 §4。

---

## 3. 调度：先落盘后启动 / load-modify-save / 崩溃恢复（真实实现定位）

> 来源：`scheduler.rs`（HEAD `38faa2d`）、`tasks.rs`、`shutdown.rs`、`session.rs`。

### 3.1 先落盘后启动（persist-then-start）

`tick()` 的逐任务触发路径（`scheduler.rs:415-428`）严格遵循"预约持久化 → 原子落盘 → 再 spawn"：

```text
415  reserve_scheduled_slot(task, slot, now);          // 写 last_fired_at（判重真相源）
419  // Reserve the slot durably before spawning.
425  if tasks::save_tasks_at(&path, &reserved_list).is_err() {
426      record_reject(app, &task.id, tasks::TASK_PERSIST_FAILED);
427      continue;                                     // 落盘失败 → 跳过本次，不 spawn
428  }
431  match fire(app, task, slot, trigger, 1) { ... }    // 仅落盘成功后启动
```

注释 `416-418` 明确："A crash after spawn but before the old end-of-tick save must not replay the slot on restart. The trade-off is intentional: persistence failure skips one run instead of risking duplicate execution." —— **这是 exactly-once 外部副作用的显式 fail-closed 取舍：宁可漏跑一次，绝不重复执行**。

### 3.2 判重真相源 = `last_fired_at`（崩溃不重放）

- `last_fired_at: Option<DateTime<Utc>>`（`scheduler.rs:118`）是计划去重的唯一真相源；`collect_missed_slots` 以它为 anchor（`scheduler.rs:368`）。
- 测试 `scheduler.rs:961` 断言："已触发的 slot 必须被过滤（判重真相源 = last_fired_at）"。
- 因 `last_fired_at` 在 spawn **之前**已通过 `save_tasks_at`→`session::atomic_write` 落盘，进程在 spawn 后崩溃重启时，该 slot 已被记为 fired，**不会被重新规划执行**。

### 3.3 load-modify-save 串行临界区

- `tick()` 入口 `let _store_guard = tasks::task_store_lock();`（`scheduler.rs:308`）锁住整段 load-modify-save。
- `tasks.rs:603-605` 断言："调度器与任务命令的 load-modify-save 临界区必须串行"——`task_store_lock()` 同时被调度器与 `task_*` 命令（add/update/remove/run_now）持有，防止并发改同一 `tasks.json`。
- 单原子写路径：`save_tasks_at`→`session::atomic_write`（`tasks.rs:463-466`），`F10` 约定"不新造第二条原子写路径"。

### 3.4 关闭协调器幂等（UI 重构必须保留）

`shutdown.rs` 的 `ShutdownCoordinator`：
- `already_shutdown` 标志 + `shutdown()` 第二次调用返回 `already_shutdown=true` 且**不再执行任何任务**（`shutdown.rs:152-174`；测试 `248-253`、`278-280`）。
- `register` 在已关闭后被拒（`324-334`："被拒绝的任务不得执行"）。
- 重入安全（`339-370`）；失败仍标记 `already_shutdown`（`531`、`541`）。
- 顺序可验证（`446-483`  realistic task ordering）。

### 3.5 既有崩溃恢复测试定位（验收锚点）

| 保证 | 测试位置 | 断言要点 |
|---|---|---|
| 原子写无 `.tmp` 残留 | `tasks.rs:900-915`（t_task_persistence_roundtrip_is_atomic）、`session.rs:386-406`（atomic_write_leaves_no_tmp） | atomic_write 后目录无 `.tmp`；内容可回读 |
| 损坏文件不产出任务且不静默清空 | `tasks.rs:926-928`（corrupt → `loaded.is_empty()`）、`session.rs:416-435`（corrupt_file_does_not_break_listing） | 解析失败返回空，**不覆盖磁盘上的损坏文件** |
| 缺失文件安全加载 | `tasks.rs:937-940` | 文件不存在 → 空列表，无备份动作 |
| 判重真相源 | `scheduler.rs:961-979` | 已 fired slot 被过滤 |
| 持久性错误识别 | `scheduler.rs:1027-1034`（is_persistent_error） | `SCRIPT_PATH_REJECTED`/`TASK_TARGET_NOT_FOUND` 为持久；`SPAWN_FAILED`/`TIMEOUT` 为瞬态 |
| 关闭幂等/重入/顺序 | `shutdown.rs:220-558` | 多次调用仅执行一次；顺序确定 |

### 3.6 UI 重构约束（交 A1/A5）

任何工作台/数据库 UI 重构**不得**破坏：① `last_fired_at` 在 spawn 前的持久化顺序；② `task_store_lock` 串行临界区；③ `ShutdownCoordinator` 幂等；④ 损坏 `tasks.json` 不静默清空。这些在 R2 审计与本文 §5 故障注入表中列为强制回归项。

---

## 4. 工作台布局 / 文档恢复最小持久化增量设计（DESIGN_DECISION, PROPOSED_NOT_AUTHORIZED）

### 4.1 现状（OBSERVED / INFERENCE）

- `useLayoutStore.ts`：grep 无 `localStorage`/`atomic_write`/`schema_version` 落点 → **布局当前未持久化**（重启回到默认）。
- `useDatabaseStore.ts`：grep 仅 `JSON.stringify(e)` 错误处理 + `result` ref；注释明确"绝不写入 form / store / localStorage（F2）"（`useDatabaseStore.ts:84`）→ **SQL 文档文本/连接/结果当前未持久化**，且凭据明确不落盘（与蓝图防回退 #5 一致）。
- 蓝图 J1/J6 要求"重启恢复允许持久化的文档与布局"——故属**待设计增量**，非已实现。

### 4.2 设计原则（对齐蓝图 §5 共同契约 + 防回退项）

1. **单一原子写原语**：复用 Rust 侧 `session::atomic_write`（`tmp`+`rename`，`prune_tmp_files` 启动清理，`session.rs:30-36/218-232`）。**禁止**前端 `localStorage` 直接写布局/文档——它非原子、无 `schema_version`、无损坏备份，违背蓝图 §5「持久化：原子写、失败恢复、容量、淘汰与迁移」。
2. **schema_version**：每个持久化文件首字段带 `schema_version: u32`，读取时按版本迁移；未知版本拒绝加载而非零初始化。
3. **凭据/敏感隔离**：凭据只在 OS keyring（`db:<conn_id>`，R2 审计 1.1.1），**绝不**进入布局/文档恢复文件；带敏感值的 URL、完整 SQL **输出全文**不随布局落盘（蓝图第 93 行）。文档恢复仅存：`{id, type, workspace, resourceLocator(conn_id 引用而非密码), dirty, execState(非结果行), lastViewState}`。
4. **UI 状态 ≠ 后端资源句柄**：蓝图第 89 行——UI 状态与后端资源句柄不混存；连接句柄在查询时由 `conn_id` 现取，不进恢复文件。
5. **损坏备份**：读取失败→保留磁盘损坏文件、另写 `.corrupt-<timestamp>` 副本、加载空安全默认；**不以零初始化覆盖损坏数据**（蓝图防回退隐含 + tasks.rs 既有行为可作参照）。
6. **重启不重放执行**：恢复文档**只恢复文本/连接/视图状态**，**不自动重跑**任何命令/查询/任务（J1/J5/J6）。调度器自身的重放防护见 §3.2。

### 4.3 建议文件与字段（PROPOSED_NEW，未经授权）

- `data_dir/layout-state.json`：`{schema_version, panels:{left:{width,collapsed},bottom:{height,collapsed},mainView,tabOrder[]},focus}`
- `data_dir/doc-recovery.json`：`{schema_version, docs:[{id,type:"sql"|"note"|"web",workspace,resourceLocator,connRef(可选),dirty,lastViewState,pendingSqlText(可选)}]}`
- 二者均经**新增 Tauri 命令**（如 `save_layout_state`/`save_doc_recovery`）调用 `session::atomic_write`；命令须含 `check_invocation_source` + ACL + 前端 `bridge.ts`/`types.ts` 暴露 + 策略脚本（一次性提交包，违背 R2B 不改产品代码的约束，故仅作候选卡）。

> 注：本设计为研究产物；具体文件名/字段以 A1 壳层契约（蓝图 J1/J6 + A1 草案）最终冻结为准，本包不抢注。

---

## 5. S0 / S5 故障注入表（与 A9 独立评审）

> 框架来源：蓝图 §6 的 S0（现有契约与可运行基线）/ S5（状态恢复与连续使用验收）。每项标 `READY`(实现已保证)/`PROPOSED`(待实现测试)/`NOT_RUN`(无 GUI 通道)。

| ID | 场景 | 注入 | 预期 | 证据/保证 | 状态 |
|---|---|---|---|---|---|
| F-CRED-1 | 凭据误持久化 | 在 `useDatabaseStore`/布局文件 grep `password` 落盘 | 0 命中；仅 keyring `db:<conn_id>` | R2 审计 1.1.1；蓝图防回退#5 | READY |
| F-SCHED-1 | 落盘失败不重复执行 | `save_tasks_at` 返回 Err | 该 slot `TASK_PERSIST_FAILED` 拒绝，不 spawn | scheduler.rs:425-427 | READY |
| F-SCHED-2 | 崩溃后不重放 | spawn 后 SIGKILL，重启 | 已 fired slot 不重复执行 | last_fired_at 先于 spawn 落盘（415-428） | READY(逻辑)/NOT_RUN(GUI) |
| F-SCHED-3 | 损坏 tasks.json | 写 `{not-json}` | 加载空、磁盘文件保留、不静默清空 | tasks.rs:926-928 | READY |
| F-SCHED-4 | 时钟回拨 | 系统时间回拨 | 记 clock_rewind，不追补/倒退（§4.5） | scheduler.rs:370-380 | READY |
| F-SHUT-1 | 重复关闭 | 多次调用 shutdown | 仅首次执行清理，后续 `already_shutdown` | shutdown.rs:248-253 | READY |
| F-SHUT-2 | 关闭后注册任务 | shutdown 后再 register | 被拒、不执行 | shutdown.rs:324-334 | READY |
| F-SHUT-3 | 关闭中部分失败 | 某任务 panic | 仍标记 `already_shutdown`，其余已执行 | shutdown.rs:531/541 | READY |
| F-DOC-1 | 损坏 doc-recovery.json | 写坏文件 | 空安全默认 + 备份，不崩溃 | 设计 §4.2(5)；待实现 | PROPOSED |
| F-DOC-2 | 重启不重跑 | 关闭有未保存 SQL 文档，重开 | 仅恢复文本/视图，不自动执行 | 设计 §4.2(6)；J1/J5/J6 | PROPOSED/NOT_RUN |
| F-LAY-1 | 布局损坏 | layout-state.json 损坏 | 回退默认布局、不白屏 | 设计 §4.2(5)；待实现 | PROPOSED |
| F-QUERY-1 | 取消竞态 | db_query 执行中 db_cancel | 结果不双写/不 panic；需 db_cancel 命令 | R2 审计 §4.2（G3）；当前 dead_code | PROPOSED |
| F-QUERY-2 | 查询超时 | 超过 timeout_secs | 中止、不无限挂起、不复用半开连接 | R2 审计 §4.3（G7） | PROPOSED |
| F-QUERY-3 | 写确认绕过 | confirm_write=true 但 SQL≠确认文本 | 拒绝（文本身份校验） | R2 审计 §4.1（G1） | PROPOSED |

> A6 与 A9 分工：A6 负责调度/查询/凭据/文档恢复侧注入（上表）；A9 负责原生 WebView 边界、debug/release 权限、远程页面威胁侧的独立评审（R2B 任务卡 §10）。禁止 A6 代签 A9 项、禁止笼统 exactly-once 外部副作用保证——F-SCHED-1/2 已显式采用"漏跑优于重跑"的 fail-closed 取舍。

---

## 6. 开放决策（OPEN_DECISIONS）

- **A4 请求合同未到位**：A4 截至本包 HEAD 仍为 R1（`971378c`，无 R2B 提交）。本包对连接/schema/SQL/取消/超时/历史/导出的函数级复用闭包以 A4 R1 映射为参考；A4 R2B 正式契约落地后，A6 的查询状态机（§2）与故障注入 F-QUERY-* 应据其更新 `conn_id`/`exec_id` 取消句柄契约。→ `WAITING_DEPENDENCY(A4-R2B)`。
- **多 SQL 文档独立结果**：当前 `useDatabaseStore` 单 `result` ref，不满足 J2"两份文档各自结果"。§4 设计建议引入 `doc-recovery.json` 的 `docs[]`，但多文档**结果**是否持久化需与 A5（数据库工作流）协商：蓝图第 93 行禁止"完整 SQL/输出全文跟布局落盘"，故结果行默认**不持久化**（仅 pendingSqlText 可选）。交 A5 定稿。
- **持久化命令归属**：§4.3 的新 Tauri 命令属产品代码，R2B 禁止 A6 落地；列入 §7 候选卡，由 A0 在 W19 授权后交对应实现 lane。
- **容量/淘汰**：`doc-recovery.json`/`layout-state.json` 需容量上限与淘汰策略（蓝图 §5「容量、淘汰」）；本包未定具体阈值，建议复用 `DbPool` 同款 `MAX_*` 思路，交 A1/A5 冻结。

---

## 7. 候选实现卡（PROPOSED_NOT_AUTHORIZED — 非开工许可）

**卡名**：`A6-R2B-layout-doc-persistence`
- 范围：新增 `save_layout_state`/`save_doc_recovery`/`load_doc_recovery` 三个 Tauri 命令（或合并为一个 `persist_workbench_state`），全部经 `session::atomic_write`；`src/stores/useLayoutStore.ts` 与 `useDatabaseStore.ts` 增加 `schema_version` 结构化状态与恢复 action（仅视图/文本，不存凭据/结果行）；新增 `scripts/check-workbench-persistence-logic.mjs` 校验"不存 password / 不存完整输出 / 损坏回退"。
- 前置：A1 壳层契约（蓝图 J1/J6）冻结布局字段；A4 R2B 连接契约可用。
- 步骤：① 定义 `WorkbenchPersist` 类型（domain.rs）；② Rust 命令 + 原子写 + ACL + check_invocation_source；③ 前端 store 恢复 action + 启动 `onMounted` 加载（App.vue:68）；④ 策略脚本；⑤ 故障注入 F-DOC-1/2、F-LAY-1。
- 测试：策略脚本断言脱敏；cargo test 覆盖 atomic_write 损坏回退；native 验收 J1/J6 重启接续（NOT_RUN 待 GUI）。
- 完成条件：重启可恢复布局与 SQL 文档文本/连接；凭据与结果行不在磁盘；损坏文件不白屏、不静默清空。
- 授权：未经 A0 在 W19 开放 slice，本卡 `PROPOSED_NOT_AUTHORIZED`。

---

## 8. 自检（SELF-CHECK，提交前）

- [x] 无事实伪装：所有 `scheduler.rs`/`tasks.rs`/`shutdown.rs`/`session.rs` 行号均来自本 worktree 实际 grep/read。
- [x] 无范围越界：仅写 `logs/research/M5-W18/A6-R2B-*.md` 与 `logs/checkpoints/A6-M5-W18-R2B-*.md`；未改 `src/`、`src-tauri/`、`scripts/`、ACL。
- [x] 无产品代码：§4/§7 设计为 PROPOSED_NOT_AUTHORIZED，未落地。
- [x] 无私密数据：未抄写任何 keyring/密码/凭据正文。
- [x] 工作树干净（除本包两文件）；rebase 到 `434e63f` 后 ahead 3。
- [x] 复用旧成果：引用 `38faa2d` R2 审计与 `971378c` A4 映射，未重写。
- [x] 依赖未齐项明确标记：A4 R2B 合同、多文档结果策略、持久化命令归属（§6）。
- 验证命令：`git diff --check`（PASS）、`grep -rn "password" src/stores/useDatabaseStore.ts`（仅 F2 注释，无落盘）= NOT_RUN 级静态核对；产品构建/GUI = NOT_RUN（研究包不跑全量构建，蓝图 §7）。
