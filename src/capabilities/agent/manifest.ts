import type { CapabilityDefinition } from "../../capability/types"

// Agent 能力 Manifest（Capability Library Expansion v1 — 从 Workspace 内嵌面板升格为独立 Capability）
// 语义 owner = useAgentStore（Semantic Registry 已冻结；同时持有 skill 状态，STAGE E 处理 skill 拆分）。
// 物理：AgentManagerPanel(+AgentChatPanel 子视图) 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽
// （view='agents'）贡献给 MainArea；MainArea 只按槽渲染、不 import 本能力内部（C3 关键）。
//
// 编码工作区经 framework coding_service 调用有界文件工具与模型循环；
// 旧蓝图 agent_chat / agent_run / agent_install 契约仍未实现，不能混淆两条入口。
// 当前未验证运行中能力暂停/销毁及完整热插拔，不提升成熟度或 HP 等级。
export const agentManifest: CapabilityDefinition = {
  id: "agent",
  name: "智能体",
  category: "CAPABILITY",
  provides: [
    "agent.chat",
    "agent.run",
    "agent.memory",
  ],
  dependsOn: ["bridge"],
  // Graph nodes may reference Agent ids as data; this is not a runtime dependency.
  optionalDependencies: ["script", "git", "database", "browser", "terminal"],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["MEDIUM", "NETWORK", "PROCESS", "BACKGROUND"],
    suspendable: true,
    destroyable: true,
  },
  permissions: ["network.connect"],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useAgentStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）。
  v1: {
    id: "agent",
    version: "1.0.0",
    displayName: "智能体",
    description:
      "编码工作区支持模型选择、Monaco 编辑、审核后应用文件修改、授权上下文及隔离 Rust 文件/模块树语言服务。旧蓝图 agent_chat/run 契约未实现；Cargo 依赖推断、调试器与完整生命周期验证待补。",
    maturity: "C1",
    maturityEvidence: ["scripts/check-developer-owners.mjs"],
    dependencies: ["bridge"],
    optionalDependencies: ["script", "git", "database", "browser", "terminal"],
    conflicts: [],
    provides: ["agent.chat", "agent.run", "agent.memory"],
    requires: [],
    contributions: [{ id: "agent.main.panel", slot: "workbench-main", type: "surface" }, { id: "agent.studio", slot: "workbench-main", type: "surface", view: "studio" }],
    permissions: ["network.connect"],
    resources: [
      // 编码工作区资源；旧 agent_chat 流式执行契约仍待实现。
      { kind: "NETWORK", ownership: "owned", evidence: "src-tauri/src/framework/ai.rs provider requests; coding.rs bounded tool loop" },
      { kind: "PROCESS", ownership: "owned", evidence: "coding_context.rs bounded approved MCP; coding_lsp.rs bounded per-request isolated rust-analyzer" },
      { kind: "BACKGROUND_TASK", ownership: "owned", evidence: "codingEditorServices.mjs lazy browser editor and TypeScript workers, released with editor" },
    ],
    persistenceScope: "disk",
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
        "HP0(STATIC)：执行后端(agent_chat/run)未实现；仅只读命令就绪，未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/agent/public.ts" }],
    entrypoint: "src/capabilities/agent/index.ts",
    semanticOwner: "useAgentStore",
  },
}
