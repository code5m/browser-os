# A1 · M5-W12 reconciliation checkpoint（2026-09-07 20:30 CST · Lane A1 · 文档对账 + W11 PUSHED/W12 ACTIVE 标注 + 图谱卡 W12 三命令范围收敛 · 不 push）

> **STATUS**：DRAFT（待 A0 W12 拣入期合并定）
> **LANE**：A1（docs-only reconciliation）
> **WAVE**：M5-W12 · **Graph Live-Query Readonly Dispatch**（`PARALLEL_COMMAND_BOARD.md` L176-227，Added 2026-09-07 20:30 CST by A0）
> **BASE**：`269269a`（origin/master HEAD · A0 W11 拣入完成 · `269269a feat(M5): integrate W11 MCP stdio dry-run hardening`）
> **HEAD**：工作树（A1 范围 13 文件改动 + 1 new checkpoint + 1 new patch；其他 lane 工作树 = 由各 lane 负责，A1 不动）
> **A1 W12 任务**（`PARALLEL_COMMAND_BOARD.md` L192 A1 行）：*START DOCS ONLY* · *Reconcile W11 as accepted after A0 push and mark W12 active. Update M5 graph cards so W12 scope is exactly three read-only graph commands plus UI consumption.* scope = `PARALLEL_COMMAND_BOARD.md` + 3 主文档 + `logs/checkpoints/M5-20260906/*.md` + `logs/checkpoints/A1-M5-W12-*.md`；Must Deliver = **One reconciliation checkpoint; no product code**。
> **整包交付结束**：本文件 + `logs/checkpoints/Lane-A1-M5-W12-reconciliation-20260907-2030.patch` 整包；不 push；A0 W12 拣入期合并策略同 W11（A1 W11 整包已 A0 拣入 → A1 W12 整包合并拣入避免 A0 分两次消）。

---

## 0. 一句话

按 `PARALLEL_COMMAND_BOARD.md` L176-227（**M5-W12 Graph Live-Query Readonly Dispatch**，Added 2026-09-07 20:30 CST by A0）的 A1 行指令 *"Reconcile W11 as accepted after A0 push and mark W12 active. Update M5 graph cards so W12 scope is exactly three read-only graph commands plus UI consumption"*，A1 在 W12 仅做文档对账与 W11→W12 状态迁移 + 图谱卡范围收敛：**(a)** 工作树 + origin/master 同步 + 拣入现状检视（HEAD = `269269a` 含 A0 W11 拣入完成；本地工作树干净，A1 W11 整包已被 A0 合并拣入）；**(b)** `M5-0-overview.md` 标题链追加 *W11 reconciliation + W11 PUSHED + W12 active* + 头部时间戳链追加 *W11 拣入（`269269a` 回填）+ W12 active* 两行 + L20 基准链追加 *W11 拣入* 行 + 末尾新增 `[W11 reconciliation]` 段 + `[W12 active]` 段（含 **W12 runtime surface 锁定状态表 11 行**，其中 **Graph live-query command 由 W10/W11 的 🔒 LOCKED 翻为 W12 的 🟢 OPENED（narrow·read-only）**）；**(c)** **`M5-8-graph-store-query.md` 追加 `[W12 scope supersession]` 段** —— 收敛 stale 的 M5-8 原始 9 命令 + SQLite DDL 设计为 **W12 真实落地 = 3 条只读命令（内存 `GraphStore`）**（A7 W11 卡 §2 L61 明确 *"M5-8 文档更新归 A1/A0 文档整合"*，本包承接）；**(d)** `M5-7` / `M5-9` 图谱卡追加 W12 范围标注（M5-7 = 抽取器仍不在 W12；M5-9 = A8 W12 UI 消费 3 命令 + `GRAPH_COMMANDS_AVAILABLE` 翻 true）；**(e)** `M5-10/11/12` 三张插件卡头部追加 *W11 PUSHED* + *W12 ACTIVE* 状态行；**(f)** `M5-13` 头部加 W11 PUSHED + W12 ACTIVE 状态行 + 末尾 `[W12 verification scope]` 段（**W12 验证矩阵 6 FAC** + W12 硬停验证必跑 8 条）；**(g)** `M5-14` 标题追加 *W11 reconciliation + W11 PUSHED + W12 active* + 顶部加 *W11 PUSHED* / *W12 ACTIVE* 行 + 末尾新增 `[W12 active]` 段（runtime 债表 + W12 增量债）；**(h)** 三份主文档 L1 追加 A1 20:30 W12 update 行（与 A0 20:30 W12 派发行并列）；**(i)** `PARALLEL_COMMAND_BOARD.md` L6/L7 修订为 W11 PUSHED（`269269a`）+ W12 active；**(j)** 写本 checkpoint + patch；不写产品代码；不重写 §1~§11 决策史；不移动 `NEXT`；不提交 / 不 push。

