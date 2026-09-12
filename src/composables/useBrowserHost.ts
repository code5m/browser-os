import { ref, onMounted, onBeforeUnmount, nextTick, watch } from "vue";
import { bridge } from "../bridge";
import { useBrowserStore } from "../stores/useBrowserStore";
import { useLayoutStore } from "../stores/useLayoutStore";
import {
  GRID_GAP,
  gridCellHostRect,
  gridCellRect,
  normalizeHostRect,
  normalizeZoom,
  type GridLayoutMode,
} from "../utils/browserLayout";
import {
  createBoundedRetrier,
  createGridSendCache,
  createHiddenIntent,
  createInvalidationGroup,
  createKeyDeduper,
  gridPositionSignature,
  hasNonZeroSize,
  tabPositionKey,
} from "../utils/browserSync";

// 单一 webview 定位调度器：全局只 1 个 ResizeObserver，合并同一帧多次请求。
//
// M6-S4：原先散落在模块级的手写同步防御（lastKey / lastAt / lastGridSession /
// lastGridSent / lastHiddenTab）收敛为可测的闭包工厂。仍为**模块级单例**，
// 保持原有"全局共享一份缓存"的语义不变。
let positionRaf: number | null = null;

const deduper = createKeyDeduper();
const gridCache = createGridSendCache();
const hiddenIntent = createHiddenIntent();
const invalidation = createInvalidationGroup();

// rect 持续为 0 时的重试上限：60 帧 ≈ 1s，与宫格 10×100ms 的布局稳定窗口对齐。
// 原实现为无上限 rAF 递归（潜在无限重试），此处收敛为有界重试，并在耗尽时留日志。
const MAX_ZERO_RECT_RETRIES = 60;
const zeroRectRetrier = createBoundedRetrier({
  maxRetries: MAX_ZERO_RECT_RETRIES,
  delayMs: 0,
  schedule: (fn) => requestAnimationFrame(() => fn()),
});
// 宫格布局重试：沿用原 10 次 × 100ms 的时序窗口
const gridRetrier = createBoundedRetrier();

// 一处失效、全部作废：原实现需手动清 3 个变量，漏一个即 bug
invalidation.register(() => deduper.reset());
invalidation.register(() => gridCache.reset());
invalidation.register(() => hiddenIntent.reset());

