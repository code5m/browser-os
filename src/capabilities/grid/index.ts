import { withLazyContributions } from "../../capability/platform/contributed";
import { gridManifest } from "./manifest";
import { activateGridLifecycle, deactivateGridLifecycle, suspendGridLifecycle } from "./lifecycle";

export const GRID_CAPABILITY_ID = "grid";

const contributedGridCapability = withLazyContributions(gridManifest, [
  {
    id: "grid.nav",
    type: "surface",
    slot: "activity-bar-nav",
    load: () => import("./ui/GridNav.vue"),
  },
  {
    id: "grid.rows",
    type: "surface",
    slot: "activity-bar-rows",
    load: () => import("./ui/GridRows.vue"),
  },
]);

export const gridCapability = {
  ...contributedGridCapability,
  lifecycle: {
    ...contributedGridCapability.lifecycle,
    onActivate: () => {
      activateGridLifecycle();
      contributedGridCapability.lifecycle.onActivate?.();
    },
    onSuspend: suspendGridLifecycle,
    onDeactivate: deactivateGridLifecycle,
  },
};
