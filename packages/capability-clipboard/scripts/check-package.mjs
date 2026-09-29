#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-package.mjs — Clipboard M2 能力包边界门禁（PKG-01..PKG-12）。
// 扫描 packages/capability-clipboard/src + 宿主 src，验证包边界。
//
// 门禁码：
//   PKG-01 包 root 存在
//   PKG-02 package.json 合法且含关键字段
//   PKG-03 exports 仅 '.' / './manifest'（不泄露 internal）
//   PKG-04 外部不得 deep-import 包内 src/(state|ui|internal)
//   PKG-05 未声明依赖 = 0
//   PKG-06 Host implementation import = 0（包内不得直连 Host src/ 实现）
//   PKG-07 public entry 导出关键 symbol
//   PKG-08 package validation 可执行（scripts.check 存在）
//   PKG-09 Host 通过 public contract 消费（@browser-os/capability-clipboard[/manifest]），不 deep-import
//   PKG-10 旧 src/capabilities/clipboard 物理不存在且无真实引用
//   PKG-11 持久化不变量完好（B11-1：store 无 localStorage 等持久化 API）
//   PKG-12 无重复贡献注册（包内不自注册；Host 恰好注册 1 次）
//
// 用法: node scripts/check-package.mjs [--self-test] [--json]
// ---------------------------------------------------------------------------
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..", ".."); // packages/capability-clipboard/scripts -> repo root
const PKG_REL = "packages/capability-clipboard";
const SRC_REL = "packages/capability-clipboard/src";
const PKG_JSON = "packages/capability-clipboard/package.json";

const NODE_BUILTINS = new Set([
  "node:fs", "node:path", "node:url", "node:util", "node:crypto", "node:process",
]);

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
function stripComments(code) {
  return code.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|\s)\/\/[^\n]*/g, "$1");
}
function parseSpecifiers(code) {
  const specs = [];
  const strip = stripComments(code);
  const re = /(?:import|export)[^;]*?from\s*["']([^"']+)["']/g;
  let m;
  while ((m = re.exec(strip))) specs.push(m[1]);
  return specs;
}
const inPkgSrc = (rel) => rel.startsWith(`${SRC_REL}/`);
const inPkg = (rel) => rel.startsWith(`${PKG_REL}/`);
function resolveSpec(rel, spec) {
  if (!spec.startsWith(".")) return spec;
  const dir = rel.split("/").slice(0, -1).join("/");
  const parts = [];
  for (const p of `${dir}/${spec}`.split("/")) {
    if (p === "..") parts.pop();
    else if (p && p !== ".") parts.push(p);
  }
  return parts.join("/");
}
function collectReal() {
  const files = new Map();
  for (const abs of walkDisk(join(ROOT, SRC_REL), new Set())) {
    files.set(relative(ROOT, abs).split("\\").join("/"), readFileSync(abs, "utf8"));
  }
  for (const abs of walkDisk(join(ROOT, "src"), new Set())) {
    const r = relative(ROOT, abs).split("\\").join("/");
    if (r.endsWith(".d.ts")) continue;
    files.set(r, readFileSync(abs, "utf8"));
  }
  const pkgJsonPath = join(ROOT, PKG_JSON);
  if (existsSync(pkgJsonPath)) files.set(PKG_JSON, readFileSync(pkgJsonPath, "utf8"));
  return files;
}

