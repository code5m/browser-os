import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const appsManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "apps",
  name: "系统应用",
  semanticOwner: "useAppsStore",
  maturity: "C2",
  version: "1.0.0",
  maturityEvidence: ["scripts/check-home-client-policy.py", "scripts/check-ui-boundaries.mjs"],
  provides: ["apps.list", "apps.launch"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["LIGHT", "PROCESS"],
  suspendable: true,
  destroyable: true,
  permissions: ["process.spawn"],
  persistence: { scope: "runtime_only", sensitive: false },
  contributions: [{ id: "apps.main.panel", slot: "workbench-main", type: "surface", view: "apps" }],
  manifestResources: [{ kind: "CHILD_PROCESS", ownership: "owned", evidence: "src-tauri/src/bridge.rs:launch_app (Command::spawn, detached)" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/apps/public.ts" }],
  deactivationPolicy: "graceful",
  description: "系统应用枚举与受策略保护的启动。",
  limitationReason: "HP0：外部进程 detached，未纳入可逆生命周期。",
})
