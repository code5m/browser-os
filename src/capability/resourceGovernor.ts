// Resource Governor v1（Phase 8E / Train F）
//
// 只协调 lifecycle，绝不持有业务 state，绝不成为 God Runtime，绝不直接操作
// Browser/Terminal/Credential 内部（只经 CapabilityRuntime 的公共编排 API）。
//
// 诚实边界：
//   - 本模块**不**释放资源（不 kill PTY、不关 webview、不碰密钥）。资源释放是各能力的
//     onSuspend/能力自有资源清理职责（owner 所有）。Governor 只调用 rt 的 public 转换。
//   - 若能力未声明支持 suspend，rt.suspend 会抛 SUSPEND_NOT_SUPPORTED；对于声明 HP2 的能力，
//     owner 的 onSuspend 仍负责决定当前资源态是否允许暂停（例如活动 PTY 可拒绝）。
//   - hibernate 在 v1 与 suspend 同义（运行时模型无独立 hibernate 态）；background 在能力
//     不支持 suspend 时降级为 disable。这些降级都是显式的，不是静默的「假装释放」。
//
// 设计：无单例、无内部状态；每次 createResourceGovernor(runtime) 只持一个 runtime 引用。

import type { CapabilityRuntime } from './runtime'

export type GovernedLifecycle =
  | 'activate'
  | 'background'
  | 'suspend'
  | 'hibernate'
  | 'destroy'

export interface ResourceGovernor {
  readonly runtime: CapabilityRuntime
  /** 激活（READY/SUSPENDED → ACTIVE）。原样透传 INVALID_TRANSITION 等错误。 */
  activate(id: string): Promise<void>
  /** 转入后台：优先 suspend（若能力支持），否则降级 disable。 */
  background(id: string): Promise<void>
  /** 暂停（若能力声明 suspendable）。owner hook 可按当前资源态拒绝。 */
  suspend(id: string): Promise<void>
  /** 休眠（v1 == suspend 同义；运行时无独立 hibernate 态）。 */
  hibernate(id: string): Promise<void>
  /** 等待 active 退出（若需要）再 disable；并不等价于释放所有底层句柄。 */
  destroy(id: string): Promise<{ capabilityId: string; ownerCleanup: boolean }>
  /** 当前各能力状态快照（只读编排元数据，不含业务 state）。 */
  snapshot(): { id: string; state: string; enabled: boolean }[]
}

export function createResourceGovernor(runtime: CapabilityRuntime): ResourceGovernor {
  async function activate(id: string) {
    await runtime.activateAsync(id)
  }
  async function background(id: string) {
    const rec = runtime.get(id)
    if (!rec) throw new Error(`GOVERNOR_NOT_FOUND: ${id}`)
    // 优先 suspend；不支持 suspend 的能力（suspendable:false）降级为 disable，不假装释放
    if (rec.definition.lifecycle.supported.includes('SUSPENDED') && rec.definition.resources?.suspendable === true) {
      await runtime.suspendAsync(id)
    } else {
      // ACTIVE non-suspendable resources cannot be called released by disabling metadata.
      if (rec.state === 'ACTIVE') throw new Error(`GOVERNOR_OWNER_SUSPEND_REQUIRED: ${id}`)
      // Fallback is a real lifecycle release, not metadata-only disable.
      // The Owner receipt is required before background() returns.
      await runtime.releaseAsync(id)
    }
  }
  async function suspend(id: string) {
    await runtime.suspendAsync(id) // owner hook 完成后才能承认已暂停
  }
  async function hibernate(id: string) {
    await suspend(id) // v1：hibernate == suspend
  }
  async function destroy(id: string) {
    return runtime.releaseAsync(id) // Owner cleanup must finish before disabling metadata.
  }
  function snapshot() {
    return runtime.inspect().map((r) => ({ id: r.id, state: r.state, enabled: r.enabled }))
  }
  return { runtime, activate, background, suspend, hibernate, destroy, snapshot }
}
