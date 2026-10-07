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
    description: "剪贴板读写与内存历史。",
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
        "HP2：focus/Tauri listener 对称解绑；Harness 验证重启。",
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
