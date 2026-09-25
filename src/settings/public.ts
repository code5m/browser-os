// Settings 公共契约（STAGE I-C — 框架/平台 SERVICE）
//
// Shell（App.vue / StatusBar.vue）与 SettingsPanel 经由此显式契约访问框架偏好，
// 而非任意组件直接 import 内部 store 路径。
// 不提供 service locator / getSettingsStore()；复用 store 本身作为受控出口
// （与 capabilities/*/public.ts 同构）。
export { useSettingsStore } from "./state/useSettingsStore";
export type { KeymapScheme, Theme } from "./state/useSettingsStore";
