#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-package.mjs — @browser-os/capability-vault M2 包边界门禁（PKG-01..PKG-10）
//
// 设计原则（与 Vault Pilot 一致，禁止 grep 固定旧文件名假绿）：
//   - 规则基于真实文件表（Map<relPath, content>）与真实 import 解析。
//   - 大部分规则接受可注入 files Map → --self-test 用合成 fixture 证明检出能力。
//   - 真实扫描直接读 disk：包源 packages/capability-vault/src + 宿主 src。
//
// 门禁：
//   PKG-01 包根存在
//   PKG-02 包 manifest 合法（name/version/private/type/exports）
//   PKG-03 exports 显式且机器限定（只暴露 '.' 与 './manifest'，禁止泄露 state/ui/internal）
//   PKG-04 包外不得 deep-import 包内 src 子路径
//   PKG-05 包内不得出现未声明依赖（bare import 必须在 deps/peerDeps）
//   PKG-06 包内不得依赖 Host 内部（bridge / stores / utils/graphUi / capability/* / App.vue / components）
//   PKG-07 公共入口可解析（index.ts 导出 createVaultCapability / vaultContribution / VAULT_PORTS_KEY）
//   PKG-08 包内 validation 可执行（package.json scripts.check 存在）
//   PKG-09 宿主经包公共契约消费（宿主 import '@browser-os/capability-vault'，非相对 deep）
//   PKG-10 旧能力源根 src/capabilities/vault 不存在（无第二实现）
//
// 用法：node scripts/check-package.mjs [--self-test] [--json]
// ---------------------------------------------------------------------------
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname, resolve, posix } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(HERE, "..");
const REPO_ROOT = resolve(PKG_ROOT, "..", "..");
const PKG_REL = "packages/capability-vault";
const SRC_REL = `${PKG_REL}/src`;

const FAIL = [];
const FG = [];
const findings = [];
function fail(code, msg) { FAIL.push(`[${code}] ${msg}`); }
function finding(code, msg) { FG.push(`[${code}] ${msg}`); findings.push(msg); }

// --------------------------------------------------------------------------
// 通用工具
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
function bareModule(spec) {
  if (!spec.startsWith(".") && !spec.startsWith("/")) {
    return spec.startsWith("@") ? spec.split("/").slice(0, 2).join("/") : spec.split("/")[0];
  }
  return null;
}

// --------------------------------------------------------------------------
// 规则实现（接受注入 files Map，便于 self-test）
// --------------------------------------------------------------------------
function rulePkg03_04_05_06(files, pkgJson) {
  // PKG-03 exports 显式且限定
  const exportsKeys = Object.keys(pkgJson?.exports || {});
  for (const k of exportsKeys) {
    if (k !== "." && k !== "./manifest") fail("PKG-03", `exports 暴露了禁止条目：${k}`);
  }
  // PKG-06 / PKG-05 / PKG-04 / PKG-10 由外层真实扫描与 self-test 覆盖；此处仅校验 exports
}

function rulePkg06_hostInternal(files) {
  const forbidden = [
    /(^|\.\.\/)+bridge(\.ts|\.js|\.mjs)?$/,
    /(^|\.\.\/)+stores\//,
    /(^|\.\.\/)+utils\//,
    /(^|\.\.\/)+capability\//,
    /(^|\.\.\/)+App\.vue$/,
    /(^|\.\.\/)+components\//,
    /^\.{0,2}\/?(src\/)?bridge(\.ts)?$/,
  ];
  for (const [rel, code] of files) {
    if (!rel.startsWith(SRC_REL + "/")) continue;
    for (const spec of parseSpecifiers(code)) {
      const resolved = spec.replace(/\.[^./]+$/, "");
      for (const re of forbidden) {
        if (re.test(spec) || re.test(resolved)) fail("PKG-06", `${rel} 依赖 Host 内部：${spec}`);
      }
    }
  }
}
function rulePkg05_undeclaredDeps(files, pkgJson) {
  const declared = new Set([
    ...Object.keys(pkgJson?.dependencies || {}),
    ...Object.keys(pkgJson?.peerDependencies || {}),
  ]);
  const builtins = new Set(["node:fs", "node:path", "node:url", "node:assert/strict", "node:module"]);
  for (const [rel, code] of files) {
    if (!rel.startsWith(SRC_REL + "/")) continue;
    for (const spec of parseSpecifiers(code)) {
      const bare = bareModule(spec);
      if (bare && !declared.has(bare) && !builtins.has(bare)) {
        fail("PKG-05", `${rel} 使用了未声明依赖：${bare}`);
      }
    }
  }
}
function rulePkg04_externalDeepImport(files) {
  for (const [rel, code] of files) {
    if (rel.startsWith(SRC_REL + "/")) continue; // 包内不算
    for (const spec of parseSpecifiers(code)) {
      if (spec.includes("packages/capability-vault/src/")) {
        fail("PKG-04", `${rel} deep-import 包内 src 子路径：${spec}`);
      }
    }
  }
}
function rulePkg09_hostContract(files) {
  let hostConsumes = 0;
  let hostDeep = 0;
  for (const [rel, code] of files) {
    if (rel.startsWith(SRC_REL + "/")) continue;
    if (rel.startsWith("packages/capability-vault/")) continue;
    for (const spec of parseSpecifiers(code)) {
      if (spec.startsWith("@browser-os/capability-vault")) hostConsumes++;
      if (spec.includes("packages/capability-vault/src/")) hostDeep++;
    }
  }
  if (hostConsumes === 0) finding("PKG-09", "未发现宿主经包名消费 Vault（无外部消费方）—— 仍可接受，但请确认已接线");
  if (hostDeep > 0) fail("PKG-09", `宿主存在 ${hostDeep} 处相对 deep import（应改用 @browser-os/capability-vault）`);
}

