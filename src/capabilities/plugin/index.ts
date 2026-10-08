import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activatePluginLifecycle, suspendPluginLifecycle } from "./lifecycle"
import { pluginManifest } from "./manifest"

export const PLUGIN_CAPABILITY_ID = "plugin"

const plugin = createLifecycleContributionCapability(pluginManifest, [
  { id: "plugin.main.panel", type: "surface", slot: "workbench-main", view: "plugin", label: "插件", icon: "🔌", panelState: true, load: () => import("./ui/PluginManager.vue") },
], activatePluginLifecycle, suspendPluginLifecycle)

export const pluginCapability = plugin.capability
export const registerPluginContributions = plugin.register
