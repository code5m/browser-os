/**
 * Vault 能力契约类型（包内声明，即边界本身；非第二真源的“逻辑/常量”，仅为契约形状）。
 *
 * 这些类型只在包边界处与 Host 的 CapabilityDefinition / Contribution 做结构化对齐，
 * 不依赖 Host 内部类型文件（满足 PKG-06 不 import src/capability/*）。
 */

export type CapabilityState =
  | "DEFINED"
  | "READY"
  | "ACTIVE"
  | "BACKGROUND"
  | "SUSPENDED"
  | "DESTROYED";

export type IntegrationStatus = "NOT_INTEGRATED" | "COMPATIBILITY_WRAPPED" | "TARGET_COMPOSABLE";

export type GovernanceStatus = "GOVERNED" | "OWNER_PENDING_SCR" | "LOCKED";

export interface CapabilityLifecycle {
  supported: CapabilityState[];
  default: CapabilityState;
  activatable: boolean;
  resident: boolean;
  onActivate?: () => void | Promise<void>;
  onSuspend?: () => void | Promise<void>;
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
  activationPolicy: string;
  deactivationPolicy: string;
  installPolicy: string;
  uninstallPolicy: string;
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
  status: IntegrationStatus;
  v1?: CapabilityManifestV1;
}

/** Vault 贡献描述符（包内声明，Host 负责注册进 contributionRegistry）。 */
export interface VaultContribution {
  id: string;
  capabilityId: string;
  type: "surface" | "navigation";
  slot: string;
  view: string;
  label: string;
  icon: string;
  component: unknown;
}
