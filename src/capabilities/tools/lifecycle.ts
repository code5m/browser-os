import { createManagedHotPlugLifecycle } from "../../capability/platform/hot-plug-lifecycle"

const lifecycle = createManagedHotPlugLifecycle()

export function toolsLifecycleSnapshot() {
  const snapshot = lifecycle.snapshot()
  return { active: snapshot.active, cleanupCount: snapshot.bindingCount }
}
export const activateToolsLifecycle = lifecycle.activate
export const suspendToolsLifecycle = lifecycle.suspend
export function registerToolsCleanup(cleanup: () => void | Promise<void>): () => void {
  return lifecycle.register({ cleanup })
}
