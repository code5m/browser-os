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
  description: "系统应用枚举（.desktop 扫描）与启动。启动经 security_policy::check_launch_target 白名单式解析（禁 sh -c）+ 审计。",
  limitationReason: "HP0(STATIC)：启动的外部应用进程为 detached；无独立生命周期可装卸，未验证 absent 无残留前不宣称 HP1。",
})
