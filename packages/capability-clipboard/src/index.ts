import { defineAsyncComponent } from "vue";
import { clipboardManifest } from "./manifest";
import type { CapabilityDefinition, ClipboardContribution } from "./types";
import {
  CLIPBOARD_PORTS_KEY,
  assertClipboardPorts,
  type ClipboardPorts,
} from "./ports";
import { activateClipboardLifecycle, suspendClipboardLifecycle } from "./lifecycle";

export const CLIPBOARD_CAPABILITY_ID = "clipboard";

export { CLIPBOARD_PORTS_KEY } from "./ports";
export type {
  ClipboardPorts,
  ClipboardNativePort,
  ClipboardUiPort,
} from "./ports";
export { useClipboardStore } from "./state/useClipboardStore";
export type { ClipItem } from "./state/useClipboardStore";
export { clipboardManifest } from "./manifest";

// 面板懒加载（与 Vault 同）：store 经动态 import 触达，保证 store 模块顶层
// inject 在 app provide 作用域内执行（PHASE 0 timing 约束）。
const panelLoading = {
  template: "<div style='padding:16px'>加载剪贴板面板…</div>",
};
const panelError = {
  template: "<div style='padding:16px;color:#b00'>剪贴板面板加载失败</div>",
};
export const ClipboardPanel = defineAsyncComponent({
  loader: () => import("./ui/ClipboardPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 0,
  timeout: 5000,
});

// 贡献描述子：包只导出，Host 单点注册（禁止包直连 contributionRegistry）。
export const clipboardContribution: ClipboardContribution = {
  id: "clipboard.main.panel",
  capabilityId: CLIPBOARD_CAPABILITY_ID,
  type: "surface",
  slot: "workbench-main",
  view: "clip",
  label: "剪贴板",
  icon: "📋",
  component: ClipboardPanel,
};

// 工厂：捕获 Host 注入的端口，返回能力定义（onActivate 由 Host 绑定注册）。
export function createClipboardCapability(ports: ClipboardPorts): {
  clipboardCapability: CapabilityDefinition;
  clipboardContribution: ClipboardContribution;
  clipboardManifest: CapabilityDefinition;
} {
  assertClipboardPorts(ports);
  const clipboardCapability: CapabilityDefinition = {
    ...clipboardManifest,
    lifecycle: {
      ...clipboardManifest.lifecycle,
      onActivate: activateClipboardLifecycle,
      onSuspend: suspendClipboardLifecycle,
      onDeactivate: suspendClipboardLifecycle,
    },
  };
  return {
    clipboardCapability,
    clipboardContribution,
    clipboardManifest,
  };
}
