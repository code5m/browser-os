import { defineAsyncComponent } from "vue"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"
import { sessionManifest } from "./manifest"

export const SESSION_CAPABILITY_ID = "session"
const SessionPanel = defineAsyncComponent(() => import("./ui/SessionPanel.vue"))

export function registerSessionContributions(): void {
  contributionRegistry.registerContribution({
    id: "session.dock",
    capabilityId: SESSION_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BROWSER_DOCK,
    view: "session",
    component: SessionPanel,
    label: "会话",
    icon: "💾",
    order: 40,
  })
}

export const sessionCapability = {
  ...sessionManifest,
  lifecycle: {
    ...sessionManifest.lifecycle,
    onActivate: registerSessionContributions,
  },
}

export { useSessionStore } from "./state/useSessionStore"
