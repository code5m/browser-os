import type { CapabilityDefinition } from "../../capability/types"

// Home 能力 Manifest（STAGE I-B — 物理隔离）
//
// 语义裁决：Home 拥有「主页快捷方式（url/app/dir）+ 最近访问 + 启动器元数据」，
// 单一状态 owner = useHomeStore（id=home）。跨能力消费者仅 workspace（经
// capabilities/home/public.ts 的窄契约 favoriteDirectory）——严禁直连内部 store。
// 物理迁移（src/stores/useHomeStore.ts → src/capabilities/home/state/；
// src/components/home/* → src/capabilities/home/ui/）不改变语义 owner，业务 UI 零改动。
export const homeManifest: CapabilityDefinition = {
  id: "home",
  name: "主页",
  category: "CAPABILITY",
  provides: ["home.shortcut", "home.recent", "home.launch", "home.favorite"],
  dependsOn: ["browser", "workspace", "apps", "bridge"],
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
  permissions: ["fs.read"],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useHomeStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  v1: {
    id: "home",
    version: "1.0.0",
    displayName: "主页",
    description:
      "默认工作台表面：主页快捷方式（网页/应用/目录）、最近访问、主要工作区启动器；经 WORKBENCH_MAIN 贡献 view='home' 渲染，跨能力仅暴露窄 public 契约。",
    maturity: "C3",
    maturityEvidence: ["scripts/check-home-ui-logic.mjs", "scripts/check-home-client-policy.py", "scripts/check-capability-platform.mjs", "scripts/check-hot-plug-acceptance.mjs"],
    dependencies: ["browser", "workspace", "apps", "bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["home.shortcut", "home.recent", "home.launch", "home.favorite"],
    requires: [],
    contributions: [{ id: "home.main", slot: "workbench-main", type: "surface", view: "home" }],
    permissions: ["fs.read"],
    resources: [{ kind: "CACHE", ownership: "owned", evidence: "src/capabilities/home/state/useHomeStore.ts" }],
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
        "HP2：无重资源、无后台任务；统一 Hot-Plug Acceptance Harness 已验证 pause/resume、disable/enable、贡献摘除/恢复、重复循环与 fresh Runtime 持久化。HP3 需动态安装链路，不在本轮范围。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/home/public.ts" }],
    entrypoint: "src/capabilities/home/index.ts",
    semanticOwner: "useHomeStore",
  },
}
