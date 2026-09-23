// Clipboard Capability — 模块入口（Capability Library Expansion v1, STAGE H）
// 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='clip'）贡献 ClipboardPanel 给 MainArea。
// 适配器不持有业务状态；absent → 槽中无 view='clip' → 不渲染 → useClipboardStore 不实例化 → 零 bridge.clipboard* invoke。
import { defineAsyncComponent, h } from "vue"
import { clipboardManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const CLIPBOARD_CAPABILITY_ID = "clipboard"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const ClipboardPanel = defineAsyncComponent({
  loader: () => import("./ui/ClipboardPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

export function registerClipboardContributions(): void {
  contributionRegistry.registerContribution({
    id: "clipboard.main.panel",
    capabilityId: CLIPBOARD_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "clip",
    label: "剪贴板",
    icon: "📋",
    component: ClipboardPanel,
  })
}

export const clipboardCapability = {
  ...clipboardManifest,
  lifecycle: {
    ...clipboardManifest.lifecycle,
    onActivate: registerClipboardContributions,
  },
}
