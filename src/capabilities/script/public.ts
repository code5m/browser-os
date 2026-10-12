export { useScriptStore } from "./state/useScriptStore"
export { useSnippetStore } from "./state/useSnippetStore"

// Public script capability contract for trusted references. Native execution
// ownership remains here; other capabilities never call script IPC directly.
import { bridge } from "../../bridge";
export async function runExistingScriptRef(id: string): Promise<string> {
  const run = await bridge.runScript(id, {});
  return run.run_id;
}
export async function runExistingCommandRef(id: string): Promise<string> {
  const run = await bridge.runCommand(id, {});
  return run.run_id;
}
