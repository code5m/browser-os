import { graphManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"

export const KNOWLEDGE_GRAPH_CAPABILITY_ID = "graph"

const registerGraph = createLazyContributionRegistrar(graphManifest, [
  { id: "graph.main.panel", type: "surface", slot: "workbench-main", view: "graph", label: "知识图谱", icon: "🕸️", panelState: true, load: () => import("./ui/GraphPanel.vue") },
])
export function registerGraphContributions(): void { registerGraph() }

export const graphCapability = {
  ...graphManifest,
  lifecycle: { ...graphManifest.lifecycle, onActivate: registerGraphContributions },
}
