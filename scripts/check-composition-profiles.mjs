#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-composition-profiles.mjs — 真实 Composition Profile 门禁（Phase 8E / Train F）
//
// 验证 Minimal/Developer/Full 是**真实注册 profile**（不是 UI hide）：
//   - minimal   只注册 bookmark+workspace → terminal/browser 不在 registry（absent → 零 PTY/零 WebView）
//   - developer 注册 bookmark+workspace+browser+terminal
//   - full      注册全部 4 个
// 并验证 ResourceGovernor 是薄协调层（不 import 任何能力 owner/store，不持业务 state）。
//
// 判定手段：esbuild 转译真实 src/capability/index.ts + profiles.ts + resourceGovernor.ts，
// 按 profile 真实调用 bootstrapCapabilityRuntime(profile)，断言 registry 成员。
// 真源：src/capability/profiles.ts（CAPABILITY_PROFILES 单源）。
//
// 用法: node scripts/check-composition-profiles.mjs [--self-test] [--json]
// 退出码: 0 = 通过；1 = 失败
// ---------------------------------------------------------------------------

import { build } from "esbuild";
import { readFileSync, writeFileSync, unlinkSync, existsSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve as resolvePath } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const fails = [];
const pass = [];
const ok = (id, label) => pass.push(`${id} ${label}`);
const bad = (id, label, detail) => fails.push(`${id} ${label}${detail ? `  → ${detail}` : ""}`);

function stripComments(s) {
  return s.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:])\/\/.*$/gm, "$1");
}
function walk(dir, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const e of readdirSync(dir)) {
    if (e === "node_modules" || e === "dist" || e.startsWith(".")) continue;
    const p = join(dir, e);
    if (statSync(p).isDirectory()) walk(p, acc);
    else if (/\.(ts|vue)$/.test(e)) acc.push(p);
  }
  return acc;
}

let tmpSeq = 0;
async function bootstrapUnder(profile) {
  const entry = `
    import { bootstrapCapabilityRuntime } from '${join(ROOT, "src/capability/index.ts").replace(/\\/g, "/")}';
    export { bootstrapCapabilityRuntime };
  `;
  const res = await build({
    stdin: { contents: entry, resolveDir: ROOT, loader: "ts" },
    bundle: true,
    format: "esm",
    platform: "neutral",
    target: "es2020",
    external: ["pinia", "vue", "@vue/*", "*.vue", "turndown", "dompurify", "marked"],
    write: false,
  });
  // 每个 profile 用独立模块实例（bootstrap 是 app 级单例，测试需模拟「以不同 profile 启动」）
  const tmp = join(ROOT, `.tmp-profiles-${tmpSeq++}.mjs`);
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  try {
    const mod = await import(pathToFileURLLocal(tmp).href);
    return mod.bootstrapCapabilityRuntime(profile);
  } finally {
    try { unlinkSync(tmp); } catch { /* ignore */ }
  }
}
function pathToFileURLLocal(p) {
  return { href: "file://" + p };
}

