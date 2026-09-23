import type { CapabilityDefinition } from "../capability/types"

// Settings 能力 Manifest（STAGE I-C）
//
// 分类：SERVICE（框架 / 平台服务，**非**产品 CAPABILITY）。
// 主题 / 键位 / 页签休眠属框架偏好，不归属任何单一业务能力；因此 settings 不作
// 为产品能力暴露，仅登记为常驻框架服务。SettingsPanel 经通用 WORKBENCH_MAIN
// 贡献（view='settings'）解耦，Shell 经 src/settings/public.ts 显式契约访问。
export const settingsManifest: CapabilityDefinition = {
  id: "settings",
  name: "Settings",
  category: "SERVICE",
  provides: ["settings.theme", "settings.keymapScheme", "settings.tabHibernation"],
  dependsOn: [],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE"],
    default: "ACTIVE",
    activatable: false,
    resident: true,
  },
  resources: {
    class: ["LIGHT"],
    suspendable: false,
    destroyable: false,
  },
  permissions: [],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useSettingsStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  v1: {
    id: "settings",
    version: "1.0.0",
    displayName: "Settings",
    description:
      "框架偏好（主题 / 键位 / 页签休眠）常驻服务；SettingsPanel 经 WORKBENCH_MAIN 贡献解耦，Shell 经 src/settings/public.ts 访问。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-capability-registry.mjs", "scripts/check-capability-composition.mjs"],
    dependencies: [],
    optionalDependencies: [],
    conflicts: [],
    provides: ["settings.theme", "settings.keymapScheme", "settings.tabHibernation"],
    requires: [],
    contributions: [{ id: "settings.main", slot: "workbench-main", type: "surface", view: "settings" }],
    permissions: [],
    resources: [{ kind: "CACHE", ownership: "owned", evidence: "src/stores/useSettingsStore.ts" }],
    persistenceScope: "disk",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "manual",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP2",
      enable: true,
      disable: true,
      register: true,
      unregister: true,
      install: false,
      uninstall: false,
      limitationReason:
        "HP2：常驻框架服务，无重资源、无后台任务，运行时 register/unregister 已由 scripts/check-capability-platform.mjs 验证；HP3 需动态安装链路，本夜不做（禁止高风险动态代码加载）。",
    },
    publicContract: [{ name: "publicApi", locator: "src/settings/public.ts" }],
    entrypoint: "src/settings/index.ts",
    semanticOwner: "useSettingsStore",
  },
}
