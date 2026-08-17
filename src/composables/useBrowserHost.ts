import { ref, onMounted, onBeforeUnmount, nextTick } from "vue";
import { bridge } from "../bridge";
import { useBrowserStore } from "../stores/useBrowserStore";
import { useLayoutStore } from "../stores/useLayoutStore";

// 单一 webview 定位调度器：全局只 1 个 ResizeObserver，合并同一帧多次请求
let positionRaf: number | null = null;
let lastKey = "";
let lastAt = 0;

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
    if (mode === "vertical") {
      // 纵向：一列竖排
      const ch = (H - gap * (n - 1)) / n;
      return { x: 0, y: i * (ch + gap), w: W, h: ch };
    }
    if (mode === "quad") {
      // 四分：固定 2x2，取前 4 格，超出堆叠在第 4 格
      const cw = (W - gap) / 2;
      const ch = (H - gap) / 2;
      const idx = Math.min(i, 3);
      const c = idx % 2;
      const rw = Math.floor(idx / 2);
      return { x: c * (cw + gap), y: rw * (ch + gap), w: cw, h: ch };
    }
    if (mode === "free") {
      // 自由：层叠错开（可拖拽基础，先做瀑布式偏移堆叠）
      const cw = W * 0.7;
      const ch = H * 0.7;
      const off = Math.min(i, 8) * 28;
      return { x: off, y: off, w: cw, h: ch };
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
    const n = browser.gridCount;
    if (!browser.gridOpen) return;
    nextTick(() => {
      requestAnimationFrame(() => {
        const host = browserHost.value;
        if (!host) return;
        const r = host.getBoundingClientRect();
        if (!r.width || !r.height) return;
        const gap = 4;
        const mode = browser.gridLayout;
        // 宫格同样走 Logical(CSS) 坐标，不乘 devicePixelRatio
        for (let i = 0; i < n; i++) {
          const cell = gridCellRect(mode, i, n, r.width, r.height, gap);
          const x = Math.round(r.left + cell.x);
          const y = Math.round(r.top + cell.y);
          bridge
            .gridPosition(i, { x, y, width: Math.round(cell.w), height: Math.round(cell.h) })
            .catch(() => {});
        }
        // 宫格模式下把主浏览器页签移出可视区（保留状态）
        if (browser.activeTabId) {
          bridge
            .tabPosition(browser.activeTabId, { x: -30000, y: -30000, width: 1, height: 1 })
            .catch(() => {});
        }
      });
    });
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
