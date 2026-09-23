// Terminal Capability — 模块入口（Phase 8E / Train D / C3）
//
// 经 **通用 Contribution Registry** 向 Shell 贡献 UI：
//   - workbench-main-resident（view=term） → 终端主视图（常驻挂载，自身按 mainView 显隐；
//                                             xterm 实例切走不卸载，PTY 会话保持）
//   - browser-dock（view=term）            → 浏览时右侧 Dock 终端（竖排）
// 业务真源仍是 state/useTerminalStore（Semantic Registry 经 owner_implementations 解析新路径）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；不 import bridge（native 调用收敛在
// state/ 与 ui/ 内）。absent（未注册/未 activate）→ 无贡献 → 不渲染终端 DOM → **不创建 PTY**。

import { defineAsyncComponent } from "vue"
import { terminalManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const TERMINAL_CAPABILITY_ID = "terminal"

const TerminalView = defineAsyncComponent(() => import("./ui/TerminalView.vue"))
const TerminalDockPanel = defineAsyncComponent(() => import("./ui/TerminalDockPanel.vue"))

/**
 * 注册 Terminal 对 Shell 的贡献（Contribution/Slot 模型）。
 * Terminal absent → 两个槽位为空 → Shell 渲染空集，且**没有任何 PTY 出生点**（C3 ABSENT 关键）。
 */
export function registerTerminalContributions(): void {
  contributionRegistry.registerContribution({
    id: "terminal.main.term",
    capabilityId: TERMINAL_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN_RESIDENT,
    view: "term",
    component: TerminalView,
  })
  contributionRegistry.registerContribution({
    id: "terminal.dock.term",
    capabilityId: TERMINAL_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BROWSER_DOCK,
    view: "term",
    component: TerminalDockPanel,
    label: "终端",
    icon: "💻",
    order: 20,
  })
}

export const terminalCapability = {
  ...terminalManifest,
  lifecycle: {
    ...terminalManifest.lifecycle,
    onActivate: registerTerminalContributions,
  },
}
