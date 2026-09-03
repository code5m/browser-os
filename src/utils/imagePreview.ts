// M2-2.b 图片预览纯逻辑层（无 DOM / 无 Tauri 依赖，可 headless 断言）。
//
// 定位：画廊/灯箱/缩放里**所有可判定的行为**都下沉到这里，Vue 组件只负责渲染
// 与事件转发，因此 `scripts/check-image-preview-logic.mjs` 能直接加载本文件，
// 断言的是产品代码本身（不 mock、不重写逻辑）。
//
// 契约来源：`logs/checkpoints/M2-2.a-20260903-1352.md` §4（七项冻结契约）。
// 隐私：本层只产出 host 级溯源文案，**绝不返回 source_url 原文**；URL 脱敏由
// 后端 `images::redact_source_url` 负责，前端这里是第二道防线。

import type { ImageRef } from "../types";
import {
  formatImageBytes,
  formatImageSize,
  imageAltText,
  imageDisplayState,
  imageSourceHost,
  isSupportedImage,
  type ImageDisplayState,
} from "./image";

/** 缩放下限（冻结：0.25×） */
export const SCALE_MIN = 0.25;
/** 缩放上限（冻结：4×） */
export const SCALE_MAX = 4;
/** 缩放步长（冻结：1.2） */
export const SCALE_STEP = 1.2;

/** 缩略图最小列宽（含间距），容器变窄时列数自动下降 */
export const GRID_MIN_COL = 148;
/** 网格列数上限 */
export const GRID_MAX_COLS = 6;
/** 说明文案展示上限（超出省略，避免撑破网格） */
export const CAPTION_MAX = 48;

/** 缩放钳制到 [0.25, 4]；非有限值或非数字回落到 1（不放大也不报错） */
export function clampScale(scale: number): number {
  if (typeof scale !== "number" || !Number.isFinite(scale)) return 1;
  if (scale < SCALE_MIN) return SCALE_MIN;
  if (scale > SCALE_MAX) return SCALE_MAX;
  return scale;
}

/**
 * 下一张索引（**不循环**）：已是最后一张时返回原索引。
 * 空列表统一返回 -1（调用方据此不渲染任何图片）。
 */
export function nextIndex(index: number, length: number): number {
  if (!Number.isInteger(length) || length <= 0) return -1;
  if (!Number.isInteger(index) || index < 0 || index >= length) return length > 0 ? 0 : -1;
  return index + 1 < length ? index + 1 : index;
}

/** 上一张索引（**不循环**）：已是第一张时返回原索引（T-prev-3）。 */
export function prevIndex(index: number, length: number): number {
  if (!Number.isInteger(length) || length <= 0) return -1;
  if (!Number.isInteger(index) || index < 0 || index >= length) return 0;
  return index - 1 >= 0 ? index - 1 : index;
}

/** 滚轮缩放：向上（deltaY < 0）放大，向下缩小；到边界即停（T-prev-4/5）。 */
export function scaleForWheel(scale: number, deltaY: number): number {
  const base = clampScale(scale);
  if (typeof deltaY !== "number" || !Number.isFinite(deltaY) || deltaY === 0) return base;
  const next = deltaY < 0 ? base * SCALE_STEP : base / SCALE_STEP;
  return clampScale(next);
}

/** 双击：1× → 2×，其余一律复位到 1×（冻结：双击复位语义）。 */
export function scaleForDoubleClick(scale: number): number {
  return clampScale(scale) === 1 ? clampScale(2) : 1;
}

/** 按钮缩放（+1 档 / -1 档），同样受边界钳制 */
export function scaleForButton(scale: number, direction: 1 | -1): number {
  const base = clampScale(scale);
  return clampScale(direction > 0 ? base * SCALE_STEP : base / SCALE_STEP);
}

/**
 * 是否进入画廊/灯箱渲染。
 * 只有「已落盘的 File 型图片 + 受支持格式」才渲染；`inline_data_url`
 * （rel_path 为 null）按契约排除（`logs/checkpoints/M2-2.a` §4.1/§4.5）。
 */
export function shouldRenderImage(ref: ImageRef | null): boolean {
  if (!ref) return false;
  if (ref.source !== "file") return false;
  if (typeof ref.rel_path !== "string" || ref.rel_path.length === 0) return false;
  return isSupportedImage(ref.mime);
}

/**
 * 相对路径白名单（前端第二道防线，与后端 `images::validate_rel_path` 同口径）。
 * 只允许 `images/<id>/<id>.<ext>` 形态：无 `..`、无绝对路径、无空段、无反斜杠。
 */
