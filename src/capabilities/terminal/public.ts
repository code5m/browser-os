// Terminal 能力公共边界（Phase 8E / Train D）
//
// Shell（App / MainArea / StatusBar / UnifiedTabBar / ActivityBar）与其它能力经此消费 Terminal，
// 而**不直接 import 能力内部**（src/capabilities/terminal/state/*、src/capabilities/terminal/ui/*）。
//
// 注意：本文件是**纯再导出**（re-export），不持有也不创建任何状态（第二真源禁止）。
// 语义 owner（useTerminalStore）与物理路径解耦，由 Semantic Registry 的 owner_implementations
// locator 解析（states.yaml）。
//
// 边界提示：import 本模块**不会**创建 PTY；PTY 的出生点是 ui/TerminalView.vue 的 ensureTerm
// （能力贡献被挂载时才可能发生）。因此 Terminal 未注册时不产生任何子进程。
export { useTerminalStore } from "./state/useTerminalStore"
