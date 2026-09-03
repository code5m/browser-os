#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M2-2.b 图片预览逻辑层自动化测试（headless，无 GUI / 无 Tauri 依赖）
//
// 直接加载**真实的** `src/utils/imagePreview.ts`（不 mock、不重写逻辑），
// 因此每条断言反映的都是产品代码行为。覆盖 `logs/checkpoints/M2-2-20260903-1340.md`
// §5 的反向用例矩阵 T-prev-1~T-prev-12，并补充通道与隐私相关的回归断言。
//
// 用法: node scripts/check-image-preview-logic.mjs
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
  return globalThis.__m22Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m22Resolve = resolveWithExt;
}

const ROOT = new URL("..", import.meta.url).pathname;

const {
  SCALE_MAX,
  SCALE_MIN,
  SCALE_STEP,
  buildAssetSrc,
  buildPreviewItem,
  buildPreviewItems,
  bumpRetry,
  clearFailed,
  clampScale,
  clampTranslate,
  createKeyBinder,
  dragEnabled,
  fitScale,
  galleryState,
  gridColumns,
  isFailed,
  isSafeRelPath,
  keyAction,
  markFailed,
  nextIndex,
  prevIndex,
  retryNonce,
  scaleForButton,
  scaleForDoubleClick,
  scaleForWheel,
  shouldRenderImage,
  truncateText,
} = await import(`${ROOT}src/utils/imagePreview.ts`);

// ---------------------------------------------------------------------------
// 迷你断言框架
// ---------------------------------------------------------------------------
let passed = 0;
const failures = [];

function ok(label, cond) {
  if (cond) {
    passed += 1;
  } else {
    failures.push(label);
    console.log(`  x ${label}`);
  }
}

function eq(label, actual, expected) {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a === e) {
    passed += 1;
  } else {
    failures.push(`${label}（期望 ${e}，实际 ${a}）`);
    console.log(`  x ${label}：期望 ${e}，实际 ${a}`);
  }
}

// ---------------------------------------------------------------------------
// 测试数据
// ---------------------------------------------------------------------------
const DIR = "/home/u/.local/share/com.jizhijiandan.mvp/mvp-browser-os/workspace/images";

function fileRef(id, over = {}) {
  return {
    id,
    source: "file",
    rel_path: `images/a1/${id}.png`,
    mime: "image/png",
    bytes: 2048,
    width: 800,
    height: 600,
    sha256: "s".repeat(64),
    source_url: "https://example.com/p?token=secret-token-value",
    caption: null,
    created_at: "2026-09-03T00:00:00Z",
    ...over,
  };
}

const THREE = [fileRef("i1"), fileRef("i2"), fileRef("i3")];

// ---------------------------------------------------------------------------
// T-prev-1 空列表：空态，不报错、不产生 0 列网格
// ---------------------------------------------------------------------------
{
  eq("T-prev-1 空列表不产生任何预览项", buildPreviewItems([], DIR), []);
  eq("T-prev-1 空列表状态为空态", galleryState(false, 0), "empty");
  eq("T-prev-1 null 入参同样安全", buildPreviewItems(null, DIR), []);
  ok("T-prev-1 空容器列数仍 >=1（不出现 0 列网格）", gridColumns(0) >= 1);
  ok("T-prev-1 空态不抛异常", buildPreviewItems(undefined, undefined).length === 0);
}

// ---------------------------------------------------------------------------
// T-prev-2 3 张中第 2 张 404：只标记第 2 张，其余不受影响且不关闭
// ---------------------------------------------------------------------------
{
  const items = buildPreviewItems(THREE, DIR);
  eq("T-prev-2 三张全部可渲染", items.length, 3);

  let failed = {};
  failed = markFailed(failed, "i2");
  ok("T-prev-2 只有第 2 张被标记失败", isFailed(failed, "i2"));
  ok("T-prev-2 第 1 张不受影响", !isFailed(failed, "i1"));
  ok("T-prev-2 第 3 张不受影响", !isFailed(failed, "i3"));
  eq("T-prev-2 失败不影响渲染项总数", items.length, 3);

  let retries = bumpRetry({}, "i2");
  eq("T-prev-2 重试 nonce 递增", retryNonce(retries, "i2"), 1);
  retries = bumpRetry(retries, "i2");
  eq("T-prev-2 再次重试 nonce 继续递增", retryNonce(retries, "i2"), 2);
  eq("T-prev-2 其它图片 nonce 仍为 0", retryNonce(retries, "i1"), 0);

  failed = clearFailed(failed, "i2");
  ok("T-prev-2 重试后失败标记被清除", !isFailed(failed, "i2"));
  ok("T-prev-2 空 id 不产生副作用", JSON.stringify(markFailed({}, "")) === "{}");
}

