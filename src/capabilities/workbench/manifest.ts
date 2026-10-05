import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const workbenchManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "workbench",
  name: "工作台",
  semanticOwner: "useWorkbenchStore",
  maturity: "C2",
  provides: ["workbench.navigation", "workbench.commands"],
  resident: true,
  resourceClass: ["MEDIUM"],
  suspendable: false,
  destroyable: false,
  persistence: { scope: "disk", sensitive: false },
  contributions: [
    { id: "workbench.commands", slot: "commands", type: "surface" },
    { id: "workbench.rail", slot: "sidebar", type: "surface" },
  ],
  manifestResources: [
    { kind: "CACHE", ownership: "owned", evidence: "src/capabilities/workbench/state/useWorkbenchStore.ts" },
  ],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/workbench/public.ts" }],
  limitationReason: "Resident shell frame is not runtime-disableable.",
})
