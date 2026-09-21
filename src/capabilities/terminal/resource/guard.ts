// Terminal Capability — 重资源准入判定（与 Browser guard 同构，防同类泄漏复发）
//
// PTY 是 Terminal capability-owned 重资源：只能由 Terminal Capability
// 受控的生命周期路径创建。判定输入只有「能力可用性 + 激活态」一份编排真源，
// 不读 UI preference、不读 profile 名、不读环境变量、不建第二真源。
//
// fail-closed：Runtime 未 bootstrap / Terminal 未注册 / 未 ACTIVE → false。

import { isCapabilityActive } from '../../../capability/runtimeSingleton'
// 同 Browser guard：能力身份取自 manifest（纯 TS 叶子），刻意不 import ../index（会拉进 .vue）。
import { terminalManifest } from '../manifest'

/** Terminal capability-owned 重资源（PTY）是否获准创建。 */
export function isTerminalResourceAllowed(): boolean {
  return isCapabilityActive(terminalManifest.id)
}