---

## 1. W12 一行 prompt

A1 W12 = 文档对账（docs-only reconciliation）：reconcile W11 as PUSHED + mark W12 active + **把 W12 图谱范围收敛为「3 条只读命令 + UI 消费」**（M5-8 的 9 命令 + SQLite 旧设计标注为 superseded）。

---

## 2. W12 范围冻结：**恰好 3 条只读命令 + UI 消费**

> 真相源 = `logs/assist/A7-M5-W11-graph-w12-impl-plan-20260907-1019.md` §1.1（A7 W11 交付的 W12 实施卡片）+ board L198（A7 W12 任务行）+ board L180（W12 goal）。

### 2.1 W12 三条只读命令（IN SCOPE）

| 命令 | 用途 | 只读 | 写 store | 引依赖 |
|---|---|---|---|---|
| `graph_query` | 从 `start_id` 出发的有界子图查询（节点 + 其间边，**删 props**）；出参含 `truncated: bool` + `applied: {depth, limit}` | ✅ | 否 | 无 |
| `graph_node_get` | 单点详情（Skill/Agent id 走 **64-hex** 校验），缺失返回 `null` | ✅ | 否 | 无 |
| `graph_stats` | 容量概览（节点/边计数 + 容量 + 黄牌信号） | ✅ | 否 | 无 |

**架构裁定**（A7 W11 卡 §3 L65，修正 W10 §2.5 的含糊）：
- 3 条命令**定义在 `bridge.rs`**（与全仓其他命令一致），调用 `crate::graph::*` pure helper + `check_invocation_source`（同在 `bridge.rs`）
- `graph.rs` **不 import `crate::bridge`**（守 `GRAPH_NO_SECOND_PATH`），仅提供 helper + `GraphState` newtype + `GraphError::code()`
- 这样既不破坏 `GRAPH_NO_SECOND_PATH`，又满足 source check 同源

**输出脱敏**：`graph_query` / `graph_node_get` 返回 `GraphNodeView` / `GraphEdgeView`（仅 `{id,kind,label}` / `{from,to,kind}`），**不含 `props`**；`GraphNodeView::from(GraphNode)` 显式丢弃 `props`（编译期保证）。

### 2.2 W12 明确 OUT OF SCOPE（后续独立 wave，不阻塞 W12）

| 项 | 说明 | 归属 |
|---|---|---|
| `graph_node_upsert` / `graph_node_delete` / `graph_edge_upsert` / `graph_edge_delete` | M5-8 原始 9 命令设计中的**写**命令 | 后续 wave |
| `graph_node_list` / `graph_edge_list` | M5-8 原始设计中的**全量列**命令 | 后续 wave |
| `graph_export`（graphml） | M5-8 原始设计中的**导出**命令（原始设计即"首期返回暂不支持"） | 后续 wave |
| 图谱抽取器（extractor） | M5-7 两阶段抽取 | 后续 wave |
| SQLite 持久化（`graph.db`） | M5-8 原始 DDL 设计 | 后续 wave |
| 后台 rebuild worker / 维护任务（`graph-vacuum`） | 后台索引 | 后续 wave |

### 2.3 M5-8 旧设计 supersession（A1 W12 承接 A7 W11 卡 §2 L61 的文档整合项）

> A7 W11 卡原文：*"偏离 stale `M5-8-graph-store-query.md` 的说明：M5-8 原始设计（9 命令 + SQLite DDL `graph_store.rs`/`graph_query.rs`/`graph_maintenance.rs` + `source`/`source_ref`/`created_at`/`updated_at`/`extractor_version` 字段）已被 W5–W10 演进**取代**：真实落地为内存 `GraphStore` + 3 只读命令 + 无 `source` 字段 DTO（W6 评审 F1 已确认这些字段不存在）。**W12 以冻结的 `graph.rs`/`domain.rs` 为真相源，不回退到 M5-8 的 9 命令 SQLite 设计**。M5-8 文档更新归 A1/A0 文档整合，不在本 Lane 范围。"*

