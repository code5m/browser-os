import type { CapabilityDefinition } from "../../../capability/types"

// Vault 能力 Manifest（STAGE I-A — 物理隔离）
// semanticOwner 指向 Semantic Registry 已登记的 owner：useVaultStore（id=vault）。
// 物理迁移（src/stores/useVaultStore.ts → src/capabilities/vault/state/useVaultStore.ts，
// src/components/workspace/VaultPanel.vue → src/capabilities/vault/ui/VaultPanel.vue）
// 不改变该语义 owner；governed_files 经由 owner_implementations locator 解析新路径。
export const vaultManifest: CapabilityDefinition = {
  id: "vault",
  name: "笔记库",
  category: "CAPABILITY",
  provides: ["vault.open", "vault.search", "vault.follow"],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["LIGHT"],
    suspendable: true,
    destroyable: true,
  },
  permissions: ["fs.read"],
  persistence: {
    scope: "none",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useVaultStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）：与上面能力定义内联同一对象，禁止另建 second truth。
  v1: {
    id: "vault",
    version: "1.0.0",
    displayName: "笔记库",
    description:
      "Obsidian Markdown 目录浏览、笔记链接图与跟随跳转；物理隔离至本能力包，语义 owner 为 useVaultStore。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-capability-platform.mjs", "scripts/check-capability-composition.mjs"],
    dependencies: [],
    // 笔记库为只读 Markdown 快照，可挂于 Browser 槽之外独立展示，但无强制依赖。
    optionalDependencies: ["browser"],
    conflicts: [],
    provides: ["vault.open", "vault.search", "vault.follow"],
    requires: [],
    contributions: [{ id: "vault.main", slot: "workbench-main", type: "surface", view: "vault" }],
    permissions: ["fs.read"],
    resources: [{ kind: "CACHE", ownership: "owned", evidence: "src/capabilities/vault/state/useVaultStore.ts" }],
    persistenceScope: "runtime_only",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "manual",
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
      limitationReason:
        "HP2：无重资源、无后台任务，运行时 register/unregister 已由 scripts/check-capability-platform.mjs 验证；HP3 需引入外部能力包的动态加载/沙箱安装链路，本夜不做（禁止高风险动态代码加载）。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/vault/public.ts" }],
    entrypoint: "src/capabilities/vault/index.ts",
    semanticOwner: "useVaultStore",
  },
}
