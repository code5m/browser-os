#!/usr/bin/env node
/**
 * verify-ui-pilot.mjs — Pilot 视觉/结构等价性证明（UI-P06 / UI-P07）
 *
 * 用真实 Vue SSR 渲染 shared/ui 组件，把输出 HTML 与「迁移前手写的 DOM」逐字节比对。
 * 目的：把「应该等价」变成「证明等价」，杜绝用断言自证。
 *
 * 渲染通过 Vite SSR 构建完成（.vue 由项目真实 toolchain 编译，与生产一致）。
 *
 * Usage: node scripts/verify-ui-pilot.mjs
 */

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, rmSync, mkdirSync, writeFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TMP = join(ROOT, "node_modules/.cache/ui-pilot-ssr");
const ENTRY = join(TMP, "entry.mjs");
const OUT = join(TMP, "out");

// 1) 构造 SSR 入口：渲染 EmptyState 的四种真实用法
mkdirSync(TMP, { recursive: true });
const entrySrc = `
import { createSSRApp, h } from 'vue'
import { renderToString } from '@vue/server-renderer'
import EmptyState from '${join(ROOT, "src/shared/ui/EmptyState.vue").replace(/\\/g, "/")}'
import ContextMenu from '${join(ROOT, "src/shared/ui/ContextMenu.vue").replace(/\\/g, "/")}'
import ContextMenuItem from '${join(ROOT, "src/shared/ui/ContextMenuItem.vue").replace(/\\/g, "/")}'

const cases = [
  ['div-default', h(EmptyState, { text: '无列信息' })],
  ['p-tag',       h(EmptyState, { as: 'p', text: '暂无参数' })],
  ['li-tag',      h(EmptyState, { as: 'li', text: '无匹配资源' })],
  ['live-div',    h(EmptyState, { live: true, text: '暂无运行记录。' })],
  ['li-live',     h(EmptyState, { as: 'li', live: true, text: '暂无记录' })],
  ['slot',        h(EmptyState, null, { default: () => '暂无定时任务。' })],
  ['ctx-menu',    h(ContextMenu, { x: 100, y: 50 }, { default: () => 'ITEM' })],
  ['ctx-item',    h(ContextMenuItem, null, { default: () => '📂 打开所在目录' })],
  ['ctx-danger',  h(ContextMenuItem, { danger: true }, { default: () => '🗑 删除' })],
]

// 注意：不能用 top-level await —— vite build target (es2020/chrome87) 不支持 TLA
const out = {}
;(async () => {
  for (const [name, vnode] of cases) {
    const app = createSSRApp({ render: () => vnode })
    out[name] = await renderToString(app)
  }
  console.log(JSON.stringify(out))
})()
`;
writeFileSync(ENTRY, entrySrc);

// 2) 用项目真实 vite toolchain 做 SSR 构建（.vue 编译与生产一致）
if (existsSync(OUT)) rmSync(OUT, { recursive: true, force: true });
execFileSync(
  "npx",
  ["vite", "build", "--ssr", ENTRY, "--outDir", OUT, "--logLevel", "error"],
  { cwd: ROOT, stdio: "pipe" }
);

// 3) 执行并收集渲染结果
// vite SSR 产出为 entry.js；本仓 package.json 非 type=module，
// node 会把 .js 当 CJS 解析 → 复制成 .mjs 再执行。
const builtJs = join(OUT, "entry.js");
const built = join(OUT, "entry.mjs");
if (!existsSync(builtJs)) {
  console.error("SSR build output not found:", builtJs);
  process.exit(1);
}
writeFileSync(built, readFileSync(builtJs, "utf8"));
const raw = execFileSync("node", [built], { cwd: ROOT, encoding: "utf8" });
const rendered = JSON.parse(raw.trim().split("\n").pop());

// 4) 逐字节比对：期望 = 迁移**前**的手写 DOM
const expected = {
  "div-default": `<div class="empty">无列信息</div>`,
  "p-tag": `<p class="empty">暂无参数</p>`,
  "li-tag": `<li class="empty">无匹配资源</li>`,
  "live-div": `<div class="empty" role="status" aria-live="polite">暂无运行记录。</div>`,
  "li-live": `<li class="empty" role="status" aria-live="polite">暂无记录</li>`,
  slot: `<div class="empty">暂无定时任务。</div>`,
  // 迁移前手写形态：<div class="ctx-menu" :style="{left:X+'px',top:Y+'px'}"> / <div class="ctx-item"> / <div class="ctx-item danger">
  "ctx-menu": (s) =>
    /^<div class="ctx-menu"/.test(s) && /left:\s*100px/.test(s) && /top:\s*50px/.test(s) && s.includes("ITEM"),
  "ctx-item": `<div class="ctx-item">📂 打开所在目录</div>`,
  "ctx-danger": `<div class="ctx-item danger">🗑 删除</div>`,
};

let fail = 0;
console.log("--- UI-PILOT STRUCTURAL EQUIVALENCE (UI-P06) ---");
for (const [name, exp] of Object.entries(expected)) {
  const norm = (s) => (s || "").replace(/<!--[^-]*-->/g, "").trim();
  const act = norm(rendered[name]);
  // exp 可以是字符串（逐字节比对）或谓词函数（结构断言），后者用于含动态 style 的场景
  const same = typeof exp === "function" ? !!exp(act) : act === exp;
  console.log(`  ${same ? "PASS" : "FAIL"}  ${name}`);
  if (!same) {
    console.log(`        expected: ${typeof exp === "function" ? "<predicate>" : exp}`);
    console.log(`        actual  : ${act}`);
    fail++;
  }
}

// 5) 反例：shared/ui 不得被业务污染（源码级）
const src = readFileSync(join(ROOT, "src/shared/ui/EmptyState.vue"), "utf8");
// 必须锚定行首：组件注释里写了「不带 <style> 块」的说明文字，裸 /<style/ 会假阳性
const hasStyle = /^\s*<style[\s>]/m.test(src);
console.log("--- UI-PILOT PURITY ---");
if (hasStyle) {
  console.log("  FAIL  EmptyState 含 <style> 块（会产生 data-v-xxx，破坏 DOM 等价）");
  fail++;
} else {
  console.log("  PASS  EmptyState 无 <style> 块 → 无 scopeId → DOM 与旧实现逐字节一致");
}
for (const [label, re] of [
  ["capabilities/**", /capabilities\//],
  ["business store", /use[A-Z]\w*Store/],
  ["bridge/invoke", /bridge\.|invoke\s*\(/],
]) {
  const hit = re.test(src);
  console.log(`  ${hit ? "FAIL" : "PASS"}  EmptyState 无 ${label}`);
  if (hit) fail++;
}

console.log(`\nUI_PILOT_RESULT=${fail === 0 ? "PASS" : "FAIL"} (fail=${fail})`);
process.exit(fail === 0 ? 0 : 1);
