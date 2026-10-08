import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const appsManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "apps",
  name: "系统应用",
  semanticOwner: "useAppsStore",
  maturity: "C3",
  version: "1.0.0",
  provides: ["apps.list", "apps.launch"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["LIGHT", "PROCESS"],
  suspendable: true,
  destroyable: true,
  permissions: ["process.spawn"],
  persistence: { scope: "runtime_only", sensitive: false },
  contributions: [{ id: "apps.main.panel", slot: "workbench-main", type: "surface", view: "apps" }],
  manifestResources: [],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/apps/public.ts" }],
  deactivationPolicy: "graceful",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：已启动应用归 OS。",
})
