import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const sessionManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "session",
  name: "会话",
  semanticOwner: "useSessionStore",
  maturity: "C2",
  provides: ["session.persist", "session.restore"],
  resident: true,
  resourceClass: ["LIGHT"],
  suspendable: false,
  destroyable: false,
  permissions: ["fs.write"],
  persistence: { scope: "disk", sensitive: false },
  contributions: [{ id: "session.dock", slot: "browser-dock", type: "surface", view: "session" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/session/public.ts" }],
  limitationReason: "Resident shutdown ordering is not hot-pluggable.",
})
