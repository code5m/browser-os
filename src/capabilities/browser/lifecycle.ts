import { createManagedHotPlugLifecycle, type HotPlugBinding } from "../../capability/platform/hot-plug-lifecycle"

type BrowserLifecycleBinding =
  Required<Pick<HotPlugBinding, "suspend" | "deactivate">> &
  Pick<HotPlugBinding, "resume">

const lifecycle = createManagedHotPlugLifecycle()

export const browserLifecycleSnapshot = lifecycle.snapshot
export const activateBrowserLifecycle = lifecycle.activate
export const suspendBrowserLifecycle = lifecycle.suspend
export const deactivateBrowserLifecycle = lifecycle.deactivate
export function registerBrowserLifecycleBinding(binding: BrowserLifecycleBinding): () => void {
  return lifecycle.register(binding)
}
