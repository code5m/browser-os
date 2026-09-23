import type { CapabilityDefinition } from "../../capability/types"

// Plugin 能力 Manifest（Capability Library Expansion v1 — STAGE F）
// 语义 owner = usePluginStore（STAGE F 物理迁入 capabilities/plugin/state，单一真源，§21）。
// 物理：PluginManager 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='plugin'）贡献给 MainArea。
//
// 诚实边界（Stage-I「manifest 生命周期登记簿」）：
//   - 后端 `plugin_list/get/install/enable/disable/keys_add/keys_list/keys_remove` 命令**已实现**：
//     本地登记簿 + 生命周期状态机 + 受信任公钥指纹，落盘 `plugins.json` / `trusted-pubkeys.json`（atomic_write）。
//   - **但无 plugin 运行时**：不解包、不真验签（仅结构校验）、不动态加载、不执行、不下载、不联网。
//     `governanceStatus=LOCKED`，门禁 `check-plugin-policy.py` 的 `PLUGIN_NO_EXEC_SURFACE` 强制无执行面。
//   - **五态分别判定，禁止合并成单一 boolean**（§18）：
//       AVAILABLE（可发现，Stage-I 无 discovery 源 → 恒 false）
//       INSTALLED（登记簿存在且 state≠uninstalled）
//       ENABLED  （state=enabled，仅生命周期开关，≠运行）
//       ACTIVE   （运行时已加载运行，无 loader → 恒 false）
//       RESOURCE_EXISTS（活本地资源实例，无 loader → 恒 false；与 resource.path_provided/verified 非同一事）
//   - 资源画像 `class:[HEAVY,NATIVE]` 为**声明**（潜在），Stage-I 从不创建 load 实例 → `v1.resources` 为空。
//
// 成熟度：C2 ISOLATED（实现经 manifest/public/index/ui/state 边界隔离 + 贡献驱动）。
//   非 C3：① 设计 LOCKED（运行时不可组合启停，profiles 保守口径）；
//          ② 无 plugin 专属 absence 运行时门禁；③ mainView='plugin' 导航项仍硬编码未贡献驱动。
export const pluginManifest: CapabilityDefinition = {
  id: "plugin",
  name: "插件",
  category: "CAPABILITY",
  provides: [
    "plugin.list",
    "plugin.manifest",
  ],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["HEAVY", "NATIVE"],
    suspendable: false,
    destroyable: false,
  },
  permissions: [],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "usePluginStore",
  governanceStatus: "LOCKED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）。
  v1: {
    id: "plugin",
    version: "1.0.0",
    displayName: "插件",
    description:
      "插件 manifest 生命周期登记簿（list/get/install/enable/disable/keys）。运行时（解包/真验签/动态加载/执行）未实现，Stage-I LOCKED；ENABLED ≠ ACTIVE，RESOURCE_EXISTS 恒 false。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-plugin-policy.py", "scripts/check-plugin-ui-logic.mjs"],
    dependencies: ["bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["plugin.list", "plugin.manifest"],
    requires: [],
    contributions: [{ id: "plugin.main.panel", slot: "workbench-main", type: "surface", view: "plugin" }],
    permissions: [],
    // Stage-I 从不创建 load 实例 → 不声明任何 owned 资源（诚实；class 为潜在声明）。
    resources: [],
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
        "HP0(STATIC)：plugin 运行时(loader/执行)未实现且治理 LOCKED；无 runtime 实例可装卸，未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/plugin/public.ts" }],
    entrypoint: "src/capabilities/plugin/index.ts",
    semanticOwner: "usePluginStore",
  },
}
