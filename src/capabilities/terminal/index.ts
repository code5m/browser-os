import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateTerminalLifecycle, suspendTerminalLifecycle } from "./lifecycle"
import { terminalManifest } from "./manifest"

export const TERMINAL_CAPABILITY_ID = "terminal"

const terminal = createLifecycleContributionCapability(terminalManifest, [
  { id: "terminal.main.term", type: "surface", slot: "workbench-main-resident", view: "term", load: () => import("./ui/TerminalView.vue") },
  { id: "terminal.dock.term", type: "surface", slot: "browser-dock", view: "term", label: "终端", icon: "💻", order: 20, load: () => import("./ui/TerminalDockPanel.vue") },
], activateTerminalLifecycle, suspendTerminalLifecycle)

export const terminalCapability = terminal.capability
export const registerTerminalContributions = terminal.register
