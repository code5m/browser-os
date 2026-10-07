import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const scriptManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "script",
  name: "脚本",
  semanticOwner: "useScriptStore",
  maturity: "C2",
  provides: ["script.run", "script.snippet"],
  resident: false,
  resourceClass: ["PROCESS"],
  suspendable: false,
  destroyable: true,
  permissions: ["process.spawn"],
  persistence: { scope: "disk", sensitive: false },
  contributions: [
    { id: "script.main.panel", slot: "workbench-main", type: "surface", view: "scripts" },
    { id: "script.commands.panel", slot: "workbench-main", type: "surface", view: "commands" },
  ],
  manifestResources: [
    { kind: "CHILD_PROCESS", ownership: "owned", evidence: "src-tauri/src/capabilities/script/script_runner.rs" },
  ],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/script/public.ts" }],
  deactivationPolicy: "graceful",
  limitationReason: "Child-process hot unplug is not proven.",
})
