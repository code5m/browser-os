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
}

export interface CapabilityRuntime {
  register(definition: CapabilityDefinition): CapabilityRecord
  resolve(id: string): CapabilityRecord
  activate(id: string): CapabilityRecord
  suspend(id: string): CapabilityRecord
  enable(id: string): CapabilityRecord
  disable(id: string): CapabilityRecord
  get(id: string): CapabilityRecord | undefined
  inspect(): CapabilityInspectionEntry[]
  /** 仅供测试：清空注册表 */
  reset(): void
}

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

  function must(id: string): CapabilityRecord {
    const rec = records.get(id)
    if (!rec) throw new CapabilityRuntimeError('NOT_FOUND', `能力未注册: ${id}`)
    return rec
  }

  return {
    register(definition) {
      assertDefinition(definition)
      if (records.has(definition.id)) {
        throw new CapabilityRuntimeError('DUPLICATE_ID', `重复注册: ${definition.id}`)
      }
      const rec: CapabilityRecord = {
        id: definition.id,
        definition,
        state: 'DEFINED',
        enabled: true,
      }
      records.set(definition.id, rec)
      log(`register: ${definition.id}`)
      return rec
    },

    resolve(id) {
      const rec = must(id)
      const missing = (rec.definition.dependsOn || []).filter((d) => !records.has(d))
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
      const rec = must(id)
      if (!rec.enabled) {
        throw new CapabilityRuntimeError('INVALID_TRANSITION', `能力已停用，无法激活: ${id}`)
      }
      if (rec.state !== 'READY' && rec.state !== 'SUSPENDED') {
        throw new CapabilityRuntimeError(
          'INVALID_TRANSITION',
          `当前状态 ${rec.state} 不可激活（须 READY 或 SUSPENDED）: ${id}`,
        )
      }
      // 只有「已接入 Runtime 的兼容包装」能力才允许激活；TARGET/NOT_INTEGRATED 一律拒绝
      if (rec.definition.status !== 'COMPATIBILITY_WRAPPED' || rec.definition.lifecycle.activatable !== true) {
        throw new CapabilityRuntimeError(
          'NOT_ACTIVATABLE',
          `能力未声明可激活（status=${rec.definition.status}, activatable=${rec.definition.lifecycle.activatable}）: ${id}`,
        )
      }
      const ctx = makeContext(rec, log)
      rec.definition.lifecycle.onActivate?.call(null)
      void ctx
      rec.state = 'ACTIVE'
      log(`activate: ${id}`)
      return rec
    },

    suspend(id) {
      const rec = must(id)
      if (rec.state !== 'ACTIVE') {
        throw new CapabilityRuntimeError('INVALID_TRANSITION', `当前状态 ${rec.state} 不可暂停: ${id}`)
      }
      const supported = rec.definition.lifecycle.supported || []
      if (!supported.includes('SUSPENDED') || rec.definition.resources?.suspendable !== true) {
        throw new CapabilityRuntimeError(
          'SUSPEND_NOT_SUPPORTED',
          `能力未声明支持 suspend: ${id}`,
        )
      }
      rec.definition.lifecycle.onSuspend?.call(null)
      rec.state = 'SUSPENDED'
      log(`suspend: ${id}`)
      return rec
    },

    enable(id) {
      const rec = must(id)
      rec.enabled = true
      log(`enable: ${id}`)
      return rec
    },

    disable(id) {
      const rec = must(id)
      // 仅"安全能力"允许停用：已治理 + 非常驻
      if (rec.definition.governanceStatus !== 'GOVERNED' || rec.definition.lifecycle.resident === true) {
        throw new CapabilityRuntimeError(
          'UNSAFE_OPERATION',
          `不允许停用（governanceStatus=${rec.definition.governanceStatus}, resident=${rec.definition.lifecycle.resident}）: ${id}`,
        )
      }
      if (rec.state === 'ACTIVE') {
        throw new CapabilityRuntimeError('INVALID_TRANSITION', `ACTIVE 状态不可直接停用，请先 suspend: ${id}`)
      }
      rec.enabled = false
      log(`disable: ${id}`)
      return rec
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
      }))
    },

    reset() {
      records.clear()
    },
  }
}
