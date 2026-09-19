// Capability SDK — 最小接口（Phase 7B）
//
// 设计约束：
//   1. 只有类型，没有实现 —— 实现在 Phase 7C（Runtime）
//   2. 不引入 DI 容器、不做反射加载、不做 God Runtime
//   3. Runtime 只做编排（orchestration），绝不成为业务状态 Owner
//   4. 语义（State/Intent/Owner/Writer/SideEffect）归 Semantic Registry，此处只引用
//
// 与 docs/architecture/capability-registry/README.md §5 一致。

export type CapabilityCategory =
  | 'CAPABILITY'
  | 'SUB_CAPABILITY'
  | 'UI_COMPONENT'
  | 'SERVICE'
  | 'ADAPTER'
  | 'INFRASTRUCTURE'
  | 'IMPLEMENTATION_DETAIL'

export type CapabilityState =
  | 'DEFINED'
  | 'READY'
  | 'ACTIVE'
  | 'BACKGROUND'
  | 'SUSPENDED'
  | 'DESTROYED'

export type ResourceClass =
  | 'LIGHT'
  | 'MEDIUM'
  | 'HEAVY'
  | 'VERY_HEAVY'
  | 'MULTI_WEBVIEW'
  | 'NATIVE'
  | 'PROCESS'
  | 'PTY'
  | 'WEBVIEW'
  | 'NETWORK'
  | 'BACKGROUND'
  | 'SECRET'
  | 'SECURITY_SENSITIVE'

export type GovernanceStatus = 'GOVERNED' | 'OWNER_PENDING_SCR' | 'LOCKED'

/** 禁止把目标态写成现状 */
export type IntegrationStatus =
  | 'NOT_INTEGRATED'
  | 'COMPATIBILITY_WRAPPED'
  | 'TARGET_COMPOSABLE'

export interface CapabilityResourcePolicy {
  /** DECLARED 分类（非实测） */
  class: ResourceClass[]
  suspendable: boolean
  destroyable: boolean
}

export interface CapabilityLifecycle {
  /** 该能力"声明"自己支持的状态；Runtime 对未声明的转换必须拒绝 */
  supported: CapabilityState[]
  default: CapabilityState
  /** 受硬规则约束：semanticOwner 未登记或 governanceStatus != GOVERNED 时必须为 false */
  activatable: boolean
  /** 常驻（不可卸载） */
  resident: boolean
  onActivate?: () => void | Promise<void>
  onSuspend?: () => void | Promise<void>
}

export interface CapabilityPersistence {
  scope: 'session' | 'disk' | 'os_keyring' | 'runtime_only'
  sensitive: boolean
}

export interface CapabilityDefinition {
  id: string
  name: string
  category: CapabilityCategory
  provides: string[]
  dependsOn: string[]
  optionalDependencies: string[]
  lifecycle: CapabilityLifecycle
  resources: CapabilityResourcePolicy
  permissions: string[]
  persistence: CapabilityPersistence
  entrypoint: string
  /** 引用 Semantic Registry 的 owner；未登记填 null */
  semanticOwner: string | null
  governanceStatus: GovernanceStatus
  status: IntegrationStatus
}

/**
 * Runtime 传给能力的只读句柄。
 * 注意：这里**不是**状态容器 —— Runtime 不得注入或持有业务状态。
 */
export interface CapabilityContext {
  readonly capabilityId: string
  readonly state: CapabilityState
  log(message: string): void
}
