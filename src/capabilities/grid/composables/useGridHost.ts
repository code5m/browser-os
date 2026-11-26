import { onScopeDispose, ref, watch, type Ref } from "vue";
import { bridge } from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useGridStore } from "../public";
import { isGridResourceAllowed } from "../resource/guard";
import { nativeSurfaceBlocked, trackNativeLayout } from "../../../utils/nativeSurfaceSync";
import {
  GRID_GAP, gridCellHostRect, gridCellRect, normalizeZoom, type GridLayoutMode,
} from "../../../utils/browserLayout";
import { createGridSendCache, gridPositionSignature, hasNonZeroSize } from "../../../utils/browserSync";

type Rect = { x:number; y:number; width:number; height:number };
type LayoutIntent = {
  owner: symbol; rect: Rect; zoom: number; current: () => boolean;
  settled: (success: boolean) => void;
};
// Native calls already issued stay owned until settlement, even across
// component remount. Only the newest unsent intent per native route survives.
const pending = new Map<number, LayoutIntent>();
const running = new Map<number, Promise<void>>();
function flush(index: number) {
  if (running.has(index)) return;
  const task = Promise.resolve().then(async () => {
    for (;;) {
      const next = pending.get(index);
      if (!next) return;
      pending.delete(index);
      if (!next.current()) continue;
      try {
        await bridge.gridPosition(index, next.rect);
        // A newer layout owns both operations. Never issue an old zoom
        // against the cell that may have been replaced during position.
        if (!next.current() || pending.has(index)) continue;
        await bridge.gridSetZoom(index, next.zoom);
        if (next.current() && !pending.has(index)) next.settled(true);
      } catch {
        if (next.current() && !pending.has(index)) next.settled(false);
      }
    }
  }).finally(() => {
    running.delete(index);
    if (pending.has(index)) flush(index);
  });
  running.set(index, task);
  trackNativeLayout(task);
}
async function settleIssuedLayout(): Promise<void> {
  // Called only after the store's synchronous suspension gate invalidates
  // pending work. Timeout does not release the original workers or authorize
  // an unacknowledged suspension.
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    await Promise.race([
      Promise.all([...running.values()]),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error("GRID_LAYOUT_SETTLEMENT_PENDING")), 15000);
      }),
    ]);
  } finally { if (timer !== undefined) clearTimeout(timer); }
}

export function useGridHost(browserHost: Ref<HTMLElement | null>) {
  const grid = useGridStore();
  const layout = useLayoutStore();
  const owner = Symbol("grid-layout-host");
  const cache = createGridSendCache();
  const layoutIssue = ref("");
  const failed = new Set<number>();
  let epoch = 0;
  let disposed = false;
  let frame: number | null = null;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;

  function eligible() {
    return !disposed && !nativeSurfaceBlocked.value && grid.gridOpen && !grid.gridStarting && !grid.gridClosing
      && !grid.gridCleanupUnconfirmed && !grid.gridLayoutSuspended
      && layout.mainView === "grid" && isGridResourceAllowed();
  }
  function invalidate() {
    ++epoch;
    if (frame !== null) cancelAnimationFrame(frame);
    if (retryTimer !== null) clearTimeout(retryTimer);
    frame = null; retryTimer = null;
    cache.reset(); failed.clear(); layoutIssue.value = "";
    for (const [index, intent] of pending) if (intent.owner === owner) pending.delete(index);
  }
  function scheduleGrid() {
    if (!eligible() || frame !== null) return;
    if (retryTimer !== null) clearTimeout(retryTimer);
    retryTimer = null;
    const admitted = epoch;
    frame = requestAnimationFrame(() => {
      frame = null;
      layoutGridNow(admitted, 0);
    });
  }
  function layoutGridNow(admitted: number, retry: number) {
    if (admitted !== epoch || !eligible()) return;
    const r = browserHost.value?.getBoundingClientRect();
    if (!hasNonZeroSize(r) || !r) {
      if (retry < 10) {
        retryTimer = setTimeout(() => {
          retryTimer = null;
          layoutGridNow(admitted, retry + 1);
        }, 100);
      } else layoutIssue.value = "宫格容器尺寸尚未确认，请重新布局。";
      return;
    }
    const n = grid.gridCount;
    for (const [index, intent] of pending) {
      if (intent.owner === owner && index >= n) pending.delete(index);
    }
    const mode = grid.gridLayout as GridLayoutMode;
    grid.gridRects.splice(0, grid.gridRects.length);
    for (let i = 0; i < n; i++) {
      const cell = gridCellRect(mode, i, n, r.width, r.height, GRID_GAP, grid.gridCols(n));
      grid.gridRects.push({ x: cell.x, y: cell.y, w: cell.w, h: cell.h });
      const rect = gridCellHostRect(r, cell);
      const zoom = normalizeZoom(cell.w, r.width);
      const sig = gridPositionSignature(rect.x, rect.y, rect.width, rect.height) + ":" + zoom;
      if (!cache.shouldSend(i, sig)) continue;
      pending.set(i, {
        owner, rect, zoom,
        current: () => admitted === epoch && i < grid.gridCount && eligible(),
        settled: (success) => {
          if (success) failed.delete(i);
          else { cache.invalidate(i); failed.add(i); }
          layoutIssue.value = failed.size
            ? "部分宫格布局或缩放未确认，请手动重新布局。" : "";
        },
      });
      flush(i);
    }
  }

  // Invalidate synchronously on topology/visibility changes, before a pending
  // microtask, animation frame, delayed retry or second IPC can enter.
  watch(
    () => [layout.mainView, nativeSurfaceBlocked.value, grid.gridOpen, grid.gridStarting, grid.gridClosing,
      grid.gridCleanupUnconfirmed, grid.gridLayoutSuspended, grid.gridSession,
      grid.gridCount, grid.gridLayout] as const,
    () => { invalidate(); if (eligible()) scheduleGrid(); },
    { immediate: true, flush: "sync" },
  );
  const unbind = grid.bindGridScheduler(scheduleGrid, settleIssuedLayout);
  onScopeDispose(() => {
    disposed = true;
    invalidate();
    unbind();
  });
  return { scheduleGrid, layoutIssue };
}
