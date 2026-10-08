import type { CapabilityDefinition } from "../../capability/types";
import { defineIntegratedCapability } from "../../capability/platform/integrated";

export const gridManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "grid",
  name: "Grid",
  semanticOwner: "useGridStore",
  maturity: "C3",
  version: "2.0.0",
  maturityEvidence: [
    "scripts/check-grid-close-logic.mjs",
    "scripts/runtime-phase1-browser-grid.mjs",
    "scripts/check-v2-maturity-boundaries.mjs",
    "scripts/check-hot-plug-acceptance.mjs",
  ],
  provides: ["grid.open", "grid.layout", "grid.archive"],
  dependencies: ["browser", "bridge"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["HEAVY", "MULTI_WEBVIEW", "NATIVE"],
  suspendable: true,
  destroyable: true,
  permissions: ["webview.create", "process.spawn"],
  persistence: { scope: "disk", sensitive: false },
  contributions: [
    { id: "grid.nav", slot: "activity-bar-nav", type: "surface" },
    { id: "grid.rows", slot: "activity-bar-rows", type: "surface" },
  ],
  manifestResources: [
    { kind: "WEBVIEW", ownership: "owned", evidence: "src-tauri/src/bridge.rs:create_grid/close_grid/grid_*" },
    { kind: "CHILD_PROCESS", ownership: "owned", evidence: "src-tauri/src/bridge.rs:create_grid/close_grid/grid_*" },
  ],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/grid/public.ts" }],
  deactivationPolicy: "graceful",
  description: "Grid 多 WebView。",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：暂停冻结，停用销毁 Grid。",
});
