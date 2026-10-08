import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const browserManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "browser",
  name: "浏览器",
  semanticOwner: "useBrowserStore",
  maturity: "C3",
  version: "1.0.0",
  maturityEvidence: [
    "scripts/check-browser-runtime.mjs",
    "scripts/check-composition-profiles.mjs",
    "scripts/measure-resources.mjs",
    "scripts/check-hot-plug-acceptance.mjs",
  ],
  provides: ["browser.navigate", "browser.tab.open", "browser.tab.close", "browser.host"],
  dependencies: ["bridge"],
  v1Dependencies: [],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["HEAVY", "WEBVIEW", "NATIVE"],
  suspendable: true,
  destroyable: true,
  permissions: ["webview.create"],
  persistence: { scope: "session", sensitive: false },
  contributions: [
    { id: "browser.host", slot: "browser-host", type: "surface" },
    { id: "browser.dock.net", slot: "browser-dock", type: "surface", view: "net" },
    { id: "browser.image-lightbox", slot: "global-overlay", type: "surface" },
    { id: "browser.ai-nav-panel", slot: "ai-nav-panel", type: "surface" },
    { id: "browser.image-gallery", slot: "artifact-image-gallery", type: "surface" },
  ],
  manifestResources: [
    { kind: "WEBVIEW", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
    { kind: "CHILD_PROCESS", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
  ],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/browser/public.ts" }],
  deactivationPolicy: "graceful",
  description: "多页签浏览器宿主。",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：暂停冻结，停用关闭页签。",
})
