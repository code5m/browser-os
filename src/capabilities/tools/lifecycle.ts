type ToolsCleanup = () => void | Promise<void>

let active = false
const cleanups = new Set<ToolsCleanup>()

export function toolsLifecycleSnapshot() {
  return { active, cleanupCount: cleanups.size }
}

export function registerToolsCleanup(cleanup: ToolsCleanup): () => void {
  cleanups.add(cleanup)
  return () => cleanups.delete(cleanup)
}

export function activateToolsLifecycle(): void {
  active = true
}

export async function suspendToolsLifecycle(): Promise<void> {
  active = false
  const results = await Promise.allSettled([...cleanups].map((cleanup) => cleanup()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
}
