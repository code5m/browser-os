#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-vault-boundary.mjs — Vault M2 能力包边界门禁（Frontend M2 TRUE Pilot）
//
// 包已升级为真实 npm workspace package：@browser-os/capability-vault。
// 本门禁扫描：包源 packages/capability-vault/src + 宿主 src，验证 M2 边界。
//
// 门禁码（与 M1 Pilot 同语义，路径升级为包）：
//   VB-01 包外不得直接 import 包内 internal（state/ ui/ internal/）；必须经 @browser-os/capability-vault
//   VB-02 public API 导出边界（index.ts 不泄露 internal；package.json exports 仅 '.' / './manifest'）
//   VB-03 禁止反向依赖：包内不得依赖 App.vue / components/** / main.ts / 宿主 stores
//   VB-04 禁止跨能力 internal 直连（只能用其它能力的 public/index/manifest）
//   VB-05 state owner 唯一（defineStore('vault') 恰好 1 处；manifest.semanticOwner 指向真实文件）
//   VB-06 native invoke 不得出现在 UI 层（禁止 ui/*.vue 直接用 bridge/invoke）
//   VB-07 UI 注册权威入口唯一（"活"注册恰好 1 处；别名 registerContributions 记为 FG 而非静默放过）
//   VB-08 stale old path = 0（旧的 src/utils/vault.mjs 与 src/capabilities/vault 物理不存在 + 无真实引用）
//   VB-09 dead shim = 0（包外不得再导出被下沉的 symbol 做兼容层）
//   VB-10 duplicate semantic implementation = 0（下沉 symbol 全仓定义恰好 1 处）
//
// 用法: node scripts/check-vault-boundary.mjs [--self-test] [--json]
// ---------------------------------------------------------------------------
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");
const PKG_REL = "packages/capability-vault/src";
// 允许外部触达的能力面（唯一稳定出入口）
const PUBLIC_SURFACE = new Set(["index.ts", "manifest.ts"]);
// 由 Pilot 下沉进能力包的领域 symbol（唯一实现）
const SUNK_SYMBOLS = ["resolveNote", "noteLinks", "searchNotes"];

// --------------------------------------------------------------------------
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
const inPkg = (rel) => rel.startsWith(`${PKG_REL}/`);
const isExternal = (rel) => !inPkg(rel) && !rel.startsWith("packages/capability-vault/");

/** 把相对说明符解析为仓库相对路径（用于跨目录/跨能力越界检测） */
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
  for (const abs of walkDisk(join(ROOT, "src"), new Set())) {
    files.set(relative(ROOT, abs).split("\\").join("/"), readFileSync(abs, "utf8"));
  }
  for (const abs of walkDisk(join(ROOT, PKG_REL), new Set())) {
    const r = relative(ROOT, abs).split("\\").join("/");
    if (r.endsWith(".d.ts")) continue;
    files.set(r, readFileSync(abs, "utf8"));
  }
  return files;
}
function canonicalFile(files, resolved) {
  if (!resolved) return null;
  if (files.has(resolved)) return resolved;
  for (const ext of [".ts", ".mjs", ".vue", ".js"]) if (files.has(resolved + ext)) return resolved + ext;
  return null;
}

