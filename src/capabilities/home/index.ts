import { defineAsyncComponent, h } from "vue";
import { homeManifest } from "./manifest";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";

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

// Home 采用与 apps/tools/bookmark 一致的显式注册模式：
// 能力描述自己的贡献（registerHomeContributions），由 Host 在能力激活时（lifecycle.onActivate）调用，
// 不再依赖模块加载副作用。Home 已纳入 full profile（见 src/capability/profiles.ts），
// 激活路径与贡献注册一一对应，absent 能力不会污染贡献注册表。
export const homeCapability = {
  ...homeManifest,
  lifecycle: {
    ...homeManifest.lifecycle,
    onActivate: registerHomeContributions,
  },
};
