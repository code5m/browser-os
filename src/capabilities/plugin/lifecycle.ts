import { createManagedHotPlugLifecycle, type HotPlugBinding } from "../../capability/platform/hot-plug-lifecycle"

const lifecycle = createManagedHotPlugLifecycle("plugin operation is still in progress")

export const pluginLifecycleSnapshot = lifecycle.snapshot
export const activatePluginLifecycle = lifecycle.activate
export const suspendPluginLifecycle = lifecycle.suspend
export function registerPluginLifecycleBinding(binding: Pick<HotPlugBinding, "canSuspend" | "cleanup">): () => void {
  return lifecycle.register(binding)
}
