# graph 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/graph/manifest.ts`。
> 诚实边界：后端 `graph_query/graph_node_get/graph_stats` 已实现、只读启动期内存快照；`GRAPH_NO_SECOND_PATH` 强制无 worker/webview/写盘；前端白名单删 props（K7）。当前为**只读投影**，无实时反查/写入。

---

## 1. Purpose
知识图谱域。负责知识图谱的只读查询、节点查看、容量统计，作为 Agent/Skill 关联的投影视图。

## 2. Domain Classification
- 领域：`graph`（原 registry id `knowledge_graph` 因含下划线不合 ID 规则，STAGE G 统一为 `graph`）
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 图谱查询（`graphQuery` / `graph_query`）、节点查看（`graphNodeGet` / `graph_node_get`）、容量统计（`graphStats` / `graph_stats`）。
- 前端白名单删 props（防敏感字段泄漏，K7）。
- 容量上限保护（`GRAPH_MAX_NODES`/`GRAPH_MAX_EDGES` / `boundedInsert`）。

## 4. Non-Responsibilities
- 不写入/重建图谱（无 graph write/export 后端）。
- 不创建 worker/webview/子进程（`GRAPH_NO_SECOND_PATH` 强制）。
- 不实时反查 Agent/Skill 后端（节点 id 引用，不持有其值）。
- 不负责数据库/终端等其它域。

## 5. Ubiquitous Language
- `GraphNode` / `GraphEdge`：图谱节点/边（来自 `src/types.ts`）。
- `GraphNodeKind` / `GraphEdgeKind`：节点/边类型枚举（file/dir/tab/script/skill/agent/tag/topic；in_dir/references/related_to/tagged_with/uses/a2a_with/memorizes）。
- `backendReady`：后端是否就绪（`=GRAPH_COMMANDS_AVAILABLE`）。
- `truncated` / `capacity`：容量上限提示。

## 6. Domain Model
- 聚合根：图谱快照（`useGraphStore` 管理，前端内存 `Map`）。
- 关键 state：`nodes` / `edges` / `selectedNodeId` / `selectedEdgeKey` / `loading` / `error` / `backendReady` / `startId` / `truncated` / `inFlightRequestId` / `filter`（`src/capabilities/graph/state/useGraphStore.ts`）。
- 后端真源：`src-tauri/src/graph.rs` `GraphStore`（内存 `Vec<GraphNode>/Vec<GraphEdge>` bounded）；`load_snapshot` 从 `data_dir/graph.json` 只读载入，失败回退空 store。

## 7. Invariants
- 后端仅读锁、零写、无第二路径（`GRAPH_NO_SECOND_PATH`）。
- `backendReady=false` 时 `guard()` 拦截，零 invoke。
- 输出必须经白名单删 props（K7 + 防御纵深）。
- `boundedInsert` 受 `GRAPH_MAX_NODES/EDGES` 限制，超限截断。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION（前端）**：`src/capabilities/graph/state/useGraphStore.ts`（`defineStore("graph")`）。
- **CURRENT PHYSICAL LOCATION（后端）**：`src-tauri/src/graph.rs` `GraphStore`。
- **semanticOwner**：`useGraphStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：无物理债务（state/ui 均在包内）；无 MULTIPLE_WRITERS。

## 9. Commands / Intents
- 业务 action：`loadGraph` / `loadNode` / `loadStats` / `refresh` / `cancelInFlight` / `selectNode` / `selectEdge` / `setFilter` / `toggleKind` / `clearFilter`。
- 原生命令（见 §17，均只读）：`graph_query` / `graph_node_get` / `graph_stats`。

## 10. Queries
- 前端内存：`nodeList`/`edgeList`/`visibleNodes`/`visibleEdges`/`capacity` 派生（均经白名单删 props）。
- 原生：`graph_query` / `graph_node_get` / `graph_stats`。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/capabilities/graph/public.ts`。
- 暴露：`useGraphStore`（再导出）、`graphManifest`、`type GraphNode/GraphEdge/GraphNodeKind/GraphEdgeKind`。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useGraphStore.ts` / `ui/`（GraphPanel/GraphFilter/GraphViewer/NodeDetail/EdgeDetail，5 个 .vue）。

## 14. Dependencies
- `dependsOn: ["bridge"]`（硬）。
- `optionalDependencies: ["agent"]`（Agent 节点与 graph 关联）。

## 15. Dependents
- `src/capability/index.ts`、`src/capability/profiles.ts`、`src/capability/platform/catalog.ts`。
- `MainArea.vue` 仅注释引用。

## 16. Frontend Boundary
- 贡献组件：`GraphPanel.vue`（WORKBENCH_MAIN，view=`graph`）+ 4 个子组件。
- 注册：`registerGraphContributions()`，懒加载 GraphPanel。
- **CURRENT PHYSICAL LOCATION**：全部 5 个 .vue 已在 `src/capabilities/graph/ui/`（已确认 `src/components` 下 0 残留，全迁入）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/graph.rs`（实现）+ `src-tauri/src/bridge.rs` 命令壳（`graph_query`/`graph_node_get`/`graph_stats`，均经 `check_invocation_source`）。
- 已注册（main.rs）：`bridge::graph_query`/`graph_node_get`/`graph_stats`。
- 前端封装：`src/bridge.ts`（`graphQuery→graph_query` / `graphNodeGet→graph_node_get` / `graphStats→graph_stats`）；`GRAPH_COMMANDS_AVAILABLE=true`。
- **诚实声明**：Native 仍集中于 `bridge.rs`/`graph.rs`，未物理模块化；本能力无独立原生模块。

