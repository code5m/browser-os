// M2-6.d 命令片段库 UI 纯逻辑层（无 Vue / bridge 运行时依赖）。

import type { CommandSnippet, ScriptInterpreter, ScriptParam } from "../types";
import { validateParam } from "./scriptUi";

export interface SnippetForm {
  id: string | null;
  name: string;
  category: string;
  interpreter: ScriptInterpreter;
  argvText: string;
  params: ScriptParam[];
  description: string;
  dangerous: boolean;
  enabled: boolean;
  builtin: boolean;
  timeout_secs: number;
}

export interface SnippetCategoryNode {
  category: string;
  snippets: CommandSnippet[];
}

export interface SnippetIssue {
  field: string;
  message: string;
}

const SNIPPET_NAME_RE = /^[A-Za-z0-9_\-. ]+$/;
const PLACEHOLDER_RE = /^\{([A-Za-z_][A-Za-z0-9_]*)\}$/;

export function emptySnippetForm(): SnippetForm {
  return {
    id: null,
    name: "",
    category: "general",
    interpreter: "bash",
    argvText: "",
    params: [],
    description: "",
    dangerous: false,
    enabled: true,
    builtin: false,
    timeout_secs: 0,
  };
}

export function loadSnippetForm(m: CommandSnippet): SnippetForm {
  return {
    id: m.id,
    name: m.name,
    category: m.category,
    interpreter: m.interpreter,
    argvText: m.argv.join("\n"),
    params: m.params.map((p) => ({ ...p, options: [...p.options] })),
    description: m.description,
    dangerous: m.dangerous,
    enabled: m.enabled,
    builtin: m.builtin,
    timeout_secs: m.timeout_secs,
  };
}

export function parseArgvText(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);
}

export function placeholderOf(element: string): string | null {
  const m = PLACEHOLDER_RE.exec(element);
  return m ? m[1] : null;
}

export function validateSnippetForm(form: SnippetForm): SnippetIssue[] {
  const issues: SnippetIssue[] = [];
  const name = form.name.trim();
  const category = form.category.trim();
  const argv = parseArgvText(form.argvText);
  const paramNames = new Set(form.params.map((p) => p.name));
  const used = new Set<string>();

  if (!name) issues.push({ field: "name", message: "命令名不能为空" });
  else if (!SNIPPET_NAME_RE.test(name)) issues.push({ field: "name", message: "命令名含非法字符" });
  if (!category) issues.push({ field: "category", message: "分类不能为空" });
  if (!argv.length) issues.push({ field: "argv", message: "argv 不能为空" });
  if (form.interpreter === "shebang") issues.push({ field: "interpreter", message: "命令片段不支持 shebang" });
  if (form.timeout_secs < 0) issues.push({ field: "timeout_secs", message: "超时不能为负" });

  argv.forEach((element, index) => {
    const ph = placeholderOf(element);
    if (ph) {
      used.add(ph);
      if (!paramNames.has(ph)) issues.push({ field: `argv[${index}]`, message: `未知占位符 ${element}` });
      return;
    }
    if (element.includes("{") || element.includes("}")) {
      issues.push({ field: `argv[${index}]`, message: "占位符必须独占一个 argv 元素" });
    }
  });

  form.params.forEach((p, index) => {
    validateParam(p, index).forEach((i) => issues.push({ field: `params[${index}].${i.field}`, message: i.message }));
    if (!used.has(p.name)) issues.push({ field: `params[${index}].name`, message: `参数 ${p.name || index + 1} 未被 argv 使用` });
  });

  return issues;
}

export function serializeSnippetForm(form: SnippetForm): {
  name: string;
  category: string;
  interpreter: ScriptInterpreter;
  argv: string[];
  params: ScriptParam[];
  description: string | null;
  dangerous: boolean;
  timeoutSecs: number | null;
  enabled: boolean;
} {
  return {
    name: form.name.trim(),
    category: form.category.trim(),
    interpreter: form.interpreter,
    argv: parseArgvText(form.argvText),
    params: form.params,
    description: form.description || null,
    dangerous: form.dangerous,
    timeoutSecs: form.timeout_secs > 0 ? form.timeout_secs : null,
    enabled: form.enabled,
  };
}

export function buildSnippetCategoryTree(
  snippets: CommandSnippet[],
  query = "",
  onlyFavorites = false,
): SnippetCategoryNode[] {
  const q = query.trim().toLowerCase();
  const map = new Map<string, CommandSnippet[]>();
  for (const s of snippets) {
    if (onlyFavorites && !isFavoriteSnippet(s.id)) continue;
    const hay = `${s.name} ${s.category} ${s.description} ${s.argv.join(" ")}`.toLowerCase();
    if (q && !hay.includes(q)) continue;
    const cat = s.category || "general";
    if (!map.has(cat)) map.set(cat, []);
    map.get(cat)!.push(s);
  }
  return [...map.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([category, list]) => ({ category, snippets: list.slice().sort((a, b) => a.name.localeCompare(b.name)) }));
}

export function canDeleteSnippet(m: CommandSnippet | null): boolean {
  return !!m && !m.builtin;
}

export function isFavoriteSnippet(id: string): boolean {
  return loadSnippetFavorites().includes(id);
}

export function loadSnippetFavorites(): string[] {
  try {
    const raw = localStorage.getItem("browser-os-command-snippet-favorites");
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}

export function toggleSnippetFavorite(id: string): string[] {
  const next = loadSnippetFavorites().filter((x) => x !== id);
  if (next.length === loadSnippetFavorites().length) next.unshift(id);
  localStorage.setItem("browser-os-command-snippet-favorites", JSON.stringify(next.slice(0, 200)));
  return next.slice(0, 200);
}
