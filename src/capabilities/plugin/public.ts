// Plugin 公共契约（Public Contract）—— 能力对外唯一稳定面。
//
// 原则（§6/§26）：禁止创造第二状态真源。本文件只**再导出**语义 owner 的既有 store（usePluginStore），
// 不创建 runtime.pluginOpen / pluginVisible / pluginActive 之类镜像状态。
// 五态（AVAILABLE/INSTALLED/ENABLED/ACTIVE/RESOURCE_EXISTS）由 usePluginStore 的 detail 经
// utils/pluginUi.ts 的 pluginFacets() **派生投影**，非常驻状态，不构成第二真源。
export { usePluginStore } from "./state/usePluginStore"
export { pluginManifest } from "./manifest"
export type {
  PluginState,
  PluginSummary,
  PluginDetail,
  TrustedKeyRecord,
  PluginManifest,
} from "../../types"
