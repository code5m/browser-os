// Apps 公共契约（Public Contract）—— 能力对外唯一稳定面。
// 仅再导出语义 owner（useAppsStore），不创建镜像状态（SECOND_TRUTHS=0）。
export { useAppsStore } from "./state/useAppsStore"
export { appsManifest } from "./manifest"
export type { AppEntry } from "../../types"
