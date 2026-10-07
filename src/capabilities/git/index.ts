import { gitManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"
export const GIT_CAPABILITY_ID = "git"
const registerGit = createLazyContributionRegistrar(gitManifest, [
  { id: "git.repo.panel", type: "surface", slot: "repo-subview", view: "git", load: () => import("./ui/GitPanel.vue") },
  { id: "git.repo.history", type: "surface", slot: "repo-subview", view: "history", load: () => import("./ui/GitHistory.vue") },
])
export function registerGitContributions(): void { registerGit() }
export const gitCapability = { ...gitManifest, lifecycle: { ...gitManifest.lifecycle, onActivate: registerGitContributions } }
