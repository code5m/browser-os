// 浏览器 / 宫格定位的纯几何计算层（M6 Reduced Scope · S2）。
//
// 从 src/composables/useBrowserHost.ts 抽取，算法与现状逐行等价，不改动任何
// 数值、分支条件与取整时机。
//
// 硬约束：
// - 纯函数：零 DOM、零 bridge、零 Vue、零 store 依赖，可在 Node 下直接单测。
// - 坐标语义：纯 CSS 像素（Logical），一律 Math.round，**绝不乘 devicePixelRatio**
//   （PROJECT-RULES 规则 3 锁定；DPI 由 Tauri / wry 统一处理）。

/** 宫格布局模式 */
export type GridLayoutMode = "horizontal" | "quad" | "grid";

/** 相对 host 内容区的格子矩形（未取整，供 gridRects / 覆盖层使用） */
export interface GridCellRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** 下发给后端的矩形：CSS 像素整数 */
export interface LayoutRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** 只读原始矩形视图（测试可用普通对象代入，无需真实 DOMRect） */
export interface RawRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** 宫格间隙（CSS px），与 useBrowserHost.layoutGridNow 的 `const gap = 4` 一致 */
export const GRID_GAP = 4;

/** 格子顶部标题栏高度：标题栏已移除、网页占满整格 ⇒ 恒为 0 */
export const GRID_BAR_HEIGHT = 0;

/** 单格内容区最小高度（CSS px），防止极端布局下 webview 塌成 0 */
export const GRID_MIN_CONTENT_HEIGHT = 40;

/** 缩放钳制下界（与现状一致） */
export const ZOOM_MIN = 0.3;

/** 缩放钳制上界：只缩小不放大（与现状一致） */
export const ZOOM_MAX = 1;

/**
 * 自动宫格列数。与 useBrowserStore.gridCols 逐行等价，复制到此处以去掉 store 依赖。
 * 不变量：n>=1 时返回值恒 >=1，保证后续除法不会除零。
 */
export function gridCols(n: number): number {
  if (n <= 2) return n;
  if (n === 3) return 3;
  if (n === 4) return 2;
  if (n <= 6) return 3;
  if (n <= 8) return 4;
  if (n <= 10) return 5;
  return 6;
}

/**
 * 按布局模式计算第 i 格的矩形（相对 host 内容区左上角，CSS 像素，**未取整**）。
 *
 * 三分支顺序与现状完全一致：
 * - "horizontal"：一排横排，均分宽度（扣 (n-1)*gap），满高 H
 * - "quad" 且 n === 4：固定 2x2
 * - 其它（含 "quad" 但 n !== 4）：通用自动宫格
 *
 * @param cols 可选显式列数；省略时回退本模块 gridCols(n)
 */
export function gridCellRect(
  mode: GridLayoutMode,
  i: number,
  n: number,
  W: number,
  H: number,
  gap: number,
  cols?: number,
): GridCellRect {
  if (mode === "horizontal") {
    const cw = (W - gap * (n - 1)) / n;
    return { x: i * (cw + gap), y: 0, w: cw, h: H };
  }
  if (mode === "quad" && n === 4) {
    const cw = (W - gap) / 2;
    const ch = (H - gap) / 2;
    const idx = Math.min(i, 3);
    const c = idx % 2;
    const rw = Math.floor(idx / 2);
    return { x: c * (cw + gap), y: rw * (ch + gap), w: cw, h: ch };
  }
  const col = cols ?? gridCols(n);
  const rows = Math.ceil(n / col);
  const cw = (W - gap * (col - 1)) / col;
  const ch = (H - gap * (rows - 1)) / rows;
  const c = i % col;
  const rw = Math.floor(i / col);
  return { x: c * (cw + gap), y: rw * (ch + gap), w: cw, h: ch };
}

/**
 * 单页签定位：宿主 DOMRect → 下发矩形（与 useBrowserHost L49-52 等价）。
 * 仅 Math.round，绝不乘 devicePixelRatio。
 */
export function normalizeHostRect(r: RawRect): LayoutRect {
  return {
    x: Math.round(r.left),
    y: Math.round(r.top),
    width: Math.round(r.width),
    height: Math.round(r.height),
  };
}

/**
 * 宫格格子：相对 rect + host 原点 → 下发矩形（与 useBrowserHost L143-152 等价）。
 * contentH 用 GRID_MIN_CONTENT_HEIGHT 兜底，BAR 恒为 0。
 */
export function gridCellHostRect(host: RawRect, cell: GridCellRect): LayoutRect {
  const contentY = cell.y + GRID_BAR_HEIGHT;
  const contentH = Math.max(GRID_MIN_CONTENT_HEIGHT, cell.h - GRID_BAR_HEIGHT);
  return {
    x: Math.round(host.left + cell.x),
    y: Math.round(host.top + contentY),
    width: Math.round(cell.w),
    height: Math.round(contentH),
  };
}

/**
 * 宫格自适应缩放（与 useBrowserHost L161-166 等价）。
 * 以 host 满宽为参考，钳制到 [0.3, 1] 并保留 2 位小数。
 */
export function normalizeZoom(cellW: number, refWidth: number): number {
  const raw = refWidth > 0 ? cellW / refWidth : 1;
  const clamped = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, raw));
  return Math.round(clamped * 100) / 100;
}
