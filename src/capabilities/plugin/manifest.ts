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
  description: "插件 manifest 生命周期登记簿（list/get/install/enable/disable/keys）。运行时（解包/真验签/动态加载/执行）未实现，Stage-I LOCKED；ENABLED ≠ ACTIVE，RESOURCE_EXISTS 恒 false。",
  limitationReason: "HP0(STATIC)：plugin 运行时(loader/执行)未实现；无 runtime 实例可装卸，未验证 absent 无残留前不宣称 HP1。",
})
