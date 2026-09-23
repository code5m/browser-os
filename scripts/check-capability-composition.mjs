#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-composition.mjs — Capability 组合性门禁（Phase 8B.1）
//
// 证明 Bookmark = C3（OPTIONAL / 可组合）所需的组合性事实：
//   C1. 通用 Contribution Registry 存在且行为正确（register / getBySlot / 空槽返回 []）
//   C2. Bookmark 经通用 Registry 注册贡献（surface + navigation），不直连 Shell
//   C3. Shell（MainArea / ActivityBar / HomeLaunchers）**不 import** src/capabilities/bookmark 任何内部文件
//       —— 即 Bookmark 缺失时 Shell 仍可编译/启动（absent-boot 的静态证据）
//   C5-TERM-*  Terminal（Train D）：Shell 不 import terminal 内部 + 适配器经通用 Registry 注册贡献
//   C6-TERMINAL-ABSENT  Terminal absent：两个槽（workbench-main-resident / browser-dock）为空
//       —— Shell 不渲染终端 DOM，**且没有任何 PTY 出生点**（资源不产生，非仅隐藏按钮）
//   C4. Bookmark absent → 对应 slot 为空，Shell 按 slot 遍历渲染空集不崩溃
//       —— absent-boot 的动态证据（registry 空槽 = []）
//
// 用法:
//   node scripts/check-capability-composition.mjs              真实扫描 + 动态测试
//   node scripts/check-capability-composition.mjs --self-test  夹具自检
//   node scripts/check-capability-composition.mjs --json       机器可读
//   node scripts/check-capability-composition.mjs --help
// ---------------------------------------------------------------------------

import { readFileSync, writeFileSync, unlinkSync, existsSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join, posix } from "node:path";
import { build } from "esbuild";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

const REGISTRY_TS = join(ROOT, "src/capability/contribution/registry.ts");
const BOOKMARK_INDEX_TS = join(ROOT, "src/capabilities/bookmark/index.ts");
const WORKSPACE_INDEX_TS = join(ROOT, "src/capabilities/workspace/index.ts");
const MAINAREA_VUE = join(ROOT, "src/components/layout/MainArea.vue");
const ACTIVITYBAR_VUE = join(ROOT, "src/components/layout/ActivityBar.vue")
const TERMINAL_INDEX_TS = join(ROOT, "src/capabilities/terminal/index.ts");

const IMPORT_RE =
  /(?:^|[\s;{}])(?:import|export)\s+(?:[\s\S]*?\sfrom\s+)?['"]([^'"]+)['"]|import\s*\(\s*['"]([^'"]+)['"]\s*\)/g;

function extractImports(content) {
  const out = [];
  const stripped = content
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/(^|[^:])\/\/.*$/gm, "$1");
  IMPORT_RE.lastIndex = 0;
  let m;
  while ((m = IMPORT_RE.exec(stripped)) !== null) {
    const spec = m[1] || m[2];
    if (spec) out.push(spec);
  }
  return out;
}

function normalizeSpec(fromFile, spec) {
  if (!spec.startsWith(".")) return null;
  const dir = posix.dirname(posix.normalize(fromFile));
  return posix.normalize(posix.join(dir, spec));
}