export function isSafeRelPath(rel: string | null | undefined): boolean {
  if (typeof rel !== "string" || rel.length === 0 || rel.length > 256) return false;
  if (rel.startsWith("/") || rel.endsWith("/")) return false;
  if (rel.includes("\\") || rel.includes("\0") || rel.includes("//")) return false;
  const segs = rel.split("/");
  if (segs.some((s) => s === "" || s === "." || s === "..")) return false;
  return segs.every((s) => /^[A-Za-z0-9._-]+$/.test(s));
}

/**
 * 拼接「目录基准 + 相对路径」为绝对路径（前端版，与后端 `join_image_path` 同义）。
 *
 * 失败返回空串：宁可显示占位图，也不退回相对路径读取（fail-closed）。
 * **不含** `convertFileSrc`——那是 Tauri 运行时调用，留在组件层，本文件保持可 headless。
 */
export function buildAssetSrc(dir: string | null | undefined, rel: string | null | undefined): string {
  if (typeof dir !== "string" || typeof rel !== "string") return "";
  if (!isSafeRelPath(rel)) return "";
  const base = dir.replace(/\/+$/, "");
  if (base.length === 0) return "";
  return `${base}/${rel}`;
}

/**
 * 初始缩放：按「适应容器」计算且**不放大**（T-prev-10：1×1 像素图保持 1×，
 * 不被拉成全屏色块）；尺寸未知时回落到 1。
 */
export function fitScale(
  imgWidth: number | null | undefined,
  imgHeight: number | null | undefined,
  boxWidth: number | null | undefined,
  boxHeight: number | null | undefined,
): number {
  const w = typeof imgWidth === "number" && imgWidth > 0 ? imgWidth : 0;
  const h = typeof imgHeight === "number" && imgHeight > 0 ? imgHeight : 0;
  const bw = typeof boxWidth === "number" && boxWidth > 0 ? boxWidth : 0;
  const bh = typeof boxHeight === "number" && boxHeight > 0 ? boxHeight : 0;
  if (w === 0 || h === 0 || bw === 0 || bh === 0) return 1;
  return clampScale(Math.min(1, Math.min(bw / w, bh / h)));
}

/** 仅放大态才允许拖拽平移（冻结：scale > 1）。 */
export function dragEnabled(scale: number): boolean {
  return clampScale(scale) > 1;
}

/** 平移偏移钳制，避免把图片拖出可视区太远（按当前缩放比例放宽） */
export function clampTranslate(offset: number, scale: number): number {
  const s = clampScale(scale);
  if (typeof offset !== "number" || !Number.isFinite(offset)) return 0;
  const limit = 240 * Math.max(1, s);
  if (offset > limit) return limit;
  if (offset < -limit) return -limit;
  return offset;
}

/** 网格列数：容器越窄列数越少，永不横向溢出（T-prev-12）。 */
export function gridColumns(containerWidth: number | null | undefined): number {
  const w = typeof containerWidth === "number" && Number.isFinite(containerWidth) ? containerWidth : 0;
  if (w <= 0) return 1;
  const cols = Math.floor(w / GRID_MIN_COL);
  if (cols < 1) return 1;
  return Math.min(cols, GRID_MAX_COLS);
}

/** 画廊整体状态：loading 优先于 empty，避免骨架与空态同时出现（T-prev-1/11）。 */
export type GalleryState = "loading" | "empty" | "ready";

export function galleryState(loading: boolean, count: number): GalleryState {
  if (loading) return "loading";
  if (!Number.isFinite(count) || count <= 0) return "empty";
  return "ready";
}

/** 说明文案截断（超出加省略号，不撑破网格，T-prev-9）。 */
export function truncateText(text: string | null | undefined, max: number = CAPTION_MAX): string {
  const raw = typeof text === "string" ? text.trim() : "";
  if (raw.length === 0) return "";
  const limit = Number.isInteger(max) && max > 1 ? max : CAPTION_MAX;
  return raw.length <= limit ? raw : `${raw.slice(0, limit - 1)}…`;
}

/**
 * 键盘动作映射（纯映射，便于逐键断言）。
 *
 * 命名一律带 `zoom_` 前缀：M1-7 的 `check-git-ui-policy.py` 会扫描全前端的
 * Git 黑名单操作字面量（reset / clean / merge 等），无前缀的 reset 字面量会被
 * 误判为「前端想绕过 Git 白名单」。这里加前缀既避免误报，也让语义更明确。
 */
export type PreviewKeyAction =
  | "close"
  | "prev"
  | "next"
  | "zoom_in"
  | "zoom_out"
  | "zoom_reset"
  | null;

