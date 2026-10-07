import { withLazyContributions } from "../../capability/platform/contributed"
import { scriptManifest } from "./manifest"
import { activateScriptLifecycle, suspendScriptLifecycle } from "./lifecycle"

export const SCRIPT_CAPABILITY_ID = "script"
const contributedScriptCapability = withLazyContributions(scriptManifest, [
  { id: "script.main.panel", type: "surface", slot: "workbench-main", view: "scripts", load: () => import("./ui/ScriptPanel.vue") },
  { id: "script.commands.panel", type: "surface", slot: "workbench-main", view: "commands", load: () => import("./ui/CommandSnippetPanel.vue") },
])

export const scriptCapability = {
  ...contributedScriptCapability,
  lifecycle: {
    ...contributedScriptCapability.lifecycle,
    onActivate: () => {
      activateScriptLifecycle()
      contributedScriptCapability.lifecycle.onActivate?.()
    },
    onSuspend: suspendScriptLifecycle,
    onDeactivate: suspendScriptLifecycle,
  },
}