**A1 W12 处置**：在 `M5-8-graph-store-query.md` 追加 `[W12 scope supersession]` 段，把该卡标注为 **SUPERSEDED（部分）** —— 存储/查询职责由 W5 落地的内存 `GraphStore`（`src-tauri/src/graph.rs`）承接，W12 只在其上暴露 3 条只读命令；9 命令 + SQLite DDL + 5 个 `source*` 字段**不作为 W12 真相源**，避免后续 lane 误按 stale 设计实施。

---

## 3. A1 W12 工作树（A1 lane 范围 11 文件改动 + 1 new checkpoint + 1 new patch）

| # | 路径 | 变更类型 | 内容 |
|---|------|----------|------|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | M | 标题链追加 *W11 reconciliation + W11 PUSHED + W12 active* + 头部时间戳链 +2 行（W11 拣入 + W12 active）+ L20 基准链追加 *W11 拣入* 行 + 末尾追加 `[W11 reconciliation]` 段 + `[W12 active]` 段（含 W12 runtime-lock 状态表 11 行）|
| 2 | `logs/checkpoints/M5-20260906/M5-7-graph-model-extract.md` | M | 头部追加 *W12 ACTIVE* 范围行（抽取器 / 两阶段抽取**不在** W12；W12 仅 3 只读命令消费已落地的 `graph.rs`）|
| 3 | `logs/checkpoints/M5-20260906/M5-8-graph-store-query.md` | M | **追加 `[W12 scope supersession]` 段**：9 命令 + SQLite DDL → 3 只读命令 + 内存 `GraphStore`；标注 SUPERSEDED（部分）；列出 IN/OUT scope |
| 4 | `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` | M | L14 后追加 *W11 PUSHED* + *W12 ACTIVE* 状态行（**A8 W12 = UI 消费 3 只读命令 + `GRAPH_COMMANDS_AVAILABLE` 翻 `true`**）|
| 5 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | M | L14 后追加 *W11 PUSHED* + *W12 ACTIVE* 状态行（plugin runtime 仍 LOCKED）|
| 6 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | M | L14 后追加 *W11 PUSHED* + *W12 ACTIVE* 状态行（plugin runtime 仍 LOCKED）|
| 7 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | M | L14 后追加 *W11 PUSHED* + *W12 ACTIVE* 状态行（plugin UI runtime 仍 LOCKED）|
| 8 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | M | 头部追加 *W11 PUSHED* + *W12 ACTIVE* 状态行 + 末尾追加 `[W12 verification scope]` 段（W12 验证矩阵 6 FAC + 硬停验证必跑 8 条）|
| 9 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | M | 标题追加 *W11 reconciliation + W11 PUSHED + W12 active* + 顶部追加 *W11 PUSHED* / *W12 ACTIVE* 行 + 末尾追加 `[W12 active]` 段（runtime 债表 + W12 增量债）|
| 10 | `AI-模型切换与接手清单.md` | M | L1 后追加 A1 20:30 W12 update 行（与 A0 20:30 W12 派发行并列）|
| 11 | `详细设计与实施计划.md` | M | 同上 |
| 12 | `后续需求TODO.md` | M | 同上 |
| 13 | `PARALLEL_COMMAND_BOARD.md` | M | L6/L7 修订为 W11 PUSHED（`269269a`）+ W12 active |

### 2 new 文件（A1 整包交付物）

- `logs/checkpoints/A1-M5-W12-reconciliation-20260907-2030.md`（本 checkpoint）
- `logs/checkpoints/Lane-A1-M5-W12-reconciliation-20260907-2030.patch`（A1 W12 整包 git diff patch）

---

## 4. W11 拣入事实（reconcile W11 as accepted after A0 push）

**HEAD = `269269a feat(M5): integrate W11 MCP stdio dry-run hardening`**（origin/master；`git log 5226aad..269269a` 单 commit）。

**A0 W11 验收**（`logs/checkpoints/A0-M5-W11-accept-W12-dispatch-20260907-2030.md`，STATUS=**PASS_WITH_DEBT**）：

- MCP stdio dry-run 已加固：有界输入 `MAX_INPUT_BYTES=1MiB`、有界响应 `MAX_RESPONSE_BYTES=4MiB`、稳定 JSON-RPC 错误、稳定 `tools/list` schema、未绑定/未知能力 **fail-closed**、原始 arguments **不回显**
- 仍为 `#![cfg(feature = "mcp")]` feature-gated，默认构建不污染，**无 rmcp/tokio、无网络监听、无 file/db/script/plugin/agent/skill/model 执行**
- A5 新增 Agent/Skill 执行锁 policy PENDING 码位，默认扫描 PASS
- A7/A9 已准备 W12/W13 图谱/插件后续卡

