// src/utils/taskUi.ts
// M4-8 定时任务 UI **纯逻辑层**：无 invoke、无 DOM、无 store 依赖，可被 node 直接 import 断言。
//
// 契约源：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`
//   - §3.1 TaskDef / TaskTrigger / TaskKind / MissedRunPolicy / RetryPolicy / TaskRunRecord
//   - §3.3 定义期校验 R-1~R-7（错误码为稳定字符串）
//   - §3.5 裁定 R-A6-1：新建任务默认 enabled = false
//   - §4.4 cron 方言：仅 5 段（分 时 日 月 周），不支持秒/年/宏
//   - §5.2 跳过原因必须可展示（O-A6-9）
//   - §5.3 重试 delay 必须 clamp 到 max_delay_secs
//
// 本层只做**前端前置校验与展示**，后端仍为唯一裁决者（同样的规则后端会再判一次）。

import type {
  MissedRunPolicy,
  RetryBackoff,
  RetryPolicy,
  RunSnapshot,
  RunStatus,
  ScriptParam,
  TaskAddArgs,
  TaskDef,
  TaskKind,
  TaskRunRecord,
  TaskRunTrigger,
  TaskSkipReason,
  TaskTrigger,
} from "../types";
import { runStatusLabel } from "./scriptUi";
import { formatRelative } from "../shared/pure/time/relative";
export { formatRelative };

// ====== 常量（与后端冻结值对齐，前端不得自行放宽）======

/// A6 §4.4：后端只接受 5 段 cron（分 时 日 月 周）。6 段（含秒）一律拒绝。
export const CRON_FIELD_COUNT = 5;

export const CRON_FIELD_RANGES: ReadonlyArray<{ name: string; min: number; max: number }> = [
  { name: "分", min: 0, max: 59 },
  { name: "时", min: 0, max: 23 },
  { name: "日", min: 1, max: 31 },
  { name: "月", min: 1, max: 12 },
  { name: "周", min: 0, max: 7 }, // 0 与 7 均为周日
];

export const MAX_TASK_NAME_BYTES = 128; // 对齐 scripts::MAX_NAME_BYTES
export const MIN_INTERVAL_SECS = 60; // A6 F-A6-6：无约束秒级周期会造成补跑风暴
export const MAX_INTERVAL_SECS = 2592000; // 30 天
export const DEFAULT_CATCH_UP_LIMIT = 3;
export const MAX_CATCH_UP_LIMIT = 10;
export const DEFAULT_MISFIRE_GRACE_SECS = 60;
export const MAX_ATTEMPTS = 5;
export const MAX_TIMEOUT_SECS = 600;
export const MAX_TASKS = 200; // 对齐 MAX_SCRIPTS / MAX_SNIPPETS
export const MAX_PARAMS = 20;
/// A6 裁定 R-A6-1：定时任务是「无人值守自动执行」，默认必须关闭。
export const DEFAULT_TASK_ENABLED = false;
/// UI 默认重试间隔（后端未冻结默认值，取保守值；可被后端拒绝并回显错误码）。
export const DEFAULT_RETRY_BASE_DELAY_SECS = 30;
export const DEFAULT_RETRY_MAX_DELAY_SECS = 300;

// ====== 类型 ======

export interface TaskTargetOption {
  id: string;
  name: string;
  kind: TaskKind;
  params: ScriptParam[];
  dangerous: boolean;
}

/// 表单态（与 TaskDef 的 snake_case 存储态解耦，序列化时再转换）。
export interface TaskDraft {
  id: string | null; // null = 新建
  name: string;
  kind: TaskKind;
  target_id: string;
  params: Record<string, string>;
  enabled: boolean;
  trigger_kind: "cron" | "interval";
  cron_expr: string;
  interval_secs: number;
  missed_run_policy: MissedRunPolicy;
  catch_up_limit: number;
  misfire_grace_secs: number;
  retry_max_attempts: number;
  retry_backoff: RetryBackoff;
  retry_base_delay_secs: number;
  retry_max_delay_secs: number;
  timeout_secs: number;
}

