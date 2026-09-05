// src/utils/scriptUi.ts
// M2-5.a 脚本库 CRUD UI 的纯逻辑层（无 Vue / bridge 运行时依赖，供 headless 测试加载真实逻辑）。
// 组件只负责渲染与事件，所有可判定逻辑集中在此，便于 `check-script-ui-logic.mjs` 验证。

import type { ScriptMeta, ScriptParam, ScriptInterpreter, ScriptRunRecord, RunStatus } from "../types";

export interface ScriptForm {
  /** null = 新建；非 null = 编辑既有 */
  id: string | null;
  name: string;
  category: string;
  interpreter: ScriptInterpreter;
  description: string;
  body: string;
  params: ScriptParam[];
  /** 0 = 使用全局默认（60） */
  timeout_secs: number;
  enabled: boolean;
  builtin: boolean;
}

export interface CategoryNode {
  category: string;
  scripts: ScriptMeta[];
}

export interface ParamIssue {
  index: number;
  field: string;
  message: string;
}

export interface FormIssue {
  field: string;
  message: string;
}

const ID_NAME_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
const SCRIPT_NAME_RE = /^[A-Za-z0-9_\-. ]+$/;

export function emptyScriptForm(): ScriptForm {
  return {
    id: null,
    name: "",
    category: "general",
    interpreter: "bash",
    description: "",
    body: "",
    params: [],
    timeout_secs: 0,
    enabled: true,
    builtin: false,
  };
}

export function loadFormFromMeta(m: ScriptMeta): ScriptForm {
  return {
    id: m.id,
    name: m.name,
    category: m.category,
    interpreter: m.interpreter,
    description: m.description,
    body: m.body,
    params: m.params.map((p) => ({ ...p, options: [...p.options] })),
    timeout_secs: m.timeout_secs,
    enabled: m.enabled,
    builtin: m.builtin,
  };
}

/** 校验单个参数：name 形态、enum 必填 options 等（U7/U9 相关） */
export function validateParam(p: ScriptParam, index: number): ParamIssue[] {
  const issues: ParamIssue[] = [];
  if (!p.name || !ID_NAME_RE.test(p.name)) {
    issues.push({
      index,
      field: "name",
      message: `参数 ${index + 1} 占位符名非法（须 ^[A-Za-z_][A-Za-z0-9_]*$）`,
    });
  }
  if (!p.label) {
    issues.push({ index, field: "label", message: `参数 ${index + 1} 显示名不能为空` });
  }
  if (p.param_type === "enum" && (!p.options || p.options.length === 0)) {
    issues.push({
      index,
      field: "options",
      message: `参数 ${index + 1}（enum）必须至少提供一个选项`,
    });
  }
  return issues;
}

/** 整表校验：name/category/timeout/各参数；返回空数组表示通过（U3/U7/U9） */
export function validateScriptForm(form: ScriptForm): FormIssue[] {
  const issues: FormIssue[] = [];
  const name = form.name.trim();
  if (!name) {
    issues.push({ field: "name", message: "脚本名不能为空" });
  } else if (!SCRIPT_NAME_RE.test(name)) {
    issues.push({ field: "name", message: "脚本名含非法字符（仅允许字母数字 _ - . 和空格）" });
  }
  if (!form.category.trim()) {
    issues.push({ field: "category", message: "分类不能为空" });
  }
  if (form.timeout_secs < 0) {
    issues.push({ field: "timeout_secs", message: "超时不能为负" });
  }
  for (let i = 0; i < form.params.length; i++) {
    for (const pi of validateParam(form.params[i], i)) {
      issues.push({ field: `params[${pi.index}].${pi.field}`, message: pi.message });
    }
  }
  return issues;
}

/** 序列化为 script_add / script_update 的载荷（剔除前端字段，只传正文不传路径） */
export function serializeScriptForm(form: ScriptForm): {
  name: string;
  category: string;
  interpreter: ScriptInterpreter;
  body: string;
  params: ScriptParam[];
  description: string | null;
  timeoutSecs: number | null;
  enabled: boolean;
} {
  return {
    name: form.name.trim(),
    category: form.category.trim(),
    interpreter: form.interpreter,
    body: form.body,
    params: form.params,
    description: form.description || null,
    timeoutSecs: form.timeout_secs > 0 ? form.timeout_secs : null,
    enabled: form.enabled,
  };
}

