export const NATIVE_MAIN_VIEWS = new Set(["browser", "grid", "term"])

export function resolveAvailableWorkbenchView(
  current: string,
  availableViews: string[],
  preferred = "home",
): string {
  if (NATIVE_MAIN_VIEWS.has(current)) return current
  // Capability contributions may register asynchronously after the first Vue
  // paint. Keep the explicitly selected preferred view stable while its
  // contribution is still being registered; otherwise a transient list that
  // only contains a service view (for example settings) permanently hijacks
  // cold startup away from Home.
  if (current === preferred) return current
  if (availableViews.includes(current)) return current
  if (availableViews.includes(preferred)) return preferred
  return availableViews[0] ?? current
}
