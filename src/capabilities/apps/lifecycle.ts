let active = false
let generation = 0

export function appsLifecycleSnapshot() {
  return { active, generation }
}

export function activateAppsLifecycle(): void {
  active = true
  generation += 1
}

export function suspendAppsLifecycle(): void {
  active = false
  generation += 1
}
