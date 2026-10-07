import { withLazyContributions } from "../../capability/platform/contributed"
import { workbenchManifest } from "./manifest"

export const WORKBENCH_CAPABILITY_ID = "workbench"
export const workbenchCapability = withLazyContributions(workbenchManifest, [
  { id: "workbench.commands", type: "surface", slot: "commands", load: () => import("./ui/WorkbenchCommands.vue") },
  { id: "workbench.rail", type: "surface", slot: "sidebar", load: () => import("./ui/WorkbenchRail.vue") },
])
