# Lane A8 — M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch / Graph UI Non-Regression Note

> Status: **SUPPORT DOCS ONLY**（A8 在 W13 不写产品代码，按 `PARALLEL_COMMAND_BOARD.md` line 205）
> Author lane: A8
> Scope: Graph UI W12 已收口；W13 plugin manifest lifecycle 不交叉 Graph UI；产出 thin non-regression note。
> Time: 2026-09-08 00:00 CST
> Base HEAD: `3c3f460` (W12 graph live-query integration commit, on `master`)

## 1. 角色与边界（A8 在 W13 做什么）

按 Board 第 205 行（`M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch` Lane Table）：

- **Status**: SUPPORT DOCS ONLY
- **Scope**: "Graph UI W12 is closed; do not write graph UI product code. Review W13 only for graph UI non-regression if asked."
- **Allowed Files**: `logs/assist/A8-M5-W13-*.md`
- **Must Deliver**: Non-regression note（本文）

因此本 lane **零产品代码、零 frontend 改动、零 policy 改动**，仅交付这份事实可核验的 non-regression 报告。

## 2. W13 改动分类与 A8 触及面分析

### 2.1 W13 写代码的 lane 及其允许范围

| Lane | 状态 | A8 graph UI 触及面 |
|---|---|---|
| A1 | DOCS ONLY（checkpoint + patch） | 无（仅三份主文档 + checkpoint 文件） |
| A2 | BOUNDARY REVIEW ONLY（plugin 边界评审） | 无（仅 policy fixtures） |
| A3 | MCP REVIEW ONLY（确认 plugin 命令不漏出到 MCP stdio） | 无 |
| A4 | PRIVACY REVIEW ONLY（plugin DTO/audit/secret echo） | 无 |
| A5 | AGENT/SKILL REVIEW ONLY（确认 Agent/Skill 执行仍锁） | 无 |
| A6 | UI REVIEW ONLY（评审 plugin manager UI：disabled invoke、lifecycle status、不假执行） | 无（A6 评审的 plugin UI 属 A9 新建组件；A8 graph UI 不在评审区） |
| A7 | SUPPORT DOCS ONLY（graph non-regression if asked） | 无 |
| **A8 (本 lane)** | **SUPPORT DOCS ONLY**（本文件） | **本文件** |
| A9 | **PRODUCT CODE NARROW**（plugin manifest lifecycle：local registry/store、`plugin_install`/`enable`/`disable`/`list`/`get`、trusted-key 数据管理；no invoke/network/dynamic-load） | 见 §2.2 |
| A10 | SECURITY REVIEW（W13 plugin 生命周期） | 无（review-only） |
| A11 | VERIFICATION（cargo test plugin, mcp feature test, plugin/graph/mcp/agent-skill policies, UI logic, npm build, pre-merge, diff check） | 仅 matrix 引用 |
| A0 | 唯一 push lane | 无 |

### 2.2 A9 W13 允许的 frontend 文件范围（Board line 206）

A9 Allowed Files 包含：

- `src-tauri/src/plugin.rs`
- `src-tauri/src/domain.rs`
- `src-tauri/src/bridge.rs`
- `src-tauri/src/main.rs`
- `src-tauri/permissions/default-commands.toml`
- `src/bridge.ts`
- `src/types.ts`
- **plugin UI files**（新建组件/视图，命名如 `src/components/plugin/**` 等，不在 A8 既有 graph UI 目录内）
- `scripts/check-plugin-policy.py`

**A8 与 A9 的 frontend 文件命名空间天然不重叠**：A8 lane 拥有 `src/components/graph/**`、`src/stores/useGraphStore.ts`、`src/utils/graphUi.ts`、`scripts/check-graph-ui-logic.mjs`；A9 的 plugin UI 与 A8 零文件共享。`src/bridge.ts` 与 `src/types.ts` 为共享文件，但 A8 W13 不写这两份（属 A9 增量空间，A8 不抢）。

