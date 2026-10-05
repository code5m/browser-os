import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const graphManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "graph",
  name: "知识图谱",
  semanticOwner: "useGraphStore",
  maturity: "C2",
  version: "1.0.0",
  maturityEvidence: ["scripts/check-graph-policy.py", "scripts/check-graph-ui-logic.mjs"],
  provides: ["graph.query", "graph.node.get", "graph.stats"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["MEDIUM"],
  suspendable: true,
  destroyable: true,
  persistence: { scope: "disk", sensitive: false },
  contributions: [{ id: "graph.main.panel", slot: "workbench-main", type: "surface", view: "graph" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/graph/public.ts" }],
  deactivationPolicy: "graceful",
  description: "图谱查询 / 节点获取 / 容量概览（只读）。后端 graph_query/graph_node_get/graph_stats 已实现；无 worker/webview/写盘。",
  limitationReason: "HP0(STATIC)：图存储为启动期只读载入的内存快照，无独立生命周期可装卸；未验证 absent 无残留前不宣称 HP1。",
})