**A0 复跑关键门**：

- `cargo test --manifest-path src-tauri/Cargo.toml --features mcp mcp_server` **21/21 PASS**（W10 基线 7/7 → W11 21/21）
- `check-mcp-policy.py --self-test/default/current` **PASS(ACTIVE=11, PENDING=0)**（W10 ACTIVE=9 → W11 ACTIVE=11）
- `check-agent-skill-policy.py --self-test/default` **PASS(ACTIVE=3, PENDING=6)**

**A1 W11 整包已被 A0 合并拣入**（`269269a` stat 含 `A1-M5-W11-reconciliation-20260907-1830.md` 174 行 + `Lane-A1-M5-W11-reconciliation-20260907-1830.patch` 545 行 + M5-0 125 行等），本 wave 工作树起点干净。

---

## 5. W12 runtime surface 锁定状态表（A1 在 M5-0/14 标注）

| Runtime Surface | W11 → W12 状态 | 责任 Lane / 依据 |
|---|---|---|
| **Graph live-query command（3 只读命令）** | 🔒 **LOCKED**（W10/W11）→ 🟢 **OPENED**（W12, narrow, **read-only**） | board L198 A7 W12 = START PRODUCT CODE NARROW；仅 `graph_query`/`graph_node_get`/`graph_stats`；无写/列/导出/构建/索引 |
| Graph build / index / write / export | 🔒 **LOCKED** | W12 Hard Stop L206；写命令 + `graph_export` + extractor + SQLite + rebuild worker 全部 OUT |
| Graph UI consumption | 🟢 **OPENED** | board L199 A8 W12 = START PRODUCT CODE UI NARROW（`GRAPH_COMMANDS_AVAILABLE` 翻 `true`）|
| MCP full runtime / rmcp server | 🔒 **LOCKED** | W12 Hard Stop L209；A3 W12 = MCP REVIEW ONLY |
| Plugin install/enable/delete/download | 🔒 **LOCKED** | W12 Hard Stop L209；A9 W12 = PLUGIN DOCS ONLY；DEBT-04 |
| Skill/Agent execution | 🔒 **LOCKED** | W12 Hard Stop L209；A5 W12 = AGENT/SKILL REVIEW ONLY（PENDING=6 执行锁）|
| Model call | 🔒 **LOCKED** | W12 Hard Stop L209 |
| Background daemon / network listener | 🔒 **LOCKED** | W12 Hard Stop L209 |
| DB / script / hidden execution path | 🔒 **LOCKED** | W12 Hard Stop L209；A2 W12 = BOUNDARY REVIEW |
| Build metrics 阈值 22% | 🟢 **维持** | W12 Hard Stop L210 |
| cargo_warnings delta | 🟢 **= 0** | W12 Hard Stop L210 |
| Push | 🔒 **仅 A0** | W12 Hard Stop L211 |

---

## 6. W12 十一 lane 角色（board L192-202）

| Lane | Status | W12 任务要点 |
|---|---|---|
| **A1**（本卡） | START DOCS ONLY | reconcile W11 PUSHED + mark W12 active + 图谱卡 W12 范围收敛为 3 只读命令 + UI 消费 |
| A2 | START BOUNDARY REVIEW ONLY | 审 `graph.rs` pure/state only、命令在 `bridge.rs`、`graph.rs` 不 import `crate::bridge`、无 DB/script/plugin/agent 执行路径、无 background worker |
| A3 | START MCP REVIEW ONLY | 审 W12 图谱命令**未**经 MCP stdio 暴露为可执行工具；MCP 仍 dry-run/read-only introspection |
| A4 | START PRIVACY REVIEW ONLY | 审输出**去 `props`**、错误稳定码、审计不含 labels/props/query bodies with secrets |
| A5 | START AGENT/SKILL REVIEW ONLY | 确认 Agent/Skill 执行仍锁（PENDING=6）、图谱 UI 不暗示 agent consumption runtime |
| A6 | START UI REVIEW ONLY | 审 workspace UI Agent/Skill 面板在 W12 图谱变更后仍 execution controls disabled + deterministic |
| **A7** | **START PRODUCT CODE NARROW** | 实施 3 只读命令：`GraphState` managed state + 启动载入既有 store + `check_invocation_source` + bounded depth/limit + 稳定错误码 + 输出去 props + ACL/bridge.ts/types 同包 + graph policy self-test 更新 |
| **A8** | **START PRODUCT CODE UI NARROW** | UI 消费 3 只读命令：backend-ready load/query/stats + AbortController/debounce + bounded rendering + 确定性 empty/error/loading |
| A9 | START PLUGIN DOCS ONLY | plugin runtime 仍锁；仅在 W11 反馈改变 blocker 时细化 W12/W13 plugin 卡 |
| A10 | START SECURITY REVIEW | 批量审 W12 图谱 bridge/UI；block on 写/构建/后台 worker、props/secret 输出、source-check/ACL 缺口、DB/script/plugin/agent 执行路径、build metric/warning 回归 |
| A11 | START VERIFICATION | W12 验证矩阵：cargo test、graph focused tests、graph policy self/default、MCP policy 仍 PASS、Agent/Skill lock 仍 PASS、graph UI logic、npm build、pre-merge、build metrics ≤22%、warnings 不变 |

