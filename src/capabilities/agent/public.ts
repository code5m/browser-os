// Agent 公共契约（Public Contract）—— 能力对外唯一稳定面。
//
// 原则（§6/§26）：禁止创造第二状态真源。本文件只**再导出**语义 owner 的既有 store，
// 不创建 runtime.agentOpen / agentVisible 之类镜像状态。调用方一律经此路径消费 Agent 状态与意图。
// 注意：useAgentStore 同时持有 skill 状态（skill 无专属 store），STAGE E 将处理 skill 拆分；
// 此处仅再导出，不复制状态。
export { useAgentStore } from "./state/useAgentStore"
export { agentManifest } from "./manifest"
export type { AgentDef } from "../../types"
