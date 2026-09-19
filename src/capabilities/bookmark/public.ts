// Bookmark 公共边界（Phase 8B Contribution/Slot 模型）
//
// Shell（MainArea / ActivityBar）经此消费 Bookmark 能力，而**不直接 import 能力内部 store**
// （src/capabilities/bookmark/state/useBookmarkStore.ts）。内部 store 物理路径可随 C3 演进变化，
// 本边界保持稳定，使 Shell 与能力实现解耦（符合 8A.1 裁决：用 Contribution/Slot 解耦 Shell 与 Bookmark）。
//
// 注意：本文件是**纯再导出**（re-export），不持有也不创建任何状态（第二真源禁止）。
export { useBookmarkStore, canBookmark } from "./state/useBookmarkStore"
