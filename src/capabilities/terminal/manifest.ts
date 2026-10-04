import type { CapabilityDefinition } from "../../capability/types"

// Terminal 能力 Manifest（Phase 8E / Train D — 物理隔离）
// 语义 owner = useTerminalStore（Semantic Registry 已冻结 Phase 4 Terminal Lifecycle 语义：
// INV-4-1 termPanes 唯一面板注册表、INV-4-2 activeTermId ∈ termPanes.id；本 Train 零语义变更，
// 仅把 owner 从混居的 useSystemStore 抽为专属 owner，见 SCR-20260920-terminal-owner-extraction）。
//
// resources: PROCESS + PTY（真实子进程 / 伪终端）→ 是「资源守门人」（Train F）首要协调对象。
export const terminalManifest: CapabilityDefinition = {
  id: "terminal",
  name: "终端",
  category: "CAPABILITY",
  provides: [
    "terminal.spawn",
    "terminal.write",
    "terminal.kill",
    "terminal.resize",
    "terminal.grid",
  ],
  dependsOn: ["bridge"],
  optionalDependencies: [],
  lifecycle: {
    // Train F 诚实裁决：Terminal 的 PTY/子进程在 suspend 时**不**释放（onSuspend 未实现资源回收），
    // 故 supported 仅 ACTIVE；与 resources.suspendable:false 一致，避免「声明可 suspend 却不释放」的误报。
    supported: ["ACTIVE"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["PROCESS", "PTY"],
    suspendable: false,
    destroyable: true,
  },
  permissions: ["process.spawn"],
  persistence: {
    scope: "session",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useTerminalStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9/§25）：PTY/子进程为真实 owned resource，释放必须有测量证据。
  v1: {
    id: "terminal",
    version: "1.0.0",
    displayName: "终端",
    description: "PTY 伪终端与子进程宿主；持有最重资源，常驻主视图以保持 xterm 实例存活。",
    maturity: "C3",
    maturityEvidence: [
      "scripts/check-terminal-owners.mjs",
      "scripts/check-composition-profiles.mjs",
      "scripts/measure-resources.mjs",
    ],
    dependencies: [],
    optionalDependencies: [],
    conflicts: [],
    provides: ["terminal.spawn", "terminal.write", "terminal.kill", "terminal.resize", "terminal.grid"],
    requires: [],
    contributions: [
      { id: "terminal.main.term", slot: "workbench-main-resident", type: "surface", view: "term" },
      { id: "terminal.dock.term", slot: "browser-dock", type: "surface", view: "term" },
    ],
    permissions: ["process.spawn"],
    resources: [
      { kind: "PTY", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
      { kind: "CHILD_PROCESS", ownership: "owned", evidence: "scripts/measure-resources.mjs" },
    ],
    persistenceScope: "session",
    persistenceSensitive: false,
    activationPolicy: "auto",
    // §25：停用前必须处理活动会话（REJECT 或 GRACEFUL CLOSE），禁止静默 kill 用户未知工作。
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
      limitationReason: "HP0(STATIC)：活动会话的停用/释放需先落地 graceful policy 与 PTY 测量证据，未验证前不宣称更高等级。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/terminal/public.ts" }],
    entrypoint: "src/capabilities/terminal/index.ts",
    semanticOwner: "useTerminalStore",
  },
}
