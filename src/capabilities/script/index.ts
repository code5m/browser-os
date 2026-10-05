import { defineAsyncComponent } from "vue"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"
import { scriptManifest } from "./manifest"

export const SCRIPT_CAPABILITY_ID = "script"
const ScriptPanel = defineAsyncComponent(() => import("./ui/ScriptPanel.vue"))

export function registerScriptContributions(): void {
  contributionRegistry.registerContribution({
    id: "script.main.panel",
    capabilityId: SCRIPT_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "scripts",
    component: ScriptPanel,
  })
}

export const scriptCapability = {
  ...scriptManifest,
  lifecycle: {
    ...scriptManifest.lifecycle,
    onActivate: registerScriptContributions,
  },
}

export { useScriptStore } from "./state/useScriptStore"
