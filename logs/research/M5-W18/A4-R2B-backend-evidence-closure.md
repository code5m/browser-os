# A4 — M5-W18-R2B 后端架构证据闭环报告

> **Lane**: A4 · `M5-W18-R2B` (dispatch `M5-W18-R2B-TASKS-20260908.md` §5) · 复核 `A0-M5-W18-R1-audit-20260908.md` 与 `A0-M5-W18-R2B-dispatch-20260908.md`
> **Role**: 补 R2；核对 dbx 身份；确立 `DbValue`/数据库契约唯一源；给出连接/schema/执行/取消/历史/导出的函数级复用闭包与多 SQL 文档隔离契约。
> **Research-only**: 不修改 `src/` `src-tauri/` `scripts/`，不安装依赖，不接真实 DB/IPC，不 push。仅报告 + checkpoint。
> **WORKDIR**: `/home/ainfinit/.codex/worktrees/m5-w18-a4/mvp-browser-os-v3`
> **BRANCH**: `codex/m5-w18-a4`（已 `git fetch origin && git rebase origin/master`，干净，ahead 1）
> **BASE**: `434e63f3fc2cb9457d9fc15fbb3014ce4d9a48e6`（rebase 后 origin/master 顶端）
> **Prior A4 (R1)**: `971378c` → rebase 后 `b737e5d`（保留，未改）
> **Reference**: `/home/ainfinit/Documents/极智简单/V3/research/dbx-src`（Apache-2.0，本地只读副本）

---

## 1. 复用旧成果清单（保留，未重写）

| 旧文件 | 状态 | 本包处理 |
|---|---|---|
| `logs/research/M5-W18/A4-dbx-backend-architecture-map.md`（R1，`971378c`） | 保留 | §3/§10/§12 的 dbx 源映射与分类结论继续有效，仅按本包 §2 更正计数与 COPY 可行性。 |
| `logs/research/M5-W18/A4-checkpoint.md`（R1） | 保留 | 旧 checkpoint 不再更新；本包新增 `A4-M5-W18-R2B-checkpoint.md` 作为 R2B 权威 checkpoint。 |
| dbx 源 `crates/dbx-core/src/{csv_export,sql_risk,query_cancel,history}.rs` | 保留为参照 | 本包对其关键符号做函数级闭包追踪（§6）。 |

---

## 2. 更正日志（对 R1 audit 与 R1 报告的据实修正）

### C1 — 数据库测试计数 22 → 23（声明数）
- **R1 audit #2** 写 "当前 `src-tauri/src/database.rs` 已包含 22 `#[test]`"。
- **实测**：`grep -c '#\[test\]' src-tauri/src/database.rs` = **23**，分布在 814/825/848/874/881/890/900/914/926/937/956/990/1010/1034/1062/1093/1121/1154/1188/1215/1235/1252/1262。
- **来源标签**：`CURRENT_PRODUCT`（OBSERVED_BEHAVIOR，grep 实跑）。
- **声明数 vs 执行数分离**：声明 `#[test]` = **23**；**执行通过数 = NOT_RUN**（本包为研究 lane，未在本次 `cargo test`，见 §9）。`A0-M5-W18-R2B-dispatch` 第 40 条独立确认 "实际有 23 行独立 #[test]" 且同样 "本轮没有重跑 cargo"，与本文互证。
- 修正本包头部 R1 报告 §2 "Result DTO … ✅ Implemented" 与 §13 "repo has zero DB-layer tests" 为 "repo has **23** DB-layer unit tests in `database.rs`"。

