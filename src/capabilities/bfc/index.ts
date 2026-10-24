import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateBfcLifecycle, suspendBfcLifecycle } from "./lifecycle"
import { bfcManifest } from "./manifest"

export const BFC_CAPABILITY_ID = "bfc"

const bfc = createLifecycleContributionCapability(
  bfcManifest,
  [{ id: "bfc.main.panel", type: "surface", slot: "workbench-main", view: "bfc", label: "BFC 内容库", icon: "▦", panelState: true, load: () => import("./ui/BfcProjectPanel.vue") }],
  activateBfcLifecycle,
  suspendBfcLifecycle,
)

export const bfcCapability = bfc.capability
export const registerBfcContributions = bfc.register
