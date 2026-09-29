import type { CapabilityDefinition } from "./types";

// Clipboard 能力清单（metadata API，经 './manifest' 子路径导出）。
// 不导出任何 runtime 实现；catalog.ts 只读取纯数据清单（零副作用）。
export const clipboardManifest: CapabilityDefinition = {
  id: "clipboard",
  name: "剪贴板",
  category: "CAPABILITY",
  provides: ["capability:clipboard"],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "BACKGROUND"],
    default: "BACKGROUND",
    activatable: true,
    resident: true,
  },
  resources: {
    class: ["LIGHT"],
    suspendable: true,
    destroyable: true,
  },
  permissions: [],
  persistence: {
    scope: "session",
    sensitive: false,
  },
  entrypoint: "packages/capability-clipboard/src/index.ts",
  semanticOwner: "useClipboardStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  v1: {
    id: "clipboard",
    version: "1.0.0",
    displayName: "剪贴板",
    description:
      "复制/粘贴与剪贴板历史，自动收集系统剪贴板内容（仅内存，关闭应用即清空）。",
    maturity: "C2",
    maturityEvidence: [
      "scripts/check-package.mjs",
      "scripts/check-clipboard-logic.mjs",
      "scripts/check-clipboard-persistence-logic.mjs",
    ],
    dependencies: ["bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["capability:clipboard"],
    requires: [],
    contributions: [
      {
        id: "clipboard.main.panel",
        slot: "workbench-main",
        type: "surface",
        view: "clip",
      },
    ],
    permissions: [],
    resources: [
      {
        kind: "CAPABILITY_STATE",
        ownership:
          "ClipboardPanel 由 useClipboardStore 拥有，clipOpen 归 useLayoutStore（视图态，非本能力域）",
        evidence: "packages/capability-clipboard/src/state/useClipboardStore.ts",
      },
    ],
    persistenceScope: "session",
    persistenceSensitive: false,
    activationPolicy: {},
    deactivationPolicy: {},
    installPolicy: {},
    uninstallPolicy: {},
    hotPlug: {
      level: "HP0",
      limitationReason:
        "HP0：剪贴板为常驻静态能力（resident:true，随构建静态编入），无运行时动态加载/卸载/注册链路；其贡献 panel 经生命周期 onActivate 单点注册（scripts/check-package.mjs PKG-12 已验证仅注册一次），不引入 HP3 所需外部能力包动态沙箱安装。",
    },
    publicContract: [
      {
        name: "useClipboardStore",
        locator: "packages/capability-clipboard/src/index.ts",
      },
    ],
    entrypoint: "packages/capability-clipboard/src/index.ts",
    semanticOwner: "useClipboardStore",
  },
};
