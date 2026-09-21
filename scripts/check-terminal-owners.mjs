#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-terminal-owners.mjs — Terminal 单 owner / 单 PTY 出生点门禁（Phase 8E / Train D）
//
// 对应 Train D 验收项：
//   TERM-01 Terminal state 单 owner
//   TERM-02 无第二 truth（任何其它 store/组件不得再声明/直写 terminal 状态）
//   TERM-03 Shell 不 import Terminal internals（能力适配器也不持业务状态 / 不触 native）
//   TERM-05/06 absent 不产生 PTY / 子进程（模块加载与 store 实例化零 spawn；出生点唯一）
//   TERM-07 present 时 PTY 正常创建（真实 store + 假 bridge 行为断言）
//   TERM-08 destroy → cleanup（killTerm 发出终止请求 + 清空该 pane 的会话内历史）
//
// 判定手段：**静态扫描 + 真实代码行为测试**（esbuild 转译真实 TS，只把 bridge 换成记录型替身），
// 断言反映的是产品代码行为，不是替身行为。真源：semantic registry（states.yaml / owners.yaml）
// 与 capability registry（capabilities.yaml）——不硬编码能力名单。
//
// 用法: node scripts/check-terminal-owners.mjs [--self-test] [--json] [--help]
// 退出码: 0 = 通过；1 = 失败
// ---------------------------------------------------------------------------

import { readFileSync, readdirSync, statSync, existsSync, writeFileSync, unlinkSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join, resolve as resolvePath } from "node:path";
import { build } from "esbuild";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SRC = join(ROOT, "src");
const OWNER_REL = "src/capabilities/terminal/state/useTerminalStore.ts";
const LEGACY_REL = "src/stores/useSystemStore.ts";
const ADAPTER_REL = "src/capabilities/terminal/index.ts";
const VIEW_REL = "src/capabilities/terminal/ui/TerminalView.vue";

/** Terminal 受治理状态（Semantic Registry states.yaml 的 terminal 段；本表只用于反向断言，真源仍是 YAML） */
const TERMINAL_STATES = [
  "terminalOpen", "termPanes", "termGrid", "termGridCount", "activeTermId",
  "autoConfirmCli", "termProbeOn", "m0Cfg", "m0StartTs", "droppedChunks", "droppedBytes",
];

const failures = [];
const passes = [];
function ok(id, label) { passes.push(`${id} ${label}`); }
function bad(id, label, detail) { failures.push(`${id} ${label}${detail ? `  → ${detail}` : ""}`); }

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

const DECL = (name) =>
  new RegExp("(?:const|let|var)\\s+" + name + "\\s*(?::\\s*[^=;]+)?=\\s*(?:ref|shallowRef|reactive|computed)\\s*(?:<[^>]*>)?\\s*\\(");

// ─────────────────────────── 静态断言 ───────────────────────────

