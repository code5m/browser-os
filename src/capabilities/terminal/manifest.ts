import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const terminalManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "terminal",
  name: "终端",
  semanticOwner: "useTerminalStore",
  maturity: "C3",
  version: "1.0.0",
  provides: ["terminal.spawn", "terminal.write", "terminal.kill", "terminal.resize", "terminal.grid"],
  dependencies: ["bridge"],
  v1Dependencies: [],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["PROCESS", "PTY"],
  suspendable: true,
  destroyable: true,
  permissions: ["process.spawn"],
  persistence: { scope: "session", sensitive: false },
  contributions: [
    { id: "terminal.main.term", slot: "workbench-main-resident", type: "surface", view: "term" },
    { id: "terminal.dock.term", slot: "browser-dock", type: "surface", view: "term" },
  ],
  manifestResources: [
    { kind: "PTY", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
    { kind: "CHILD_PROCESS", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
  ],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/terminal/public.ts" }],
  deactivationPolicy: "graceful",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：活动 PTY 拒绝暂停/停用。",
})