// ---------------------------------------------------------------------------
// T-prev-3 第 1 张按 ←：索引不变（不循环）
// ---------------------------------------------------------------------------
{
  eq("T-prev-3 首张按左键索引不变", prevIndex(0, 3), 0);
  eq("T-prev-3 末张按右键索引不变", nextIndex(2, 3), 2);
  eq("T-prev-3 中间可正常前进", nextIndex(1, 3), 2);
  eq("T-prev-3 中间可正常后退", prevIndex(1, 3), 0);
  eq("T-prev-3 空列表索引为 -1", prevIndex(0, 0), -1);
  eq("T-prev-3 空列表 next 为 -1", nextIndex(0, 0), -1);
  eq("T-prev-3 越界索引回落到首张", prevIndex(9, 3), 0);
}

// ---------------------------------------------------------------------------
// T-prev-4 / T-prev-5 缩放到边界即停
// ---------------------------------------------------------------------------
{
  eq("T-prev-4 已在 4x 继续放大停在 4", scaleForWheel(SCALE_MAX, -100), SCALE_MAX);
  eq("T-prev-4 已在 4x 按按钮放大停在 4", scaleForButton(SCALE_MAX, 1), SCALE_MAX);
  eq("T-prev-5 已在 0.25x 继续缩小停在 0.25", scaleForWheel(SCALE_MIN, 100), SCALE_MIN);
  eq("T-prev-5 已在 0.25x 按按钮缩小停在 0.25", scaleForButton(SCALE_MIN, -1), SCALE_MIN);
  eq("T-prev-4 一档放大等于步长", scaleForWheel(1, -100), SCALE_STEP);
  ok("T-prev-5 一档缩小等于 1/步长", Math.abs(scaleForWheel(1, 100) - 1 / SCALE_STEP) < 1e-9);
  eq("T-prev-4/5 越界值被钳制", clampScale(99), SCALE_MAX);
  eq("T-prev-4/5 负值被钳制", clampScale(-99), SCALE_MIN);
  eq("T-prev-4/5 NaN 回落到 1", clampScale(NaN), 1);
  eq("T-prev-4/5 非数字回落到 1", clampScale("2"), 1);
  eq("T-prev-4/5 deltaY=0 不缩放", scaleForWheel(1.5, 0), 1.5);
}

