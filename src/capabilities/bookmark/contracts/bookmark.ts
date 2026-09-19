// Bookmark 公共契约（Phase 8B）
// Shell / 其它 Capability 经此了解 Bookmark 的能力边界，而不依赖其内部实现。
// 这些是"类型契约"，不持有也不创建任何状态（第二真源禁止）。
export interface BookmarkProjection {
  readonly items: unknown[]
  readonly panelOpen: boolean
  readonly loaded: boolean
}

export interface BookmarkIntents {
  toggle(url: string, title: string): Promise<void>
  togglePanel(): void
  open(url: string): Promise<void>
  remove(id: string): Promise<void>
}
