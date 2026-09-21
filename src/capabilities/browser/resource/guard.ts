// Browser Capability — 重资源准入判定（H-G RELEASE BLOCKER 修复）
//
// 唯一原则（架构 §12）：
//   Browser/Grid capability-owned 重资源（原生 WebView、宫格子进程）
//   只能由 **Browser Capability 受控的生命周期路径** 创建。
//
// 因此判定输入只能是「能力可用性 + 激活态」这一份编排真源，
// 绝不能是：
//   - Shell 的持久化 UI preference（gridToolbarOpen 之类）
//   - 硬编码 profile 名（framework / full ...）
//   - 环境变量（VITE_CAPABILITY_PROFILE）
//   - 任何 browserEnabled / gridVisible 之类的第二真源
//
// 四个必须严格区分的真相（缺一即复发同类 blocker）：
//   preference（用户想看到什么 UI）  ← 不参与本判定
//   availability（能力是否已注册）   ← 参与
//   activation（能力是否 ACTIVE）    ← 参与
//   resource existence（资源是否已存在）← 能力内部 gridOpen，不参与本判定
//
// fail-closed：Runtime 未 bootstrap / Browser 未注册 / 未 ACTIVE → false。
// 这样无论调用链来自何处（已知入口、历史遗留入口、未来新增入口），
// absent Browser 都不可能创建出宫格子进程。

import { isCapabilityActive } from '../../../capability/runtimeSingleton'
// 能力身份取自 manifest（纯 TS 叶子，不 import .vue）—— 刻意不 import ../index，
// 否则会经 defineAsyncComponent 拉进 .vue，破坏 esbuild 门禁（无 .vue loader）。
import { browserManifest } from '../manifest'

/**
 * Browser capability-owned 重资源是否获准创建。
 * 唯一判定实现在 runtimeSingleton.isCapabilityActive —— 本函数只是能力侧的具名入口，
 * 让调用点（buildGrid）读起来就是「Browser 资源闸」，而不是通用布尔。
 */
export function isBrowserResourceAllowed(): boolean {
  return isCapabilityActive(browserManifest.id)
}
