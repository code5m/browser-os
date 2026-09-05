// M3.c（WBS M3-4 · E2）：终端 resize 静默窗口。
//
// 契约来源 `logs/assist/M3-4.b-prework-20260902-1055.md` §4.2 三条硬规则：
//   1. **去重**：行列没变 → 完全不下发（消除 `ResizeObserver` 抖动带来的无效 IPC）。
//   2. **静默窗口**：连续 resize 只在停手后下发一次（`quietMs`）。
//   3. **硬上界**：距上次下发超过 `maxWaitMs` 时立即下发，避免"用户持续慢拖时
//      终端一直不重排"这一纯静默窗口方案的经典缺陷。
//
// 边界：本模块只决定**何时**下发，不负责错误处理；调用方对 `term_resize` 一律
// `.catch(() => {})`，高频路径不得弹窗/吐司（会刷屏）。
//
// 时间源与定时器可注入（`now` / `schedule` / `cancel`），便于 headless 单测用假时钟
// 复现三条硬规则，无需真实等待。

/// 静默窗口：连续无新尺寸事件超过该时长才真正下发（fileterm `TERMINAL_RESIZE_SETTLE_MS=140`）。
export const TERM_RESIZE_QUIET_MS = 140;
/// 硬上界：距上次下发超过该时长，无论是否仍在拖动都下发一次。
export const TERM_RESIZE_MAX_WAIT_MS = 500;

const TERM_RESIZE_MAX_COLS = 1000;
const TERM_RESIZE_MAX_ROWS = 1000;

export interface TerminalResizeDims {
  cols: number;
  rows: number;
}

export interface TerminalResizeOptions {
  quietMs?: number;
  maxWaitMs?: number;
  /** 时间源（测试注入假时钟）。 */
  now?: () => number;
  schedule?: (fn: () => void, ms: number) => number;
  cancel?: (handle: number) => void;
}

export function useTerminalResize(
  onCommit: (cols: number, rows: number) => void,
  opts: TerminalResizeOptions = {},
) {
  const quietMs = opts.quietMs ?? TERM_RESIZE_QUIET_MS;
  const maxWaitMs = opts.maxWaitMs ?? TERM_RESIZE_MAX_WAIT_MS;
  const now = opts.now ?? (() => Date.now());
  const schedule =
    opts.schedule ??
    ((fn: () => void, ms: number) => window.setTimeout(fn, ms) as unknown as number);
  const cancel = opts.cancel ?? ((handle: number) => window.clearTimeout(handle));

  let timer: number | null = null;
  let pending: TerminalResizeDims | null = null;
  let lastCols = -1;
  let lastRows = -1;
  let lastCommitAt = 0;

  function commit(): void {
    timer = null;
    const next = pending;
    pending = null;
    if (!next) return;
    lastCols = next.cols;
    lastRows = next.rows;
    lastCommitAt = now();
    onCommit(next.cols, next.rows);
  }

  /** 收到一次尺寸变化。尺寸未变 → 零排程；否则按静默窗口 + 硬上界计算等待时长。 */
  function notify(cols: number, rows: number): void {
    if (!Number.isFinite(cols) || !Number.isFinite(rows)) return;
    const c = Math.max(1, Math.min(TERM_RESIZE_MAX_COLS, Math.floor(cols)));
    const r = Math.max(1, Math.min(TERM_RESIZE_MAX_ROWS, Math.floor(rows)));
    // 规则 1：去重（不排程、不回调，因此也不产生 IPC）。
    if (c === lastCols && r === lastRows) return;
    pending = { cols: c, rows: r };
    if (timer !== null) cancel(timer);
    const elapsed = now() - lastCommitAt;
    // 规则 2 + 规则 3：静默窗口内推后，但绝不超过硬上界。
    const wait = elapsed >= maxWaitMs ? 0 : Math.min(quietMs, maxWaitMs - elapsed);
    timer = schedule(commit, wait);
  }

  /** 新会话（首次启动 / ↻ 重启）：清去重态，保证新 PTY 一定能收到首个 resize。 */
  function reset(): void {
    if (timer !== null) cancel(timer);
    timer = null;
    pending = null;
    lastCols = -1;
    lastRows = -1;
    lastCommitAt = 0;
  }

  /** 组件卸载：清掉在途定时器，不留回调（用例 N5）。 */
  function dispose(): void {
    reset();
  }

  return { notify, reset, dispose };
}
