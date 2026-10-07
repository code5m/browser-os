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
    maturity: "C3",
    maturityEvidence: [
      "scripts/check-package.mjs",
      "scripts/check-clipboard-logic.mjs",
      "scripts/check-clipboard-persistence-logic.mjs",
      "scripts/check-hot-plug-acceptance.mjs",
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
      level: "HP2",
      enable: true,
      disable: true,
      register: true,
      unregister: true,
      install: false,
      uninstall: false,
      limitationReason:
        "HP2：focus/Tauri focus listener 由 Clipboard 生命周期拥有并对称解绑；统一 Harness 验证贡献摘除、重复启停与 fresh Runtime。",
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
