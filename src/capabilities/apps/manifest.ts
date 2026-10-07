import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const appsManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "apps",
  name: "系统应用",
  semanticOwner: "useAppsStore",
  maturity: "C3",
  version: "1.0.0",
  maturityEvidence: ["scripts/check-home-client-policy.py", "scripts/check-ui-boundaries.mjs", "scripts/check-hot-plug-acceptance.mjs"],
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
  description: "系统应用枚举与受策略保护的启动。",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：启动后的系统应用已交给 OS，不属于 capability-owned runtime resource；Harness 验证入口撤销/恢复。",
})
