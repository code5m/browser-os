// Capability Contribution types（Phase 8B.1 — Generic Contribution Registry）
//
// 通用贡献契约：Shell 经「槽（slot）」消费能力贡献，而**不持有任何能力的专属知识**。
// 能力（如 Bookmark）向 Registry 注册 Contribution；Shell 只按 slot 遍历渲染。
//
// 设计边界（与 Semantic Governance 协调）：
//   - 本文件只定义「编排层」契约，不持有、不读取任何业务状态。
//   - panelOpen 等**业务真源**仍在 Semantic Registry 登记的 owner（如 useBookmarkStore）手中，
//     本契约不创建第二真源；能力内部 UI 读取自己的 store 是允许的（在能力包内）。
//   - component 是 Vue 组件（或 defineAsyncComponent 懒加载器）：由能力提供，Shell 经 <component :is> 渲染。

import type { Component } from "vue"

/** 贡献类型：surface=面板/区域占位；navigation=工具栏/导航项 */
export type ContributionType = "surface" | "navigation"

/**
 * 通用槽名（Shell 与能力共享的约定字符串）。
 * Shell 只认槽名，不认能力；能力把组件挂到槽上。
 */
export interface ContributionSlots {
  /** 浏览器主视图左侧栏（收藏夹侧栏等） */
  readonly BROWSER_SIDEBAR: "browser-sidebar"
  /** 地址栏操作区（⭐ 收藏按钮等） */
  readonly ADDRESS_BAR_ACTIONS: "address-bar-actions"
  /** 顶部工具栏尾部（收藏夹入口按钮等） */
  readonly ACTIVITY_BAR_TRAILING: "activity-bar-trailing"
}

export const CONTRIBUTION_SLOTS = {
  BROWSER_SIDEBAR: "browser-sidebar",
  ADDRESS_BAR_ACTIONS: "address-bar-actions",
  ACTIVITY_BAR_TRAILING: "activity-bar-trailing",
} as const satisfies ContributionSlots

/** 一条能力贡献 */
export interface Contribution {
  /** 全局唯一 id（建议 `${capabilityId}.${name}`） */
  id: string
  /** 来源能力 id（须与 CapabilityDefinition.id 一致） */
  capabilityId: string
  type: ContributionType
  /** 槽名：决定 Shell 在哪里渲染它 */
  slot: string
  /** Vue 组件或异步组件加载器（由能力提供） */
  component?: Component | (() => Promise<Component>)
  /** 同槽多贡献排序（小在前） */
  order?: number
}