export function useBrowserHost() {
  const browser = useBrowserStore();
  const layout = useLayoutStore();
  const browserHost = ref<HTMLElement | null>(null);
  let ro: ResizeObserver | null = null;
  watch(() => layout.mainView, () => invalidation.invalidateAll());

  function schedulePosition(retry = 0) {
    if (positionRaf) return; // 合并同一帧内多次请求
    positionRaf = requestAnimationFrame(() => {
      positionRaf = requestAnimationFrame(() => {
        positionRaf = null;
        if (browser.gridOpen && layout.mainView === 'grid') {
          scheduleGrid();
          return;
        }
        if (layout.mainView !== "browser" || !browserHost.value || !browser.activeTabId)
          return;
        const host = browserHost.value;
        const r = host.getBoundingClientRect();
        // 若 viewport 尚未完成布局（v-if 刚挂载 / v-show 切换中），rect 可能为 0。
        // 不要直接跳过，而是延迟一帧重试，避免子 webview 永远停在 (0,0,1,1) 的偏位。
        // 重试有明确上限（60 帧），耗尽后留日志，避免无限 rAF 递归。
        if (!hasNonZeroSize(r)) {
          if (!zeroRectRetrier.scheduleRetry(retry, () => {
            positionRaf = null;
            schedulePosition(retry + 1);
          })) {
            bridge.debugLog(
              `schedulePosition: rect 持续为 0，重试 ${retry} 次已达上限 ${MAX_ZERO_RECT_RETRIES}，停止`
            );
          }
          return;
        }
        // 纯 CSS 像素坐标，绝不乘 devicePixelRatio（规则 3 锁定）
        const { x, y, width, height } = normalizeHostRect(r);
        const key = tabPositionKey(browser.activeTabId, x, y, width, height);
        if (!deduper.shouldSend(key)) return;
        bridge
          .tabPosition(browser.activeTabId, { x, y, width, height })
          .catch(() => {});
      });
    });
  }

  function scheduleGrid() {
    bridge.debugLog(`scheduleGrid entry gridOpen=${browser.gridOpen}`);
    if (!browser.gridOpen || layout.mainView !== 'grid') return;
    nextTick(() => {
      requestAnimationFrame(() => layoutGridNow(0));
    });
  }

  // 实际执行宫格布局。host 未就绪 / rect 为 0 时重试（10 次 × 100ms），
  // 覆盖"工具条刚展开/视图刚切换，布局尚未稳定"的时序窗口——之前直接 return
  // 导致宫格永不定位（灰底空白、无格子、无标题栏）。
  function layoutGridNow(retry: number) {
    if (!browser.gridOpen || layout.mainView !== 'grid') return;
    const host = browserHost.value;
    const r = host?.getBoundingClientRect();
    if (retry === 0) {
      bridge.debugLog(
        `layoutGridNow host=${!!host} rect=${r ? `${Math.round(r.left)},${Math.round(r.top)},${Math.round(r.width)}x${Math.round(r.height)}` : "null"}`
      );
    }
    if (!host || !hasNonZeroSize(r)) {
      if (!gridRetrier.scheduleRetry(retry, () => layoutGridNow(retry + 1))) {
        bridge.debugLog("layoutGridNow 重试 10 次仍无有效 host/rect，放弃");
      }
      return;
    }
    // 会话切换（buildGrid 重建）时清空发送缓存与隐藏记录
    if (gridCache.syncSession(browser.gridSession)) {
      hiddenIntent.reset();
    }
    const n = browser.gridCount;
    const mode = browser.gridLayout as GridLayoutMode;
    // 自适应缩放：以 host 满宽为参考 —— 每格都按比例缩小，完整页面缩进格宽
    const refWidth = r.width;
    // 同步每格相对 host 的 rect
    browser.gridRects.splice(0, browser.gridRects.length);
    for (let i = 0; i < n; i++) {
      const cell = gridCellRect(mode, i, n, r.width, r.height, GRID_GAP, browser.gridCols(n));
      browser.gridRects.push({ x: cell.x, y: cell.y, w: cell.w, h: cell.h });
      // 1) 定位（不含 zoom，避免每次定位都触发整页重排卡顿）。
      //    同会话同 rect 去重，失败则清除缓存下轮重发。
      const rect = gridCellHostRect(r, cell);
      const sig = gridPositionSignature(rect.x, rect.y, rect.width, rect.height);
      if (gridCache.shouldSend(i, sig)) {
        bridge.gridPosition(i, rect).catch((e) => {
          gridCache.invalidate(i);
          bridge.debugLog(`gridPosition i=${i} 失败: ${e}`);
        });
      }
      // 2) 缩放单独下发（后端按 label 去重，zoom 变化才真正应用）
      bridge
        .gridSetZoom(i, normalizeZoom(cell.w, refWidth))
        .catch(() => {});
    }
    // 宫格模式下把主浏览器页签移出可视区（保留状态）。
    // 用无去重的 hideWebview，避免 tabPosition 的 50ms 去重把移出请求丢弃。
    // 同一会话同一页签只移一次（layoutGridNow 会被反复触发）。
    if (browser.activeTabId && hiddenIntent.shouldHide(browser.activeTabId)) {
      bridge.hideWebview(browser.activeTabId).catch(() => {});
    }
  }

  function positionBrowserNow() {
    schedulePosition();
  }

  onMounted(() => {
    // 把调度器注入 store，供 store 的 tab 操作回调
    browser.bindPositionScheduler(schedulePosition);
    browser.bindGridScheduler(scheduleGrid);
    // 首次挂载时也同步显隐。主页/文件等非浏览器视图可能已经恢复了
    // 浏览器页签，但此时还没有发生 mainView 变化，旧 webview 会保留尺寸并
    // 漂移到主界面上方，形成白色遮挡。
    nextTick(() => {
      if (layout.mainView !== "browser" && layout.mainView !== "grid") {
        bridge.hideAllWebviews().catch(() => {});
      }
    });
    window.addEventListener("resize", positionBrowserNow);
    // 主窗缩放后 viewport 大小会变，需要重定位
    window.addEventListener("tauri://window-resized", positionBrowserNow as any);
    ro = new ResizeObserver(() => positionBrowserNow());
    if (browserHost.value) ro.observe(browserHost.value);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("resize", positionBrowserNow);
    if (ro) ro.disconnect();
  });

  return { browserHost, schedulePosition, scheduleGrid };
}
