#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-contribution-registration.mjs — Contribution Liveness 门禁（Wave 3）
//
// 用 esbuild 转译真实 src/capability/index.ts（不是复制逻辑另测），
// 按默认 full profile 真实调用 bootstrapCapabilityRuntime()，
// 然后检查 contributionRegistry 中关键贡献是否真的 live。
//
// 为什么需要它：home/vault/settings 都进了 ALL_CAPABILITIES，但它们的贡献
// 注册依赖「运行时激活 → lifecycle.onActivate → registerContribution」这一链路。
// 静态阅读无法证明贡献最终真的进了 Registry（vault/settings 历史上从未进入
// 任何 CAPABILITY_PROFILES，且其 onActivate/register 调用点缺失）。本脚本用真实
// 代码执行来判定 EXPECTED_COUNT == ACTUAL_REGISTRATION_PATH_COUNT。
//
// 目标贡献（均位于 WORKBENCH_MAIN 槽）：
//   home.main     — 控制样本：Wave 2 后 home 已纳入 full profile，必须 live
//   vault.main    — Vault M2 package：Host 经 onActivate 注册，需 full profile 激活
//   settings.main — framework SERVICE：贡献须无条件注册（不依赖 profile）
//
// 用法: node scripts/check-contribution-registration.mjs [profile] [--json]
// 退出码: 0 = 全部 live；1 = 至少一项缺失
// ---------------------------------------------------------------------------

import { build } from "esbuild";
import { writeFileSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SLOT = "workbench-main";

// 目标贡献：id → 期望在该 profile 下 live
// full 是默认 profile，home/vault/settings 都应 live。
const TARGETS = [
  { id: "home.main", label: "home (control sample)" },
  { id: "vault.main", label: "vault (M2 package)" },
  { id: "settings.main", label: "settings (framework SERVICE)" },
];

async function collect(profile) {
  const entry = `
    import { bootstrapCapabilityRuntime } from '${join(ROOT, "src/capability/index.ts").replace(/\\/g, "/")}';
    import { contributionRegistry } from '${join(ROOT, "src/capability/contribution/registry.ts").replace(/\\/g, "/")}';
    export { bootstrapCapabilityRuntime, contributionRegistry };
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
  const tmp = join(ROOT, `.tmp-contrib-${process.pid}.mjs`);
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  try {
    const mod = await import("file://" + tmp);
    mod.bootstrapCapabilityRuntime(profile);
    const live = mod.contributionRegistry
      .getBySlot(SLOT)
      .map((c) => c.id);
    return live;
  } finally {
    try { unlinkSync(tmp); } catch { /* ignore */ }
  }
}

async function run() {
  const argv = process.argv.slice(2);
  const profile = argv.find((a) => !a.startsWith("--")) || "full";
  const wantJson = argv.includes("--json");

  const live = await collect(profile);
  const pass = [];
  const fails = [];
  for (const t of TARGETS) {
    const present = live.includes(t.id);
    if (present) pass.push(`LIVE  ${t.id}  (${t.label})`);
    else fails.push(`MISSING  ${t.id}  (${t.label})`);
  }

  const result = fails.length === 0 ? "PASS" : "FAIL";
  if (wantJson) {
    console.log(JSON.stringify({
      result,
      profile,
      slot: SLOT,
      targets: TARGETS.map((t) => ({ id: t.id, live: live.includes(t.id) })),
      allWorkbenchMain: live,
    }, null, 2));
  } else {
    for (const p of pass) console.log(p);
    for (const f of fails) console.log(f);
    console.log(`\nworkbench-main live ids: ${live.join(", ") || "(none)"}`);
    console.log(`\nCONTRIBUTION_REGISTRATION_RESULT=${result} (profile=${profile})`);
  }
  return fails.length === 0 ? 0 : 1;
}

const isMain = process.argv[1] && process.argv[1].includes("check-contribution-registration.mjs");
if (isMain) run().then((c) => process.exit(c));
