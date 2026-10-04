#!/usr/bin/env node
// ---------------------------------------------------------------------------
// Terminal 前端逻辑层自动化测试（headless，无 GUI 依赖）
//
// Phase 8E / Train D 重基线：本脚本原对接 M3.c 时期的**单终端** store API
// （startShell / termId / termHistory / killShell / bindTermWriter(fn)），
// 而 commit 2a96cb1 已把 store 改为**多面板（per-pane）**语义，之后本脚本一直未被同步
// —— 属 PRE_EXISTING CHECKER DEBT（证据见
// docs/architecture/capability-modularization/phase8e/TRAIN-D-TERMINAL-AUDIT.md §6.1）。
// 本次把断言重新对接到 per-pane 真实 API，并**新增**若干断言（不删断言、不放宽不变量）：
//   - 历史/回放/清理改为 per-pane（每个 pane 独立 40 条，互不串味）
//   - 新增「killTerm 后该 pane 回放为空」「重启后的新 pane 无旧会话残影」
//
// 直接加载**真实的** `src/capabilities/terminal/state/useTerminalStore.ts` 与
// `src/capabilities/terminal/ui/useTerminalResize.ts`，只把 `src/bridge.ts` 的终端方法
// 替换为记录型 mock（不 mock store / composable 自身逻辑）：下面每条断言反映的都是
// **产品代码**的行为，而非测试替身的行为。
//
// 用法: node scripts/check-terminal-ui-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";

function resolveWithExt(specifier, context, next) {
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {}
      }
    }
    throw err;
  }
}

