import type { CapabilityDefinition } from "../../capability/types";
import { defineIntegratedCapability } from "../../capability/platform/integrated";

export const gridManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "grid",
  name: "Grid",
  semanticOwner: "useGridStore",
  maturity: "C2",
  version: "2.0.0",
  maturityEvidence: [
    "scripts/check-grid-close-logic.mjs",
    "scripts/runtime-phase1-browser-grid.mjs",
    "scripts/check-v2-maturity-boundaries.mjs",
  ],
  provides: ["grid.open", "grid.layout", "grid.archive"],
  dependencies: ["browser", "bridge"],
  resident: false,
  resourceClass: ["VERY_HEAVY", "MULTI_WEBVIEW", "NATIVE"],
  suspendable: false,
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
  description: "独立 Grid 多 WebView 生命周期、布局、AI 群发与回复归档能力。",
  limitationReason: "B maturity：Grid 已独立装配并拥有重资源；尚未证明 suspend/resume 与 Hot-Plug Harness 全契约，因此保持 HP0。",
});
