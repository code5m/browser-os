// Agent Capability — 模块入口（Capability Library Expansion v1）
//
// 经 **通用 Contribution Registry** 向 MainArea 贡献主视图 UI：
//   - workbench-main（surface, view='agents'）  → 主工作区「智能体」视图的 AgentManagerPanel
// MainArea 只按槽 + view 渲染，不 import 本能力内部（C3 关键）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/ 与 store 内。
// Agent absent（未注册/未 activate）→ workbench-main 槽中无 view='agents' 贡献 → MainArea 不渲染 AgentManagerPanel。
//
// 已知缺口：执行后端未实现（AGENT_SKILL_COMMANDS_AVAILABLE=false），chat/run 走 guard 拦截；
// 本文件仅负责边界与贡献，不伪装执行能力（§8 不得高报成熟度）。

import { defineAsyncComponent, h } from "vue"
import { agentManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const AGENT_CAPABILITY_ID = "agent"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const AgentManagerPanel = defineAsyncComponent({
  loader: () => import("./ui/AgentManagerPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

/**
 * 注册 Agent 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='agents'）。
 * Agent absent → 槽中无 view='agents' 贡献 → MainArea 的 viewOf('agents') 返回 undefined → 不渲染。
 */
export function registerAgentContributions(): void {
  contributionRegistry.registerContribution({
    id: "agent.main.panel",
    capabilityId: AGENT_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "agents",
    label: "智能体",
    icon: "🤖",
    component: AgentManagerPanel,
  })
}

export const agentCapability = {
  ...agentManifest,
  lifecycle: {
    ...agentManifest.lifecycle,
    onActivate: registerAgentContributions,
  },
}
