// Workspace 能力公共边界（Phase 8C Train B）
//
// Shell（App / MainArea / ActivityBar / StatusBar / UnifiedTabBar）与其它能力经此消费 Workspace，
// 而**不直接 import 能力内部**（src/capabilities/workspace/state/*）。内部路径可随演进变化，本边界保持稳定。
//
// 注意：本文件是**纯再导出**（re-export），不持有也不创建任何状态（第二真源禁止）。
// 语义 owner（useWorkspaceStore / useFileStore / useArtifactStore / useRepoStore / useScriptStore /
// useSnippetStore）与物理路径解耦，由 Semantic Registry 的 owner_implementations locator 解析。
export { useWorkspaceStore } from "./state/useWorkspaceStore"
export { useFileStore } from "./state/useFileStore"
export { useArtifactStore } from "./state/useArtifactStore"
export { useRepoStore } from "./state/useRepoStore"
