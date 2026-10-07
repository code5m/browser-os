import { withLazyContributions } from "../../capability/platform/contributed"
import { scriptManifest } from "./manifest"

export const SCRIPT_CAPABILITY_ID = "script"
export const scriptCapability = withLazyContributions(scriptManifest, [
  { id: "script.main.panel", type: "surface", slot: "workbench-main", view: "scripts", load: () => import("./ui/ScriptPanel.vue") },
  { id: "script.commands.panel", type: "surface", slot: "workbench-main", view: "commands", load: () => import("./ui/CommandSnippetPanel.vue") },
])
