#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-package.mjs — Generic Capability Package Boundary Checker
//
// 单一参数化门禁，取代每个能力各自的边界脚本（check-vault-boundary.mjs、
// packages/*/scripts/check-package.mjs）。能力专属领域逻辑（Vault domain /
// Clipboard B11-1）仍留在各包自身的 scripts/check-*.mjs 中（directive §8）。
//
// 通用规则（参数化，适用于任意能力包）：
//   G-ROOT          包根存在
//   G-MANIFEST      package.json 合法（name/version/private/type/exports）
//   G-EXPORTS       exports 显式且机器限定（仅 expectedExports）
//   G-PUBLIC        公共入口解析且导出 expected 公共符号
//   G-DEEP-IMPORT   外部 deep-import 包内 src = 0
//   G-HOST-INTERNAL 包内依赖 Host 内部实现 = 0（bridge/stores/utils/capability/App.vue/components + forbiddenInPackage）
//   G-UNDECLARED-DEP 包内使用未声明依赖 = 0
//   G-OLD-ABSENT    旧实现路径不存在且无引用
//   G-DUP-IMPL      下沉 symbol 全仓定义恰好 1 处
//   G-SECOND-TRUTH  仅 manifestSubpath 声明该 capability（无第二真源）
//   G-DUP-CONTRIB   贡献描述符导出恰好 1 次（duplicate contribution registration = 0）
//   G-STATE-OWNER   defineStore(storeId) 恰好 1 处
//   G-NATIVE-IN-UI  ui/*.vue 不直连 bridge/invoke
//   G-DEAD-SHIM     包外不重复导出下沉 symbol
//   G-HOST-CONSUMES 宿主经包名消费（非致命 finding；宿主 deep import 计为 G-DEEP-IMPORT）
//
// 用法：
//   node scripts/check-capability-package.mjs --self-test
//   node scripts/check-capability-package.mjs --list
//   node scripts/check-capability-package.mjs --all
//   node scripts/check-capability-package.mjs --capability vault
// ---------------------------------------------------------------------------
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..");

// ---------------------------------------------------------------------------
// Specs（directive §8 参数）
// ---------------------------------------------------------------------------
const SPECS = {
  vault: {
    packageRoot: "packages/capability-vault",
    packageName: "@browser-os/capability-vault",
    capabilityId: "vault",
    manifestSubpath: "src/manifest.ts",
    publicEntry: "src/index.ts",
    expectedExports: [".", "./manifest"],
    contributionId: "vault.main",
    storeId: "vault",
    oldImplementationPaths: ["src/capabilities/vault", "src/utils/vault.mjs"],
    specialRules: {
      sunkSymbols: ["resolveNote", "noteLinks", "searchNotes"],
      publicSymbols: ["createVaultCapability", "vaultContribution", "VAULT_PORTS_KEY"],
      uiDir: "ui",
      contributionDescriptorName: "vaultContribution",
      forbiddenInPackage: [],
    },
  },
  // 预置：Clipboard M2 包当前在主工作树为 WORKTREE_VERIFIED_UNCOMMITTED（DO_NOT_TOUCH）。
  // 一旦其包提交进某 worktree，本 checker 即可用同一参数化逻辑验证，无需新建脚本。
  clipboard: {
    packageRoot: "packages/capability-clipboard",
    packageName: "@browser-os/capability-clipboard",
    capabilityId: "clipboard",
    manifestSubpath: "src/manifest.ts",
    publicEntry: "src/index.ts",
    expectedExports: [".", "./manifest"],
    contributionId: "clipboard.main",
    storeId: "clipboard",
    oldImplementationPaths: ["src/capabilities/clipboard", "src/stores/useClipboardStore.ts"],
    specialRules: {
      sunkSymbols: [],
      publicSymbols: ["createClipboardCapability", "clipboardContribution", "CLIPBOARD_PORTS_KEY"],
      uiDir: "ui",
      contributionDescriptorName: "clipboardContribution",
      // B11-1 红线：剪贴板历史禁止落 localStorage（领域不变量，仍由各包自身 check 兜底）
      forbiddenInPackage: ["localStorage"],
    },
  },
};

