#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M6 Reduced Scope · S5 —— browserLayout / browserSync 的 deterministic 测试。
//
// 零新增依赖：沿用 scripts/check-clipboard-persistence-logic.mjs 的
// node:module registerHooks + 扩展名回退机制，直接加载真实 TS 源码
//（Node >= 22.18 默认 type-stripping，本仓库 v26.7 可用）。
//
// 覆盖：
//   G1  gridCellRect 多模式（horizontal / quad n=4 / grid n=5 / n=1 / W<gap）
//   G2  zoom ∈ [0.3, 1] 且保留 2 位
//   G3  rect normalization（Math.round 语义）
//   G4  gridCellHostRect：Math.round + contentH 最小 40 兜底
//   G5  rect=0 retry 有明确上限
//   G6  duplicate 不产生额外同步（50ms 去重）
//   G7  hidden intent（同会话同页签只移出一次）
//   G8  gridSession 变化后缓存失效
//   G9  invalid rect 不下发（hasNonZeroSize）
//   G10 reject 后缓存恢复（能重发）
//
// 用法: node scripts/check-browser-sync-logic.mjs
// 退出码: 0 = 全部通过；1 = 有失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

function resolveWithExt(specifier, context, next) {
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {
          /* try next */
        }
      }
    }
    throw err;
  }
}

if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
}

const layout = await import(join(ROOT, "src/utils/browserLayout.ts"));
const sync = await import(join(ROOT, "src/utils/browserSync.ts"));

let pass = 0;
const failures = [];
function ok(name, cond, detail) {
  if (cond) pass += 1;
  else failures.push(detail ? `${name} — ${detail}` : name);
}
const near = (a, b, eps = 1e-6) => Math.abs(a - b) < eps;

// ── G1 gridCellRect 多模式 ────────────────────────────────────────────────
{
  const W = 1000, H = 500, gap = layout.GRID_GAP;
  // horizontal n=3：均分宽度扣 2*gap，满高
  const h0 = layout.gridCellRect("horizontal", 0, 3, W, H, gap);
  ok("G1 horizontal n=3 cell0", near(h0.w, (W - gap * 2) / 3) && h0.h === H && h0.y === 0);
  const h2 = layout.gridCellRect("horizontal", 2, 3, W, H, gap);
  ok("G1 horizontal n=3 last cell 不越界", h2.x + h2.w <= W + 1e-6);

  // quad n=4：固定 2x2
  const q0 = layout.gridCellRect("quad", 0, 4, W, H, gap);
  const q3 = layout.gridCellRect("quad", 3, 4, W, H, gap);
  ok("G1 quad n=4 半宽半高", near(q0.w, (W - gap) / 2) && near(q0.h, (H - gap) / 2));
  ok("G1 quad n=4 第4格在右下", q3.x > q0.x && q3.y > q0.y);

  // quad 但 n!==4 → 走通用算法（不产生"只占上排"空白）
  const q2 = layout.gridCellRect("quad", 0, 2, W, H, gap);
  ok("G1 quad n=2 回退通用", q2.w > 0 && q2.h > 0);

  // grid n=5 → cols=3
  const g4 = layout.gridCellRect("grid", 4, 5, W, H, gap);
  ok("G1 grid n=5 cols=3 第5格换行", g4.y > 0, `y=${g4.y}`);

  // n=1
  const one = layout.gridCellRect("horizontal", 0, 1, W, H, gap);
  ok("G1 n=1 满宽", near(one.w, W) && near(one.h, H));

  // W < gap 不崩且不出 NaN
  const tiny = layout.gridCellRect("grid", 0, 4, 2, 100, gap);
  ok("G1 W<gap 不产生 NaN", Number.isFinite(tiny.x) && Number.isFinite(tiny.w));
}

// ── G2 zoom ───────────────────────────────────────────────────────────────
{
  ok("G2 zoom 0.5", layout.normalizeZoom(500, 1000) === 0.5);
  ok("G2 zoom 上钳 1", layout.normalizeZoom(2000, 1000) === 1);
  ok("G2 zoom 下钳 0.3", layout.normalizeZoom(100, 1000) === 0.3);
  ok("G2 zoom 保留 2 位", layout.normalizeZoom(333, 1000) === 0.33);
  ok("G2 refWidth=0 回落 1", layout.normalizeZoom(500, 0) === 1);
  ok("G2 常量范围", layout.ZOOM_MIN === 0.3 && layout.ZOOM_MAX === 1);
}

// ── G3 / G4 rect normalization ────────────────────────────────────────────
{
  const r = layout.normalizeHostRect({ left: 10.6, top: 20.2, width: 300.5, height: 400.4 });
  ok("G3 Math.round 四字段", r.x === 11 && r.y === 20 && r.width === 301 && r.height === 400);
  ok("G3 字段名为 width/height", "width" in r && "height" in r);

  const cell = { x: 1.4, y: 2.6, w: 100.5, h: 10 };
  const gr = layout.gridCellHostRect({ left: 5.5, top: 6.5, width: 800, height: 600 }, cell);
  ok("G4 host+cell 取整", Number.isInteger(gr.x) && Number.isInteger(gr.y));
  ok("G4 contentH 最小 40 兜底", gr.height === layout.GRID_MIN_CONTENT_HEIGHT, `h=${gr.height}`);
}

