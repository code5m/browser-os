// Bookmark Capability — 模块入口（Phase 8B.1 Contribution Activation / C3）
//
// 兼容优先：声明 manifest + 提供 lifecycle 钩子，**不接管业务状态真源**。
// 业务 Owner 仍是 useBookmarkStore（Semantic Registry 经 owner_implementations 解析新路径）。
//
// 硬约束：本文件（能力适配器）不得 import useBookmarkStore，也不得读写其任何 state。
//   违反即等于 Runtime/Adapter 成为第二真源 → 语义治理回归。
//   由 scripts/check-capability-pilot.mjs 的 PLT-05 静态断言守护（src/capability 扫描域）。
//
// Phase 8B.1：经 **通用 Contribution Registry** 向 Shell 贡献 UI（Contribution/Slot 模型）。
//   Shell 遍历 slot 渲染，不 import 本能力内部 store / ui（消除 Debt-8B-3，C3 达成）。
//   panelOpen 真源仍归 useBookmarkStore——可见性判定已下沉到 BookmarkPanel 自身（能力包内）。

import { defineAsyncComponent } from "vue"
import { bookmarkManifest } from "./manifest"
import { onActivate, onSuspend } from "./lifecycle/bookmark"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const BOOKMARK_CAPABILITY_ID = "bookmark"

// 组件经异步懒加载注册（defineAsyncComponent）：
//   - 组件位于本能力包内（ui/），由能力提供；Shell 只经 <component :is> 渲染，绝不直连。
//   - 懒加载把面板/按钮拆出主 chunk，符合构建体积闸门（IF-2）。
const BookmarkPanel = defineAsyncComponent(() => import("./ui/BookmarkPanel.vue"))
const BookmarkStar = defineAsyncComponent(() => import("./ui/BookmarkStar.vue"))

/**
 * 注册 Bookmark 对 Shell 的 UI 贡献（Contribution/Slot 模型）。
 *   - surface  browser-sidebar     → 收藏夹侧栏（BookmarkPanel 自行按 panelOpen 控制显隐）
 *   - navigation address-bar-actions → 地址栏 ⭐ 收藏按钮（BookmarkStar）
 *   - navigation activity-bar-trailing → 工具栏尾部「收藏夹」入口（BookmarkEntryButton）
 * 组件内部读取各自 store，Shell 零 Bookmark 专属知识（C3 关键）。
 */
export function registerBookmarkContributions(): void {
  contributionRegistry.registerContribution({
    id: "bookmark.sidebar",
    capabilityId: BOOKMARK_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BROWSER_SIDEBAR,
    component: BookmarkPanel,
  })
  contributionRegistry.registerContribution({
    id: "bookmark.address-star",
    capabilityId: BOOKMARK_CAPABILITY_ID,
    type: "navigation",
    slot: CONTRIBUTION_SLOTS.ADDRESS_BAR_ACTIONS,
    component: BookmarkStar,
  })

}

export const bookmarkCapability = {
  ...bookmarkManifest,
  lifecycle: {
    ...bookmarkManifest.lifecycle,
    onActivate,
    onSuspend,
  },
}