export function buildCategoryTree(scripts: ScriptMeta[]): CategoryNode[] {
  const map = new Map<string, ScriptMeta[]>();
  for (const s of scripts) {
    const cat = s.category || "general";
    if (!map.has(cat)) map.set(cat, []);
    map.get(cat)!.push(s);
  }
  return [...map.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([category, list]) => ({
      category,
      scripts: list.slice().sort((a, b) => a.name.localeCompare(b.name)),
    }));
}

export function groupScriptsByCategory(scripts: ScriptMeta[]): Record<string, ScriptMeta[]> {
  const out: Record<string, ScriptMeta[]> = {};
  for (const s of scripts) {
    const cat = s.category || "general";
    (out[cat] ||= []).push(s);
  }
  return out;
}

/** 内置脚本（builtin=true）不可删（U5） */
export function canDeleteScript(m: ScriptMeta | null): boolean {
  return !!m && !m.builtin;
}

/** 列出某脚本的参数名（供执行面板后续消费，a 卡仅暴露数据） */
export function paramNames(form: ScriptForm): string[] {
  return form.params.map((p) => p.name);
}

// ---------------- M2-5.c 运行历史纯逻辑（无 Vue/bridge 依赖） ----------------

/** 状态筛选：'all' 表示不过滤 */
export type RunStatusFilter = RunStatus | "all";

/** 过滤运行记录：按状态 + 按 script_id（scriptId 空表示不按 id 过滤） */
export function filterRuns(
  records: ScriptRunRecord[],
  status: RunStatusFilter,
  scriptId?: string,
): ScriptRunRecord[] {
  let out = records;
  if (status !== "all") {
    out = out.filter((r) => r.status === status);
  }
  if (scriptId) {
    out = out.filter((r) => r.script_id === scriptId);
  }
  return out;
}

/** 状态徽章文本（中文短词，给 UI 标签用） */
export function runStatusLabel(status: RunStatus): string {
  switch (status) {
    case "running":
      return "运行中";
    case "succeeded":
      return "成功";
    case "failed":
      return "失败";
    case "cancelled":
      return "已取消";
    case "timeout":
      return "超时";
  }
}

/** 状态颜色（仅返回 CSS class 后缀，便于主题切换） */
export function runStatusClass(status: RunStatus): string {
  return `status-${status}`;
}

/** 人类可读运行时长：秒/分:秒 / 时:分:秒。毫秒级精度不必要。 */
export function formatRunDuration(r: ScriptRunRecord): string {
  const start = Date.parse(r.started_at);
  // 历史记录全部是终态（M2-4.a 冻结），finished_at 必非空；防御兜底
  const end = r.finished_at ? Date.parse(r.finished_at) : NaN;
  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) {
    return "-";
  }
  const sec = Math.round((end - start) / 1000);
  if (sec < 60) return `${sec} 秒`;
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  if (m < 60) return `${m} 分 ${s} 秒`;
  const h = Math.floor(m / 60);
  return `${h} 时 ${m % 60} 分`;
}

/** 提取 YYYY-MM-DD（本地时区）作为分组 key */
function dayKey(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "未知日期";
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/** 按本地日分组，组内按 started_at 倒序（最近在前） */
export function groupRunsByDay(
  records: ScriptRunRecord[],
): Array<{ day: string; items: ScriptRunRecord[] }> {
  const sorted = records.slice().sort((a, b) => b.started_at.localeCompare(a.started_at));
  const map = new Map<string, ScriptRunRecord[]>();
  for (const r of sorted) {
    const k = dayKey(r.started_at);
    if (!map.has(k)) map.set(k, []);
    map.get(k)!.push(r);
  }
  return [...map.entries()].map(([day, items]) => ({ day, items }));
}

/** 统计：各状态计数（运行态记录若存在，会归到 running） */
export function summarizeRuns(records: ScriptRunRecord[]): Record<RunStatus, number> {
  const out: Record<RunStatus, number> = {
    running: 0,
    succeeded: 0,
    failed: 0,
    cancelled: 0,
    timeout: 0,
  };
  for (const r of records) out[r.status] += 1;
  return out;
}

/** 解析运行记录的 `output_tail` 预览：去掉尾部空白并截到前 N 字符（纯文本，不解析 ANSI） */
export function tailPreview(r: ScriptRunRecord, max = 240): string {
  const t = (r.output_tail || "").replace(/\s+$/, "");
  if (t.length <= max) return t;
  return `…${t.slice(-max)}`;
}