### 2.3 共享文件 non-regression 关注点（A9 触 `bridge.ts`/`types.ts` 时）

- `bridge.ts` 新增 `plugin_install/enable/disable/list/get/trusted_key_*` 包装；A8 graph 命令包装（`graphQuery/graphNodeGet/graphStats`，由 A7 W12 落地）应**完全保留**，不被 A9 改动替换。
- `types.ts` 新增 `PluginManifest/PluginLifecycle/TrustedKey` DTO；A8 graph DTO（`GraphNode/GraphEdge/GraphNodeKind/GraphEdgeKind/GraphNodeView/GraphEdgeView/GraphQueryRequest/GraphQueryResult/GraphNodeDetail/GraphStatsView`）应**完全保留**。
- A8 已有的 `bridge.ts` 包装签名与 `types.ts` graph 部分**不在 A9 W13 范围内**（A9 任务描述未要求重写 graph 桥），A8 不会回写。

## 3. 实测证据（确认 W12 graph UI 状态健康）

### 3.1 Graph UI logic 自动化门禁（最关键 deterministic gate）

```text
$ node scripts/check-graph-ui-logic.mjs
...
图谱 UI 逻辑测试：通过 113，失败 0
```

**W12 落地的 113 条断言全部通过**。其中关键覆盖项（与 W13 风险相关的）：

- `normalizeGraphQueryRequest depth` 截 `GRAPH_MAX_DEPTH(4)`
- `normalizeGraphQueryRequest limit` 截 `GRAPH_QUERY_LIMIT(1000)`
- `abortableDebounce` 防抖/取消/错误不冒泡
- `newGraphRequestId` 唯一性 + 格式 `req-[0-9a-f]{8}`
- `GRAPH_DEBOUNCE_MS=300`（与 A7 §5 / W10 W11 一致）
- **W12 用户可见文案不泄露 props/secret/token 关键字**（与 W13 A4 privacy review 同源约束）

### 3.2 Graph DTO 在 `src/types.ts` 中存在性

```text
$ grep -nE "GraphNode|GraphEdge|GraphNodeView|GraphEdgeView|GraphQueryRequest|GraphStatsView" src/types.ts
794:// GraphNode/Edge 无 privacy 字段——脱敏属后端职责，前端只负责不显示 props。
796:export type GraphNodeKind =
799:export type GraphEdgeKind =
806:export interface GraphNode {
808:  kind: GraphNodeKind;
814:export interface GraphEdge {
817:  kind: GraphEdgeKind;
823:// ====== M5-W12 图谱 live-query View DTO（与后端 domain.rs GraphNodeView/GraphEdgeView 对齐）======
828:// 字段 snake_case：与后端 GraphNodeView {id,kind,label} / GraphEdgeView {from,to,kind}
831:export interface GraphNodeView {
833:  kind: GraphNodeKind;
837:export interface GraphEdgeView {
840:  kind: GraphEdgeKind;
848:export interface GraphQueryRequest {
859:  nodes: GraphNodeView[];
860:  edges: GraphEdgeView[];
```

A8 graph DTO 全部存活、未被任何 W13 改写。W12 View 类型（`GraphNodeView/GraphEdgeView/GraphQueryRequest`）与后端 `domain.rs` 的 `GraphNodeView/GraphEdgeView` 对齐注释（line 823–828）亦健在。

### 3.3 Graph UI 文件对 plugin/agent/mcp 零跨引用

```text
$ grep -rn "import.*plugin\|from.*plugin\|require.*plugin" \
    src/components/graph/ src/stores/useGraphStore.ts src/utils/graphUi.ts
(empty = no cross-import)
```

A8 graph UI 视图/store/utils **不引用任何 plugin 模块**，因此 A9 W13 plugin 命名空间不可能反向影响 A8。

### 3.4 `useGraphStore.ts` 头部契约注释（W12 LIVE 行为）