// ---------------------------------------------------------------------------
// T-prev-6 / T-prev-7 键盘监听：ESC 关闭且解绑；快速开关无残留
// ---------------------------------------------------------------------------
{
  eq("T-prev-6 ESC 映射为关闭", keyAction("Escape"), "close");
  eq("T-prev-6 左箭头映射为上一张", keyAction("ArrowLeft"), "prev");
  eq("T-prev-6 右箭头映射为下一张", keyAction("ArrowRight"), "next");
  eq("T-prev-6 数字 0 映射为缩放复位", keyAction("0"), "zoom_reset");
  eq("T-prev-6 加号映射为放大", keyAction("+"), "zoom_in");
  eq("T-prev-6 减号映射为缩小", keyAction("-"), "zoom_out");
  eq("T-prev-6 无关按键不响应", keyAction("a"), null);

  let live = 0;
  const dispatched = [];
  const binder = createKeyBinder(
    {
      add: () => {
        live += 1;
      },
      remove: () => {
        live -= 1;
      },
    },
    (key) => dispatched.push(key)
  );

  binder.bind();
  eq("T-prev-6 打开后监听已注册", live, 1);
  ok("T-prev-6 打开后处于已绑定态", binder.isBound());
  binder.handler("Escape");
  eq("T-prev-6 按键被派发", dispatched, ["Escape"]);
  binder.unbind();
  eq("T-prev-6 关闭后监听已解绑", live, 0);
  ok("T-prev-6 关闭后不再处于绑定态", !binder.isBound());

  // T-prev-7 快速开关 20 次：无残留监听、无重复绑定
  const b2 = createKeyBinder({ add: () => {}, remove: () => {} });
  for (let i = 0; i < 20; i += 1) {
    b2.bind();
    b2.unbind();
  }
  eq("T-prev-7 开关 20 次绑定计数为 20", b2.bindCount(), 20);
  eq("T-prev-7 开关 20 次解绑计数为 20", b2.unbindCount(), 20);
  ok("T-prev-7 开关 20 次后无残留绑定", !b2.isBound());

  // 幂等：重复 bind 不叠加监听
  const b3 = createKeyBinder({ add: () => {}, remove: () => {} });
  b3.bind();
  b3.bind();
  b3.bind();
  eq("T-prev-7 重复 bind 只注册一次", b3.bindCount(), 1);
  b3.unbind();
  b3.unbind();
  eq("T-prev-7 重复 unbind 只解绑一次", b3.unbindCount(), 1);
}

// ---------------------------------------------------------------------------
// T-prev-8 尺寸未知：按 unknown_size 降级，不塌陷
// ---------------------------------------------------------------------------
{
  const unknown = fileRef("i-nosize", { width: null, height: null });
  const item = buildPreviewItem(unknown, DIR);
  eq("T-prev-8 展示态为 unknown_size", item.state, "unknown_size");
  eq("T-prev-8 尺寸文案降级为 -", item.sizeText, "-");
  ok("T-prev-8 仍可渲染（不因缺尺寸被剔除）", item.renderable);
  eq("T-prev-8 缺尺寸时初始缩放为 1", fitScale(null, null, 800, 600), 1);
  eq("T-prev-8 容器尺寸未知时缩放为 1", fitScale(800, 600, null, null), 1);
}

// ---------------------------------------------------------------------------
// T-prev-9 超长 caption：截断加省略号，不撑破网格
// ---------------------------------------------------------------------------
{
  const longCaption = "这是一段非常非常长的图片说明".repeat(20);
  const item = buildPreviewItem(fileRef("i-long", { caption: longCaption }), DIR);
  ok("T-prev-9 caption 被截断", item.caption.length <= 48);
  ok("T-prev-9 截断后以省略号结尾", item.caption.endsWith("…"));
  eq("T-prev-9 空 caption 回落到空串", truncateText(null), "");
  eq("T-prev-9 短文案不截断", truncateText("短"), "短");
  eq("T-prev-9 空 caption 时用 alt 兜底", buildPreviewItem(fileRef("i-alt"), DIR).alt, "image/png");
}

// ---------------------------------------------------------------------------
// T-prev-10 1x1 像素图：按 fit 显示，不放大成全屏色块
// ---------------------------------------------------------------------------
{
  eq("T-prev-10 1x1 图在 800x600 容器内保持 1x", fitScale(1, 1, 800, 600), 1);
  // 4000x3000 在 800x600 容器里理论缩放 0.2，低于缩放下限，故被兜到 0.25
  eq("T-prev-10 超大图被缩放下限兜住", fitScale(4000, 3000, 800, 600), SCALE_MIN);
  ok("T-prev-10 大图缩放结果不超过 1", fitScale(4000, 3000, 800, 600) <= 1);
  ok("T-prev-10 中等大图按容器等比缩小", Math.abs(fitScale(1600, 1200, 800, 600) - 0.5) < 1e-9);
  ok("T-prev-10 小图不被放大", fitScale(10, 10, 800, 600) === 1);
  ok("T-prev-10 零尺寸输入回落为 1", fitScale(0, 0, 800, 600) === 1);
}

