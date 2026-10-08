import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateGitLifecycle, suspendGitLifecycle } from "./lifecycle"
import { gitManifest } from "./manifest"

export const GIT_CAPABILITY_ID = "git"

const git = createLifecycleContributionCapability(gitManifest, [
  { id: "git.repo.panel", type: "surface", slot: "repo-subview", view: "git", load: () => import("./ui/GitPanel.vue") },
  { id: "git.repo.history", type: "surface", slot: "repo-subview", view: "history", load: () => import("./ui/GitHistory.vue") },
], activateGitLifecycle, suspendGitLifecycle)

export const gitCapability = git.capability
export const registerGitContributions = git.register
