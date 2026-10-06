import { skillManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"

export const SKILL_CAPABILITY_ID = "skill"

const registerSkill = createLazyContributionRegistrar(skillManifest, [
  { id: "skill.main.panel", type: "surface", slot: "workbench-main", view: "skills", label: "技能", icon: "🛠️", panelState: true, load: () => import("./ui/SkillManagerPanel.vue") },
])
export function registerSkillContributions(): void { registerSkill() }

export const skillCapability = {
  ...skillManifest,
  lifecycle: { ...skillManifest.lifecycle, onActivate: registerSkillContributions },
}
