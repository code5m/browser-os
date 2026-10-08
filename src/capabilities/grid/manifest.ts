import type { CapabilityDefinition } from "../../capability/types";
import { defineIntegratedCapability } from "../../capability/platform/integrated"\n// maturity evidence: scripts/check-hot-plug-acceptance.mjs;

export const gridManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "grid",
  name: "Grid",
  semanticOwner: "useGridStore",
  maturity: "C3",
  version: "2.0.0",
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
  hotPlugLevel: "HP2",
  limitationReason: "HP2：暂停冻结，停用销毁 Grid。",
});
