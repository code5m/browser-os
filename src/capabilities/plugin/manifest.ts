import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const pluginManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "plugin",
  name: "插件",
  semanticOwner: "usePluginStore",
  maturity: "C2",
  version: "1.0.0",
  maturityEvidence: ["scripts/check-plugin-policy.py", "scripts/check-plugin-ui-logic.mjs"],
  provides: ["plugin.list", "plugin.manifest"],
  resident: false,
  resourceClass: ["HEAVY", "NATIVE"],
  suspendable: false,
  destroyable: false,
  persistence: { scope: "disk", sensitive: false },
  contributions: [{ id: "plugin.main.panel", slot: "workbench-main", type: "surface", view: "plugin" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/plugin/public.ts" }],
  deactivationPolicy: "graceful",
  description: "插件登记簿；无动态加载/执行运行时。",
  limitationReason: "HP0：无可装卸插件运行时。",
})
