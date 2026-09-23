import type { CapabilityDefinition } from "../../capability/types"

// Clipboard 能力 Manifest（Capability Library Expansion v1 — STAGE H）
// 语义 owner = useClipboardStore（STAGE H 从 useSystemStore 拆出，解除 Debt-8E-1：
// Clipboard 与 Apps 曾共居一个 store、owner 同名）。
// 物理：ClipboardPanel 经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='clip'）贡献给 MainArea。
// 安全：剪贴板历史**不落盘**（B11-1 红线）——persistence.scope=`session`（仅内存会话态，绝不写 localStorage）；
// UI 渲染经 redactSecrets 脱敏（凭据/token 不入渲染）。
// 后端 clipboard_read/clipboard_write 已实现（arboard）；无资源创建。
//
// 成熟度：C2 ISOLATED（边界隔离 + 贡献驱动）。非 C3：无 absence 门禁 + mainView='clip' nav 硬编码
//   + 面板开合态 `clipOpen` 仍归 useLayoutStore（视图态，非本能力域）。
export const clipboardManifest: CapabilityDefinition = {
  id: "clipboard",
  name: "剪贴板",
  category: "CAPABILITY",
  provides: ["clipboard.read", "clipboard.write", "clipboard.history"],
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
    // 会话内存态（不落盘）——B11-1 红线。
    scope: "session",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useClipboardStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  v1: {
    id: "clipboard",
    version: "1.0.0",
    displayName: "剪贴板",
    description: "系统剪贴板读写 + 会话内历史（不落盘，B11-1）。后端 clipboard_read/clipboard_write（arboard）；无资源创建。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-clipboard-persistence-logic.mjs"],
    dependencies: ["bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["clipboard.read", "clipboard.write", "clipboard.history"],
    requires: [],
    contributions: [{ id: "clipboard.main.panel", slot: "workbench-main", type: "surface", view: "clip" }],
    permissions: [],
    resources: [],
    persistenceScope: "session",
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
        "HP0(STATIC)：剪贴板历史为会话内存态，无独立生命周期可装卸；未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/clipboard/public.ts" }],
    entrypoint: "src/capabilities/clipboard/index.ts",
    semanticOwner: "useClipboardStore",
  },
}