// ── G5 retry 上限 ─────────────────────────────────────────────────────────
{
  let scheduled = 0;
  const r = sync.createBoundedRetrier({
    maxRetries: 3,
    delayMs: 0,
    schedule: (fn) => { scheduled += 1; },
  });
  ok("G5 retry<上限 可重试", r.shouldRetry(0) && r.shouldRetry(2));
  ok("G5 retry=上限 停止", !r.shouldRetry(3));
  ok("G5 超上限不安排", r.scheduleRetry(3, () => {}) === false && scheduled === 0);
  ok("G5 耗尽标记", r.exhausted === true);
  ok("G5 默认上限为 10", sync.DEFAULT_MAX_RETRIES === 10);
}

// ── G6 50ms 去重 ──────────────────────────────────────────────────────────
{
  let t = 1000;
  const d = sync.createKeyDeduper({ windowMs: 50, now: () => t });
  const key = sync.tabPositionKey("t1", 0, 0, 100, 100);
  ok("G6 首次下发", d.shouldSend(key) === true);
  t = 1020;
  ok("G6 窗口内重复被丢弃", d.shouldSend(key) === false);
  t = 1080;
  ok("G6 超窗口后放行", d.shouldSend(key) === true);
  d.reset();
  ok("G6 reset 后可再发", d.shouldSend(key) === true);
  ok(
    "G6 key 格式逐字保持",
    sync.tabPositionKey("t1", 1, 2, 3, 4) === "t1:1,2,3,4",
  );
  ok("G6 grid 签名格式", sync.gridPositionSignature(1, 2, 3, 4) === "1,2,3x4");
}

// ── G7 hidden intent ──────────────────────────────────────────────────────
{
  const h = sync.createHiddenIntent();
  ok("G7 首次移出", h.shouldHide("t1") === true);
  ok("G7 同页签不重复移出", h.shouldHide("t1") === false);
  ok("G7 换页签可再移出", h.shouldHide("t2") === true);
  ok("G7 空 id 不触发", h.shouldHide("") === false);
  h.reset();
  ok("G7 reset 后可再移出", h.shouldHide("t1") === true);
  ok("G7 隐藏常量 -30000", sync.HIDDEN_OFFSCREEN_X === -30000);
}

// ── G8 gridSession 失效 ───────────────────────────────────────────────────
{
  const c = sync.createGridSendCache();
  c.syncSession(1);
  const sig = sync.gridPositionSignature(0, 0, 10, 10);
  ok("G8 首发送出", c.shouldSend(0, sig) === true);
  ok("G8 同签名不重发", c.shouldSend(0, sig) === false);
  const changed = c.syncSession(2);
  ok("G8 session 变化触发失效", changed === true);
  ok("G8 失效后可重发", c.shouldSend(0, sig) === true);
  ok("G8 同 session 不重复失效", c.syncSession(2) === false);
}

// ── G9 invalid rect 不下发 ────────────────────────────────────────────────
{
  ok("G9 零宽不可用", sync.hasNonZeroSize({ width: 0, height: 100 }) === false);
  ok("G9 零高不可用", sync.hasNonZeroSize({ width: 100, height: 0 }) === false);
  ok("G9 null 不可用", sync.hasNonZeroSize(null) === false);
  ok("G9 正常可用", sync.hasNonZeroSize({ width: 1, height: 1 }) === true);
}

// ── G10 reject 后缓存恢复 ─────────────────────────────────────────────────
{
  const c = sync.createGridSendCache();
  c.syncSession(1);
  const sig = sync.gridPositionSignature(5, 5, 20, 20);
  c.shouldSend(1, sig);
  ok("G10 发送后已被缓存", c.shouldSend(1, sig) === false);
  c.invalidate(1);
  ok("G10 reject 后可重发（防 1x1）", c.shouldSend(1, sig) === true);
}

// ── 失效组：一处失效全部作废（原 L21 漏一个即 bug）────────────────────────
{
  const d = sync.createKeyDeduper({ windowMs: 50, now: () => 1000 });
  const h = sync.createHiddenIntent();
  const g = sync.createInvalidationGroup();
  g.register(() => d.reset());
  g.register(() => h.reset());
  d.shouldSend("k");
  h.shouldHide("t1");
  g.invalidateAll();
  ok("失效组：deduper 作废", d.shouldSend("k") === true);
  ok("失效组：hidden intent 作废", h.shouldHide("t1") === true);
}

if (failures.length > 0) {
  console.log("BROWSER_SYNC_LOGIC=FAIL");
  for (const f of failures) console.log("  FAIL - " + f);
  console.log(`\n${pass} passed, ${failures.length} failed`);
  process.exit(1);
}
console.log(`BROWSER_SYNC_LOGIC=PASS (${pass}/${pass} assertions)`);