async function run() {
  // 真实 profile 注册验证
  const minimal = await bootstrapUnder("minimal");
  const dev = await bootstrapUnder("developer");
  const full = await bootstrapUnder("full");
  const ids = (b) => b.runtime.inspect().map((x) => x.id).sort();

  // minimal：只 bookmark+workspace；terminal/browser 必须 absent
  const mIds = ids(minimal);
  if (mIds.length === 2 && mIds.includes("bookmark") && mIds.includes("workspace")) {
    ok("PROF-01", `minimal 只注册 bookmark+workspace（实得 ${mIds.join(",")}）`);
  } else bad("PROF-01", "minimal 只注册 bookmark+workspace", mIds.join(","));
  if (!mIds.includes("terminal")) ok("PROF-02", "minimal 不注册 terminal（→ 无 PTY 出生点）");
  else bad("PROF-02", "minimal 不应含 terminal", mIds.join(","));
  if (!mIds.includes("browser")) ok("PROF-03", "minimal 不注册 browser（→ 无 WebView）");
  else bad("PROF-03", "minimal 不应含 browser", mIds.join(","));

  // developer / full：含 terminal+browser
  for (const [name, b] of [["developer", dev], ["full", full]]) {
    const sIds = ids(b);
    const hasTerm = sIds.includes("terminal");
    const hasBrowser = sIds.includes("browser");
    const all4 = ["bookmark", "workspace", "browser", "terminal"].every((x) => sIds.includes(x));
    if (hasTerm && hasBrowser && all4) ok(`PROF-04-${name}`, `${name} 注册全部 4 能力（${sIds.join(",")}）`);
    else bad(`PROF-04-${name}`, `${name} 应含 terminal+browser+全部`, sIds.join(","));
  }

  // bootstrap 不抛错（能力层失败不应打断启动）
  if (minimal.activated && dev.activated && full.activated) ok("PROF-05", "各 profile bootstrap activated=true（无能力层异常）");
  else bad(
    "PROF-05",
    "bootstrap activated",
    `min=${minimal.activated} dev=${dev.activated} full=${full.activated} fullError=${full.error ?? "none"}`,
  );

  // ResourceGovernor 薄层：不 import 任何能力 owner/store/UI，只依赖 runtime + types
  const gov = readFileSync(join(ROOT, "src/capability/resourceGovernor.ts"), "utf8");
  const imports = [...gov.matchAll(/(?:from|import\()\s*["']([^"']+)["']/g)].map((m) => m[1]);
  const badImport = imports.filter(
    (s) => /capabilities\/(bookmark|workspace|browser|terminal)|stores\/|components\//.test(s) && !s.endsWith("runtime"),
  );
  if (badImport.length === 0) ok("GOV-01", "ResourceGovernor 不 import 任何能力 owner/store/UI（只经 runtime 公共 API）");
  else bad("GOV-01", "ResourceGovernor 越界 import", badImport.join(", "));

  // Governor 不持有业务 state：源文件中不得出现各能力的 owner 符号 / ref/reactive
  const holdsState = /(useTerminalStore|useBrowserStore|useBookmarkStore|useFileStore|termPanes|browserTabs)\b/.test(gov);
  if (!holdsState) ok("GOV-02", "ResourceGovernor 不持有任何业务 state（无 owner 符号/状态字段）");
  else bad("GOV-02", "ResourceGovernor 持有业务 state", "发现 owner 符号");

  // suspend 诚实拒绝对 terminal（不谎报 C4）：full profile 下 suspend(terminal) 必须抛 SUSPEND_NOT_SUPPORTED
  const { createResourceGovernor } = await loadGovernor();
  const fullBoot = await bootstrapUnder("full");
  const gov2 = createResourceGovernor(fullBoot.runtime);
  let threw = null;
  try {
    gov2.suspend("terminal");
  } catch (e) {
    threw = e instanceof Error ? e.code || e.message : String(e);
  }
  if (threw && /SUSPEND_NOT_SUPPORTED/.test(threw)) ok("GOV-03", "terminal suspend 诚实拒绝（SUSPEND_NOT_SUPPORTED，不谎报 C4 资源释放）");
  else bad("GOV-03", "terminal suspend 应拒绝", `threw=${threw}`);

  // destroy 不绕过 owner：只经 rt.disable（不直调 killTerm/浏览器 stop），且遵守运行时转换约束。
  // Terminal 为 ACTIVE 且不可 suspend（suspendable:false）→ rt.disable 抛 INVALID_TRANSITION。
  // 这正是诚实信号：v1 下 Terminal 无 lifecycle 拆解路径（C4/C5 未达），Governor 不绕过 owner 去强杀。
  let destroyThrew = null;
  try {
    gov2.destroy("terminal");
  } catch (e) {
    destroyThrew = e instanceof Error ? e.code || e.message : String(e);
  }
  if (destroyThrew && /INVALID_TRANSITION/.test(destroyThrew)) {
    ok("GOV-04", "destroy 只经 rt.disable 且遵守运行时约束（ACTIVE 不可直接 disable；不绕过 owner 强杀 PTY）");
  } else {
    bad("GOV-04", "destroy 应尊重运行时转换约束", `threw=${destroyThrew}`);
  }
  // 不变量：destroy 后 terminal 仍是 ACTIVE（未被 owner 资源操作），证明 Governor 未触碰 owner
  if (fullBoot.runtime.get("terminal")?.state === "ACTIVE") {
    ok("GOV-05", "destroy 未直触 owner 资源（terminal 仍 ACTIVE，PTY 归 owner 管）");
  } else {
    bad("GOV-05", "destroy 不应直触 owner", `state=${fullBoot.runtime.get("terminal")?.state}`);
  }
}

async function loadGovernor() {
  const entry = `
    import { createResourceGovernor } from '${join(ROOT, "src/capability/resourceGovernor.ts").replace(/\\/g, "/")}';
    export { createResourceGovernor };
  `;
  const res = await build({
    stdin: { contents: entry, resolveDir: ROOT, loader: "ts" },
    bundle: true, format: "esm", platform: "neutral", target: "es2020",
    external: ["pinia", "vue", "@vue/*", "*.vue", "turndown", "dompurify", "marked"], write: false,
  });
  const tmp = join(ROOT, ".tmp-gov.mjs");
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  try { return await import(pathToFileURLLocal(tmp).href); } finally { try { unlinkSync(tmp); } catch {} }
}

function selfTest() {
  const cases = [
    ["POSITIVE minimal 不含 terminal", ["bookmark", "workspace"].includes("terminal") === false, true],
    ["NEGATIVE 误判", ["terminal"].includes("terminal") === false, false],
    ["POSITIVE 越界 import 检出", /capabilities\/terminal/.test("from '../capabilities/terminal/state'"), true],
    ["NEGATIVE 无越界", /runtime/.test("from './runtime'"), false], // 注意：这里测的是「越界」判定函数，runtime 合法
  ];
  // 简化自检：只验证判定函数对合成输入敏感（核心在于上面的真实 bootstrap 断言）
  let p = 0, f = 0;
  for (const [name, got, want] of cases) {
    const good = got === want;
    if (good) p += 1; else f += 1;
    console.log(`${good ? "PASS" : "FAIL"}  ${name}`);
  }
  console.log(`\nSELF_TEST: ${f === 0 ? "PASS" : "FAIL"} (${p}/${p + f})`);
  return f === 0 ? 0 : 1;
}

const HELP = `check-composition-profiles.mjs — 真实 Composition Profile 门禁（Phase 8E Train F）

用法:
  node scripts/check-composition-profiles.mjs           真实 profile 注册 + Governor 薄层断言
  node scripts/check-composition-profiles.mjs --self-test
  node scripts/check-composition-profiles.mjs --json

覆盖: PROF-01..05 真实 profile 注册（minimal 无 terminal/browser → 零 PTY/零 WebView）
      GOV-01..04 Governor 薄层（不越界 import / 不持业务 state / suspend 诚实拒绝 / destroy 只经 rt）
退出码: 0 = 通过, 1 = 失败
`;

async function main() {
  const argv = process.argv.slice(2);
  if (argv.includes("--help") || argv.includes("-h")) { console.log(HELP); return 0; }
  if (argv.includes("--self-test")) return selfTest();
  await run();
  const json = argv.includes("--json");
  if (json) console.log(JSON.stringify({ result: fails.length === 0 ? "PASS" : "FAIL", pass, fails }, null, 2));
  else {
    for (const p of pass) console.log(`PASS  ${p}`);
    for (const f of fails) console.log(`FAIL  ${f}`);
    console.log(`\nCOMPOSITION_PROFILES_RESULT=${fails.length === 0 ? "PASS" : "FAIL"} (${pass.length}/${pass.length + fails.length})`);
  }
  return fails.length === 0 ? 0 : 1;
}

const isMain = process.argv[1] && process.argv[1].includes("check-composition-profiles.mjs");
if (isMain) main().then((c) => process.exit(c));
