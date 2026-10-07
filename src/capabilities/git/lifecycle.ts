type GitLifecycleBinding = {
  canSuspend: () => boolean
  cleanup: () => void | Promise<void>
}

let active = false
const bindings = new Set<GitLifecycleBinding>()

export function gitLifecycleSnapshot() {
  return { active, bindingCount: bindings.size }
}

export function registerGitLifecycleBinding(binding: GitLifecycleBinding): () => void {
  bindings.add(binding)
  return () => bindings.delete(binding)
}

export function activateGitLifecycle(): void {
  active = true
}

export async function suspendGitLifecycle(): Promise<void> {
  if ([...bindings].some((binding) => !binding.canSuspend())) {
    throw new Error("git write operation is still running")
  }
  active = false
  const results = await Promise.allSettled([...bindings].map((binding) => binding.cleanup()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
}
