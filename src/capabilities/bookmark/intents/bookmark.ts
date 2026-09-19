// Bookmark 公开 Intent 标识（Phase 8B）
// 供 Contribution / Shell 派发；意图落到 Bookmark 公共 boundary，不建立第二真源。
export const BOOKMARK_INTENTS = {
  toggle: "bookmark.toggle",
  togglePanel: "bookmark.togglePanel",
  open: "bookmark.open",
  remove: "bookmark.remove",
} as const

export type BookmarkIntent = (typeof BOOKMARK_INTENTS)[keyof typeof BOOKMARK_INTENTS]
