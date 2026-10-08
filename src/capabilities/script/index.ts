import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateScriptLifecycle, suspendScriptLifecycle } from "./lifecycle"
import { scriptManifest } from "./manifest"

export const SCRIPT_CAPABILITY_ID = "script"

export const scriptCapability = createLifecycleContributionCapability(scriptManifest, [
  { id: "script.main.panel", type: "surface", slot: "workbench-main", view: "scripts", load: () => import("./ui/ScriptPanel.vue") },
  { id: "script.commands.panel", type: "surface", slot: "workbench-main", view: "commands", load: () => import("./ui/CommandSnippetPanel.vue") },
], activateScriptLifecycle, suspendScriptLifecycle).capability
