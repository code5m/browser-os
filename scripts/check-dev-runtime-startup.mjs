#!/usr/bin/env node
/**
 * R12 — Fresh Dev Runtime Startup Gate（FINAL_HUMAN_ACCEPTANCE_H01_BLOCKER_REPAIR）
 *
 * 真实盲区：npm run build / npm run check 全 PASS，却没发现 npm run tauri dev
 * 会因 webview 持久化旧模块而 ENOENT（BUILD/L2 PASS ≠ DEV RUNTIME/L3-L4 PASS）。
 *
 * 本门禁在 fresh vite dev 启动中真实爬取模块图，证明：
 *   - Vite 对模块响应带 Cache-Control: no-store（H01 根因防御：dev 模块不持久化）
 *   - 启动图内无任何缺失/失败的模块（含 ENOENT 500）—— 正是 H01 的失败形态
 *   - 关键 capability 入口 + 真实组件文件可加载为合法 JS 模块（content-type=javascript）
 *   - TOMBSTONED_MODULE_REQUEST=0：tombstone 中的旧路径不构成合法模块（派生自 module-migrations.yaml）
 *   - NEGATIVE FIXTURE：人为请求一个不存在的模块必须不被识别为合法模块，证明门禁对
 *     「缺失模块」有区分力（这一能力本应捕获 H01 的 ENOENT）
 *
 * 关键：必须用 content-type 判定「是否真的是 JS 模块」。Vite SPA fallback 会对缺失路径
 * 返回 index.html（text/html, 200），若只看 status 会误判为 PASS。
 *
 * 证据等级（诚实标注）：
 *   R12A（本脚本，CI/无 GUI 可稳定执行）：Vite dev module smoke —— 覆盖 L3 DEV_MODULE_GRAPH。
 *   R12B（可选，需真实 dev 日志）：设环境变量 TAURI_DEV_LOG=/path/to/tauri-dev.log，
 *        解析实际 dev 运行日志中的 ENOENT / tombstoned 请求，断言为 0 —— 覆盖 L4 RUNTIME。
 *        未提供时 R12B 证据标记为 NOT_RUN（由人工 tauri dev 复测补足，见 H01_HUMAN_RETEST）。
 *
 * 仅用 Node 标准库 + 复用 check-capability-registry.mjs 的 YAML 子集解析器。
 */
import { spawn } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { resolve as pathResolve, dirname, join } from "node:path";
import { parseYaml } from "./check-capability-registry.mjs";

const ROOT = process.cwd();
const PORT = 1437; // 避开 1421，避免与正在运行的 tauri dev 冲突
// 允许对已有 dev server 跑门禁（R12B 真实运行时证据 / CI 已有 dev server 场景）：
//   R12_BASE_URL=http://localhost:1421 node scripts/check-dev-runtime-startup.mjs
const USE_EXTERNAL = !!process.env.R12_BASE_URL;
const BASE = USE_EXTERNAL ? process.env.R12_BASE_URL.replace(/\/$/, "") : `http://localhost:${PORT}`;
const VITE_BIN = pathResolve(ROOT, "node_modules/vite/bin/vite.js");
const TOMB = join(ROOT, "docs/architecture/module-resolution/module-migrations.yaml");

/** 取 tombstone 中的 TOMBSTONED from 路径（派生，不从脚本硬编码） */
function tombstonedPaths() {
  if (!existsSync(TOMB)) return [];
  const tomb = parseYaml(readFileSync(TOMB, "utf8"));
  return (tomb.migrations || [])
    .filter((m) => m.migration_status === "TOMBSTONED")
    .map((m) => "/" + m.from.split("\\").join("/"));
}

if (!existsSync(VITE_BIN)) {
  console.error("[R12] vite not found, skip (CI 环境可能缺 node_modules)");
  process.exit(0);
}

const results = [];
const record = (name, ok, info) => {
  results.push({ name, ok, info });
  console.log(`[${ok ? "PASS" : "FAIL"}] ${name}${info !== undefined ? " -> " + JSON.stringify(info) : ""}`);
};

const isValidModule = (res) =>
  res.status === 200 && /javascript|ecmascript/.test(res.contentType);

// ---- 启动 fresh vite dev（除非已指定外部 dev server）----
const vite = USE_EXTERNAL ? null : spawn(process.execPath, [VITE_BIN, "--port", String(PORT), "--strictPort", "--config", "vite.config.ts"], {
  cwd: ROOT,
  stdio: ["ignore", "pipe", "pipe"],
  env: { ...process.env, NODE_ENV: "development" },
});
let viteLog = "";
if (vite) {
  vite.stdout.on("data", (d) => (viteLog += d));
  vite.stderr.on("data", (d) => (viteLog += d));
}

let killed = false;
const killVite = () => {
  if (killed || !vite) return;
  killed = true;
  try { vite.kill("SIGTERM"); } catch { /* ignore */ }
};
process.on("exit", killVite);
process.on("SIGINT", () => { killVite(); process.exit(1); });

async function waitReady(timeoutMs = 60000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const r = await fetch(`${BASE}/src/main.ts`);
      if (r.status === 200 || r.status === 404) return true;
    } catch { /* not ready */ }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error("vite dev did not become ready in " + timeoutMs + "ms");
}

