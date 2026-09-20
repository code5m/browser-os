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

let runtime: CapabilityRuntime | null = null
let lastError: string | null = null

export interface BootstrapResult {
  runtime: CapabilityRuntime
  activated: boolean
  error: string | null
}

export function bootstrapCapabilityRuntime(): BootstrapResult {
  if (runtime) {
    return {
      runtime,
      activated: runtime.get(BOOKMARK_CAPABILITY_ID)?.state === 'ACTIVE',
      error: lastError,
    }
  }
  const rt = createCapabilityRuntime()
  let activated = false
  try {
    rt.register(bookmarkCapability)
    rt.resolve(BOOKMARK_CAPABILITY_ID)
    rt.activate(BOOKMARK_CAPABILITY_ID)
    rt.register(workspaceCapability)
    rt.resolve(WORKSPACE_CAPABILITY_ID)
    rt.activate(WORKSPACE_CAPABILITY_ID)
    rt.register(browserCapability)
    rt.resolve(BROWSER_CAPABILITY_ID)
    rt.activate(BROWSER_CAPABILITY_ID)
    rt.register(terminalCapability)
    rt.resolve(TERMINAL_CAPABILITY_ID)
    rt.activate(TERMINAL_CAPABILITY_ID)
    activated = true
  } catch (e) {
    lastError = e instanceof Error ? `${e.code}: ${e.message}` : String(e)
  }
  runtime = rt
  return { runtime: rt, activated, error: lastError }
}

export function getCapabilityRuntime(): CapabilityRuntime | null {
  return runtime
}

/** 供 UI / 诊断读取能力清单（只读编排元数据） */
export function inspectCapabilities() {
  return runtime ? runtime.inspect() : []
}
