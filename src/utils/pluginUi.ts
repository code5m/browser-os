// M5-W14 插件管理器 UI 纯逻辑层（headless 可测，不依赖 vue / bridge / Tauri）。
//
// 红线（承 W14 Hard Stops 与 types.ts 顶部注释）：
//  - 本文件只做 UI 状态机/脱敏投影/manifest 轻量预检，**不**调用任何后端命令。
//  - 绝不包含、回显、持久化：签名原文（signature.value）、资源绝对路径、
//    公钥原文、manifest metadata 正文、凭据、请求/响应体、stdout/stderr。
//  - 状态机与后端 `plugin.rs::can_transition` 1:1 镜像；后端是真源，本层只用于 UI 门控。

import type {
  PluginState,
  PluginSummary,
  PluginDetail,
  PluginCapabilityView,
  PluginSignatureView,
  TrustedKeyRecord,
  PluginManifest,
} from "../types";

// ---------------------------------------------------------------------------
// 状态机（镜像后端 plugin.rs can_transition）
// ---------------------------------------------------------------------------

/** 8 态全集（与 domain.rs `PluginState` 顺序一致，供筛选下拉/标签使用）。 */
export const PLUGIN_STATES: PluginState[] = [
  "discovered",
  "validating",
  "signed_ok",
  "signed_failed",
  "loaded",
  "enabled",
  "disabled",
  "uninstalled",
];

/** 合法迁移边（与后端 `can_transition` 完全一致）。 */
const TRANSITIONS: ReadonlyArray<readonly [PluginState, PluginState]> = [
  ["discovered", "validating"],
  ["validating", "signed_ok"],
  ["validating", "signed_failed"],
  ["signed_ok", "loaded"],
  ["signed_failed", "uninstalled"],
  ["signed_failed", "discovered"],
  ["loaded", "enabled"],
  ["loaded", "disabled"],
  ["loaded", "uninstalled"],
  ["enabled", "disabled"],
  ["enabled", "uninstalled"],
  ["disabled", "enabled"],
  ["disabled", "uninstalled"],
];

/** UI 侧状态机门控：与后端 `can_transition` 镜像。后端为真源，UI 仅据此灰显按钮。 */
export function canTransitionUi(from: PluginState, to: PluginState): boolean {
  return TRANSITIONS.some(([a, b]) => a === from && b === to);
}

/** 给定状态下，三个状态变更按钮是否可用（按 `can_transition` 推导）。 */
export interface PluginActionGate {
  enable: boolean;
  disable: boolean;
  uninstall: boolean;
}

export function enabledActionsFor(state: PluginState): PluginActionGate {
  return {
    enable: canTransitionUi(state, "enabled"),
    disable: canTransitionUi(state, "disabled"),
    uninstall: canTransitionUi(state, "uninstalled"),
  };
}

/** 状态中文标签（清晰生命周期状态；不得暗示运行时执行）。 */
const STATE_LABELS: Record<PluginState, string> = {
  discovered: "已发现",
  validating: "校验中",
  signed_ok: "签名通过",
  signed_failed: "签名失败",
  loaded: "已加载",
  enabled: "已启用",
  disabled: "已禁用",
  uninstalled: "已卸载",
};

export function pluginStateLabel(s: PluginState): string {
  return STATE_LABELS[s] ?? s;
}

// ---------------------------------------------------------------------------
// 能力风险档
// ---------------------------------------------------------------------------

export type AclLevel = "safe" | "confirm" | "dangerous";

const ACL_LABELS: Record<AclLevel, string> = {
  safe: "安全",
  confirm: "需确认",
  dangerous: "危险（已阻止）",
};

export function aclLevelLabel(l: AclLevel): string {
  return ACL_LABELS[l] ?? l;
}

/** 危险档（或任意能力，因后端白名单首期空集合 fail-closed → 安装即被拒）。 */
export function isDangerousGate(l: AclLevel): boolean {
  return l === "dangerous";
}

/** 风险档对应的 CSS class（UI 着色用）。 */
export function aclLevelClass(l: AclLevel): "ok" | "warn" | "danger" {
  if (l === "dangerous") return "danger";
  if (l === "confirm") return "warn";
  return "ok";
}

// ---------------------------------------------------------------------------
// 脱敏投影（确保 UI 永不渲染签名原文 / 资源路径 / 公钥原文 / metadata）
// ---------------------------------------------------------------------------

