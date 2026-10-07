// Clipboard 能力包本地契约类型（M2 隔离）。
//
// 设计决定（RULE_OF_TWO 复判 @ 2026-09-28）：Host 的 src/capability/types.ts
// 定义的是更严格的治理契约（category: CapabilityCategory、persistence.scope
// 不含 "none"、class 为 ResourceClass[] 字面量联合），而 Vault 已采用刻意放宽的
// 本地投影（category: string、scope 含 "none"、class: string[]）。二者结构相似
// 但语义不等价。强制统一会破坏 Vault 或削弱 Host 治理类型，故本包沿用 Vault 的
// 本地契约投影，不抽 shared contracts。各包本地维持自己的 CapabilityDefinition
// 投影（类型是接口，不是数据真源；SECOND_TRUTH=0 不受影响）。

export type CapabilityState =
  | "DEFINED"
  | "READY"
  | "ACTIVE"
  | "BACKGROUND"
  | "SUSPENDED"
  | "DESTROYED"
  | "COMPATIBILITY_WRAPPED";

export type IntegrationStatus =
  | "NOT_INTEGRATED"
  | "COMPATIBILITY_WRAPPED"
  | "TARGET_COMPOSABLE";

export type GovernanceStatus = "GOVERNED" | "OWNER_PENDING_SCR" | "LOCKED";

export interface CapabilityLifecycle {
  supported: CapabilityState[];
  default: CapabilityState;
  activatable: boolean;
  resident: boolean;
  onActivate?: () => void | Promise<void>;
  onSuspend?: () => void | Promise<void>;
  onDeactivate?: () => void | Promise<void>;
}

export interface CapabilityResourcePolicy {
  class: string[];
  suspendable: boolean;
  destroyable: boolean;
}

export interface CapabilityPersistence {
  scope: "none" | "session" | "disk" | "os_keyring" | "runtime_only";
  sensitive: boolean;
}

export interface CapabilityManifestV1 {
  id: string;
  version: string;
  displayName: string;
  description: string;
  maturity: string;
  maturityEvidence: string[];
  dependencies: string[];
  optionalDependencies: string[];
  conflicts: string[];
  provides: string[];
  requires: string[];
  contributions: { id: string; slot: string; type: string; view: string }[];
  permissions: string[];
  resources: { kind: string; ownership: string; evidence: string }[];
  persistenceScope: string;
  persistenceSensitive: boolean;
  activationPolicy: Record<string, unknown>;
  deactivationPolicy: Record<string, unknown>;
  installPolicy: Record<string, unknown>;
  uninstallPolicy: Record<string, unknown>;
  hotPlug: Record<string, unknown>;
  publicContract: { name: string; locator: string }[];
  entrypoint: string;
  semanticOwner: string;
}

export interface CapabilityDefinition {
  id: string;
  name: string;
  category: string;
  provides: string[];
  dependsOn: string[];
  optionalDependencies: string[];
  lifecycle: CapabilityLifecycle;
  resources: CapabilityResourcePolicy;
  permissions: string[];
  persistence: CapabilityPersistence;
  entrypoint: string;
  semanticOwner: string | null;
  governanceStatus: GovernanceStatus;
  status: CapabilityState;
  v1?: CapabilityManifestV1;
}

// Clipboard 贡献描述子（Host 单点注册，包不自注册到 Host 单例）。
export interface ClipboardContribution {
  id: string;
  capabilityId: string;
  type: "surface" | "navigation";
  slot: string;
  view: string;
  label: string;
  icon: string;
  component: unknown;
}

// Clipboard 仅使用 workbench-main 槽位；本地声明，避免反向依赖 Host 的 CONTRIBUTION_SLOTS。
export const CB_CONTRIBUTION_SLOTS = {
  WORKBENCH_MAIN: "workbench-main",
} as const;
