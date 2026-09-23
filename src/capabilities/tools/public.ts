// Tools 公共契约（Public Contract）—— 能力对外唯一稳定面。
// 仅再导出语义 owner（useToolsStore），不创建镜像状态（SECOND_TRUTHS=0）。
export { useToolsStore } from "./state/useToolsStore"
export { toolsManifest } from "./manifest"
export type { ToolMeta } from "../../types"
