type TerminalLifecycleBinding = {
  canSuspend: () => boolean
  cleanup?: () => void | Promise<void>
}

let active = false
const bindings = new Set<TerminalLifecycleBinding>()

export function terminalLifecycleSnapshot() {
  return { active, bindingCount: bindings.size }
}

export function registerTerminalLifecycleBinding(binding: TerminalLifecycleBinding): () => void {
  bindings.add(binding)
  return () => bindings.delete(binding)
}

export function activateTerminalLifecycle(): void {
  active = true
}

export async function suspendTerminalLifecycle(): Promise<void> {
  if ([...bindings].some((binding) => !binding.canSuspend())) {
    throw new Error("terminal has active PTY sessions")
  }
  active = false
  const results = await Promise.allSettled([...bindings].map((binding) => binding.cleanup?.()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
}
