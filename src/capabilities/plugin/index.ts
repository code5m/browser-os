import { pluginManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"
import { activatePluginLifecycle, suspendPluginLifecycle } from "./lifecycle"
export const PLUGIN_CAPABILITY_ID = "plugin"
const registerPlugin = createLazyContributionRegistrar(pluginManifest, [
  { id: "plugin.main.panel", type: "surface", slot: "workbench-main", view: "plugin", label: "插件", icon: "🔌", panelState: true, load: () => import("./ui/PluginManager.vue") },
])
export function registerPluginContributions(): void { registerPlugin() }
export const pluginCapability = {
  ...pluginManifest,
  lifecycle: {
    ...pluginManifest.lifecycle,
    onActivate: () => {
      activatePluginLifecycle()
      registerPluginContributions()
    },
    onSuspend: suspendPluginLifecycle,
    onDeactivate: suspendPluginLifecycle,
  },
}
