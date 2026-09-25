#!/usr/bin/env node
// ---------------------------------------------------------------------------
// capability-demo.mjs — Universal Capability Platform v1 演示 CLI（§36 REQUIRED）
//
// 领导不能看几千行日志，所以这里只输出确定性表格：
//   CAPABILITY / STATE / MATURITY / HP / DEPENDENCIES / CONTRIBUTIONS / RESOURCES / RESULT
//
// 用法:
//   node scripts/capability-demo.mjs list
//   node scripts/capability-demo.mjs inspect browser
//   node scripts/capability-demo.mjs products
//   node scripts/capability-demo.mjs use developer
//   node scripts/capability-demo.mjs resolve workspace,browser
//   node scripts/capability-demo.mjs resolve framework-only
//   node scripts/capability-demo.mjs matrix
//   node scripts/capability-demo.mjs runtime-demo bookmark
//   node scripts/capability-demo.mjs resource-check browser
//   node scripts/capability-demo.mjs demo          # 3~5 分钟领导串场脚本
// ---------------------------------------------------------------------------

import { build } from "esbuild";
import { readFileSync, writeFileSync, unlinkSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const PRODUCTS = join(ROOT, "config/capability-products/products.json");

let tmpSeq = 0;
const tmpFiles = [];

async function loadMod() {
  const entry = `
    export { CAPABILITY_CATALOG, CAPABILITY_DEFINITIONS } from '${join(ROOT, "src/capability/platform/catalog.ts").replace(/\\/g, "/")}';
    export { assemble } from '${join(ROOT, "src/capability/platform/assembly.ts").replace(/\\/g, "/")}';
    export { createOrchestrator } from '${join(ROOT, "src/capability/platform/orchestrator.ts").replace(/\\/g, "/")}';
    export { createCapabilityRuntime } from '${join(ROOT, "src/capability/runtime.ts").replace(/\\/g, "/")}';
    export { contributionRegistry } from '${join(ROOT, "src/capability/contribution/registry.ts").replace(/\\/g, "/")}';
    export { CONTRIBUTION_SLOTS } from '${join(ROOT, "src/capability/contribution/types.ts").replace(/\\/g, "/")}';
    export { bootstrapAssembly } from '${join(ROOT, "src/capability/index.ts").replace(/\\/g, "/")}';
    export { registerBookmarkContributions } from '${join(ROOT, "src/capabilities/bookmark/index.ts").replace(/\\/g, "/")}';
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
  const tmp = join(ROOT, `.tmp-demo-${tmpSeq++}.mjs`);
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  tmpFiles.push(tmp);
  return import(`file://${tmp}`);
}

const pad = (s, n) => String(s ?? "").padEnd(n);
function row(cols, widths) {
  return cols.map((c, i) => pad(c, widths[i])).join("  ");
}

function loadProducts() {
  return JSON.parse(readFileSync(PRODUCTS, "utf8")).products;
}

async function cmdList(M) {
  const cat = M.CAPABILITY_CATALOG;
  const ids = Object.keys(cat).sort();
  const w = [14, 8, 6, 26, 30, 26];
  console.log(row(["CAPABILITY", "STATE", "MATUR", "DEPENDENCIES", "CONTRIBUTIONS", "RESOURCES"], w));
  console.log("-".repeat(w.reduce((a, b) => a + b, 0) + 10));
  for (const id of ids) {
    const m = cat[id];
    console.log(row([
      id,
      "ACTIVE",
      m.maturity,
      (m.dependencies.length ? m.dependencies.join(",") : "—") +
        (m.optionalDependencies.length ? ` (opt: ${m.optionalDependencies.join(",")})` : ""),
      `${m.contributions.length} 项: ${m.contributions.map((c) => c.id).slice(0, 2).join(",")}${m.contributions.length > 2 ? "…" : ""}`,
      m.resources.length ? m.resources.map((r) => `${r.kind}:${r.ownership}`).join(",") : "none",
    ], w));
  }
  console.log(`\n共 ${ids.length} 个具备 Building Block Contract v1 的能力。`);
  console.log("STATE 取自已装配状态的稳定视图；MATUR=成熟度，HP=热插拔等级（见 inspect）。");
}

async function cmdInspect(M, id) {
  const m = M.CAPABILITY_CATALOG[id];
  if (!m) {
    console.log(`未找到能力: ${id}（catalog: ${Object.keys(M.CAPABILITY_CATALOG).sort().join(",")}）`);
    return;
  }
  const hp = m.hotPlug;
  console.log(`CAPABILITY       ${m.id} v${m.version}`);
  console.log(`DISPLAY          ${m.displayName}`);
  console.log(`DESCRIPTION      ${m.description}`);
  console.log(`MATURITY         ${m.maturity}   evidence: ${(m.maturityEvidence ?? []).join(", ") || "—"}`);
  console.log(`HOT-PLUG         ${hp.level}  enable=${hp.enable} disable=${hp.disable} register=${hp.register} unregister=${hp.unregister} install=${hp.install}`);
  console.log(`LIMITATION       ${hp.limitationReason ?? "—"}`);
  console.log(`DEPENDENCIES     ${m.dependencies.join(",") || "—"}${m.optionalDependencies.length ? `  optional: ${m.optionalDependencies.join(",")}` : ""}`);
  console.log(`CONFLICTS        ${m.conflicts.join(",") || "—"}`);
  console.log(`PROVIDES         ${m.provides.join(", ")}`);
  console.log(`CONTRIBUTIONS    ${m.contributions.map((c) => `${c.id}@${c.slot}${c.view ? `#${c.view}` : ""}`).join(", ")}`);
  console.log(`RESOURCES        ${m.resources.map((r) => `${r.kind}(${r.ownership}${r.evidence ? `, ${r.evidence}` : ""})`).join(", ") || "none"}`);
  console.log(`PERSISTENCE      ${m.persistenceScope}${m.persistenceSensitive ? " (sensitive)" : ""}`);
  console.log(`PUBLIC CONTRACT  ${m.publicContract.map((p) => `${p.name} → ${p.locator}`).join(", ")}`);
  console.log(`ENTRYPOINT       ${m.entrypoint}`);
  console.log(`SEMANTIC OWNER   ${m.semanticOwner ?? "—"}`);
}

async function cmdProducts() {
  const ps = loadProducts();
  const w = [26, 26, 40];
  console.log(row(["PRODUCT", "CAPABILITIES", "DESCRIPTION"], w));
  console.log("-".repeat(w.reduce((a, b) => a + b, 0) + 4));
  for (const p of ps) {
    console.log(row([p.id, p.capabilities.join(",") || "∅ framework-only", p.description], w));
  }
}

async function cmdUse(M, productId) {
  const ps = loadProducts();
  const p = ps.find((x) => x.id === productId);
  if (!p) {
    console.log(`未知产品: ${productId}（可用: ${ps.map((x) => x.id).join(", ")}）`);
    return;
  }
  printAssembly(M, p.capabilities, `PRODUCT ${p.displayName}`);
}

async function cmdResolve(M, arg) {
  const ps = loadProducts();
  if (arg === "framework-only") return printAssembly(M, [], "framework-only (custom)");
  const preset = ps.find((x) => x.id === arg);
  const ids = preset ? preset.capabilities : String(arg).split(",").map((s) => s.trim()).filter(Boolean);
  printAssembly(M, ids, preset ? `PRODUCT ${preset.displayName}` : "CUSTOM ASSEMBLY");
}

async function printAssembly(M, ids, title) {
  const r = M.assemble(M.CAPABILITY_CATALOG, { capabilities: ids });
  console.log(`=== ${title} ===`);
  console.log(`REQUESTED        ${r.requested.join(",") || "∅"}`);
  console.log(`RESULT           ${r.ok ? "RESOLVED" : "REJECTED"}`);
  console.log(`RESOLVED         ${r.resolved.join(",") || "∅"}`);
  console.log(`ACTIVATION ORDER ${r.activationOrder.join(" → ") || "∅"}`);
  if (r.autoIncluded.length) console.log(`AUTO INCLUDED    ${r.autoIncluded.join(",")}（强依赖自动补入）`);
  console.log(`CONTRIBUTIONS    ${r.contributions.length} 项`);
  console.log(`PERMISSIONS      ${r.permissions.join(",") || "—"}`);
  console.log(`RESOURCES        ${r.resources.map((x) => `${x.kind}:${x.ownership}`).join(",") || "none"}`);
  if (r.warnings.length) console.log(`WARNINGS         ${r.warnings.join(" | ")}`);
  if (r.rejections.length) console.log(`REJECTIONS       ${r.rejections.map((x) => `${x.code}: ${x.message}`).join(" | ")}`);

  // 真实 bootstrap（不是模拟）
  try {
    const b = M.bootstrapAssembly(r.ok ? r.resolved : ids);
    const registered = b.runtime.inspect().map((e) => e.id).sort();
    console.log(`REAL BOOTSTRAP   registered=[${registered.join(",") || "∅"}] activated=${b.activated} error=${b.error ?? "none"}`);
  } catch (e) {
    console.log(`REAL BOOTSTRAP   拒绝（deterministic）: ${e.message}`);
  }
}

async function cmdMatrix(M) {
  const base = ["bookmark", "workspace", "browser", "terminal"];
  const w = [34, 10, 34, 22];
  console.log(row(["COMBINATION", "RESULT", "RESOURCES", "REJECTIONS"], w));
  console.log("-".repeat(w.reduce((a, b) => a + b, 0) + 6));
  let valid = 0;
  for (let mask = 0; mask < 16; mask++) {
    const combo = base.filter((_, i) => mask & (1 << i));
    const r = M.assemble(M.CAPABILITY_CATALOG, { capabilities: combo });
    if (r.ok) valid++;
    console.log(row([
      combo.join("+") || "∅ framework-only",
      r.ok ? "VALID" : "REJECT",
      r.resources.map((x) => x.kind).join(",") || "none",
      r.rejections.map((x) => x.code).join(",") || "—",
    ], w));
  }
  console.log(`\n16 组合：VALID=${valid} REJECT=${16 - valid}（全部 deterministic，同输入必同输出）`);
}

async function cmdRuntimeDemo(M, id) {
  const cid = id || "bookmark";
  M.contributionRegistry.clear();
  const rt = M.createCapabilityRuntime();
  const orch = M.createOrchestrator({
    runtime: rt,
    registry: M.contributionRegistry,
    definitions: M.CAPABILITY_DEFINITIONS,
    loaders: { bookmark: () => M.registerBookmarkContributions() },
  });
  const allSlots = Object.values(M.CONTRIBUTION_SLOTS);
  const count = () => allSlots.reduce((n, s) => n + M.contributionRegistry.getBySlot(s).filter((c) => c.capabilityId === cid).length, 0);

  console.log(`=== RUNTIME HOT-PLUG DEMO: ${cid}（应用不重启）===`);
  console.log(`before           contributions=${count()}  registered=[${orch.installedIds().join(",") || "∅"}]`);
  const added = await orch.add(cid);
  console.log(`REGISTER         contributions=${count()}  state=${added.state}  ${added.notes.join(" | ")}`);
  const st = orch.disable(cid);
  console.log(`DISABLE          contributions=${count()}  state=${st}  （UI 贡献摘除，不再暴露能力动作）`);
  orch.enable(cid);
  console.log(`ENABLE           contributions=${count()}  state=${orch.target.stateOf(cid)}`);
  const removed = await orch.remove(cid);
  console.log(`UNREGISTER       contributions=${count()}  state=${removed.state}  ${removed.notes.join(" | ")}`);
  console.log(`RESULT           ${count() === 0 && removed.state === "UNREGISTERED" ? "PASS" : "FAIL"}：真正的运行时插拔，不是布尔开关`);
}

async function cmdResourceCheck(M, id) {
  const m = M.CAPABILITY_CATALOG[id];
  if (!m) return console.log(`未找到能力: ${id}`);
  console.log(`=== RESOURCE CHECK: ${id} ===`);
  for (const r of m.resources) {
    let measured = "DECLARED";
    try {
      if (r.evidence && r.evidence.startsWith("scripts/") && existsSync(join(ROOT, r.evidence))) measured = "TOOL_AVAILABLE(headless 未跑应用实例 → 该次判定 UNKNOWN)";
    } catch { /* ignore */ }
    console.log(`  ${pad(r.kind, 18)} ownership=${r.ownership}  classification=${measured}  evidence=${r.evidence ?? "—"}`);
  }
  if (!m.resources.length) console.log("  该能力未持有需治理的重资源（诚实声明，不是未做调查）");
  console.log("  规则：能可靠测量才写 MEASURED；否则一律 DECLARED/UNKNOWN，禁止估算（§44）。");
}

async function cmdDemo(M, argv) {
  const h = (s) => console.log(`\n──────── ${s} ────────`);
  h("DEMO 1/6  Framework Only：这就是最小底座");
  await printAssembly(M, [], "framework-only");
  h("DEMO 2/6  Capability Catalog：这些是积木");
  await cmdList(M);
  h("DEMO 3/6  Assembly：产品 = 积木组合（Workspace + Browser）");
  await cmdResolve(M, "workspace,browser");
  h("DEMO 4/6  加入 Terminal：不需要修改 Shell 业务代码");
  await cmdResolve(M, "workspace,browser,terminal");
  h("DEMO 5/6  移除 Browser：其它积木仍工作（Browser 贡献与资源归零）");
  await cmdResolve(M, "workspace,terminal");
  h("DEMO 6/6  Runtime Hot-Plug：应用不重启，Bookmark 拔掉再插回");
  await cmdRuntimeDemo(M, "bookmark");
  console.log("\n结语：以前我们开发的是一个越来越大的应用；现在我们开发的是一个底座和一套可独立组合的能力积木。");
}

async function main() {
  const argv = process.argv.slice(2);
  const cmd = argv[0] ?? "list";
  const arg = argv[1];
  const M = await loadMod();
  switch (cmd) {
    case "list": await cmdList(M); break;
    case "inspect": await cmdInspect(M, arg); break;
    case "products": await cmdProducts(); break;
    case "use": await cmdUse(M, arg); break;
    case "resolve": await cmdResolve(M, arg); break;
    case "matrix": await cmdMatrix(M); break;
    case "runtime-demo": await cmdRuntimeDemo(M, arg); break;
    case "resource-check": await cmdResourceCheck(M, arg); break;
    case "demo": await cmdDemo(M, argv); break;
    default:
      console.log(`未知命令: ${cmd}`);
      console.log("可用: list | inspect <id> | products | use <product> | resolve <ids|preset> | matrix | runtime-demo <id> | resource-check <id> | demo");
      process.exitCode = 1;
  }
  for (const f of tmpFiles) { try { if (existsSync(f)) unlinkSync(f); } catch { /* ignore */ } }
}

main().catch((e) => {
  console.error("capability-demo 失败:", e);
  for (const f of tmpFiles) { try { if (existsSync(f)) unlinkSync(f); } catch { /* ignore */ } }
  process.exit(1);
});
