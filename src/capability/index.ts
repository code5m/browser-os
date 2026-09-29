// Capability Bootstrap（Phase 7D）
//
// 应用侧入口：创建 Runtime、注册试点能力、resolve 并 activate。
// 设计要点：
//   1. 幂等（重复调用返回同一实例）
//   2. 绝不抛错打断应用启动 —— 能力层失败只记录，不影响既有功能
//   3. 不引入 DI、不做动态加载

import { createCapabilityRuntime, type CapabilityRuntime } from './runtime'
import { CLIPBOARD_CAPABILITY_ID, createClipboardCapability, clipboardContribution, CLIPBOARD_PORTS_KEY, type ClipboardPorts } from '@browser-os/capability-clipboard'
import { useLayoutStore } from '../stores/useLayoutStore'
import { redactSecrets } from '../utils/redact'
import { createVaultCapability, vaultContribution, VAULT_PORTS_KEY, type VaultPorts } from '@browser-os/capability-vault'
import { bridge } from '../bridge'
import { useWorkbenchStore } from '../stores/useWorkbenchStore'
import { layoutPositions } from '../utils/graphUi'
import { contributionRegistry } from './contribution/registry'
import { registerSettingsContributions } from '../settings'
import { CAPABILITY_PROFILES, DEFAULT_PROFILE, type CapabilityProfileId, profileFromEnv, resolveProfile } from './profiles'
import { CAPABILITY_CATALOG, CAPABILITY_DEFINITIONS } from './platform/catalog'
import { assemble } from './platform/assembly'
import { configFromEnv, type CapabilityEnabledConfig } from './platform/config'
import { createPluggableRuntime } from './platform/pluggable'
// H-G 修复：把 Runtime 单例发布到叶子模块，供能力内部在**调用时**判定
// 「本能力是否获准创建自己 owned 的重资源」。不这样做就会形成 ESM 循环。
import { setCapabilityRuntime, peekCapabilityRuntime } from './runtimeSingleton'
import { hostServices } from './platform/host-services'

// 导出：保证任何打包器（esbuild / Rollup）都不会把「仅被 bootstrapCapabilityRuntime 内部
// 引用的能力入口」摇树删除——否则新增能力（如 git）会在组合测试甚至生产包中凭空消失。
// 顺序 = **依赖安全顺序**：bootstrap 逐条 register→resolve→activate，故能力依赖必须先行注册
// （如 git 依赖 workspace）。bridge/credential 为外部基础设施，由 runtime 豁免，不在此列。
// === Vault M2 package wiring（Host 侧适配）===
// 窄 Host Contract 显式提供：桥接 bridge.vaultOpen / 工作区折叠 / 图布局。
// 包本身不 import 这些 host 内部（满足 PKG-06）；仅 Host 在此组装。
hostServices.register('bridge', bridge)
hostServices.register('layout', useLayoutStore)
hostServices.register('workbench', useWorkbenchStore)
hostServices.register('graph-layout', (nodes: unknown[], edges: unknown[]) => layoutPositions(nodes as any, edges as any))
hostServices.register('redact-secrets', redactSecrets)
const vaultInstancePorts: VaultPorts = {
  native: { openVault: (path: string) => hostServices.require<typeof bridge>('bridge').vaultOpen(path) },
  shell: { isWorkbenchCollapsed: () => hostServices.require<typeof useWorkbenchStore>('workbench')().collapsed.value },
  graphLayout: { layoutGraph: (nodes, edges) => hostServices.require<(nodes: unknown[], edges: unknown[]) => unknown>('graph-layout')(nodes as any, edges as any) },
};
const vault = createVaultCapability(vaultInstancePorts);
// Host 负责注册贡献（避免 Vault → Host registry 反向依赖）：onActivate 时注册包导出的描述符。
// 注意：registry 方法名是 registerContribution（非 register），此前误写 register 导致 onActivate 抛错、
// 激活循环中断、其后的能力（home 等）一并无法注册（真实执行已复现）。
vault.vaultCapability.lifecycle.onActivate = () => contributionRegistry.registerContribution(vaultContribution as any);
/** 供 main.ts 经 app.provide(VAULT_PORTS_KEY, vaultPorts) 注入表现层契约 */
export const vaultPorts = vaultInstancePorts;

