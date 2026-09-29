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
  readonly COMMANDS: "commands"
  readonly MENUS: "menus"
  readonly SIDEBAR: "sidebar"
  readonly PANELS: "panels"
  readonly SETTINGS: "settings"
  /** 浏览器主视图左侧栏（收藏夹侧栏等） */
  readonly BROWSER_SIDEBAR: "browser-sidebar"
  /** 地址栏操作区（⭐ 收藏按钮等） */
  readonly ADDRESS_BAR_ACTIONS: "address-bar-actions"
  /** 顶部工具栏尾部（收藏夹入口按钮等） */
  readonly ACTIVITY_BAR_TRAILING: "activity-bar-trailing"
  /** 主工作区视图槽：能力按 `view`（=layout.mainView 值）认领一个主视图；Shell 只按 view 渲染 */
  readonly WORKBENCH_MAIN: "workbench-main"
  /**
   * 常驻主视图槽：能力提供的组件**始终挂载**（Shell 不按 view 卸载），显隐由能力组件内部
   * 自管（如绑定 layout.mainView）。用于「切走不得卸载」的视图（终端 xterm 实例保活）。
   * Shell 仍不持有任何能力专属知识：槽为空即不渲染任何东西。
   */
  readonly WORKBENCH_MAIN_RESIDENT: "workbench-main-resident"
  /** 浏览器原生宿主槽：能力提供 webview 宿主组件（native 定位容器），Shell 只按槽渲染 */
  readonly BROWSER_HOST: "browser-host"
  /** 浏览器右侧 Dock 面板槽：能力按 `view` 认领一个 Dock 页签面板 */
  readonly BROWSER_DOCK: "browser-dock"
  /**
   * 仓库主视图内子视图槽：能力（如 git）向 RepoPanel 贡献面板。
   * RepoPanel 只按槽渲染、不 import 能力内部，避免 workspace→git 反向依赖环（C3 关键）。
   */
  readonly REPO_SUBVIEW: "repo-subview"
  /** 全局浮层槽：能力向 Shell 贡献顶层模态/灯箱（ConfirmModal / ImageLightbox 等）。
   * Shell 只按槽渲染，不 import 能力内部 UI（C3 关键）。 */
  readonly GLOBAL_OVERLAY: "global-overlay"
  /** AI 导航面板槽：browser 向 Shell 贡献 AINavPanel。 */
  readonly AI_NAV_PANEL: "ai-nav-panel"
  /** 活动栏宫格归档条槽：browser 向 ActivityBar 贡献 GridArchiveBar。 */
  readonly ACTIVITY_BAR_GRID_ARCHIVE: "activity-bar-grid-archive"
  /** 收藏夹凭证列表槽：browser 向 BookmarkPanel 贡献 CredentialList。 */
  readonly BOOKMARK_CREDENTIALS: "bookmark-credentials"
  /** 成果图库槽：browser 向 ArtifactPanel 贡献 ImageGallery。 */
  readonly ARTIFACT_IMAGE_GALLERY: "artifact-image-gallery"
}

export const CONTRIBUTION_SLOTS = {
  COMMANDS: "commands",
  MENUS: "menus",
  SIDEBAR: "sidebar",
  PANELS: "panels",
  SETTINGS: "settings",
  BROWSER_SIDEBAR: "browser-sidebar",
  ADDRESS_BAR_ACTIONS: "address-bar-actions",
  ACTIVITY_BAR_TRAILING: "activity-bar-trailing",
  WORKBENCH_MAIN: "workbench-main",
  WORKBENCH_MAIN_RESIDENT: "workbench-main-resident",
  BROWSER_HOST: "browser-host",
  BROWSER_DOCK: "browser-dock",
  REPO_SUBVIEW: "repo-subview",
  GLOBAL_OVERLAY: "global-overlay",
  AI_NAV_PANEL: "ai-nav-panel",
  ACTIVITY_BAR_GRID_ARCHIVE: "activity-bar-grid-archive",
  BOOKMARK_CREDENTIALS: "bookmark-credentials",
  ARTIFACT_IMAGE_GALLERY: "artifact-image-gallery",
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
  /**
   * 视图认领键（仅 WORKBENCH_MAIN / BROWSER_DOCK 槽使用）：
   * 能力声明自己服务哪个视图（值 = layout.mainView 的取值，如 files/arts/repo/...）。
   * Shell 只按此键匹配渲染，不持有能力专属知识。
   */
  view?: string
  /** Vue 组件或异步组件加载器（由能力提供） */
  component?: Component | (() => Promise<Component>)
  /** 同槽多贡献排序（小在前） */
  order?: number
  /**
   * 页签/导航项的**展示名**（UI-4 DockContribution）。
   * 纯展示字符串，不含业务语义；Shell 只做渲染，不理解其含义。
   */
  label?: string
  /**
   * 页签/导航项的**图标**（UI-4 DockContribution）。
   * 当前为 emoji 字符（与现有 Dock 一致），纯展示。
   */
  icon?: string
}
