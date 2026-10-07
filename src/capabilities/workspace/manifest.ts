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
    "workspace.main-view",
  ],
  dependsOn: ["bridge"],
  optionalDependencies: ["browser"],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["LIGHT", "MEDIUM"],
    suspendable: true,
    destroyable: true,
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
  // Building Block Contract v1（§9）：不持有需治理的重资源；子域 owner 仍在本包内由 locator 解析。
  v1: {
    id: "workspace",
    version: "1.0.0",
    displayName: "工作区",
    description: "文件/产物/仓库/脚本/命令片段/审计六大主视图与文件编辑器；子域 owner 均在本能力包内。",
    maturity: "C3",
    maturityEvidence: ["scripts/check-capability-pilot.mjs", "scripts/check-composition-profiles.mjs", "scripts/check-hot-plug-acceptance.mjs"],
    dependencies: [],
    optionalDependencies: ["browser"],
    conflicts: [],
    provides: [
      "workspace.files",
      "workspace.artifact",
      "workspace.repo",
      "workspace.main-view",
    ],
    requires: [],
    contributions: [
      { id: "workspace.main.files", slot: "workbench-main", type: "surface", view: "files" },
      { id: "workspace.main.arts", slot: "workbench-main", type: "surface", view: "arts" },
      { id: "workspace.main.repo", slot: "workbench-main", type: "surface", view: "repo" },
      { id: "workspace.main.scripts", slot: "workbench-main", type: "surface", view: "scripts" },
      { id: "workspace.main.commands", slot: "workbench-main", type: "surface", view: "commands" },
      { id: "workspace.main.audit", slot: "workbench-main", type: "surface", view: "audit" },
      { id: "workspace.main.editor", slot: "workbench-main", type: "surface", view: "editor" },
      { id: "workspace.dock.files", slot: "browser-dock", type: "surface", view: "files" },
    ],
    permissions: [],
    // 无需治理的重资源（既有 LIGHT/MEDIUM 无 native handle 需回收）
    resources: [],
    persistenceScope: "disk",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "graceful",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP2",
      enable: true,
      disable: true,
      register: true,
      unregister: true,
      install: false,
      uninstall: false,
      limitationReason: "HP2：Workspace 仅拥有可逆 UI contributions；Shell 通用 availability 回退保证停用当前 Workspace 视图时不会留下死视图，统一 Harness 验证重复启停与 fresh Runtime。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/workspace/public.ts" }],
    entrypoint: "src/capabilities/workspace/index.ts",
    semanticOwner: "useWorkspaceStore",
  },
}
