// Task 公共契约（Public Contract）—— 能力对外唯一稳定面。
//
// 原则（§6/§26）：禁止创造第二状态真源。本文件只**再导出**语义 owner 的既有 store（useTaskStore），
// 不创建 runtime.taskOpen / taskVisible 之类镜像状态。
export { useTaskStore } from "./state/useTaskStore"
export { taskManifest } from "./manifest"
export type { TaskDef, TaskRunRecord } from "../../types"