if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," +
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__m3cResolve(specifier, context, next);
}`
      )
  );
  globalThis.__m3cResolve = resolveWithExt;
}

// ---------- 最小浏览器桩（只补产品代码真正用到的全局） ----------
const storageWrites = [];
globalThis.window = {
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  clearTimeout: (h) => clearTimeout(h),
  addEventListener: () => {},
};
globalThis.localStorage = {
  getItem: () => null,
  setItem: (k, v) => storageWrites.push([String(k), String(v)]),
  removeItem: () => {},
};

const ROOT = new URL("..", import.meta.url).pathname;

const { bridge } = await import(`${ROOT}src/bridge.ts`);
const { useTerminalStore } = await import(
  `${ROOT}src/capabilities/terminal/state/useTerminalStore.ts`
);
const { useTerminalResize, TERM_RESIZE_QUIET_MS, TERM_RESIZE_MAX_WAIT_MS } = await import(
  `${ROOT}src/capabilities/terminal/ui/useTerminalResize.ts`
);
const { createPinia, setActivePinia } = await import(`${ROOT}node_modules/pinia/dist/pinia.mjs`);
const { createCapabilityRuntime } = await import(`${ROOT}src/capability/runtime.ts`);
const { setCapabilityRuntime } = await import(`${ROOT}src/capability/runtimeSingleton.ts`);
const { terminalManifest } = await import(`${ROOT}src/capabilities/terminal/manifest.ts`);

// ---------- mock bridge（只替换终端相关方法，记录调用） ----------
const calls = [];
let spawnSeq = 0;
bridge.createTermChannel = () => ({ __channel: true });
bridge.termSpawnChannel = async () => ({ id: `term-${++spawnSeq}` });
bridge.termSpawn = async () => {
  throw new Error("前端不得调用 term_spawn（Event 广播）");
};
bridge.termWrite = async (id, data) => {
  calls.push(["write", id, data]);
};
bridge.termResize = async (id, cols, rows) => {
  calls.push(["resize", id, cols, rows]);
};
bridge.termKill = async (id) => {
  calls.push(["kill", id]);
};
bridge.debugLog = () => {};
bridge.m0Config = async () => null;

let passed = 0;
const failures = [];
function assert(cond, label) {
  if (cond) {
    passed += 1;
  } else {
    failures.push(label);
    console.error(`  ✗ ${label}`);
  }
}

setActivePinia(createPinia());
const terminal = useTerminalStore();

/** 采集某 pane 重建时的回放内容（模拟面板重新挂载）。 */
function replayOf(id) {
  const out = [];
  terminal.bindTermWriter(id, (d) => out.push(d));
  terminal.replayTermHistory(id);
  return out;
}

// ===========================================================================
// E1 per-pane 临时历史 40 条
// ===========================================================================

// T1：还没有 writer 时回放不得抛错（面板未挂载的边界）。
terminal.replayTermHistory("not-exist");
assert(true, "E1-T1 replayTermHistory 在无 writer / 未知 pane 时安全 no-op");

assert(await terminal.spawnTerm() === "" && spawnSeq === 0, "E1-T1b 未装配 Terminal 时不得创建 PTY");
// 与产品启动顺序一致，使用真实 Runtime 和 Manifest 打开资源闸；不 mock 守门逻辑。
const runtime = createCapabilityRuntime();
runtime.register(terminalManifest);
runtime.resolve("terminal");
runtime.activate("terminal");
setCapabilityRuntime(runtime);

const p1 = await terminal.spawnTerm();
assert(p1 === "term-1", `E1-T2a spawnTerm 返回 pane id（实得 ${p1}）`);
assert(
  terminal.termPanes.length === 1 && terminal.activeTermId === "term-1",
  "E1-T2b 首个 pane 自动成为 activeTermId（INV-4-2）"
);

// T3：灌入 100 条输出 → 该 pane 历史上限 40，且保留的是**最新**的 40 条。
const live = [];
terminal.bindTermWriter(p1, (d) => live.push(d));
for (let i = 1; i <= 100; i += 1) {
  terminal.onTermChannelMsg({ id: p1, kind: "data", data: `OUT-${String(i).padStart(4, "0")}` });
}
assert(live.length === 100, `E1-T3a 100 条输出全部实时投递（实得 ${live.length}）`);
terminal.bindTermWriter(p1, null);
const r1 = replayOf(p1);
assert(r1.length === 40, `E1-T3b 历史上限生效（期望 40，实得 ${r1.length}）`);
assert(
  r1[0] === "OUT-0061" && r1[39] === "OUT-0100",
  "E1-T3c 超限后保留最新 40 条（丢弃最旧的）"
);

// T4：flow / exit 帧不进历史（只有 PTY 输出块才算历史）。
const before = replayOf(p1).length;
terminal.onTermChannelMsg({ id: p1, kind: "flow", dropped_chunks: 3, dropped_bytes: 30 });
terminal.onTermChannelMsg({ id: p1, kind: "exit", reason: "eof", data: "\r\n[终端已退出]\r\n" });
assert(replayOf(p1).length === before, "E1-T4 flow/exit 帧不写入临时历史");
assert(
  terminal.droppedChunks === 3 && terminal.droppedBytes === 30,
  "E1-T4b flow 帧进丢弃遥测（droppedChunks/droppedBytes）"
);

// T5：面板重建（旧 writer 解绑 → 新 writer 绑定）后回放，内容等于历史且**不自喂**。
const replayed = replayOf(p1);
assert(replayed.length === 40, `E1-T5a 重建后回放 40 条（实得 ${replayed.length}）`);
assert(
  replayed.join("") === r1.join(""),
  "E1-T5b 回放内容与历史一致"
);
assert(replayOf(p1).length === 40, "E1-T5c 回放不回写历史（长度不翻倍）");

// T5d：per-pane 隔离 —— 第二个 pane 的历史不含第一个 pane 的内容。
const p2 = await terminal.spawnTerm();
assert(terminal.termPanes.length === 2 && terminal.activeTermId === p1, "E1-T5d 第二个 pane 不夺焦");
for (let i = 1; i <= 5; i += 1) {
  terminal.onTermChannelMsg({ id: p2, kind: "data", data: `P2-${i}` });
}
const p2replay = replayOf(p2);
assert(
  p2replay.length === 5 && p2replay.every((d) => d.startsWith("P2-")),
  `E1-T5e per-pane 历史互不串味（实得 ${JSON.stringify(p2replay)}）`
);
assert(replayOf(p1).length === 40, "E1-T5f 新 pane 的输出不影响旧 pane 历史");

// T6：killTerm 后该 pane 历史清空（会话结束即消亡，不残留到下一段会话）。
await terminal.killTerm(p2);
assert(!terminal.termPanes.some((p) => p.id === p2), "E1-T6a killTerm 从注册表移除");
assert(
  terminal.activeTermId === p1,
  `E1-T6b activeTermId 不悬空（实得 ${terminal.activeTermId}）`
);
assert(replayOf(p2).length === 0, "E1-T6c 被杀 pane 回放为空（无跨会话残影）");

// T7：重启（kill + spawn）后新 pane 同样空，旧会话输出不得作为残影回放到新终端。
terminal.onTermChannelMsg({ id: p1, kind: "data", data: "OLD-RESIDUE" });
const afterResidue = replayOf(p1);
assert(
  afterResidue.length === 40 && afterResidue[afterResidue.length - 1] === "OLD-RESIDUE",
  "E1-T7a 旧会话历史已累积且上限仍生效（保留最新）"
);
await terminal.killTerm(p1);
const p3 = await terminal.spawnTerm();
assert(replayOf(p3).length === 0, "E1-T7b 重启新 pane 历史为空（无跨会话残影）");

// T8：不落盘 —— 全流程跑完后，所有 localStorage 写入都不得含终端输出内容，
//     也不得出现终端历史键（历史只存在于内存）。
for (let i = 0; i < 60; i += 1) {
  terminal.onTermChannelMsg({ id: p3, kind: "data", data: `SECRET-${i}-sk-abc` });
}
await terminal.killTerm(p3);
const leaked = storageWrites.filter(
  ([k, v]) =>
    k.toLowerCase().includes("term") ||
    v.includes("OUT-") ||
    v.includes("SECRET-") ||
    v.includes("P2-") ||
    v.includes("OLD-RESIDUE")
);
assert(leaked.length === 0, `E1-T8 终端输出/历史零落盘（泄漏 ${leaked.length} 条）`);

// ===========================================================================
// E2 resize 静默窗口（假时钟，行为完全确定）
// ===========================================================================

function fakeClock() {
  let now = 10_000;
  const timers = new Map();
  let nextId = 1;
  return {
    now: () => now,
    schedule: (fn, ms) => {
      const id = nextId++;
      timers.set(id, { fn, at: now + ms });
      return id;
    },
    cancel: (id) => {
      timers.delete(id);
    },
    advance(ms) {
      const target = now + ms;
      for (;;) {
        let pick = null;
        for (const [id, t] of timers) {
          if (t.at <= target && (pick === null || t.at < pick[1].at)) pick = [id, t];
        }
        if (!pick) break;
        timers.delete(pick[0]);
        now = pick[1].at;
        pick[1].fn();
      }
      now = target;
    },
    pending: () => timers.size,
  };
}

function harness(opts = {}) {
  const clock = fakeClock();
  const commits = [];
  const r = useTerminalResize((cols, rows) => commits.push([cols, rows]), {
    quietMs: opts.quietMs ?? TERM_RESIZE_QUIET_MS,
    maxWaitMs: opts.maxWaitMs ?? TERM_RESIZE_MAX_WAIT_MS,
    now: clock.now,
    schedule: clock.schedule,
    cancel: clock.cancel,
  });
  return { clock, commits, r };
}

// N3：尺寸未变 → 零下发（ResizeObserver 抖动不产生 IPC）。
{
  const { clock, commits, r } = harness();
  r.notify(120, 40);
  clock.advance(1_000);
  r.notify(120, 40);
  clock.advance(1_000);
  assert(commits.length === 1, `E2-N3 同尺寸去重后只下发 1 次（实得 ${commits.length}）`);
}

// N1：快速连续 resize（拖窗口）→ 静默窗口内只下发一次，且最后一次尺寸正确。
// 首个 resize 零等待下发（会话首帧要尽快把真实行列交给 PTY，新 PTY 默认 100×24）。
{
  const { clock, commits, r } = harness();
  r.notify(100, 30);
  clock.advance(1);
  assert(commits.length === 1, `E2-N1a 首个 resize 零等待下发（实得 ${commits.length}）`);
  for (let i = 1; i <= 30; i += 1) {
    r.notify(100 + i, 30);
    clock.advance(5);
  }
  clock.advance(TERM_RESIZE_QUIET_MS);
  assert(commits.length === 2, `E2-N1b 30 次快速 resize 只再下发 1 次（实得 ${commits.length - 1}）`);
  assert(
    commits[1][0] === 130 && commits[1][1] === 30,
    `E2-N1c 下发的是最后一次尺寸（实得 ${JSON.stringify(commits[1])}）`
  );
}

// N2：持续慢拖 3 秒（每 50ms 变一次）→ 500ms 硬上界生效，终端不会一直不重排。
{
  const { clock, commits, r } = harness();
  for (let t = 0; t < 3_000; t += 50) {
    r.notify(100 + (t % 200), 30);
    clock.advance(50);
  }
  clock.advance(TERM_RESIZE_QUIET_MS);
  assert(commits.length >= 6, `E2-N2 持续慢拖 3s 至少下发 6 次（实得 ${commits.length}）`);
  assert(
    commits.length <= 3_000 / TERM_RESIZE_MAX_WAIT_MS + 2,
    `E2-N2b 下发次数受硬上界约束（实得 ${commits.length}）`
  );
}

// N5：dispose 清掉在途定时器，卸载后不再有回调。
{
  const { clock, commits, r } = harness();
  r.notify(120, 40);
  r.dispose();
  clock.advance(5_000);
  assert(commits.length === 0, `E2-N5 dispose 后零回调（实得 ${commits.length}）`);
}

// N9：reset（新会话）后，同尺寸也能再次下发——首个 resize 不会被去重吞掉。
{
  const { clock, commits, r } = harness();
  r.notify(120, 40);
  clock.advance(1_000);
  assert(commits.length === 1, "E2-N9a 首次下发成功");
  r.reset();
  r.notify(120, 40);
  clock.advance(TERM_RESIZE_QUIET_MS);
  assert(commits.length === 2, `E2-N9b reset 后同尺寸再次下发（实得 ${commits.length}）`);
}

// N6：非有限值（容器过渡态产生的 NaN/undefined）直接忽略，零下发。
{
  const { clock, commits, r } = harness();
  r.notify(Number.NaN, 40);
  r.notify(120, Number.NaN);
  r.notify(Number.POSITIVE_INFINITY, 40);
  clock.advance(5_000);
  assert(commits.length === 0, `E2-N6 非有限行列零下发（实得 ${commits.length}）`);
}

// 硬上界语义：距上次下发超过 maxWaitMs 时立即下发（wait = 0）。
{
  const { clock, commits, r } = harness();
  r.notify(80, 24);
  clock.advance(1_000);
  assert(commits.length === 1, "E2-N2c 首次下发");
  clock.advance(TERM_RESIZE_MAX_WAIT_MS + 10);
  r.notify(90, 24);
  clock.advance(0);
  assert(commits.length === 2, `E2-N2d 超硬上界后零等待下发（实得 ${commits.length}）`);
}

// ===========================================================================
// 汇总
// ===========================================================================
if (failures.length) {
  console.error(`\ncheck-terminal-ui-logic: FAIL（${passed} passed / ${failures.length} failed）`);
  process.exit(1);
}
console.log(`check-terminal-ui-logic: OK（${passed} assertions passed）`);