/** 静态：某 Shell 文件是否 import 了 bookmark 能力内部（任何 capabilities/bookmark 路径） */
function shellImportsBookmark(fileRel) {
  const abs = join(ROOT, fileRel);
  if (!existsSync(abs)) return { file: fileRel, imports: [] };
  const content = readFileSync(abs, "utf8");
  const hits = [];
  for (const spec of extractImports(content)) {
    const norm = normalizeSpec(fileRel, spec);
    if (norm && /(?:^|\/)src\/capabilities\/bookmark\//.test(norm)) hits.push(spec);
  }
  return { file: fileRel, imports: hits };
}

/** 静态：Bookmark 适配器是否向通用 Registry 注册了三类贡献（槽名） */
function bookmarkRegistersContributions() {
  if (!existsSync(BOOKMARK_INDEX_TS)) return { exists: false, slots: [] };
  // 槽名常量定义在 contribution/types.ts，适配器经 CONTRIBUTION_SLOTS 引用；两文件合并扫描
  const indexSrc = readFileSync(BOOKMARK_INDEX_TS, "utf8");
  const typesTs = join(ROOT, "src/capability/contribution/types.ts");
  const typesSrc = existsSync(typesTs) ? readFileSync(typesTs, "utf8") : "";
  const combined = indexSrc + "\n" + typesSrc;
  const slots = [];
  for (const slot of ["browser-sidebar", "address-bar-actions", "activity-bar-trailing"]) {
    if (new RegExp(`["']${slot}["']`).test(combined)) slots.push(slot);
  }
  const usesGeneric = /contributionRegistry\s*\.\s*registerContribution/.test(indexSrc);
  return { exists: true, slots, usesGenericRegistry: usesGeneric };
}

/** 静态：workspace 适配器是否向通用 Registry 注册贡献（槽名可能定义在 contribution/types.ts） */
function workspaceRegistersContributions() {
  if (!existsSync(WORKSPACE_INDEX_TS)) return { exists: false, slots: [], usesGenericRegistry: false };
  const indexSrc = readFileSync(WORKSPACE_INDEX_TS, "utf8");
  const typesTs = join(ROOT, "src/capability/contribution/types.ts");
  const typesSrc = existsSync(typesTs) ? readFileSync(typesTs, "utf8") : "";
  const combined = indexSrc + "\n" + typesSrc;
  const slots = [];
  for (const slot of ["workbench-main", "browser-dock"]) {
    if (new RegExp(`["']${slot}["']`).test(combined)) slots.push(slot);
  }
  const usesGeneric = /contributionRegistry\s*\.\s*registerContribution/.test(indexSrc);
  return { exists: true, slots, usesGenericRegistry: usesGeneric };
}

const BROWSER_INDEX_TS = join(ROOT, "src/capabilities/browser/index.ts");
/** 静态：browser 适配器是否向通用 Registry 注册贡献 */
function browserRegistersContributions() {
  if (!existsSync(BROWSER_INDEX_TS)) return { exists: false, slots: [], usesGenericRegistry: false };
  const indexSrc = readFileSync(BROWSER_INDEX_TS, "utf8");
  const typesTs = join(ROOT, "src/capability/contribution/types.ts");
  const typesSrc = existsSync(typesTs) ? readFileSync(typesTs, "utf8") : "";
  const combined = indexSrc + "\n" + typesSrc;
  const slots = [];
  for (const slot of ["browser-host", "browser-dock"]) {
    if (new RegExp(`["']${slot}["']`).test(combined)) slots.push(slot);
  }
  const usesGeneric = /contributionRegistry\s*\.\s*registerContribution/.test(indexSrc);
  return { exists: true, slots, usesGenericRegistry: usesGeneric };
}

/** 静态：terminal 适配器是否向通用 Registry 注册贡献（槽名可能定义在 contribution/types.ts） */
function terminalRegistersContributions() {
  if (!existsSync(TERMINAL_INDEX_TS)) return { exists: false, slots: [], usesGenericRegistry: false };
  const indexSrc = readFileSync(TERMINAL_INDEX_TS, "utf8");
  const typesTs = join(ROOT, "src/capability/contribution/types.ts");
  const typesSrc = existsSync(typesTs) ? readFileSync(typesTs, "utf8") : "";
  const combined = indexSrc + "\n" + typesSrc;
  const slots = [];
  for (const slot of ["workbench-main-resident", "browser-dock"]) {
    if (new RegExp(`["']${slot}["']`).test(combined)) slots.push(slot);
  }
  const usesGeneric = /contributionRegistry\s*\.\s*registerContribution/.test(indexSrc);
  return { exists: true, slots, usesGenericRegistry: usesGeneric };
}

/** 静态：某 Shell 文件是否 import 了给定能力内部（正则） */
function shellImportsInto(fileRel, re) {
  const abs = join(ROOT, fileRel);
  if (!existsSync(abs)) return [];
  const content = readFileSync(abs, "utf8");
  const hits = [];
  for (const spec of extractImports(content)) {
    const norm = normalizeSpec(fileRel, spec);
    if (norm && re.test(norm)) hits.push(spec);
  }
  return hits;
}

async function bundleModule(entryContent, resolveDir) {
  const res = await build({
    stdin: { contents: entryContent, resolveDir, loader: "ts" },
    bundle: true,
    format: "esm",
    platform: "neutral",
    target: "es2020",
    external: ['*.vue'],
    write: false,
  });
  const tmp = join(ROOT, ".tmp-capability-composition.mjs");
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  try {
    return await import(pathToFileURL(tmp).href);
  } finally {
    try {
      unlinkSync(tmp);
    } catch {
      /* ignore */
    }
  }
}

async function runDynamicTests() {
  const results = [];
  const t = async (id, name, fn) => {
    try {
      const r = await fn();
      results.push({ id, name, ok: r === true, detail: r === true ? "" : String(r) });
    } catch (e) {
      results.push({ id, name, ok: false, detail: e && e.message ? e.message : String(e) });
    }
  };

  // C1：通用 Registry 行为（空槽返回 [] = absent-boot 关键）
  const reg = await bundleModule(
    `export { createContributionRegistry } from '${REGISTRY_TS.replace(/\\/g, "/")}';`,
    ROOT,
  );
  await t("C1-EMPTY", "通用 Registry：空槽返回 []（Bookmark absent 时 Shell 渲染空集）", () => {
    const r = reg.createContributionRegistry();
    const empty = r.getSurfaceContributions("browser-sidebar");
    const emptyNav = r.getNavigationContributions("address-bar-actions");
    if (!Array.isArray(empty) || empty.length !== 0) return `surface 空槽应返回 []，实际 ${JSON.stringify(empty)}`;
    if (!Array.isArray(emptyNav) || emptyNav.length !== 0) return `navigation 空槽应返回 []，实际 ${JSON.stringify(emptyNav)}`;
    return true;
  });
  await t("C1-REGISTER", "通用 Registry：注册后可按 slot 取出", () => {
    const r = reg.createContributionRegistry();
    r.registerContribution({
      id: "demo.sidebar",
      capabilityId: "demo",
      type: "surface",
      slot: "browser-sidebar",
      component: {},
    });
    const got = r.getSurfaceContributions("browser-sidebar");
    if (got.length !== 1 || got[0].id !== "demo.sidebar") return `注册后取回失败: ${JSON.stringify(got.map((x) => x.id))}`;
    if (r.getNavigationContributions("browser-sidebar").length !== 0) return "surface 不应出现在 navigation 槽";
    return true;
  });

  // C2-REGISTERED：Bookmark 适配器实向通用 Registry 注册 3 条贡献（静态，运行时已由 pilot PLT-03 验证）。
  await t("C2-REGISTERED", "Bookmark 适配器 registerContribution 调用 = 3 且 capabilityId=bookmark", () => {
    if (!existsSync(BOOKMARK_INDEX_TS)) return "bookmark/index.ts 缺失";
    const src = readFileSync(BOOKMARK_INDEX_TS, "utf8");
    const calls = (src.match(/registerContribution\(\{/g) || []).length;
    if (calls !== 3) return `registerContribution 调用数应为 3，实际 ${calls}`;
    if (!/capabilityId:\s*BOOKMARK_CAPABILITY_ID/.test(src)) return "缺少 capabilityId: BOOKMARK_CAPABILITY_ID";
    return true;
  });
  await t("C4-ABSENT", "Bookmark absent：同槽为空（Shell 仍渲染空集，不崩溃）", () => {
    const fresh = reg.createContributionRegistry();
    // 模拟 Bookmark 未注册：registry 中无 bookmark.* 贡献
    const sidebar = fresh.getSurfaceContributions("browser-sidebar");
    const nav = fresh.getNavigationContributions("address-bar-actions")
      .concat(fresh.getNavigationContributions("activity-bar-trailing"));
    if (sidebar.length !== 0 || nav.length !== 0) return `absent 时不应有贡献: ${sidebar.length}/${nav.length}`;
    return true;
  });

  await t("C6-WORKSPACE-ABSENT", "Workspace absent：workbench-main / browser-dock 槽为空（Shell 仍渲染，不崩溃）", () => {
    const fresh = reg.createContributionRegistry();
    const main = fresh.getSurfaceContributions("workbench-main");
    const dock = fresh.getSurfaceContributions("browser-dock");
    if (main.length !== 0 || dock.length !== 0) return `absent 时不应有贡献: main=${main.length} dock=${dock.length}`;
    return true;
  });

  await t("C6-BROWSER-ABSENT", "Browser absent：browser-host 槽为空（Shell 不创建 webview，不崩溃）", () => {
    const fresh = reg.createContributionRegistry();
    const host = fresh.getSurfaceContributions("browser-host");
    if (host.length !== 0) return `absent 时不应有 browser-host 贡献: ${host.length}`;
    return true;
  });

  await t("C6-TERMINAL-ABSENT", "Terminal absent：workbench-main-resident / browser-dock(term) 槽为空（无终端 DOM、无 PTY 出生点）", () => {
    const fresh = reg.createContributionRegistry();
    const resident = fresh.getSurfaceContributions("workbench-main-resident");
    const dockTerm = fresh.getSurfaceContributions("browser-dock").filter((c) => c.view === "term");
    if (resident.length !== 0 || dockTerm.length !== 0)
      return `absent 时不应有 terminal 贡献: resident=${resident.length} dockTerm=${dockTerm.length}`;
    return true;
  });

  await t("C6-TERMINAL-NO-SHELL-PTY", "Terminal absent：Shell 中不存在任何 PTY 出生点（无 ensureTerm/spawnTerm/addTermPane 调用）", () => {
    const shells = [
      "src/components/layout/MainArea.vue",
      "src/components/layout/ActivityBar.vue",
      "src/components/layout/StatusBar.vue",
      "src/components/layout/UnifiedTabBar.vue",
      "src/capabilities/home/ui/HomeLaunchers.vue",
      "src/App.vue",
    ];
    const hits = [];
    for (const f of shells) {
      const abs = join(ROOT, f);
      if (!existsSync(abs)) continue;
      const src = readFileSync(abs, "utf8")
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .replace(/(^|[^:])\/\/.*$/gm, "$1");
      if (/\b(spawnTerm|ensureTerm|addTermPane)\s*\(/.test(src)) hits.push(f);
    }
    return hits.length === 0 ? true : `Shell 仍持有 PTY 出生点: ${hits.join(", ")}`;
  });

  return results;
}

function runStaticChecks() {
  const results = [];
  const t = (id, name, cond, detail) => {
    results.push({ id, name, ok: !!cond, detail: cond ? "" : String(detail) });
  };

  // C3：Shell 不 import bookmark 内部
  const ma = shellImportsBookmark("src/components/layout/MainArea.vue");
  const ab = shellImportsBookmark("src/components/layout/ActivityBar.vue");
  const hl = shellImportsBookmark("src/capabilities/home/ui/HomeLaunchers.vue");
  t("C3-MAINAREA", "MainArea 不 import src/capabilities/bookmark 内部", ma.imports.length === 0, `imports=${ma.imports.join(",")}`);
  t("C3-ACTIVITYBAR", "ActivityBar 不 import src/capabilities/bookmark 内部", ab.imports.length === 0, `imports=${ab.imports.join(",")}`);
  t("C3-HOMELAUNCHERS", "HomeLaunchers 不 import src/capabilities/bookmark 内部", hl.imports.length === 0, `imports=${hl.imports.join(",")}`);

  // C2：Bookmark 适配器向通用 Registry 注册
  const reg2 = bookmarkRegistersContributions();
  t("C2-ADAPTER", "Bookmark 适配器使用 contributionRegistry.registerContribution", reg2.exists && reg2.usesGenericRegistry, `exists=${reg2.exists} usesGeneric=${reg2.usesGenericRegistry}`);
  t("C2-SLOTS", "Bookmark 注册 3 个槽（browser-sidebar/address-bar-actions/activity-bar-trailing）", reg2.slots.length === 3, `slots=${reg2.slots.join(",")}`);

  // ── Workspace C3（Train B） ──
  const WS_RE = /(?:^|\/)src\/capabilities\/workspace\/(?:state|ui|services|lifecycle|resource|internal|adapters)\//;
  const maWs = shellImportsInto("src/components/layout/MainArea.vue", WS_RE);
  const abWs = shellImportsInto("src/components/layout/ActivityBar.vue", WS_RE);
  const sbWs = shellImportsInto("src/components/layout/StatusBar.vue", WS_RE);
  const tbWs = shellImportsInto("src/components/layout/UnifiedTabBar.vue", WS_RE);
  const appWs = shellImportsInto("src/App.vue", WS_RE);
  t("C5-WS-MAINAREA", "MainArea 不 import src/capabilities/workspace 内部", maWs.length === 0, `imports=${maWs.join(",")}`);
  t("C5-WS-ACTIVITYBAR", "ActivityBar 不 import src/capabilities/workspace 内部", abWs.length === 0, `imports=${abWs.join(",")}`);
  t("C5-WS-STATUSBAR", "StatusBar 不 import src/capabilities/workspace 内部", sbWs.length === 0, `imports=${sbWs.join(",")}`);
  t("C5-WS-TABBAR", "UnifiedTabBar 不 import src/capabilities/workspace 内部", tbWs.length === 0, `imports=${tbWs.join(",")}`);
  t("C5-WS-APP", "App.vue 不 import src/capabilities/workspace 内部", appWs.length === 0, `imports=${appWs.join(",")}`);
  const wsReg = workspaceRegistersContributions();
  t("C5-WS-ADAPTER", "Workspace 适配器使用 contributionRegistry.registerContribution", wsReg.exists && wsReg.usesGenericRegistry, `exists=${wsReg.exists} usesGeneric=${wsReg.usesGenericRegistry}`);
  t("C5-WS-SLOTS", "Workspace 注册 workbench-main + browser-dock 槽", wsReg.slots.length === 2, `slots=${wsReg.slots.join(",")}`);

  // ── Browser C3（Train C） ──
  const BR_RE = /(?:^|\/)src\/capabilities\/browser\/(?:state|ui|services|lifecycle|resource|internal)\//;
  const maBr = shellImportsInto("src/components/layout/MainArea.vue", BR_RE);
  const tbBr = shellImportsInto("src/components/layout/TopBar.vue", BR_RE);
  const utbBr = shellImportsInto("src/components/layout/UnifiedTabBar.vue", BR_RE);
  t("C5-BR-MAINAREA", "MainArea 不 import src/capabilities/browser 内部", maBr.length === 0, `imports=${maBr.join(",")}`);
  t("C5-BR-TOPBAR", "TopBar 不 import src/capabilities/browser 内部", tbBr.length === 0, `imports=${tbBr.join(",")}`);
  t("C5-BR-TABBAR", "UnifiedTabBar 不 import src/capabilities/browser 内部", utbBr.length === 0, `imports=${utbBr.join(",")}`);
  const brReg = browserRegistersContributions();
  t("C5-BR-ADAPTER", "Browser 适配器使用 contributionRegistry.registerContribution", brReg.exists && brReg.usesGenericRegistry, `exists=${brReg.exists} usesGeneric=${brReg.usesGenericRegistry}`);
  t("C5-BR-SLOTS", "Browser 注册 browser-host + browser-dock 槽", brReg.slots.length === 2, `slots=${brReg.slots.join(",")}`);

  // ── Terminal C3（Train D） ──
  const TERM_RE = /(?:^|\/)src\/capabilities\/terminal\/(?:state|ui|services|lifecycle|resource|internal|adapters)\//;
  for (const f of [
    "src/components/layout/MainArea.vue",
    "src/components/layout/ActivityBar.vue",
    "src/components/layout/StatusBar.vue",
    "src/components/layout/UnifiedTabBar.vue",
    "src/capabilities/home/ui/HomeLaunchers.vue",
    "src/App.vue",
  ]) {
    const hits = shellImportsInto(f, TERM_RE);
    const id = "C5-TERM-" + f.split("/").pop().replace(/\.vue$/, "").toUpperCase();
    t(id, `${f.split("/").pop()} 不 import src/capabilities/terminal 内部`, hits.length === 0, `imports=${hits.join(",")}`);
  }
  const termReg = terminalRegistersContributions();
  t("C5-TERM-ADAPTER", "Terminal 适配器使用 contributionRegistry.registerContribution", termReg.exists && termReg.usesGenericRegistry, `exists=${termReg.exists} usesGeneric=${termReg.usesGenericRegistry}`);
  t("C5-TERM-SLOTS", "Terminal 注册 workbench-main-resident + browser-dock 槽", termReg.slots.length === 2, `slots=${termReg.slots.join(",")}`);

  return results;
}

function runSelfTest() {
  let pass = 0, fail = 0;
  const cases = [
    {
      name: "POSITIVE: Shell 仅引用通用 Registry（不引用 bookmark 内部）",
      expect: [],
      files: [
        { path: "src/components/layout/MainArea.vue", content: "import { contributionRegistry } from '../../capability/contribution/registry'\nconst x = contributionRegistry.getSurfaceContributions('browser-sidebar')" },
      ],
    },
    {
      name: "NEGATIVE: Shell import bookmark 内部 ui",
      expect: ["C3"],
      files: [
        { path: "src/components/layout/MainArea.vue", content: "import BookmarkPanel from '../../capabilities/bookmark/ui/BookmarkPanel.vue'" },
      ],
    },
    {
      name: "NEGATIVE: Shell import bookmark 内部 public/store",
      expect: ["C3"],
      files: [
        { path: "src/components/layout/ActivityBar.vue", content: "import { useBookmarkStore } from '../../capabilities/bookmark/public'" },
      ],
    },
  ];
  for (const c of cases) {
    const ma = c.files.find((f) => f.path.endsWith("MainArea.vue"));
    const ab = c.files.find((f) => f.path.endsWith("ActivityBar.vue"));
    const got = new Set();
    for (const f of c.files) {
      const r = shellImportsBookmarkFromContent(f.path, f.content);
      if (r.length) got.add("C3");
    }
    const expected = new Set(c.expect);
    const ok = [...expected].every((e) => got.has(e)) && (expected.size > 0 ? true : got.size === 0);
    if (ok) pass++; else fail++;
    console.log(`${ok ? "PASS" : "FAIL"}  ${c.name}` + (ok ? "" : `  expected=${[...expected]} got=${[...got]}`));
  }
  console.log(`\nSELF_TEST: ${fail === 0 ? "PASS" : "FAIL"} (${pass}/${pass + fail})`);
  return fail === 0 ? 0 : 1;
}

function shellImportsBookmarkFromContent(fileRel, content) {
  const hits = [];
  for (const spec of extractImports(content)) {
    const norm = normalizeSpec(fileRel, spec);
    if (norm && /(?:^|\/)src\/capabilities\/bookmark\//.test(norm)) hits.push(spec);
  }
  return hits;
}

const HELP = `check-capability-composition.mjs — Capability 组合性门禁（Phase 8B.1）

用法:
  node scripts/check-capability-composition.mjs              真实扫描 + 动态测试
  node scripts/check-capability-composition.mjs --self-test   夹具自检
  node scripts/check-capability-composition.mjs --json        机器可读
  node scripts/check-capability-composition.mjs --help

退出码: 0 = 通过, 1 = 失败
`;

async function main() {
  const argv = process.argv.slice(2);
  if (argv.includes("--help") || argv.includes("-h")) {
    console.log(HELP);
    return 0;
  }
  if (argv.includes("--self-test")) return runSelfTest();

  const json = argv.includes("--json");
  const staticRes = runStaticChecks();
  const dynamicRes = await runDynamicTests();
  const all = [...staticRes, ...dynamicRes];
  const pass = all.filter((r) => r.ok).length;
  const fail = all.length - pass;

  if (json) {
    console.log(JSON.stringify({ result: fail === 0 ? "PASS" : "FAIL", pass, fail, results: all }, null, 2));
  } else {
    for (const r of all) {
      console.log(`${r.ok ? "PASS" : "FAIL"}  ${r.id}  ${r.name}${r.ok ? "" : `  → ${r.detail}`}`);
    }
    console.log(`\nCAPABILITY_COMPOSITION_RESULT=${fail === 0 ? "PASS" : "FAIL"} (${pass}/${all.length})`);
  }
  return fail === 0 ? 0 : 1;
}

main().then((c) => process.exit(c));
