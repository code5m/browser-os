// src/utils/agentSkillUi.ts
// M5-6 Agent/Skill UI **纯逻辑层**：无 invoke、无 DOM、无 store 依赖，可被 node 直接 import 断言
// （见 scripts/check-agent-skill-ui-logic.mjs）。
//
// 契约源：
//   - src-tauri/src/domain.rs（Lane A5，W4）：AgentDef / SkillDef / SkillExec / AclLevel /
//     SkillInput / CapabilityRef / PermissionPreview / A2aConfig / AgentDialect
//   - A6 W4 数据契约：logs/assist/A6-M5-W4-20260906-1810.md
//
// 本层只做**前端前置校验与展示**，后端仍是唯一裁决者（同样的规则后端会再判一次）。
// 边界（board M5-W5 Hard Stop）：不执行 Skill、不安装插件、不调用实时运行时；
// 所有 secret 值不落盘、不进 store、不进审计。

import type {
  AclLevel,
  AgentDef,
  AgentDialect,
  A2aConfig,
  CapabilityRef,
  PendingConfirm,
  PermissionPreview,
  RunStatus,
  SkillDef,
  SkillExec,
  SkillInput,
  StreamChunk,
} from "../types";
import { runStatusClass, runStatusLabel } from "./scriptUi";
import { formatRelative } from "./taskUi";

// ====== 常量（与后端冻结值对齐，前端不得自行放宽） ======

/// 安装/运行闸门三档的中文档位与配色。
export const ACL_LABELS: Readonly<Record<AclLevel, string>> = {
  safe: "安全",
  confirm: "需确认",
  dangerous: "高危",
};

/// 配色 class（供 .tag.safe/.warn/.danger 复用）。
export const ACL_TONE: Readonly<Record<AclLevel, "safe" | "warn" | "danger">> = {
  safe: "safe",
  confirm: "warn",
  dangerous: "danger",
};

export const DIALECT_LABELS: Readonly<Record<AgentDialect, string>> = {
  open_ai_compatible: "OpenAI 兼容",
  external_cli: "外部 CLI",
  custom: "自定义",
};

/// 能力 id 的「尽力」中文释义。单一真源在后端 security_policy.rs 的
/// SKILL_CAPABILITY_V1 / AGENT_CAPABILITY_V1（G4，W4 尚未落齐）；此处仅作展示兜底，
/// 未知能力一律回显原 id，绝不伪造授权语义。
const CAPABILITY_LABELS: Readonly<Record<string, string>> = {
  "fs:read": "文件系统读",
  "fs:write": "文件系统写",
  "net:http": "网络请求",
  "skill:invoke": "调用其它 Skill",
  "agent:delegate": "委派给其它 Agent",
  "agent:delegated_from": "允许被委派",
};

export const MAX_SKILLS = 200; // 与脚本/命令片段上限对齐（A5 落码时定，前端仅展示上限）
export const MAX_AGENTS = 64;
export const MAX_PARAMS = 32; // 单个 Skill 入参条目上限（前端前置校验）
export const MAX_STREAM_BUFFER_BYTES = 65536; // 单会话 UI 流式缓冲上限（W4 有界约束）

// ====== 类型 ======

export interface SkillIssue {
  field: string;
  code: string;
  message: string;
}

export interface CapabilityRow {
  id: string;
  label: string;
  /// 是否在后端白名单内（白名单为空时一律 unknown，因为单一真源未就绪）。
  granted: boolean;
  unknown: boolean;
}

export interface PermissionPreviewView {
  gate: AclLevel;
  gateLabel: string;
  gateTone: "safe" | "warn" | "danger";
  /// 安装/运行将执行的目标（脚本/命令引用或串联），供用户二次确认时看清。
  targetSummary: string;
  capabilities: CapabilityRow[];
  /// 高危提示：Dangerous 必显、Confirm 建议显。
  dangerHint: string | null;
}

// ====== 展示用工具 ======

export function aclLabel(acl: AclLevel): string {
  return ACL_LABELS[acl] ?? String(acl);
}

export function aclTone(acl: AclLevel): "safe" | "warn" | "danger" {
  return ACL_TONE[acl] ?? "warn";
}

export function dialectLabel(d: AgentDialect): string {
  return DIALECT_LABELS[d] ?? String(d);
}

export function a2aSummary(a2a: A2aConfig | undefined): string {
  if (!a2a) return "不可委派";
  const parts: string[] = [];
  if (a2a.delegate_to) parts.push("可委派他人");
  if (a2a.delegated_from) parts.push("可被委派");
  return parts.length ? parts.join(" / ") : "不可委派";
}

