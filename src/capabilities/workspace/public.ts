// Workspace 能力公共边界（Phase 8C Train B）
//
// Shell（App / MainArea / ActivityBar / StatusBar / UnifiedTabBar）与其它能力经此消费 Workspace，
// 而**不直接 import 能力内部**（src/capabilities/workspace/state/*）。内部路径可随演进变化，本边界保持稳定。
//
// 本边界保持无状态；仅导出 Workspace 归属的受控只读文件服务与状态入口。
// 语义 owner（useWorkspaceStore / useFileStore / useArtifactStore / useRepoStore / useScriptStore /
// useSnippetStore）与物理路径解耦，由 Semantic Registry 的 owner_implementations locator 解析。
export { useWorkspaceStore } from "./state/useWorkspaceStore"
export { useFileStore } from "./state/useFileStore"
export { useArtifactStore } from "./state/useArtifactStore"
export { useRepoStore } from "./state/useRepoStore"

// 只通过 Workspace 能力公开边界提供只读文件读取；原生 allowed_roots 校验保持生效。
// 消费者不得在其他 Capability 内直接调用 Workspace 的 read_file Native 命令。
import { bridge } from "../../bridge"
export function readAuthorizedWorkspaceTextFile(path: string): Promise<string> {
  return bridge.readFile(path)
}
// Directory enumeration belongs to Workspace; other capabilities consume the
// public facade, never call workspace native list_dir directly.
export function listAuthorizedWorkspaceDirectory(path: string): Promise<DirEntry[]> {
  return bridge.listDir(path)
}
