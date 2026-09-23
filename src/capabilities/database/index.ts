// Database Capability — 模块入口（Capability Library Expansion v1）
//
// 经 **通用 Contribution Registry** 向 MainArea 贡献主视图 UI：
//   - workbench-main（surface, view='db'）  → 主工作区「数据库」视图的 DatabasePanel
// MainArea 只按槽 + view 渲染，不 import 本能力内部（C3 关键，且避免 workspace→database 反向依赖环）。
//
// 硬约束：本文件（能力适配器）不持有、不读写任何业务状态；原生调用收敛在 ui/ 与 store 内。
// Database absent（未注册/未 activate）→ workbench-main 槽中无 view='db' 贡献 → MainArea 不渲染 DatabasePanel。
//
// 资源法（§18）：DatabasePanel.onMounted 仅 db.refreshConnections()（列配置，不建连接）；
// 连接仅在用户点击「连接」时 db.connect() 触发，且 Rust 侧每次 connect/query 瞬态建连即弃。
// 故 absent → 无面板 → 无 connect → 无 keyring 读、无 DbPool（无隐藏重资源）。

import { defineAsyncComponent } from "vue"
import { databaseManifest } from "./manifest"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"

export const DATABASE_CAPABILITY_ID = "database"

// M4-4 懒加载保持：15KB+ dbUi + useDatabaseStore 仍拆出主 chunk（IF-2 构建体积闸门）。
const panelLoading = {
  render: () =>
    // @ts-expect-error 纯展示兜底组件
    h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () =>
    // @ts-expect-error 纯展示兜底组件
    h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}
const DatabasePanel = defineAsyncComponent({
  loader: () => import("./ui/DatabasePanel.vue"),
  loadingComponent: panelLoading,
  errorComponent: panelError,
  delay: 80,
  timeout: 10000,
})

/**
 * 注册 Database 对 MainArea 的贡献（WORKBENCH_MAIN 槽, view='db'）。
 * Database absent → 槽中无 view='db' 贡献 → MainArea 的 viewOf('db') 返回 undefined → 不渲染 DatabasePanel。
 */
export function registerDatabaseContributions(): void {
  contributionRegistry.registerContribution({
    id: "database.main.panel",
    capabilityId: DATABASE_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.WORKBENCH_MAIN,
    view: "db",
    label: "数据库",
    icon: "🗄️",
    component: DatabasePanel,
  })
}

export const databaseCapability = {
  ...databaseManifest,
  lifecycle: {
    ...databaseManifest.lifecycle,
    onActivate: registerDatabaseContributions,
  },
}