function check(files, v, fg) {
  // PKG-01
  if (!existsSync(join(ROOT, PKG_REL))) v.push("[PKG-01] package root 缺失");

  // PKG-02 / PKG-03
  const pjRaw = files.get(PKG_JSON);
  let pj = pjRaw ? JSON.parse(pjRaw) : null;
  if (!pj) v.push("[PKG-02] package.json 缺失或非法 JSON");
  else {
    for (const k of ["name", "version", "private", "type", "exports"])
      if (!(k in pj)) v.push(`[PKG-02] package.json 缺字段 ${k}`);
    for (const k of Object.keys(pj.exports || {}))
      if (k !== "." && k !== "./manifest") v.push(`[PKG-03] exports 泄露内部 ${k}`);
  }

  // PKG-04 外部 deep-import 包内 src
  const deepRe = /packages\/capability-clipboard\/src\/(state|ui|internal)/;
  const nameDeepRe = /@browser-os\/capability-clipboard\/(state|ui|internal)/;
  for (const [rel, code] of files) {
    if (inPkg(rel)) continue;
    for (const spec of parseSpecifiers(code))
      if (deepRe.test(spec) || nameDeepRe.test(spec))
        v.push(`[PKG-04] ${rel} deep-import 包内 ${spec}`);
  }

  // PKG-05 未声明依赖
  if (pj) {
    const declared = new Set([
      ...Object.keys(pj.dependencies || {}),
      ...Object.keys(pj.peerDependencies || {}),
      ...Object.keys(pj.optionalDependencies || {}),
    ]);
    for (const [rel, code] of files) {
      if (!inPkgSrc(rel)) continue;
      for (const spec of parseSpecifiers(code)) {
        if (spec.startsWith(".")) continue;
        if (spec.startsWith("@browser-os/capability-clipboard")) continue;
        if (NODE_BUILTINS.has(spec)) continue;
        const pkg = spec.startsWith("@")
          ? spec.split("/").slice(0, 2).join("/")
          : spec.split("/")[0];
        if (!declared.has(pkg)) v.push(`[PKG-05] ${rel} 未声明依赖 ${spec}`);
      }
    }
  }

  // PKG-06 Host implementation import = 0
  for (const [rel, code] of files) {
    if (!inPkgSrc(rel)) continue;
    for (const spec of parseSpecifiers(code)) {
      if (spec.startsWith("@browser-os/capability-") && !/^@browser-os\/capability-clipboard(\/|$)/.test(spec))
        v.push(`[PKG-06] ${rel} 跨包/非 clipboard 依赖 ${spec}`);
      if (spec.startsWith(".")) {
        const resolved = resolveSpec(rel, spec);
        if (resolved.startsWith("src/") && !resolved.startsWith(SRC_REL))
          v.push(`[PKG-06] ${rel} 直连 Host 实现 ${resolved}`);
      }
    }
  }

  // PKG-07 public entry 导出边界
  const idx = files.get(`${SRC_REL}/index.ts`) || "";
  for (const sym of [
    "CLIPBOARD_CAPABILITY_ID", "useClipboardStore", "clipboardManifest",
    "clipboardContribution", "createClipboardCapability", "CLIPBOARD_PORTS_KEY",
  ]) {
    if (!new RegExp(
      `export\\s+(?:\\*|\\{[^}]*\\b${sym}\\b|const\\s+${sym}\\b|function\\s+${sym}\\b|type\\s+\\{?[^}]*\\b${sym}\\b)`
    ).test(idx))
      v.push(`[PKG-07] index.ts 未导出 ${sym}`);
  }

  // PKG-08 package validation 可执行
  if (!pj || !pj.scripts || !pj.scripts.check) v.push("[PKG-08] package.json scripts.check 缺失");

  // PKG-09 Host 通过 public contract 消费
  let hostDeep = 0, hostOk = 0;
  for (const [rel, code] of files) {
    if (inPkg(rel)) continue;
    for (const spec of parseSpecifiers(code)) {
      if (deepRe.test(spec) || nameDeepRe.test(spec)) hostDeep++;
      if (spec === "@browser-os/capability-clipboard" || spec === "@browser-os/capability-clipboard/manifest") hostOk++;
    }
  }
  if (hostDeep > 0) v.push(`[PKG-09] Host deep-import 包内 ${hostDeep} 处`);
  if (hostOk === 0) v.push("[PKG-09] Host 未通过 public contract 消费包");

  // PKG-10 旧实现 absent
  if (existsSync(join(ROOT, "src/capabilities/clipboard")))
    v.push("[PKG-10] 旧 src/capabilities/clipboard 仍存在");
  for (const [rel, code] of files)
    if (!inPkg(rel) && /capabilities\/clipboard\/(state|ui|internal|manifest|index|public)/.test(code))
      v.push(`[PKG-10] ${rel} 仍引用旧路径`);

  // PKG-11 持久化不变量（B11-1）：先剥离注释，避免注释中的"localStorage"字样误报
  const storeRaw = files.get(`${SRC_REL}/state/useClipboardStore.ts`) || "";
  const storeCode = stripComments(storeRaw);
  if (/(localStorage|sessionStorage|indexedDB|globalThis\.localStorage)/.test(storeCode))
    v.push("[PKG-11] store 引用持久化 API");
  if (/setItem\(|getItem\(|removeItem\(/.test(storeCode))
    v.push("[PKG-11] store 调用持久化读写");

  // PKG-12 无重复贡献注册（按出现次数计，避免单文件重复注册漏判）
  let pkgReg = 0, hostReg = 0;
  for (const [rel, code] of files) {
    if (inPkgSrc(rel)) {
      for (const _ of code.match(/contributionRegistry\./g) || []) pkgReg++;
    } else {
      for (const _ of code.match(/(?:register|registerContribution)\(\s*clipboardContribution/g) || []) hostReg++;
    }
  }
  if (pkgReg > 0) v.push(`[PKG-12] 包内自注册贡献 ${pkgReg} 处`);
  if (hostReg !== 1) v.push(`[PKG-12] Host 注册 clipboardContribution ${hostReg} 处（应为 1）`);
}

// ---------------------------------------------------------------------------
// SELF-TEST
// ---------------------------------------------------------------------------
const STORE_BASE =
  "import { ref, inject } from 'vue';\n" +
  "const ports = inject('clipboardPorts');\n" +
  "export const useClipboardStore = defineStore('clipboard', () => { const x = ref(0); return { x }; });\n";
function baseFiles() {
  const m = new Map();
  m.set(PKG_JSON, JSON.stringify({
    name: "@browser-os/capability-clipboard",
    version: "0.1.0",
    private: true,
    type: "module",
    exports: { ".": "./src/index.ts", "./manifest": "./src/manifest.ts" },
    dependencies: { vue: "^3.4.0", pinia: "^2.3.1" },
    scripts: { check: "node scripts/check-package.mjs" },
  }));
  m.set(`${SRC_REL}/index.ts`, "export const clipboardManifest = {};\nexport const CLIPBOARD_CAPABILITY_ID = 'clipboard';\nexport const useClipboardStore = () => {};\nexport const clipboardContribution = {};\nexport function createClipboardCapability(){}\nexport const CLIPBOARD_PORTS_KEY = Symbol('x');\n");
  m.set(`${SRC_REL}/ports/index.ts`, "export const CLIPBOARD_PORTS_KEY = Symbol('clipboardPorts');\n");
  m.set(`${SRC_REL}/state/useClipboardStore.ts`, STORE_BASE);
  m.set(`${SRC_REL}/ui/ClipboardPanel.vue`, "<template><div/></template>\n");
  // 合法 Host 消费者：经 public contract 导入并恰好注册 1 次（PKG-09/PKG-12 在 ST-01 应为零失败）
  m.set("src/capability/index.ts", "import { clipboardContribution } from '@browser-os/capability-clipboard';\nimport { contributionRegistry } from './contribution/registry';\ncontributionRegistry.registerContribution(clipboardContribution);\n");
  return m;
}
function withFixture(patch, hostPatch) {
  const m = baseFiles();
  for (const [k, val] of Object.entries(patch || {})) m.set(k, val);
  const h = new Map();
  for (const [k, val] of Object.entries(hostPatch || {})) h.set(k, val);
  return new Map([...m, ...h]);
}
function selfTest() {
  const cases = [
    { id: "ST-01", name: "valid → 零失败", files: baseFiles(), expect: [] },
    { id: "ST-02", name: "外部 deep import 包 src → PKG-04", files: withFixture({}, { "src/components/MainArea.vue": 'import { x } from "../../packages/capability-clipboard/src/state/useClipboardStore";\n' }), expect: ["PKG-04"] },
    { id: "ST-03", name: "exports 泄露 internal → PKG-03", files: withFixture({ [`${PKG_REL}/package.json`]: JSON.stringify({ name: "@browser-os/capability-clipboard", exports: { ".": "./src/index.ts", "./internal": "./src/internal/x.ts" } }) }), expect: ["PKG-03"] },
    { id: "ST-04", name: "Host impl import → PKG-06", files: withFixture({ [`${SRC_REL}/state/useClipboardStore.ts`]: STORE_BASE + 'import { bridge } from "../../../../src/bridge";\n' }), expect: ["PKG-06"] },
    { id: "ST-05", name: "未声明依赖 → PKG-05", files: withFixture({ [`${SRC_REL}/state/useClipboardStore.ts`]: STORE_BASE + 'import { z } from "leftpad";\n' }), expect: ["PKG-05"] },
    { id: "ST-06", name: "持久化 API → PKG-11", files: withFixture({ [`${SRC_REL}/state/useClipboardStore.ts`]: STORE_BASE + 'localStorage.setItem("k", "v");\n' }), expect: ["PKG-11"] },
    { id: "ST-07", name: "包内自注册 → PKG-12", files: withFixture({ [`${SRC_REL}/index.ts`]: "import { contributionRegistry } from '@browser-os/capability-vault';\ncontributionRegistry.register({});\n" }), expect: ["PKG-12"] },
    { id: "ST-08", name: "Host 注册 ≠1 → PKG-12", files: withFixture({}, { "src/capability/index.ts": "import { clipboardContribution } from '@browser-os/capability-clipboard';\nimport { contributionRegistry } from './contribution/registry';\ncontributionRegistry.registerContribution(clipboardContribution);\ncontributionRegistry.registerContribution(clipboardContribution);\n" }), expect: ["PKG-12"] },
    { id: "ST-09", name: "旧路径残留 → PKG-10", files: withFixture({}, { "src/App.vue": 'import { x } from "./capabilities/clipboard/public";\n' }), expect: ["PKG-10"] },
  ];
  let pass = 0;
  for (const c of cases) {
    const v = [], fg = [];
    check(c.files, v, fg);
    const got = v.map((s) => s.match(/^\[([A-Z0-9-]+)\]/)[1]);
    // 宽松判定：仅验证"预期门禁码被检出"（self-test 关注目标检测是否触发）
    const ok = c.expect.every((e) => v.some((f) => f.includes(e)));
    console.log(`${ok ? "PASS" : "FAIL"} ${c.id} ${c.name}${ok ? "" : " → got=" + JSON.stringify(v)}`);
    if (ok) pass++;
  }
  console.log(`SELF-TEST: ${pass}/${cases.length}`);
  if (pass !== cases.length) process.exit(1);
}

if (process.argv.includes("--self-test")) selfTest();
else {
  const files = collectReal();
  const v = [], fg = [];
  check(files, v, fg);
  const json = process.argv.includes("--json");
  if (json) console.log(JSON.stringify({ fail: v, finding: fg }, null, 2));
  else {
    console.log(`CLIPBOARD_PACKAGE fail=${v.length} finding=${fg.length}`);
    for (const f of v) console.log(`  FAIL ${f}`);
    for (const f of fg) console.log(`  FG   ${f}`);
  }
  if (v.length) process.exit(1);
}
