# terminal 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/terminal/manifest.ts`。
> 诚实边界：M3 终态已交付（worker→mpsc→pump→sink 三段式，Channel 单播）；PTY 在 suspend 时不释放（`suspendable=false`，supported 仅 `ACTIVE`）。

---

## 1. Purpose
终端域。负责 PTY/子进程终端的创建、写入、杀死、resize、宫格多终端编排，以及背压遥测与历史 replay。

## 2. Domain Classification
- 领域：`terminal`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 终端 PTY 生命周期（`spawnTerm`/`ensureTerm`/`killTerm`）。
- 终端写入/键盘事件（`termWrite`/`termKeydown`）。
- 多终端注册表（`termPanes`）与宫格（`termGrid`/`termGridCount`）。
- Channel 单播分发（`onTermChannelMsg`）、背压遥测（`droppedChunks`/`droppedBytes`）、历史 replay（`replayTermHistory`）。

## 4. Non-Responsibilities
- 不负责浏览器 WebView（宫格归 `browser`）。
- 不负责其它域。
- 不在 suspend 时回收 PTY（设计约束）。

## 5. Ubiquitous Language
- `termPane`：一个终端 PTY 注册项（id/pty ref）。
- `activeTermId`：当前活动终端（∈ `termPanes.id` 不变式）。
- `termGrid` / `termGridCount`：终端宫格编排。
- `droppedChunks` / `droppedBytes`：背压丢弃遥测。
- `m0Cfg`：M0 吞吐驱动配置。

## 6. Domain Model
- 聚合根：终端注册表 + 宫格（`useTerminalStore` 管理，pinia id `terminal`）。
- 关键 state：`terminalOpen` / `termPanes` / `termGrid` / `termGridCount` / `activeTermId` / `autoConfirmCli` / `termProbeOn` / `droppedChunks` / `droppedBytes` / `m0Cfg`（`src/capabilities/terminal/state/useTerminalStore.ts`）。

## 7. Invariants
- `activeTermId` 必须 ∈ `termPanes.id`。
- PTY 出生点是 `ui/TerminalView.vue` 的 `ensureTerm`，import 本模块**不会**创建 PTY（public 注释强调）。
- `spawnTerm` 必须经 `isTerminalResourceAllowed()` 闸（fail-closed，依赖 `isCapabilityActive`）。
- `termPanes`/`activeTermId` 变更须经 `onTermChannelMsg` 单播分发。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/terminal/state/useTerminalStore.ts`（`defineStore("terminal")`）。
- **semanticOwner**：`useTerminalStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：无物理债务（state/ui 均在包内，已确认 terminal UI 未混入 `components/layout`）。

## 9. Commands / Intents
- 业务 action：`spawnTerm` / `ensureTerm` / `killTerm` / `termWrite` / `termKeydown` / `onTermChannelMsg` / `replayTermHistory` / `openTerminalAt`。
- 原生命令（见 §17）：`term_spawn` / `term_spawn_channel` / `term_write` / `term_resize` / `term_kill`。

## 10. Queries
- 前端内存：`termPanes`/`termGrid`/`activeTermId` 派生。
- 原生：无独立查询命令（spawn 后由 Channel 流式）。

## 11. Events
- Channel 单播消息（`onTermChannelMsg`）：`data`/`flow`/`exit` 三类。

## 12. Public Contract
- 入口：`src/capabilities/terminal/public.ts`。
- 暴露：`useTerminalStore`（再导出，**无** manifest 再导出、**无**类型导出；注释强调 import 不创建 PTY）。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useTerminalStore.ts` / `resource/guard.ts`（`isTerminalResourceAllowed`）/ `ui/`（TerminalView/TerminalPane/TerminalDockPanel + useTerminalResize.ts）。

## 14. Dependencies
- `dependsOn: []`（无声明硬依赖）。
- `optionalDependencies: []`。
- 跨能力耦合：import `useLayoutStore`（视图态）。

## 15. Dependents
- 消费 `capabilities/terminal/public` 的 **5 处**：`src/App.vue`、`src/composables/terminalNav.ts`、`src/components/layout/UnifiedTabBar.vue`、`src/components/layout/ActivityBar.vue`、`src/components/layout/StatusBar.vue`（均经公共契约消费，符合 C3）。

## 16. Frontend Boundary
- 贡献组件：`TerminalView.vue`（WORKBENCH_MAIN_RESIDENT，view=`term`）+ `TerminalDockPanel.vue`（BROWSER_DOCK，view=`term`）。
- 注册：`registerTerminalContributions()`（2 条贡献）。
- **CURRENT PHYSICAL LOCATION**：全部 3 个 .vue + useTerminalResize.ts 在 `src/capabilities/terminal/ui/`（已确认未混入 `components/layout`）。`UnifiedTabBar.vue` 仅消费 `useTerminalStore` 的 `m0Cfg?.driver`（2 处），其"宫格"是 browser 的 webview 宫格，**非**终端宫格。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/terminal.rs`（PTY/子进程，35KB）+ `src-tauri/src/bridge.rs` 命令壳。
- 已注册（main.rs）：`bridge::term_spawn`/`term_spawn_channel`/`term_write`/`term_resize`/`term_kill`。
- 前端封装：`src/bridge.ts`（`termSpawnChannel`/`termWrite`/`termResize`/`termKill` + `createTermChannel`）。
- **诚实声明**：Native 仍集中于 `bridge.rs`/`terminal.rs`，未物理模块化；`term_spawn` 为 Event sink（仅内部），`term_spawn_channel` 为前端唯一入口（Tauri2 Channel 限制）。

