import type { CapabilityDefinition } from "../../../capability/types"

// Bookmark 能力 Manifest（Phase 8B — 物理隔离）
// semanticOwner 指向 Semantic Registry 已冻结的 owner：useBookmarkStore。
// 物理迁移（src/stores/useBookmarkStore.ts → src/capabilities/bookmark/state/useBookmarkStore.ts）
// 不改变该语义 owner；governed_files 经由 owner_implementations locator 解析新路径。
export const bookmarkManifest: CapabilityDefinition = {
  id: "bookmark",
  name: "收藏夹",
  category: "UI_COMPONENT",
  provides: ["bookmark.storage", "bookmark.panel", "bookmark.navigation"],
  dependsOn: ["bridge"],
  optionalDependencies: ["browser"],
  lifecycle: {
    supported: ["REGISTERED", "ACTIVE", "SUSPENDED"],
    default: "REGISTERED",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["LIGHT"],
    suspendable: true,
    destroyable: false,
  },
  permissions: [],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useBookmarkStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）：与上面能力定义内联同一对象，禁止另建 second truth。
  v1: {
    id: "bookmark",
    version: "1.0.0",
    displayName: "收藏夹",
    description: "浏览器侧栏收藏夹、地址栏收藏星标与工具栏入口；物理隔离至本能力包，语义 owner 不变。",
    maturity: "C3",
    maturityEvidence: ["scripts/check-capability-pilot.mjs", "scripts/check-composition-profiles.mjs", "scripts/check-hot-plug-acceptance.mjs"],
    dependencies: [],
    // 收藏贡献渲染于 Browser 槽：Browser 缺失时该能力仍成立但无展示面 → 降级而非拒绝（§13）。
    optionalDependencies: ["browser"],
    conflicts: [],
    provides: ["bookmark.storage", "bookmark.panel", "bookmark.navigation"],
    requires: [],
    contributions: [
      { id: "bookmark.sidebar", slot: "browser-sidebar", type: "surface" },
      { id: "bookmark.address-star", slot: "address-bar-actions", type: "navigation" },
      { id: "bookmark.entry-button", slot: "activity-bar-trailing", type: "navigation" },
    ],
    permissions: [],
    resources: [{ kind: "CACHE", ownership: "owned", evidence: "src/capabilities/bookmark/state/useBookmarkStore.ts" }],
    persistenceScope: "disk",
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
    publicContract: [{ name: "publicApi", locator: "src/capabilities/bookmark/public.ts" }],
    entrypoint: "src/capabilities/bookmark/index.ts",
    semanticOwner: "useBookmarkStore",
  },
}
