import type { CapabilityDefinition } from "../../capability/types"

// Skill 能力 Manifest（Capability Library Expansion v1 — STAGE E）
// 语义 owner = useSkillStore（STAGE E 从 useAgentStore 迁出；§21：Skill 不再被 Agent 偷偷持有）。
// 物理：SkillManagerPanel 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='skills'）贡献给 MainArea。
//
// 诚实声明：执行后端（skill_run / skill_install 等）Rust 未实现（AGENT_SKILL_COMMANDS_AVAILABLE=false），
// 仅只读命令（skill_parse/validate/permission_preview）就绪 → 当前为只读壳。
// Skill 与 Agent 为独立能力，共用中性安装/确认协调器（useInstallConfirmStore）与 readonly 命令族，
// 不再声明 skill 依赖 agent（已解除纠缠）。
export const skillManifest: CapabilityDefinition = {
  id: "skill",
  name: "技能",
  category: "CAPABILITY",
  provides: [
    "skill.list",
    "skill.run",
    "skill.install",
  ],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["LIGHT"],
    suspendable: true,
    destroyable: true,
  },
  permissions: [],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useSkillStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）。
  v1: {
    id: "skill",
    version: "1.0.0",
    displayName: "技能",
    description:
      "Skill 定义/校验/预览与安装壳。执行通道（skill_run/install）后端尚未实现，当前为只读壳。",
    maturity: "C1",
    maturityEvidence: ["scripts/check-developer-owners.mjs"],
    dependencies: ["bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["skill.list", "skill.run", "skill.install"],
    requires: [],
    contributions: [{ id: "skill.main.panel", slot: "workbench-main", type: "surface" }],
    permissions: [],
    resources: [
      { kind: "LIGHT", ownership: "owned", evidence: "src-tauri/src/skills.rs (readonly: skill_parse/validate/permission_preview)" },
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
        "HP0(STATIC)：执行后端(skill_run/install)未实现；仅只读命令就绪，未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/skill/public.ts" }],
    entrypoint: "src/capabilities/skill/index.ts",
    semanticOwner: "useSkillStore",
  },
}
