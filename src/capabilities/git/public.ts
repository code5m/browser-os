// Git 能力公共边界（Capability Library Expansion v1）
//
// Shell（App / RepoPanel）与其它能力经此消费 Git，而不直接 import 能力内部
// （src/stores/useGitStore.ts 的物理路径或 ui/ 内部组件）。本文件是**纯再导出**（re-export），
// 不持有也不创建任何状态（第二真源禁止）。
// 语义 owner（useGitStore）与物理路径解耦，由 Semantic Registry 的 owner_implementations locator 解析。
export * from "../../stores/useGitStore"

// 写操作确认对话框：经公共边界暴露给 Shell（App.vue 全局挂载），
// 而非让 Shell 直连 src/capabilities/git/ui/ 内部（CB-02/06 禁止）。
export { default as GitWriteConfirmDialog } from "./ui/GitWriteConfirmDialog.vue"
