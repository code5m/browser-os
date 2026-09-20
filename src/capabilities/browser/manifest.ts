import type { CapabilityDefinition } from "../../capability/types"

// Browser 能力 Manifest（Phase 8D Train C — 物理隔离）
// 语义 owner = useBrowserStore（Semantic Registry 已冻结 ADR-P1A-1/2/3/10、ADR-SEM-P6A-1/3）。
// Grid 作为 Browser 的 heavy 子资源面（同一 store / 同一原生面），本阶段不强制拆成独立 capability。
export const browserManifest: CapabilityDefinition = {
  id: "browser",
  name: "浏览器",
  category: "CAPABILITY",
  provides: [
    "browser.navigate",
    "browser.tab.open",
    "browser.tab.close",
    "browser.host",
    "browser.grid",
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
    class: ["HEAVY", "WEBVIEW", "NATIVE"],
    suspendable: false,
    destroyable: false,
  },
  permissions: ["webview.create"],
  persistence: {
    scope: "session",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useBrowserStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
}
