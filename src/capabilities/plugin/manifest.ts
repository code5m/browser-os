import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const pluginManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "plugin",
  name: "插件",
  semanticOwner: "usePluginStore",
  maturity: "C3",
  version: "1.0.0",
  provides: ["plugin.list", "plugin.manifest"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["HEAVY", "NATIVE"],
  suspendable: true,
  destroyable: true,
  persistence: { scope: "disk", sensitive: false },
  contributions: [{ id: "plugin.main.panel", slot: "workbench-main", type: "surface", view: "plugin" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/plugin/public.ts" }],
  deactivationPolicy: "graceful",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：管理操作中拒绝暂停。",
})
