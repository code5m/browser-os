// M2-1 图片展示逻辑（纯函数，无 DOM/GUI 依赖）。
//
// 定位：本卡只做「领域 + 持久化」，画廊/灯箱/缩放（M2-2）尚未实现。
// 这里抽出的是预览层**无争议的展示逻辑**（格式判定、体积格式化、错误态文案），
// 供 M2-2 直接复用，也为「前端预览显示逻辑」提供可 headless 验证的对象。
//
// 隐私：展示文案只取 mime / bytes / caption 等元数据，**绝不渲染 source_url 原文**；
// URL 的脱敏统一由后端 `images::redact_source_url` 负责（落库前完成）。

import type { ImageRef } from "../types";

/** 与后端 `domain.rs::IMAGE_MIME_EXT` 一一对应（顺序一致） */
export const IMAGE_MIME_EXT: ReadonlyArray<readonly [string, string]> = [
  ["image/png", "png"],
  ["image/jpeg", "jpg"],
  ["image/webp", "webp"],
  ["image/gif", "gif"],
];

/** 后端契约错误码 → 中文降级文案（前端不得自行发明错误码） */
export const IMAGE_ERROR_TEXT: Record<string, string> = {
  MIME_NOT_ALLOWED: "不支持的图片格式（SVG 等已拒绝）",
  MIME_MAGIC_MISMATCH: "文件内容与声明的格式不符",
  PATH_ESCAPE: "图片路径非法，已拒绝",
  IMAGE_TOO_LARGE: "图片超过单张 10 MB 上限",
  IMAGE_EMPTY: "图片为空",
  IMAGE_DIMENSION_EXCEEDED: "图片尺寸超过 8000px 上限",
  IMAGE_COUNT_EXCEEDED: "单个成果最多 50 张图片",
  IMAGE_BUDGET_EXCEEDED: "单个成果图片总大小超过 50 MB",
  INLINE_TOO_LARGE: "图片超过内联上限，需落盘保存",
  INVALID_ID: "标识非法",
  IO_FAILED: "图片保存失败（磁盘错误）",
};

/** MIME → 扩展名（非白名单返回 null） */
export function extFromMime(mime: string | null | undefined): string | null {
  if (!mime) return null;
  const key = mime.trim().toLowerCase();
  const hit = IMAGE_MIME_EXT.find(([m]) => m === key);
  return hit ? hit[1] : null;
}

/** 是否为受支持的图片格式（fail-closed：SVG 与未知类型一律 false） */
export function isSupportedImage(mime: string | null | undefined): boolean {
  return extFromMime(mime) !== null;
}

/** 体积展示：1024 进制，保留一位小数；空值返回 "-" */
export function formatImageBytes(bytes: number | null | undefined): string {
  if (typeof bytes !== "number" || !Number.isFinite(bytes) || bytes < 0) return "-";
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  return `${(mb / 1024).toFixed(1)} GB`;
}

/** 尺寸展示：宽×高；任一边未知（后端不伪造）则显示 "-" */
export function formatImageSize(
  width: number | null | undefined,
  height: number | null | undefined
): string {
  if (typeof width !== "number" || typeof height !== "number") return "-";
  return `${width}×${height}`;
}

/** 替代文本：优先说明文案，其次格式，最后兜底「图片」 */
export function imageAltText(ref: Pick<ImageRef, "caption" | "mime"> | null): string {
  if (!ref) return "图片";
  const caption = ref.caption?.trim();
  if (caption) return caption;
  return ref.mime || "图片";
}

/** 展示态：正常展示 / 降级说明（未知格式、缺尺寸、无文件） */
export type ImageDisplayState = "ok" | "unsupported" | "missing_file" | "unknown_size";

export function imageDisplayState(ref: ImageRef | null): ImageDisplayState {
  if (!ref) return "missing_file";
  if (!isSupportedImage(ref.mime)) return "unsupported";
  if (ref.source === "file" && !ref.rel_path) return "missing_file";
  if (typeof ref.width !== "number" || typeof ref.height !== "number") return "unknown_size";
  return "ok";
}

/** 错误码 → 文案（未知错误码如实回显，不伪装成“成功”） */
export function imageErrorText(code: string | null | undefined): string {
  if (!code) return "";
  return IMAGE_ERROR_TEXT[code] ?? `图片处理失败（${code}）`;
}

/** 溯源展示：只显示来源主机，不渲染完整 URL（避免把脱敏前的原文带上屏） */
export function imageSourceHost(sourceUrl: string | null | undefined): string {
  if (!sourceUrl) return "-";
  try {
    return new URL(sourceUrl).host || "-";
  } catch {
    return "-";
  }
}
