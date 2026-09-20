import type { CapabilityDefinition } from "../../capability/types"

// Workspace 能力 Manifest（Phase 8C Train B — 物理隔离）
// 语义 owner = useWorkspaceStore（Semantic Registry 已冻结）；物理实现已迁入本能力包（state/*）。
// 子域 owner（Files/Artifact/Repo/Script/Snippet）同属本能力，由 owner_implementations locator 解析。
export const workspaceManifest: CapabilityDefinition = {
  id: "workspace",
  name: "工作区",
  category: "CAPABILITY",
  provides: [
    "workspace.files",
    "workspace.artifact",
    "workspace.repo",
    "workspace.script",
    "workspace.snippet",
    "workspace.main-view",
  ],
  dependsOn: [],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["LIGHT", "MEDIUM"],
    suspendable: false,
    destroyable: false,
  },
  permissions: [],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useWorkspaceStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
}
