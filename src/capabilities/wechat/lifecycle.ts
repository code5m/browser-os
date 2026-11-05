let active = false

export function activateWechatLifecycle(): void {
  active = true
}

export function suspendWechatLifecycle(): void {
  active = false
}

export function wechatLifecycleSnapshot() {
  return { active }
}
