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
  dependsOn: [],
  optionalDependencies: [],
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
}
