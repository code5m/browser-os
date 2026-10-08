import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateBrowserLifecycle, deactivateBrowserLifecycle, suspendBrowserLifecycle } from "./lifecycle"
import { browserManifest } from "./manifest"

export const BROWSER_CAPABILITY_ID = "browser"

const browser = createLifecycleContributionCapability(browserManifest, [
  { id: "browser.host", type: "surface", slot: "browser-host", load: () => import("./ui/BrowserHost.vue") },
  { id: "browser.dock.net", type: "surface", slot: "browser-dock", view: "net", label: "资源", icon: "🌊", order: 30, load: () => import("./ui/ResourceWaterfall.vue") },
  { id: "browser.image-lightbox", type: "surface", slot: "global-overlay", load: () => import("./ui/ImageLightbox.vue") },
  { id: "browser.ai-nav-panel", type: "surface", slot: "ai-nav-panel", load: () => import("./ui/AINavPanel.vue") },
  { id: "browser.image-gallery", type: "surface", slot: "artifact-image-gallery", load: () => import("./ui/ImageGallery.vue") },
], activateBrowserLifecycle, suspendBrowserLifecycle, deactivateBrowserLifecycle)

export const browserCapability = browser.capability
export const registerBrowserContributions = browser.register
