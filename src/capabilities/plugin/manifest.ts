import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const pluginManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "plugin",
  name: "插件",
  semanticOwner: "usePluginStore",
  maturity: "C3",
  version: "1.0.0",
  maturityEvidence: ["scripts/check-plugin-policy.py", "scripts/check-plugin-ui-logic.mjs", "scripts/check-hot-plug-acceptance.mjs"],
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
  description: "插件登记簿；无动态加载/执行运行时。",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：本能力仅管理插件登记簿，不持有插件执行运行时；进行中的管理操作会阻止暂停。",
})
