// Capability Runtime — 最小实现（Phase 7C）
//
// 硬约束（违反即为设计缺陷）：
//   1. Runtime 只做编排（orchestration）：register / resolve / enable / disable /
//      activate / suspend / inspect
//   2. Runtime **绝不成为业务状态 Owner**
//      —— 不管理 Browser tabs、不管理 Terminal 进程、不管理 Credential
//      —— 业务真源仍在 Semantic Registry 登记的 owner 手中
//   3. 不引入 DI 容器、不做反射加载、不做 God Runtime
//   4. Runtime 自己持有的是"编排元数据"（id / state / enabled），不是业务状态
//
// 与 docs/architecture/capability-registry/README.md §5 一致。

import type {
  CapabilityDefinition,
  CapabilityState,
  CapabilityContext,
} from './types'

export type CapabilityRuntimeErrorCode =
  | 'DUPLICATE_ID'
  | 'INVALID_DEFINITION'
  | 'NOT_FOUND'
  | 'MISSING_DEPENDENCY'
  | 'NOT_ACTIVATABLE'
  | 'SUSPEND_NOT_SUPPORTED'
  | 'INVALID_TRANSITION'
  | 'UNSAFE_OPERATION'
  | 'DEPENDENT_PRESENT'
  | 'TRANSITION_IN_PROGRESS'
  | 'TRANSITION_CANCELLED'

export class CapabilityRuntimeError extends Error {
  code: CapabilityRuntimeErrorCode
  constructor(code: CapabilityRuntimeErrorCode, message: string) {
    super(message)
    this.name = 'CapabilityRuntimeError'
    this.code = code
  }
}

/** Runtime 持有的编排记录 —— 只含元数据，不含业务状态 */
export interface CapabilityRecord {
  id: string
  definition: CapabilityDefinition
  state: CapabilityState
  enabled: boolean
  /** Last successful/failed activation hook duration; runtime diagnostic only. */
  activationDurationMs: number | null
  /** Last lifecycle error observed by the runtime; never a business-state mirror. */
  lastError: string | null
}

export interface CapabilityInspectionEntry {
  id: string
  name: string
  category: string
  state: CapabilityState
  enabled: boolean
  governanceStatus: string
  status: string
  resourceClass: string[]
  resident: boolean
  activationDurationMs: number | null
  lastError: string | null
}

export interface CapabilityRuntime {
  register(definition: CapabilityDefinition): CapabilityRecord
  resolve(id: string): CapabilityRecord
  activate(id: string): CapabilityRecord
  suspend(id: string): CapabilityRecord
  /** 交互式编排等待异步 hook 完成后才提交状态；既有同步启动入口保持不变。 */
  activateAsync(id: string): Promise<CapabilityRecord>
  suspendAsync(id: string): Promise<CapabilityRecord>
  /** Abort an in-flight activation. The method never marks resources released prematurely. */
  cancelTransition(id: string): boolean
  /** Owner cleanup receipts mean hooks completed, not proof of OS handles being zero. */
  releaseAsync(id: string): Promise<{ capabilityId: string; ownerCleanup: boolean }>
  enable(id: string): CapabilityRecord
  disable(id: string): CapabilityRecord
  /** HP2：从 runtime 移除能力记录（此前必须已 DISABLED；存在强依赖方则拒绝） */
  unregister(id: string): void
  get(id: string): CapabilityRecord | undefined
  inspect(): CapabilityInspectionEntry[]
  /** 仅供测试：清空注册表 */
  reset(): void
}

/**
 * 外部基础设施依赖（**非 Capability**）：一律视为「壳已提供」，不参与能力注册校验。
 * - `bridge`：native IPC 适配层（各能力 FULL-STACK 文档记为 PUBLIC_DEPENDENCY），随壳恒存在；
 * V2: credential 已升级为正式 resident capability，不再作为外部豁免。
 * 这里只保留真正随壳恒存在、没有独立生命周期的 bridge。
 */
const EXTERNAL_INFRA_DEPS = new Set(['bridge'])

function assertDefinition(def: CapabilityDefinition): void {
  if (!def || typeof def.id !== 'string' || def.id === '') {
    throw new CapabilityRuntimeError('INVALID_DEFINITION', 'capability id 必须为非空字符串')
  }
  if (!Array.isArray(def.dependsOn) || !Array.isArray(def.optionalDependencies)) {
    throw new CapabilityRuntimeError('INVALID_DEFINITION', 'dependsOn / optionalDependencies 必须为数组')
  }
  if (!def.lifecycle) {
    throw new CapabilityRuntimeError('INVALID_DEFINITION', 'lifecycle 缺失')
  }
}

function makeContext(rec: CapabilityRecord, log: (m: string) => void): CapabilityContext {
  return {
    capabilityId: rec.id,
    state: rec.state,
    log,
  }
}

