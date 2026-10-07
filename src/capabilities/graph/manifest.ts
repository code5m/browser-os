import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const graphManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "graph",
  name: "知识图谱",
  semanticOwner: "useGraphStore",
  maturity: "C3",
  version: "1.0.0",
  maturityEvidence: ["scripts/check-graph-policy.py", "scripts/check-graph-ui-logic.mjs", "scripts/check-hot-plug-acceptance.mjs"],
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
  description: "只读图谱查询与节点/容量概览。",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：停用取消在途查询；Harness 验证恢复/重启。",
})