export interface TaskIssue {
  field: string;
  code: string;
  message: string;
}

export interface TaskValidateContext {
  targets: TaskTargetOption[];
  taskCount: number;
}

// ====== 工具 ======

export function byteLength(s: string): number {
  // TextEncoder 在浏览器与 node 均可用；按 UTF-8 字节计（与后端 MAX_NAME_BYTES 同口径）。
  return new TextEncoder().encode(s ?? "").length;
}

function isIntToken(s: string): boolean {
  return /^\d+$/.test(s);
}

// ====== cron（仅 5 段，A6 §4.4）======

// 校验 5 段 cron。支持「通配」「通配加步长」「范围 a-b」「列表 a,b」「字面量」五种写法；
// 不支持秒/年字段、`@reboot` / `@daily` 等宏、范围步长（如 `1-10/2`）。
// 返回 null 表示合法；否则返回可直接展示的原因（供 UI 直接呈现）。
export function validateCron(expr: string): string | null {
  const raw = (expr ?? "").trim();
  if (raw.length === 0) return "cron 表达式不可为空";
  if (raw.startsWith("@")) return "不支持 @reboot / @daily 等宏，请使用 5 段标准表达式";

  const parts = raw.split(/\s+/);
  if (parts.length !== CRON_FIELD_COUNT) {
    return `cron 必须是 ${CRON_FIELD_COUNT} 段（分 时 日 月 周），实得 ${parts.length} 段；含秒的 6 位表达式不被支持`;
  }

  for (let i = 0; i < CRON_FIELD_COUNT; i += 1) {
    const spec = CRON_FIELD_RANGES[i];
    const field = parts[i];
    const items = field.split(",");
    for (const item of items) {
      const problem = validateCronItem(item, spec.name, spec.min, spec.max);
      if (problem) return problem;
    }
  }
  return null;
}

function validateCronItem(item: string, name: string, min: number, max: number): string | null {
  if (item === "*") return null;
  if (item.startsWith("*/")) {
    const step = item.slice(2);
    if (!isIntToken(step)) return `${name}字段步长必须是正整数（${item}）`;
    const n = Number(step);
    if (n < 1 || n > max) return `${name}字段步长必须在 1~${max}（${item}）`;
    return null;
  }
  if (item.includes("/")) return `${name}字段不支持范围步长写法（${item}），仅支持 */n`;
  if (item.includes("-")) {
    const [a, b] = item.split("-");
    if (!isIntToken(a) || !isIntToken(b)) return `${name}字段范围必须是整数（${item}）`;
    const lo = Number(a);
    const hi = Number(b);
    if (lo < min || hi > max) return `${name}字段范围越界（${item}），合法区间 ${min}~${max}`;
    if (lo > hi) return `${name}字段范围起止颠倒（${item}）`;
    return null;
  }
  if (!isIntToken(item)) return `${name}字段含非法字符（${item}）`;
  const v = Number(item);
  if (v < min || v > max) return `${name}字段越界（${item}），合法区间 ${min}~${max}`;
  return null;
}

/// 展示用简述；无法简化时原样返回表达式。
export function describeCron(expr: string): string {
  if (validateCron(expr) !== null) return "—";
  const [minute, hour, dom, month, dow] = expr.trim().split(/\s+/);
  if (minute.startsWith("*/")) return `每 ${minute.slice(2)} 分钟`;
  if (/^\d+$/.test(minute) && hour === "*" && dom === "*" && month === "*" && dow === "*") {
    return `每小时第 ${minute} 分`;
  }
  if (/^\d+$/.test(minute) && /^\d+$/.test(hour) && dom === "*" && month === "*" && dow === "*") {
    return `每天 ${hour}:${String(minute).padStart(2, "0")}`;
  }
  return expr.trim();
}