export function createCapabilityRuntime(
  options: { log?: (message: string) => void } = {},
): CapabilityRuntime {
  const log = options.log || (() => {})
  const records = new Map<string, CapabilityRecord>()
  type Pending = { kind: 'activate' | 'suspend'; controller: AbortController; promise: Promise<CapabilityRecord> }
  const pending = new Map<string, Pending>()
  type ReleaseReceipt = { capabilityId: string; ownerCleanup: boolean }
  const releasing = new Map<string, Promise<ReleaseReceipt>>()
  // Completed Owner releases are cached until an explicit enable; disabled != released.
  const released = new Map<string, ReleaseReceipt>()

  function assertDependenciesNotClosing(definition: CapabilityDefinition): void {
    for (const dependency of definition.dependsOn ?? []) {
      if (releasing.has(dependency)) {
        throw new CapabilityRuntimeError('TRANSITION_IN_PROGRESS', `依赖能力正在关闭: ${dependency}`)
      }
    }
  }

  function assertIdle(id: string): void {
    if (pending.has(id) || releasing.has(id)) throw new CapabilityRuntimeError('TRANSITION_IN_PROGRESS', `能力转换或释放进行中: ${id}`)
  }
  function transition(
    id: string,
    kind: Pending['kind'],
    work: (signal: AbortSignal) => Promise<CapabilityRecord>,
  ): Promise<CapabilityRecord> {
    const old = pending.get(id)
    if (old) {
      if (old.kind === kind) return old.promise // idempotent concurrent callers
      return Promise.reject(new CapabilityRuntimeError('TRANSITION_IN_PROGRESS', `能力正在 ${old.kind}: ${id}`))
    }
    const controller = new AbortController()
    let promise!: Promise<CapabilityRecord>
    promise = (async () => {
      try { return await work(controller.signal) }
      finally { if (pending.get(id)?.promise === promise) pending.delete(id) }
    })()
    pending.set(id, { kind, controller, promise })
    return promise
  }

  function must(id: string): CapabilityRecord {
    const rec = records.get(id)
    if (!rec) throw new CapabilityRuntimeError('NOT_FOUND', `能力未注册: ${id}`)
    return rec
  }

  function activatable(id: string): CapabilityRecord {
    const rec = must(id)
    if (!rec.enabled) {
      throw new CapabilityRuntimeError('INVALID_TRANSITION', `能力已停用，无法激活: ${id}`)
    }
    if (rec.state !== 'READY' && rec.state !== 'SUSPENDED') {
      throw new CapabilityRuntimeError('INVALID_TRANSITION', `当前状态 ${rec.state} 不可激活（须 READY 或 SUSPENDED）: ${id}`)
    }
    if (rec.definition.status !== 'COMPATIBILITY_WRAPPED' || rec.definition.lifecycle.activatable !== true) {
      throw new CapabilityRuntimeError('NOT_ACTIVATABLE', `能力未声明可激活（status=${rec.definition.status}, activatable=${rec.definition.lifecycle.activatable}）: ${id}`)
    }
    return rec
  }

  function suspendable(id: string): CapabilityRecord {
    const rec = must(id)
    if (rec.state !== 'ACTIVE') {
      throw new CapabilityRuntimeError('INVALID_TRANSITION', `当前状态 ${rec.state} 不可暂停: ${id}`)
    }
    if (!rec.definition.lifecycle.supported.includes('SUSPENDED') || rec.definition.resources?.suspendable !== true) {
      throw new CapabilityRuntimeError('SUSPEND_NOT_SUPPORTED', `能力未声明支持 suspend: ${id}`)
    }
    return rec
  }

  function assertDisableAllowed(id: string): CapabilityRecord {
    const rec = must(id)
    if (rec.definition.governanceStatus !== 'GOVERNED' || rec.definition.lifecycle.resident === true) {
      throw new CapabilityRuntimeError('UNSAFE_OPERATION', `不允许停用（governanceStatus=${rec.definition.governanceStatus}, resident=${rec.definition.lifecycle.resident}）: ${id}`)
    }
    const dependents = [...records.values()]
      .filter(candidate => candidate.id !== id && candidate.enabled && candidate.definition.dependsOn.includes(id))
      .map(candidate => candidate.id)
    if (dependents.length) {
      throw new CapabilityRuntimeError('DEPENDENT_PRESENT', `仍有能力依赖 ${id}：${dependents.join(', ')}`)
    }
    return rec
  }

  function disableRecord(id: string): CapabilityRecord {
    const rec = assertDisableAllowed(id)
    if (rec.state === 'ACTIVE') throw new CapabilityRuntimeError('INVALID_TRANSITION', `ACTIVE 状态不可直接停用，请先 suspend: ${id}`)
    rec.enabled = false
    log(`disable: ${id}`)
    return rec
  }

  return {
    register(definition) {
      assertDefinition(definition)
      assertDependenciesNotClosing(definition)
      if (records.has(definition.id)) {
        throw new CapabilityRuntimeError('DUPLICATE_ID', `重复注册: ${definition.id}`)
      }
      const rec: CapabilityRecord = {
        id: definition.id,
        definition,
        state: 'DEFINED',
        enabled: true,
        activationDurationMs: null,
        lastError: null,
      }
      records.set(definition.id, rec)
      log(`register: ${definition.id}`)
      return rec
    },

    resolve(id) {
      assertIdle(id)
      const rec = must(id)
      // 外部基础设施（bridge/credential）非能力，豁免；能力依赖仍严格校验。
      const missing = (rec.definition.dependsOn || []).filter(
        (d) => !records.has(d) && !EXTERNAL_INFRA_DEPS.has(d),
      )
      if (missing.length > 0) {
        throw new CapabilityRuntimeError(
          'MISSING_DEPENDENCY',
          `强依赖未注册: ${missing.join(', ')}`,
        )
      }
      if (rec.state === 'DEFINED') rec.state = 'READY'
      log(`resolve: ${id} -> ${rec.state}`)
      return rec
    },

    activate(id) {
      assertIdle(id)
      const rec = activatable(id)
      assertDependenciesNotClosing(rec.definition)
      const ctx = makeContext(rec, log)
      const startedAt = Date.now()
      try {
        if (rec.definition.lifecycle.onActivate?.constructor.name === 'AsyncFunction') {
          throw new CapabilityRuntimeError('INVALID_TRANSITION', `异步 onActivate 必须使用 activateAsync: ${id}`)
        }
        const result = rec.definition.lifecycle.onActivate?.call(null)
        if (result && typeof (result as Promise<void>).then === 'function') {
          // The synchronous transition must never report ACTIVE while async work is pending.
          void Promise.resolve(result).catch(() => {})
          throw new CapabilityRuntimeError('INVALID_TRANSITION', `异步 onActivate 必须使用 activateAsync: ${id}`)
        }
        void ctx
        rec.state = 'ACTIVE'
        rec.activationDurationMs = Date.now() - startedAt
        rec.lastError = null
        log(`activate: ${id}`)
        return rec
      } catch (error) {
        rec.activationDurationMs = Date.now() - startedAt
        rec.lastError = error instanceof Error ? error.message : String(error)
        throw error
      }
    },

    suspend(id) {
      assertIdle(id)
      const rec = suspendable(id)
      if (rec.definition.lifecycle.onSuspend?.constructor.name === 'AsyncFunction') {
        throw new CapabilityRuntimeError('INVALID_TRANSITION', `异步 onSuspend 必须使用 suspendAsync: ${id}`)
      }
      const result = rec.definition.lifecycle.onSuspend?.call(null)
      if (result && typeof (result as Promise<void>).then === 'function') {
        void Promise.resolve(result).catch(() => {})
        throw new CapabilityRuntimeError('INVALID_TRANSITION', `异步 onSuspend 必须使用 suspendAsync: ${id}`)
      }
      rec.state = 'SUSPENDED'
      log(`suspend: ${id}`)
      return rec
    },

    activateAsync(id) {
      if (releasing.has(id)) return Promise.reject(new CapabilityRuntimeError('TRANSITION_IN_PROGRESS', `能力正在释放: ${id}`))
      const old = pending.get(id)
      if (old) return old.kind === 'activate' ? old.promise : Promise.reject(new CapabilityRuntimeError('TRANSITION_IN_PROGRESS', `正在暂停: ${id}`))
      const rec = activatable(id)
      assertDependenciesNotClosing(rec.definition)
      return transition(id, 'activate', async signal => {
        const startedAt = Date.now()
        let startedHook = false
        try {
          if (signal.aborted) throw new CapabilityRuntimeError('TRANSITION_CANCELLED', `激活已取消: ${id}`)
          startedHook = true
          await rec.definition.lifecycle.onActivate?.({ signal })
          if (signal.aborted) throw new CapabilityRuntimeError('TRANSITION_CANCELLED', `激活已取消: ${id}`)
          rec.state = 'ACTIVE'
          rec.activationDurationMs = Date.now() - startedAt
          rec.lastError = null
          log(`activate: ${id}`)
          return rec
        } catch (error) {
          rec.activationDurationMs = Date.now() - startedAt
          let message = error instanceof Error ? error.message : String(error)
          if (startedHook) {
            // Cleanup is explicitly delegated to the Owner, even on late failure.
            try { await rec.definition.lifecycle.onDeactivate?.({ signal: new AbortController().signal }) }
            catch (cleanupError) {
              message += `; cleanup failed: ${String(cleanupError)}`
              rec.lastError = message
              throw new CapabilityRuntimeError('UNSAFE_OPERATION', message)
            }
          }
          rec.lastError = message
          throw error
        }
      })
    },

    suspendAsync(id) {
      const old = pending.get(id)
      if (old) return old.kind === 'suspend' ? old.promise : Promise.reject(new CapabilityRuntimeError('TRANSITION_IN_PROGRESS', `正在激活: ${id}`))
      const rec = suspendable(id)
      return transition(id, 'suspend', async signal => {
        try {
          await rec.definition.lifecycle.onSuspend?.({ signal })
          rec.state = 'SUSPENDED'
          rec.lastError = null
          log(`suspend: ${id}`)
          return rec
        } catch (error) {
          rec.lastError = error instanceof Error ? error.message : String(error)
          throw error
        }
      })
    },

    cancelTransition(id) {
      must(id)
      const current = pending.get(id)
      if (!current || current.kind !== 'activate') return false
      current.controller.abort()
      return true
    },

    releaseAsync(id) {
      const currentRelease = releasing.get(id)
      if (currentRelease) return currentRelease
      const oldReceipt = released.get(id)
      if (oldReceipt) return Promise.resolve(oldReceipt)
      const record = assertDisableAllowed(id) // Preflight before ANY Owner hook or cancellation.
      if (!record.enabled) {
        return Promise.reject(new CapabilityRuntimeError('INVALID_TRANSITION', `能力已停用但没有 Owner 释放凭证: ${id}`))
      }
      let promise!: Promise<ReleaseReceipt>
      promise = Promise.resolve().then(async () => {
        const current = pending.get(id)
        if (current?.kind === 'activate') {
          current.controller.abort()
          try { await current.promise } catch { /* Owner rollback result is recorded */ }
        } else if (current) {
          await current.promise
        }
        const rec = must(id)
        if (rec.state === 'ACTIVE') await this.suspendAsync(id)
        const release = rec.definition.lifecycle.onDestroy ?? rec.definition.lifecycle.onDeactivate
        const ownerCleanup = !!release
        if (release) {
          try { await release({ signal: new AbortController().signal }) }
          catch (error) {
            rec.lastError = `Owner cleanup failed: ${String(error)}`
            throw error
          }
        }
        // Defensive verification before committing metadata (preflight already passed).
        disableRecord(id)
        const receipt = { capabilityId: id, ownerCleanup }
        released.set(id, receipt)
        return receipt
      }).finally(() => { if (releasing.get(id) === promise) releasing.delete(id) })
      releasing.set(id, promise)
      return promise
    },

    enable(id) {
      assertIdle(id)
      const rec = must(id)
      assertDependenciesNotClosing(rec.definition)
      released.delete(id) // A new activation opens a new resource lifecycle.
      rec.enabled = true
      log(`enable: ${id}`)
      return rec
    },

    disable(id) {
      assertIdle(id)
      return disableRecord(id)
    },

    unregister(id) {
      assertIdle(id)
      const rec = must(id)
      if (rec.definition.lifecycle.resident === true) {
        throw new CapabilityRuntimeError('UNSAFE_OPERATION', `常驻能力不可移除: ${id}`)
      }
      if (rec.state === 'ACTIVE') {
        throw new CapabilityRuntimeError('INVALID_TRANSITION', `ACTIVE 状态不可直接移除，请先停用: ${id}`)
      }
      const dependents = [...records.values()]
        .filter((r) => r.id !== id && (r.definition.dependsOn || []).includes(id) && r.enabled)
        .map((r) => r.id)
        .sort()
      if (dependents.length > 0) {
        throw new CapabilityRuntimeError('DEPENDENT_PRESENT', `存在强依赖方，拒绝移除: ${dependents.join(', ')}`)
      }
      records.delete(id)
      released.delete(id)
      log(`unregister: ${id}`)
    },

    get(id) {
      return records.get(id)
    },

    inspect() {
      return [...records.values()].map((r) => ({
        id: r.id,
        name: r.definition.name,
        category: r.definition.category,
        state: r.state,
        enabled: r.enabled,
        governanceStatus: r.definition.governanceStatus,
        status: r.definition.status,
        resourceClass: r.definition.resources?.class || [],
        resident: r.definition.lifecycle.resident === true,
        activationDurationMs: r.activationDurationMs,
        lastError: r.lastError,
      }))
    },

    reset() {
      if (pending.size > 0 || releasing.size > 0) throw new CapabilityRuntimeError('TRANSITION_IN_PROGRESS', '不能清空运行中的生命周期')
      records.clear()
      released.clear()
    },
  }
}
