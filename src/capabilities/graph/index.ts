// KnowledgeGraph Capability — 模块入口（Capability Library Expansion v1, STAGE G）
//
// 经 **通用 Contribution Registry** 向 MainArea 贡献主视图 UI：
//   - workbench-main（surface, view='graph'） → 主工作区「知识图谱」视图的 GraphPanel
// MainArea 只按槽 + view 渲染，不 import 本能力内部（C3 关键）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/ 与 state/ 内。
// Graph absent → workbench-main 槽中无 view='graph' 贡献 → MainArea 不渲染 GraphPanel
//   → useGraphStore 不被实例化 → 零 bridge.graph* invoke（absent 语义）。

import { defineAsyncComponent, h } from "vue"
import { graphManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const KNOWLEDGE_GRAPH_CAPABILITY_ID = "graph"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const GraphPanel = defineAsyncComponent({
  loader: () => import("./ui/GraphPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

/**
 * 注册 KnowledgeGraph 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='graph'）。
 * Graph absent → 槽中无 view='graph' 贡献 → MainArea 的 viewOf('graph') 返回 undefined → 不渲染。
 */
export function registerGraphContributions(): void {
  contributionRegistry.registerContribution({
    id: "graph.main.panel",
    capabilityId: KNOWLEDGE_GRAPH_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "graph",
    label: "知识图谱",
    icon: "🕸️",
    component: GraphPanel,
  })
}

export const graphCapability = {
  ...graphManifest,
  lifecycle: {
    ...graphManifest.lifecycle,
    onActivate: registerGraphContributions,
  },
}
