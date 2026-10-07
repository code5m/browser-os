import { agentManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"

export const AGENT_CAPABILITY_ID = "agent"

const registerAgent = createLazyContributionRegistrar(agentManifest, [
  { id: "agent.main.panel", type: "surface", slot: "workbench-main", view: "agents", label: "智能体", icon: "🤖", panelState: true, load: () => import("./ui/AgentManagerPanel.vue") },
])
export function registerAgentContributions(): void { registerAgent() }

export const agentCapability = {
  ...agentManifest,
  lifecycle: { ...agentManifest.lifecycle, onActivate: registerAgentContributions },
}
