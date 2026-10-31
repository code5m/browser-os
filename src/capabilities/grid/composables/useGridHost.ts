import { nextTick, type Ref } from "vue";
import { bridge } from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useGridStore } from "../public";
import {
  GRID_GAP,
  gridCellHostRect,
  gridCellRect,
  normalizeZoom,
  type GridLayoutMode,
} from "../../../utils/browserLayout";
import {
  createGridSendCache,
  createBoundedRetrier,
  gridPositionSignature,
  hasNonZeroSize,
} from "../../../utils/browserSync";

const gridCache = createGridSendCache();
const gridRetrier = createBoundedRetrier();
// Coalesce rapid ResizeObserver updates while one native UDS request is in flight.
// Only the latest geometry survives; stale rectangles are never replayed.
const pendingRects = new Map<number, { rect: {x:number;y:number;width:number;height:number}; sig:string }>();
const sendingRects = new Set<number>();
let positioningEpoch = 0;
async function flushRect(index: number, epoch: number) {
  if (sendingRects.has(index)) return;
  sendingRects.add(index);
  try {
    while (epoch === positioningEpoch) {
      const next = pendingRects.get(index);
      if (!next) break;
      pendingRects.delete(index);
      try {
        await bridge.gridPosition(index, next.rect);
      } catch (e) {
        gridCache.invalidate(index);
        bridge.debugLog(`gridPosition i=${index} 失败: ${e}`);
      }
    }
  } finally {
    sendingRects.delete(index);
    if (pendingRects.has(index)) void flushRect(index, positioningEpoch);
  }
}


export function useGridHost(browserHost: Ref<HTMLElement | null>) {
  const grid = useGridStore();
  const layout = useLayoutStore();

  function scheduleGrid() {
    bridge.debugLog(`scheduleGrid entry gridOpen=${grid.gridOpen}`);
    if (!grid.gridOpen || layout.mainView !== "grid") return;
    nextTick(() => requestAnimationFrame(() => layoutGridNow(0)));
  }

  function layoutGridNow(retry: number) {
    if (!grid.gridOpen || layout.mainView !== "grid") return;
    const host = browserHost.value;
    const r = host?.getBoundingClientRect();
    if (!host || !hasNonZeroSize(r)) {
      if (!gridRetrier.scheduleRetry(retry, () => layoutGridNow(retry + 1))) {
        bridge.debugLog("layoutGridNow 重试 10 次仍无有效 host/rect，放弃");
      }
      return;
    }
    if (gridCache.syncSession(grid.gridSession)) {
      ++positioningEpoch;
      pendingRects.clear();
    }
    const n = grid.gridCount;
    for (const index of pendingRects.keys()) if (index >= n) pendingRects.delete(index);
    const mode = grid.gridLayout as GridLayoutMode;
    const refWidth = r.width;
    grid.gridRects.splice(0, grid.gridRects.length);
    for (let i = 0; i < n; i++) {
      const cell = gridCellRect(mode, i, n, r.width, r.height, GRID_GAP, grid.gridCols(n));
      grid.gridRects.push({ x: cell.x, y: cell.y, w: cell.w, h: cell.h });
      const rect = gridCellHostRect(r, cell);
      const sig = gridPositionSignature(rect.x, rect.y, rect.width, rect.height);
      if (gridCache.shouldSend(i, sig)) {
        pendingRects.set(i, { rect, sig });
        void flushRect(i, positioningEpoch);
      }
      bridge.gridSetZoom(i, normalizeZoom(cell.w, refWidth)).catch(() => {});
    }
  }

  grid.bindGridScheduler(scheduleGrid);
  return { scheduleGrid };
}