## 18. Resources
- `resources.class: ["PROCESS","PTY"]`；`suspendable: false`（onSuspend 未实现资源回收）；`destroyable: true`。
- `v1.resources: [{kind:"PTY",owned},{kind:"CHILD_PROCESS",owned}]`（evidence=measure-resources.mjs）。
- `persistence.scope: "session"`；`sensitive: false`。

## 19. Side Effects
- 创建/销毁 PTY 与子进程（重资源）。
- 背压丢弃（`droppedChunks`/`droppedBytes`）。

## 20. Security
- `isTerminalResourceAllowed()` fail-closed 守门（依赖 `isCapabilityActive`）。
- 进程组回收（terminal.rs）。

## 21. Persistence
- 声明 `session`；PTY/子进程不落盘，历史为临时环形数组（40 条），切 Tab 卸载 xterm 但 PTY 仍活，故须 replay。

## 22. Failure Model
- PTY 创建失败：`error` 态 + UI 提示。
- Channel 死：`on_channel_dead` 待 wait（挂账 D25）。

## 23. Capability Absence
- Absent 时：`registerTerminalContributions` 未执行 → WORKBENCH_MAIN_RESIDENT / BROWSER_DOCK 无 view=`term` → Shell 不渲染。
- 但 `useTerminalStore` 被 5 处经 public 消费（物理债务无，但 absent 不保证该 store 完全不实例化）。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE"]`（仅）；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: graceful`（PLY 资源回收未实现）。

## 25. UI Contributions
- `terminal.main.term`（WORKBENCH_MAIN_RESIDENT / surface / view=`term` / TerminalView）
- `terminal.dock.term`（BROWSER_DOCK / surface / view=`term` / TerminalDockPanel，order=20）

## 26. Testing
- `src/capabilities/terminal/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 门禁：`scripts/check-terminal-policy.py`、`scripts/check-terminal-owners.mjs`、`scripts/check-terminal-ui-logic.mjs`、`scripts/measure-resources.mjs`、`scripts/check-composition-profiles.mjs`。

## 27. Gates
- `scripts/check-terminal-policy.py`（**强**，M3.a/M3.c，channel 单播、进程组回收、resize、背压丢弃、测量模式、ACL）。
- `scripts/check-terminal-owners.mjs`（PTY 重资源 owner 断言）。
- `scripts/check-terminal-ui-logic.mjs`、`scripts/measure-resources.mjs`、`scripts/check-composition-profiles.mjs`。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useTerminalStore.ts` → `resource/guard.ts` → `ui/TerminalView.vue` → 后端 `terminal.rs`。
- 关注点：PTY 资源回收（suspendable=false）、Channel 单播、背压、absence 门禁。

## 29. AI Modification Guide
- 改 PTY 逻辑：必须保持 Channel 单播 + 进程组回收，并同步 `check-terminal-policy.py` 断言。
- 禁止：用裸 `invoke`、把 store 移出时不同步 manifest、新增 UI 不进 Contribution Registry。
- 实现 suspend 资源回收前不得宣称 `suspendable=true`。

## 30. Known Debt
- Debt-8E-1：Clipboard/Apps 不属本 owner，仍留在 `useSystemStore`（注释引用，非本模块债务）。
- PTY 在 suspend 时不释放 → `suspendable=false`、supported 仅 `ACTIVE`。
- 历史 replay 已实现的特性（非债务）；挂账 D23~D26（终端 GUI 实点/吞吐基线/on_channel_dead wait/历史封顶）。

## 31. C / HP / M / RV / D
- **C = C3**：`manifest.v1.maturity="C3"`，owners+composition+measure 三 checker；state/ui 完全隔离、5 处经 public 消费。
- **HP = HP0**：`manifest.v1.hotPlug.level="HP0"`（全 false）；PTY 释放需先落地 graceful policy 与测量证据。
- **M = M1（完全）**：`src/capabilities/terminal/` 目录隔离（含 resource/ guard，6 .ts + 3 .vue），UI 未混入 layout。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2（强） + RV3（否）**：owner 已登记（RV1）；`check-terminal-owners.mjs` + `check-composition-profiles.mjs` + `measure-resources.mjs` 定向覆盖（RV2 强）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、PTY suspend 回收未实现（影响 HP）、absence 门禁。
- 物理隔离完备（含 resource guard），可作 Package Extraction 范本候选。

## 33. Source of Truth
- manifest：`src/capabilities/terminal/manifest.ts`
- public：`src/capabilities/terminal/public.ts`
- state：`src/capabilities/terminal/state/useTerminalStore.ts`
- UI：`src/capabilities/terminal/ui/`
- native：`src-tauri/src/terminal.rs`、`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useTerminalStore"`
- gates：`scripts/check-terminal-policy.py`、`scripts/check-terminal-owners.mjs`、`scripts/check-terminal-ui-logic.mjs`（见 §27）