export function validateIntervalSecs(secs: number): string | null {
  if (!Number.isFinite(secs) || Math.trunc(secs) !== secs) return "间隔必须是整数秒";
  if (secs < MIN_INTERVAL_SECS) return `间隔下界 ${MIN_INTERVAL_SECS} 秒（秒级周期会造成错过补偿风暴）`;
  if (secs > MAX_INTERVAL_SECS) return `间隔上界 ${MAX_INTERVAL_SECS} 秒（30 天）`;
  return null;
}

export function formatTrigger(trigger: TaskTrigger | null | undefined): string {
  if (!trigger) return "—";
  if ("cron" in trigger) {
    const expr = trigger.cron?.expr ?? "";
    return describeCron(expr) === "—" ? expr || "—" : `${describeCron(expr)}（${expr}）`;
  }
  if ("interval" in trigger) {
    const s = Number(trigger.interval?.every_secs ?? 0);
    if (s >= 3600 && s % 3600 === 0) return `每 ${s / 3600} 小时`;
    if (s >= 60 && s % 60 === 0) return `每 ${s / 60} 分钟`;
    return `每 ${s} 秒`;
  }
  return "—";
}

// ====== 表单态 ⇄ 存储态 ======

export function emptyTaskDraft(): TaskDraft {
  return {
    id: null,
    name: "",
    kind: "script",
    target_id: "",
    params: {},
    // A6 裁定 R-A6-1：默认关闭；创建即静默启用属违约。
    enabled: DEFAULT_TASK_ENABLED,
    trigger_kind: "cron",
    cron_expr: "0 * * * *",
    interval_secs: MIN_INTERVAL_SECS,
    missed_run_policy: "skip",
    catch_up_limit: DEFAULT_CATCH_UP_LIMIT,
    misfire_grace_secs: DEFAULT_MISFIRE_GRACE_SECS,
    retry_max_attempts: 1,
    retry_backoff: "fixed",
    retry_base_delay_secs: DEFAULT_RETRY_BASE_DELAY_SECS,
    retry_max_delay_secs: DEFAULT_RETRY_MAX_DELAY_SECS,
    timeout_secs: 0,
  };
}

export function loadTaskDraft(def: TaskDef): TaskDraft {
  const draft = emptyTaskDraft();
  draft.id = def.id;
  draft.name = def.name ?? "";
  draft.kind = def.kind;
  draft.target_id = def.target_id ?? "";
  draft.params = { ...(def.params ?? {}) };
  draft.enabled = Boolean(def.enabled);
  draft.missed_run_policy = def.missed_run_policy ?? "skip";
  draft.catch_up_limit = def.catch_up_limit ?? DEFAULT_CATCH_UP_LIMIT;
  draft.misfire_grace_secs = def.misfire_grace_secs ?? DEFAULT_MISFIRE_GRACE_SECS;
  draft.retry_max_attempts = def.retry?.max_attempts ?? 1;
  draft.retry_backoff = def.retry?.backoff ?? "fixed";
  draft.retry_base_delay_secs = def.retry?.base_delay_secs ?? DEFAULT_RETRY_BASE_DELAY_SECS;
  draft.retry_max_delay_secs = def.retry?.max_delay_secs ?? DEFAULT_RETRY_MAX_DELAY_SECS;
  draft.timeout_secs = def.timeout_secs ?? 0;

  const trigger = def.trigger as TaskTrigger | undefined;
  if (trigger && "cron" in trigger) {
    draft.trigger_kind = "cron";
    draft.cron_expr = trigger.cron?.expr ?? "";
  } else if (trigger && "interval" in trigger) {
    draft.trigger_kind = "interval";
    draft.interval_secs = Number(trigger.interval?.every_secs ?? MIN_INTERVAL_SECS);
  }
  return draft;
}

export function triggerFromDraft(draft: TaskDraft): TaskTrigger {
  if (draft.trigger_kind === "interval") {
    return { interval: { every_secs: Math.trunc(draft.interval_secs) } };
  }
  return { cron: { expr: draft.cron_expr.trim() } };
}

