export type HotPlugBinding = {
  canSuspend?: () => boolean
  resume?: () => void | Promise<void>
  suspend?: () => void | Promise<void>
  deactivate?: () => void | Promise<void>
  cleanup?: () => void | Promise<void>
}

export function createManagedHotPlugLifecycle(blockedMessage = "capability resource is busy") {
  let active = false
  const bindings = new Set<HotPlugBinding>()

  const snapshot = () => ({ active, bindingCount: bindings.size })
  const register = (binding: HotPlugBinding): (() => void) => {
    bindings.add(binding)
    if (active) void binding.resume?.()
    return () => bindings.delete(binding)
  }
  const activate = (): void => {
    active = true
    for (const binding of bindings) void binding.resume?.()
  }
  const transition = async (deactivate: boolean): Promise<void> => {
    for (const binding of bindings) {
      if (binding.canSuspend && !binding.canSuspend()) throw new Error(blockedMessage)
    }
    for (const binding of bindings) {
      const op = deactivate
        ? binding.deactivate ?? binding.cleanup ?? binding.suspend
        : binding.suspend ?? binding.cleanup
      await op?.()
    }
    // Runtime remains ACTIVE if any hook throws, so only commit inactive after all cleanup succeeds.
    active = false
  }

  return {
    snapshot,
    register,
    activate,
    suspend: () => transition(false),
    deactivate: () => transition(true),
  }
}
