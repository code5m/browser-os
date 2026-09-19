// Bookmark Capability — 模块入口（Phase 8B 物理隔离 / C3 边界）
//
// 兼容优先：声明 manifest + 提供 lifecycle 钩子，**不接管业务状态真源**。
// 业务 Owner 仍是 useBookmarkStore（Semantic Registry 经 owner_implementations 解析新路径）。
//
// 硬约束：本文件（能力适配器）不得 import useBookmarkStore，也不得读写其任何 state。
//   违反即等于 Runtime/Adapter 成为第二真源 → 语义治理回归。
//   由 scripts/check-capability-pilot.mjs 的 PLT-05 静态断言守护（src/capability 扫描域）。
//   Shell 经 ./public 公共边界消费 Bookmark（Contribution/Slot 模型），不直接 import 本能力内部 store。

import { bookmarkManifest } from "./manifest"
import { onActivate, onSuspend } from "./lifecycle/bookmark"

export const BOOKMARK_CAPABILITY_ID = "bookmark"

/**
 * 注册 Bookmark 对 Shell 的 UI 贡献（Contribution/Slot 模型）。
 * Phase 8B 当前为兼容占位：不读写业务状态、不建立第二真源。
 * 真实 Slot 接线（Shell 经公共边界渲染 panelOpen）在后续 C3 收口步骤补齐，
 * 且必须保持 scripts/check-semantic-closure-logic.mjs 的冻结断言
 * （MainArea 字面保留 `bookmarks.panelOpen && mainView === 'browser'`）。
 */
export function registerBookmarkContributions(): void {
  // TODO(8B-C3): 向 Shell Contribution Registry 注册 bookmark.panel 槽贡献。
}

export const bookmarkCapability = {
  ...bookmarkManifest,
  lifecycle: {
    ...bookmarkManifest.lifecycle,
    onActivate,
    onSuspend,
  },
}
