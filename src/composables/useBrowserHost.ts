import { ref, onMounted, onBeforeUnmount, nextTick, watch } from "vue";
import { bridge } from "../bridge";
import { normalizeHostRect } from "../utils/browserLayout";
import { useBrowserStore } from "../capabilities/browser/public";
import { useLayoutStore } from "../stores/useLayoutStore";
import {
  createBoundedRetrier,
  createInvalidationGroup,
  createKeyDeduper,
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

// 一处失效、全部作废：原实现需手动清 3 个变量，漏一个即 bug
invalidation.register(() => deduper.reset());

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
        const tabId = browser.activeTabId;
        const key = tabPositionKey(tabId, x, y, width, height);
        if (!deduper.shouldSend(key)) return;
        bridge.tabPosition(tabId, { x, y, width, height }).catch((error) => {
          // A failed native placement is NOT a successful sync. Reset the cache so
          // the same rect can be retried after WebView creation/activation settles.
          if (layout.mainView !== "browser" || browser.activeTabId !== tabId) return;
          deduper.reset();
          bridge.debugLog(`tabPosition failed for ${tabId}: ${String(error)}`);
          if (retry < 3) {
            window.setTimeout(() => schedulePosition(retry + 1), 180);
          } else {
            layout.showToast("当前页签定位失败，请切换页签重试");
          }
        });
      });
    });
  }

  function positionBrowserNow() {
    schedulePosition();
  }

  onMounted(() => {
    // 把调度器注入 store，供 store 的 tab 操作回调
    browser.bindPositionScheduler(schedulePosition);
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
    window.removeEventListener("tauri://window-resized", positionBrowserNow as any);
    if (positionRaf !== null) {
      cancelAnimationFrame(positionRaf);
      positionRaf = null;
    }
    if (ro) ro.disconnect();
  });

  return { browserHost, schedulePosition };
}