/// 把 SkillExec 渲染成一行人类可读的目标摘要（不展开参数值，避免 secret 泄露）。
export function execSummary(exec: SkillExec | undefined): string {
  if (!exec) return "—";
  switch (exec.kind) {
    case "script_ref":
      return `脚本 #${exec.script_id}`;
    case "command_ref":
      return `命令片段 #${exec.command_id}`;
    case "sequence": {
      const n = Array.isArray(exec.steps) ? exec.steps.length : 0;
      return `串联 ${n} 步`;
    }
    default:
      return "—";
  }
}

export function capabilityLabel(id: string): string {
  return CAPABILITY_LABELS[id] ?? id;
}

/// 能力列表渲染：对照后端白名单（whitelist）标出 granted / unknown。
/// whitelist 为空数组表示单一真源尚未就绪，此时全部 unknown（不谎报已授权）。
export function renderCapabilityList(
  caps: (CapabilityRef | string)[] | undefined,
  whitelist: string[] = [],
): CapabilityRow[] {
  const list = caps ?? [];
  const wl = new Set(whitelist);
  return list.map((c) => {
    const id = typeof c === "string" ? c : c.id;
    const granted = wl.has(id);
    return {
      id,
      label: capabilityLabel(id),
      granted,
      unknown: !granted,
    };
  });
}

/// 后端 PermissionPreview 的展示态（安装/运行前用户可见闸门档与所需能力）。
export function buildPermissionPreview(
  def: SkillDef | AgentDef,
  whitelist: string[] = [],
): PermissionPreviewView {
  const gate: AclLevel = (def as SkillDef).acl ?? "safe";
  const caps = (def.capabilities ?? []) as CapabilityRef[];
  const target = "exec" in def ? execSummary((def as SkillDef).exec) : a2aSummary((def as AgentDef).a2a);
  let dangerHint: string | null = null;
  if (gate === "dangerous") {
    dangerHint = "高危操作：需二次确认与凭据复核，安装后具备所列能力。";
  } else if (gate === "confirm") {
    dangerHint = "该项的执行需你手动确认一次。";
  }
  return {
    gate,
    gateLabel: aclLabel(gate),
    gateTone: aclTone(gate),
    targetSummary: target,
    capabilities: renderCapabilityList(caps, whitelist),
    dangerHint,
  };
}

/// 友好的能力判定（供 PermissionPreviewModal 直接渲染）。
export function classifyCapability(
  id: string,
  whitelist: string[] = [],
): CapabilityRow {
  const granted = whitelist.includes(id);
  return { id, label: capabilityLabel(id), granted, unknown: !granted };
}

// ====== 运行态展示 ======

/// 复用 M2-4 / M4-8 的运行状态标签与配色（scriptUi 为单一真源）。
export function runStatusUi(status: RunStatus): { label: string; cls: string } {
  return { label: runStatusLabel(status), cls: runStatusClass(status) };
}

export function isTerminalStatus(status: RunStatus): boolean {
  return status === "succeeded" || status === "failed" || status === "cancelled" || status === "timeout";
}

/// 运行记录相对时间摘要（复用 taskUi.formatRelative，统一口径）。
export function runRelative(iso: string | undefined, nowMs: number): string {
  return formatRelative(iso ?? null, nowMs);
}

// ====== 表单态校验（前端前置） ======

/// Skill 入参表单校验：必填项缺失即报错；secret 值不在此层落盘（由 serializeSkillForm 脱敏）。
export function validateSkillForm(
  def: SkillDef | undefined,
  values: Record<string, string>,
): SkillIssue[] {
  const issues: SkillIssue[] = [];
  if (!def) {
    issues.push({ field: "def", code: "SKILL_DEF_MISSING", message: "Skill 定义缺失" });
    return issues;
  }
  const inputs: SkillInput[] = def.inputs ?? [];
  if (inputs.length > MAX_PARAMS) {
    issues.push({ field: "inputs", code: "SKILL_PARAMS_OVERFLOW", message: `入参条目不得超过 ${MAX_PARAMS}` });
  }
  const names = new Set(inputs.map((i) => i.name));
  for (const input of inputs) {
    if (input.required && (values[input.name] === undefined || String(values[input.name]).length === 0)) {
      issues.push({
        field: input.name,
        code: "SKILL_PARAM_REQUIRED",
        message: `必填入参「${input.name}」未填`,
      });
    }
  }
  for (const key of Object.keys(values)) {
    if (!names.has(key)) {
      issues.push({ field: key, code: "SKILL_PARAM_UNKNOWN", message: `未知入参「${key}」` });
    }
  }
  return issues;
}

