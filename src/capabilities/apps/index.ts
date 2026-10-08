import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateAppsLifecycle, suspendAppsLifecycle } from "./lifecycle"
import { appsManifest } from "./manifest"

export const APPS_CAPABILITY_ID = "apps"

const apps = createLifecycleContributionCapability(appsManifest, [
  { id: "apps.main.panel", type: "surface", slot: "workbench-main", view: "apps", label: "系统应用", icon: "🚀", panelState: true, load: () => import("./ui/AppPanel.vue") },
], activateAppsLifecycle, suspendAppsLifecycle)

export const appsCapability = apps.capability
export const registerAppsContributions = apps.register
