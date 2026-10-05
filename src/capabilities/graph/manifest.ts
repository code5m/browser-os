import type { CapabilityDefinition } from "../../capability/types"

// KnowledgeGraph 能力 Manifest（Capability Library Expansion v1 — STAGE G）
// 语义 owner = useGraphStore（STAGE G 物理迁入 capabilities/graph/state，单一真源）。
// 物理：GraphPanel（+ GraphFilter/GraphViewer/NodeDetail/EdgeDetail）经通用 Contribution Registry
// 的 WORKBENCH_MAIN 槽（view='graph'）贡献给 MainArea。
//
// 命名：物理目录 `graph/` 对应 capability id **`graph`**（原 registry id `knowledge_graph` 含下划线，
// 不满足 Building Block Contract 的 ID 规则 `^[a-z][a-z0-9.]*$`；STAGE G 统一为 `graph`）。
//
// 诚实边界：后端 `graph_query` / `graph_node_get` / `graph_stats` **已实现**（`GRAPH_COMMANDS_AVAILABLE=true`），
// 只读查询启动期载入的内存图快照（`data_dir/graph.json`，只读）。**无 worker / 无子进程 / 无 webview / 无写盘**
// （门禁 `check-graph-policy.py` 的 `GRAPH_NO_SECOND_PATH` 强制）。K7 双闸：`GraphNodeView/GraphEdgeView` 删 props
// + 前端 `viewToNode/viewToEdge` 白名单。
//
// 成熟度：C2 ISOLATED（实现经 manifest/public/index/ui/state 边界隔离 + 贡献驱动）。
//   非 C3：① 无 graph 专属 absence 运行时门禁；② mainView='graph' 导航项仍硬编码未贡献驱动。
export const graphManifest: CapabilityDefinition = {
  id: "graph",
  name: "知识图谱",
  category: "CAPABILITY",
  provides: [
    "graph.query",
    "graph.node.get",
    "graph.stats",
  ],
  dependsOn: ["bridge"],
  // Agent ids are graph payload data; Graph does not depend on Agent runtime code.\n  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["MEDIUM"],
    suspendable: true,
    destroyable: true,
  },
  permissions: [],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useGraphStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）。
  v1: {
    id: "graph",
    version: "1.0.0",
    displayName: "知识图谱",
    description:
      "图谱查询 / 节点获取 / 容量概览（只读）。后端 graph_query/graph_node_get/graph_stats 已实现；无 worker/webview/写盘。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-graph-policy.py", "scripts/check-graph-ui-logic.mjs"],
    dependencies: ["bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["graph.query", "graph.node.get", "graph.stats"],
    requires: [],
    contributions: [{ id: "graph.main.panel", slot: "workbench-main", type: "surface", view: "graph" }],
    permissions: [],
    // 只读查询既有内存快照，不创建 owned 资源。
    resources: [],
    persistenceScope: "disk",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "graceful",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP0",
      enable: false,
      disable: false,
      register: false,
      unregister: false,
      install: false,
      uninstall: false,
      limitationReason:
        "HP0(STATIC)：图存储为启动期只读载入的内存快照，无独立生命周期可装卸；未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/graph/public.ts" }],
    entrypoint: "src/capabilities/graph/index.ts",
    semanticOwner: "useGraphStore",
  },
}
