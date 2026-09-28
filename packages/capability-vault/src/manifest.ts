import type { CapabilityDefinition } from "./types";

// Vault 能力 Manifest（Frontend M2 —— 独立 capability package）
// semanticOwner 指向 Semantic Registry 已登记的 owner：useVaultStore（id=vault）。
// 物理隔离至 packages/capability-vault；不再位于 src/capabilities/vault。
// 不改变该语义 owner；governed_files 经由 owner_implementations locator 解析新路径。
export const vaultManifest: CapabilityDefinition = {
  id: "vault",
  name: "笔记库",
  category: "CAPABILITY",
  provides: ["vault.open", "vault.search", "vault.follow"],
  dependsOn: ["bridge"],
  optionalDependencies: ["browser"],
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
      "Obsidian Markdown 目录浏览、笔记链接图与跟随跳转；物理隔离为独立 capability package，语义 owner 为 useVaultStore。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-package.mjs", "scripts/check-vault-logic.mjs"],
    dependencies: [],
    // 笔记库为只读 Markdown 快照，可挂于 Browser 槽之外独立展示，但无强制依赖。
    optionalDependencies: ["browser"],
    conflicts: [],
    provides: ["vault.open", "vault.search", "vault.follow"],
    requires: [],
    contributions: [{ id: "vault.main", slot: "workbench-main", type: "surface", view: "vault" }],
    permissions: ["fs.read"],
    resources: [
      {
        kind: "CACHE",
        ownership: "owned",
        evidence: "packages/capability-vault/src/state/useVaultStore.ts",
      },
    ],
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
        "HP2：无重资源、无后台任务，运行时 register/unregister 已由 scripts/check-package.mjs 验证；HP3 需引入外部能力包的动态加载/沙箱安装链路，本 Pilot 不做（禁止高风险动态代码加载）。",
    },
    publicContract: [{ name: "publicApi", locator: "packages/capability-vault/src/index.ts" }],
    entrypoint: "packages/capability-vault/src/index.ts",
    semanticOwner: "useVaultStore",
  },
};