// ---------------------------------------------------------------------------
// 通用工具
// ---------------------------------------------------------------------------
function walkDisk(dir, out) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walkDisk(p, out);
    else if (/\.(ts|vue|mjs|js)$/.test(name) && !/\.d\.ts$/.test(name)) out.add(p);
  }
  return out;
}
function parseSpecifiers(code) {
  const specs = [];
  const strip = code.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|\s)\/\/[^\n]*/g, "$1");
  const re = /(?:import|export)[^;]*?from\s*["']([^"']+)["']/g;
  let m;
  while ((m = re.exec(strip))) specs.push(m[1]);
  return specs;
}
function bareModule(spec) {
  if (!spec.startsWith(".") && !spec.startsWith("/")) {
    return spec.startsWith("@") ? spec.split("/").slice(0, 2).join("/") : spec.split("/")[0];
  }
  return null;
}
function safeJson(raw) {
  try { return JSON.parse(raw); } catch { return null; }
}

// ---------------------------------------------------------------------------
// 真实扫描：包 src + 包根关键文件 + 宿主 src
// ---------------------------------------------------------------------------
function collectReal(spec) {
  const files = new Map();
  const srcAbs = join(REPO_ROOT, spec.packageRoot, "src");
  for (const abs of walkDisk(srcAbs, new Set())) {
    const r = relative(REPO_ROOT, abs).split("\\").join("/");
    if (r.endsWith(".d.ts")) continue;
    files.set(r, readFileSync(abs, "utf8"));
  }
  for (const rel of [
    join(spec.packageRoot, "package.json"),
    spec.manifestSubpath ? join(spec.packageRoot, spec.manifestSubpath) : null,
    join(spec.packageRoot, spec.publicEntry),
  ].filter(Boolean)) {
    const abs = join(REPO_ROOT, rel);
    if (existsSync(abs)) files.set(rel.split("\\").join("/"), readFileSync(abs, "utf8"));
  }
  for (const abs of walkDisk(join(REPO_ROOT, "src"), new Set())) {
    const r = relative(REPO_ROOT, abs).split("\\").join("/");
    if (r.endsWith(".d.ts")) continue;
    files.set(r, readFileSync(abs, "utf8"));
  }
  return files;
}

