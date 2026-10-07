import type { CapabilityDefinition } from "../../capability/types"

// Browser 能力 Manifest（Phase 8D Train C — 物理隔离）
// 语义 owner = useBrowserStore（Semantic Registry 已冻结 ADR-P1A-1/2/3/10、ADR-SEM-P6A-1/3）。
export const browserManifest: CapabilityDefinition = {
  id: "browser",
  name: "浏览器",
  category: "CAPABILITY",
  provides: [
    "browser.navigate",
    "browser.tab.open",
    "browser.tab.close",
    "browser.host",
  ],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["HEAVY", "WEBVIEW", "NATIVE"],
    suspendable: true,
    destroyable: true,
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
  // Building Block Contract v1（§9）：真实 resource ownership 由 scripts/measure-resources.mjs 测量支撑。
  v1: {
    id: "browser",
    version: "1.0.0",
    displayName: "浏览器",
    description: "多页签浏览器宿主与网络资源能力；Grid / Session 已独立。",
    maturity: "C3",
    maturityEvidence: ["scripts/check-browser-runtime.mjs", "scripts/check-composition-profiles.mjs", "scripts/measure-resources.mjs", "scripts/check-hot-plug-acceptance.mjs"],
    dependencies: [],
    optionalDependencies: [],
    conflicts: [],
    provides: ["browser.navigate", "browser.tab.open", "browser.tab.close", "browser.host"],
    requires: [],
    contributions: [
      { id: "browser.host", slot: "browser-host", type: "surface" },
      { id: "browser.dock.net", slot: "browser-dock", type: "surface", view: "net" },
      { id: "browser.image-lightbox", slot: "global-overlay", type: "surface" },
      { id: "browser.ai-nav-panel", slot: "ai-nav-panel", type: "surface" },
      { id: "browser.image-gallery", slot: "artifact-image-gallery", type: "surface" },
    ],
    permissions: ["webview.create"],
    resources: [
      { kind: "WEBVIEW", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
      { kind: "CHILD_PROCESS", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
    ],
    persistenceScope: "session",
    persistenceSensitive: false,
    activationPolicy: "auto",
    // §24：Browser 停用必须先处理活动 WebView/会话 → graceful，不得静默摘除。
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
      limitationReason: "HP2：pause 冻结/隐藏 Browser WebView；disable 关闭全部页签资源；Harness 验证恢复与重启。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/browser/public.ts" }],
    entrypoint: "src/capabilities/browser/index.ts",
    semanticOwner: "useBrowserStore",
  },
}
