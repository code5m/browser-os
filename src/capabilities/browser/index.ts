// Browser Capability — 模块入口（Phase 8D Train C / C3）
//
// 经 **通用 Contribution Registry** 向 Shell 贡献 UI：
//   - browser-host（surface）     → 原生 webview 宿主（BrowserHost），Shell 只按槽渲染，不 import 其内部
//   - browser-dock（surface view=net/session） → 右侧 Dock 资源瀑布 / 历史会话面板
// 业务真源仍是 state/useBrowserStore（Semantic Registry 经 owner_implementations 解析新路径）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/BrowserHost → composables/useBrowserHost。

import { defineAsyncComponent } from "vue"
import { browserManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const BROWSER_CAPABILITY_ID = "browser"

const BrowserHost = defineAsyncComponent(() => import("./ui/BrowserHost.vue"))
const ResourceWaterfall = defineAsyncComponent(() => import("./ui/ResourceWaterfall.vue"))
const SessionPanel = defineAsyncComponent(() => import("./ui/SessionPanel.vue"))

/**
 * 注册 Browser 对 Shell 的贡献（Contribution/Slot 模型）。
 * Browser absent → browser-host 槽为空 → Shell 不创建原生 webview（C3 ABSENT 关键）。
 */
export function registerBrowserContributions(): void {
  contributionRegistry.registerContribution({
    id: "browser.host",
    capabilityId: BROWSER_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BROWSER_HOST,
    component: BrowserHost,
  })
  contributionRegistry.registerContribution({
    id: "browser.dock.net",
    capabilityId: BROWSER_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BROWSER_DOCK,
    view: "net",
    component: ResourceWaterfall,
    // Dock 页签展示元数据（保持与迁移前完全一致的图标/文案/顺序）
    label: "资源",
    icon: "🌊",
    order: 30,
  })
  contributionRegistry.registerContribution({
    id: "browser.dock.session",
    capabilityId: BROWSER_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BROWSER_DOCK,
    view: "session",
    component: SessionPanel,
    label: "会话",
    icon: "💾",
    order: 40,
  })
}

export const browserCapability = {
  ...browserManifest,
  lifecycle: {
    ...browserManifest.lifecycle,
    onActivate: registerBrowserContributions,
  },
}