// ---------------------------------------------------------------------------
// 通用规则实现（files 可注入，便于 self-test）
// ---------------------------------------------------------------------------
function runSpec(spec, injected) {
  const files = injected || collectReal(spec);
  const fail = [], finding = [];
  const f = (c, m) => fail.push(`[${c}] ${m}`);
  const fg = (c, m) => finding.push(`[${c}] ${m}`);

  const sr = spec.packageRoot;
  const srcRel = `${sr}/src`;
  const pkgJsonRel = `${sr}/package.json`;
  const idxRel = `${sr}/${spec.publicEntry}`;
  const manRel = spec.manifestSubpath ? `${sr}/${spec.manifestSubpath}` : null;
  const srRules = spec.specialRules || {};
  const sunk = srRules.sunkSymbols || [];
  const pub = srRules.publicSymbols || [];
  const uiDir = srRules.uiDir || "ui";
  const storeId = spec.storeId;
  const contribName = srRules.contributionDescriptorName || `${spec.capabilityId}Contribution`;
  const forbidden = srRules.forbiddenInPackage || [];
  const oldPaths = spec.oldImplementationPaths || [];

  // G-ROOT
  if (!existsSync(join(REPO_ROOT, sr))) {
    f("G-ROOT", `包根 ${sr} 不存在`);
    return { fail, finding };
  }

  // G-MANIFEST + G-EXPORTS + G-UNDECLARED-DEP（package.json）
  const pjRaw = files.get(pkgJsonRel);
  const pj = pjRaw ? safeJson(pjRaw) : null;
  if (!pj) f("G-MANIFEST", "package.json 缺失/非法");
  else {
    if (!pj.name || pj.name !== spec.packageName) f("G-MANIFEST", `包名应为 ${spec.packageName}，实际 ${pj.name}`);
    if (!pj.version) f("G-MANIFEST", "缺 version");
    if (!pj.private) f("G-MANIFEST", "缺 private");
    if (pj.type !== "module") f("G-MANIFEST", "type 应为 module");
    if (!pj.exports) f("G-MANIFEST", "缺 exports");
    for (const k of Object.keys(pj.exports || {}))
      if (!spec.expectedExports.includes(k)) f("G-EXPORTS", `exports 暴露禁止条目：${k}`);
    const declared = new Set([
      ...Object.keys(pj.dependencies || {}),
      ...Object.keys(pj.peerDependencies || {}),
    ]);
    const builtins = new Set([
      "node:fs", "node:path", "node:url", "node:assert/strict", "node:module", "node:util",
    ]);
    for (const [rel, code] of files) {
      if (!rel.startsWith(srcRel + "/")) continue;
      for (const s of parseSpecifiers(code)) {
        const b = bareModule(s);
        if (b && !declared.has(b) && !builtins.has(b)) f("G-UNDECLARED-DEP", `${rel} 使用未声明依赖：${b}`);
      }
    }
  }

  // G-PUBLIC
  const idx = files.get(idxRel) || "";
  if (!idx) f("G-PUBLIC", `${idxRel} 缺失`);
  else for (const sym of pub) {
    if (!new RegExp(`export\\s+(?:\\{[^}]*\\b${sym}\\b|\\w*[\\s*]*\\b${sym}\\b)`).test(idx) && !idx.includes(sym))
      f("G-PUBLIC", `${idxRel} 未导出公共符号：${sym}`);
  }

  // G-DEEP-IMPORT（外部 deep import 包内 src）
  const deepRe = new RegExp(`${sr.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}/src/`);
  for (const [rel, code] of files) {
    if (rel.startsWith(sr + "/") || rel.startsWith(srcRel + "/")) continue;
    for (const s of parseSpecifiers(code))
      if (deepRe.test(s)) f("G-DEEP-IMPORT", `${rel} deep-import 包内 src：${s}`);
  }

  // G-HOST-INTERNAL（包内依赖 Host 内部实现）
  const hostForbidden = [
    /(^|\.\.\/)+bridge(\.ts|\.js|\.mjs)?$/,
    /(^|\.\.\/)+stores\//,
    /(^|\.\.\/)+utils\//,
    /(^|\.\.\/)+capability\//,
    /(^|\.\.\/)+App\.vue$/,
    /(^|\.\.\/)+components\//,
    /^\.{0,2}\/?(src\/)?bridge(\.ts)?$/,
  ];
  for (const [rel, code] of files) {
    if (!rel.startsWith(srcRel + "/")) continue;
    for (const s of parseSpecifiers(code)) {
      const resolved = s.replace(/\.[^./]+$/, "");
      for (const re of hostForbidden) if (re.test(s) || re.test(resolved)) f("G-HOST-INTERNAL", `${rel} 依赖 Host 内部：${s}`);
      for (const fb of forbidden) if (new RegExp(`\\b${fb}\\b`).test(code)) f("G-HOST-INTERNAL", `${rel} 含禁止 API：${fb}`);
    }
  }

  // G-OLD-ABSENT
  for (const op of oldPaths) {
    if (existsSync(join(REPO_ROOT, op))) f("G-OLD-ABSENT", `旧实现路径仍存在：${op}`);
    const opRe = new RegExp(op.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
    for (const [rel, code] of files) {
      if (rel.startsWith(sr + "/")) continue;
      if (opRe.test(code)) f("G-OLD-ABSENT", `${rel} 仍引用旧路径 ${op}`);
    }
  }

  // G-DUP-IMPL（下沉 symbol 定义恰好 1 处）
  for (const sym of sunk) {
    let sites = 0;
    for (const [, code] of files)
      if (new RegExp(`(?:export\\s+)?function\\s+${sym}\\s*\\(`).test(code) || new RegExp(`const\\s+${sym}\\s*=`).test(code)) sites++;
    if (sites !== 1) f("G-DUP-IMPL", `${sym} 实现出现 ${sites} 处（应为 1）`);
  }

  // G-SECOND-TRUTH（仅 manifestSubpath 声明该 capability）
  if (manRel) {
    let manifests = 0;
    for (const [rel, code] of files) {
      if (rel.startsWith(srcRel + "/") || rel.startsWith(sr + "/")) {
        if (new RegExp(`id:\\s*["']${spec.capabilityId}["']`).test(code) && /CapabilityDefinition/.test(code)) manifests++;
      }
    }
    if (manifests > 1) f("G-SECOND-TRUTH", `检测到 ${manifests} 处 capability 定义（应为 1，位于 ${manRel}）`);
  }

  // G-DUP-CONTRIB（贡献描述符重复导出 = 0；缺失由 G-PUBLIC 兜底）
  if (idx) {
    const exportsCount = (idx.match(new RegExp(`export[^;]*\\b${contribName}\\b`, "g")) || []).length;
    if (exportsCount > 1) f("G-DUP-CONTRIB", `${contribName} 导出出现 ${exportsCount} 次（应为 1）`);
  }

  // G-STATE-OWNER
  let storeCount = 0;
  for (const [rel, code] of files)
    if (rel.startsWith(srcRel + "/") && new RegExp(`defineStore\\(\\s*["']${storeId}["']`).test(code)) storeCount++;
  if (storeCount !== 1) f("G-STATE-OWNER", `defineStore('${storeId}') 出现 ${storeCount} 次（应为 1）`);

  // G-NATIVE-IN-UI
  for (const [rel, code] of files)
    if (rel.startsWith(`${srcRel}/${uiDir}/`) && /\.(vue|ts|js|mjs)$/.test(rel) && /(^|[^.\w])bridge\.|invoke\(/.test(code))
      f("G-NATIVE-IN-UI", `${rel} UI 层直连 native`);

  // G-DEAD-SHIM（包外再导出下沉 symbol）
  if (sunk.length) {
    for (const [rel, code] of files) {
      if (rel.startsWith(sr + "/")) continue;
      if (code.includes(spec.packageName) && sunk.some((sym) => /export/.test(code) && new RegExp(`\\b${sym}\\b`).test(code)))
        f("G-DEAD-SHIM", `${rel} 包外再导出下沉 symbol`);
    }
  }

  // G-HOST-CONSUMES（非致命）
  let hostConsumes = 0, hostDeep = 0;
  for (const [rel, code] of files) {
    if (rel.startsWith(sr + "/") || rel.startsWith(srcRel + "/")) continue;
    for (const s of parseSpecifiers(code)) {
      if (s.startsWith(spec.packageName)) hostConsumes++;
      if (s.includes(`${sr}/src/`)) hostDeep++;
    }
  }
  if (hostConsumes === 0) fg("G-HOST-CONSUMES", `未发现宿主经包名消费 ${spec.packageName}（请确认已接线）`);
  if (hostDeep > 0) f("G-DEEP-IMPORT", `宿主存在 ${hostDeep} 处相对 deep import（应改用 ${spec.packageName}）`);

  return { fail, finding };
}

// ---------------------------------------------------------------------------
// SELF-TEST（合成 fixture 证明每类违规均可检出；silent-skip 由检测断言保证）
// ---------------------------------------------------------------------------
function baseFiles(spec) {
  const m = new Map();
  const sr = spec.packageRoot, srcRel = `${sr}/src`;
  m.set(`${sr}/package.json`, JSON.stringify({
    name: spec.packageName,
    version: "0.1.0",
    private: true,
    type: "module",
    exports: Object.fromEntries(spec.expectedExports.map((e) => [e, `./${spec.publicEntry}`])),
    dependencies: { vue: "^3.4.0" },
    peerDependencies: { vue: ">=3.4.0" },
    scripts: { check: "tsc --noEmit" },
  }));
  m.set(`${sr}/${spec.publicEntry}`,
    `export { ${spec.specialRules.publicSymbols.filter((s) => s !== spec.specialRules.contributionDescriptorName).join(", ")} } from './ports';\n` +
    `export const ${spec.specialRules.contributionDescriptorName} = {};\n`);
  m.set(`${srcRel}/ports/index.ts`,
    `export const VAULT_PORTS_KEY = Symbol('x');\nexport function ${spec.specialRules.publicSymbols[0]}(){}\n`);
  m.set(`${srcRel}/state/useStore.ts`,
    `import { ref } from 'vue';\nexport const useX = defineStore('${spec.storeId}', () => ({ p: ref('') }));\n`);
  m.set(`${srcRel}/ui/Panel.vue`, "<template><div/></template>\n");
  m.set(`${sr}/${spec.manifestSubpath}`,
    `import type { CapabilityDefinition } from './types';\nexport const ${spec.capabilityId}Manifest: CapabilityDefinition = { id: '${spec.capabilityId}', name: 'x' };\n`);
  if (spec.specialRules.sunkSymbols.length)
    m.set(`${srcRel}/internal/impl.mjs`,
      spec.specialRules.sunkSymbols.map((s) => `export function ${s}(){}\n`).join(""));
  return m;
}

function selfTest() {
  const spec = SPECS.vault;
  const sr = spec.packageRoot, srcRel = `${sr}/src`;
  const base = () => baseFiles(spec);
  const cases = [];

  cases.push({ id: "ST-01", name: "valid → 零失败", files: base(), expect: [] });
  cases.push({
    id: "ST-02", name: "包根缺失 → G-ROOT", files: new Map(),
    specOverride: { ...spec, packageRoot: "packages/__nonexistent__" }, expect: ["G-ROOT"],
  });
  {
    const m = base(); const pj = JSON.parse(m.get(`${sr}/package.json`)); delete pj.version;
    m.set(`${sr}/package.json`, JSON.stringify(pj));
    cases.push({ id: "ST-03", name: "manifest 缺 version → G-MANIFEST", files: m, expect: ["G-MANIFEST"] });
  }
  {
    const m = base(); const pj = JSON.parse(m.get(`${sr}/package.json`));
    pj.exports["./internal"] = "./src/internal/impl.mjs";
    m.set(`${sr}/package.json`, JSON.stringify(pj));
    cases.push({ id: "ST-04", name: "exports 泄露 internal → G-EXPORTS", files: m, expect: ["G-EXPORTS"] });
  }
  {
    const m = base();
    m.set(`${sr}/${spec.publicEntry}`, "export const x = 1;\n");
    cases.push({ id: "ST-05", name: "公共入口缺符号 → G-PUBLIC", files: m, expect: ["G-PUBLIC"] });
  }
  {
    const m = base();
    m.set("src/components/MainArea.vue", `<script>import { x } from "../../${sr}/src/state/useStore";</script>\n`);
    cases.push({ id: "ST-06", name: "外部 deep import 包 src → G-DEEP-IMPORT", files: m, expect: ["G-DEEP-IMPORT"] });
  }
  {
    const m = base();
    m.set(`${srcRel}/state/useStore.ts`, `import { bridge } from '../../../bridge';\nexport const useX = defineStore('${spec.storeId}', () => ({}));\n`);
    cases.push({ id: "ST-07", name: "包内依赖 bridge → G-HOST-INTERNAL", files: m, expect: ["G-HOST-INTERNAL"] });
  }
  {
    const m = base();
    m.set(`${srcRel}/state/useStore.ts`, `import { z } from 'undeclared-pkg';\nexport const useX = defineStore('${spec.storeId}', () => ({}));\n`);
    cases.push({ id: "ST-08", name: "包内未声明依赖 → G-UNDECLARED-DEP", files: m, expect: ["G-UNDECLARED-DEP"] });
  }
  {
    const m = base();
    m.set("src/App.vue", `import { x } from "src/utils/vault.mjs";\n`);
    cases.push({ id: "ST-09", name: "引用旧路径 → G-OLD-ABSENT", files: m, expect: ["G-OLD-ABSENT"] });
  }
  {
    const m = base();
    m.set(`${srcRel}/state/dup.ts`, `export function ${spec.specialRules.sunkSymbols[0]}(){}\n`);
    cases.push({ id: "ST-10", name: "下沉 symbol 重复实现 → G-DUP-IMPL", files: m, expect: ["G-DUP-IMPL"] });
  }
  {
    const m = base();
    m.set(`${srcRel}/contract.ts`, `import type { CapabilityDefinition } from './types';\nexport const other: CapabilityDefinition = { id: '${spec.capabilityId}' };\n`);
    cases.push({ id: "ST-11", name: "第二真源 manifest → G-SECOND-TRUTH", files: m, expect: ["G-SECOND-TRUTH"] });
  }
  {
    const m = base();
    m.set(`${sr}/${spec.publicEntry}`, m.get(`${sr}/${spec.publicEntry}`) + `\nexport const ${spec.specialRules.contributionDescriptorName} = {};\n`);
    cases.push({ id: "ST-12", name: "贡献描述符重复导出 → G-DUP-CONTRIB", files: m, expect: ["G-DUP-CONTRIB"] });
  }
  {
    const m = base();
    m.set(`${srcRel}/ui/Shadow.vue`, `export const s = defineStore('${spec.storeId}', () => ({}));\n`);
    cases.push({ id: "ST-13", name: "双 state owner → G-STATE-OWNER", files: m, expect: ["G-STATE-OWNER"] });
  }
  {
    const m = base();
    m.set(`${srcRel}/ui/Panel.vue`, "<script>bridge.x();</script>\n");
    cases.push({ id: "ST-14", name: "UI 直连 native → G-NATIVE-IN-UI", files: m, expect: ["G-NATIVE-IN-UI"] });
  }
  {
    const m = base();
    m.set("src/utils/shim.mjs", `export { ${spec.specialRules.sunkSymbols[0]} } from '${spec.packageName}';\n`);
    cases.push({ id: "ST-15", name: "包外再导出下沉 symbol → G-DEAD-SHIM", files: m, expect: ["G-DEAD-SHIM"] });
  }

  let pass = 0;
  for (const c of cases) {
    const sp = c.specOverride || spec;
    const { fail } = runSpec(sp, c.files);
    const got = fail.map((s) => s.match(/^\[([A-Z0-9-]+)\]/)[1]).filter((x) => c.expect.includes(x));
    // 严格匹配：期望值全部出现且无非期望失败
    const ok = c.expect.every((e) => fail.some((fl) => fl.includes(e))) &&
      fail.every((fl) => c.expect.some((e) => fl.includes(e)));
    console.log(`${ok ? "PASS" : "FAIL"} ${c.id} ${c.name}${ok ? "" : ` → got=${JSON.stringify(fail)}`}`);
    if (ok) pass++;
  }
  console.log(`SELF-TEST: ${pass}/${cases.length}`);
  if (pass !== cases.length) process.exit(1);
}

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------
const argv = process.argv.slice(2);
if (argv.includes("--self-test")) { selfTest(); process.exit(0); }
if (argv.includes("--list")) { console.log(Object.keys(SPECS).join("\n")); process.exit(0); }

const capIdx = argv.indexOf("--capability");
const caps = capIdx >= 0
  ? [argv[capIdx + 1]]
  : (argv.includes("--all") ? Object.keys(SPECS) : Object.keys(SPECS).filter((k) => existsSync(join(REPO_ROOT, SPECS[k].packageRoot))));

let totalFail = 0;
for (const k of caps) {
  const spec = SPECS[k];
  if (!existsSync(join(REPO_ROOT, spec.packageRoot))) {
    console.log(`SKIP ${k} (packageRoot 不存在 —— 若已提交请确认 worktree)`);
    continue;
  }
  const { fail, finding } = runSpec(spec, null);
  console.log(`${spec.packageName}: fail=${fail.length} finding=${finding.length}`);
  for (const x of fail) console.log(`  FAIL ${x}`);
  for (const x of finding) console.log(`  FG   ${x}`);
  totalFail += fail.length;
}
console.log(`PACKAGE_CHECKER fail=${totalFail}`);
process.exit(totalFail ? 1 : 0);
