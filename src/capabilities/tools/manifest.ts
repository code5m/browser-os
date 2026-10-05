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
  description: "内置/用户工具枚举与打开。工具在独立子 webview（tool:// 协议）中运行，零能力隔离；种子离线零外链。",
  limitationReason: "HP0(STATIC)：工具子 webview 由 Rust 侧按需创建/聚焦，前端无独立生命周期可装卸；未验证 absent 无残留前不宣称 HP1。",
})
