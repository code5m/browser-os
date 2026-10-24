import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const bfcManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "bfc",
  name: "BFC 内容库",
  semanticOwner: "BfcProjectPanel",
  maturity: "C3",
  version: "1.0.0",
  provides: ["bfc.web", "bfc.workbench"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["LIGHT"],
  suspendable: true,
  destroyable: true,
  persistence: { scope: "runtime_only", sensitive: false },
  contributions: [{ id: "bfc.main.panel", slot: "workbench-main", type: "surface", view: "bfc" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/bfc/public.ts" }],
  deactivationPolicy: "graceful",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：停用后移除入口，不影响已打开的网页页签。",
})