export function retryPolicyFromDraft(draft: TaskDraft): RetryPolicy {
  return {
    max_attempts: Math.trunc(draft.retry_max_attempts),
    backoff: draft.retry_backoff,
    base_delay_secs: Math.trunc(draft.retry_base_delay_secs),
    max_delay_secs: Math.trunc(draft.retry_max_delay_secs),
  };
}

/// 提交给 `task_add` / `task_update` 的载荷（id 与时间戳由服务端生成/维护）。
export function serializeTaskDraft(draft: TaskDraft): Omit<
  TaskDef,
  "id" | "created_at" | "updated_at" | "last_fired_at" | "next_run_at"
> {
  return {
    name: draft.name.trim(),
    kind: draft.kind,
    target_id: draft.target_id,
    params: { ...draft.params },
    enabled: draft.enabled,
    trigger: triggerFromDraft(draft),
    missed_run_policy: draft.missed_run_policy,
    catch_up_limit: Math.trunc(draft.catch_up_limit),
    misfire_grace_secs: Math.trunc(draft.misfire_grace_secs),
    retry: retryPolicyFromDraft(draft),
    timeout_secs: Math.trunc(draft.timeout_secs),
  };
}

// ====== 提交给 task_add 的平铺入参（camelCase）======
// 注意：与 `serializeTaskDraft`（task_update 用、snake_case 结构体）不同，
// `task_add` 是后端**平铺命名参数**，且 Tauri 默认把参数名从 snake_case 转 camelCase，
// 多词键必须写成 camelCase（targetId / catchUpLimit / misfireGraceSecs / timeoutSecs）。
export function serializeTaskAddArgs(draft: TaskDraft): TaskAddArgs {
  return {
    name: draft.name.trim(),
    kind: draft.kind,
    targetId: draft.target_id,
    trigger: triggerFromDraft(draft),
    params: { ...draft.params },
    missedRunPolicy: draft.missed_run_policy,
    catchUpLimit: Math.trunc(draft.catch_up_limit),
    misfireGraceSecs: Math.trunc(draft.misfire_grace_secs),
    retry: retryPolicyFromDraft(draft),
    timeoutSecs: Math.trunc(draft.timeout_secs),
    enabled: draft.enabled,
  };
}

// ====== 校验（前端前置，错误码沿用 A6 §3.3 稳定码）======

