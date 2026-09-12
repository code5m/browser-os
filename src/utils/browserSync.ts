// 浏览器同步防御逻辑的闭包工厂集合（M6 Reduced Scope · S3）。
//
// 把 src/composables/useBrowserHost.ts 里手写的同步防御提取成可测的闭包工厂。
// 纯 TS：零 Vue、零 DOM、零 store、零 bridge，可在 Node 下直接加载测试。
//
// 行号映射（Wave 1 时点，207 行版）：
//   createKeyDeduper        L8-9, L53-57               tabPosition 50ms 同 key 去重
//   createGridSendCache     L12-13, L126-130, L153-160 宫格按 index 签名缓存 + session 失效
//   createBoundedRetrier    L39-45（补上限）, L109-122  有上限重试
//   createHiddenIntent      L14, L129, L171-174        同会话同页签只移出一次
//   createInvalidationGroup L21                        mainView 变化清空全部缓存
//
// 硬约束：
// 1. 不拥有 UI 状态：不 import store、不读 DOM、不调 bridge。时钟与调度器全部注入。
// 2. 不改 backend semantics：只决定"发不发"，绝不改写下发 payload；
//    去重键 / 签名格式与原实现逐字一致。
// 3. PROJECT-RULES 3.5【锁定】：隐藏 = 只把 x 移到 -30000 且保持尺寸。
//    本文件只产出"是否要发隐藏 intent"的布尔，绝不产生 webview.hide() 或 1x1 尺寸。
// 4. 与 native 400ms 布局守护线程共存：不与之竞争，不做"暂停定位"旁路。

/** 只需宽高即可判定是否可下发的最小矩形形状（不绑定 DOMRect） */
export type SyncRect = { width: number; height: number };

/** 原 L55 的 50ms 去重窗口 */
export const DEFAULT_DEDUPE_WINDOW_MS = 50;

/** 原 L120 的 10 次重试；rect=0 分支（原 L39-45 无上限）沿用同一保守值 */
export const DEFAULT_MAX_RETRIES = 10;

/** 原 L120 的 100ms 重试间隔 */
export const DEFAULT_RETRY_DELAY_MS = 100;

/**
 * PROJECT-RULES 3.5【锁定】：隐藏 = 只移 x 到 -30000，尺寸保持不变。
 * 该坐标由 bridge.rs 的 hide_bounds 下发（并 remember_layout），
 * 前端不得自行构造 —— 此处导出仅供文档与测试断言使用。
 * 禁止：webview.hide()（主线程死锁）；尺寸缩到 1x1（重排死锁）。
 */
export const HIDDEN_OFFSCREEN_X = -30000;

type Now = () => number;
type Scheduler = (fn: () => void, delayMs: number) => void;

function defaultNow(): number {
  return typeof performance !== "undefined" && typeof performance.now === "function"
    ? performance.now()
    : Date.now();
}

function defaultSchedule(fn: () => void, delayMs: number): void {
  setTimeout(fn, delayMs);
}

/** 原 L39 / L119 的 `!r.width || !r.height` 判定（NaN 同样视为不可用） */
export function hasNonZeroSize(r: SyncRect | null | undefined): boolean {
  return !!r && r.width > 0 && r.height > 0;
}

/** 原 L53：`${id}:${x},${y},${w},${h}` —— 格式不可改 */
export function tabPositionKey(
  tabId: string,
  x: number,
  y: number,
  w: number,
  h: number,
): string {
  return `${tabId}:${x},${y},${w},${h}`;
}

/** 原 L153：`${x},${y},${w}x${h}` —— 格式不可改 */
export function gridPositionSignature(
  x: number,
  y: number,
  w: number,
  h: number,
): string {
  return `${x},${y},${w}x${h}`;
}

// ── 1) deduper：50ms 同 key 去重（原 L8-9 / L53-57）──────────────────────────
export interface KeyDeduper {
  /** key 与上次相同且距上次 < windowMs → false（丢弃）；否则记账并 true（下发） */
  shouldSend(key: string): boolean;
  /** 作废缓存（原 L21 的 `lastKey = ''`） */
  reset(): void;
  readonly lastKey: string;
  readonly lastAt: number;
}

