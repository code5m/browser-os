/**
 * shared/ui 统一公开入口（PUBLIC ENTRY）。
 *
 * 规则（SHARED UI DEPENDENCY LAW）：
 *  - 外部**只允许**从本入口 import，禁止依赖 shared/ui 内部路径。
 *  - shared/ui 自身禁止依赖：capabilities/**、业务 store、业务 composable、
 *    bridge 业务 API、Tauri invoke、任何具体 Capability 域（Browser/Terminal/
 *    Workspace/Bookmark/Git/Database/Agent/Skill/Plugin）。
 *  - shared/ui 允许依赖：Vue、通用 UI 契约、design tokens、底层工具函数。
 *
 * 以上由 scripts/check-ui-boundaries.mjs 的 UI-01 / UI-02 / UI-08 机器校验。
 */
export { default as EmptyState } from "./EmptyState.vue";
export { default as ContextMenu } from "./ContextMenu.vue";
export { default as ContextMenuItem } from "./ContextMenuItem.vue";