// === Clipboard M2 package wiring（Host 侧适配）===
// 窄 Host Contract 显式提供：桥接 clipboardRead/Write、showToast、requestClose(关面板)、redactSecrets。
// 包本身不 import 这些 host 内部（满足 PKG-06）；仅 Host 在此组装。
// 注：EmptyState 不进端口——本模块会被 check-capability-platform 等 node 侧 checker 经 esbuild 打包，
// 而 *.vue 被 external 后 node 无法加载；为与 Vault 一致，面板自渲染最小空态，避免端口引入 .vue。
const clipboardInstancePorts: ClipboardPorts = {
  native: {
    clipboardRead: () => hostServices.require<typeof bridge>('bridge').clipboardRead(),
    clipboardWrite: (text: string) => hostServices.require<typeof bridge>('bridge').clipboardWrite(text),
  },
  ui: {
    showToast: (message: string) => hostServices.require<ReturnType<typeof useLayoutStore>>('layout')().showToast(message),
    requestClose: () => {
      hostServices.require<ReturnType<typeof useLayoutStore>>('layout')().clipOpen = false;
    },
    redactSecrets: (text: string) => hostServices.require<(text: string) => string>('redact-secrets')(text),
  },
}
const clipboard = createClipboardCapability(clipboardInstancePorts)
// Host 负责注册贡献（避免 Clipboard → Host registry 反向依赖）：onActivate 时注册包导出的描述符。
clipboard.clipboardCapability.lifecycle.onActivate = () => contributionRegistry.registerContribution(clipboardContribution as any)
/** 供 main.ts 经 app.provide(CLIPBOARD_PORTS_KEY, clipboardPorts) 注入表现层契约 */
export const clipboardPorts = clipboardInstancePorts;

export const ALL_CAPABILITIES = Object.entries(CAPABILITY_DEFINITIONS)
  .map(([id, def]) => ({ id, def }))
  .map((entry) => entry.id === CLIPBOARD_CAPABILITY_ID ? { id: entry.id, def: clipboard.clipboardCapability } : entry)
  .map((entry) => entry.id === 'vault' ? { id: entry.id, def: vault.vaultCapability } : entry)

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

function enabledConfigFromEnv(): CapabilityEnabledConfig | null {
  // @ts-expect-error Vite 注入
  const env = typeof import.meta !== 'undefined' ? (import.meta.env ?? {}) : {}
  return configFromEnv(env as Record<string, unknown>)
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
  // 框架常驻 SERVICE（settings）贡献无条件注册：不依赖 profile、不经 runtime 激活
  // （settingsManifest.activatable=false，走 profile 循环会被运行时拒激活）。
  // 与 Vault 同为 Host 显式注册（PKG-06：包/模块不反向依赖 Host registry），
  // 但 settings 属 framework SERVICE，故在 bootstrap 首次初始化时直接注册其贡献。
  // 幂等：contributionRegistry.registerContribution 以 id 为键覆盖，重复调用安全。
  registerSettingsContributions()
  const envConfig = enabledConfigFromEnv()
  if (profile == null && envConfig) {
    try {
      const built = bootstrapConfiguredCapabilityRuntime(envConfig)
      lastProfile = 'custom'
      return { runtime: built.runtime, activated: built.activated, error: built.error, profile: 'custom' }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e)
      const empty = createCapabilityRuntime()
      runtime = empty
      setCapabilityRuntime(runtime)
      return { runtime: empty, activated: false, error: lastError, profile: 'custom' }
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

/**
 * Manifest + enabled 配置驱动的真实装配入口。
 * 依赖由 Manifest 自动补齐，注册/激活顺序由拓扑排序决定。
 */
export function bootstrapConfiguredCapabilityRuntime(
  config?: CapabilityEnabledConfig,
): BootstrapResult {
  const fallback = config ?? configFromEnv(import.meta.env ?? {}) ?? {
    enabled: Object.fromEntries(CAPABILITY_PROFILES[DEFAULT_PROFILE].map((id) => [id, true])),
  }
  const built = createPluggableRuntime({
    catalog: CAPABILITY_CATALOG,
    definitions: CAPABILITY_DEFINITIONS,
    config: fallback,
    contributions: contributionRegistry,
  })
  built.activate()
  runtime = built.runtime
  setCapabilityRuntime(runtime)
  return {
    runtime,
    activated: built.runtime.inspect().some((entry) => entry.state === 'ACTIVE'),
    error: null,
    profile: 'custom',
  }
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
