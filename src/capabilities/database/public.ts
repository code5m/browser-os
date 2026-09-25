// Database 公共契约（Public Contract）—— 能力对外唯一稳定面。
//
// 原则（§6/§26）：禁止创造第二状态真源。本文件只**再导出**语义 owner 的既有 store，
// 不创建 runtime.databaseOpen / databaseVisible 之类镜像状态。调用方（含 App.vue / 其他能力）
// 一律经此路径消费 Database 状态与意图，不得 import 能力内部。
export { useDatabaseStore } from "./state/useDatabaseStore"
export { databaseManifest } from "./manifest"
export type { DbKind } from "../../utils/dbUi"
