import { browserManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"
export const BROWSER_CAPABILITY_ID = "browser"
const registerBrowser = createLazyContributionRegistrar(browserManifest, [
  { id: "browser.host", type: "surface", slot: "browser-host", load: () => import("./ui/BrowserHost.vue") },
  { id: "browser.dock.net", type: "surface", slot: "browser-dock", view: "net", label: "资源", icon: "🌊", order: 30, load: () => import("./ui/ResourceWaterfall.vue") },
  { id: "browser.image-lightbox", type: "surface", slot: "global-overlay", load: () => import("./ui/ImageLightbox.vue") },
  { id: "browser.ai-nav-panel", type: "surface", slot: "ai-nav-panel", load: () => import("./ui/AINavPanel.vue") },
  { id: "browser.image-gallery", type: "surface", slot: "artifact-image-gallery", load: () => import("./ui/ImageGallery.vue") },
])
export function registerBrowserContributions(): void { registerBrowser() }
export const browserCapability = { ...browserManifest, lifecycle: { ...browserManifest.lifecycle, onActivate: registerBrowserContributions } }
