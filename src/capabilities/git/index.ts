// Git Capability — 模块入口（Capability Library Expansion v1）
//
// 经 **通用 Contribution Registry** 向 RepoPanel 贡献 UI：
//   - repo-subview（surface）  → 仓库主视图内「状态」页签的 Git 面板
// RepoPanel 只按槽渲染，不 import 本能力内部（C3 关键，且避免 workspace→git 反向依赖环）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/ 与 store 内。
// Git absent（未注册/未 activate）→ repo-subview 槽为空 → RepoPanel 的 git 页签不渲染 GitPanel。

import { defineAsyncComponent } from "vue"
import { gitManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const GIT_CAPABILITY_ID = "git"

const GitPanel = defineAsyncComponent(() => import("./ui/GitPanel.vue"))
const GitHistory = defineAsyncComponent(() => import("./ui/GitHistory.vue"))

/**
 * 注册 Git 对 RepoPanel 的贡献（REPO_SUBVIEW 槽，按 view 认领两个子页签）。
 * Git absent → 槽为空 → RepoPanel 的 git 页签不渲染 GitPanel / GitHistory（能力模型一致：能力缺席即无面板）。
 */
export function registerGitContributions(): void {
  contributionRegistry.registerContribution({
    id: "git.repo.panel",
    capabilityId: GIT_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.REPO_SUBVIEW,
    view: "git",
    component: GitPanel,
  })
  contributionRegistry.registerContribution({
    id: "git.repo.history",
    capabilityId: GIT_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.REPO_SUBVIEW,
    view: "history",
    component: GitHistory,
  })
}

export const gitCapability = {
  ...gitManifest,
  lifecycle: {
    ...gitManifest.lifecycle,
    onActivate: registerGitContributions,
  },
}
