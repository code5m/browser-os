import { withLazyContributions } from "../../capability/platform/contributed"
import { sessionManifest } from "./manifest"

export const SESSION_CAPABILITY_ID = "session"
export const sessionCapability = withLazyContributions(sessionManifest, [
  { id: "session.dock", type: "surface", slot: "browser-dock", view: "session", label: "会话", icon: "💾", order: 40, load: () => import("./ui/SessionPanel.vue") },
])
