// Workspace Capability — 模块入口（Phase 8C Train B / C3）
//
// 兼容优先：声明 manifest + 经 **通用 Contribution Registry** 向 Shell 贡献 UI（Contribution/Slot 模型）。
// Shell 遍历 slot 按 view 渲染，不 import 本能力内部 store / ui（C3 关键）。
// 业务真源仍是 state/* 内的 owner store（Semantic Registry 经 owner_implementations 解析新路径）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态。

import { defineAsyncComponent } from "vue"
import { workspaceManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const WORKSPACE_CAPABILITY_ID = "workspace"

// 主视图组件经异步懒加载注册（defineAsyncComponent）：组件位于本能力包内（ui/），
// Shell 只经 <component :is> 渲染，绝不直连。
const FileIdeView = defineAsyncComponent(() => import("./ui/FileIdeView.vue"))
const ArtifactPanel = defineAsyncComponent(() => import("./ui/ArtifactPanel.vue"))
const RepoPanel = defineAsyncComponent(() => import("./ui/RepoPanel.vue"))
const ScriptPanel = defineAsyncComponent(() => import("./ui/ScriptPanel.vue"))
const CommandSnippetPanel = defineAsyncComponent(() => import("./ui/CommandSnippetPanel.vue"))
const AuditPanel = defineAsyncComponent(() => import("./ui/AuditPanel.vue"))
const FileEditor = defineAsyncComponent(() => import("./ui/FileEditor.vue"))
const FilePanel = defineAsyncComponent(() => import("./ui/FilePanel.vue"))

function registerMainView(view: string, component: unknown): void {
  contributionRegistry.registerContribution({
    id: `workspace.main.${view}`,
    capabilityId: WORKSPACE_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view,
    component: component as never,
  })
}

/**
 * 注册 Workspace 对 Shell 的主视图贡献（Contribution/Slot 模型）：
 *   - workbench-main  files/arts/repo/scripts/commands/audit/editor → 主工作区视图
 *   - browser-dock    files                                        → 浏览器右侧 Dock 文件面板
 * 组件内部读取各自 store，Shell 零 Workspace 专属知识（C3 关键）。
 */
export function registerWorkspaceContributions(): void {
  registerMainView("files", FileIdeView)
  registerMainView("arts", ArtifactPanel)
  registerMainView("repo", RepoPanel)
  registerMainView("scripts", ScriptPanel)
  registerMainView("commands", CommandSnippetPanel)
  registerMainView("audit", AuditPanel)
  registerMainView("editor", FileEditor)
  contributionRegistry.registerContribution({
    id: "workspace.dock.files",
    capabilityId: WORKSPACE_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BROWSER_DOCK,
    view: "files",
    component: FilePanel,
  })
}

export const workspaceCapability = {
  ...workspaceManifest,
  lifecycle: {
    ...workspaceManifest.lifecycle,
    onActivate: registerWorkspaceContributions,
  },
}
