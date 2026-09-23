import type { CapabilityDefinition } from "../../capability/types"

// Git 能力 Manifest（Capability Library Expansion v1 — 从 Workspace 内嵌面板升格为独立 Capability）
// 语义 owner = useGitStore（Semantic Registry 已冻结）。
// 物理：面板经通用 Contribution Registry 的 REPO_SUBVIEW 槽贡献给 RepoPanel，
// RepoPanel 只按槽渲染、不 import 本能力内部（C3 关键，且避免 workspace→git 反向依赖环）。
export const gitManifest: CapabilityDefinition = {
  id: "git",
  name: "Git 版本控制",
  category: "CAPABILITY",
  provides: [
    "git.status",
    "git.diff",
    "git.log",
    "git.commit",
    "git.write",
  ],
  dependsOn: ["credential", "workspace", "bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["MEDIUM", "NETWORK"],
    suspendable: true,
    destroyable: true,
  },
  permissions: ["fs.read", "fs.write", "keyring.access"],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useGitStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）：真实 resource ownership 由 scripts/measure-resources.mjs 测量支撑。
  v1: {
    id: "git",
    version: "1.0.0",
    displayName: "Git 版本控制",
    description:
      "仓库状态 / diff / 提交历史 / 提交，以及写操作的双阶段闸门（request → confirm）。写操作 spawn git 子进程执行。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-developer-owners.mjs", "scripts/measure-resources.mjs"],
    dependencies: ["credential", "workspace", "bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["git.status", "git.diff", "git.log", "git.commit", "git.write"],
    requires: [],
    contributions: [{ id: "git.repo.panel", slot: "repo-subview", type: "surface" }],
    permissions: ["fs.read", "fs.write", "keyring.access"],
    resources: [
      { kind: "PROCESS", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
    ],
    persistenceScope: "disk",
    persistenceSensitive: false,
    activationPolicy: "auto",
    // §24：Git 停用须先处理进行中的写任务（双阶段闸门）→ graceful，不得静默摘除。
    deactivationPolicy: "graceful",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP0",
      enable: false,
      disable: false,
      register: false,
      unregister: false,
      install: false,
      uninstall: false,
      limitationReason:
        "HP0(STATIC)：写操作需双阶段闸门，未验证 absent 态无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/git/public.ts" }],
    entrypoint: "src/capabilities/git/index.ts",
    semanticOwner: "useGitStore",
  },
}
