import { terminalManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"
import { activateTerminalLifecycle, suspendTerminalLifecycle } from "./lifecycle"
export const TERMINAL_CAPABILITY_ID = "terminal"
const registerTerminal = createLazyContributionRegistrar(terminalManifest, [
  { id: "terminal.main.term", type: "surface", slot: "workbench-main-resident", view: "term", load: () => import("./ui/TerminalView.vue") },
  { id: "terminal.dock.term", type: "surface", slot: "browser-dock", view: "term", label: "终端", icon: "💻", order: 20, load: () => import("./ui/TerminalDockPanel.vue") },
])
export function registerTerminalContributions(): void { registerTerminal() }
export const terminalCapability = {
  ...terminalManifest,
  lifecycle: {
    ...terminalManifest.lifecycle,
    onActivate: () => {
      activateTerminalLifecycle()
      registerTerminalContributions()
    },
    onSuspend: suspendTerminalLifecycle,
    onDeactivate: suspendTerminalLifecycle,
  },
}
