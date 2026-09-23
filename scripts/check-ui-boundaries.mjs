#!/usr/bin/env node
/**
 * check-ui-boundaries.mjs — UI Component System 边界门禁（Phase UI-1 / §19）
 *
 * 设计原则：
 *  - **禁止只 grep 一两个字符串就宣称完整治理**：每条 gate 都有明确扫描域 + 基线比对。
 *  - **禁止静默洗绿**：历史债务走显式 baseline；新增耦合 FAIL，baseline 与现实不符也 FAIL。
 *  - **禁止把 VACUOUS 伪装成 PASS**：shared/ui 尚未创建时，UI-01/02/08 显式标记 VACUOUS。
 *
 * Gates:
 *   UI-01 shared/ui 不得 import capability internals
 *   UI-02 shared/ui 不得 import business stores
 *   UI-03 Workbench 不得直接 import capability internals / 渲染业务面板（基线化）
 *   UI-04 Capability A 不得 import Capability B internals
 *   UI-05 禁止新增 position: fixed browser overlay
 *   UI-06 禁止重新出现已废弃 toast-pop 等历史反模式
 *   UI-07 App.vue 不得新增 capability-specific native commands
 *   UI-08 shared/ui 不得调用 native invoke
 *   UI-09 Contribution Host 不得保存 capability business truth（硬编码视图白名单基线化）
 *   UI-10 机器可读 catalog 与真实文件必须一致
 *
 * Usage:
 *   node scripts/check-ui-boundaries.mjs              # 正常检查
 *   node scripts/check-ui-boundaries.mjs --self-test  # 自检（含 positive/negative fixture）
 */

import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname, resolve, basename } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const BASELINE_PATH = join(ROOT, "docs/architecture/ui-system/ui-boundary-baseline.json");
const CATALOG_PATH = join(ROOT, "docs/architecture/ui-system/ui-components.yaml");
const SHARED_UI_DIR = join(ROOT, "src/shared/ui");

const results = [];
function ok(id, msg) { results.push({ id, status: "PASS", msg }); }
function bad(id, msg, detail) { results.push({ id, status: "FAIL", msg, detail }); }
function warn(id, msg, detail) { results.push({ id, status: "WARN", msg, detail }); }
function vacuous(id, msg) { results.push({ id, status: "VACUOUS", msg }); }

// ---------------------------------------------------------------- 工具

function walk(dir, exts) {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) out.push(...walk(p, exts));
    else if (exts.some((x) => p.endsWith(x))) out.push(p);
  }
  return out;
}
const rel = (p) => relative(ROOT, p).replace(/\\/g, "/");
const read = (p) => (existsSync(p) ? readFileSync(p, "utf8") : "");

/** 去掉注释，避免「说明文字里的禁用词」造成假阳性 */
function stripComments(src) {
  return src.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:])\/\/.*$/gm, "$1");
}

/**
 * 提取所有 import 的模块说明符。
 * 必须同时覆盖两种写法，否则会漏检造成假 PASS：
 *   1) 静态：import X from "..."
 *   2) 动态：defineAsyncComponent(() => import("..."))  ← MainArea 加载业务面板正是这种
 */
