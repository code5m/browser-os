// Capability Bootstrap（Phase 7D）
//
// 应用侧入口：创建 Runtime、注册试点能力、resolve 并 activate。
// 设计要点：
//   1. 幂等（重复调用返回同一实例）
//   2. 绝不抛错打断应用启动 —— 能力层失败只记录，不影响既有功能
//   3. 不引入 DI、不做动态加载

import { createCapabilityRuntime, type CapabilityRuntime } from './runtime'
import { bookmarkCapability, BOOKMARK_CAPABILITY_ID } from '../capabilities/bookmark'
import { workspaceCapability, WORKSPACE_CAPABILITY_ID } from '../capabilities/workspace'
import { browserCapability, BROWSER_CAPABILITY_ID } from '../capabilities/browser'
import { terminalCapability, TERMINAL_CAPABILITY_ID } from '../capabilities/terminal'
import { CAPABILITY_PROFILES, DEFAULT_PROFILE, type CapabilityProfileId, profileFromEnv, resolveProfile } from './profiles'
import { CAPABILITY_CATALOG, CAPABILITY_DEFINITIONS } from './platform/catalog'
import { assemble } from './platform/assembly'
// H-G 修复：把 Runtime 单例发布到叶子模块，供能力内部在**调用时**判定
// 「本能力是否获准创建自己 owned 的重资源」。不这样做就会形成 ESM 循环。
import { setCapabilityRuntime, peekCapabilityRuntime } from './runtimeSingleton'

const ALL_CAPABILITIES = [
  { id: BOOKMARK_CAPABILITY_ID, def: bookmarkCapability },
  { id: WORKSPACE_CAPABILITY_ID, def: workspaceCapability },
  { id: BROWSER_CAPABILITY_ID, def: browserCapability },
  { id: TERMINAL_CAPABILITY_ID, def: terminalCapability },
]

let runtime: CapabilityRuntime | null = null
let lastError: string | null = null
let lastProfile: CapabilityProfileId = DEFAULT_PROFILE

/**
 * 自由装配源（非 preset）：VITE_CAPABILITY_ASSEMBLY="workspace,terminal"
 * 顺序无关——真正的解析/排序交给 Assembly Engine；未设置则返回 null（走 profile preset）。
 */
function customAssemblyFromEnv(): string[] | null {
  // @ts-expect-error Vite 注入
  const raw = typeof import.meta !== 'undefined' ? (import.meta.env?.VITE_CAPABILITY_ASSEMBLY as string | undefined) : undefined
  if (typeof raw !== 'string') return null
  return raw.split(',').map((s) => s.trim()).filter(Boolean)
}

export interface BootstrapResult {
  runtime: CapabilityRuntime
  activated: boolean
  error: string | null
  profile: CapabilityProfileId
}

/**
 * 按 profile 真实注册能力（不是 UI hide）。
 * @param profile 可选；缺省读运行环境（VITE_CAPABILITY_PROFILE）或默认 full。
 *   未列出的能力**不注册** → 其贡献槽为空 → 不加载其内部 store / 不创建重资源。
 */
export function bootstrapCapabilityRuntime(profile?: CapabilityProfileId | string | null): BootstrapResult {
  if (runtime) {
    return {
      runtime,
      activated: runtime.get(BOOKMARK_CAPABILITY_ID)?.state === 'ACTIVE',
      error: lastError,
      profile: lastProfile,
    }
  }
  // 自由装配优先：设置 VITE_CAPABILITY_ASSEMBLY 时按任意组合装配（不是 preset）。
  // 非法组合由 Assembly Engine 在启动前确定性拒绝——这里只记录错误 + 以零能力启动，绝不崩溃。
  const customIds = customAssemblyFromEnv()
  if (profile == null && customIds) {
    try {
      const built = bootstrapAssembly(customIds)
      runtime = built.runtime
      setCapabilityRuntime(runtime)
      lastProfile = 'custom'
      lastError = built.error
      return { runtime, activated: built.activated, error: lastError, profile: 'custom' }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e)
      const empty = createCapabilityRuntime()
      runtime = empty
      setCapabilityRuntime(runtime)
      return { runtime: empty, activated: false, error: lastError, profile: 'custom' }
    }
  }
  const pid = typeof profile === 'string' ? resolveProfile(profile) : (profile ?? profileFromEnv())
  lastProfile = pid
  const allowed = new Set(CAPABILITY_PROFILES[pid])
  const rt = createCapabilityRuntime()
  let activated = false
  try {
    for (const { id, def } of ALL_CAPABILITIES) {
      if (!allowed.has(id)) continue // profile 未列出 → 跳过注册（absent 语义）
      rt.register(def)
      rt.resolve(id)
      rt.activate(id)
    }
    activated = true
  } catch (e) {
    lastError = e instanceof Error ? `${e.code}: ${e.message}` : String(e)
  }
  runtime = rt
  setCapabilityRuntime(runtime)
  return { runtime: rt, activated, error: lastError, profile: pid }
}

/**
 * Assembly Engine 驱动的真实装配入口（§14）。
 *
 * 与 profile 的区别：profile 只是**预设**；这里接受任意合法能力集合（custom assembly），
 * 由 Dependency Resolver 决定 resolved / activationOrder，并按该顺序真实 register+activate。
 * 请求非法（强依赖缺失/环/冲突/未知）→ deterministic 抛错，绝不「启动后才 undefined」。
 *
 * 注意：本函数返回独立 runtime，不写 app 级单例（避免污染线上 bootstrap）。
 */
export function bootstrapAssembly(capabilityIds: string[]): BootstrapResult {
  const report = assemble(CAPABILITY_CATALOG, { capabilities: capabilityIds ?? [] })
  if (!report.ok) {
    throw new Error(
      `装配失败: ${report.rejections.map((r) => `${r.code}(${r.message})`).join('; ')}`,
    )
  }
  const rt = createCapabilityRuntime()
  let activated = false
  let error: string | null = null
  for (const id of report.activationOrder) {
    const def = CAPABILITY_DEFINITIONS[id]
    if (!def) continue
    try {
      rt.register(def)
      rt.resolve(id)
      rt.activate(id)
      activated = true
    } catch (e) {
      error = e instanceof Error ? `${e instanceof Error ? String((e as { code?: string }).code ?? '') : ''}: ${e.message}` : String(e)
    }
  }
  return { runtime: rt, activated, error, profile: 'custom' as CapabilityProfileId }
}

export function getCapabilityRuntime(): CapabilityRuntime | null {
  // 单例真源在 runtimeSingleton（叶子模块），本地 `runtime` 变量是其镜像。
  // 保持二者一致：能力内部读 runtimeSingleton，应用侧读本函数。
  return peekCapabilityRuntime() ?? runtime
}

// 能力的重资源准入判定复用叶子模块的同一份实现（见 runtimeSingleton.isCapabilityActive），
// 此处 re-export 仅为应用侧/诊断提供只读入口，避免各处再写一份判定造成第二真源。
export { isCapabilityActive } from './runtimeSingleton'

/** 供 UI / 诊断读取能力清单（只读编排元数据） */
export function inspectCapabilities() {
  return runtime ? runtime.inspect() : []
}