// ====== secret 脱敏（W4 / W5 边界：token/cookie/Authorization/body/密钥不持久化） ======

const SECRET_KEY_RE = /(pass|password|pwd|token|secret|api[_-]?key|authorization|cookie|private[_-]?key)/i;

function isSecretKey(key: string): boolean {
  return SECRET_KEY_RE.test(key);
}

/// 把任意参数对象里的 secret 键的值替换为 "***"；返回新对象，不修改入参。
/// 用于把 Skill 入参 / exec.params 交给后端前的本地脱敏（即便传输层已安全，前端日志/状态也不应持有明文）。
export function redactSecrets<T extends Record<string, unknown>>(params: T): T {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(params)) {
    if (isSecretKey(k)) {
      out[k] = "***";
    } else if (v && typeof v === "object" && !Array.isArray(v)) {
      out[k] = redactSecrets(v as Record<string, unknown>);
    } else {
      out[k] = v;
    }
  }
  return out as T;
}

/// 序列化 Skill 表单提交：拷贝 + secret 脱敏。绝不把明文密码带出本函数。
export function serializeSkillForm(
  inputs: SkillInput[] | undefined,
  values: Record<string, string>,
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const input of inputs ?? []) {
    const v = values[input.name];
    if (v === undefined) continue;
    out[input.name] = isSecretKey(input.name) ? "***" : v;
  }
  return out;
}

// ====== 面板三态（空 / 错误 / 加载） ======

export interface PanelStateView {
  state: "loading" | "empty" | "ready" | "error";
  message: string;
}

/// 统一生成面板占位文案；backendReady=false 时强调「只读壳」。
export function panelState(
  kind: "skills" | "agents",
  opts: { loading: boolean; count: number; error: string | null; backendReady: boolean },
): PanelStateView {
  if (opts.error) {
    return { state: "error", message: opts.error };
  }
  if (opts.loading) {
    return { state: "loading", message: "加载中…" };
  }
  if (opts.count === 0) {
    const name = kind === "skills" ? "技能" : "Agent";
    return {
      state: "empty",
      message: opts.backendReady
        ? `暂无已安装的${name}。点击「刷新」或安装一个新的。`
        : `后端 ${kind === "skills" ? "skill" : "agent"} 命令尚未就绪，当前为只读壳。`,
    };
  }
  return { state: "ready", message: "" };
}

// ====== 流式缓冲（有界） ======

/// 把一条 StreamChunk 累加到会话缓冲，并保证单会话 UI 缓冲不超 MAX_STREAM_BUFFER_BYTES。
/// 超出时丢弃最旧 data 块并计入 droppedBytes（不抛错、不增长无界）。
export function appendChunk(
  chunks: StreamChunk[],
  chunk: StreamChunk,
  budgetBytes: number = MAX_STREAM_BUFFER_BYTES,
): StreamChunk[] {
  const next = [...chunks, chunk];
  let bytes = 0;
  let dropped = 0;
  const kept: StreamChunk[] = [];
  for (let i = next.length - 1; i >= 0; i -= 1) {
    const c = next[i];
    const size = c.kind === "data" ? (c.data?.length ?? 0) : 0;
    if (bytes + size > budgetBytes && c.kind === "data") {
      dropped += 1;
      continue;
    }
    bytes += size;
    kept.unshift(c);
  }
  if (dropped > 0) {
    kept.push({ kind: "flow", droppedBytes: dropped });
  }
  return kept;
}

/// 把会话内 data 块拼接为可读文本（供 ChatPanel 渲染）。
export function chunksToText(chunks: StreamChunk[] | undefined): string {
  return (chunks ?? [])
    .filter((c): c is Extract<StreamChunk, { kind: "data" }> => c.kind === "data")
    .map((c) => c.data)
    .join("");
}

// ====== 二段式闸门待确认项（辅助） ======

export function makePendingConfirm(
  action: PendingConfirm["action"],
  payload: unknown,
  ttlMs = 5 * 60 * 1000,
): PendingConfirm {
  return { action, payload, expiresAt: Date.now() + ttlMs };
}

export function isConfirmExpired(c: PendingConfirm | undefined): boolean {
  return !c || Date.now() > c.expiresAt;
}