function importsOf(src) {
  const out = [];
  const reStatic = /(?:^|\n)\s*import\s+(?:[^"']*?\s+from\s+)?["']([^"']+)["']/g;
  let m;
  while ((m = reStatic.exec(src))) out.push(m[1]);
  const reDynamic = /\bimport\s*\(\s*["']([^"']+)["']\s*\)/g;
  while ((m = reDynamic.exec(src))) out.push(m[1]);
  return out;
}

// ---------------------------------------------------------------- 通用核（可被 fixture 复用）

/**
 * 检查单个 shared/ui 文件内容是否违反 UI-01/02/08。
 * 纯函数 —— positive/negative fixture 直接喂字符串。
 */
export function scanSharedUiContent(file, content) {
  const v = [];
  const src = stripComments(content);
  for (const spec of importsOf(src)) {
    if (/(^|\/)capabilities\//.test(spec) && !/\/public$/.test(spec)) {
      v.push({ gate: "UI-01", file, spec, reason: "shared/ui 不得 import capability internals" });
    }
    if (/(^|\/)capabilities\//.test(spec)) {
      v.push({ gate: "UI-01", file, spec, reason: "shared/ui 不得 import capabilities/**（含 public）" });
    }
    if (/(^|\/)stores\//.test(spec) || /\/state\/use[A-Z]\w*Store/.test(spec)) {
      v.push({ gate: "UI-02", file, spec, reason: "shared/ui 不得 import business stores" });
    }
  }
  // UI-08：不仅禁止「调用」，也禁止「具备调用能力」—— import invoke / @tauri-apps 已属违规
  if (/\binvoke\s*\(/.test(src) || /\bbridge\s*\./.test(src) || /__TAURI/.test(src)) {
    v.push({ gate: "UI-08", file, spec: "-", reason: "shared/ui 不得调用 native invoke / bridge" });
  } else {
    for (const spec of importsOf(src)) {
      if (/@tauri-apps\//.test(spec)) {
        v.push({ gate: "UI-08", file, spec, reason: "shared/ui 不得 import @tauri-apps/*" });
      }
    }
    if (/import\s*\{[^}]*\binvoke\b[^}]*\}/.test(src)) {
      v.push({ gate: "UI-08", file, spec: "-", reason: "shared/ui 不得 import invoke" });
    }
  }
  return v;
}

/** UI-06：历史反模式 */
export function scanDeprecatedPatterns(file, content) {
  const v = [];
  const lines = content.split("\n");
  lines.forEach((line, i) => {
    if (/toast-pop/.test(line)) {
      v.push({ gate: "UI-06", file, line: i + 1, reason: "禁止恢复 .toast-pop 固定浮层（PROJECT-RULES 3.8）" });
    }
    if (/已新建页签/.test(line) && /toast|Toast|showToast/i.test(line)) {
      v.push({ gate: "UI-06", file, line: i + 1, reason: "已废弃：新建页签成功不显示提示" });
    }
  });
  return v;
}

// ---------------------------------------------------------------- Gates

function gateSharedUi(base) {
  const exists = existsSync(SHARED_UI_DIR);
  const status = base?.shared_ui?.status ?? "NOT_CREATED";
  // §13：shared/ui 已声明建立却不存在 → 不得退回 VACUOUS，直接 FAIL（防止 baseline 洗绿）
  if (!exists && status === "CREATED") {
    for (const g of ["UI-01", "UI-02", "UI-08"]) {
      bad(g, `baseline 声明 shared_ui.status=CREATED 但 ${rel(SHARED_UI_DIR)} 不存在 → 禁止退回 VACUOUS`);
    }
    return;
  }
  if (!exists) {
    // 显式标记 VACUOUS：绝不伪装成「已治理」
    vacuous("UI-01", `shared/ui 尚未创建（baseline status=${status}）→ VACUOUS，非治理 PASS`);
    vacuous("UI-02", `shared/ui 尚未创建 → VACUOUS，非治理 PASS`);
    vacuous("UI-08", `shared/ui 尚未创建 → VACUOUS，非治理 PASS`);
    return;
  }
  const files = walk(SHARED_UI_DIR, [".vue", ".ts"]);
  const all = [];
  for (const f of files) all.push(...scanSharedUiContent(rel(f), read(f)));
  for (const gate of ["UI-01", "UI-02", "UI-08"]) {
    const hits = all.filter((x) => x.gate === gate);
    if (hits.length === 0) ok(gate, `shared/ui 无 ${gate} 违规（扫描 ${files.length} 文件）`);
    else bad(gate, `shared/ui 存在 ${gate} 违规`, hits.map((h) => `${h.file}: ${h.spec} — ${h.reason}`).join("; "));
  }
}

