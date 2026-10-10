// Shared, side-effect-free relative timestamp formatter.
// UI contract (unchanged): future → 后, past → 前, invalid → —.
// Keep time rounding, padding and units aligned with existing scheduler UI.

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/// 相对时间。未来 → “x 后”；过去 → “x 前”。非法时间返回 "—"。
export function formatRelative(iso: string | null | undefined, nowMs: number): string {
  if (!iso) return "—";
  const t = Date.parse(iso);
  if (!Number.isFinite(t)) return "—";
  const diff = Math.trunc((t - nowMs) / 1000);
  const abs = Math.abs(diff);
  const suffix = diff >= 0 ? "后" : "前";
  if (abs < 60) return `${abs} 秒${suffix}`;
  if (abs < 3600) return `${Math.floor(abs / 60)} 分${pad(abs % 60)} 秒${suffix}`;
  if (abs < 86400) return `${Math.floor(abs / 3600)} 小时${pad(Math.floor((abs % 3600) / 60))} 分${suffix}`;
  return `${Math.floor(abs / 86400)} 天${pad(Math.floor((abs % 86400) / 3600))} 小时${suffix}`;
}
