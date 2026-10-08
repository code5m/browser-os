import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateToolsLifecycle, suspendToolsLifecycle } from "./lifecycle"
import { toolsManifest } from "./manifest"

export const TOOLS_CAPABILITY_ID = "tools"

const tools = createLifecycleContributionCapability(toolsManifest, [
  { id: "tools.main.panel", type: "surface", slot: "workbench-main", view: "tools", label: "工具箱", icon: "🧰", panelState: true, load: () => import("./ui/ToolBox.vue") },
], activateToolsLifecycle, suspendToolsLifecycle)

export const toolsCapability = tools.capability
export const registerToolsContributions = tools.register
