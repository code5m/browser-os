// Tools Capability — 模块入口（Capability Library Expansion v1, STAGE H）
// 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='tools'）贡献 ToolBox 给 MainArea。
// absent → 槽中无 view='tools' → 不渲染 → useToolsStore 不实例化 → 零 bridge.listTools/openTool invoke。
import { defineAsyncComponent, h } from "vue"
import { toolsManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const TOOLS_CAPABILITY_ID = "tools"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const ToolBox = defineAsyncComponent({
  loader: () => import("./ui/ToolBox.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

export function registerToolsContributions(): void {
  contributionRegistry.registerContribution({
    id: "tools.main.panel",
    capabilityId: TOOLS_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "tools",
    label: "工具箱",
    icon: "🧰",
    component: ToolBox,
  })
}

export const toolsCapability = {
  ...toolsManifest,
  lifecycle: {
    ...toolsManifest.lifecycle,
    onActivate: registerToolsContributions,
  },
}
