# A8 M5-9 图谱 UI · W6 Checkpoint（PRODUCT CODE · PANEL SHELL + PURE LOGIC）

> LANE=A8 · WAVE=M5-W6 · 依据：`PARALLEL_COMMAND_BOARD.md` M5-W6（line 135-171，base `4b438ef`）
> STATUS=PASS（W6 整包交付：图谱 UI 纯逻辑 + 面板壳；构建 PASS、逻辑测试 PASS、零新依赖、零实时消费）
> BASE=4b438ef（board dispatch 基点；当前 working tree 已含 A7 W5 图谱 DTO/存储）
> 官方 M5-9 卡：`logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md`（实施候选 = A19；本波 A8 按 W6 指派实际落地面板壳）
> 上游 schema 源：`src-tauri/src/domain.rs` L2040-2116（GraphNode/GraphEdge/GRAPH_*）、`src-tauri/src/graph.rs`

---

## 1. W6 指派（权威源，board line 159）

> A8 | **START PRODUCT CODE** | Implement M5-9 graph UI pure logic and panel shell: graph list/search/filter, node detail summary, capacity/error/empty states, helper module + headless logic test. Do not call live agent consumption or backend graph commands unless already existing and fully typed.

允许范围：`src/components/**`、`src/stores/**`、`src/types.ts`、`src/bridge.ts`(仅新增占位封装不增命令)、`scripts/check-graph-ui-logic.mjs`、可选 UI 策略脚本、docs/checkpoint。
Must Deliver：`npm run build` PASS、UI 逻辑测试 PASS、无新依赖、无实时 agent 消费。

W6 Hard Stop（line 164-171）：仅 A8/A9 可写产品代码；A8 不得新增后端命令、实时 agent 消费、模型调用、图重建 worker；所有 store/map/list 必须有界；不得记录/持久化 token/cookie/Authorization/body/prompt-secret。

## 2. 实现内容

### 2.1 纯逻辑层 `src/utils/graphUi.ts`（无 invoke / DOM / store 依赖，可被 node 直接导入断言）
- `elementKind(d)`：判别 GraphNode / GraphEdge / unknown。
- `nodeKindLabel` / `edgeKindLabel` / `nodeColor`：种类中文标签与配色。
- `summarizeNode` / `summarizeEdge`：**K7 硬禁**——摘要中**绝不出现 `props` 字段**（GraphNode.props 后端已脱敏，UI 不渲染正文）。
- `isAgentConsumptionEdge`：识别 `uses`/`a2a_with`/`memorizes`（对应官方卡 Agent 消费子视图 (b)）。
- `layoutKindOf` / `layoutPositions`：纯 TS 确定性布局（tree/cluster/force），**无 D3、无随机**（满足「无新依赖」与可测试性）。
- `estimateCapacity`：容量估算（对齐 `GRAPH_MAX_NODES=5000` / `GRAPH_MAX_EDGES=20000` / `GRAPH_MAX_DEPTH=4` / `GRAPH_QUERY_LIMIT=1000`）。
- `filterNodes` / `filterEdges`：列表搜索（标签/ID/类型，忽略大小写）+ 按种类过滤 + 两端可见性约束。
- `boundedInsert`：store 有界合并（超上限丢最旧）。
- `panelStateGraph`：空/错误/加载三态（backendReady=false 时提示「只读壳」）。

### 2.2 状态机 `src/stores/useGraphStore.ts`（Pinia）
- 有界 `Map` 存储 nodes/edges（上限对齐 `GRAPH_MAX_*`）。
- `backendReady = GRAPH_COMMANDS_AVAILABLE`（当前 **false**）。
- `guard()`：后端未就绪时**零 invoke**，只置错误提示（LIMITED START 口径；与 `useAgentStore` 一致）。
- `loadGraph()`：仅当 backendReady 时调用 `bridge.graphQuery()`（占位封装）；当前永不触发。
- 选择/过滤/容量/三态全部 computed 暴露给组件。

### 2.3 组件 `src/components/graph/`（懒加载，不进主 chunk）
- `GraphPanel.vue`：头部标题 + 容量计（`节点 n/5000 · 边 n/20000`）+ 刷新按钮（backendReady=false 时禁用）+ backendReady 横幅 + 错误横幅 + `GraphFilter` + `GraphViewer` + 右侧 `NodeDetail`/`EdgeDetail`。
- `GraphFilter.vue`：搜索框 + 种类 chips + 清除。
- `GraphViewer.vue`：SVG 节点-连图（按 kind 着色、可点选、键盘可达、`aria-label`），空数据时占位文案。
- `NodeDetail.vue` / `EdgeDetail.vue`：节点/边摘要；**明确注释不渲染 props 正文（K7）**。

### 2.4 接线与可达性
- `src/types.ts`：追加 M5-9 图谱 DTO（`GraphNodeKind`/`GraphEdgeKind`/`GraphNode`/`GraphEdge`/`GraphProps` + `GRAPH_*` 常量），与 A7 `domain.rs` 逐字段对齐，附 K7 隐私红线注释。
- `src/bridge.ts`：新增 `GRAPH_COMMANDS_AVAILABLE = false` 开关 + `graphQuery` 契约占位封装（命令未落地，受开关拦截，**未新增任何 Tauri 命令**）。
- `src/stores/useLayoutStore.ts`：`MainView` 联合类型加 `"graph"`，`MOD_META` 注册 `graph: { icon:"🕸️", label:"图谱" }`。
- `src/components/layout/MainArea.vue`：懒加载 `GraphPanel`（`defineAsyncComponent`，压低首屏体积，符合 IF-2）+ 渲染块。
- `src/components/layout/ActivityBar.vue`：工具分节加入「图谱」入口。

