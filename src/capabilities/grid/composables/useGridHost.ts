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
    gridCache.syncSession(grid.gridSession);
    const n = grid.gridCount;
    const mode = grid.gridLayout as GridLayoutMode;
    const refWidth = r.width;
    grid.gridRects.splice(0, grid.gridRects.length);
    for (let i = 0; i < n; i++) {
      const cell = gridCellRect(mode, i, n, r.width, r.height, GRID_GAP, grid.gridCols(n));
      grid.gridRects.push({ x: cell.x, y: cell.y, w: cell.w, h: cell.h });
      const rect = gridCellHostRect(r, cell);
      const sig = gridPositionSignature(rect.x, rect.y, rect.width, rect.height);
      if (gridCache.shouldSend(i, sig)) {
        bridge.gridPosition(i, rect).catch((e) => {
          gridCache.invalidate(i);
          bridge.debugLog(`gridPosition i=${i} 失败: ${e}`);
        });
      }
      bridge.gridSetZoom(i, normalizeZoom(cell.w, refWidth)).catch(() => {});
    }
  }

  grid.bindGridScheduler(scheduleGrid);
  return { scheduleGrid };
}
