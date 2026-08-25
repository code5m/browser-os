import { ref, onMounted, onBeforeUnmount, nextTick } from "vue";
import { bridge } from "../bridge";
import { useBrowserStore } from "../stores/useBrowserStore";
import { useLayoutStore } from "../stores/useLayoutStore";

// 单一 webview 定位调度器：全局只 1 个 ResizeObserver，合并同一帧多次请求
let positionRaf: number | null = null;
let lastKey = "";
let lastAt = 0;
// 宫格定位发送缓存：同一会话内相同 rect 不重复下发（RO/事件会反复触发 scheduleGrid），
// buildGrid 重建（gridSession+1）后缓存作废必须重发——新 webview 初始是 1x1。
let lastGridSession = -1;
const lastGridSent = new Map<number, string>();
let lastHiddenTab = "";

export function useBrowserHost() {
  const browser = useBrowserStore();
  const layout = useLayoutStore();
  const browserHost = ref<HTMLElement | null>(null);
  let ro: ResizeObserver | null = null;

  function schedulePosition() {
    if (positionRaf) return; // 合并同一帧内多次请求
    positionRaf = requestAnimationFrame(() => {
      positionRaf = requestAnimationFrame(() => {
        positionRaf = null;
        if (browser.gridOpen) {
          scheduleGrid();
          return;
        }
        if (layout.mainView !== "browser" || !browserHost.value || !browser.activeTabId)
          return;
        const host = browserHost.value;
        const r = host.getBoundingClientRect();
        // 若 viewport 尚未完成布局（v-if 刚挂载 / v-show 切换中），rect 可能为 0。
        // 不要直接跳过，而是延迟一帧重试，避免子 webview 永远停在 (0,0,1,1) 的偏位。
        if (!r.width || !r.height) {
          requestAnimationFrame(() => {
            positionRaf = null;
            schedulePosition();
          });
          return;
        }
        // 前端 JS 运行在主窗 webview 内，getBoundingClientRect 的 x/y 在主窗内容区坐标系下
        // 已经是相对主窗内容区左上角的 CSS 坐标。后端已迁移到 browser-tabs 插件，
        // 全链路使用 Logical(CSS) 坐标，由 Tauri 统一处理 DPI，前端【不再乘 devicePixelRatio】。
        const x = Math.round(r.left);
        const y = Math.round(r.top);
        const w = Math.round(r.width);
        const h = Math.round(r.height);
        const key = `${browser.activeTabId}:${x},${y},${w},${h}`;
        const now = performance.now();
        if (key === lastKey && now - lastAt < 50) return;
        lastKey = key;
        lastAt = now;
        bridge
          .tabPosition(browser.activeTabId, { x, y, width: w, height: h })
          .catch(() => {});
      });
    });
  }

  // 按布局模式计算第 i 格的 rect（相对 host 内容区，CSS 坐标）
  function gridCellRect(
    mode: string,
    i: number,
    n: number,
    W: number,
    H: number,
    gap: number
  ): { x: number; y: number; w: number; h: number } {
    if (mode === "horizontal") {
      // 横向：一排横排
      const cw = (W - gap * (n - 1)) / n;
      return { x: i * (cw + gap), y: 0, w: cw, h: H };
    }
    if (mode === "quad" && n === 4) {
      // 四分：固定 2x2（仅 n=4 时；n<4 走下方通用算法避免"只占上排、下半空白"）
      const cw = (W - gap) / 2;
      const ch = (H - gap) / 2;
      const idx = Math.min(i, 3);
      const c = idx % 2;
      const rw = Math.floor(idx / 2);
      return { x: c * (cw + gap), y: rw * (ch + gap), w: cw, h: ch };
    }
    // grid：自动宫格（默认）
    const cols = browser.gridCols(n);
    const rows = Math.ceil(n / cols);
    const cw = (W - gap * (cols - 1)) / cols;
    const ch = (H - gap * (rows - 1)) / rows;
    const c = i % cols;
    const rw = Math.floor(i / cols);
    return { x: c * (cw + gap), y: rw * (ch + gap), w: cw, h: ch };
  }

  function scheduleGrid() {
    bridge.debugLog(`scheduleGrid entry gridOpen=${browser.gridOpen}`);
    if (!browser.gridOpen) return;
    nextTick(() => {
      requestAnimationFrame(() => layoutGridNow(0));
    });
  }

  // 实际执行宫格布局。host 未就绪 / rect 为 0 时重试（最多 10 次，每次 100ms），
  // 覆盖"工具条刚展开/视图刚切换，布局尚未稳定"的时序窗口——之前直接 return
  // 导致宫格永不定位（灰底空白、无格子、无标题栏）。
  function layoutGridNow(retry: number) {
    if (!browser.gridOpen) return;
    const host = browserHost.value;
    const r = host?.getBoundingClientRect();
    if (retry === 0) {
      bridge.debugLog(
        `layoutGridNow host=${!!host} rect=${r ? `${Math.round(r.left)},${Math.round(r.top)},${Math.round(r.width)}x${Math.round(r.height)}` : "null"}`
      );
    }
    if (!host || !r || !r.width || !r.height) {
      if (retry < 10) window.setTimeout(() => layoutGridNow(retry + 1), 100);
      else bridge.debugLog("layoutGridNow 重试 10 次仍无有效 host/rect，放弃");
      return;
    }
    {
        // 会话切换（buildGrid 重建）时清空发送缓存
        if (browser.gridSession !== lastGridSession) {
          lastGridSession = browser.gridSession;
          lastGridSent.clear();
          lastHiddenTab = "";
        }
        const n = browser.gridCount;
        const gap = 4; // 宫格间隙（白底细分隔线即可，不再需要灰底大间隙）
        const mode = browser.gridLayout;
        // 自适应缩放：以 host 满宽为参考 —— 每格都按比例缩小，完整页面缩进格宽
        // （原设计 2 格时 zoom=1 不缩放，用户反馈"宫格没有自动适应缩放"）
        const refWidth = r.width;
        // 同步每格相对 host 的 rect
        browser.gridRects.splice(0, browser.gridRects.length);
        // 宫格同样走 Logical(CSS) 坐标，不乘 devicePixelRatio
        const BAR = 0; // 格子顶部标题栏已移除，网页占满整格
        for (let i = 0; i < n; i++) {
          const cell = gridCellRect(mode, i, n, r.width, r.height, gap);
          // 标题栏占每格顶部 BAR 高度，网页内容区下移并减高
          const contentY = cell.y + BAR;
          const contentH = Math.max(40, cell.h - BAR);
          const x = Math.round(r.left + cell.x);
          const y = Math.round(r.top + contentY);
          browser.gridRects.push({ x: cell.x, y: cell.y, w: cell.w, h: cell.h });
          // 1) 定位（不含 zoom，避免每次定位都触发整页重排卡顿）。
          //    同会话同 rect 去重，失败则清除缓存下轮重发。
          const w = Math.round(cell.w);
          const h = Math.round(contentH);
          const sig = `${x},${y},${w}x${h}`;
          if (lastGridSent.get(i) !== sig) {
            lastGridSent.set(i, sig);
            bridge.gridPosition(i, { x, y, width: w, height: h }).catch((e) => {
              lastGridSent.delete(i);
              bridge.debugLog(`gridPosition i=${i} 失败: ${e}`);
            });
          }
          // 2) 缩放单独下发（后端按 label 去重，zoom 变化才真正应用）
          let zoom = refWidth > 0 ? cell.w / refWidth : 1;
          zoom = Math.max(0.3, Math.min(1, zoom));
          bridge
            .gridSetZoom(i, Math.round(zoom * 100) / 100)
            .catch(() => {});
        }
        // 宫格模式下把主浏览器页签移出可视区（保留状态）。
        // 用无去重的 hideWebview，避免 tabPosition 的 50ms 去重把移出请求丢弃。
        // 同一会话同一页签只移一次（layoutGridNow 会被反复触发）。
        if (browser.activeTabId && lastHiddenTab !== browser.activeTabId) {
          lastHiddenTab = browser.activeTabId;
          bridge.hideWebview(browser.activeTabId).catch(() => {});
        }
    }
  }

  function positionBrowserNow() {
    schedulePosition();
  }

  onMounted(() => {
    // 把调度器注入 store，供 store 的 tab 操作回调
    browser.bindPositionScheduler(schedulePosition);
    browser.bindGridScheduler(scheduleGrid);
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
