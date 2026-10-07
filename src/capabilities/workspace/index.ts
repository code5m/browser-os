import { workspaceManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"

export const WORKSPACE_CAPABILITY_ID = "workspace"

const registerWorkspace = createLazyContributionRegistrar(workspaceManifest, [
  { id: "workspace.main.files", type: "surface", slot: "workbench-main", view: "files", load: () => import("./ui/FileIdeView.vue") },
  { id: "workspace.main.arts", type: "surface", slot: "workbench-main", view: "arts", load: () => import("./ui/ArtifactPanel.vue") },
  { id: "workspace.main.repo", type: "surface", slot: "workbench-main", view: "repo", load: () => import("./ui/RepoPanel.vue") },
  { id: "workspace.main.audit", type: "surface", slot: "workbench-main", view: "audit", load: () => import("./ui/AuditPanel.vue") },
  { id: "workspace.main.editor", type: "surface", slot: "workbench-main", view: "editor", load: () => import("./ui/FileEditor.vue") },
  { id: "workspace.dock.files", type: "surface", slot: "browser-dock", view: "files", label: "文件", icon: "📂", order: 10, load: () => import("./ui/FilePanel.vue") },
])
export function registerWorkspaceContributions(): void { registerWorkspace() }

export const workspaceCapability = {
  ...workspaceManifest,
  lifecycle: { ...workspaceManifest.lifecycle, onActivate: registerWorkspaceContributions },
}
