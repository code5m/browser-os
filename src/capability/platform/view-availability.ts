export const NATIVE_MAIN_VIEWS = new Set(["browser", "grid", "term"])

export function resolveAvailableWorkbenchView(
  current: string,
  availableViews: string[],
  preferred = "home",
): string {
  if (NATIVE_MAIN_VIEWS.has(current)) return current
  if (availableViews.includes(current)) return current
  if (availableViews.includes(preferred)) return preferred
  return availableViews[0] ?? current
}