### C2 — `DbValue` 漂移由 "风险" 升级为 **已确认根因**（B8-1）
- **R1 audit #3** 仅称 "two Rust definitions with different variant names … duplicate-contract risk"。
- **实测根因**（详见 §5）：`database.rs::DbValue` 带 `#[serde(rename_all="snake_case")]`，线格式为 `i64`/`f64`/`binary`；而 `types.ts` 与 `domain.rs::DbValue` 期望 `int`/`float`/`blob_len`。前端 `decodeDbValue`（`src/utils/dbUi.ts:273`）只识别 `int`/`float`/`blob_len`，故**每个数值/二进制单元格都落入兜底分支渲染成原始 JSON**。这是 B8-1 的真实结构性根因，非仅"风险"。
- 另发现 **`DbQueryResult` 字段错位**：后端（database.rs:148-162）= `{columns, rows, row_count, truncated, field_truncated, elapsed_ms, query_id}`；TS（`types.ts:633-642`）= `{query_id, columns, rows, row_count, truncated, field_truncated, limit_hit, elapsed_ms, state}`。TS 多出 `limit_hit`/`state` 两个后端不产出的字段 → 单源化须一并收口。
- **来源标签**：`CURRENT_PRODUCT`（OBSERVED_BEHAVIOR，读源码 + 注释 `types.ts:600-602` 自述 "导致…落进 decodeDbValue 兜底分支" 直接印证）。

### C3 — dbx 身份确认（R2B §5 要求）
- 本地 `dbx-src/README.md` 指向 `github.com/t8y2/dbx`，自述 "90+ databases in 20 MB. Desktop, Docker, CLI, built-in AI assistant, and MCP Server"；workspace 含 `dbx-core/dbx-web/dbx-mcp/dbx-cli`；`dbx-core/Cargo.toml` `license = "Apache-2.0"`。
- **结论**：本地参照源身份 = **`t8y2/dbx`**（独立 Rust 开源数据库管理器，Apache-2.0），**不是 JetBrains IDEA**，也未见证据它是用户所用 IDEA 插件。`WORKBENCH_BLUEPRINT-20260908.md:18`、`A0-M5-W18-R2B-dispatch:40` 同此判定。
- **IDEA Database Tools 身份 = UNKNOWN**（仅能列出公开文档 `jetbrains.com/help/idea/relational-databases.html` 作体验参照，不能据此认定代码来源）。按 R2B 卡："未知插件身份阻塞'精确复刻'声明，不阻塞通用数据库流程研究" → 通用查询 UX 研究继续，精确复刻声明保持 UNKNOWN。

### C4 — COPY 候选 `push_csv_escaped` 函数级闭包与编译可行性（补 R1 缺口）
- R1 audit 批 "COPY candidates lack function-level dependency closure and compile feasibility"。本包补齐：
- `csv_export.rs:33-37 push_csv_escaped(out, value)` = `out.push('"'); push_csv_escaped_content(...); out.push('"');`
- `csv_export.rs:21-29 push_csv_escaped_content(out, value)` = 纯字符串遍历，仅 `std`，**零外部依赖、零 IO**。
- 注意 `csv_export.rs` 顶部 `use crate::connection/query/sql_dialect…` 仅被文件内**其它**函数使用；这两个转义函数是**自包含纯函数**，移植时只拷这两个 fn 即可，**不引入 dbx 的 connection/query 依赖闭包**。
- **编译可行性**：`std`-only，可直接落入新建 `src-tauri/src/db_export.rs`，**不增加 crate 依赖**。→ COPY 成立且可行（R1 仅声明 COPY 未给闭包，本包补全）。

### C5 — `sql_risk::mcp_sql_has_forbidden_database_switch` 不可整体 ADAPT（修正 R1 分类）
- R1 报告 §10/§11 把 `sql_risk.rs` 标 ADAPT（"diff vs repo"）。本包追踪其**真正缺口函数** `mcp_sql_has_forbidden_database_switch`（`sql_risk.rs:732`）的依赖闭包：
  - 依赖 `crate::query_execution_sql::classify_search_engine_query_risk`、`normalize_dialect`、`resolve_dialect`、`sql_has_use_statement`，后者又依赖 dbx 自有 `Parser`/AST（`sql_parser/`、`sql_dialect/` 25+ 文件）。
  - 即：要 ADAPT 这一个 "禁止 USE 切库" 缺口，必须连带引入 dbx 的**整套 SQL 解析器**。
