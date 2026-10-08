import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateGridLifecycle, deactivateGridLifecycle, suspendGridLifecycle } from "./lifecycle"
import { gridManifest } from "./manifest"

export const GRID_CAPABILITY_ID = "grid"

export const gridCapability = createLifecycleContributionCapability(gridManifest, [
  { id: "grid.nav", type: "surface", slot: "activity-bar-nav", load: () => import("./ui/GridNav.vue") },
  { id: "grid.rows", type: "surface", slot: "activity-bar-rows", load: () => import("./ui/GridRows.vue") },
], activateGridLifecycle, suspendGridLifecycle, deactivateGridLifecycle).capability