function gateWorkbenchBoundary(base) {
  const shellFiles = (base.shell_files ?? []).map((f) => join(ROOT, f)).filter(existsSync);
  const bizDirs = base.business_component_dirs ?? [];

  // (a) Shell 渲染业务组件
  const actualRenders = new Set();
  for (const f of shellFiles) {
    const src = stripComments(read(f));
    for (const spec of importsOf(src)) {
      if (!spec.startsWith(".")) continue;
      const target = resolve(dirname(f), spec);
      const r = rel(target);
      if (bizDirs.some((d) => r.startsWith(d))) {
        actualRenders.add(`${rel(f)}|${basename(target).replace(/\.vue$/, "")}`);
      }
    }
  }
  const baseRenders = new Set(
    (base.ui03_shell_renders_business_baseline ?? []).map((x) => `${x.shell}|${x.component}`)
  );
  const newR = [...actualRenders].filter((x) => !baseRenders.has(x));
  const goneR = [...baseRenders].filter((x) => !actualRenders.has(x));
  if (newR.length === 0 && goneR.length === 0) {
    ok("UI-03", `Shell→业务渲染与基线一致（${baseRenders.size} 处历史债务，未新增）`);
  } else {
    bad(
      "UI-03",
      "Shell→业务渲染与基线不一致（禁止静默洗绿）",
      [newR.length ? `新增: ${newR.join(", ")}` : "", goneR.length ? `基线已失效需更新: ${goneR.join(", ")}` : ""]
        .filter(Boolean).join(" | ")
    );
  }

  // (b) Shell 持有业务 store
  const actualStores = new Set();
  for (const f of shellFiles) {
    const src = stripComments(read(f));
    const re = /import\s*\{([^}]*)\}\s*from\s*["']([^"']+)["']/g;
    let m;
    while ((m = re.exec(src))) {
      const names = m[1].split(",").map((s) => s.trim()).filter(Boolean);
      const spec = m[2];
      const isBiz =
        /(^|\/)capabilities\//.test(spec) || /\/stores\/useGitStore/.test(spec);
      if (!isBiz) continue;
      for (const n of names) if (/^use\w*Store$/.test(n)) actualStores.add(`${rel(f)}|${n}`);
    }
  }
  const baseStores = new Set(
    (base.ui03_shell_business_stores_baseline ?? []).map((x) => `${x.shell}|${x.store}`)
  );
  const newS = [...actualStores].filter((x) => !baseStores.has(x));
  const goneS = [...baseStores].filter((x) => !actualStores.has(x));
  if (newS.length === 0 && goneS.length === 0) {
    ok("UI-03b", `Shell 持有业务 store 与基线一致（${baseStores.size} 处历史债务，未新增）`);
  } else {
    bad(
      "UI-03b",
      "Shell 持有业务 store 与基线不一致",
      [newS.length ? `新增: ${newS.join(", ")}` : "", goneS.length ? `基线已失效需更新: ${goneS.join(", ")}` : ""]
        .filter(Boolean).join(" | ")
    );
  }
}