export function createKeyDeduper(
  options: { windowMs?: number; now?: Now } = {},
): KeyDeduper {
  const windowMs = options.windowMs ?? DEFAULT_DEDUPE_WINDOW_MS;
  const now = options.now ?? defaultNow;
  let lastKey = "";
  let lastAt = -Infinity;
  return {
    shouldSend(key: string): boolean {
      const t = now();
      if (key === lastKey && t - lastAt < windowMs) return false;
      lastKey = key;
      lastAt = t;
      return true;
    },
    reset(): void {
      lastKey = "";
      lastAt = -Infinity;
    },
    get lastKey() {
      return lastKey;
    },
    get lastAt() {
      return lastAt;
    },
  };
}

// ── 2) grid send cache：按 index 的签名缓存 + session 失效（原 L12-13/126-130/153-160）
export interface GridSendCache {
  /** session 变化即整体失效（原 L126-130）。返回是否发生了失效 */
  syncSession(session: number): boolean;
  /** sig 与上次相同 → false；否则记账并 true */
  shouldSend(index: number, sig: string): boolean;
  /** 下发失败时回滚该格缓存，保证下轮重发（原 L156-158） */
  invalidate(index: number): void;
  reset(): void;
  readonly session: number;
}

export function createGridSendCache(): GridSendCache {
  let session = -1;
  const sent = new Map<number, string>();
  return {
    syncSession(next: number): boolean {
      if (next === session) return false;
      session = next;
      sent.clear();
      return true;
    },
    shouldSend(index: number, sig: string): boolean {
      if (sent.get(index) === sig) return false;
      sent.set(index, sig);
      return true;
    },
    invalidate(index: number): void {
      sent.delete(index);
    },
    reset(): void {
      sent.clear();
    },
    get session() {
      return session;
    },
  };
}

// ── 3) bounded retrier：有上限重试（原 L39-45 无上限 → 补上限；L109-122）────────
export interface BoundedRetrier {
  /** 是否还应继续重试（retry 为已重试次数，从 0 起） */
  shouldRetry(retry: number): boolean;
  /** 安排下一次重试；超过上限则不安排并返回 false */
  scheduleRetry(retry: number, fn: () => void): boolean;
  reset(): void;
  readonly maxRetries: number;
  /** 上次是否已耗尽重试预算（供日志/测试断言） */
  readonly exhausted: boolean;
}

export function createBoundedRetrier(
  options: { maxRetries?: number; delayMs?: number; schedule?: Scheduler } = {},
): BoundedRetrier {
  const maxRetries = options.maxRetries ?? DEFAULT_MAX_RETRIES;
  const delayMs = options.delayMs ?? DEFAULT_RETRY_DELAY_MS;
  const schedule = options.schedule ?? defaultSchedule;
  let exhausted = false;
  return {
    shouldRetry(retry: number): boolean {
      return retry < maxRetries;
    },
    scheduleRetry(retry: number, fn: () => void): boolean {
      if (retry >= maxRetries) {
        exhausted = true;
        return false;
      }
      schedule(fn, delayMs);
      return true;
    },
    reset(): void {
      exhausted = false;
    },
    get maxRetries() {
      return maxRetries;
    },
    get exhausted() {
      return exhausted;
    },
  };
}

// ── 4) hidden intent：同会话同页签只移出一次（原 L14 / L129 / L171-174）────────
export interface HiddenIntent {
  /** 该页签是否已移出过；未移过则记账并 true */
  shouldHide(tabId: string): boolean;
  reset(): void;
  readonly lastHiddenTab: string;
}

export function createHiddenIntent(): HiddenIntent {
  let lastHiddenTab = "";
  return {
    shouldHide(tabId: string): boolean {
      if (!tabId) return false;
      if (lastHiddenTab === tabId) return false;
      lastHiddenTab = tabId;
      return true;
    },
    reset(): void {
      lastHiddenTab = "";
    },
    get lastHiddenTab() {
      return lastHiddenTab;
    },
  };
}

// ── 5) invalidation group：一处失效，全部作废（原 L21）────────────────────────
/**
 * 原实现里 mainView 变化要手动清 3 个变量（lastGridSent / lastHiddenTab / lastKey），
 * 漏一个即 bug。这里把失效动作集中注册，避免"缓存与失效分离"。
 */
export interface InvalidationGroup {
  register(fn: () => void): void;
  /** 触发全部失效 */
  invalidateAll(): void;
}

export function createInvalidationGroup(): InvalidationGroup {
  const handlers: Array<() => void> = [];
  return {
    register(fn: () => void): void {
      handlers.push(fn);
    },
    invalidateAll(): void {
      for (const h of handlers) h();
    },
  };
}
