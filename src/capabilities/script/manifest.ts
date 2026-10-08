import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const scriptManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "script",
  name: "脚本",
  semanticOwner: "useScriptStore",
  maturity: "C3",
  provides: ["script.run", "script.snippet"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["PROCESS"],
  suspendable: true,
  destroyable: true,
  permissions: ["process.spawn"],
  persistence: { scope: "disk", sensitive: false },
  contributions: [
    { id: "script.main.panel", slot: "workbench-main", type: "surface", view: "scripts" },
    { id: "script.commands.panel", slot: "workbench-main", type: "surface", view: "commands" },
  ],
  manifestResources: [
    { kind: "CHILD_PROCESS", ownership: "shared", evidence: "script_runner is shared with Task scheduler; Script UI tracks only manually started runs" },
  ],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/script/public.ts" }],
  deactivationPolicy: "graceful",
  maturityEvidence: ["scripts/check-hot-plug-acceptance.mjs"],
  hotPlugLevel: "HP2",
  limitationReason: "HP2：手工运行中拒绝暂停。",
})