- **修正建议**：仓库 `security_policy.rs` 已有自主 `classify_sql_risk`/`is_production_database`；缺口仅 "USE 语句切换库" 一项。建议 **REIMPLEMENT_FROM_BEHAVIOR** 一个针对 `SupportedDb` 方言的轻量 `has_use_statement`（正则/词法，~30 行，零解析器依赖），**而非** ADAPT 整个 `sql_risk` 模块。A10 做移植范围判定时据此收窄。→ 修正 R1 的 `ADAPT` 为 `REIMPLEMENT_FROM_BEHAVIOR (small)`，缩小闭包。

---

## 3. 证据标签与来源（R2B §1 要求）

| 标签 | 条目 | 来源 SHA/文件/符号 |
|---|---|---|
| `CURRENT_PRODUCT` | 23 `#[test]`、DbValue 三定义、DbQueryResult 字段 | `database.rs:120-162,814-1262`；`domain.rs:1094-1114,2425-2449`；`types.ts:600-642`；`src/utils/dbUi.ts:273-291`；`bridge.rs:6040-6049`（返回 `crate::database::DbQueryResult`） |
| `REFERENCE_SOURCE` | dbx 后端源映射、COPY/ADAPT 闭包 | `dbx-src/crates/dbx-core/src/{csv_export.rs:21-37, sql_risk.rs:679-740, query_cancel.rs:1-100/170-260, history.rs:1-95}`（本地只读副本，Apache-2.0） |
| `OFFICIAL_DOC` | dbx 身份、IDEA 参照 | `dbx-src/README.md`（github.com/t8y2/dbx）；`WORKBENCH_BLUEPRINT-20260908.md:18,22` |
| `OBSERVED_BEHAVIOR` | B8-1 兜底渲染 | `types.ts:600-602` 自述 + `decodeDbValue` 只匹配 int/float/blob_len |
| `INFERENCE` | IDEA 身份 UNKNOWN；通用 UX 继续 | `A0-M5-W18-R2B-dispatch:40` |
| `DESIGN_DECISION` | DbValue 单源化、多 SQL 文档隔离、S0/S2 候选卡 | 见 §5/§7（标 `PROPOSED_NOT_AUTHORIZED`） |

---

## 4. 用户体验参照身份记录（R2B §5）

```
REFERENCE_IDENTITY_RECORD (A4)
- local_source: t8y2/dbx
    evidence: dbx-src/README.md (github.com/t8y2/dbx, "90+ databases in 20MB, Desktop/Docker/CLI/AI/MCP"),
              dbx-src/Cargo.toml workspace {dbx-core, dbx-web, dbx-mcp, dbx-cli},
              dbx-core/Cargo.toml license=Apache-2.0
    classification: independent Rust OSS database manager (NOT JetBrains IDEA)
- idea_database_tools: UNKNOWN
    evidence: only public doc URL jetbrains.com/help/idea/relational-databases.html (experience reference, not code source)
    decision: cannot assert precise replication of IDEA; generic query UX research continues
- reuse stance: dbx is architecture/behavior reference; transplant only under A10 provenance + NOTICE; never copy dbx no-ACL habit or FileSecretStore
```

---

## 5. `DbValue` / `DbQueryResult` 唯一源与混合旧数据处理设计（核心）

### 5.1 现状（三源 + 一消费者）
- **S1 真源（线序列化）**：`database.rs::DbValue`（`I64/F64/Binary{bytes}`，`rename_all=snake_case` → 线标签 `i64`/`f64`/`binary`）。`db_query` 经 `bridge.rs:6047` 返回此类型，是**唯一实际落线**的定义。
- **S2 死代码**：`domain.rs::DbValue`（`Int/Float/BlobLen`，snake_case → `int`/`float`/`blob_len`），`#[allow(dead_code)]`，但**已带断言测试**（`domain.rs:2425-2449` 验证 `{"int":7}`/`{"float":1.5}`/`{"blob_len":1024}` 且禁止 PascalCase）。
- **S3 消费契约**：`types.ts:603-610` `DbValue` 与 `decodeDbValue`（`dbUi.ts:273-291`）只识别 `int`/`float`/`blob_len` → **与 S1 线格式不匹配** → B8-1。
- **S4 结果结构错位**：`DbQueryResult` 后端缺 `state`/`limit_hit`（TS 多出两字段）。

