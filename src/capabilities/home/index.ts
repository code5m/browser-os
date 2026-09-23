import { defineAsyncComponent, h } from "vue";
import { homeManifest } from "./manifest";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";
import type { Capability } from "../../capability/types";

export const HOME_CAPABILITY_ID = "home";

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
};
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
};
const HomePanel = defineAsyncComponent({
  loader: () => import("./ui/HomePanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});

/**
 * 注册 Home 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='home'）。
 * Home absent → 槽中无 view='home' 贡献 → viewOf('home') 返回 undefined → 不渲染。
 */
export function registerHomeContributions(): void {
  contributionRegistry.registerContribution({
    id: "home.main",
    capabilityId: HOME_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "home",
    label: "主页",
    icon: "🏠",
    component: HomePanel,
  });
}

export const homeCapability: Capability = {
  id: HOME_CAPABILITY_ID,
  manifest: homeManifest,
  registerContributions(reg) {
    reg.registerContribution({
      id: "home.main",
      capabilityId: HOME_CAPABILITY_ID,
      type: "surface",
      slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
      view: "home",
      label: "主页",
      icon: "🏠",
      component: HomePanel,
    });
  },
};
