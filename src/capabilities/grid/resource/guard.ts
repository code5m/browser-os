import { isCapabilityActive } from "../../../capability/runtimeSingleton";
import { gridManifest } from "../manifest";

export function isGridResourceAllowed(): boolean {
  return isCapabilityActive(gridManifest.id);
}
