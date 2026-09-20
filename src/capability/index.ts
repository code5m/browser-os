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

const ALL_CAPABILITIES = [
  { id: BOOKMARK_CAPABILITY_ID, def: bookmarkCapability },
  { id: WORKSPACE_CAPABILITY_ID, def: workspaceCapability },
  { id: BROWSER_CAPABILITY_ID, def: browserCapability },
  { id: TERMINAL_CAPABILITY_ID, def: terminalCapability },
]

let runtime: CapabilityRuntime | null = null
let lastError: string | null = null
let lastProfile: CapabilityProfileId = DEFAULT_PROFILE

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
  return { runtime: rt, activated, error: lastError, profile: pid }
}

export function getCapabilityRuntime(): CapabilityRuntime | null {
  return runtime
}

/** 供 UI / 诊断读取能力清单（只读编排元数据） */
export function inspectCapabilities() {
  return runtime ? runtime.inspect() : []
}