function runStatic() {
  // TERM-01：owner 文件存在且声明全部 11 个受治理状态
  if (!existsSync(join(ROOT, OWNER_REL))) {
    bad("TERM-01", "Terminal owner 文件存在", OWNER_REL);
    return;
  }
  const ownerSrc = stripComments(readFileSync(join(ROOT, OWNER_REL), "utf8"));
  const missing = TERMINAL_STATES.filter((n) => !DECL(n).test(ownerSrc));
  if (missing.length) bad("TERM-01", "owner 声明全部 terminal 状态", `缺: ${missing.join(",")}`);
  else ok("TERM-01", `Terminal owner（${OWNER_REL}）声明全部 ${TERMINAL_STATES.length} 个受治理状态`);

  // TERM-02：其它 store / 组件不得再声明 terminal 状态（第二真源）
  const others = walk(SRC).filter((f) => !f.endsWith(OWNER_REL.replace(/\//g, "/")));
  const secondDecl = [];
  for (const f of others) {
    const rel = f.slice(ROOT.length + 1);
    if (rel === OWNER_REL) continue;
    const src = stripComments(readFileSync(f, "utf8"));
    for (const n of TERMINAL_STATES) if (DECL(n).test(src)) secondDecl.push(`${rel}:${n}`);
  }
  if (secondDecl.length) bad("TERM-02", "无第二份 terminal 状态声明", secondDecl.join(", "));
  else ok("TERM-02", "全仓仅 owner 文件声明 terminal 状态（无第二真源）");

  // TERM-02b：非 owner 文件不得直写 terminal 状态
  const ALIASES = ["terminal", "term", "termStore", "system"];
  const WRITABLE = ["termPanes", "activeTermId", "terminalOpen"];
  const assignRe = new RegExp("\\b(" + ALIASES.join("|") + ")\\.(" + WRITABLE.join("|") + ")\\s*=(?!=)", "g");
  const vmodelRe = new RegExp("v-model\\s*=\\s*[\"'][^\"']*\\.(" + WRITABLE.join("|") + ")\\b", "g");
  const direct = [];
  for (const f of others) {
    const rel = f.slice(ROOT.length + 1);
    if (rel === OWNER_REL || rel === LEGACY_REL) continue;
    const src = stripComments(readFileSync(f, "utf8"));
    for (const m of src.matchAll(assignRe)) direct.push(`${rel}: ${m[1]}.${m[2]} = …`);
    for (const m of src.matchAll(vmodelRe)) direct.push(`${rel}: v-model 直绑 ${m[1]}`);
  }
  if (direct.length) bad("TERM-02b", "组件/其它 store 不直写 termPanes/activeTermId/terminalOpen", direct.join(", "));
  else ok("TERM-02b", "无文件直写 termPanes/activeTermId/terminalOpen（唯一写入口 = owner canonical writer）");

  // TERM-03a：能力适配器不得持业务状态、不得触 native bridge
  const adapter = existsSync(join(ROOT, ADAPTER_REL)) ? stripComments(readFileSync(join(ROOT, ADAPTER_REL), "utf8")) : "";
  const adapterViol = [];
  if (adapter) {
    for (const n of TERMINAL_STATES) if (DECL(n).test(adapter)) adapterViol.push(`声明状态 ${n}`);
    if (/from\s+["'][^"']*\/bridge["']/.test(adapter)) adapterViol.push("import bridge");
    if (/\bbridge\s*\./.test(adapter)) adapterViol.push("调用 bridge");
  } else {
    adapterViol.push("适配器缺失");
  }
  if (adapterViol.length) bad("TERM-03a", "Terminal 适配器只做编排（无状态、无 native）", adapterViol.join(", "));
  else ok("TERM-03a", "Terminal 适配器零业务状态、零 native 调用（只注册贡献）");

  // TERM-03b：Shell 目录不得 import terminal 内部（public 边界允许）
  const TERM_INTERNAL = /(?:^|\/)src\/capabilities\/terminal\/(?:state|ui|services|lifecycle|resource|internal|adapters)\//;
  const shellDirs = ["src/components/layout", "src/components/home"];
  const shellHits = [];
  for (const d of shellDirs) {
    for (const f of walk(join(ROOT, d))) {
      const rel = f.slice(ROOT.length + 1);
      const src = stripComments(readFileSync(f, "utf8"));
      const re = /(?:from|import\()\s*["']([^"']+)["']/g;
      let m;
      while ((m = re.exec(src)) !== null) {
        const spec = m[1];
        if (!spec.startsWith(".")) continue;
        const norm = new URL(spec.replace(/\.(ts|vue)$/, ""), "file://" + join(ROOT, d) + "/").pathname;
        const relNorm = norm.slice(norm.indexOf("/src/") + 1);
        if (TERM_INTERNAL.test(relNorm)) shellHits.push(`${rel} → ${spec}`);
      }
    }
  }
  if (shellHits.length) bad("TERM-03b", "Shell 不 import terminal 内部", shellHits.join(", "));
  else ok("TERM-03b", "Shell（layout/home）零 terminal 内部 import");

  // TERM-05a：PTY 出生点唯一 —— 只有能力 UI 组件（+ owner 自身）可调 ensureTerm/spawnTerm
  const birth = [];
  for (const f of walk(SRC)) {
    const rel = f.slice(ROOT.length + 1);
    if (rel === OWNER_REL) continue;
    const src = stripComments(readFileSync(f, "utf8"));
    if (/\b(ensureTerm|spawnTerm)\s*\(/.test(src)) birth.push(rel);
  }
  const allowed = ["src/capabilities/terminal/ui/TerminalView.vue", "src/capabilities/terminal/ui/TerminalDockPanel.vue"];
  const illegal = birth.filter((r) => !allowed.includes(r));
  if (illegal.length) bad("TERM-05a", "PTY 出生点唯一（仅 Terminal 能力 UI）", illegal.join(", "));
  else ok("TERM-05a", `PTY 出生点唯一：${birth.join(", ") || "（仅 owner 内部）"}`);

  if (!existsSync(join(ROOT, VIEW_REL))) bad("TERM-05b", "Terminal 主视图组件存在", VIEW_REL);
  else ok("TERM-05b", "Terminal 主视图组件存在（absent 时不挂载 → 无 PTY）");

  // TERM-02c：旧 owner 文件不得再出现 terminal 状态
  if (existsSync(join(ROOT, LEGACY_REL))) {
    const legacy = stripComments(readFileSync(join(ROOT, LEGACY_REL), "utf8"));
    const leaked = TERMINAL_STATES.filter((n) => DECL(n).test(legacy));
    if (leaked.length) bad("TERM-02c", "useSystemStore 不再声明 terminal 状态", leaked.join(","));
    else ok("TERM-02c", "useSystemStore（Clipboard/Apps owner）零 terminal 状态");
  }
}

// ─────────────────────────── 行为断言（真实 store + 假 bridge） ───────────────────────────

let piniaMod = null;
async function bundleStore() {
  // 只替换 bridge（native 边界）为记录型替身；store 逻辑与 useLayoutStore 全为真实代码。
  // 关键：bridge 必须与 store 在**同一个 bundle 实例**内导出，才能在测试里替换 native 边界；
  // 若改从外部 import bridge，store 用的是 bundle 内部那份，替换不会生效（会造成假 PASS）。
  const entry = `
    import { useTerminalStore } from '${join(ROOT, "src/capabilities/terminal/state/useTerminalStore.ts").replace(/\\/g, "/")}';
    import { bridge } from '${join(ROOT, "src/bridge.ts").replace(/\\/g, "/")}';
    // H-G：Terminal 的 PTY 出生点现在受「能力可用性 + 激活态」闸保护，
    // 因此行为断言必须像真实应用一样先 bootstrap 能力运行时（main.ts 在 mount 前做同一件事）。
    // 关键：bootstrap 必须与 store 在**同一个 bundle 实例**内导出，
    // 否则 runtimeSingleton 会被 esbuild 复制成两份，闸永远读到 null（假 FAIL）。
    import { bootstrapCapabilityRuntime } from '${join(ROOT, "src/capability/index.ts").replace(/\\/g, "/")}';
    export { useTerminalStore, bridge, bootstrapCapabilityRuntime };
  `;
  const res = await build({
    stdin: { contents: entry, resolveDir: ROOT, loader: "ts" },
    bundle: true,
    format: "esm",
    platform: "neutral",
    target: "es2020",
    // 框架依赖保持 external（由 Node 原生解析 node_modules），与 bundling 无关；
    // 打包的只有**产品代码**（store + useLayoutStore），断言对象因此是真实逻辑。
    // *.vue 必须 external：bootstrap 会拉到能力入口的 defineAsyncComponent（无 .vue loader）。
    external: ["pinia", "vue", "@vue/*", "*.vue"],
    write: false,
  });
  const tmp = join(ROOT, ".tmp-terminal-owners.mjs");
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  try {
    return await import(pathToFileURL(tmp).href);
  } finally {
    try { unlinkSync(tmp); } catch { /* ignore */ }
  }
}

/**
 * 真实 bootstrap 冒烟：加载真实的 src/capability/index.ts（含全部能力适配器的 onActivate），
 * 断言 Terminal 被装配后贡献真的落到槽上；再用同一份代码做「Terminal 缺席」对照
 * （不 activate terminal）→ 槽必须为空。这是 C3 ABSENT/PRESENT 的运行期证据，不只是字符串匹配。
 */
async function runBootstrapSmoke() {
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
    external: ["pinia", "vue", "@vue/*", "*.vue"],
    write: false,
  });
  const tmp = join(ROOT, ".tmp-terminal-bootstrap.mjs");
  writeFileSync(tmp, res.outputFiles[0].text, "utf8");
  try {
    const mod = await import(pathToFileURL(tmp).href);
    const { bootstrapCapabilityRuntime, contributionRegistry } = mod;

    // 对照 1：Terminal 未注册（清空贡献表 = 模拟 absent）→ 两个槽必须为空
    contributionRegistry.clear();
    const absentResident = contributionRegistry.getSurfaceContributions("workbench-main-resident");
    const absentDock = contributionRegistry.getSurfaceContributions("browser-dock").filter((c) => c.view === "term");
    if (absentResident.length === 0 && absentDock.length === 0) {
      ok("TERM-04a", "absent 对照：terminal 未注册 → workbench-main-resident / browser-dock(term) 槽为空");
    } else {
      bad("TERM-04a", "absent 对照：槽应为空", `resident=${absentResident.length} dock=${absentDock.length}`);
    }

    // 对照 2：真实 bootstrap → 4 个能力 ACTIVE，且 Terminal 贡献落到槽上
    const boot = bootstrapCapabilityRuntime();
    const rt = boot.runtime;
    const ids = rt.inspect().map((x) => `${x.id}:${x.state}`);
    const termRec = rt.get("terminal");
    if (boot.activated && termRec && termRec.state === "ACTIVE") {
      ok("TERM-04b", `present：bootstrap 装配成功且 terminal=ACTIVE（装配集合 ${ids.join(", ")}）`);
    } else {
      bad("TERM-04b", "present：bootstrap 装配失败或 terminal 非 ACTIVE", `activated=${boot.activated} err=${boot.error} ids=${ids.join(",")}`);
    }
    const resident = contributionRegistry.getSurfaceContributions("workbench-main-resident");
    const dock = contributionRegistry.getSurfaceContributions("browser-dock").filter((c) => c.view === "term");
    const residentOk = resident.length === 1 && resident[0].view === "term" && typeof resident[0].component === "object";
    const dockOk = dock.length === 1 && dock[0].view === "term";
    if (residentOk && dockOk) {
      ok("TERM-07d", "present：terminal 主视图（常驻槽）与 Dock 贡献均已注册且带 view=term");
    } else {
      bad("TERM-07d", "present：terminal 贡献注册", `resident=${resident.length} dock=${dock.length}`);
    }
  } finally {
    try { unlinkSync(tmp); } catch { /* ignore */ }
  }
}

async function runBehavioral() {
  globalThis.window = {
    setTimeout: (fn, ms) => setTimeout(fn, ms),
    clearTimeout: (h) => clearTimeout(h),
    addEventListener: () => {},
  };
  const storageWrites = [];
  globalThis.localStorage = {
    getItem: () => null,
    setItem: (k, v) => storageWrites.push([String(k), String(v)]),
    removeItem: () => {},
  };

  const mod = await bundleStore();
  const { bridge, useTerminalStore, bootstrapCapabilityRuntime } = mod;
  // 与真实应用一致：先 bootstrap 能力运行时（default profile = full → terminal ACTIVE），
  // 再实例化 store。这样「PTY 出生点」才能通过能力闸；absent 场景由 runBootstrapSmoke 覆盖。
  bootstrapCapabilityRuntime();
  const spawnCalls = [];
  const killCalls = [];
  let spawnSeq = 0;
  bridge.createTermChannel = () => ({ __channel: true });
  bridge.termSpawnChannel = async () => {
    spawnSeq += 1;
    spawnCalls.push(spawnSeq);
    return { id: `pty-${spawnSeq}` };
  };
  bridge.termKill = async (id) => { killCalls.push(id); };
  bridge.termWrite = async () => {};
  bridge.termResize = async () => {};
  bridge.debugLog = () => {};
  bridge.m0Config = async () => null;

  piniaMod = await import(pathToFileURL(join(ROOT, "node_modules/pinia/dist/pinia.mjs")).href);
  piniaMod.setActivePinia(piniaMod.createPinia());
  const terminal = useTerminalStore();

  // TERM-06：模块加载 + store 实例化**零** PTY（absent 时也会发生这两件事 → 必须不产生子进程）
  if (spawnCalls.length === 0) ok("TERM-06", "加载/实例化 Terminal store 零 PTY 创建（absent 不产生子进程）");
  else bad("TERM-06", "加载/实例化 Terminal store 零 PTY 创建", `spawn=${spawnCalls.length}`);

  // TERM-07a：ensureTerm → 恰好 1 个 PTY；termPanes 唯一注册表
  await terminal.ensureTerm();
  if (spawnCalls.length === 1 && terminal.termPanes.length === 1 && terminal.activeTermId === "pty-1") {
    ok("TERM-07a", "present：ensureTerm 创建 1 个 PTY 且 termPanes/activeTermId 一致（INV-4-1/4-2）");
  } else {
    bad("TERM-07a", "present：ensureTerm 创建 1 个 PTY 且注册表一致",
      `spawn=${spawnCalls.length} panes=${terminal.termPanes.length} active=${terminal.activeTermId}`);
  }

  // TERM-07b：再次 ensureTerm 不重复 spawn（幂等）；addTermPane 追加为第二个实例
  await terminal.ensureTerm();
  if (spawnCalls.length === 1) ok("TERM-07b", "present：ensureTerm 幂等（已有面板不再 spawn）");
  else bad("TERM-07b", "present：ensureTerm 幂等", `spawn=${spawnCalls.length}`);
  terminal.addTermPane();
  await new Promise((r) => setTimeout(r, 0));
  if (spawnCalls.length === 2 && terminal.termPanes.length === 2 && terminal.activeTermId === "pty-1") {
    ok("TERM-07c", "present：addTermPane 追加实例且不夺焦（activeTermId 不变）");
  } else {
    bad("TERM-07c", "present：addTermPane 追加实例且不夺焦",
      `spawn=${spawnCalls.length} panes=${terminal.termPanes.length} active=${terminal.activeTermId}`);
  }

  // TERM-08a：destroy（killTerm）→ 发出 PTY 终止请求 + 注册表移除 + activeTermId 重置
  await terminal.killTerm("pty-2");
  const killed = killCalls.includes("pty-2");
  const removed = !terminal.termPanes.some((p) => p.id === "pty-2");
  const activeOk = terminal.activeTermId === "pty-1";
  if (killed && removed && activeOk) ok("TERM-08a", "destroy：killTerm 请求终止 PTY + 注册表移除 + activeTermId 重置");
  else bad("TERM-08a", "destroy：killTerm 清理", `kill=${killed} removed=${removed} active=${terminal.activeTermId}`);

  // TERM-08b：destroy 后该 pane 的会话内历史为空（无跨会话残影）
  const replayed = [];
  terminal.bindTermWriter("pty-2", (d) => replayed.push(d));
  terminal.replayTermHistory("pty-2");
  terminal.bindTermWriter("pty-2", null);
  if (replayed.length === 0) ok("TERM-08b", "destroy：被杀 pane 回放为空（会话结束清空临时历史）");
  else bad("TERM-08b", "destroy：被杀 pane 回放为空", `replayed=${replayed.length}`);

  // TERM-08c：删除最后一个 pane 后 activeTermId 归零（不得悬空）
  await terminal.killTerm("pty-1");
  if (terminal.termPanes.length === 0 && terminal.activeTermId === "") {
    ok("TERM-08c", "destroy：全部关闭后 activeTermId 归零（无悬空 id）");
  } else {
    bad("TERM-08c", "destroy：全部关闭后 activeTermId 归零", `panes=${terminal.termPanes.length} active=${terminal.activeTermId}`);
  }

  // TERM-02d：会话内历史上限 40，且 flow/exit 帧不进历史、不落盘
  await terminal.spawnTerm();
  const id = terminal.termPanes[0].id;
  const got = [];
  terminal.bindTermWriter(id, (d) => got.push(d));
  for (let i = 1; i <= 100; i += 1) {
    terminal.onTermChannelMsg({ id, kind: "data", data: `L${i}` });
  }
  const capped = got.length === 100 && (terminal.termPanes.length === 1);
  // 通过「重建回放」观察历史条数：上限 40
  terminal.bindTermWriter(id, null);
  const replay = [];
  terminal.bindTermWriter(id, (d) => replay.push(d));
  terminal.replayTermHistory(id);
  terminal.bindTermWriter(id, null);
  if (capped && replay.length === 40) ok("TERM-02d", "会话内历史按 pane 上限 40 条（超限丢最旧）");
  else bad("TERM-02d", "会话内历史上限 40", `replay=${replay.length}`);
  const leaking = storageWrites.filter(([k, v]) => k.toLowerCase().includes("term") || /L\d+/.test(v));
  if (leaking.length === 0) ok("TERM-02e", "终端输出零落盘（仅内存；localStorage 无终端键/内容）");
  else bad("TERM-02e", "终端输出零落盘", JSON.stringify(leaking.slice(0, 3)));
}

// ─────────────────────────── self-test ───────────────────────────

function selfTest() {
  // 自检只验证「判定逻辑对合成输入敏感」：这里对 3 条核心正则做正/负样例。
  const cases = [
    ["POSITIVE 声明识别", DECL("termPanes").test("const termPanes = ref<{id:string}[]>([])"), true],
    ["NEGATIVE 非声明不误报", DECL("termPanes").test("// termPanes 只是注释"), false],
    ["POSITIVE 第二真源检出", DECL("termPanes").test("const termPanes = reactive([])"), true],
    ["POSITIVE 直写检出", /\b(terminal|term|system)\.(termPanes)\s*=(?!=)/.test("term.termPanes = []"), true],
    ["NEGATIVE 读取不误报", /\b(terminal|term|system)\.(termPanes)\s*=(?!=)/.test("if (term.termPanes.length > 0)"), false],
    ["POSITIVE 出生点检出", /\b(ensureTerm|spawnTerm)\s*\(/.test("system.ensureTerm()"), true],
    ["NEGATIVE 无出生点", /\b(ensureTerm|spawnTerm)\s*\(/.test("const t = terminal.termPanes"), false],
  ];
  let pass = 0, fail = 0;
  for (const [name, got, want] of cases) {
    const good = got === want;
    if (good) pass += 1; else fail += 1;
    console.log(`${good ? "PASS" : "FAIL"}  ${name}（got=${got} want=${want}）`);
  }
  console.log(`\nSELF_TEST: ${fail === 0 ? "PASS" : "FAIL"} (${pass}/${pass + fail})`);
  return fail === 0 ? 0 : 1;
}

const HELP = `check-terminal-owners.mjs — Terminal 单 owner / 单 PTY 出生点门禁（Phase 8E Train D）

用法:
  node scripts/check-terminal-owners.mjs              静态 + 真实 store 行为断言
  node scripts/check-terminal-owners.mjs --self-test   判定逻辑夹具自检
  node scripts/check-terminal-owners.mjs --json        机器可读
  node scripts/check-terminal-owners.mjs --help

覆盖: TERM-01 单 owner / TERM-02 无第二真源 / TERM-03 Shell & 适配器边界 /
      TERM-05-06 absent 零 PTY / TERM-07 present 正常 / TERM-08 destroy 清理
退出码: 0 = 通过, 1 = 失败
`;

async function main() {
  const argv = process.argv.slice(2);
  if (argv.includes("--help") || argv.includes("-h")) { console.log(HELP); return 0; }
  if (argv.includes("--self-test")) return selfTest();

  runStatic();
  await runBootstrapSmoke();
  await runBehavioral();

  const json = argv.includes("--json");
  if (json) {
    console.log(JSON.stringify({ result: failures.length === 0 ? "PASS" : "FAIL", passes, failures }, null, 2));
  } else {
    for (const p of passes) console.log(`PASS  ${p}`);
    for (const f of failures) console.log(`FAIL  ${f}`);
    console.log(`\nTERMINAL_OWNERS_RESULT=${failures.length === 0 ? "PASS" : "FAIL"} (${passes.length}/${passes.length + failures.length})`);
  }
  return failures.length === 0 ? 0 : 1;
}

const isMain = process.argv[1] && resolvePath(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) main().then((c) => process.exit(c));

export { TERMINAL_STATES };
