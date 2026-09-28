import type { InjectionKey } from "vue";

/**
 * Vault Host Port — 最小窄契约（Frontend M2 Pilot）。
 *
 * 设计约束（用户裁决 A · MINIMAL PORTS & ADAPTERS）：
 *   - 禁止 IoC 容器 / Service Locator / 全局可变注册表 / 事件总线 / 万能 adapter / 字符串键查找 / 第二真源。
 *   - Vault package 只依赖显式 Host Contract，绝不直接 import：
 *       src/bridge.ts · src/stores/useWorkbenchStore · src/utils/graphUi · src/capability/*
 *   - 三个 port 各自对应一项真实外部能力，不合并成 God contract。
 */

/** 原生能力：打开一个 Obsidian Vault 目录并读取其 Markdown 快照。 */
export interface VaultNativePort {
  openVault(path: string): Promise<VaultOpenResult>;
}

export interface VaultOpenResult {
  root: string;
  notes: { path: string; text: string }[];
  skipped: number;
  truncated: boolean;
}

/** Shell 表现层状态：工作区是否折叠（仅影响 Vault 面板布局，不进领域层）。 */
export interface VaultShellPort {
  isWorkbenchCollapsed(): boolean;
}

/** 图布局投影：把 Vault 局部笔记图节点/边投影为坐标（算法由 Host 提供，Vault 不复制 graphUi）。 */
export interface VaultGraphLayoutPort {
  layoutGraph(nodes: GraphNodeLike[], edges: GraphEdgeLike[]): GraphPointLike[];
}

export interface GraphNodeLike {
  id: string;
  kind?: string;
  label?: string;
}
export interface GraphEdgeLike {
  from: string;
  to: string;
}
export interface GraphPointLike {
  id: string;
  x: number;
  y: number;
}

/** Vault 所需的全部 Host 契约（由 Host 在显式工厂中提供）。 */
export interface VaultPorts {
  native: VaultNativePort;
  shell: VaultShellPort;
  graphLayout: VaultGraphLayoutPort;
}

/** 注入键（符号，非字符串键；类型安全；由 Host 经 app.provide 提供）。 */
export const VAULT_PORTS_KEY: InjectionKey<VaultPorts> = Symbol("vaultPorts");

/** 缺 port 时统一 fail-fast，绝不静默 fallback。 */
export function assertVaultPorts(ports: VaultPorts | null | undefined): asserts ports is VaultPorts {
  if (!ports) {
    throw new Error("[vault] VaultPorts 未提供 —— M2 边界违反：Host 必须显式提供 VaultHostPort");
  }
  if (!ports.native || typeof ports.native.openVault !== "function") {
    throw new Error("[vault] VaultPorts.native.openVault 缺失或非函数");
  }
  if (!ports.shell || typeof ports.shell.isWorkbenchCollapsed !== "function") {
    throw new Error("[vault] VaultPorts.shell.isWorkbenchCollapsed 缺失或非函数");
  }
  if (!ports.graphLayout || typeof ports.graphLayout.layoutGraph !== "function") {
    throw new Error("[vault] VaultPorts.graphLayout.layoutGraph 缺失或非函数");
  }
}
