export { useScriptStore } from "./state/useScriptStore"
export { useSnippetStore } from "./state/useSnippetStore"

// Public script capability contract for trusted references. Native execution
// ownership remains here; other capabilities never call script IPC directly.
import { bridge } from "../../bridge";
import type { CommandSnippet, RunSnapshot } from "../../types";
import { isCapabilityActive } from "../../capability/runtimeSingleton";
import { trackScriptRun, completeScriptRun } from "./lifecycle";
function requireCommandCapability(){if(!isCapabilityActive('script'))throw new Error('SCRIPT_CAPABILITY_INACTIVE');}
// Controlled workbench adapter: existing snippet execution, never arbitrary shell text.
export const commandWorkbench = {
  list: (): Promise<CommandSnippet[]> => { requireCommandCapability(); return bridge.snippetList(); },
  run: async (id: string, values: Record<string,string>): Promise<RunSnapshot> => {
    requireCommandCapability();
    const run=await bridge.runCommand(id, values);
    if(run.status==='running'){
      trackScriptRun(run.run_id,async()=> (await bridge.scriptStatus(run.run_id)).status==='running');
      if(!isCapabilityActive('script'))await bridge.cancelScript(run.run_id);
    }
    return run;
  },
  status: async (id: string): Promise<RunSnapshot> => { const run=await bridge.scriptStatus(id);if(run.status!=='running')completeScriptRun(id);return run; },
  cancel: (id: string) => bridge.cancelScript(id),
};
export async function runExistingScriptRef(id: string): Promise<string> {
  const run = await bridge.runScript(id, {});
  return run.run_id;
}
export async function runExistingCommandRef(id: string): Promise<string> {
  const run = await bridge.runCommand(id, {});
  return run.run_id;
}
