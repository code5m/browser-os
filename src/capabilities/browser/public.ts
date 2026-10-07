// Browser 能力公共边界（Phase 8D Train C）
//
// Shell（App / MainArea / TopBar / UnifiedTabBar / ActivityBar / StatusBar / SidebarResizer）与其它能力
// 经此消费 Browser，而**不直接 import 能力内部**（src/capabilities/browser/state/*）。
//
// 注意：本文件是**纯再导出**（re-export），不持有也不创建任何状态（第二真源禁止）。
// 语义 owner（useBrowserStore）与物理路径解耦，由 Semantic Registry 的 owner_implementations locator 解析。
export { useBrowserStore } from "./state/useBrowserStore"
export { useResourceStore, displayUrl, formatDuration, formatSize } from "./state/useResourceStore"
export { useImagePreviewStore } from "./state/useImagePreviewStore"