export function validateTaskDraft(draft: TaskDraft, ctx: TaskValidateContext): TaskIssue[] {
  const issues: TaskIssue[] = [];
  const push = (field: string, code: string, message: string) => issues.push({ field, code, message });

  // R-1 名称
  if (draft.name.trim().length === 0) {
    push("name", "TASK_PARAM_INVALID", "任务名不可为空");
  } else if (byteLength(draft.name.trim()) > MAX_TASK_NAME_BYTES) {
    push("name", "TASK_NAME_TOO_LONG", `任务名不得超过 ${MAX_TASK_NAME_BYTES} 字节（当前 ${byteLength(draft.name.trim())}）`);
  }

  // R-1 目标存在性（含类型匹配）
  const target = ctx.targets.find((t) => t.id === draft.target_id && t.kind === draft.kind);
  if (!draft.target_id) {
    push("target_id", "TASK_TARGET_NOT_FOUND", "必须选择执行目标");
  } else if (!target) {
    push("target_id", "TASK_TARGET_NOT_FOUND", "目标不存在或类型不匹配");
  }

  // R-2 触发表达式
  if (draft.trigger_kind === "cron") {
    const problem = validateCron(draft.cron_expr);
    if (problem) push("cron_expr", "TASK_INVALID_CRON", problem);
  } else {
    const problem = validateIntervalSecs(draft.interval_secs);
    if (problem) push("interval_secs", "TASK_INTERVAL_OUT_OF_RANGE", problem);
  }

  // R-3 / R-4 参数：secret 参数值不落盘；键集必须与目标参数一致
  if (target) {
    const names = new Set(target.params.map((p) => p.name));
    for (const p of target.params) {
      if (p.secret && Object.prototype.hasOwnProperty.call(draft.params, p.name)) {
        push("params", "TASK_SECRET_PARAM_FORBIDDEN", `敏感参数「${p.name}」的值不得保存在任务里（定时任务无人值守，凭据只走 Keyring）`);
      }
      if (p.required && !p.secret) {
        const v = draft.params[p.name];
        if (v === undefined || String(v).length === 0) {
          push("params", "TASK_PARAM_INVALID", `必填参数「${p.name}」未填`);
        }
      }
    }
    for (const key of Object.keys(draft.params)) {
      if (!names.has(key)) push("params", "TASK_PARAM_INVALID", `未知参数「${key}」（目标不存在该参数）`);
    }
    if (Object.keys(draft.params).length > MAX_PARAMS) {
      push("params", "TASK_PARAM_INVALID", `参数条目不得超过 ${MAX_PARAMS}`);
    }
  }

  // R-5 数值边界
  if (draft.catch_up_limit < 1 || draft.catch_up_limit > MAX_CATCH_UP_LIMIT) {
    push("catch_up_limit", "TASK_PARAM_INVALID", `补跑上限必须在 1~${MAX_CATCH_UP_LIMIT}`);
  }
  if (draft.timeout_secs < 0 || draft.timeout_secs > MAX_TIMEOUT_SECS) {
    push("timeout_secs", "TASK_PARAM_INVALID", `超时必须在 0~${MAX_TIMEOUT_SECS}（0 = 后端默认 60）`);
  }
  if (draft.retry_max_attempts < 1 || draft.retry_max_attempts > MAX_ATTEMPTS) {
    push("retry_max_attempts", "TASK_PARAM_INVALID", `重试次数必须在 1~${MAX_ATTEMPTS}（1 = 不重试）`);
  }
  if (draft.retry_max_delay_secs < 0 || draft.retry_max_delay_secs > MAX_TIMEOUT_SECS) {
    push("retry_max_delay_secs", "TASK_PARAM_INVALID", `重试最大间隔不得超过 ${MAX_TIMEOUT_SECS} 秒`);
  }
  if (draft.misfire_grace_secs < 0 || draft.misfire_grace_secs > MAX_INTERVAL_SECS) {
    push("misfire_grace_secs", "TASK_PARAM_INVALID", "迟到宽限秒数不合法");
  }

  // R-6 容量（仅新建时判定）
  if (!draft.id && ctx.taskCount >= MAX_TASKS) {
    push("name", "TASK_LIMIT_REACHED", `任务总数已达上限 ${MAX_TASKS}`);
  }

  return issues;
}

/// A6 §3.2：catch_up_limit 仅在 CatchUp 策略下生效，其余策略 UI 必须置灰。
export function isCatchUpLimitEditable(policy: MissedRunPolicy): boolean {
  return policy === "catch_up";
}

// ====== 展示 ======

export function triggerLabelOf(trigger: TaskRunTrigger): string {
  switch (trigger) {
    case "scheduled":
      return "定时";
    case "manual":
      return "手工";
    case "catch_up":
      return "补跑";
    case "retry":
      return "重试";
    default:
      return String(trigger);
  }
}

/// O-A6-9：跳过原因必须能展示，否则用户只看到「没跑」却不知为何。
export function skipReasonLabel(reason: TaskSkipReason | string | null | undefined): string {
  switch (reason) {
    case "reentrant":
      return "同任务并发，已跳过";
    case "target_busy":
      return "目标正被占用（手工或其它任务在执行）";
    case "global_limit":
      return "已达全局并发上限";
    default:
      return reason ? `已跳过（${reason}）` : "已跳过";
  }
}

export function statusLabel(status: RunStatus): string {
  return runStatusLabel(status);
}