## 18. Resources
- `resources.class: ["MEDIUM"]`；`suspendable: true`；`destroyable: true`。
- `v1.resources: []`（只读快照，不创建 owned 资源）。
- `persistence.scope: "disk"`（**实为只读 `graph.json` 快照，无写盘** —— 命名与「无写盘」诚实边界有张力，文档注明）。

## 19. Side Effects
- 当前无（纯只读查询）；`load_snapshot` 失败回退空 store，不影响其它域。

## 20. Security
- 输出白名单删 props（K7），防敏感字段泄漏。
- `check_invocation_source` 校验调用来源。

## 21. Persistence
- 声明 `disk`；实际为只读 `data_dir/graph.json` 快照，无写盘、无前端落盘。

## 22. Failure Model
- 后端未就绪：`backendReady=false` → `guard()` 拦截，UI 显示未就绪。
- 快照载入失败：回退空 store，`truncated`/`error` 提示。

## 23. Capability Absence
- Absent 时：`registerGraphContributions` 未执行 → WORKBENCH_MAIN 无 view=`graph` → MainArea 不渲染。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`installPolicy: static`。

## 25. UI Contributions
- `graph.main.panel`（WORKBENCH_MAIN / surface / view=`graph` / GraphPanel）。

## 26. Testing
- `src/capabilities/graph/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 原生侧：`src-tauri/src/graph.rs` 含 `#[cfg(test)]` 内核单测。

## 27. Gates
- `scripts/check-graph-policy.py`（**强**，强制 `GRAPH_NO_SECOND_PATH`，7~8 个 ACTIVE 码）。
- `scripts/check-graph-ui-logic.mjs`（**强**，21.98 KB，前端逻辑门禁）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useGraphStore.ts` → `ui/GraphPanel.vue` → 后端 `graph.rs`。
- 关注点：只读边界、白名单删 props、容量上限、absence 门禁缺失。

## 29. AI Modification Guide
- 改图结构：必须保持 `GRAPH_NO_SECOND_PATH` 零写，并同步 `check-graph-policy.py` 断言。
- 禁止：把 store 移出时不同步 manifest、用裸 `invoke`、新增 UI 不进 Contribution Registry。
- 若业务确需写/导出图谱，需经新 wave 在 DTO 上扩字段并新增后端（非当前范围）。

## 30. Known Debt
- C2→C3 缺口（`manifest` 注释）：① 无 graph 专属 absence 运行时门禁；② `mainView='graph'` 导航硬编码。
- `persistence.scope="disk"` 实为只读快照，命名与「无写盘」边界有张力（建议文档注明，已在本 README §18/§21 注明）。
- 后端只读快照、无实时反查。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`，诚实自述非 C3（缺 absence 门禁 + nav 硬编码）；有强 checker。
- **HP = HP0**：`manifest.v1.hotPlug.level="HP0"`（全 false）；只读快照，无运行时装卸需求声明。
- **M = M1**：`src/capabilities/graph/` 目录隔离（state/ui 均在包内，旧 `src/components` 0 残留）。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2（强） + RV3（否）**：owner 已登记（RV1）；`check-graph-policy.py` + `check-graph-ui-logic.mjs` 均定向覆盖（RV2 强）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、absence 门禁缺失。
- 物理隔离完备，可作 Package Extraction 范本候选。

## 33. Source of Truth
- manifest：`src/capabilities/graph/manifest.ts`
- public：`src/capabilities/graph/public.ts`
- state：`src/capabilities/graph/state/useGraphStore.ts`
- UI：`src/capabilities/graph/ui/`
- native：`src-tauri/src/graph.rs`、`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useGraphStore"`
- gates：`scripts/check-graph-policy.py`、`scripts/check-graph-ui-logic.mjs`（见 §27）
