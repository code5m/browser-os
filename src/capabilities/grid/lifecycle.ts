import { createManagedHotPlugLifecycle, type HotPlugBinding } from "../../capability/platform/hot-plug-lifecycle"

type GridLifecycleBinding =
  Required<Pick<HotPlugBinding, "suspend" | "deactivate">> &
  Pick<HotPlugBinding, "resume">

const lifecycle = createManagedHotPlugLifecycle()

export const gridLifecycleSnapshot = lifecycle.snapshot
export const activateGridLifecycle = lifecycle.activate
export const suspendGridLifecycle = lifecycle.suspend
export const deactivateGridLifecycle = lifecycle.deactivate
export function registerGridLifecycleBinding(binding: GridLifecycleBinding): () => void {
  return lifecycle.register(binding)
}
