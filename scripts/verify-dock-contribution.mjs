#!/usr/bin/env node
/**
 * verify-dock-contribution.mjs — UI-4 DockContribution 验证
 *
 * 证明三件事：
 *  D1 full profile 下 Dock 页签的**顺序 / 图标 / 文案**与迁移前逐项一致（UX 未变）
 *  D2 capability absent 时页签**消失**（修复旧实现「死页签」缺陷），不会渲染空面板
 *  D3 Shell 源码中不再硬编码 Dock view 名（抽象是 generic，不是 Browser/Terminal 特例）
 *
 * 实现说明：
 *  - contributionRegistry 是**单例**，同一进程内第二次 bootstrap 会提前返回，
 *    因此每个 profile 在**独立 node 进程**中运行，才能真正反映该 profile 的注册结果。
 *  - 用 Vite SSR 构建（项目真实 toolchain 处理 .vue），不用 esbuild 直接跑
 *    （esbuild 无法在 node 里加载 external 的 .vue）。
 */

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, rmSync, mkdirSync, writeFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CACHE = join(ROOT, "node_modules/.cache/dock-verify");
const P = (p) => join(ROOT, p).replace(/\\/g, "/");

let fails = 0;
function ok(id, msg) { console.log(`  [PASS   ] ${id}  ${msg}`); }
function bad(id, msg, detail) {
  console.log(`  [FAIL   ] ${id}  ${msg}`);
  if (detail) console.log(`             ${detail}`);
  fails++;
}

// ---------- 构建一次 SSR 探针，按 argv 接收 profile ----------
mkdirSync(CACHE, { recursive: true });
const entrySrc = `
import { bootstrapCapabilityRuntime } from '${P("src/capability/index.ts")}'
import { contributionRegistry } from '${P("src/capability/contribution/registry.ts")}'
import { CONTRIBUTION_SLOTS } from '${P("src/capability/contribution/types.ts")}'
bootstrapCapabilityRuntime(process.argv[2])
const tabs = contributionRegistry
  .getDockTabContributions(CONTRIBUTION_SLOTS.BROWSER_DOCK)
  .map((c) => ({ view: c.view, icon: c.icon ?? '', label: c.label ?? '' }))
console.log(JSON.stringify(tabs))
`;
writeFileSync(join(CACHE, "probe.mjs"), entrySrc);
const OUT = join(CACHE, "out");
if (existsSync(OUT)) rmSync(OUT, { recursive: true, force: true });
execFileSync("npx", ["vite", "build", "--ssr", join(CACHE, "probe.mjs"), "--outDir", OUT, "--logLevel", "error"], {
  cwd: ROOT, stdio: "pipe",
});
const builtJs = join(OUT, "probe.js");
if (!existsSync(builtJs)) { console.error("SSR build failed"); process.exit(1); }
// vite 产出 .js；本仓非 type=module → 复制成 .mjs 让 node 按 ESM 解析
writeFileSync(join(OUT, "probe.mjs"), readFileSync(builtJs, "utf8"));

function tabsOf(profile) {
  const raw = execFileSync("node", [join(OUT, "probe.mjs"), profile], { cwd: ROOT, encoding: "utf8" });
  return JSON.parse(raw.trim().split("\n").pop());
}

// 迁移**前** MainArea 中硬编码的 Dock 页签（顺序/图标/文案）——UX 基线
const BEFORE_TABS = [
  { view: "files", icon: "📂", label: "文件" },
  { view: "term", icon: "💻", label: "终端" },
  { view: "net", icon: "🌊", label: "资源" },
  { view: "session", icon: "💾", label: "会话" },
];

console.log("--- DOCK CONTRIBUTION (UI-4) ---");

// ---- D1: full profile 与迁移前逐项一致 ----
const fullTabs = tabsOf("full");
const same =
  fullTabs.length === BEFORE_TABS.length &&
  BEFORE_TABS.every((b, i) => {
    const a = fullTabs[i];
    return a && a.view === b.view && a.icon === b.icon && a.label === b.label;
  });
if (same) ok("D1", `full profile Dock 页签与迁移前一致：${fullTabs.map((t) => t.icon + t.label).join(" ")}`);
else bad("D1", "Dock 页签顺序/图标/文案与迁移前不一致", `before=${JSON.stringify(BEFORE_TABS)} after=${JSON.stringify(fullTabs)}`);

const missingMeta = fullTabs.filter((t) => !t.icon || !t.label);
if (missingMeta.length === 0) ok("D1b", "所有 Dock 贡献均声明 icon/label（无空白页签）");
else bad("D1b", "存在未声明 icon/label 的 Dock 贡献", JSON.stringify(missingMeta));

// ---- D2: capability absent → 无死页签 ----
const fwTabs = tabsOf("framework");
if (fwTabs.length === 0) ok("D2", "framework profile（能力缺席）Dock 页签 = 0 → 无死页签");
else bad("D2", "framework profile 仍存在 Dock 页签（应为 0）", JSON.stringify(fwTabs));

// ---- D3: Shell 源码不再硬编码 Dock view ----
const src = readFileSync(join(ROOT, "src/components/layout/MainArea.vue"), "utf8")
  .replace(/\/\*[\s\S]*?\*\//g, "")
  .replace(/(^|[^:])\/\/.*$/gm, "$1");
const hardcoded = [...src.matchAll(/dockOf\(\s*['"]([a-z]+)['"]\s*\)/g)].map((m) => m[1]);
if (hardcoded.length === 0) ok("D3", "MainArea 不再硬编码 dockOf('…')，改为贡献驱动");
else bad("D3", "MainArea 仍硬编码 Dock view", hardcoded.join(", "));

if (!/dock\s*type\s*===|isBrowser\s*===|isTerminal\s*===/.test(src)) {
  ok("D3b", "Shell 无 dock/isBrowser/isTerminal 等业务 switch（抽象是 generic）");
} else {
  bad("D3b", "Shell 出现 capability-specific switch");
}

console.log(`\nDOCK_CONTRIBUTION_RESULT=${fails === 0 ? "PASS" : "FAIL"} (fail=${fails})`);
process.exit(fails === 0 ? 0 : 1);