// ---------------------------------------------------------------------------
// T-prev-11 loading true->false 且 0 张：骨架消失切空态
// ---------------------------------------------------------------------------
{
  eq("T-prev-11 loading 优先显示骨架", galleryState(true, 0), "loading");
  eq("T-prev-11 有图且 loading 仍显示骨架", galleryState(true, 3), "loading");
  eq("T-prev-11 loading 结束且 0 张切空态", galleryState(false, 0), "empty");
  eq("T-prev-11 loading 结束且有图切就绪", galleryState(false, 3), "ready");
}

// ---------------------------------------------------------------------------
// T-prev-12 容器 320px：列数降为 2，不横向溢出
// ---------------------------------------------------------------------------
{
  eq("T-prev-12 320px 宽为 2 列", gridColumns(320), 2);
  eq("T-prev-12 超窄容器至少 1 列", gridColumns(80), 1);
  ok("T-prev-12 宽容器列数有上限", gridColumns(4000) <= 6);
  ok("T-prev-12 列数随宽度单调不减", gridColumns(320) <= gridColumns(800));
  eq("T-prev-12 非法宽度回落为 1 列", gridColumns(null), 1);
}

// ---------------------------------------------------------------------------
// 补充：字节通道与隐私红线（冻结契约 §4.1 / §4.4）
// ---------------------------------------------------------------------------
{
  eq("通道 基准+相对路径拼成绝对路径", buildAssetSrc(DIR, "images/a1/x.png"), `${DIR}/images/a1/x.png`);
  eq("通道 基准尾部斜杠不产生双斜杠", buildAssetSrc(`${DIR}/`, "images/a1/x.png"), `${DIR}/images/a1/x.png`);
  eq("通道 越界相对路径被拒（空串）", buildAssetSrc(DIR, "../secret.png"), "");
  eq("通道 绝对路径被拒", buildAssetSrc(DIR, "/etc/passwd"), "");
  eq("通道 基准缺失被拒", buildAssetSrc("", "images/a1/x.png"), "");
  eq("通道 rel_path 为 null 被拒", buildAssetSrc(DIR, null), "");
  ok("通道 白名单判定：合法路径通过", isSafeRelPath("images/a1/x.png"));
  ok("通道 白名单判定：空段被拒", !isSafeRelPath("images//x.png"));
  ok("通道 白名单判定：反斜杠被拒", !isSafeRelPath("images\\x.png"));

  ok("隐私 只出主机不出原文", buildPreviewItem(fileRef("i1"), DIR).host === "example.com");
  const serialized = JSON.stringify(buildPreviewItem(fileRef("i1"), DIR));
  ok("隐私 预览项不含 source_url 原文", !serialized.includes("token=secret-token-value"));
  ok("隐私 预览项不含 source_url 字段", !serialized.includes("source_url"));

  ok("范围 inline_data_url 不进预览", !shouldRenderImage({
    ...fileRef("i-inline"),
    source: "inline_data_url",
    rel_path: null,
  }));
  eq("范围 不可渲染项被过滤", buildPreviewItems([
    fileRef("ok1"),
    { ...fileRef("inline1"), source: "inline_data_url", rel_path: null },
    fileRef("ok2"),
  ], DIR).length, 2);
  ok("范围 SVG 不进预览", !shouldRenderImage({ ...fileRef("i-svg"), mime: "image/svg+xml" }));

  ok("交互 仅放大后可拖拽", !dragEnabled(1) && dragEnabled(1.5));
  eq("交互 双击 1x 变 2x", scaleForDoubleClick(1), 2);
  eq("交互 双击非 1x 复位", scaleForDoubleClick(2), 1);
  eq("交互 双击 0.5x 也复位", scaleForDoubleClick(0.5), 1);
  eq("交互 平移受边界限制", clampTranslate(99999, 1), 240);
  eq("交互 平移负向受边界限制", clampTranslate(-99999, 1), -240);
  eq("交互 非法平移回落 0", clampTranslate(NaN, 1), 0);
}

// ---------------------------------------------------------------------------
console.log("");
if (failures.length === 0) {
  console.log(`check-image-preview-logic: ok（${passed} 条断言全部通过）`);
  process.exit(0);
}
console.log(`check-image-preview-logic: FAIL（${failures.length}/${passed + failures.length} 条失败）`);
process.exit(1);
