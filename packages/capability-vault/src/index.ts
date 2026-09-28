import { defineAsyncComponent, h } from "vue";
import { vaultManifest } from "./manifest";
import type { CapabilityDefinition, VaultContribution } from "./types";
import { VAULT_PORTS_KEY, assertVaultPorts, type VaultPorts } from "./ports";

export const VAULT_CAPABILITY_ID = "vault";

// 重新导出包内公共契约表面（Host 只经此文件消费 Vault）。
export { VAULT_PORTS_KEY } from "./ports";
export type {
  VaultPorts,
  VaultNativePort,
  VaultShellPort,
  VaultGraphLayoutPort,
  VaultOpenResult,
} from "./ports";

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
};
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
};

/**
 * Vault 主面板（WORKBENCH_MAIN 槽, view='vault'）。
 * 经 defineAsyncComponent 动态加载，Host 注册该贡献描述符后由 MainArea 按槽渲染。
 * 面板内部经 inject(VAULT_PORTS_KEY) 获取 Host 提供的表现层契约，不反向依赖 Host。
 */
export const VaultPanel = defineAsyncComponent({
  loader: () => import("./ui/VaultPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
});

/**
 * Vault 贡献描述符 —— 仅声明“自己提供什么”，由 Host 负责 contributionRegistry.register(...)。
 * 包本身不直接 import Host contribution registry（避免 Vault → Host registry 反向依赖）。
 */
export const vaultContribution: VaultContribution = {
  id: "vault.main",
  capabilityId: VAULT_CAPABILITY_ID,
  type: "surface",
  slot: "workbench-main",
  view: "vault",
  label: "笔记库",
  icon: "📓",
  component: VaultPanel,
};

/**
 * 显式工厂（用户裁决 A · MINIMAL PORTS & ADAPTERS）。
 * Host 必须提供窄 Host Contract；缺 port 时 fail-fast，绝不静默 fallback。
 * 工厂返回能力定义（onActivate 留空，由 Host 注入为 contributionRegistry.register(vaultContribution)）
 * 与贡献描述符，供 Host 编排。
 */
export function createVaultCapability(ports: VaultPorts): {
  vaultCapability: CapabilityDefinition;
  vaultContribution: VaultContribution;
  vaultManifest: CapabilityDefinition;
} {
  assertVaultPorts(ports);
  const vaultCapability: CapabilityDefinition = {
    ...vaultManifest,
    lifecycle: { ...vaultManifest.lifecycle, onActivate: undefined },
  };
  return { vaultCapability, vaultContribution, vaultManifest };
}
