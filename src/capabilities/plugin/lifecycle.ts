type PluginLifecycleBinding = {
  canSuspend?: () => boolean
  cleanup?: () => void | Promise<void>
}

let active = false
const bindings = new Set<PluginLifecycleBinding>()

export function pluginLifecycleSnapshot() {
  return { active, bindingCount: bindings.size }
}

export function registerPluginLifecycleBinding(binding: PluginLifecycleBinding): () => void {
  bindings.add(binding)
  return () => bindings.delete(binding)
}

export function activatePluginLifecycle(): void {
  active = true
}

export async function suspendPluginLifecycle(): Promise<void> {
  if ([...bindings].some((binding) => binding.canSuspend && !binding.canSuspend())) {
    throw new Error("plugin operation is still in progress")
  }
  active = false
  const results = await Promise.allSettled([...bindings].map((binding) => binding.cleanup?.()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
}
