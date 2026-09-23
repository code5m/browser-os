import type { CapabilityDefinition } from "../../capability/types"

// Agent 能力 Manifest（Capability Library Expansion v1 — 从 Workspace 内嵌面板升格为独立 Capability）
// 语义 owner = useAgentStore（Semantic Registry 已冻结；同时持有 skill 状态，STAGE E 处理 skill 拆分）。
// 物理：AgentManagerPanel(+AgentChatPanel 子视图) 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽
// （view='agents'）贡献给 MainArea；MainArea 只按槽渲染、不 import 本能力内部（C3 关键）。
//
// 诚实声明：执行后端（agent_chat / agent_run / agent_install 等）在 Rust 侧**尚未实现**
// （bridge.ts: AGENT_SKILL_COMMANDS_AVAILABLE=false，仅 agent_parse/validate/permission_preview 只读命令就绪）。
// 故当前 Agent 为「只读壳 + 解析/校验/权限预览」，chat/run 走 guard() 拦截、不 invoke。
// 这是真实功能缺口，不是边界缺失 —— 边界隔离已完成（C1），执行后端留作后续 stage。
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
  optionalDependencies: ["knowledge_graph"],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["MEDIUM", "NETWORK"],
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
      "Agent 定义/安装/校验与对话壳。执行通道（agent_chat/run）后端尚未实现，当前为只读视图 + 解析/校验/权限预览。",
    maturity: "C1",
    maturityEvidence: ["scripts/check-developer-owners.mjs"],
    dependencies: ["bridge"],
    optionalDependencies: ["knowledge_graph"],
    conflicts: [],
    provides: ["agent.chat", "agent.run", "agent.memory"],
    requires: [],
    contributions: [{ id: "agent.main.panel", slot: "workbench-main", type: "surface" }],
    permissions: ["network.connect"],
    resources: [
      // 后端仅只读命令就绪；执行资源（agent_chat 流式）待后端实现。
      { kind: "NETWORK", ownership: "owned", evidence: "src-tauri/src/agent*.rs (readonly: agent_parse/validate/permission_preview)" },
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
