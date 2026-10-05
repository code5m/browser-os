import { defineAsyncComponent } from "vue"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"
import { workbenchManifest } from "./manifest"

export const WORKBENCH_CAPABILITY_ID = "workbench"
const WorkbenchCommands = defineAsyncComponent(() => import("./ui/WorkbenchCommands.vue"))
const WorkbenchRail = defineAsyncComponent(() => import("./ui/WorkbenchRail.vue"))

export function registerWorkbenchContributions(): void {
  contributionRegistry.registerContribution({
    id: "workbench.commands",
    capabilityId: WORKBENCH_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.COMMANDS,
    component: WorkbenchCommands,
  })
  contributionRegistry.registerContribution({
    id: "workbench.rail",
    capabilityId: WORKBENCH_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.SIDEBAR,
    component: WorkbenchRail,
  })
}

export const workbenchCapability = {
  ...workbenchManifest,
  lifecycle: {
    ...workbenchManifest.lifecycle,
    onActivate: registerWorkbenchContributions,
  },
}
