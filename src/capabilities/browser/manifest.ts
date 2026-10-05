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
  // Building Block Contract v1（§9）：真实 resource ownership 由 scripts/measure-resources.mjs 测量支撑。
  v1: {
    id: "browser",
    version: "1.0.0",
    displayName: "浏览器",
    description: "多页签浏览器宿主、网络资源瀑布与 Grid heavy 子资源面；Session 已独立为可选展示能力。",
    maturity: "C3",
    maturityEvidence: ["scripts/check-browser-runtime.mjs", "scripts/check-composition-profiles.mjs", "scripts/measure-resources.mjs"],
    dependencies: [],
    optionalDependencies: [],
    conflicts: [],
    provides: ["browser.navigate", "browser.tab.open", "browser.tab.close", "browser.host", "browser.grid"],
    requires: [],
    contributions: [
      { id: "browser.host", slot: "browser-host", type: "surface" },
      { id: "browser.dock.net", slot: "browser-dock", type: "surface", view: "net" },
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
      level: "HP0",
      enable: false,
      disable: false,
      register: false,
      unregister: false,
      install: false,
      uninstall: false,
      limitationReason: "HP0(STATIC)：WebView 生命周期需先建立真正的 GRACEFUL 停用策略并测量无残留，未验证前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/browser/public.ts" }],
    entrypoint: "src/capabilities/browser/index.ts",
    semanticOwner: "useBrowserStore",
  },
}
