type DatabaseCleanup = () => void | Promise<void>

let active = false
let generation = 0
const cleanups = new Set<DatabaseCleanup>()

export function databaseLifecycleSnapshot() {
  return { active, generation, cleanupCount: cleanups.size }
}

export function registerDatabaseCleanup(cleanup: DatabaseCleanup): () => void {
  cleanups.add(cleanup)
  return () => cleanups.delete(cleanup)
}

export function activateDatabaseLifecycle(): void {
  active = true
  generation += 1
}

export async function suspendDatabaseLifecycle(): Promise<void> {
  active = false
  generation += 1
  const results = await Promise.allSettled([...cleanups].map((cleanup) => cleanup()))
  const rejected = results.find((result): result is PromiseRejectedResult => result.status === "rejected")
  if (rejected) throw rejected.reason
}