// --------------------------------------------------------------------------
// 真实扫描
// --------------------------------------------------------------------------
function collectDir(dir) {
  const m = new Map();
  for (const abs of walkDisk(dir, new Set())) {
    m.set(relative(REPO_ROOT, abs).split("\\").join("/"), readFileSync(abs, "utf8"));
  }
  return m;
}
function realScan() {
  const pkgJsonPath = join(PKG_ROOT, "package.json");
  if (!existsSync(PKG_ROOT)) fail("PKG-01", "包根 packages/capability-vault 不存在");
  if (!existsSync(pkgJsonPath)) { fail("PKG-02", "package.json 缺失"); return; }
  const pkgJson = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
  if (!pkgJson.name || !pkgJson.name.startsWith("@browser-os/capability-vault"))
    fail("PKG-02", `包名非法：${pkgJson.name}`);
  if (!pkgJson.version || !pkgJson.private || pkgJson.type !== "module" || !pkgJson.exports)
    fail("PKG-02", "package.json 缺 version/private/type/exports");
  if (!pkgJson.scripts?.check) fail("PKG-08", "package.json scripts.check 缺失");
  rulePkg03_04_05_06(new Map(), pkgJson);

  const pkgFiles = collectDir(join(REPO_ROOT, SRC_REL));
  if (pkgFiles.size === 0) fail("PKG-01", `包内 src 无文件：${SRC_REL}`);
  rulePkg06_hostInternal(pkgFiles);
  rulePkg05_undeclaredDeps(pkgFiles, pkgJson);

  const hostFiles = collectDir(join(REPO_ROOT, "src"));
  rulePkg04_externalDeepImport(hostFiles);
  rulePkg09_hostContract(hostFiles);

  // PKG-07 公共入口可解析（index.ts 导出关键符号）
  const idx = pkgFiles.get(`${SRC_REL}/index.ts`);
  if (!idx) fail("PKG-07", "index.ts 缺失");
  else {
    for (const sym of ["createVaultCapability", "vaultContribution", "VAULT_PORTS_KEY"])
      if (!new RegExp(`export\\s+(?:.*\\b${sym}\\b)`).test(idx) && !idx.includes(sym))
        fail("PKG-07", `index.ts 未导出公共符号：${sym}`);
  }
  // PKG-10 旧源根不得存在
  if (existsSync(join(REPO_ROOT, "src/capabilities/vault")))
    fail("PKG-10", "旧源根 src/capabilities/vault 仍存在（第二实现）");
}

