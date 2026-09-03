#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M2-1 图片展示逻辑前端自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/utils/image.ts`（不 mock、不重写逻辑），因此下面每条
// 断言反映的都是产品代码行为。本卡只做「领域 + 持久化」，画廊/灯箱/缩放（M2-2）
// 未实现，故这里验证的是预览层的**展示与降级逻辑**：
// 格式识别 / 非支持格式降级 / 体积与尺寸格式化 / 展示态判定 / 错误码文案 /
// 溯源只出主机不出 URL 原文。
//
// 用法: node scripts/check-image-ui-logic.mjs
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
  return globalThis.__m21Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m21Resolve = resolveWithExt;
}

const ROOT = new URL("..", import.meta.url).pathname;

const {
  IMAGE_MIME_EXT,
  IMAGE_ERROR_TEXT,
  extFromMime,
  isSupportedImage,
  formatImageBytes,
  formatImageSize,
  imageAltText,
  imageDisplayState,
  imageErrorText,
  imageSourceHost,
} = await import(`${ROOT}src/utils/image.ts`);

let passed = 0;
const failures = [];

function eq(actual, expected, label) {
  if (actual === expected) {
    passed += 1;
  } else {
    failures.push(`${label}\n    期望: ${JSON.stringify(expected)}\n    实际: ${JSON.stringify(actual)}`);
  }
}

function ok(cond, label) {
  if (cond) passed += 1;
  else failures.push(label);
}

// ---------- 1) 支持格式识别（与后端白名单一致） ----------
eq(extFromMime("image/png"), "png", "PNG → .png");
eq(extFromMime("image/jpeg"), "jpg", "JPEG → .jpg");
eq(extFromMime("image/webp"), "webp", "WEBP → .webp");
eq(extFromMime("image/gif"), "gif", "GIF → .gif");
eq(IMAGE_MIME_EXT.length, 4, "白名单恰为 4 项（与后端一致）");

// ---------- 2) 非支持格式降级（fail-closed） ----------
eq(extFromMime("image/svg+xml"), null, "SVG 必须降级（XSS 面）");
eq(extFromMime("image/bmp"), null, "BMP 首期不开");
eq(extFromMime("text/html"), null, "非图片 MIME 拒绝");
eq(extFromMime("image/png.html"), null, "拼接型 MIME 拒绝");
eq(extFromMime(null), null, "null MIME 安全降级");
eq(extFromMime(undefined), null, "undefined MIME 安全降级");
eq(extFromMime(""), null, "空 MIME 安全降级");
eq(extFromMime("  IMAGE/PNG  "), "png", "大小写与空格归一化");
eq(isSupportedImage("image/png"), true, "受支持格式判定为真");
eq(isSupportedImage("image/svg+xml"), false, "SVG 判定为假");

// ---------- 3) 体积格式化（空/非法值 → "-"，不显示 NaN） ----------
eq(formatImageBytes(0), "0 B", "0 字节");
eq(formatImageBytes(512), "512 B", "不足 1KB");
eq(formatImageBytes(1024), "1.0 KB", "整 1KB");
eq(formatImageBytes(1536), "1.5 KB", "1.5KB");
eq(formatImageBytes(1048576), "1.0 MB", "整 1MB");
eq(formatImageBytes(5 * 1024 * 1024), "5.0 MB", "5MB");
eq(formatImageBytes(null), "-", "null → 占位");
eq(formatImageBytes(undefined), "-", "undefined → 占位");
eq(formatImageBytes(-1), "-", "负数 → 占位");
eq(formatImageBytes(NaN), "-", "NaN → 占位");

// ---------- 4) 尺寸格式化（后端解不出尺寸时如实显示 "-"） ----------
eq(formatImageSize(800, 600), "800×600", "正常尺寸");
eq(formatImageSize(null, 600), "-", "宽未知 → 占位（不伪造 0）");
eq(formatImageSize(800, null), "-", "高未知 → 占位");
eq(formatImageSize(undefined, undefined), "-", "全未知 → 占位");

