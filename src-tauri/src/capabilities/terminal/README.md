# Capability Native Module: `terminal` (Rust)

> 迁移自 `src-tauri/src/terminal.rs`（Native Physical Boundary Matrix Pilot 11）。
> 分类：**CAPABILITY_NATIVE(terminal)**（矩阵 §2.3；target `src-tauri/src/capabilities/terminal/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.14。
> 同级 TS 能力：`src/capabilities/terminal/`（成熟度 C1，命令接线仍经 `bridge.ts`）。
> 注意：本模块是 **PTY / 终端输出管道内核**（M3 终端 worker），**无 `#[tauri::command]`**；终端命令体在 `bridge.rs`。

## 1. DDD 职责（Domain Responsibility）

终端能力的原生实现，承载 `terminal.rs`（PTY 内核 + 输出管道）：

- **三段式管道**：worker→mpsc(128)→pump→sink（`EventSink` 落本地 / `ChannelSink` 推前端 `Channel`）。
- **临时历史**：环形数组（TerminalPane 切 Tab 卸载 xterm 但 PTY 仍活 → 重进 Tab 须 replay）。
- **resize 真实化**：master.resize(PtySize) 落到 PTY master。
- **背压退避**：`send_with_backoff`（10ms×2^n 上限 30s / 总预算 60s）。
- **内核接口**：`term_spawn` / `term_spawn_channel` / `term_kill` / `TermInfo` / `TerminalSession` / `ChannelSink` / `EventSink` / `DropCounter` / `TerminalHandles` / `TermProbe`。

## 2. 边界（Boundary / Non-Responsibility）

- **不含任何 `#[tauri::command]`**：终端命令（`term_spawn_channel` / `terminal_resize` / `terminal_send` / `terminal_kill` 等）命令体在 `bridge.rs`（经 `generate_handler!`），本模块只提供内核；`bridge.rs` 以 `pub use crate::terminal::{TermInfo, TerminalSession}` 再导出供命令层使用。
- **不含调度**：终端只做进程/PTY 生命周期 + 输出管道（契约 §4.3）。
- **后端零持久化状态**：输出历史只在前端会话内（契约 §6.2）；后端不留输出历史（由 `check-terminal-policy.py` 机器守护：TERM_HISTORY_BACKEND 码）。
- 无 WebView / socket / 定时器 / 网络（PTY 子进程除外，受 `security_policy` 黑名单约束）。

## 3. Commands

**本模块无命令。** terminal 命令体归属 terminal 能力，注册在 `bridge.rs`（经 `generate_handler!`），命令体调用本模块内核。命令体在 **bridge.rs 逐 command 分解阶段**迁移至 `capabilities/terminal/commands.rs`。

## 4. Resources

- PTY（`portable_pty`）、子进程、后台线程（worker/pump 不得泄漏，join 收口）。
- 子进程受 `security_policy.rs` 黑名单（BLOCKED_LAUNCH_PROGRAMS / WRAPPERS / INTERPRETERS）约束。
- 无文件系统持久化（历史只在前端）；无 AppState 字段（terminal.rs 零 `bridge::AppState` 耦合，命令体以局部句柄持有 `TerminalSession`）。

## 5. 生命周期（Lifecycle）

- 启动：`bridge::term_spawn` 命令体创建 `TerminalSession`（PTY + worker + pump + 双 sink）。
- 运行：mpsc(128) 背压、send_with_backoff 退避；resize 真实化 master。
- 退出：命令体 `terminal_kill` → 进程组回收（委托 `script_runner::process_group_alive` 判定）+ DropCounter 观测丢弃 + 线程 join，无 worker/pump 泄漏。

## 6. 依赖（Dependencies）

- `crate::script_runner`：`process_group_alive`（进程组回收，SHARED_NATIVE_INFRASTRUCTURE）。
- `crate::domain`：`TerminalSession` 关联类型（如需要）。
- 外部 crate：`portable_pty` / `tauri`（`AppHandle` / `Channel` / `Manager`）/ 标准库。
- **零 `bridge::AppState` 耦合**（grep 确认 terminal.rs 无 `app.state` / `State<` / `bridge::AppState`）。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（AppState 共享态枢纽 / 命令 hub）—— 本模块已零耦合。
- 禁止后端留存输出历史（历史只在前端会话内）。
- 禁止 worker/pump 线程泄漏（须 join 收口）。

## 8. Public / Native Contract

- 内核接口：`term_spawn` / `term_spawn_channel` / `term_kill` / `TermInfo` / `TerminalSession` / `ChannelSink` / `EventSink` / `DropCounter` / `TerminalHandles` / `TermProbe`。
- 既有 `crate::terminal::` 调用点（bridge.rs ×2）经 `main.rs` 顶部 `pub use crate::capabilities::terminal::terminal;` re-export shim 解析，**未逐处改写**。
- TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。

## 9. Security / ACL

- terminal 命令在 `permissions/default-commands.toml` 放行（与 `bridge.ts` / `src/types.ts` 镜像一致）。
- 子进程启动受 `security_policy.rs` 黑名单硬约束（防侧车/解释器注入）。
- 门禁真源：`scripts/check-terminal-policy.py`（扫 bridge.rs + terminal.rs；ACTIVE 码含 TERM_*）。

## 10. Tests

- `terminal.rs` 内 `#[cfg(test)] mod`（line ~751 起：退避 / flush / 历史 replay / 进程组回收）。
- 运行：`cd src-tauri && cargo test capabilities::terminal`（或 `cargo test` 全量）。

## 11. Source of Truth

- 终端内核：`src-tauri/src/capabilities/terminal/terminal.rs`（本模块）。
- 命令体（暂留）：`src-tauri/src/bridge.rs`（`term_spawn_channel` / `terminal_resize` / `terminal_send` / `terminal_kill`）。
- 前端接线：`src/bridge.ts`（`termSpawnChannel` / `terminalResize` / `terminalSend` 等）+ `src/types.ts`（`TermInfo` / `TerminalSession`）+ `src/stores/useTerminalStore.ts` + `workspace/TerminalPane.vue`。
- 门禁真源：`scripts/check-terminal-policy.py`。

## 12. Known Debt

- terminal 命令体仍在 `bridge.rs`（LEGACY_MIXED_MODULE 残核）；属矩阵 §8 末段 bridge 分解计划。
- M3 挂账（非本批）：D23 终端 GUI 实点 / D24 吞吐基线未重采 / D25 on_channel_dead 未 wait / D26 历史未按字符封顶——均属产物质量债，不影响本模块物理边界。

## 13. Extraction Readiness

- **高。** 纯内核、零命令、零 AppState 字段、零 `bridge::AppState` 耦合；跨模块依赖仅 SHARED（`script_runner`） + 标准库 + `portable_pty` / `tauri`。
- 达到阈值后可随 `terminal` 能力提级为 crate `mvp-terminal-rust`（不改领域语义）。
- 同能力下一个低风险同批候选：terminal 命令体 `commands.rs`（需先解除对 `bridge` hub 的耦合，见矩阵 §4 / 协议 §8）。