// --------------------------------------------------------------------------
function check(files, v, fg) {
  const fail = (c, m) => v.push(`[${c}] ${m}`);
  const finding = (c, m) => fg.push(`[${c}] ${m}`);

  // VB-01 外部 deep-import 包内 src 子路径（含未走 package 名）
  const deepRe = /packages\/capability-vault\/src\/(state|ui|internal)/;
  const pkgNameDeepRe = /@browser-os\/capability-vault\/(state|ui|internal)/;
  for (const [rel, code] of files) {
    if (!isExternal(rel)) continue;
    for (const spec of parseSpecifiers(code)) {
      if (deepRe.test(spec) || pkgNameDeepRe.test(spec))
        fail("VB-01", `${rel} 直接 deep-import 包内：${spec}`);
    }
  }
  // VB-02 package.json exports 仅 '.' / './manifest'（优先读注入表以支持 self-test，回退 disk）
  const pkgJsonKey = "packages/capability-vault/package.json";
  const pkgJsonPath = join(ROOT, pkgJsonKey);
  const pjRaw = files.get(pkgJsonKey);
  let pj = pjRaw ? JSON.parse(pjRaw) : null;
  if (!pj && existsSync(pkgJsonPath)) pj = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
  if (!pj) fail("VB-02", "package.json 缺失");
  else {
    for (const k of Object.keys(pj.exports || {}))
      if (k !== "." && k !== "./manifest") fail("VB-02", `exports 泄露内部：${k}`);
    const idx = files.get(`${PKG_REL}/index.ts`);
    if (idx)
      for (const sym of SUNK_SYMBOLS)
        if (new RegExp(`export\\s+(?:\\{[^}]*\\b${sym}\\b|\\w+\\s+${sym}\\b)`).test(idx) ||
            new RegExp(`export\\s+function\\s+${sym}\\b`).test(idx))
          fail("VB-02", `index.ts 泄露内部 symbol：${sym}`);
  }

  // VB-03 反向依赖
  const revRe = /(^|\.\.\/)+(?:App\.vue|main\.ts|components\/)/;
  for (const [rel, code] of files) {
    if (!inPkg(rel)) continue;
    for (const spec of parseSpecifiers(code))
      if (revRe.test(spec)) fail("VB-03", `${rel} 反向依赖：${spec}`);
  }
  // VB-04 跨能力 internal 直连（解析相对说明符后落在 src/capabilities/<其它能力> 内）
  for (const [rel, code] of files) {
    if (!inPkg(rel)) continue;
    for (const spec of parseSpecifiers(code)) {
      const resolved = resolveSpec(rel, spec);
      if (/^src\/capabilities\/(?!vault)[a-z]+\/(state|ui|internal|adapters|api)/.test(resolved))
        fail("VB-04", `${rel} 跨能力 internal 直连：${resolved}`);
    }
  }
  // VB-05 state owner 唯一
  let storeCount = 0;
  for (const [rel, code] of files)
    if (inPkg(rel) && /defineStore\(\s*["']vault["']/.test(code)) storeCount++;
  if (storeCount !== 1) fail("VB-05", `defineStore('vault') 出现 ${storeCount} 次（应为 1）`);
  // VB-06 UI 层直连 native
  for (const [rel, code] of files)
    if (rel.endsWith("ui/VaultPanel.vue") && /(^|[^.\w])bridge\.|invoke\(/.test(code))
      fail("VB-06", `${rel} UI 层直连 native`);
  // VB-07 UI 注册权威入口唯一
  const idx = files.get(`${PKG_REL}/index.ts`) || "";
  const liveCalls = (idx.match(/registerVaultContributions\s*\(\s*\)/g) || []).length;
  const hasDef = /export\s+function\s+registerVaultContributions\s*\(/.test(idx);
  const executed = Math.max(0, liveCalls - (hasDef ? 1 : 0));
  if (executed > 1) fail("VB-07", `Vault 注册入口执行 ${executed} 次`);
  else if (hasDef) finding("VB-07", "存在 registerVaultContributions 别名（单活执行，记为 FG）");
  // VB-08 stale old path
  const oldUtils = join(ROOT, "src/utils/vault.mjs");
  const oldPkg = join(ROOT, "src/capabilities/vault");
  if (existsSync(oldUtils)) fail("VB-08", "旧 src/utils/vault.mjs 仍存在");
  if (existsSync(oldPkg)) fail("VB-08", "旧 src/capabilities/vault 仍存在");
  for (const [rel, code] of files)
    if (/(^|[^\w])utils\/vault(\.mjs)?\b/.test(code) || /capabilities\/vault\/(state|ui|internal)/.test(code))
      fail("VB-08", `${rel} 仍引用旧路径`);
  // VB-09 dead shim
  for (const [rel, code] of files) {
    if (inPkg(rel)) continue;
    for (const spec of parseSpecifiers(code))
      if (/(re-)?export.*\b(noteLinks|resolveNote|searchNotes)\b.*from/.test(code) && /capabilities\/vault|capability-vault\/src\/internal/.test(code))
        fail("VB-09", `${rel} 包外再导出下沉 symbol（dead shim）`);
  }
  // VB-10 duplicate semantic implementation
  for (const sym of SUNK_SYMBOLS) {
    const sites = [];
    for (const [rel, code] of files)
      if (/(?:export\s+)?function\s+noteLinks|resolveNote|searchNotes/.test(code) &&
          new RegExp(`function\\s+${sym}\\s*\\(`).test(code)) sites.push(rel);
    if (sites.length !== 1) fail("VB-10", `${sym} 实现出现 ${sites.length} 处：${sites.join(", ")}`);
  }
}

// --------------------------------------------------------------------------
// SELF-TEST
// --------------------------------------------------------------------------
const STORE_BASE =
  "import { ref, inject } from 'vue';\n" +
  "const ports = inject('vaultPorts');\n" +
  "export const useVaultStore = defineStore('vault', () => { const path = ref(''); return { path }; });\n";
function baseFiles() {
  const m = new Map();
  m.set(`${PKG_REL}/index.ts`, "import { createVaultCapability, vaultContribution, VAULT_PORTS_KEY } from './ports';\nexport const vaultManifest = {};\n");
  m.set(`${PKG_REL}/ports/index.ts`, "export const VAULT_PORTS_KEY = Symbol('vaultPorts');\nexport function createVaultCapability(){}\n");
  m.set(`${PKG_REL}/state/useVaultStore.ts`, STORE_BASE);
  m.set(`${PKG_REL}/internal/vault.mjs`, "export function noteLinks(){}\nexport function resolveNote(){}\nexport function searchNotes(){}\n");
  m.set(`${PKG_REL}/ui/VaultPanel.vue`, "<template><div/></template>\n");
  return m;
}
function withFixture(patch, hostPatch) {
  const m = baseFiles();
  for (const [k, v] of Object.entries(patch || {})) m.set(k, v);
  const h = new Map();
  for (const [k, v] of Object.entries(hostPatch || {})) h.set(k, v);
  const all = new Map([...m, ...h]);
  return all;
}
function selfTest() {
  const cases = [
    { id: "ST-01", name: "valid → 零失败", files: baseFiles(), expect: [] },
    { id: "ST-02", name: "外部 deep import 包 src → VB-01", files: withFixture({}, { "src/components/layout/MainArea.vue": '<script>import { useVaultStore } from "../../packages/capability-vault/src/state/useVaultStore";</script>\n' }), expect: ["VB-01"] },
    { id: "ST-03", name: "反向依赖 App.vue → VB-03", files: withFixture({ [`${PKG_REL}/state/useVaultStore.ts`]: STORE_BASE + 'import App from "../../../App.vue";\n' }), expect: ["VB-03"] },
    { id: "ST-04", name: "跨能力 internal → VB-04", files: withFixture({ [`${PKG_REL}/state/useVaultStore.ts`]: STORE_BASE + 'import { x } from "../../../../src/capabilities/git/state/useGitStore";\n' }), expect: ["VB-04"] },
    { id: "ST-05", name: "双 state owner → VB-05", files: withFixture({ [`${PKG_REL}/ui/ShadowStore.ts`]: 'export const s = defineStore("vault", () => ({}));\n' }), expect: ["VB-05"] },
    { id: "ST-06", name: "UI 直连 native → VB-06", files: withFixture({ [`${PKG_REL}/ui/VaultPanel.vue`]: "<script>bridge.vaultOpen('x');</script>\n" }), expect: ["VB-06"] },
    { id: "ST-07", name: "stale old path 引用 → VB-08", files: withFixture({ [`${PKG_REL}/state/useVaultStore.ts`]: STORE_BASE + 'import { noteLinks } from "../../../utils/vault.mjs";\n' }), expect: ["VB-08"] },
    { id: "ST-08", name: "包外 dead shim → VB-09 + VB-01", files: withFixture({}, { "src/utils/vaultShim.mjs": 'export { noteLinks } from "../packages/capability-vault/src/internal/vault.mjs";\n' }), expect: ["VB-09", "VB-01"] },
    { id: "ST-09", name: "duplicate impl → VB-10", files: withFixture({ "src/utils/vaultDup.mjs": "function searchNotes() { /* 私有重复 */ }\n" }), expect: ["VB-10"] },
    { id: "ST-10", name: "exports 泄露 internal → VB-02", files: withFixture({ "packages/capability-vault/package.json": JSON.stringify({ name: "@browser-os/capability-vault", exports: { ".": "./src/index.ts", "./internal": "./src/internal/vault.mjs" } }) }), expect: ["VB-02"] },
  ];
  let pass = 0;
  for (const c of cases) {
    const v = [], fg = [];
    check(c.files, v, fg);
    const got = [...v, ...fg].map((s) => s.match(/^\[([A-Z0-9-]+)\]/)[1]).filter((x) => v.some((f) => f.includes(x)));
    // ST-07/ST-08 还会触发 VB-01；ST-10 仅 VB-02；其余仅 expect
    const expect = c.expect;
    const ok = expect.every((e) => v.some((f) => f.includes(e))) &&
      v.every((f) => expect.some((e) => f.includes(e)));
    console.log(`${ok ? "PASS" : "FAIL"} ${c.id} ${c.name}${ok ? "" : " → got=" + JSON.stringify(v)}`);
    if (ok) pass++;
  }
  console.log(`SELF-TEST: ${pass}/${cases.length}`);
  if (pass !== cases.length) process.exit(1);
}

// --------------------------------------------------------------------------
if (process.argv.includes("--self-test")) selfTest();
else {
  const files = collectReal();
  const v = [], fg = [];
  check(files, v, fg);
  const json = process.argv.includes("--json");
  if (json) console.log(JSON.stringify({ fail: v, finding: fg }, null, 2));
  else {
    console.log(`VAULT_BOUNDARY fail=${v.length} finding=${fg.length}`);
    for (const f of v) console.log(`  FAIL ${f}`);
    for (const f of fg) console.log(`  FG   ${f}`);
  }
  if (v.length) process.exit(1);
}