function gateCapabilityCrossImports(base) {
  const capRoot = join(ROOT, "src/capabilities");
  const files = walk(capRoot, [".vue", ".ts"]);
  const internal = [];
  const crossPublic = [];
  for (const f of files) {
    const r = rel(f);
    const own = r.split("/")[2]; // src/capabilities/<id>/...
    const src = stripComments(read(f));
    for (const spec of importsOf(src)) {
      // 关键：能力内部多用相对路径（"../../browser/public"），说明符里不含 "capabilities/"。
      // 必须先 resolve 再判断归属，否则跨能力 import 会被整条漏检（假 PASS）。
      const target = resolve(dirname(f), spec);
      const tr = rel(target);
      if (!tr.startsWith("src/capabilities/")) continue;
      const other = tr.split("/")[2];
      if (!other || other === own) continue;
      if (/\/state\/|\/ui\//.test(tr)) {
        internal.push({ from: r, to: tr });
      } else if (/\/public$/.test(tr)) {
        crossPublic.push({ from: r, to: tr, own });
      }
    }
  }
  if (internal.length === 0) ok("UI-04", "Capability 之间无 internal（state/ui）直接 import");
  else bad("UI-04", "Capability 之间存在 internal import", internal.map((x) => `${x.from} → ${x.to}`).join("; "));

  // UI-04b：跨能力 public import 必须与基线一致；其中「manifest 未声明」的部分显式 WARN，不得静默
  const baseAll = new Set(base.ui04b_cross_capability_public_baseline ?? []);
  const baseUndeclared = base.ui04b_undeclared_cross_capability_baseline ?? [];
  const found = new Set(crossPublic.map((x) => x.from));
  const newOnes = [...found].filter((f) => !baseAll.has(f));
  const gone = [...baseAll].filter((f) => !found.has(f));
  if (newOnes.length === 0 && gone.length === 0) {
    ok("UI-04b", `跨能力 public import 与基线一致（${baseAll.size} 处，全部走 public 出口）`);
  } else {
    bad(
      "UI-04b",
      "跨能力 public import 与基线不一致",
      [newOnes.length ? `新增: ${newOnes.join(", ")}` : "",
       gone.length ? `基线已失效需更新: ${gone.join(", ")}` : ""].filter(Boolean).join(" | ")
    );
  }
  // 已知未声明依赖：显式 WARN（这是当前全仓唯一真实边界瑕疵）
  for (const u of baseUndeclared) {
    if (found.has(u.file)) warn("UI-04b-U", `未声明的跨能力依赖（${u.severity}）`, `${u.file}: ${u.note}`);
  }
}

function gatePositionFixed(base) {
  // 基线以「文件 + 规则文本」为键，**不以行号为键**：
  // 行号会因任意一处插入/删除而整体漂移，导致假 FAIL（本次 pilot 加 import 即触发）。
  // 以文本为键仍可精确识别「新增的 fixed 浮层」。
  // 以「文件 + 选择器」为键：
  //  - 不用行号（任何插入都会让行号漂移）
  //  - 不用整条规则文本（UI-5 token 化会把 #fff 改成 var(--ui-surface)，属合法变更）
  // 选择器能稳定标识「哪个浮层存在」，同时仍能精确发现新增的 fixed 浮层。
  const norm = (s) => (s || "").replace(/\s+/g, "");
  const selectorOf = (t) => norm((t || "").split("{")[0]);
  const keyOf = (x) => `${x.file}|${selectorOf(x.text ?? x.selector)}`;
  const baseSet = new Set((base.ui05_position_fixed_baseline ?? []).map(keyOf));
  const found = [];
  for (const f of [join(ROOT, "src/styles/global.css"), ...walk(join(ROOT, "src"), [".vue"])]) {
    const lines = read(f).split("\n");
    lines.forEach((line, i) => {
      if (/position:\s*fixed/.test(line)) found.push({ file: rel(f), line: i + 1, text: line.trim().slice(0, 80) });
    });
  }
  const newOnes = found.filter((x) => !baseSet.has(keyOf(x)));
  const gone = [...baseSet].filter((k) => !found.some((x) => keyOf(x) === k));
  if (newOnes.length === 0 && gone.length === 0) {
    ok("UI-05", `position:fixed 与基线一致（${baseSet.size} 处既有浮层，未新增）`);
  } else {
    bad(
      "UI-05",
      "position:fixed 与基线不一致（禁止新增覆盖 browser 的固定浮层）",
      [newOnes.length ? `新增: ${newOnes.map((x) => `${x.file}:${x.line}`).join(", ")}` : "",
       gone.length ? `基线已失效需更新: ${gone.join(", ")}` : ""].filter(Boolean).join(" | ")
    );
  }
}

function gateDeprecated(base) {
  const all = [];
  for (const f of walk(join(ROOT, "src"), [".vue", ".ts", ".css"])) {
    all.push(...scanDeprecatedPatterns(rel(f), read(f)));
  }
  if (all.length === 0) ok("UI-06", "无已废弃 UI 反模式（toast-pop / 已新建页签 Toast）");
  else bad("UI-06", "出现已废弃 UI 反模式", all.map((x) => `${x.file}:${x.line}`).join("; "));
}

