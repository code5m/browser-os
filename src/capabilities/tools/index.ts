import { toolsManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"
import { activateToolsLifecycle, suspendToolsLifecycle } from "./lifecycle"
export const TOOLS_CAPABILITY_ID = "tools"
const registerTools = createLazyContributionRegistrar(toolsManifest, [
  { id: "tools.main.panel", type: "surface", slot: "workbench-main", view: "tools", label: "工具箱", icon: "🧰", panelState: true, load: () => import("./ui/ToolBox.vue") },
])
export function registerToolsContributions(): void { registerTools() }
export const toolsCapability = {
  ...toolsManifest,
  lifecycle: {
    ...toolsManifest.lifecycle,
    onActivate: () => {
      activateToolsLifecycle()
      registerToolsContributions()
    },
    onSuspend: suspendToolsLifecycle,
    onDeactivate: suspendToolsLifecycle,
  },
}
