// Skill 公共契约（Public Contract）—— 能力对外唯一稳定面。
//
// 原则（§6/§26）：禁止创造第二状态真源。本文件只**再导出**语义 owner 的既有 store（useSkillStore），
// 不创建 runtime.skillOpen / skillVisible 之类镜像状态。调用方一律经此路径消费 Skill 状态与意图。
// Skill 不再经 Agent store 暴露（STAGE E 已解除纠缠）。
export { useSkillStore } from "./state/useSkillStore"
export { skillManifest } from "./manifest"
export type { SkillDef } from "../../types"
