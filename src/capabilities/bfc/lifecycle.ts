let active = false

export function activateBfcLifecycle(): void {
  active = true
}

export function suspendBfcLifecycle(): void {
  active = false
}

export function bfcLifecycleSnapshot() {
  return { active }
}
