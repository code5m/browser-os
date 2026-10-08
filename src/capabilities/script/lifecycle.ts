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
  for (const [runId, probe] of runs) {
    let stopped = false
    try {
      stopped = !(await probe())
    } catch {}
    if (!stopped) throw new Error("script runs still active: " + runId)
    runs.delete(runId)
  }
  active = false
}
