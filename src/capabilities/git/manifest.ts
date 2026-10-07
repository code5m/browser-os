import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const gitManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "git",
  name: "Git 版本控制",
  semanticOwner: "useGitStore",
  maturity: "C3",
  version: "1.0.0",
  maturityEvidence: [
    "scripts/check-developer-owners.mjs",
    "scripts/measure-resources.mjs",
    "scripts/check-hot-plug-acceptance.mjs",
  ],
  provides: ["git.status", "git.diff", "git.log", "git.commit", "git.write"],
  dependencies: ["credential", "workspace", "bridge"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["MEDIUM", "NETWORK"],
  suspendable: true,
  destroyable: true,
  permissions: ["fs.read", "fs.write", "keyring.access"],
  persistence: { scope: "disk", sensitive: false },
  contributions: [
    { id: "git.repo.panel", slot: "repo-subview", type: "surface", view: "git" },
    { id: "git.repo.history", slot: "repo-subview", type: "surface", view: "history" },
  ],
  manifestResources: [
    { kind: "PROCESS", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
  ],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/git/public.ts" }],
  deactivationPolicy: "graceful",
  description: "仓库状态、diff、历史与双阶段写操作；写任务运行时拒绝热停用。",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：有写任务时拒绝暂停；空闲时失效读请求与 preview，并支持恢复与重启。",
})