### 5.2 单源决策（`PROPOSED_NOT_AUTHORIZED`）
**采用 `domain.rs::DbValue`（`Int/Float/BlobLen`，线标签 `int`/`float`/`blob_len`）作为唯一真源**：
1. 它已通过契约测试、标签与前端 `types.ts`/`decodeDbValue` 完全一致 → 采用它可**零前端改动**修复 B8-1。
2. `database.rs` 删除本地 `DbValue` 副本，改为 `use crate::domain::DbValue`；移除 `domain.rs` 上 `#[allow(dead_code)]`（被 `database.rs` 实际引用后消除）。
3. `DbQueryResult` 同样单源化：以 `database.rs` 当前字段为基准，补 `state: DbQueryState`/`limit_hit: DbLimitKind` 两字段使其与 `types.ts` 对齐（或反向让 TS 删多余字段，二选一；推荐补齐后端字段，因 `state` 对取消/超时 UI 有用）。
4. 线契约锁定为：`int`/`float`/`blob_len`/`text`/`bool`/`null`（BLOB 仍只回长度，沿用 M4-1.c §2 不回传字节）。

### 5.3 混合旧数据处理
- 当前**无持久化历史**（R1 报告 §2：`history` 模块 absent），故尚无落盘旧 `DbValue` 数据需迁移。
- 锁定规则：自本契约生效后，所有 `db_query` 返回与（未来）历史持久化均使用 `int`/`float`/`blob_len` 单一线格式；若 W19 引入历史持久化，写入方必须采用同一 `domain.rs::DbValue` 序列化，避免再次分裂。
- 不做"按变量名直接删定义"：保留 `domain.rs` 定义（带测试）作为真源，`database.rs` 改为引用而非删除其自身定义后另起炉灶。

### 5.4 验收（供 A5/A6/A10）
- A5：可据 `int`/`float`/`blob_len` 契约写面板（无需兜底分支）；`decodeDbValue` 维持不变即可正确渲染。
- A6：取消/超时状态用 `DbQueryResult.state`（`completed|cancelled|timeout|failed`）+ `query_id` 固定句柄（见 §7）。
- A10：移植范围判定以 `domain.rs::DbValue` 为单源，不再把 `database.rs` 旧 `I64/F64/Binary` 视为独立可移植单元。

---

## 6. 连接/schema/执行/取消/超时/历史/导出 函数级复用闭包与目标映射

| 能力 | dbx 参照（file:line） | 函数级闭包 | 目标映射（复用既有同步驱动，不引入第二运行时） | 可行性 |
|---|---|---|---|---|
| CSV 转义 | `csv_export.rs:21-37` | 纯 `std`，零闭包（见 C4） | 新建 `src-tauri/src/db_export.rs` 拷两 fn | ✅ COPY 可行 |
| SQL 风险（缺口） | `sql_risk.rs:732 mcp_sql_has_forbidden_database_switch` | 依赖 `query_execution_sql`+解析器（见 C5） | 轻量 `has_use_statement` REIMPLEMENT | ✅ 小闭包 |
| 生产判定 | `production_safety.rs:98/184` | 纯函数；仓库已有 `is_production_database` | DIFF 多语句 targeting 缺口 | ✅ ADAPT(diff) |
| 取消注册表 | `query_cancel.rs:83-92,170-256` | `Arc<Mutex<HashMap<exec_id,RunningTask>>>` + `interrupt` 与 task **同锁同生命周期**（关键模式） | 同步版 `Arc<Mutex<HashMap<String, SyncRunningTask>>>`：`SyncRunningTask{ flag: Arc<AtomicBool>(复用 QueryCancel L1), interrupt: Option<Box<dyn Fn()+Send>>, meta }`；`register_interrupt`/`cancel` 同锁；grace 30min 用 `std::thread` 清扫 | ✅ ADAPT 可行（去 tokio） |
| 历史 | `history.rs:5-95` | `HistoryEntry`+`MAX_HISTORY=1000`+`read_all(path)`（serde） | REIMPLEMENT 落在 workspace（`Artifact`/`ImageRef` 同模式）JSON 或 workspace sqlite；结构同形 | ✅ 纯 serde |
| schema 发现 | `schema.rs` | async info-schema 查询 | REIMPLEMENT 同步 rusqlite/mysql/postgres 查询镜像行为 | ✅ |
| 取数 | `query_execution_sql.rs` | async fetch+paging | REIMPLEMENT 同步 fetch（仓库最大缺口） | ✅ 但工作量大 |
| 连接注册表 | `connection.rs:1182 ConnectionRegistry` | `Storage`+plugin/agent dirs | 仓库仅需 `HashMap<conn_id, SyncConn>`（keyring `db:<conn_id>` 已存在），不做 plugin/agent | ✅ 最小 |