/// A6 §5.3：delay 必须 clamp 到 max_delay_secs（指数退避 base * 2^(n-1)）。
export function retryDelaySeconds(policy: RetryPolicy, attempt: number): number {
  const max = Math.max(0, Math.trunc(policy.max_delay_secs || 0));
  const base = Math.max(0, Math.trunc(policy.base_delay_secs || 0));
  if (attempt <= 1) return 0;
  const n = attempt - 1;
  const raw = policy.backoff === "exponential" ? base * Math.pow(2, n - 1) : base;
  return Math.min(Math.trunc(raw), max);
}

export function retrySummary(policy: RetryPolicy): string {
  const attempts = Math.trunc(policy.max_attempts || 1);
  if (attempts <= 1) return "不重试";
  const kind = policy.backoff === "exponential" ? "指数退避" : "固定间隔";
  const last = retryDelaySeconds(policy, attempts);
  return `最多 ${attempts} 次（${kind}，末次延迟 ${last}s）`;
}

export function missedPolicyLabel(policy: MissedRunPolicy): string {
  switch (policy) {
    case "skip":
      return "跳过（默认）";
    case "run_once":
      return "补跑一次";
    case "catch_up":
      return "按上限补跑";
    default:
      return String(policy);
  }
}

export function kindLabel(kind: TaskKind): string {
  return kind === "script" ? "脚本" : "命令片段";
}

export type NextRunState =
  | { kind: "disabled" }
  | { kind: "none" }
  | { kind: "stale" }
  | { kind: "scheduled"; at: string; label: string };

/// 下次执行展示态：未启用 → disabled；无计划 → none；计划已过期（待后端重算）→ stale。
export function nextRunState(def: TaskDef, nowMs: number): NextRunState {
  if (!def.enabled) return { kind: "disabled" };
  if (!def.next_run_at) return { kind: "none" };
  const t = Date.parse(def.next_run_at);
  if (!Number.isFinite(t)) return { kind: "none" };
  if (t <= nowMs) return { kind: "stale" };
  return { kind: "scheduled", at: def.next_run_at, label: formatRelative(def.next_run_at, nowMs) };
}

export function lastRunLabel(def: TaskDef, nowMs: number): string {
  if (!def.last_fired_at) return "从未执行";
  return formatRelative(def.last_fired_at, nowMs);
}

/// 历史按开始时间倒序（最新在前）；不修改入参。
export function sortRuns(runs: TaskRunRecord[]): TaskRunRecord[] {
  return [...runs].sort((a, b) => {
    const ta = Date.parse(a.started_at);
    const tb = Date.parse(b.started_at);
    const va = Number.isFinite(ta) ? ta : 0;
    const vb = Number.isFinite(tb) ? tb : 0;
    return vb - va;
  });
}

/// 按任务分组（值为倒序后的数组）。
export function runsByTask(runs: TaskRunRecord[]): Record<string, TaskRunRecord[]> {
  const out: Record<string, TaskRunRecord[]> = {};
  for (const r of sortRuns(runs)) {
    (out[r.task_id] ||= []).push(r);
  }
  return out;
}

/// 最近一次运行（倒序第一条）。
export function latestRun(runs: TaskRunRecord[], taskId: string): TaskRunRecord | null {
  const list = runsByTask(runs)[taskId];
  return list && list.length > 0 ? list[0] : null;
}

/// 把 `task_run_now` 返回的 RunSnapshot 映射成本地会话 `TaskRunRecord`（手动触发）。
/// 用于面板「本次会话触发」即时展示；持久历史仍以后端 task-runs / script-runs 为准。
export function runRecordFromSnapshot(taskId: string, snap: RunSnapshot): TaskRunRecord {
  const when = snap.started_at ?? new Date(0).toISOString();
  return {
    task_id: taskId,
    run_id: snap.run_id,
    trigger: "manual",
    scheduled_at: when,
    attempt: 1,
    started_at: when,
    finished_at: snap.finished_at,
    status: snap.status,
    exit_code: snap.exit_code,
    error_code: snap.error ?? null,
  };
}
