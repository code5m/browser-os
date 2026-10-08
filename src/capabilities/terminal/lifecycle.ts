import { createManagedHotPlugLifecycle, type HotPlugBinding } from "../../capability/platform/hot-plug-lifecycle"

type TerminalLifecycleBinding =
  Required<Pick<HotPlugBinding, "canSuspend">> &
  Pick<HotPlugBinding, "cleanup">

const lifecycle = createManagedHotPlugLifecycle("terminal has active PTY sessions")

export const terminalLifecycleSnapshot = lifecycle.snapshot
export const activateTerminalLifecycle = lifecycle.activate
export const suspendTerminalLifecycle = lifecycle.suspend
export function registerTerminalLifecycleBinding(binding: TerminalLifecycleBinding): () => void {
  return lifecycle.register(binding)
}
