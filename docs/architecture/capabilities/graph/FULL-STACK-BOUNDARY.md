# KnowledgeGraph Capability — Full-Stack Boundary（Capability Library Expansion v1, STAGE G）

> STAGE G 产物。物理：GraphPanel（+ GraphFilter/GraphViewer/NodeDetail/EdgeDetail）迁入
> `src/capabilities/graph/ui/`，语义 owner `useGraphStore` 迁入 `src/capabilities/graph/state/`；
> 经通用 Contribution Registry 的 `WORKBENCH_MAIN` 槽（view='graph'）贡献给 MainArea。
> **命名**：能力 id 由 registry 旧名 `knowledge_graph` 统一为 `graph`（下划线不满足 Building Block
> Contract 的 id 规则 `^[a-z][a-z0-9.]*$`），并同步 agent 的 optionalDependencies。最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C2 ISOLATED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C2`；governanceStatus=`GOVERNED`）。

评级依据（按 `C0..C5` 标度）：
- **C2 ISOLATED 达成**：实现经 `manifest.ts` / `public.ts` / `index.ts` / `state/` / `ui/` 五段边界隔离；
  语义 owner `useGraphStore` 唯一（无第二真源）；MainArea 不再静态 import GraphPanel（贡献驱动）。
- **非 C3**：① 无 graph 专属 absence 运行时门禁（依赖 profile 预设 + 探针实测，缺常驻自动化断言）；
  ② `mainView='graph'` 导航项仍硬编码于 `useLayoutStore`/`homeUi`/`HomeLaunchers`（未贡献驱动）。
- **非 C4/C5**：图存储为启动期只读载入的内存快照，无独立生命周期可装卸。

## 十七段 Full-Stack 契约

```text
Graph UI (capabilities/graph/ui/GraphPanel.vue + GraphFilter/GraphViewer/NodeDetail/EdgeDetail)
  ↓ OWNED_BY_CAPABILITY（经 public.ts 消费语义 owner）
Graph State Owner: useGraphStore (id="graph", src/capabilities/graph/state/useGraphStore.ts)
  ↓ 意图（intents）
  loadGraph(startId) / loadNode(id) / loadStats() / refresh()
  / selectNode / selectEdge / setFilter / toggleKind / clearFilter / cancelInFlight
  ↓ PUBLIC_DEPENDENCY（bridge）
Graph Adapter: src/bridge.ts → graphQuery / graphNodeGet / graphStats（GRAPH_COMMANDS_AVAILABLE=true）
  ↓ NATIVE_ADAPTER（Rust tauri::command）
src-tauri/src/bridge.rs: graph_query(7710) / graph_node_get(7729) / graph_stats(7731+)
  ↓ 后端资源（Rust）
src-tauri/src/graph.rs: GraphStore 内存有界存储（RwLock）+ graph.json 启动期**只读**载入
  —— 无 worker / 无子进程 / 无网络 / 无 webview / 无写盘（门禁 GRAPH_NO_SECOND_PATH）。
```

## 十七段逐项事实

| 段 | 事实（证据） |
|---|---|
| Identity / Manifest | `src/capabilities/graph/manifest.ts`（id=graph，C2，HP0） |
| Public Contract | `src/capabilities/graph/public.ts`（仅再导出 `useGraphStore`，SECOND_TRUTHS=0） |
| Dependencies | required `bridge`（外部基础设施豁免）；optional `agent` |
| State Owner | `useGraphStore`（id="graph"）唯一 |
| Canonical Writers | 仅 `useGraphStore` 的 action 写 `nodes/edges/selected*/filter/truncated`；组件只读经 public |
| Intents | loadGraph / loadNode / loadStats / refresh / selectNode / selectEdge / setFilter / toggleKind / clearFilter |
| Application Logic | `src/utils/graphUi.ts`（headless：有界插入、布局、容量、稳定错误码映射、viewTo* 白名单） |
| UI | `capabilities/graph/ui/*.vue`（GraphPanel 懒加载，经贡献注册） |
| Contributions | `graph.main.panel`（slot=workbench-main, view='graph'） |
| Side Effects | 全部经 `bridge.graph*`；UI 零裸 invoke；无写盘副作用 |
| Adapter / Native Boundary | `bridge.ts` → Rust `graph_*`（3 只读命令，ACL 放行，来源校验） |
| Permissions | 无 |
| Persistence | 后端 `data_dir/graph.json`（只读载入）；前端零浏览器存储 |
| Resource Ownership | 只读查询既有内存快照 → `v1.resources=[]`（class `[MEDIUM]` 为潜在声明） |
| Lifecycle | `ACTIVE` / `SUSPENDED`（suspendable=true，但无独立资源可释放） |
| Absence Behavior | 见下（实测证据） |
| Tests / Gates | `check-graph-policy.py`（Rust，8 码）+ `check-graph-ui-logic.mjs`（113 断言）+ `npm run check` |

## 安全 / 边界（K7 双闸）

- **无第二条执行路径**：门禁 `GRAPH_NO_SECOND_PATH` 禁 `std::process` / `Command::new` / `tokio` / `use tauri` / `crate::bridge`。
- **props 不外泄**：后端 `GraphNodeView/GraphEdgeView` 删 `props`；前端 `viewToNode/viewToEdge` 白名单。
- **有界存储**：`GRAPH_MAX_NODES` / `GRAPH_MAX_EDGES`，`boundedInsert` 截断并置 `truncated` 信号。
- **无 secret**：graph 不涉及 credential；稳定错误码映射，零 secret echo。

## Absence Behavior（§18/§29，实测证据）

STAGE G 用探针实测（esbuild 真实 bundle + `bootstrapCapabilityRuntime(profile)`）：

```text
framework: activated=true graph=false tasks=false plugin=false
minimal:   activated=true graph=false tasks=false plugin=false
developer: activated=true graph=false tasks=false plugin=false
full:      activated=true graph=true  tasks=true  plugin=true
```

- Graph absent（framework/minimal/developer）→ `registerGraphContributions` 不运行 → `WORKBENCH_MAIN` 槽无 `view='graph'`
  → MainArea `viewOf('graph')` 返回 `undefined` → 不渲染 GraphPanel → `useGraphStore` 不被实例化 → **零 bridge.graph\* invoke**。
- Shell 不崩溃：通用 `viewOf` 分支对未知 view 返回 undefined 自然跳过。
- 资源缺席：graph 本身不创建运行时资源（只读内存快照），absent 时资源恒为 0（`RUNTIME_RESOURCE_ABSENCE 12/12`）。

## Legacy Debt（诚实，不静默消失）

1. 无 graph 专属 absence 运行时门禁。
2. `mainView='graph'` 导航项硬编码（`useLayoutStore`/`homeUi`/`HomeLaunchers`，未贡献驱动）。
3. 能力 id 由 `knowledge_graph` 改名 `graph`（registry/docs 历史文档仍可能保留旧名，属历史记录）。

## SECOND_TRUTHS = 0 / RESOURCE_LEAKS = 0

`public.ts` 仅再导出 `useGraphStore`（语义 owner），未创建镜像状态。DOMAIN STATE（useGraphStore）
≠ CAPABILITY COMPOSITION STATE ≠ UI LOCAL STATE ≠ RESOURCE RESULT。新增资源泄漏 = 0。