**硬约束重申**（R1 报告 §1）：不引入 tokio/async（M4-1.a）；不引 dbx `[patch.crates-io]` 的 gaussdb/mysql_async fork（用标准同步 `rusqlite`/`mysql`/`postgres`）；不引 `FileSecretStore`/加密文件凭据（用 keyring）；不复制 dbx 无 ACL 习惯（保持 `default-commands.toml`）。

---

## 7. 多 SQL 文档隔离契约 + S0/S2 候选实现卡（`PROPOSED_NOT_AUTHORIZED`）

### 7.1 多 SQL 文档隔离契约（A5 消费、A6 画取消竞争所需）
- 每个 SQL 文档持有独立：`{ conn_ref: String, active_query_id: Option<String>, result: Option<DbQueryResult>, cancel_handle: Option<Arc<AtomicBool>> }`。
- **固定请求 ID 模型**：`db_query` 永远返回 `query_id`（`database.rs:161` 已存在）；`db_cancel(query_id)` 定位（R1 报告 D-A4-3：命令名待 A0/M4-3.a 冻结，契约先用 `query_id` 参数）。
- **隔离规则**：结果到达时仅当 `result.query_id === doc.active_query_id` 才渲染；切换文档时旧请求的结果若以新文档 `active_query_id` 到达则丢弃 → **旧请求不覆盖新文档**（R2B §5 "切换文档旧请求不得覆盖新文档"）。
- **取消竞争（A6 所需）**：`cancel` 置 `QueryCancel` L1 标志 +（L3）`interrupt` 闭包；UI 显示 `state=cancelled` 当且仅当 `result.state==cancelled && result.query_id===doc.active_query_id`；一次完成的 `query_id` 不可被另一文档的取消误伤（按 `query_id` 而非 `conn_id` 定位）。

### 7.2 S0 候选卡（现有契约与可运行基线）
```
SLICE=S0  STATUS=PROPOSED_NOT_AUTHORIZED  OWNER=A4(+A6/A9/A11 实证)
GOAL: DbValue/DbQueryResult 单源化；确认 release 无 Vite 依赖、debug IPC 正确；保留调度并发/崩溃行为
FILES: src-tauri/src/database.rs (删本地 DbValue, use domain::DbValue), src-tauri/src/domain.rs (去 dead_code),
       src/types.ts (已对齐, 仅补 state/limit_hit 或反向), src/utils/dbUi.ts (不变)
STEPS: 1) domain.rs::DbValue 作为单源并去 allow(dead_code); 2) database.rs 改引用; 3) DbQueryResult 补 state/limit_hit;
       4) cargo test database (23 用例应全绿); 5) npm 检查 dbUi decode 不再兜底
TESTS: database.rs 23 #[test]; domain.rs:2425-2449 标签断言; 新增 db_query→TS round-trip 契约测试
COMPLETION: B8-1 修复、线契约锁定 int/float/blob_len、前端 decode 零兜底
BLOCKED_BY: 无 (A4 证据已齐); 但合入需 A0 slice-opening 记录 (W19=CLOSED 期间禁止产品代码)
```

