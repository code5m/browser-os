// Plugin Capability — 模块入口（Capability Library Expansion v1, STAGE F）
//
// 经 **通用 Contribution Registry** 向 MainArea 贡献主视图 UI：
//   - workbench-main（surface, view='plugin'） → 主工作区「插件」视图的 PluginManager
// MainArea 只按槽 + view 渲染，不 import 本能力内部（C3 关键）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/ 与 state/ 内。
// Plugin absent → workbench-main 槽中无 view='plugin' 贡献 → MainArea 不渲染 PluginManager
//   → usePluginStore 不被实例化 → 零 bridge.plugin* invoke（absent 语义）。
//
// 已知缺口：plugin 运行时（loader/执行）未实现（governanceStatus=LOCKED），
//   UI 为「本地 manifest 生命周期登记簿」；ENABLED ≠ ACTIVE，RESOURCE_EXISTS 恒 false。

import { defineAsyncComponent, h } from "vue"
import { pluginManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const PLUGIN_CAPABILITY_ID = "plugin"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const PluginManager = defineAsyncComponent({
  loader: () => import("./ui/PluginManager.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

/**
 * 注册 Plugin 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='plugin'）。
 * Plugin absent → 槽中无 view='plugin' 贡献 → MainArea 的 viewOf('plugin') 返回 undefined → 不渲染。
 */
export function registerPluginContributions(): void {
  contributionRegistry.registerContribution({
    id: "plugin.main.panel",
    capabilityId: PLUGIN_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "plugin",
    label: "插件",
    icon: "🔌",
    component: PluginManager,
  })
}

export const pluginCapability = {
  ...pluginManifest,
  lifecycle: {
    ...pluginManifest.lifecycle,
    onActivate: registerPluginContributions,
  },
}
