import { taskManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"
export const TASK_CAPABILITY_ID = "task"
const registerTask = createLazyContributionRegistrar(taskManifest, [
  { id: "task.main.panel", type: "surface", slot: "workbench-main", view: "tasks", label: "定时任务", icon: "⏱️", panelState: true, load: () => import("./ui/TaskPanel.vue") },
])
export function registerTaskContributions(): void { registerTask() }
export const taskCapability = { ...taskManifest, lifecycle: { ...taskManifest.lifecycle, onActivate: registerTaskContributions } }
