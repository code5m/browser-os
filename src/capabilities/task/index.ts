// Task Capability — 模块入口（Capability Library Expansion v1, STAGE G）
//
// 经 **通用 Contribution Registry** 向 MainArea 贡献主视图 UI：
//   - workbench-main（surface, view='tasks'） → 主工作区「定时任务」视图的 TaskPanel
// MainArea 只按槽 + view 渲染，不 import 本能力内部（C3 关键）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/ 与 state/ 内。
// Task absent → workbench-main 槽中无 view='tasks' 贡献 → MainArea 不渲染 TaskPanel
//   → useTaskStore 不被实例化 → 零 bridge.task* invoke（absent 语义）。
//
// 面板懒加载（defineAsyncComponent）保留在本适配器内，TaskPanel + store 拆主 chunk（IF-2 体积闸门）。

import { defineAsyncComponent, h } from "vue"
import { taskManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const TASK_CAPABILITY_ID = "task"

const panelLoading = {
  render: () =>
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const TaskPanel = defineAsyncComponent({
  loader: () => import("./ui/TaskPanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

/**
 * 注册 Task 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='tasks'）。
 * Task absent → 槽中无 view='tasks' 贡献 → MainArea 的 viewOf('tasks') 返回 undefined → 不渲染。
 */
export function registerTaskContributions(): void {
  contributionRegistry.registerContribution({
    id: "task.main.panel",
    capabilityId: TASK_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "tasks",
    label: "定时任务",
    icon: "⏱️",
    component: TaskPanel,
  })
}

export const taskCapability = {
  ...taskManifest,
  lifecycle: {
    ...taskManifest.lifecycle,
    onActivate: registerTaskContributions,
  },
}