export function keyAction(key: string): PreviewKeyAction {
  switch (key) {
    case "Escape":
    case "Esc":
      return "close";
    case "ArrowLeft":
      return "prev";
    case "ArrowRight":
      return "next";
    case "+":
    case "=":
      return "zoom_in";
    case "-":
      return "zoom_out";
    case "0":
      return "zoom_reset";
    default:
      return null;
  }
}

/**
 * 键盘监听绑定器（纯逻辑，注入 add/remove 以便 headless 断言）。
 *
 * 契约：`bind()` 幂等（重复调用不叠加监听），`unbind()` 在未绑定时也是安全的空操作。
 * 组件在灯箱 open 时 bind、关闭与卸载时 unbind，杜绝监听泄漏（T-prev-6/7）。
 */
export interface KeyBinderHooks {
  add: (handler: (key: string) => void) => void;
  remove: (handler: (key: string) => void) => void;
}

export interface KeyBinder {
  handler: (key: string) => void;
  bind: () => void;
  unbind: () => void;
  isBound: () => boolean;
  bindCount: () => number;
  unbindCount: () => number;
}

export function createKeyBinder(
  hooks: KeyBinderHooks,
  dispatch: (key: string) => void = () => {},
): KeyBinder {
  let bound = false;
  let adds = 0;
  let removes = 0;
  const handler = (key: string) => {
    dispatch(key);
  };
  return {
    handler,
    bind() {
      if (bound) return;
      bound = true;
      adds += 1;
      hooks.add(handler);
    },
    unbind() {
      if (!bound) return;
      bound = false;
      removes += 1;
      hooks.remove(handler);
    },
    isBound: () => bound,
    bindCount: () => adds,
    unbindCount: () => removes,
  };
}

/** 预览项：组件渲染所需的全部展示数据（**结构上不含 source_url 原文**） */
export interface PreviewItem {
  id: string;
  /** 绝对路径（组件层再经 convertFileSrc 转 asset://）；不可用时为空串 */
  src: string;
  alt: string;
  /** 溯源只出主机 */
  host: string;
  sizeText: string;
  bytesText: string;
  caption: string;
  state: ImageDisplayState;
  renderable: boolean;
}

/** 把 ImageRef 组装成渲染项；不可渲染时给出降级态而不抛错。 */
export function buildPreviewItem(ref: ImageRef, dir: string | null | undefined): PreviewItem {
  const src = buildAssetSrc(dir, ref.rel_path);
  return {
    id: ref.id,
    src,
    alt: imageAltText(ref),
    host: imageSourceHost(ref.source_url),
    sizeText: formatImageSize(ref.width, ref.height),
    bytesText: formatImageBytes(ref.bytes),
    caption: truncateText(ref.caption),
    state: imageDisplayState(ref),
    renderable: shouldRenderImage(ref) && src.length > 0,
  };
}

/**
 * 失败态原语（纯函数，便于 headless 断言）。
 *
 * 单图加载失败只标记**这一张**：其余格子/其余图片照常渲染，
 * 且**不关闭灯箱**（T-prev-2，冻结契约 §4.3）。
 */
export type FailedMap = Record<string, boolean>;
export type RetryMap = Record<string, number>;

export function markFailed(failed: FailedMap, id: string): FailedMap {
  if (typeof id !== "string" || id.length === 0) return failed;
  return { ...failed, [id]: true };
}

export function clearFailed(failed: FailedMap, id: string): FailedMap {
  if (!(id in failed)) return failed;
  const next = { ...failed };
  delete next[id];
  return next;
}

export function isFailed(failed: FailedMap, id: string): boolean {
  return failed?.[id] === true;
}

/** 重试：递增 nonce（组件据此给 <img> 换 key，强制重新加载同一个 src） */
export function bumpRetry(retries: RetryMap, id: string): RetryMap {
  if (typeof id !== "string" || id.length === 0) return retries;
  return { ...retries, [id]: (retries[id] ?? 0) + 1 };
}

export function retryNonce(retries: RetryMap, id: string): number {
  return retries?.[id] ?? 0;
}

/** 批量组装，并**过滤掉**不可渲染项（inline_data_url 与无 rel_path 的一律不进预览）。 */
export function buildPreviewItems(
  images: ImageRef[] | null | undefined,
  dir: string | null | undefined,
): PreviewItem[] {
  if (!Array.isArray(images)) return [];
  return images
    .filter((r) => shouldRenderImage(r))
    .map((r) => buildPreviewItem(r, dir))
    .filter((it) => it.renderable);
}