### 2.5 测试 `scripts/check-graph-ui-logic.mjs`
- 复用 `node:module` `registerHooks` 解析 `.ts`，直接导入真实 `src/utils/graphUi.ts`，断言产品代码行为（34 项全过）。
- 覆盖：判别、标签/配色、摘要（K7 不含 props）、消费边、布局（确定性/坐标有限/稳定）、容量（百分比/上限）、搜索过滤（查询+种类+两端可见）、有界合并、面板三态。

## 3. 验证（Verification）

| 项 | 命令/结果 |
|---|---|
| UI 纯逻辑测试 | `node scripts/check-graph-ui-logic.mjs` → **通过 34，失败 0**（EXIT=0） |
| 生产构建 | `npm run build` → **✓ built in 2.32s**，GraphPanel 拆为独立 chunk（`GraphPanel-*.js` 9.12 kB），主 chunk `index` 164.15 kB **未膨胀**（满足 IF-2） |
| 新依赖 | **无** `package.json` 未改动；未引入 d3/vue-router（布局用纯 TS+SVG） |
| 实时消费 | **无**：`backendReady=false`，`guard()` 拦截，`loadGraph()` 零 invoke；未触碰任何 agent/模型/RAG 路径 |
| 后端命令 | **未新增**：仅 `graphQuery` 占位封装（受开关拦截），未改 `bridge.rs`/`main.rs`/ACL/`default-commands.toml` |
| 有界 store | nodes/edges `Map` 上限对齐 `GRAPH_MAX_*`；`boundedInsert` 超容丢最旧 |
| K7 合规 | NodeDetail/EdgeDetail 不渲染 `props`；摘要函数不返回 `props` 字段 |

## 4. FORBID 遵守

- W6 内仅 A8/A9 写产品代码；A8 未越界修改其他 lane 文件（仅 `types.ts`/`bridge.ts`/`useLayoutStore.ts`/`MainArea.vue`/`ActivityBar.vue` 本 lane 接线所需）。
- 未新增后端命令、未做实时 agent 消费、未做图重建 worker、未引入新依赖。
- 未提交、未 push（W6 Hard Stop：「All lanes ... must not push」）。

## 5. 输出模板回填

```text
LANE=A8
STATUS=PASS
WAVE=M5-W6 (START PRODUCT CODE)
BASE=4b438ef
HEAD=patch logs/checkpoints/Lane-A8-M5-W6-graph-ui-20260906-2010.patch
FILES=src/utils/graphUi.ts, src/stores/useGraphStore.ts, src/components/graph/{GraphPanel,GraphFilter,GraphViewer,NodeDetail,EdgeDetail}.vue, scripts/check-graph-ui-logic.mjs, src/types.ts, src/bridge.ts, src/stores/useLayoutStore.ts, src/components/layout/MainArea.vue, src/components/layout/ActivityBar.vue
VERIFY=node scripts/check-graph-ui-logic.mjs → 34/34 PASS; npm run build → PASS(GraphPanel 独立 chunk 9.12kB, 主 chunk 164kB 未膨胀); 无新依赖; backendReady=false 零 invoke
CHECKPOINT=logs/assist/A8-M5-W6-graph-ui-checkpoint-20260906-2010.md
MERGE_NOTES=W6 A8 落地 M5-9 图谱 UI 纯逻辑+面板壳；GraphNode/GraphEdge DTO 镜像 A7 domain.rs; backendReady=false(无 graph_* 命令)→只读壳零 invoke; 未新增后端命令/依赖; K7 不渲染 props; store 有界(GRAPH_MAX_*); GraphPanel 懒加载不膨胀主 chunk; 解锁= A7 落地 graph_query 等命令且把 GRAPH_COMMANDS_AVAILABLE 置 true 后，useGraphStore.loadGraph 即接 bridge.graphQuery 实时载入（组件无需改）
NEXT=A7 落地 graph_query/graph_build/graph_neighbors/graph_stats/graph_export 并将 GRAPH_COMMANDS_AVAILABLE 置 true（同包含 source check+ACL+bridge/types+策略+测试）→ A8 面板自动转实时；后续如需 RAG 注入(/graph 前缀)再补 useGraphRag（本波未做，因 W6 禁实时消费）
```

## 6. 风险/备注

- 官方 M5-9 卡 §3 原计划用 D3 力导向 + `useGraphRag`/router。本波受「无新依赖」「禁实时消费」约束：布局用纯 TS+SVG（无 D3，无 vue-router，沿用 layout store 模块切换），RAG 注入未做（留待命令+标志就绪后）。
- 图谱真实数据需 `graph_query` 等命令；当前面板为只读壳，展示空/容量/过滤 UI 与确定性布局预览。
- 与 A7 协同点：`GRAPH_*` 常量与 `GraphNode/GraphEdge` 字段必须保持与 `domain.rs` 单一真源一致（已注释对齐）。
