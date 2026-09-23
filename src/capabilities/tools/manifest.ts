import type { CapabilityDefinition } from "../../capability/types"

// Tools（工具箱）能力 Manifest（Capability Library Expansion v1 — STAGE H）
// 语义 owner = useToolsStore（STAGE H **新建**：原 ToolBox.vue 把状态放在组件内，无 owner）。
// 物理：ToolBox 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='tools'）贡献给 MainArea。
// 后端 list_tools（内置种子 + 扫描 workspace/tools/*.html）与 open_tool（**独立子 webview**，
// label=`tool-<id>`，`tool://` 协议；路径越权防御 validate_user_tool_path；MAX_USER_TOOL_BYTES 2 MiB）已实现。
// 安全：工具窗口零能力隔离——不属任何 capability，工具 HTML 即便 invoke() 亦被 ACL 拒绝
// （门禁 TOOL_CAPABILITY_LEAK / SEED_CAPABILITY_LEAK）；种子离线、零外链。
//
// 成熟度：C2 ISOLATED。非 C3：无 absence 门禁 + mainView='tools' nav 硬编码 + 工具子 webview 归 workspace 工具目录。
export const toolsManifest: CapabilityDefinition = {
  id: "tools",
  name: "工具箱",
  category: "CAPABILITY",
  provides: ["tools.list", "tools.open"],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["MEDIUM", "WEBVIEW"],
    suspendable: true,
    destroyable: true,
  },
  permissions: [],
  persistence: {
    scope: "runtime_only",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useToolsStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  v1: {
    id: "tools",
    version: "1.0.0",
    displayName: "工具箱",
    description: "内置/用户工具枚举与打开。工具在独立子 webview（tool:// 协议）中运行，零能力隔离；种子离线零外链。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-tools-policy.py", "scripts/check-seed-tools.py"],
    dependencies: ["bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["tools.list", "tools.open"],
    requires: [],
    contributions: [{ id: "tools.main.panel", slot: "workbench-main", type: "surface", view: "tools" }],
    permissions: [],
    resources: [
      { kind: "WEBVIEW", ownership: "owned", evidence: "src-tauri/src/tools.rs:open_tool (WebviewWindowBuilder, label=tool-<id>)" },
    ],
    persistenceScope: "runtime_only",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "graceful",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP0",
      enable: false,
      disable: false,
      register: false,
      unregister: false,
      install: false,
      uninstall: false,
      limitationReason:
        "HP0(STATIC)：工具子 webview 由 Rust 侧按需创建/聚焦，前端无独立生命周期可装卸；未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/tools/public.ts" }],
    entrypoint: "src/capabilities/tools/index.ts",
    semanticOwner: "useToolsStore",
  },
}
