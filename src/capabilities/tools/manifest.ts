import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const toolsManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "tools",
  name: "工具箱",
  semanticOwner: "useToolsStore",
  maturity: "C2",
  version: "1.0.0",
  maturityEvidence: ["scripts/check-tools-policy.py", "scripts/check-seed-tools.py"],
  provides: ["tools.list", "tools.open"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["MEDIUM", "WEBVIEW"],
  suspendable: true,
  destroyable: true,
  persistence: { scope: "runtime_only", sensitive: false },
  contributions: [{ id: "tools.main.panel", slot: "workbench-main", type: "surface", view: "tools" }],
  manifestResources: [{ kind: "WEBVIEW", ownership: "owned", evidence: "src-tauri/src/tools.rs:open_tool (WebviewWindowBuilder, label=tool-<id>)" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/tools/public.ts" }],
  deactivationPolicy: "graceful",
  description: "工具枚举与独立 WebView 打开。",
  limitationReason: "HP0：工具 WebView 尚无可逆装卸契约。",
})
