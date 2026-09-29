import { defineAsyncComponent, h } from "vue";
import { settingsManifest } from "./manifest";
import { contributionRegistry } from "../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../capability/contribution/types";
import type { CapabilityDefinition } from "../capability/types";

export const SETTINGS_CAPABILITY_ID = "settings";

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
};
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
};
const SettingsPanel = defineAsyncComponent({
  loader: () => import("../components/system/SettingsPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});
const CapabilityManagerPanel = defineAsyncComponent(() => import("../components/system/CapabilityManagerPanel.vue"));

/**
 * 注册 Settings 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='settings'）。
 * settings 为常驻框架 SERVICE，贡献无条件注册；MainArea 经 viewOf('settings') 渲染，
 * 不再静态 import SettingsPanel。
 */
export function registerSettingsContributions(): void {
  contributionRegistry.registerContribution({
    id: "settings.main",
    capabilityId: SETTINGS_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "settings",
    label: "设置",
    icon: "⚙️",
    component: SettingsPanel,
  });
  contributionRegistry.registerContribution({
    id: "settings.capability-manager",
    capabilityId: SETTINGS_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "capability-manager",
    label: "Capability Manager",
    icon: "▦",
    component: CapabilityManagerPanel,
  });
}

export const settingsCapability: CapabilityDefinition = {
  ...settingsManifest,
};
