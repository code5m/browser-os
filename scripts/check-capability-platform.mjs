#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-platform.mjs — Universal Capability Platform v1 门禁
//
// 这是 Building Block Contract / Resolver / Assembly / Lifecycle / Hot-Plug 的
// **真实证据来源**：所有 maturity / HP 等级申报必须能被本脚本复现，禁止空口宣称。
//
// 判定手段：esbuild 转译真实 src（不是 mock），真实调用 assembly/hotplug/runtime。
//
// 用法: node scripts/check-capability-platform.mjs
// 退出码: 0 = 通过；1 = 失败
// ---------------------------------------------------------------------------

import { build } from "esbuild";
import { readFileSync, writeFileSync, unlinkSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const fails = [];
const pass = [];
const ok = (id, label) => pass.push(`${id} ${label}`);
const bad = (id, label, detail) => fails.push(`${id} ${label}${detail ? `  → ${detail}` : ""}`);
const expect = (cond, id, label, detail) => (cond ? ok(id, label) : bad(id, label, detail));

let tmpSeq = 0;
const tmpFiles = [];

async function loadRuntime() {
  const entry = `
    export { createCapabilityRuntime } from '${join(ROOT, "src/capability/runtime.ts").replace(/\\/g, "/")}';
    export { contributionRegistry, createContributionRegistry } from '${join(ROOT, "src/capability/contribution/registry.ts").replace(/\\/g, "/")}';
    export { CONTRIBUTION_SLOTS } from '${join(ROOT, "src/capability/contribution/types.ts").replace(/\\/g, "/")}';
    export { CAPABILITY_CATALOG, CAPABILITY_DEFINITIONS } from '${join(ROOT, "src/capability/platform/catalog.ts").replace(/\\/g, "/")}';
    export { assemble } from '${join(ROOT, "src/capability/platform/assembly.ts").replace(/\\/g, "/")}';
    export { validateManifestV1 } from '${join(ROOT, "src/capability/platform/contract.ts").replace(/\\/g, "/")}';
    export { canTransition, nextStates, assertTransition } from '${join(ROOT, "src/capability/platform/lifecycle.ts").replace(/\\/g, "/")}';
    export { createOrchestrator } from '${join(ROOT, "src/capability/platform/orchestrator.ts").replace(/\\/g, "/")}';
    export { bootstrapAssembly } from '${join(ROOT, "src/capability/index.ts").replace(/\\/g, "/")}';
    export { registerBookmarkContributions } from '${join(ROOT, "src/capabilities/bookmark/index.ts").replace(/\\/g, "/")}';
    export { registerWorkspaceContributions } from '${join(ROOT, "src/capabilities/workspace/index.ts").replace(/\\/g, "/")}';
    export { registerBrowserContributions } from '${join(ROOT, "src/capabilities/browser/index.ts").replace(/\\/g, "/")}';
    export { registerTerminalContributions } from '${join(ROOT, "src/capabilities/terminal/index.ts").replace(/\\/g, "/")}';
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
  const tmp = join(ROOT, `.tmp-platform-${tmpSeq++}.mjs`);
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  tmpFiles.push(tmp);
  return import(`file://${tmp}`);
}

/** 构造最小合法 fixture manifest（负例也需要是结构合法的 manifest） */
function mk(id, extra = {}) {
  return {
    id,
    version: "1.0.0",
    displayName: id,
    description: `fixture ${id}`,
    maturity: "C0",
    maturityEvidence: ["fixture"],
    dependencies: [],
    optionalDependencies: [],
    conflicts: [],
    provides: [`${id}.x`],
    requires: [],
    contributions: [{ id: `${id}.c`, slot: "browser-sidebar", type: "surface" }],
    permissions: [],
    resources: [],
    persistenceScope: "runtime_only",
    persistenceSensitive: false,
    activationPolicy: "manual",
    deactivationPolicy: "manual",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: { level: "HP0", enable: false, disable: false, register: false, unregister: false, install: false, uninstall: false, limitationReason: "fixture" },
    publicContract: [],
    entrypoint: `src/capabilities/${id}/index.ts`,
    semanticOwner: null,
    ...extra,
  };
}

async function main() {
  const M = await loadRuntime();
  const { CAPABILITY_CATALOG, CAPABILITY_DEFINITIONS, assemble, validateManifestV1 } = M;
  const { createCapabilityRuntime, contributionRegistry, CONTRIBUTION_SLOTS } = M;
  const { canTransition } = M;

  const ids = Object.keys(CAPABILITY_CATALOG).sort();
  const ALL_SLOTS = Object.values(CONTRIBUTION_SLOTS);

  function countContributions(registry, capabilityId) {
    let n = 0;
    for (const s of ALL_SLOTS) n += registry.getBySlot(s).filter((c) => c.capabilityId === capabilityId).length;
    return n;
  }
  function totalContributions(registry) {
    let n = 0;
    for (const s of ALL_SLOTS) n += registry.getBySlot(s).length;
    return n;
  }

  // ---- PLT2-01 catalog 存在且机器可读 -------------------------------------
  expect(ids.length >= 4, "PLT2-01", `capability catalog 机器可读（${ids.length} 项: ${ids.join(",")}）`,
    ids.length < 4 ? `catalog 过小: ${ids.length}` : "");

  // ---- PLT2-02 每个 manifest 通过 Building Block Contract v1 校验 ---------
  {
    const violations = [];
    for (const id of ids) for (const v of validateManifestV1(CAPABILITY_CATALOG[id])) violations.push(`${id}: ${v.code}`);
    expect(violations.length === 0, "PLT2-02", "全部 manifest 通过 v1 契约校验",
      violations.length ? violations.join("; ") : "");
  }

  // ---- PLT2-03 / PLT2-10 determinism + framework-only --------------------
  {
    const a = assemble(CAPABILITY_CATALOG, { capabilities: ["workspace", "browser"] });
    const b = assemble(CAPABILITY_CATALOG, { capabilities: ["browser", "workspace"] });
    expect(JSON.stringify(a) === JSON.stringify(b), "PLT2-03", "依赖解析 deterministic（输入顺序无关）");
    const fw = assemble(CAPABILITY_CATALOG, { capabilities: [] });
    expect(fw.ok && fw.resolved.length === 0 && fw.activationOrder.length === 0,
      "PLT2-10", "framework-only 装配通过（空能力集合法）",
      `ok=${fw.ok} resolved=${fw.resolved.length}`);
  }

  // ---- PLT2-04 强依赖缺失 → deterministic REJECT -------------------------
  {
    const cat = { ghost: mk("ghost", { dependencies: ["nope"] }) };
    const r = assemble(cat, { capabilities: ["ghost"] });
    const hasMissing = r.missingRequired.length > 0 && r.ok === false;
    expect(hasMissing, "PLT2-04", "强依赖缺失 → 确定性拒绝",
      `ok=${r.ok} missing=${r.missingRequired.length}`);
  }

  // ---- PLT2-05 依赖环 → REJECT --------------------------------------------
  {
    const cat = {
      a: mk("a", { dependencies: ["b"] }),
      b: mk("b", { dependencies: ["a"] }),
    };
    const r = assemble(cat, { capabilities: ["a"] });
    expect(r.cycles.length > 0 && r.ok === false, "PLT2-05", "依赖环 → 确定性拒绝",
      `cycles=${JSON.stringify(r.cycles)}`);
  }

  // ---- PLT2-06 冲突 → REJECT ---------------------------------------------
  {
    const cat = {
      a: mk("a", { conflicts: ["b"] }),
      b: mk("b"),
    };
    const r = assemble(cat, { capabilities: ["a", "b"] });
    expect(r.conflicts.length > 0 && r.ok === false, "PLT2-06", "冲突组合 → 确定性拒绝",
      `conflicts=${r.conflicts.length}`);
  }

  // ---- PLT2-07 未知 capability → REJECT ----------------------------------
  {
    const r = assemble(CAPABILITY_CATALOG, { capabilities: ["not-exist"] });
    expect(r.ok === false && r.unknown.length === 1, "PLT2-07", "未知能力 → 确定性拒绝",
      `unknown=${r.unknown.join(",")}`);
  }

  // ---- PLT2-08 optional 缺失 → 合法降级（不是拒绝） ----------------------
  {
    const r = assemble(CAPABILITY_CATALOG, { capabilities: ["bookmark"] });
    const degraded = r.ok === true && r.missingOptional.some((o) => o.id === "bookmark" && o.missing === "browser");
    expect(degraded, "PLT2-08", "optional 依赖缺失 → 合法降级运行（Bookmark 无 Browser）",
      `ok=${r.ok} optional=${JSON.stringify(r.missingOptional)}`);
  }

  // ---- PLT2-09 16 组合插拔矩阵（Bookmark/Workspace/Browser/Terminal） -----
  {
    const base = ["bookmark", "workspace", "browser", "terminal"];
    let valid = 0;
    const unexpected = [];
    for (let mask = 0; mask < 16; mask++) {
      const combo = base.filter((_, i) => mask & (1 << i));
      const r = assemble(CAPABILITY_CATALOG, { capabilities: combo });
      if (r.ok) {
        // 缺席 = 真正不在 resolved / 不产生其贡献与资源
        const missingOrExtra = base.filter((b) => !combo.includes(b) && r.resolved.includes(b));
        if (missingOrExtra.length > 0) unexpected.push(`[${combo.join("+")}] 未请求却出现: ${missingOrExtra.join(",")}`);
        else valid++;
      } else {
        unexpected.push(`[${combo.join("+") || "framework-only"}] 意外拒绝: ${r.rejections.map((x) => x.code).join(",")}`);
      }
    }
    expect(unexpected.length === 0 && valid === 16, "PLT2-09", `16 组合插拔矩阵确定性通过（valid=${valid}/16）`,
      unexpected.slice(0, 3).join(" | "));
  }

  // ---- PLT2-11 / PLT2-12 absent means absent（真实 bootstrap） ----------
  {
    contributionRegistry.clear();
    const rt = createCapabilityRuntime();
    // framework-only：一个能力都不注册
    expect(rt.inspect().length === 0, "PLT2-11", "framework-only bootstrap 成功且零能力注册");
    expect(totalContributions(contributionRegistry) === 0, "PLT2-11b", "framework-only 零贡献（Shell 无残留插头）",
      `contributions=${totalContributions(contributionRegistry)}`);

    // 只装配 bookmark+workspace → browser/terminal 的 contribution 必须为零
    const chosen = assemble(CAPABILITY_CATALOG, { capabilities: ["bookmark", "workspace"] });
    for (const id of chosen.resolved) rt.register(CAPABILITY_DEFINITIONS[id]);
    const browserContrib = chosen.contributions.filter((c) => c.id.startsWith("browser."));
    const terminalContrib = chosen.contributions.filter((c) => c.id.startsWith("terminal."));
    expect(browserContrib.length === 0, "PLT2-12a", "Browser absent → 零 Browser 贡献（无 WebView 宿主接入）");
    expect(terminalContrib.length === 0, "PLT2-12b", "Terminal absent → 零 Terminal 贡献（无 PTY 出生点）");
  }

  // ---- PLT2-19 Assembly Engine 驱动真实 bootstrap（非脚本内自嗨） -------
  {
    const empty = M.bootstrapAssembly([]);
    expect(empty.runtime.inspect().length === 0, "PLT2-19a", "bootstrapAssembly([]) → 真实零注册（framework-only）",
      `registered=${empty.runtime.inspect().map((e) => e.id).join(",")}`);
    const dev = M.bootstrapAssembly(["workspace", "browser"]);
    const regIds = dev.runtime.inspect().map((e) => e.id).sort();
    expect(
      regIds.length === 2 && regIds.includes("workspace") && regIds.includes("browser"),
      "PLT2-19b", `bootstrapAssembly([workspace,browser]) → 真实装配（${regIds.join(",")}）`,
      `registered=${regIds.join(",")}`);
    let threw = false;
    try { M.bootstrapAssembly(["not-exist"]); } catch { threw = true; }
    expect(threw, "PLT2-19c", "非法装配请求在启动前被确定性拒绝");
  }

  // ---- PLT2-13 lifecycle 非法迁移必须被拒 --------------------------------
  {
    const illegal = !canTransition("AVAILABLE", "ENABLED");
    let threw = false;
    try { M.assertTransition("AVAILABLE", "ENABLED"); } catch { threw = true; }
    expect(illegal && threw, "PLT2-13", "生命周期模型拒绝非法迁移（AVAILABLE ⇏ ENABLED）");
    expect(
      M.canTransition("REGISTERED", "ENABLED") && M.canTransition("ENABLED", "ACTIVE") && M.canTransition("ACTIVE", "DISABLED"),
      "PLT2-13b", "生命周期正向路径合法（REGISTERED→ENABLED→ACTIVE→DISABLED）");
  }

  // ---- PLT2-14 HP2 试点：Bookmark 运行时 register / unregister ------------
  {
    contributionRegistry.clear();
    const rt = createCapabilityRuntime();
    const orch = M.createOrchestrator({
      runtime: rt,
      registry: contributionRegistry,
      definitions: CAPABILITY_DEFINITIONS,
      loaders: { bookmark: () => M.registerBookmarkContributions() },
    });

    // register
    let added = null;
    try { added = await orch.add("bookmark"); } catch (e) { bad("PLT2-14a", "Bookmark 运行时 REGISTER 失败", e.message); }
    if (added) {
      const nAfterAdd = countContributions(contributionRegistry, "bookmark");
      const declared = CAPABILITY_CATALOG.bookmark.contributions.length;
      expect(nAfterAdd === declared && nAfterAdd > 0, "PLT2-14a",
        `Bookmark 运行时 REGISTER → 贡献出现（${nAfterAdd}/${declared}）`,
        `registered=${nAfterAdd} declared=${declared}`);
      // 契约漂移：真实注册的贡献 id 必须与 manifest 声明完全一致
      const realIds = ALL_SLOTS.flatMap((s) => contributionRegistry.getBySlot(s).filter((c) => c.capabilityId === "bookmark").map((c) => c.id)).sort();
      const declaredIds = CAPABILITY_CATALOG.bookmark.contributions.map((c) => c.id).sort();
      expect(JSON.stringify(realIds) === JSON.stringify(declaredIds), "PLT2-14b",
        "manifest 声明的贡献与真实注册完全一致（零契约漂移）",
        `real=${realIds.join(",")} declared=${declaredIds.join(",")}`);
      expect(orch.installedIds().includes("bookmark"), "PLT2-14c", "REGISTER 后 runtime 记录存在");

      // unregister
      try {
        const removed = await orch.remove("bookmark");
        const nAfterRemove = countContributions(contributionRegistry, "bookmark");
        expect(nAfterRemove === 0 && removed.state === "UNREGISTERED", "PLT2-14d",
          "Bookmark 运行时 UNREGISTER → 贡献消失且无残留",
          `remaining=${nAfterRemove} state=${removed.state}`);
        expect(!orch.installedIds().includes("bookmark"), "PLT2-14e", "UNREGISTER 后 runtime 记录已移除");
      } catch (e) {
        bad("PLT2-14d", "Bookmark 运行时 UNREGISTER 失败", e.message);
      }

      // 再次 register 必须成功（可重复插拔）
      try {
        contributionRegistry.clear();
        await orch.add("bookmark");
        expect(countContributions(contributionRegistry, "bookmark") > 0, "PLT2-14f", "UNREGISTER 后可再次 REGISTER（真插拔不是布尔量）");
      } catch (e) {
        bad("PLT2-14f", "再次 REGISTER 失败", e.message);
      }
    }
  }

  // ---- PLT2-15 存在强依赖方时拒绝 remove ---------------------------------
  {
    contributionRegistry.clear();
    const rt = createCapabilityRuntime();
    const dependent = mk("reader", { dependencies: ["bookmark"], maturity: "C0" });
    dependent.hotPlug = { level: "HP2", enable: true, disable: true, register: true, unregister: true, install: false, uninstall: false, limitationReason: "fixture" };
    // fixture 必须是**合法 CapabilityDefinition**（runtime.register 强校验），v1 内联其中
    const readerDef = {
      id: "reader",
      name: "reader",
      category: "CAPABILITY",
      provides: ["reader.x"],
      dependsOn: ["bookmark"],
      optionalDependencies: [],
      lifecycle: { supported: ["ACTIVE"], default: "ACTIVE", activatable: false, resident: false },
      resources: { class: [], suspendable: false, destroyable: false },
      permissions: [],
      persistence: { scope: "runtime_only", sensitive: false },
      entrypoint: "index.ts",
      semanticOwner: null,
      governanceStatus: "GOVERNED",
      status: "NOT_INTEGRATED",
      v1: dependent,
    };
    const defs = { ...CAPABILITY_DEFINITIONS, reader: readerDef };
    const orch = M.createOrchestrator({
      runtime: rt,
      registry: contributionRegistry,
      definitions: defs,
      loaders: { bookmark: () => M.registerBookmarkContributions(), reader: () => {} },
    });
    await orch.add("bookmark");
    await orch.add("reader");
    let rejected = false;
    try { await orch.remove("bookmark"); } catch (e) { rejected = e.code === "DEPENDENT_PRESENT"; }
    expect(rejected, "PLT2-15", "存在强依赖方时拒绝 hot-remove（reader → bookmark）",
      rejected ? "" : "未抛出 DEPENDENT_PRESENT");
  }

  // ---- PLT2-16 public contract locator 必须指向真实代码 ------------------
  {
    const missing = [];
    for (const id of ids) {
      for (const p of CAPABILITY_CATALOG[id].publicContract ?? []) {
        if (!existsSync(join(ROOT, p.locator))) missing.push(`${id}:${p.locator}`);
      }
    }
    expect(missing.length === 0, "PLT2-16", "public contract locator 全部指向真实代码", missing.join("; "));
  }

  // ---- PLT2-17 零第二真源（catalog 与 definition 同一对象） --------------
  {
    const same = ids.every((id) => CAPABILITY_CATALOG[id] === CAPABILITY_DEFINITIONS[id]?.v1);
    expect(same, "PLT2-17", "catalog 与能力定义内联同一对象（结构上不可能漂移）");
  }

  // ---- PLT2-18 HP 声明必须自洽且有理由 -----------------------------------
  {
    const problems = [];
    for (const id of ids) {
      const hp = CAPABILITY_CATALOG[id].hotPlug;
      if (hp.level !== "HP3" && !hp.limitationReason) problems.push(`${id} 缺 limitationReason`);
      if (hp.level === "HP0" && (hp.enable || hp.register)) problems.push(`${id} HP0 却声明运行时操作`);
      if (hp.level === "HP2" && !(hp.enable && hp.register && hp.unregister)) problems.push(`${id} HP2 声明不自洽`);
    }
    expect(problems.length === 0, "PLT2-18", "Hot-Plug 等级声明自洽且有诚实限制说明", problems.join("; "));
  }

  // 汇总
  for (const l of pass) console.log(`  PASS  ${l}`);
  for (const l of fails) console.log(`  FAIL  ${l}`);
  console.log(`\ncheck-capability-platform: ${pass.length} PASS / ${fails.length} FAIL`);

  for (const f of tmpFiles) { try { if (existsSync(f)) unlinkSync(f); } catch { /* ignore */ } }
  process.exit(fails.length === 0 ? 0 : 1);
}

main().catch((e) => {
  console.error("check-capability-platform 崩溃:", e);
  for (const f of tmpFiles) { try { if (existsSync(f)) unlinkSync(f); } catch { /* ignore */ } }
  process.exit(1);
});
