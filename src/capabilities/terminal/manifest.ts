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
  dependsOn: [],
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
}