// ---------- 5) 替代文本与展示态 ----------
eq(imageAltText({ caption: "示意图", mime: "image/png" }), "示意图", "优先用说明文案");
eq(imageAltText({ caption: "  ", mime: "image/jpeg" }), "image/jpeg", "空说明回退 MIME");
eq(imageAltText({ caption: null, mime: "" }), "图片", "全空兜底");
eq(imageAltText(null), "图片", "null 引用兜底");

eq(
  imageDisplayState({
    id: "i1",
    source: "file",
    rel_path: "images/a/i1.png",
    mime: "image/png",
    bytes: 10,
    width: 2,
    height: 2,
    sha256: "s",
    source_url: null,
    caption: null,
    created_at: "",
  }),
  "ok",
  "落盘图且尺寸已知 → 正常展示"
);
eq(
  imageDisplayState({
    id: "i2",
    source: "file",
    rel_path: "images/a/i2.svg",
    mime: "image/svg+xml",
    bytes: 10,
    width: 2,
    height: 2,
    sha256: "s",
    source_url: null,
    caption: null,
    created_at: "",
  }),
  "unsupported",
  "非白名单格式 → 降级提示"
);
eq(
  imageDisplayState({
    id: "i3",
    source: "file",
    rel_path: null,
    mime: "image/png",
    bytes: 10,
    width: 2,
    height: 2,
    sha256: "s",
    source_url: null,
    caption: null,
    created_at: "",
  }),
  "missing_file",
  "落盘型却无路径（文件被手工删除）→ 错误占位而非崩溃"
);
eq(
  imageDisplayState({
    id: "i4",
    source: "file",
    rel_path: "images/a/i4.png",
    mime: "image/png",
    bytes: 10,
    width: null,
    height: null,
    sha256: "s",
    source_url: null,
    caption: null,
    created_at: "",
  }),
  "unknown_size",
  "尺寸未知 → 标记为未知而非填 0"
);
eq(imageDisplayState(null), "missing_file", "空引用 → 错误占位");

// ---------- 6) 错误码文案（后端契约码全覆盖） ----------
const BACKEND_CODES = [
  "MIME_NOT_ALLOWED",
  "MIME_MAGIC_MISMATCH",
  "PATH_ESCAPE",
  "IMAGE_TOO_LARGE",
  "IMAGE_EMPTY",
  "IMAGE_DIMENSION_EXCEEDED",
  "IMAGE_COUNT_EXCEEDED",
  "IMAGE_BUDGET_EXCEEDED",
  "INLINE_TOO_LARGE",
  "INVALID_ID",
  "IO_FAILED",
];
for (const code of BACKEND_CODES) {
  ok(
    typeof IMAGE_ERROR_TEXT[code] === "string" && IMAGE_ERROR_TEXT[code].length > 0,
    `错误码 ${code} 必须有中文文案`
  );
}
eq(imageErrorText(null), "", "无错误不显示文案");
eq(imageErrorText("IMAGE_TOO_LARGE"), IMAGE_ERROR_TEXT.IMAGE_TOO_LARGE, "已知码走既定文案");
ok(
  imageErrorText("SOMETHING_NEW").includes("SOMETHING_NEW"),
  "未知错误码如实回显，不得伪装成成功"
);

// ---------- 7) 隐私：溯源只出主机，不渲染 URL 原文 ----------
eq(imageSourceHost("https://ex.com/a.png?token=sekrit&x=1"), "ex.com", "只显示主机");
eq(imageSourceHost("https://user:pw@ex.com/p.png"), "ex.com", "userinfo 不出现在展示文本");
eq(imageSourceHost("not-a-url"), "-", "非法 URL → 占位");
eq(imageSourceHost(null), "-", "null → 占位");
eq(imageSourceHost(""), "-", "空串 → 占位");
ok(
  !imageSourceHost("https://ex.com/a.png?token=sekrit").includes("sekrit"),
  "展示文本不得包含凭据原值"
);

// ---------- 结果 ----------
if (failures.length === 0) {
  console.log(`check-image-ui-logic: ${passed} 断言全部通过`);
  process.exit(0);
} else {
  console.error(`check-image-ui-logic: ${failures.length} 项失败（通过 ${passed}）`);
  for (const f of failures) console.error(`  ✗ ${f}`);
  process.exit(1);
}
