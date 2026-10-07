import { appsManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"

export const APPS_CAPABILITY_ID = "apps"

const registerApps = createLazyContributionRegistrar(appsManifest, [
  { id: "apps.main.panel", type: "surface", slot: "workbench-main", view: "apps", label: "系统应用", icon: "🚀", panelState: true, load: () => import("./ui/AppPanel.vue") },
])
export function registerAppsContributions(): void { registerApps() }

export const appsCapability = {
  ...appsManifest,
  lifecycle: { ...appsManifest.lifecycle, onActivate: registerAppsContributions },
}
