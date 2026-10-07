type BrowserLifecycleBinding = {
  resume?: () => void | Promise<void>
  suspend: () => void | Promise<void>
  deactivate: () => void | Promise<void>
}

let active = false
const bindings = new Set<BrowserLifecycleBinding>()

export function browserLifecycleSnapshot() {
  return { active, bindingCount: bindings.size }
}

export function registerBrowserLifecycleBinding(binding: BrowserLifecycleBinding): () => void {
  bindings.add(binding)
  if (active) void binding.resume?.()
  return () => bindings.delete(binding)
}

export function activateBrowserLifecycle(): void {
  active = true
  for (const binding of bindings) void binding.resume?.()
}

export async function suspendBrowserLifecycle(): Promise<void> {
  const results = await Promise.allSettled([...bindings].map((binding) => binding.suspend()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
  active = false
}

export async function deactivateBrowserLifecycle(): Promise<void> {
  const results = await Promise.allSettled([...bindings].map((binding) => binding.deactivate()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
  active = false
}
