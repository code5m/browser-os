// Apps Capability — 模块入口（Capability Library Expansion v1, STAGE H）
// 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='apps'）贡献 AppPanel 给 MainArea。
// absent → 槽中无 view='apps' → 不渲染 → useAppsStore 不实例化 → 零 bridge.listApps/launchApp invoke。
import { defineAsyncComponent, h } from "vue"
import { appsManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const APPS_CAPABILITY_ID = "apps"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const AppPanel = defineAsyncComponent({
  loader: () => import("./ui/AppPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

export function registerAppsContributions(): void {
  contributionRegistry.registerContribution({
    id: "apps.main.panel",
    capabilityId: APPS_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "apps",
    label: "系统应用",
    icon: "🚀",
    component: AppPanel,
  })
}

export const appsCapability = {
  ...appsManifest,
  lifecycle: {
    ...appsManifest.lifecycle,
    onActivate: registerAppsContributions,
  },
}
