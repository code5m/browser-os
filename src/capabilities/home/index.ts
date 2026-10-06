import { homeManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"

export const HOME_CAPABILITY_ID = "home"

const registerHome = createLazyContributionRegistrar(homeManifest, [
  { id: "home.main", type: "surface", slot: "workbench-main", view: "home", label: "主页", icon: "🏠", panelState: true, load: () => import("./ui/HomePanel.vue") },
])
export function registerHomeContributions(): void { registerHome() }

export const homeCapability = {
  ...homeManifest,
  lifecycle: { ...homeManifest.lifecycle, onActivate: registerHomeContributions },
}
