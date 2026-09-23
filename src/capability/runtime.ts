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
 * - `credential`：OS keyring 基础设施（registry 标记 NOT_COMPOSABLE_BY_DESIGN 的常驻安全能力），随壳恒存在。
 *
 * 背景：manifest 的 `dependsOn` 同时承载「能力依赖」（如 git→workspace）与「基础设施前置」
 * （bridge/credential）。后者不是可注册能力，若按能力严格校验会导致跨能力 capability 永远
 * 无法激活（MISSING_DEPENDENCY），其 contribution 永不注册 → Shell 视图静默空白。
 * 本集合仅豁免基础设施；真正的能力依赖（workspace 等）仍严格校验（缺失即 MISSING_DEPENDENCY）。
 */
const EXTERNAL_INFRA_DEPS = new Set(['bridge', 'credential'])

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

    unregister(id) {
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
      }))
    },

    reset() {
      records.clear()
    },
  }
}
