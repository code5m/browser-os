# A1 · M5-W18-R 当前产品端到端基线研究报告

> Lane: A1（RESEARCH，仅基线，不实现）
> BASE = `origin/master` `78d2cfb`（docs(M5-W18): switch lanes to research-first blueprint）
> Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3` @ branch `codex/m5-w18-a1`
> 依据：`PARALLEL_COMMAND_BOARD.md` L1350-1437（M5-W18-R Research and Replication Blueprint Dispatch），A1 行 L1395
> 边界：只读研究；零产品代码改动；不 push。仅产出本报告 + checkpoint 至 `logs/research/M5-W18/`。

---

## 0. 输出头（board L1416 格式）

```text
LANE=A1
STATUS=PASS_WITH_DEBT
BASE=78d2cfb
HEAD=<本 worktree 研究 commit，见 A1-checkpoint>
REFERENCE_EVIDENCE=本地主仓库源码树（HEAD 78d2cfb）；WORKSPACE_IDENTITY.md；PARALLEL_COMMAND_BOARD.md L1350-1437；logs/checkpoints/M5-20260906/M5-14-debt-ledger.md；scripts/measure-build-metrics.py
FILES=logs/research/M5-W18/A1-product-baseline-20260908.md, logs/research/M5-W18/A1-checkpoint-20260908.md
SOURCE_MAP=src-tauri/src/*.rs（29 模块）, src/stores/*.ts（16）, src/components/*, src-tauri/permissions/*.toml, src-tauri/capabilities/*.json, scripts/check-*.py|mjs（49）
CLASSIFICATION=N/A（A1 为基线 lane，不做 COPY/ADAPT 判定；该判定属 A4/A7/A10 职责）
VERIFY=read-only：grep/wc/静态比对；未运行构建或测试（research-only 边界）
CHECKPOINT=logs/research/M5-W18/A1-checkpoint-20260908.md
MERGE_NOTES=A1 是 A0 集成顺序首位（A1→A2/A3→A4/A5/A6→A7/A8/A9→A10→A11→A0）；本报告 gap inventory 与 requirements checklist 为后续 10 个 lane 的强制输入
NEXT=A0 据本报告打开 W19 时，须先消 W17 残留债（W17-D1~D6）与 DbValue 三源漂移（见 §3.G4）
```

---

## 1. 研究范围与方法

A1 任务（board L1395）：*"Baseline the current product end to end: graph/database Rust modules, DTOs, commands, stores, panels, policies, tests, known debt, build-size budget, and locked authorities. Produce the canonical gap inventory and a requirements checklist that all other reports must answer."*

方法：对当前产品（HEAD `78d2cfb`）做静态端到端基线盘点。所有数据来自只读扫描（wc/grep/sed/静态比对），未运行 `npm run build`、未运行测试、未改动任何产品文件。本报告将三个上游复用目标（Obsidian / dbx / zvec-grep，见 board L1354-1358）映射为**当前产品能力 vs 目标功能**的规范缺口（§3），并给出**所有 A2-A11 必须回答的统一需求清单**（§4）。

---

## 2. 当前产品端到端基线（10 维度）

### 2.1 graph / database Rust 模块

| 模块 | 行数 | 关键符号 | 状态 |
|---|---|---|---|
| `graph.rs` | 733 | `GraphStore`(L185) / `GraphState`(L311) / `graph_query_impl`(L338) / `graph_node_get_impl`(L371) / `graph_stats_impl`(L380) / `load_snapshot`(L395) / `validate_graph_node/edge`(L164/173) | 后端完整；命令已注册（main.rs:1482-1484）+ ACL 放行（default-commands.toml:128-130） |
| `database.rs` | 1273 | `DbValue`(L122) / `DbQueryResult`(L150) / `DbConnectionConfig`(L911 在 domain.rs) / `DbPool`(L405) / `validate_config` / `validate_sqlite_path` / `detect_multiple_statements` / `resolve_timeout_secs` | 后端完整；`db_connect/db_query/db_disconnect` 已由 A4 落地（useDatabaseStore.ts:19 注释确认） |
| `domain.rs` | 2521 | `DbValue`(L1094, **`#[allow(dead_code)]`**) / `DbConnectionConfig`(L911) / `DbQueryResult`(L1110) / `GraphNode/Edge/NodeView/EdgeView/QueryRequest/QueryLimits`(L2042-2158) | DTO 集中定义处，**但含死代码 DTO** |

**关键发现 G1（graph 已落地）**：W18-R 目标是"从零引入 Obsidian 图能力"，但当前产品**已有** GraphStore 后端 + 前端消费层（见 §2.5）。A2/A3 研究的 Obsidian 语义须叠加在现有 GraphStore 之上，而非另起炉灶 —— 这是 A1 基线对 A2/A3 的核心约束。

### 2.2 DTOs（含重大漂移发现）

**关键发现 G4（DbValue 三重真源漂移，B8-1 深层根因）**：

| 真源 | 位置 | 变体定义 | serde | 状态 |
|---|---|---|---|---|
| 实际生效 | `database.rs:122` | `Null/Bool/I64/F64/Text/Binary{bytes:usize}` | `snake_case` | **运行时使用**（有 `byte_len()`） |
| 死代码 | `domain.rs:1094` | `Null/Bool/Int/Float/Text/BlobLen(u64)` | `snake_case` | **`#[allow(dead_code)]`，未使用** |
| 前端镜像 | `src/types.ts:601-607` | `{Null}/{Bool}/{Text}` 等 **PascalCase** | — | 前端消费 |

差异：变体名不一致（`I64` vs `Int`、`F64` vs `Float`、`Binary{bytes}` vs `BlobLen(u64)` 且 struct/tuple variant 不同）；`domain.rs` 版本标 `dead_code`（消费者 M4-2 结果组装，疑似历史残留）；前端 `types.ts` 用 PascalCase，与两处 Rust snake_case 均不匹配。→ A11 复跑已 CONFIRMED B8-1（DB 面板渲染错误），此处定位到**三源漂移 + 死代码 DTO** 的结构性根因。A10（移植账本）须将其列为"必须统一为单一真源"项。

### 2.3 commands（tauri::command）

- 总数：**137** 个 `#[tauri::command]`；ACL（`default-commands.toml`）放行 **137** 条（全量）。
- 注册分两段：主窗 `main.rs:1356` 段 **133** 个 + 宫格子窗 `main.rs:112` 段 **5** 个（`collect_selection/save_note/request_open_terminal/report_grid_load_failed/debug_log`）= 138？实测 ACL 137，差异来自 112 段 5 个中有 1 个（`debug_log`）可能不入 ACL 或不计 tauri::command。结论：**137 命令 = 133 主窗 + 4 宫格子窗（不含 debug_log）**，需 A11 在 integration 时精确核对一致性（已有 `check-command-set-consistency.py` 门禁）。
- 按前缀分类（主窗段）：tab(10) / plugin(8) / session(7) / list(7) / script(6) / term(5) / task(5) / snippet(4) / set(4) / grid(4) / get(4) / skill(3) / request(3) / report(3) / mcp(3) / m0(3) / graph(3) / git(3) / db(3) / create(3) / agent(3) … 覆盖浏览器/插件/会话/脚本/终端/任务/片段/网格/技能/资源/MCP/构建/图/Git/数据库/Agent。

### 2.4 stores（前端状态层）

16 个 store：`useAgentStore / useBookmarkStore / useBrowserStore / useDatabaseStore / useGitStore / useGraphStore / useHomeStore / useImagePreviewStore / useLayoutStore / usePluginStore / useResourceStore / useSessionStore / useSettingsStore / useSystemStore / useTaskStore / useWorkspaceStore`。

- `useGraphStore.ts`：310 行，消费 `graph_query/graph_node_get/graph_stats`。
- `useDatabaseStore.ts`：218 行，消费 `db_connect/db_query/db_disconnect`（注释 L19 确认 fail-closed 兜底）。

### 2.5 panels（前端视图层）

面板目录：`browser / graph / home / layout / plugin / shared / system / workspace`。
- **graph 消费层完整**：`src/components/graph/` 含 `GraphPanel.vue / GraphViewer.vue / NodeDetail.vue / EdgeDetail.vue / GraphFilter.vue`（5 组件）+ `useGraphStore.ts`。
- **database 消费层存在**：`src/components/workspace/DatabasePanel.vue` + `useDatabaseStore.ts`。
- **检索消费层缺失**：无 search/grep/find 面板或 store（见 §3.G3）。

### 2.6 policies（策略/验证脚本）

`scripts/check-*.py|mjs` **49** 个，构成事实上的回归门禁矩阵（命令集一致性、导航、剪贴板、home、graph、database、git、plugin、image、mcp、scheduler、script、security、a11y、markdown-xss 等）。其中 `check-command-set-consistency.py` 为 A11 在复跑中确认仍缺失"三源比对"后的补强目标（B12-02）。

### 2.7 tests

- **Rust `#[test]`/`#[tokio::test]`：470 个**（含 database/security/scheduler 等）。
- **前端单元测试：0 个**（package.json 无 vitest/jest/cypress/playwright；无 `*.test.ts`/`*.spec.ts`）。**关键缺口 G5**：前端质量仅靠 49 策略脚本（逻辑级 mjs）与人工/原生实跑，无自动化单测网。
- 验证入口：`scripts/measure-build-metrics.py`、`pre-merge.sh`（A11 口径 ALL_PASS，但 W17 三个新门禁 `check-home-*` 未被 pre-merge 覆盖，见 W17-D2 挂账）。

### 2.8 known debt（M5-14 债务账）

`logs/checkpoints/M5-20260906/M5-14-debt-ledger.md`（643 行，32 条债务行）。关键项：
- **历史挂账 D23-D26**（M4-1 遗留，仍未消）：D23 终端 GUI 实点、D24 吞吐基线未重采、D25 `on_channel_dead` 未 `wait`（僵尸 shell + 表项残留）、D26 历史未按字符封顶。
- **W17 残留债 W17-D1~D6**：D1 体积干净树复测（25.2% 上限，W17 实测 25.55% 超限需复测）、D2 pre-merge 未覆盖 W17 三新门禁、D3 A11 矩阵过期行、D4 活动条目录路径持久化、D5 原生视觉验收 headless BLOCKED、D6 B10-b MainArea 兜底 `v-else` 恒渲染（W17/A7 引入回归，CONFIRMED）。
- P0 级标记 4 处（数据丢失/死锁/安全绕过关键词命中）。

### 2.9 build-size budget

- 上限常量：`scripts/measure-build-metrics.py:39` `TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.2`（A0 为已验收键盘可达性修复一次性上调，warning 不得增加）。
- W17 实测：A11 修复前 **25.55%**（超限 +0.35pp，未抬上限）；A1 脏树全量 29.77%（多 lane 叠加口径，基线被脏树污染，不作判定值）。
- 现状 metrics json（`build-metrics-052b18a.json`，W15）仅记录 `chunk_over_500kb = False`，无完整体积百分比字段——**指标采集口径需 A11 在 W19 前补齐**（W17-D3）。

### 2.10 locked authorities（运行时权限锁定面）

- **capabilities（release）**：`capabilities/default.json` 仅 **4** 权限（`core:default` / `core:window:allow-create` / `browser-tabs:default` / `default-commands`）；`capabilities/browser-remote.json` **2** 权限。
- **dev-capabilities/main.json**：**4** 权限，仅 `#[cfg(debug_assertions)]` 动态注册，不在 `capabilities/` 下，release 无法自动包含（W18-R L1374 接受的 debug-only 方案）。
- **命令执行锁定**：`security_policy.rs` 含 `BLOCKED_LAUNCH_PROGRAMS`(12) / `BLOCKED_LAUNCH_WRAPPERS`(10) / `BLOCKED_INTERPRETERS`(12) 黑名单（L158-182），`is_blocked_launch_name` 拦截。
- **运行时权限 LOCKED**：board L7 "Runtime authority remains LOCKED" —— 插件调用/命令执行/动态加载/远端收藏调用均受 ACL 严格约束；C−A（bridge.ts skill/agent 11 条执行类契约占位）因 LOCKED 未注册，属预期态（A11 复跑 CONFIRMED）。
- **结论**：任何 W19 新功能（Obsidian/dbx/zvec-grep）的命令必须入 `default-commands.toml` + capability，且不得扩张运行时权限面（plugin invocation / dynamic loading / 远端收藏）。

---

## 3. Canonical Gap Inventory（规范缺口清单）

> 下列缺口为"当前产品能力 vs 三个上游目标功能"的结构性差距，是 A2-A11 研究的输入。

| # | 缺口 | 当前产品状态 | 上游目标 | 涉及 lane | 严重度 |
|---|---|---|---|---|---|
| **G1** | Obsidian 图语义叠加 | GraphStore 后端 + 前端 GraphPanel **已存在**（节点/边/基础查询） | wikilink/backlink/outgoing-link/alias/heading-block/tag/frontmatter/orphan/unresolved/本地图 filters/depth | A2/A3 | 高（须叠加非重写） |
| **G2** | dbx 完整 workbench | database.rs 有连接/查询/取消/池 + DatabasePanel 基础 | schema browser / editor tabs / 结果分页·筛选·拷贝·导出 / 历史 / SQL 风险分析 / 连接树 | A4/A5/A6 | 高 |
| **G3** | 代码检索后端 | **全仓零检索能力**（无 ripgrep/fts5/bm25/vector/zvec 引用） | managed ripgrep / FTS-BM25 / vector / hybrid-RRF / 增量索引 / 紧凑输出 | A7/A8/A9 | 极高（从零） |
| **G4** | DTO 单真源 | `DbValue` 三源漂移（database.rs 实用于 I64/F64/Binary；domain.rs 死代码 Int/Float/BlobLen；types.ts PascalCase）+ 其他 DTO 可能同类 | 统一单一真源 + 前端镜像逐字对齐 | A10/A11 | 高（已导致 B8-1） |
| **G5** | 前端单测网 | 0 个前端单测（无 vitest/jest）；仅 49 策略脚本 | 行为级单测覆盖新增 Vue 组件/store | A11（测试矩阵） | 中 |
| **G6** | 体积预算余量 | 上限 25.2%，W17 已达 25.55%（超限） | 新增三大功能须保持 ≤25.2% delta | A4/A5/A7/A10 | 高（硬约束） |
| **G7** | 权限面零扩张 | 运行时 LOCKED；ACL 137 全放行 | 新命令须入 ACL + capability，不扩张 LOCKED 面 | A2-A9/A10 | 高（硬约束） |
| **G8** | 债务未消 | M5-14 含 D23-D26 + W17-D1~D6，P0×4 | W19 前须消或显式 carry-forward | A0/A11 | 中 |

---

## 4. Requirements Checklist（所有 A2-A11 必须回答的统一清单）

> 每个下游 lane 的报告**必须逐条回答**下列项（board L1384 的 11 维 + A1 针对三对接面的具体化）。未答项视为研究不完整。

**R1. 当前产品 gap（必答）**：你研究的目标功能（Obsidian/dbx/zvec-grep）相对 §2/§3 当前产品能力的精确缺口是什么？复用现有 GraphStore/database.rs/检索层（G1/G2/G3）的哪部分？

**R2. 上游源映射（必答）**：精确文件路径/符号/函数/测试（本地 pinned 源：Obsidian vault `secondBrain/.obsidian`、dbx `research/dbx-src` @ Cargo.lock sha `c0a7be12…`、zvec-grep `research/zvec-grep-src` @ `5265395…`）。逐文件列出，标注 Apache-2.0（dbx/zvec）或行为-only（Obsidian，非源码 donor）。

**R3. 数据/控制流（必答）**：目标功能的数据流与控制流；如何接入现有 `bridge.rs` 命令注册 + `default-commands.toml` ACL + `types.ts` DTO 镜像（**禁止新增长期偏离的 DTO 真源，见 G4**）。

**R4. 持久化格式（必答）**：新增/复用哪些文件格式（graph snapshot / db 连接配置 / 索引 manifest）？密钥/DSN 如何不落明文（对照 `check-clipboard-persistence-logic.mjs` 与 `HOME_NO_SECRET_PERSIST` 已闭环范式）？

**R5. 并发/生命周期（必答）**：连接池/索引 watcher/检索 daemon 的并发模型；如何复用 `shutdown.rs`(667 行) / `grid_process.rs` 生命周期范式；daemon/MCP/模型下载**禁止在 W18-R 启动**（board L1381），仅设计。

**R6. 安全/隐私（必答）**：对照 `security_policy.rs` 黑名单与 LOCKED 运行时；credential/API-key/DSN 存储与脱敏（复用 `redactSecrets`）；远端 embedding/内容外发须显式授权（zvec-grep 信任边界，A9）；错误回显须接 `redactSecrets`（B11-2 未闭环，前端 0 处接力）。

**R7. 性能/容量（必答）**：行/字节/文件/索引大小的所有上限（复用 `DB_MAX_RESULT_BYTES` 范式）；体积 delta 必须 ≤25.2%（G6）；检索延迟/内存/索引体积须 benchmarking（A8 合成语料，禁装依赖）。

**R8. 依赖/许可证（必答）**：每个上游依赖的 license/native footprint；Apache-2.0 须记录 NOTICE/modified-file 声明（A10）；Obsidian 行为-only 不得复制专有代码/图标/品牌（board L1365）。

**R9. 目标映射（必答）**：移植到本产品的哪个模块/组件/store/面板？最小可移植单元是什么（board L1366 禁整库复制）？

**R10. 测试复用（必答）**：上游哪些测试可移植？前端新增部分如何补单测（G5，当前 0 前端单测）？如何接入 49 策略脚本矩阵 + `pre-merge.sh` + `check-command-set-consistency.py`？

**R11. 未解问题（必答）**：列出无法静态判定的项，标注需 A0/A8 真机验证或需其他 lane 先交付的依赖。

**R12. 分类判定（A4/A7/A10 必答，其余 N/A）**：每个拟定功能分类 `COPY / ADAPT / REIMPLEMENT_FROM_BEHAVIOR / DEFER / REJECT` 并给理由（board L1383）。

---

## 5. 跨 lane 一致性约束（W18-R 边界）

1. **research-only**：A1 及所有 lane 在 W18-R **不写产品代码、不 push**（board L1378-1381）。W19 才实现。
2. **worktree 隔离**：A1 在 `m5-w18-a1`；A0 集成用 binary patch，**禁止 reset/clean 脏工作树**（W17-D5 教训）。
3. **license**：dbx/zvec-grep Apache-2.0 可 COPY/ADAPT（经 A10 记录 provenance）；Obsidian 仅行为复制。
4. **ACL/容量硬约束**：新命令必入 ACL + capability；体积 delta ≤25.2%；运行时权限面零扩张。
5. **DTO 单真源**：任何新增 DTO 必须对齐现有 `database.rs`（生效真源），**消除** `domain.rs` 死代码 DTO 与 `types.ts` PascalCase 漂移（G4）。

---

## 6. 未解问题（A1 视角）

- U1：`main.rs:112` 段 5 命令中 `debug_log` 是否计入 137 ACL？需 A11 用 `check-command-set-consistency.py` 精确核对三源差集。
- U2：database 专用面板 `DatabasePanel.vue` 的功能完整度（是否仅 M4 基础，缺 dbx 级 schema browser/导出）需 A5 核实。
- U3：体积 metrics json 字段不完整（仅 `chunk_over_500kb`），W19 前需 A11 补齐百分比采集（W17-D3）。
- U4：当前产品 graph 的 snapshot 格式（`load_snapshot` 路径）是否与 Obsidian `graph.json` 兼容，需 A2 比对。
