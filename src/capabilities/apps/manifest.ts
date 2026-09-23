import type { CapabilityDefinition } from "../../capability/types"

// Apps 能力 Manifest（Capability Library Expansion v1 — STAGE H）
// 语义 owner = useAppsStore（STAGE H 从 useSystemStore 拆出，解除 Debt-8E-1）。
// 物理：AppPanel 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='apps'）贡献给 MainArea。
// 后端 list_apps（扫描 .desktop）与 launch_app（security_policy::check_launch_target 白名单式解析，
// 禁 sh -c，写审计）已实现。launch_app 会 `Command::spawn` 启动外部应用进程 → CHILD_PROCESS（detached）。
// 主页应用快捷方式持久化：app 命令体禁止落浏览器存储（isStorageSafe，门禁 HOME_NO_SECRET_PERSIST）。
//
// 成熟度：C2 ISOLATED。非 C3：无 absence 门禁 + mainView='apps' nav 硬编码。
export const appsManifest: CapabilityDefinition = {
  id: "apps",
  name: "系统应用",
  category: "CAPABILITY",
  provides: ["apps.list", "apps.launch"],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["LIGHT", "PROCESS"],
    suspendable: true,
    destroyable: true,
  },
  permissions: ["process.spawn"],
  persistence: {
    scope: "runtime_only",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useAppsStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  v1: {
    id: "apps",
    version: "1.0.0",
    displayName: "系统应用",
    description: "系统应用枚举（.desktop 扫描）与启动。启动经 security_policy::check_launch_target 白名单式解析（禁 sh -c）+ 审计。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-home-client-policy.py", "scripts/check-ui-boundaries.mjs"],
    dependencies: ["bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["apps.list", "apps.launch"],
    requires: [],
    contributions: [{ id: "apps.main.panel", slot: "workbench-main", type: "surface", view: "apps" }],
    permissions: ["process.spawn"],
    resources: [
      { kind: "CHILD_PROCESS", ownership: "owned", evidence: "src-tauri/src/bridge.rs:launch_app (Command::spawn, detached)" },
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
        "HP0(STATIC)：启动的外部应用进程为 detached（本能力不管理其生命周期）；无独立生命周期可装卸，未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/apps/public.ts" }],
    entrypoint: "src/capabilities/apps/index.ts",
    semanticOwner: "useAppsStore",
  },
}