/** 详情脱敏投影：显式挑选字段，丢弃 `signature.value` / 任何路径 / `metadata`。 */
export interface PluginDisplayDetail {
  id: string;
  version: string;
  display_name: string;
  description: string;
  min_app_version: string;
  state: PluginState;
  /** 能力视图（含 acl_level）；UI 须逐项展示且禁折叠。 */
  capabilities: PluginCapabilityView[];
  /** 仅 sha256 前缀。 */
  hash_prefix: string;
  /** 签名视图：**无** `value` 原文。 */
  signature: PluginSignatureView;
  installed_at: string;
  updated_at: string;
  /** 资源包元数据：仅布尔 + 声明摘要，**无**绝对路径。 */
  resource: { declared_hash: string; path_provided: boolean; verified: boolean };
}

/** 把后端 `PluginDetail` 投影为只含脱敏字段的展示对象（丢弃一切敏感原文）。 */
export function redactDetail(d: PluginDetail): PluginDisplayDetail {
  return {
    id: d.id,
    version: d.version,
    display_name: d.display_name,
    description: d.description,
    min_app_version: d.min_app_version,
    state: d.state,
    capabilities: d.capabilities, // PluginCapabilityView 本身即脱敏（含 acl_level，无正文）
    hash_prefix: d.hash_prefix,
    signature: { algorithm: d.signature.algorithm, key_id: d.signature.key_id, status: d.signature.status },
    installed_at: d.installed_at,
    updated_at: d.updated_at,
    resource: {
      declared_hash: d.resource.declared_hash,
      path_provided: d.resource.path_provided,
      verified: d.resource.verified,
    },
  };
}

/** `PluginSummary` 已是脱敏视图（无签名/无路径），原样透传。 */
export function redactSummary(s: PluginSummary): PluginSummary {
  return s;
}

/** 受信任密钥：仅指纹，**无**公钥原文（TrustedKeyRecord 类型本身即如此）。 */
export function redactKeyRecord(k: TrustedKeyRecord): TrustedKeyRecord {
  return { key_id: k.key_id, fingerprint: k.fingerprint, note: k.note, added_at: k.added_at };
}

// ---------------------------------------------------------------------------
// manifest 轻量预检（仅判断"能否构造给后端"，不替代后端校验）
// ---------------------------------------------------------------------------

export interface ManifestParseResult {
  ok: boolean;
  manifest?: PluginManifest;
  error?: string;
}

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/**
 * 解析用户粘贴的 manifest 文本（JSON）。仅做前端轻量结构预检：
 * 必填字段存在 + 形态③ entry 为对象。不做后端级语义校验（那个归 plugin.rs）。
 * 返回的 `manifest` 中可能含 `signature.value`（原始 base64）——它是**瞬时**传给
 * 后端的入参，本层/store/面板都**不持久化、不回显**；安装成功后以 `plugin_get`
 * 的脱敏 `PluginDetail` 覆盖展示。
 */
export function parseManifestInput(text: string): ManifestParseResult {
  const t = (text ?? "").trim();
  if (!t) return { ok: false, error: "manifest 内容为空" };
  let raw: unknown;
  try {
    raw = JSON.parse(t);
  } catch {
    return { ok: false, error: "manifest 不是合法 JSON" };
  }
  if (!isObj(raw)) return { ok: false, error: "manifest 必须是 JSON 对象" };
  for (const f of ["id", "version", "display_name", "description", "min_app_version", "hash"] as const) {
    if (typeof raw[f] !== "string" || (raw[f] as string).length === 0) {
      return { ok: false, error: `manifest 缺少必填字符串字段：${f}` };
    }
  }
  if (!isObj(raw.entry) || typeof (raw.entry as Record<string, unknown>).entry_url !== "string") {
    return { ok: false, error: "manifest.entry.entry_url 缺失或非字符串" };
  }
  if (!Array.isArray(raw.capabilities)) {
    return { ok: false, error: "manifest.capabilities 必须是数组" };
  }
  if (!isObj(raw.signature) || typeof (raw.signature as Record<string, unknown>).algorithm !== "string") {
    return { ok: false, error: "manifest.signature 缺失或非对象" };
  }
  // 结构通过；按后端预期形状回传（metadata 可选）。
  const manifest = raw as unknown as PluginManifest;
  return { ok: true, manifest };
}
