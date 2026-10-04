// Capability Manager 的唯一生命周期编排入口。
// 只协调 Runtime + Contribution Registry，不持有任何能力业务状态。

import type { CapabilityRuntime } from '../runtime'
import type { ContributionRegistry } from '../contribution/registry'
import type { CapabilityManifestV1 } from './contract'

export interface CapabilityManagerTransition {
  message: string
  persistedEnabled?: boolean
}

export type CapabilityManagerAction = 'enable' | 'pause' | 'resume' | 'disable'

// 按 Runtime 串行化操作，防止异步清理期间重新启用或改变依赖关系。
const pending = new WeakSet<CapabilityRuntime>()

/** C3 才能证明 absent 后 Shell 无死入口；HP1+ 才允许运行时开关。 */
export function isCapabilityToggleSafe(manifest: CapabilityManifestV1 | undefined): boolean {
  const maturity = Number(manifest?.maturity.slice(1) ?? 0)
  return maturity >= 3 && manifest?.kind !== 'core' && manifest?.hotPlug.enable === true && manifest.hotPlug.disable === true
}

export async function transitionCapability(
  runtime: CapabilityRuntime,
  registry: ContributionRegistry,
  id: string,
  action: CapabilityManagerAction,
): Promise<CapabilityManagerTransition> {
  if (pending.has(runtime)) throw new Error('能力操作进行中，请稍后重试')
  const record = runtime.get(id)
  if (!record) throw new Error(`${id} 当前未装配`)
  if (!isCapabilityToggleSafe(record.definition.v1)) {
    throw new Error(`${id} 尚未达到可安全切换的 C3/HP1 契约`)
  }
  if (record.definition.governanceStatus !== 'GOVERNED' || record.definition.lifecycle.resident) {
    throw new Error(`${id} 为常驻或尚未治理的能力，不允许切换`)
  }
  if (action === 'pause' || action === 'disable') {
    const dependents = runtime.inspect().filter((entry) => entry.enabled && entry.id !== id
      && runtime.get(entry.id)?.definition.dependsOn.includes(id))
    if (dependents.length) throw new Error(`仍有能力依赖 ${id}：${dependents.map((entry) => entry.id).join(', ')}`)
  } else {
    const unavailable = record.definition.dependsOn.filter((dependency) => {
      const required = runtime.get(dependency)
      return required && (!required.enabled || required.state !== 'ACTIVE')
    })
    if (unavailable.length) throw new Error(`请先启用依赖：${unavailable.join(', ')}`)
  }
  // 显式动作不随新状态自动变成下一步，避免陈旧点击跨越生命周期保护。
  if (action === 'disable' && (!record.enabled || record.state !== 'SUSPENDED')) {
    throw new Error(`${id} 必须先暂停，再停用`)
  }
  if (action === 'resume' && (!record.enabled || record.state !== 'SUSPENDED')) {
    throw new Error(`${id} 当前不可恢复`)
  }
  if (action === 'enable' && record.enabled && record.state !== 'READY' && record.state !== 'DEFINED') {
    throw new Error(`${id} 当前已启用`)
  }
  const wasEnabled = record.enabled
  const previousContributions = registry.getByCapability(id)
  pending.add(runtime)
  try {
    if (action === 'pause') {
      await runtime.suspendAsync(id)
      return { message: `${id} 已暂停，可恢复或继续停用（暂停仅对本次会话生效）` }
    }
    if (action === 'disable') {
      runtime.disable(id)
      await record.definition.lifecycle.onDeactivate?.()
      registry.unregisterCapability(id)
      return { message: `${id} 已停用`, persistedEnabled: false }
    }
    runtime.enable(id)
    runtime.resolve(id)
    await runtime.activateAsync(id)
    return { message: `${id} 已${action === 'resume' ? '恢复' : '启用'}`, persistedEnabled: true }
  } catch (error) {
    // 只恢复编排元数据和入口；业务资源的失败补偿仍由能力的 hook 负责。
    if (wasEnabled && !record.enabled) runtime.enable(id)
    if (!wasEnabled && record.enabled) runtime.disable(id)
    registry.unregisterCapability(id)
    for (const contribution of previousContributions) registry.registerContribution(contribution)
    throw error
  } finally {
    pending.delete(runtime)
  }
}