```text
// M5-W12 增量（A7 §3.4 / §5 / §6）：
//   - `loadGraph(startId)`：接 AbortController + 300ms debounce + request_id（取消上一个）
//   - `loadNode(id)`：单节点查询（graph_node_get 包装）
//   - `loadStats()`：容量概览（graph_stats 包装），落 `capState` + `truncated` 信号
//   - `refresh()`：当前 start_id 重新查询
//   - 错误落 `error.value`（稳定码映射；零 secret echo）
//   - `truncated` 暴露（与 A7 §4 truncated/applied 信号配套）
//   - 维持 GRAPH_COMMANDS_AVAILABLE=false 时零 invoke 守门（W11 既有）
//   - 边用稳定标识 `selectedEdgeKey` 选择（W11 既有），保持不随索引漂移
```

W12 LIVE 行为完整、未被 W13 改动破坏。

## 4. A8 W13 风险面

| 风险 | 评级 | 说明 | 缓解 |
|---|---|---|---|
| A9 W13 plugin UI 引入新依赖/全局样式污染 graph UI | **极低** | A9 plugin UI files 在 A8 范围外（`src/components/plugin/**`），共享样式仅经 `src/styles/**`（属 A0/A1 lane），A9 不触 | 期望 A11 W13 跑 `npm run build` 仍 PASS 且 graph UI 视觉无 regression |
| A9 改 `bridge.ts`/`types.ts` 时意外替换 graph 包装/类型 | **低** | A8 既有包装与 DTO 已落地，A9 任务描述（plugin manifest lifecycle）不涉及 graph | A8 期望 A11 跑 `node scripts/check-graph-ui-logic.mjs` 仍 113/113 PASS（任何类型/包装不兼容会导致它失败） |
| A9 W13 `domain.rs` 改动反向影响 graph DTO | **极低** | A8 graph DTO 在 `domain.rs` 是 `GraphNode/GraphEdge/GraphNodeKind/GraphEdgeKind/GraphNodeView/GraphEdgeView/GraphQueryRequest/GraphStatsView`（A7 W5 落）`GraphError`，A9 W13 plugin types 不重叠 | 由 A2 boundary review 守 |
| A6 UI REVIEW 评 plugin UI 时连带改动 graph UI | **极低** | A6 任务限定评审 A9 plugin manager UI 改动；A6 既有 A8 graph UI 不在评审区 | A6 若发现需改 graph UI 应移交 A8 评估 |
| A10 security review 触发 graph UI 改动 | **极低** | A10 review-only；A10 不写 graph UI | A8 等待 A10 反馈后才决定是否动手 |

## 5. A8 W13 行动清单

按 SUPPORT DOCS ONLY 角色，本 lane 在 W13 batch 中**唯一行动**是本笔记。无 patch、无代码、无新文件（除本 note）。

如 W13 进行中 A0/A11 反馈"graph UI 出问题"或 A6 review 发现 plugin UI 与 graph UI 视觉冲突（属发现式触发），A8 才会启动修复流程，按 A6 W12 同模式产 `logs/checkpoints/A8-M5-W13-*.patch` + checkpoint。

## 6. 与 A8 历史笔记的关系

- `logs/assist/A8-M5-W12-graph-ui-consumer-20260907-2251.md` — W12 graph UI 完整消费实现记录（live-query + AbortController + bounded rendering + 确定性状态），是本 non-regression note 的"前置事实"。
- `logs/assist/A8-M5-W9-graph-ui-readonly-20260907-1015.md` — W9 graph UI 仍 read-only/no-backend 守门。
- `logs/assist/A8-M5-W8-graph-ui-polish-20260907-0945.md` — W8 graph UI polish（bounded rendering、deterministic states）。
- 本文件 = W13 起点，引用 W12 已交付事实，不再重复 W12 内容。

## 7. 不 push 声明

按规则，**未 push**。本 lane 零产品代码，无 push 内容。后续如需出 patch 修复（§5 触发），仍按 A8 历史模式本地工作树留待 A0 集成。
