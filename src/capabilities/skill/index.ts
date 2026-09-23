// Skill Capability — 模块入口（Capability Library Expansion v1, STAGE E）
//
// 经 **通用 Contribution Registry** 向 MainArea 贡献主视图 UI：
//   - workbench-main（surface, view='skills'）  → 主工作区「技能」视图的 SkillManagerPanel
// MainArea 只按槽 + view 渲染，不 import 本能力内部（C3 关键）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/ 与 store 内。
// Skill absent → workbench-main 槽中无 view='skills' 贡献 → MainArea 不渲染 SkillManagerPanel。
//
// 已知缺口：执行后端未实现（AGENT_SKILL_COMMANDS_AVAILABLE=false），list/run 走 guard 拦截。

import { defineAsyncComponent, h } from "vue"
import { skillManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const SKILL_CAPABILITY_ID = "skill"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const SkillManagerPanel = defineAsyncComponent({
  loader: () => import("./ui/SkillManagerPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

/**
 * 注册 Skill 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='skills'）。
 * Skill absent → 槽中无 view='skills' 贡献 → MainArea 的 viewOf('skills') 返回 undefined → 不渲染。
 */
export function registerSkillContributions(): void {
  contributionRegistry.registerContribution({
    id: "skill.main.panel",
    capabilityId: SKILL_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "skills",
    label: "技能",
    icon: "🛠️",
    component: SkillManagerPanel,
  })
}

export const skillCapability = {
  ...skillManifest,
  lifecycle: {
    ...skillManifest.lifecycle,
    onActivate: registerSkillContributions,
  },
}
