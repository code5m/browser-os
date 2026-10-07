type ClipboardLifecycleBinding = {
  start(): void | Promise<void>
  stop(): void | Promise<void>
}

let active = false
const bindings = new Set<ClipboardLifecycleBinding>()

export function clipboardLifecycleSnapshot() {
  return { active, bindingCount: bindings.size }
}

export function registerClipboardLifecycleBinding(binding: ClipboardLifecycleBinding): () => void {
  bindings.add(binding)
  if (active) void binding.start()
  return () => {
    bindings.delete(binding)
    void binding.stop()
  }
}

export function activateClipboardLifecycle(): void {
  if (active) return
  active = true
  for (const binding of bindings) void binding.start()
}

export function suspendClipboardLifecycle(): void {
  if (!active) return
  active = false
  for (const binding of bindings) void binding.stop()
}
