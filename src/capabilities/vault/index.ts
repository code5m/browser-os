import { defineAsyncComponent, h } from "vue";
import { vaultManifest } from "./manifest";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";
import type { Capability } from "../../capability/types";

export const VAULT_CAPABILITY_ID = "vault";

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
};
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
};
const VaultPanel = defineAsyncComponent({
  loader: () => import("./ui/VaultPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});

/**
 * 注册 Vault 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='vault'）。
 * Vault absent → 槽中无 view='vault' 贡献 → MainArea 的 viewOf('vault') 返回 undefined → 不渲染。
 */
export function registerVaultContributions(): void {
  contributionRegistry.registerContribution({
    id: "vault.main",
    capabilityId: VAULT_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "vault",
    label: "笔记库",
    icon: "📓",
    component: VaultPanel,
  });
}

export const vaultCapability: Capability = {
  id: VAULT_CAPABILITY_ID,
  manifest: vaultManifest,
  registerContributions(reg) {
    reg.registerContribution({
      id: "vault.main",
      capabilityId: VAULT_CAPABILITY_ID,
      type: "surface",
      slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
      view: "vault",
      label: "笔记库",
      icon: "📓",
      component: VaultPanel,
    });
  },
};

// 模块加载即注册 Vault 贡献（与 home 同模式：runtime.activate 不调 def.registerContributions，
// 且本能力对象未展开 manifest，故不能走 onActivate 路径）。否则贡献注册表无 view='vault' 条目
// → MainArea 的 viewOf('vault') 为 undefined → VaultPanel 不渲染（STAGE I-A 迁移遗漏）。
registerVaultContributions();