### 7.3 S2 候选卡（数据库日常闭环）
```
SLICE=S2  STATUS=PROPOSED_NOT_AUTHORIZED  OWNER=A5 (依 A4 契约)
GOAL: 连接树 + 独立 SQL 文档 + 查询/取消/结果
FILES: src/components/workspace/DatabasePanel.vue, src/stores/useDatabaseStore.ts, src/utils/dbUi.ts
       (后端 db_query/db_cancel 契约由 A4 §7.1 定义; 取数通道 S2 后端实现属 W19 编码切片, 不在本 R2B)
STEPS: 1) 每文档独立 state (§7.1); 2) 连接树渲染 SupportedDb; 3) 中央结果区按 active_query_id 渲染;
       4) 取消按钮置 doc.active_query_id 的 L1 标志; 5) 错误/空/断线/大结果/长字段/NULL/二进制长度 分支
TESTS: scripts/check-database-ui-logic.mjs 扩展 (J2); 两文档互相不覆盖的回归
COMPLETION: 至少两份 SQL 文档相互独立; 查询→结果→取消→重试完整
BLOCKED_BY: S0 契约 + S1 (若存在) + A4/A5/A6 一致结论
```

---

## 8. 验收对照（R2B §5 验收项）
- ✅ A5 能据契约写界面：§5.2/§7.1 给出 `int`/`float`/`blob_len` 与 `query_id`/`state` 契约。
- ✅ A6 能用固定 `query_id` 画取消竞争：§7.1 固定请求 ID 模型 + L1/L3 语义。
- ✅ A10 能逐个判断移植范围：§2(C5)/§6 把 `sql_risk` 从 ADAPT 收窄为轻量 REIMPLEMENT，并给出每单元闭包与可行性。
- ⚠️ "精确复刻 IDEA" 声明：因 IDEA 身份 UNKNOWN（§4），该声明保持阻塞；不阻塞通用数据库流程研究。

---

## 9. 未运行 / 未决项（明确标记）
- **cargo test 执行数 = NOT_RUN**：本包研究 lane 未重跑 `cargo test`；声明 `#[test]`=23（C1）。旧报告 "0 DB tests" 已据 C1 更正。
- **IDEA 身份 = UNKNOWN**（§4）：未联网核验，仅列公开文档 URL 作体验参照。
- **`db_cancel` 命令名 = 待 A0/M4-3.a 冻结**（R1 D-A4-3）：契约已用 `query_id` 参数规避，命名待定。
- **async-vs-sync**：维持仓库 M4-1.a 禁止 tokio 契约（§6 全部同步端口）。
- **历史/导出/取数**：属 W19 编码切片，本包仅给契约与闭包，不含产品代码。

---

## 10. 终态输出块
```
LANE=A4
DISPATCH=M5-W18-R2B
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a4/mvp-browser-os-v3
BRANCH=codex/m5-w18-a4
BASE=434e63f3fc2cb9457d9fc15fbb3014ce4d9a48e6
HEAD=<set on commit of this report + checkpoint>
CONSUMED_PEERS=A1 a54eed1 (baseline, read-only reference); A4 prior 971378c/b737e5d (own R1)
FILES=logs/research/M5-W18/A4-R2B-backend-evidence-closure.md, logs/research/M5-W18/A4-M5-W18-R2B-checkpoint.md
       (prior A4-dbx-backend-architecture-map.md, A4-checkpoint.md preserved)
CORRECTIONS=C1 test 22->23 (declared; executed NOT_RUN); C2 DbValue drift -> confirmed B8-1 root cause;
            C3 dbx identity = t8y2/dbx (IDEA UNKNOWN); C4 push_csv_escaped closure+feasibility proven;
            C5 sql_risk gap reclassified to small REIMPLEMENT
VERIFY=read-only static mapping + grep (23 #[test]); cargo test NOT_RUN; no DB/IPC connection
PROPOSED_SLICES=S0 (DbValue single-source, PROPOSED_NOT_AUTHORIZED), S2 (multi-doc DB loop, PROPOSED_NOT_AUTHORIZED)
OPEN_DECISIONS=db_cancel name freeze (A0/M4-3.a); IDEA identity UNKNOWN blocks precise-replication claim only
NEXT=W19: A0 opens slice with explicit record; A5 consumes §7.1; A6 draws cancel race; A10 ledger per §2(C5)/§6
NO_PRODUCT_CODE=true
NO_PUSH=true
```
