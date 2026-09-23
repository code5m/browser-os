#!/usr/bin/env node
/**
 * verify-design-tokens.mjs — Design Token Foundation 验证（UI-5 / §22~§25）
 *
 * 核心不变量：**before computed value === after computed value**。
 * 做法：token 的声明值必须与它替换掉的硬编码字面量**逐字相同**，
 *       因此 var() 解析结果与迁移前完全一致 —— token 化不等于 redesign。
 *
 * 同时校验 §24 冲突处置：死规则可删、真冲突不动。
 */

import { readFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TOKENS = join(ROOT, "src/styles/tokens.css");
const GLOBAL = join(ROOT, "src/styles/global.css");
const MAIN = join(ROOT, "src/main.ts");

let fails = 0;
const ok = (id, m) => console.log(`  [PASS   ] ${id}  ${m}`);
const bad = (id, m, d) => { console.log(`  [FAIL   ] ${id}  ${m}`); if (d) console.log(`             ${d}`); fails++; };

/** token → 它替换掉的原始字面量（迁移前的值） */
const ORIGINALS = {
  "--ui-surface": "#fff",
  "--ui-text": "#333",
  "--ui-text-muted": "#bbb",
  "--ui-danger": "#c33",
  "--ui-hover-bg": "#eef3ff",
  "--ui-danger-bg": "#ffeaea",
};

/** 已 token 化的规则：选择器片段 → 期望的 var() 声明片段 */
const SUBSTITUTIONS = [
  [".empty {", "color: var(--ui-text-muted)"],
  [".ctx-menu {", "background: var(--ui-surface)"],
  [".ctx-item {", "color: var(--ui-text)"],
  [".ctx-item:hover {", "background: var(--ui-hover-bg)"],
  [".ctx-item.danger {", "color: var(--ui-danger)"],
  [".ctx-item.danger:hover {", "background: var(--ui-danger-bg)"],
];

console.log("--- DESIGN TOKEN FOUNDATION (UI-5) ---");

// 必须去注释：tokens.css 的说明文字里写了「禁止 --gray-1 / --gray-2」，
// 裸匹配会把说明本身当成违规（假阳性）。
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:])\/\/.*$/gm, "$1");
const tokensSrc = strip(readFileSync(TOKENS, "utf8"));
const globalSrc = readFileSync(GLOBAL, "utf8");
const mainSrc = readFileSync(MAIN, "utf8");

// T1: token 声明值必须逐字等于原始字面量
for (const [name, original] of Object.entries(ORIGINALS)) {
  const m = new RegExp(`${name}\\s*:\\s*([^;]+);`).exec(tokensSrc);
  if (!m) { bad("T1", `token ${name} 未声明`); continue; }
  const val = m[1].trim();
  if (val === original) ok("T1", `${name} = ${val}（与原子面量一致，computed 不变）`);
  else bad("T1", `${name} 取值漂移`, `expected=${original} actual=${val}`);
}

// T2: token 必须是 :root 语义变量，不得是 --gray-N 编号
const numbered = [...tokensSrc.matchAll(/--(?:gray|grey|color)-?\d+/gi)].map((m) => m[0]);
if (numbered.length === 0) ok("T2", "无 --gray-N 编号式 token（语义命名）");
else bad("T2", "出现编号式 token", numbered.join(", "));

// T3: 已 token 化的规则确实使用 var()
for (const [sel, decl] of SUBSTITUTIONS) {
  const idx = globalSrc.indexOf(sel);
  if (idx < 0) { bad("T3", `未找到规则 ${sel}`); continue; }
  const rule = globalSrc.slice(idx, globalSrc.indexOf("}", idx) + 1);
  if (rule.includes(decl)) ok("T3", `${sel} → ${decl}`);
  else bad("T3", `${sel} 未使用 token`, `rule=${rule.slice(0, 90)}`);
}

// T4: tokens.css 必须被引入（否则 var() 全部失效 → 视觉崩塌）
if (/import\s+["'].*styles\/tokens\.css["']/.test(mainSrc)) ok("T4", "tokens.css 已在 main.ts 引入");
else bad("T4", "tokens.css 未在 main.ts 引入");

// T5: §24 冲突处置 —— 死规则已删、真冲突保持不动
const deadRemoved = !/^\.path-bar\s*\{\s*display:\s*flex;[^}]*gap:\s*6px/m.test(globalSrc);
if (deadRemoved) ok("T5", "LEGACY_DRIFT 死规则 .path-bar(gap 6px) 已删除（被后者完全覆盖）");
else bad("T5", "死规则 .path-bar(gap 6px) 仍存在");
if (/\.path-bar\s+\.path\s*\{/.test(globalSrc)) ok("T5b", "未被覆盖的 .path-bar .path 已保留");
else bad("T5b", ".path-bar .path 被误删（它无后续覆盖）");

// T6: 真冲突不得被自动统一（保持两个不同值）
const modalBgs = [...globalSrc.matchAll(/\.(?:home-)?modal-mask\s*\{[^}]*background:\s*([^;]+);/g)].map((m) => m[1].trim());
const uniqueBgs = new Set(modalBgs);
if (uniqueBgs.size >= 2) ok("T6", `modal 遮罩冲突未被强行统一（保留 ${[...uniqueBgs].join(" / ")}）`);
else bad("T6", "modal 遮罩被统一（会改变视觉）", [...uniqueBgs].join(","));

console.log(`\nDESIGN_TOKEN_RESULT=${fails === 0 ? "PASS" : "FAIL"} (fail=${fails})`);
process.exit(fails === 0 ? 0 : 1);