function gateAppNativeCommands(base) {
  const appVue = join(ROOT, "src/App.vue");
  const src = stripComments(read(appVue));
  const found = new Set();
  let m;
  const re = /bridge\.(\w+)/g;
  while ((m = re.exec(src))) found.add(m[1]);
  const baseSet = new Set(base.ui07_app_native_commands_baseline ?? []);
  const newOnes = [...found].filter((x) => !baseSet.has(x));
  const gone = [...baseSet].filter((x) => !found.has(x));
  if (newOnes.length === 0 && gone.length === 0) {
    ok("UI-07", `App.vue native 命令与基线一致（${baseSet.size} 个，未新增）`);
  } else {
    bad(
      "UI-07",
      "App.vue native 命令与基线不一致（不得新增 capability-specific native 命令）",
      [newOnes.length ? `新增: ${newOnes.join(", ")}` : "",
       gone.length ? `基线已失效需更新: ${gone.join(", ")}` : ""].filter(Boolean).join(" | ")
    );
  }
}

function gateContributionHostTruth(base) {
  // Contribution Host 不得新增硬编码业务视图白名单（business truth 泄入 Shell）
  const hosts = [join(ROOT, "src/components/layout/MainArea.vue"), join(ROOT, "src/components/layout/ActivityBar.vue")];
  const baseViews = new Set(base.ui09_host_hardcoded_views_baseline ?? []);
  const found = new Set();
  for (const f of hosts) {
    const src = stripComments(read(f));
    let m;
    const re = /["']([a-z][a-z0-9_-]{2,})["']/g;
    while ((m = re.exec(src))) if (baseViews.has(m[1])) found.add(m[1]);
  }
  // 检测：是否出现「形如 view id 但不在基线内」的字符串字面量，且位于 view 判定上下文
  const extras = new Set();
  for (const f of hosts) {
    const src = stripComments(read(f));
    let m;
    const re = /mainView\s*===?\s*["']([a-z][a-z0-9_-]{2,})["']/g;
    while ((m = re.exec(src))) extras.add(m[1]);
    const re2 = /viewOf\(\s*["']([a-z][a-z0-9_-]{2,})["']\s*\)/g;
    while ((m = re2.exec(src))) extras.add(m[1]);
    const re3 = /dockOf\(\s*["']([a-z][a-z0-9_-]{2,})["']\s*\)/g;
    while ((m = re3.exec(src))) extras.add(m[1]);
  }
  const newViews = [...extras].filter((x) => !baseViews.has(x));
  if (newViews.length === 0) {
    ok("UI-09", `Contribution Host 未新增硬编码视图白名单（基线 ${baseViews.size} 个）`);
  } else {
    bad("UI-09", "Contribution Host 新增了硬编码业务视图（business truth 泄入 Shell）", newViews.join(", "));
  }
}

function gateCatalogConsistency() {
  if (!existsSync(CATALOG_PATH)) {
    bad("UI-10", "机器可读 catalog 不存在", CATALOG_PATH);
    return;
  }
  const text = read(CATALOG_PATH);
  const entries = [];
  for (const line of text.split("\n")) {
    if (!/^\s*-\s*\{/.test(line)) continue;
    const id = /id:\s*([^,}]+)/.exec(line)?.[1]?.trim();
    const file = /file:\s*([^,}]+)/.exec(line)?.[1]?.trim();
    if (id && file) entries.push({ id, file });
  }
  const missing = entries.filter((e) => !existsSync(join(ROOT, e.file)));
  const actualVues = new Set(walk(join(ROOT, "src"), [".vue"]).map(rel));
  const cataloged = new Set(entries.map((e) => e.file));
  const uncataloged = [...actualVues].filter((f) => !cataloged.has(f));
  const dupIds = entries.map((e) => e.id).filter((v, i, a) => a.indexOf(v) !== i);

  if (missing.length === 0 && uncataloged.length === 0 && dupIds.length === 0) {
    ok("UI-10", `catalog 与真实文件一致（${entries.length} 条 / ${actualVues.size} 个 .vue）`);
  } else {
    bad(
      "UI-10",
      "catalog 与真实文件不一致",
      [missing.length ? `catalog 指向不存在文件: ${missing.map((m) => m.file).join(", ")}` : "",
       uncataloged.length ? `未登记的真实组件: ${uncataloged.join(", ")}` : "",
       dupIds.length ? `重复 id: ${dupIds.join(", ")}` : ""].filter(Boolean).join(" | ")
    );
  }
}

// ---------------------------------------------------------------- 自检

function selfTest() {
  console.log("--- UI boundary gate self-test ---");
  let fails = 0;
  const t = (name, cond, extra = "") => {
    console.log(`  ${cond ? "PASS" : "FAIL"}  ${name}${extra ? " — " + extra : ""}`);
    if (!cond) fails++;
  };

  // negative fixture：shared/ui 组件触碰能力内部 + 业务 store + native invoke
  const neg = `
    <script setup lang="ts">
    import { useBrowserStore } from '../../capabilities/browser/state/useBrowserStore'
    import { useGitStore } from '../../stores/useGitStore'
    import { invoke } from '@tauri-apps/api/core'
    </script>
  `;
  const negV = scanSharedUiContent("fixtures/negative/BadShared.vue", neg);
  t("negative: UI-01 检出 capability internals", negV.some((v) => v.gate === "UI-01"));
  t("negative: UI-02 检出 business store", negV.some((v) => v.gate === "UI-02"));
  t("negative: UI-08 检出 native invoke", negV.some((v) => v.gate === "UI-08"));

  // positive fixture：干净的 shared/ui 组件
  const pos = `
    <script setup lang="ts">
    import { computed } from 'vue'
    const props = defineProps<{ text: string }>()
    </script>
    <template><p class="empty">{{ props.text }}</p></template>
  `;
  const posV = scanSharedUiContent("fixtures/positive/EmptyState.vue", pos);
  t("positive: 干净 shared 组件零违规", posV.length === 0, `violations=${posV.length}`);

  // negative fixture：废弃反模式
  const dep = `<div class="toast-pop">hi</div>\n`;
  t("negative: UI-06 检出 toast-pop", scanDeprecatedPatterns("f.vue", dep).length === 1);
  const clean = `<div class="empty">暂无</div>\n`;
  t("positive: 干净模板无反模式", scanDeprecatedPatterns("f.vue", clean).length === 0);

  // 注释不造成假阳性
  const commented = `// import { useBrowserStore } from '../../capabilities/browser/state'\n`;
  t("positive: 注释中的禁用 import 不误报", scanSharedUiContent("f.vue", commented).length === 0);

  console.log(fails === 0 ? "SELF_TEST=PASS" : `SELF_TEST=FAIL (${fails})`);
  return fails === 0;
}

// ---------------------------------------------------------------- main

const args = process.argv.slice(2);
if (args.includes("--self-test")) {
  process.exit(selfTest() ? 0 : 1);
}

const base = existsSync(BASELINE_PATH) ? JSON.parse(read(BASELINE_PATH)) : {};
gateSharedUi(base);
gateWorkbenchBoundary(base);
gateCapabilityCrossImports(base);
gatePositionFixed(base);
gateDeprecated(base);
gateAppNativeCommands(base);
gateContributionHostTruth(base);
gateCatalogConsistency();

let fail = 0, warnN = 0, vac = 0;
console.log("--- UI BOUNDARY GATES ---");
for (const r of results) {
  const tag = r.status.padEnd(7);
  console.log(`  [${tag}] ${r.id}  ${r.msg}`);
  if (r.detail) console.log(`             detail: ${r.detail}`);
  if (r.status === "FAIL") fail++;
  if (r.status === "WARN") warnN++;
  if (r.status === "VACUOUS") vac++;
}
const sharedUiStatus = existsSync(SHARED_UI_DIR) ? "CREATED" : "NOT_CREATED";
console.log(`\nSHARED_UI_STATUS=${sharedUiStatus}`);
console.log(`UI_BOUNDARIES_RESULT=${fail === 0 ? "PASS" : "FAIL"} (fail=${fail} warn=${warnN} vacuous=${vac})`);
process.exit(fail === 0 ? 0 : 1);
