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
  description: "只读图谱查询、节点获取与容量概览。",
  limitationReason: "HP0：启动期图快照尚无独立装卸生命周期。",
})