// ---- ESM import 提取 ----
const IMPORT_RE = /(?:import\s+(?:[^'"]*?\s+from\s+)?|export\s+[^'"]*?\s+from\s+|import\s*\(\s*)(['"])([^'"]+)\1/g;
function extractSpecifiers(src) {
  const out = new Set();
  let m;
  while ((m = IMPORT_RE.exec(src)) !== null) out.add(m[2]);
  return [...out];
}
function resolveSpec(spec, fromUrl) {
  if (/^(https?:)?\/\//.test(spec)) return null; // 外部 http(s)
  if (spec.startsWith("/")) return spec.split("?")[0].split("#")[0];
  if (spec.startsWith(".")) {
    const base = dirname(new URL(fromUrl).pathname);
    const merged = pathResolve(base, spec).replace(ROOT, ""); // 得到 /src/...
    return merged.split("?")[0].split("#")[0];
  }
  return null; // bare（node_modules）跳过
}

async function fetchText(url) {
  const r = await fetch(url);
  const body = await r.text();
  return {
    status: r.status,
    body,
    cacheControl: r.headers.get("cache-control") || "",
    contentType: r.headers.get("content-type") || "",
  };
}

const visited = new Set();
const failures = [];
async function crawl(url, depth) {
  if (depth > 14 || visited.size > 800) return;
  if (visited.has(url)) return;
  visited.add(url);
  let res;
  try {
    res = await fetchText(BASE + url);
  } catch (e) {
    failures.push({ url, status: "FETCH_ERR", contentType: "", info: e.message });
    return;
  }
  if (!isValidModule(res)) {
    const enoent = /ENOENT/.test(res.body);
    failures.push({ url, status: res.status, contentType: res.contentType, enoent });
    return;
  }
  const specs = extractSpecifiers(res.body);
  for (const s of specs) {
    const resolved = resolveSpec(s, url);
    if (resolved) await crawl(resolved, depth + 1);
  }
}

function isRealModuleCheck(name, res) {
  const ok = isValidModule(res);
  record(name, ok, { status: res.status, contentType: res.contentType });
  return ok;
}

// ---- R12B：可选解析真实 tauri dev 日志里的 ENOENT / tombstoned 请求 ----
function checkTauriDevLog(logPath, tombstoned) {
  if (!logPath || !existsSync(logPath)) {
    record("R12B tauri dev runtime log (optional)", true, { status: "NOT_RUN (no TAURI_DEV_LOG provided)" });
    return;
  }
  const txt = readFileSync(logPath, "utf8");
  const enoent = (txt.match(/ENOENT/g) || []).length;
  const tombHits = tombstoned.filter((p) => txt.includes(p.replace(/^\//, ""))).length;
  record("R12B ENOENT in tauri dev log = 0", enoent === 0, { count: enoent });
  record("R12B TOMBSTONED_REQUEST in tauri dev log = 0", tombHits === 0, { count: tombHits });
}

(async () => {
  try {
    const tombstoned = tombstonedPaths();
    await waitReady();

    // 1) 爬取整张启动模块图（从 /src/main.ts 出发）
    await crawl("/src/main.ts", 0);
    record("module-graph crawl completed", true, { visited: visited.size, failures: failures.length });

    // 2) 任何模块非合法 JS 模块（含 ENOENT 500 / SPA fallback html）即 FAIL —— H01 的失败形态
    if (failures.length > 0) {
      for (const f of failures) record(`module invalid ${f.status}${f.enoent ? " ENOENT" : ""}: ${f.url}`, false, f);
    } else {
      record("no missing/failed module in startup graph", true, { crawled: visited.size });
    }

    // 3) no-store 头（H01 根因防御）
    const mainRes = await fetchText(BASE + "/src/main.ts");
    const cc = mainRes.cacheControl;
    record("dev server Cache-Control no-store", /no-store/.test(cc), cc);
    isRealModuleCheck("main.ts is real module", mainRes);

    // 4) 关键 capability 入口 + 真实组件文件必须是合法 JS 模块
    const mustBeModules = [
      "/src/capabilities/home/index.ts",
      "/src/capabilities/vault/index.ts",
      "/src/capabilities/tools/index.ts",
      "/src/capabilities/home/ui/HomePanel.vue",
      "/src/capabilities/vault/ui/VaultPanel.vue",
      "/src/capabilities/tools/ui/ToolBox.vue",
    ];
    for (const m of mustBeModules) {
      const r = await fetchText(BASE + m);
      isRealModuleCheck(`real module ${m}`, r);
    }

    // 5) TOMBSTONED_MODULE_REQUEST=0：tombstone 旧路径绝不能是合法 JS 模块
    for (const p of tombstoned) {
      const r = await fetchText(BASE + p);
      record(`tombstoned path must NOT be a real module: ${p}`, !isValidModule(r), { status: r.status, contentType: r.contentType });
    }

    // 6) NEGATIVE FIXTURE：人为请求不存在的模块必须不被识别为合法模块
    //    （证明门禁对「缺失模块」有区分力 —— 正是 H01 的 ENOENT 会被本门禁捕获）
    const neg = await fetchText(BASE + "/src/__dev_runtime_negative_fixture_missing__.vue");
    record("negative fixture: missing module NOT a real module", !isValidModule(neg), { status: neg.status, contentType: neg.contentType });

    // 7) R12B（可选）：真实 tauri dev 日志
    checkTauriDevLog(process.env.TAURI_DEV_LOG, tombstoned);

    killVite();
  } catch (e) {
    killVite();
    record("gate execution", false, e.message);
  }

  const failCount = results.filter((r) => !r.ok).length;
  console.log(`R12_FRESH_DEV_RUNTIME=${failCount === 0 ? "PASS" : "FAIL"} (fail=${failCount})`);
  if (failCount > 0) console.log(`R12_VITE_LOG_TAIL:\n${viteLog.slice(-800)}`);
  process.exit(failCount === 0 ? 0 : 1);
})();
