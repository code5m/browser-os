// Capability Runtime 单例引用（叶子模块）
//
// 存在理由（架构，不是便利）：
//   能力内部（如 Browser 的 useBrowserStore）需要在**调用时**读取能力编排真源
//   （该能力是否注册 / 是否 ACTIVE），才能判断自己是否获准创建 capability-owned
//   重资源（宫格子进程 / 原生 WebView / PTY）。
//
//   若直接 `import { getCapabilityRuntime } from '../../capability'`，
//   而 src/capability/index.ts 又静态 import 了所有能力入口，就会形成
//   capability ↔ capability 的 ESM 循环，运行时拿到半初始化模块。
//   故把「单例引用」下沉到这个只依赖 type 的叶子模块，切断循环。
//
// 硬约束：
//   1. 本文件不 import 任何能力、不 import 任何业务 store —— 只持有编排引用。
//   2. 读取必须发生在**调用时**（函数内），不能在模块顶层求值：
//      main.ts 的 import 会被提升，App.vue（及 useBrowserStore）先于
//      bootstrapCapabilityRuntime() 执行；顶层求值必然读到 null。
//   3. 本模块不是业务状态 Owner，也不做任何编排决策。

import type { CapabilityRuntime } from './runtime'

let current: CapabilityRuntime | null = null

/** 由 src/capability/index.ts 在 bootstrap 时写入（唯一写入方）。 */
export function setCapabilityRuntime(rt: CapabilityRuntime | null): void {
  current = rt
}

/** 只读窥视当前 Runtime；未 bootstrap 时返回 null（调用方按 fail-closed 处理）。 */
export function peekCapabilityRuntime(): CapabilityRuntime | null {
  return current
}

/** 仅供测试：清空单例引用。 */
export function resetCapabilityRuntimeRef(): void {
  current = null
}

/**
 * 判定某能力当前是否获准创建自己 owned 的重资源（RESOURCE ISOLATION 判定真源）。
 *
 * 严格区分四个不是同一件事的真相：
 *   - preference（用户/持久化 UI 偏好）      —— 不是本函数输入
 *   - availability（能力是否已注册）          —— 参与判定
 *   - activation（能力是否 ACTIVE）           —— 参与判定
 *   - resource existence（资源是否已存在）    —— 能力内部状态，不参与本判定
 *
 * fail-closed：Runtime 未 bootstrap / 能力未注册 / 未 ACTIVE / 被 disable → false。
 * 各能力 guard（browser / terminal）一律复用本函数，禁止各自再写一份判定。
 */
export function isCapabilityActive(id: string): boolean {
  if (!current) return false
  const rec = current.get(id)
  return !!rec && rec.state === 'ACTIVE' && rec.enabled !== false
}
