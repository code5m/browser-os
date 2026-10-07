type ScriptRunProbe = () => boolean | Promise<boolean>

let active = false
const runs = new Map<string, ScriptRunProbe>()

export function scriptLifecycleSnapshot() {
  return { active, trackedRuns: runs.size }
}

export function trackScriptRun(runId: string, probe: ScriptRunProbe): void {
  runs.set(runId, probe)
}

export function completeScriptRun(runId: string): void {
  runs.delete(runId)
}

export function activateScriptLifecycle(): void {
  active = true
}

export async function suspendScriptLifecycle(): Promise<void> {
  const activeRuns: string[] = []
  for (const [runId, probe] of [...runs]) {
    let running = true
    try {
      running = await probe()
    } catch {
      running = true
    }
    if (running) activeRuns.push(runId)
    else runs.delete(runId)
  }
  if (activeRuns.length) throw new Error("script runs still active: " + activeRuns.join(","))
  active = false
}
