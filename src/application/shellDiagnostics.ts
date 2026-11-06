import { bridge } from "../bridge";

// Only low-cardinality event codes and counters may cross this diagnostics
// boundary; URL contents, page bodies and credentials are never accepted.
export function recordShellHeartbeat(lagMs: number, view: string): void {
  bridge.debugLog(`[health] shell-pulse unix_ms=${Date.now()} lag_ms=${lagMs} view=${view}`);
}

export function recordNavigationApplied(tabId: string, seq: number, urlBytes: number): void {
  bridge.debugLog(`[navigation] frontend_applied seq=${seq} tab=${tabId} url_bytes=${urlBytes}`);
}