**W12 = 仅 A7 + A8 可写产品代码**（A7 图谱后端 3 命令 / A8 图谱 UI 消费）；其余 9 lane 全部 review / docs / verification。

---

## 7. W12 硬停止遵守记录（board L204-211）

- ① ✅ W12 图谱范围**只读查询**：无 graph build / index / write / export / background worker（本卡 §2.2 已列 OUT）
- ② ✅ 图谱输出 DTO **去 `props`**；错误为稳定码，**不 echo** labels / props / query bodies / paths / URLs / tokens / cookies / Authorization
- ③ ✅ 新命令必须同包过 `check_invocation_source` + ACL + `main.rs` handler + `bridge.ts` + `types.ts` + policy + tests
- ④ ✅ MCP full runtime / plugin install·enable·delete·download / Agent/Skill execution / network listener / daemon / model call / hidden script·db execution **全部 LOCKED**
- ⑤ ✅ build metrics 阈值 **22%** 维持；cargo warnings 不增加
- ⑥ ✅ 仅 A0 push（本卡不 push，工作树留待 A0 拣入）

---

## 8. W12 验证矩阵（A1 在 M5-13 `[W12 verification scope]` 段标注）

| FAC | 子卡 | W12 AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|--------|------|----------------|------------|
| **FAC-7/8.W12 (new·核心)** | M5-7/8/9 图谱 3 只读命令 | `graph_query` 有界子图（删 props + `truncated`/`applied`）/ `graph_node_get`（64-hex + 缺失 `null`）/ `graph_stats`（容量 + 黄牌）；3 命令在 `bridge.rs` 过 `check_invocation_source`；`graph.rs` 不 import `crate::bridge` | **ACTIVE · A7 W12 实施** | `cargo test graph`（W5 9/9 基线 + W12 增量）PASS；`check-graph-policy.py --self-test/default` PASS（W5 ACTIVE=7 + W12 新增 `GRAPH_OUTPUT_NO_PROPS` 等守门码） | **W12 唯一后端产品代码 lane**；A2 boundary + A3 MCP + A4 privacy + A10 security 四重 review 必过；无写/列/导出/构建/后台 worker 是 W12 红线 |
| **FAC-9.W12 (new)** | M5-9 图谱 UI 消费 | A8 W12 UI 消费 3 只读命令：`GRAPH_COMMANDS_AVAILABLE` 翻 `true` + AbortController/debounce + bounded rendering 保留 + 确定性 empty/error/loading | **ACTIVE · A8 W12 实施** | `check-graph-ui-logic.mjs` PASS（W10 43 断言基线 + W12 增量）+ `npm run build` PASS | 不渲染 props / secret；no backend changes beyond bridge/types use |
| **FAC-2.W12 (carried)** | M5-2 MCP stdio dry-run | W11 加固态维持：21/21 + ACTIVE=11/PENDING=0 + 无 listener/network/raw-arg echo | **PASS (carried) · runtime LOCKED** | `cargo test --features mcp mcp_server` 21/21 + `check-mcp-policy.py --self-test/default/current` PASS(ACTIVE=11,PENDING=0) | A3 W12 = MCP REVIEW ONLY；MCP full runtime 仍 LOCKED；**W12 图谱命令不得经 MCP stdio 暴露为可执行工具** |
| **FAC-4/5/6.W12 (carried)** | M5-4/5/6 Agent/Skill | Agent/Skill 执行锁维持（PENDING=6 默认 PASS）；图谱 UI 不暗示 agent consumption runtime | **PASS (carried) · execution LOCKED** | `check-agent-skill-policy.py --self-test/default` PASS(ACTIVE=3,PENDING=6) + `check-agent-skill-ui-logic.mjs` PASS | A5 W12 = REVIEW ONLY；A6 W12 = UI REVIEW ONLY（execution controls 仍 disabled）|
| **FAC-10/11.W12 (carried)** | M5-10/11 plugin | plugin runtime 仍锁；W12/W13 plugin 卡仅 docs | **PASS (stub) · runtime LOCKED** | `check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=6) + 5 stub 维持 `Err("not-implemented-in-W6")` | A9 W12 = PLUGIN DOCS ONLY；DEBT-04 仍挂账 |
| **FAC-13/14.W12 (carried)** | build metrics / A11 验证 | build metrics ≤22% + warnings 不变 + pre-merge ALL_PASS | **ACTIVE · A11 W12 收口** | `scripts/measure-build-metrics.sh` + `scripts/pre-merge.sh` + `logs/checkpoints/A11-M5-W12-*.md` | A11 W12 verification matrix 必出 delta |

**W12 硬停验证必跑**（8 条）：
- ① 图谱只读：`grep -E 'upsert|delete|export|insert' src-tauri/src/bridge.rs` 在 graph 命令上下文 0 命中
- ② 输出去 props：`GraphNodeView`/`GraphEdgeView` 无 `props` 成员 + `check-graph-policy.py` 新增 `GRAPH_OUTPUT_NO_PROPS` 守门码
- ③ 稳定错误码：`GraphError::code()` 返回稳定码；错误体不 echo props/labels/query/path/URL/token/cookie/Authorization
- ④ 同包三同步：3 命令 + ACL（插末条 `list_artifact_images` 前）+ `main.rs` + `bridge.ts` + `types.ts` + policy + tests 同包
- ⑤ `graph.rs` 不 import `crate::bridge`（守 `GRAPH_NO_SECOND_PATH`）
- ⑥ MCP 未暴露：`check-mcp-policy.py` 仍 PASS + A3 W12 review note
- ⑦ build metrics ≤22% + cargo_warnings delta = 0
- ⑧ 仅 A0 push

---

## 9. M5 final debt ledger（W12 维持 53 条 + W12 增量预期）

- **DEBT-03**（carried）：M5-1.b review note 待 A2 v3 收口
- **DEBT-04**（carried）：plugin UI runtime LOCKED；A9 W12 = PLUGIN DOCS ONLY；收口推 W13+
- **DEBT-19**（carried → **W12 收口**）：`GRAPH_OUTPUT_NO_PROPS` —— W12 输出去 props 落地后关闭（A7 实施 + `check-graph-policy.py` 新增守门码）
- **A7 W12 3 只读命令**（new·窄）：bounded depth/limit + 稳定错误码 + `GraphState` 载入 + ACL/bridge/types 同包；A2/A3/A4/A10 四重 review 必过
- **A8 W12 UI 消费**（new·窄）：`GRAPH_COMMANDS_AVAILABLE` 翻 true + AbortController/debounce + bounded rendering
- **M5-8 stale 设计债**（new·文档）：9 命令 + SQLite DDL + 5 个 `source*` 字段 vs 真实落地（3 只读 + 内存 GraphStore）—— **本包已加 `[W12 scope supersession]` 段标注**，避免后续 lane 误按 stale 设计实施
- **runtime 债**（MCP server / plugin runtime / skill-exec / 图谱写与持久化）：按设计延后到后续 runtime wave（W12 硬停止禁运行时），非 W12 阻塞

---

## 10. FORBID 遵守记录

- 本卡为 A1 M5-W12 文档展开，**未写任何产品代码**
- 未触 `src/`、`src-tauri/`、`package.json`、ACL/Capability、`graph.rs`/`domain.rs`/`bridge.rs`/`main.rs`/`check-graph-policy.py`
- 未移动 `NEXT`（仍 `M5-W12`，A0 拣入期管理）
- 未提交、未 push（本卡包为 A0 待拣入的待选文档）
- 11 文件改动全在 A1 范围内，未触其他 lane 工作树（A2-A11 W12 等）
- 3 份主文档 L1 修订与 A0 20:30 W12 派发行并列，未触决策史
- M5-0/8/13/14 新增段不影响 §1~§11 决策史
- 严格遵守 `PARALLEL_COMMAND_BOARD.md` L7 规则：*`Each lane 整包 deliver patch+checkpoint; only A0 pushes`*