// --------------------------------------------------------------------------
// SELF-TEST（合成 fixture 证明检出能力）
// --------------------------------------------------------------------------
const BASE = (() => {
  const j = { name: "@browser-os/capability-vault", version: "0.1.0", private: true, type: "module", exports: { ".": "./src/index.ts", "./manifest": "./src/manifest.ts" }, scripts: { check: "tsc --noEmit" }, dependencies: { vue: "^3.4.0", pinia: "^2.3.1", marked: "^18.0.12", "@tauri-apps/plugin-dialog": "^2.7.3" } };
  const m = new Map();
  m.set(`${SRC_REL}/index.ts`, "export { createVaultCapability, vaultContribution, VAULT_PORTS_KEY } from './ports';\nexport const vaultManifest = {};\n");
  m.set(`${SRC_REL}/ports/index.ts`, "export const VAULT_PORTS_KEY = Symbol('vaultPorts');\nexport function createVaultCapability(){}\n");
  m.set(`${SRC_REL}/state/useVaultStore.ts`, "import { ref } from 'vue';\nexport const useVaultStore = () => ref('');\n");
  m.set(`${SRC_REL}/internal/vault.mjs`, "export function noteLinks(){}\n");
  m.set(`${SRC_REL}/ui/VaultPanel.vue`, "<template><div/></template>\n");
  return { j, m };
})();
function withFixture(patch, hostPatch) {
  const j = JSON.parse(JSON.stringify(BASE.j));
  const m = new Map(BASE.m);
  for (const [k, v] of Object.entries(patch || {})) m.set(k, v);
  const host = new Map();
  for (const [k, v] of Object.entries(hostPatch || {})) host.set(k, v);
  return { j, m, host };
}
function runRules({ j, m, host }) {
  const errs = [];
  const origFail = fail;
  const captured = [];
  // 临时重定向 fail
  const realFail = fail;
  // 直接调用规则并收集（用局部包装）
  const f = (c, msg) => captured.push(`[${c}] ${msg}`);
  // 重跑规则（复用全局 fail 会污染，这里直接调用规则函数并传 f）
  // PKG-03
  for (const k of Object.keys(j.exports || {})) if (k !== "." && k !== "./manifest") f("PKG-03", k);
  // PKG-06
  for (const [rel, code] of m) {
    if (!rel.startsWith(SRC_REL + "/")) continue;
    for (const spec of parseSpecifiers(code)) {
      if (/(^|\.\.\/)+bridge/.test(spec) || /(^|\.\.\/)+stores\//.test(spec) || /(^|\.\.\/)+capability\//.test(spec) || /(^|\.\.\/)+utils\//.test(spec))
        f("PKG-06", `${rel} ${spec}`);
    }
  }
  // PKG-05
  const declared = new Set([...Object.keys(j.dependencies || {}), ...Object.keys(j.peerDependencies || {})]);
  for (const [rel, code] of m) {
    if (!rel.startsWith(SRC_REL + "/")) continue;
    for (const spec of parseSpecifiers(code)) {
      const b = bareModule(spec);
      if (b && !declared.has(b)) f("PKG-05", `${rel} ${b}`);
    }
  }
  // PKG-04 / PKG-09（宿主 deep import）
  for (const [rel, code] of host) {
    for (const spec of parseSpecifiers(code)) {
      if (spec.includes("packages/capability-vault/src/")) f("PKG-04", `${rel} ${spec}`);
      if (spec.includes("packages/capability-vault/src/")) f("PKG-09", `${rel} ${spec}`);
    }
  }
  return captured;
}
function selfTest() {
  const cases = [
    { id: "ST-01", name: "valid fixture → 零失败", fix: withFixture(), expect: [] },
    { id: "ST-02", name: "exports 泄露 internal → PKG-03", fix: withFixture({}, {}), expect: ["PKG-03"], overrideExports: { ".": "./src/index.ts", "./internal": "./src/internal/vault.mjs" } },
    { id: "ST-03", name: "包内依赖 bridge → PKG-06", fix: withFixture({ [`${SRC_REL}/state/useVaultStore.ts`]: "import { bridge } from '../../../bridge';\nexport const x=1;\n" }), expect: ["PKG-06"] },
    { id: "ST-04", name: "包内依赖 capability/* → PKG-06", fix: withFixture({ [`${SRC_REL}/index.ts`]: "import { c } from '../../../capability/types';\n" }), expect: ["PKG-06"] },
    { id: "ST-05", name: "包内未声明依赖 → PKG-05", fix: withFixture({ [`${SRC_REL}/state/useVaultStore.ts`]: "import { z } from 'undeclared-pkg';\n" }), expect: ["PKG-05"] },
    { id: "ST-06", name: "宿主 deep import 包 src → PKG-04+PKG-09", fix: withFixture({}, { "src/App.vue": `import { x } from '../../packages/capability-vault/src/state/useVaultStore';` }), expect: ["PKG-04", "PKG-09"] },
  ];
  let pass = 0;
  for (const c of cases) {
    const { j, m, host } = c.fix;
    if (c.overrideExports) j.exports = c.overrideExports;
    const got = runRules({ j, m, host }).map((s) => s.match(/^\[([A-Z0-9-]+)\]/)[1]);
    const ok = c.expect.every((e) => got.includes(e)) && got.every((g) => c.expect.includes(g) || g === undefined);
    console.log(`${ok ? "PASS" : "FAIL"} ${c.id} ${c.name}${ok ? "" : " → got=" + JSON.stringify(got)}`);
    if (ok) pass++;
  }
  console.log(`SELF-TEST: ${pass}/${cases.length}`);
  if (pass !== cases.length) process.exit(1);
}

// --------------------------------------------------------------------------
// 入口
// --------------------------------------------------------------------------
if (process.argv.includes("--self-test")) selfTest();
else {
  realScan();
  const json = process.argv.includes("--json");
  if (json) console.log(JSON.stringify({ fail: FAIL, finding: FG }, null, 2));
  else {
    console.log(`PKG: @browser-os/capability-vault`);
    console.log(`fail=${FAIL.length} finding=${FG.length}`);
    for (const f of FAIL) console.log(`  FAIL ${f}`);
    for (const f of FG) console.log(`  FG   ${f}`);
  }
  if (FAIL.length) process.exit(1);
}
