#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M3.c（WBS M3-4）终端体验项前端逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/stores/useSystemStore.ts` 与
// `src/composables/useTerminalResize.ts`，只把 `src/bridge.ts` 的终端方法替换为
// 记录型 mock（不 mock store / composable 自身逻辑）：下面每条断言反映的都是
// **产品代码**的行为，而非测试替身的行为。
//
// 覆盖：
//   E1 临时历史 40 条 —— 上限生效、只留最新、面板重建回放、会话结束清空、
//                        flow/exit 帧不进历史、**不落盘/不审计**。
//   E2 resize 静默窗口 —— 去重（尺寸未变零下发）、静默窗口（连续 resize 只发一次）、
//                        500ms 硬上界（持续慢拖仍会下发）、dispose 清定时器、
//                        reset 后同尺寸可再次下发、非有限值忽略。
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
};
globalThis.localStorage = {
  getItem: () => null,
  setItem: (k, v) => storageWrites.push([String(k), String(v)]),
  removeItem: () => {},
};

const ROOT = new URL("..", import.meta.url).pathname;

const { bridge } = await import(`${ROOT}src/bridge.ts`);
const { useSystemStore } = await import(`${ROOT}src/stores/useSystemStore.ts`);
const { useTerminalResize, TERM_RESIZE_QUIET_MS, TERM_RESIZE_MAX_WAIT_MS } = await import(
  `${ROOT}src/composables/useTerminalResize.ts`
);
const { createPinia, setActivePinia } = await import(`${ROOT}node_modules/pinia/dist/pinia.mjs`);

// ---------- mock bridge（只替换终端相关方法，记录调用） ----------
const calls = [];
bridge.createTermChannel = () => ({ __channel: true });
bridge.termSpawnChannel = async () => ({ id: "term-m3c" });
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
const system = useSystemStore();

// ===========================================================================
// E1 临时历史 40 条
// ===========================================================================

// T1：还没有 writer 时回放不得抛错（面板未挂载的边界）。
system.replayTermHistory();
assert(true, "E1-T1 replayTermHistory 在无 writer 时安全 no-op");

const captured = [];
system.bindTermWriter((data) => captured.push(data));

await system.startShell();
assert(system.termId === "term-m3c", "E1-T2 startShell 后 termId 就绪");
captured.length = 0;

// T3：灌入 100 条输出 → 历史上限 40，且保留的是**最新**的 40 条。
for (let i = 1; i <= 100; i += 1) {
  system.onTermChannelMsg({ id: "term-m3c", kind: "data", data: `OUT-${String(i).padStart(4, "0")}` });
}
assert(system.termHistory.length === 40, `E1-T3a 历史上限生效（期望 40，实得 ${system.termHistory.length}）`);
assert(
  system.termHistory[0] === "OUT-0061" && system.termHistory[39] === "OUT-0100",
  "E1-T3b 超限后保留最新 40 条（丢弃最旧的）"
);

// T4：flow / exit 帧不进历史（只有 PTY 输出块才算历史）。
const before = system.termHistory.length;
system.onTermChannelMsg({ id: "term-m3c", kind: "flow", dropped_chunks: 3, dropped_bytes: 30 });
system.onTermChannelMsg({ id: "term-m3c", kind: "exit", reason: "eof", data: "\r\n[终端已退出]\r\n" });
assert(system.termHistory.length === before, "E1-T4 flow/exit 帧不写入临时历史");

// T5：面板重建（旧 writer 解绑 → 新 writer 绑定）后回放，内容等于历史且**不自喂**。
system.bindTermWriter(null);
const replayed = [];
system.bindTermWriter((data) => replayed.push(data));
system.replayTermHistory();
assert(replayed.length === 40, `E1-T5a 重建后回放 40 条（实得 ${replayed.length}）`);
assert(
  replayed.join("") === system.termHistory.join(""),
  "E1-T5b 回放内容与历史一致"
);
assert(system.termHistory.length === 40, "E1-T5c 回放不回写历史（长度不翻倍）");

// T6：killShell 后历史清空（会话结束即消亡，不残留到下一段会话）。
await system.killShell();
assert(system.termHistory.length === 0, "E1-T6a killShell 清空临时历史");
replayed.length = 0;
system.replayTermHistory();
assert(replayed.length === 0, "E1-T6b 会话结束后回放为空");

// T7：重启（startShell(true)）同样清空，旧会话输出不得作为残影回放到新终端。
await system.startShell();
for (let i = 0; i < 5; i += 1) {
  system.onTermChannelMsg({ id: "term-m3c", kind: "data", data: `OLD-${i}` });
}
assert(system.termHistory.length === 5, "E1-T7a 旧会话历史已累积");
await system.startShell(true);
assert(system.termHistory.length === 0, "E1-T7b 重启新会话清空历史（无跨会话残影）");

// T8：不落盘 / 不审计 —— 全流程跑完后，所有 localStorage 写入都不得含终端输出内容，
//     也不得出现终端历史键（历史只存在于内存）。
for (let i = 0; i < 60; i += 1) {
  system.onTermChannelMsg({ id: "term-m3c", kind: "data", data: `SECRET-${i}-sk-abc` });
}
await system.killShell();
const leaked = storageWrites.filter(
  ([k, v]) => k.toLowerCase().includes("term") || v.includes("OUT-") || v.includes("SECRET-") || v.includes("OLD-")
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
