// Graph HP2 lifecycle gate.
// Runtime hooks operate before Pinia exists, so lifecycle ownership must stay store-agnostic.
type Canceller = () => void

let active = true
let generation = 0
const cancellers = new Set<Canceller>()

export function graphLifecycleSnapshot() {
  return { active, generation }
}

export function registerGraphCanceller(cancel: Canceller): () => void {
  cancellers.add(cancel)
  return () => cancellers.delete(cancel)
}

export function resumeGraphLifecycle(): void {
  active = true
  generation += 1
}

export function suspendGraphLifecycle(): void {
  active = false
  generation += 1
  for (const cancel of [...cancellers]) cancel()
}
