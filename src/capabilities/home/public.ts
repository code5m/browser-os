// STAGE I-B：Home 域最小公开契约（capabilities/home/public.ts）。
//
// 这不是 God facade / service locator / global event bus —— 仅暴露 Home **确实对外
// 合法**的「公开意图」，其余 Home 内部状态一律不外泄。
//
// 审计结论（STAGE I-B，基于真实代码）：
//   - useHomeStore 持有 Home 域状态（shortcuts / recents / editing），语义 owner = Home。
//   - 真实跨域消费者**只有** workspace（useFileStore.ctxFavorite → favoriteDirectory）。
//     bookmark（BookmarkStar.vue / useBookmarkStore.ts）仅在注释里提及 Home，无实际 import。
//   - 按裁决：属 Home 的意图经最小 public contract 暴露；禁止外部直连内部 store、
//     禁止暴露整个 useHomeStore（故此处仅具名导出动作包装器，不 re-export store）。
//
// 说明：Home 已升格为能力（capabilities/home/）：state 位于 ./state/useHomeStore.ts，
// UI 位于 ./ui/，MainArea 经 WORKBENCH_MAIN 贡献 view='home' 渲染（不再是 Shell 静态 import）。
// 本模块**不** re-export HomePanel.vue —— 组件经 ./index.ts 的 defineAsyncComponent 懒加载，
// 避免把 .vue 静态依赖注入 capability 运行时打包图（check-terminal-owners 的 external *.vue）。
import { useHomeStore } from "./state/useHomeStore";

/** 公开意图：把指定目录加入 Home 收藏（workspace 右键「收藏目录」消费）。 */
export function favoriteDirectory(dir: string): void {
  useHomeStore().favoriteDirectory(dir);
}

/** 公开意图：收藏当前浏览的网页。 */
export function favoriteCurrentPage(): void {
  useHomeStore().favoriteCurrentPage();
}

/** 公开意图：收藏当前目录（来源由 layout.mainView 判定，Home 内部处理）。 */
export function favoriteCurrentDir(): void {
  useHomeStore().favoriteCurrentDir();
}
