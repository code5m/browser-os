import { createManagedHotPlugLifecycle, type HotPlugBinding } from "../../capability/platform/hot-plug-lifecycle"

const lifecycle = createManagedHotPlugLifecycle("git write operation is still running")

export const gitLifecycleSnapshot = lifecycle.snapshot
export const activateGitLifecycle = lifecycle.activate
export const suspendGitLifecycle = lifecycle.suspend
export function registerGitLifecycleBinding(binding: Required<Pick<HotPlugBinding, "canSuspend" | "cleanup">>): () => void {
  return lifecycle.register(binding)
}
