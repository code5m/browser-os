type GridLifecycleBinding = {
  resume?: () => void | Promise<void>
  suspend: () => void | Promise<void>
  deactivate: () => void | Promise<void>
}

let active = false
const bindings = new Set<GridLifecycleBinding>()

export function gridLifecycleSnapshot() {
  return { active, bindingCount: bindings.size }
}

export function registerGridLifecycleBinding(binding: GridLifecycleBinding): () => void {
  bindings.add(binding)
  if (active) void binding.resume?.()
  return () => bindings.delete(binding)
}

export function activateGridLifecycle(): void {
  active = true
  for (const binding of bindings) void binding.resume?.()
}

export async function suspendGridLifecycle(): Promise<void> {
  const results = await Promise.allSettled([...bindings].map((binding) => binding.suspend()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
  active = false
}

export async function deactivateGridLifecycle(): Promise<void> {
  const results = await Promise.allSettled([...bindings].map((binding) => binding.deactivate()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
  active = false
}
