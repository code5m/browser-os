# M0-2.a 资源所有权表（现状冻结，V1.0）

> 冻结时间：2026-08-30 ｜ 冻结提交：`5872bc5`+（本检查点提交前） ｜ 契约：`logs/m0-baseline-contract-v1.md`
> 范围：只盘点**现状所有权与缺口**，不实现 `ShutdownCoordinator`（属 M0-2.b），不迁移调用方（属 M0-2.c）。
> 机器夹具：`scripts/check-lifecycle-contract.py`（默认模式预期 `EXIT=1`，M0-2.b/c 关闭缺口后转 `EXIT=0`）。

## 1. 所有权总表

| # | 资源 | 创建点 | 当前所有者 | 正常清理路径 | 窗口关闭覆盖 | 系统退出覆盖 | 单资源失败降级 | 现状 GAP |
|---|------|--------|-----------|-------------|-------------|-------------|---------------|---------|
| R1 | 主窗口 / `AppHandle` | `main.rs` `tauri::Builder` | Tauri 运行时 | Tauri 自动销毁 | ✅ | ❌ 无 `RunEvent` 闭包 | — | **GAP3** |
| R2 | tab 子 webview | `bridge.rs::tab_new`（插件 `TabManagerState`） | 插件 `TabManagerState` | `close_tab` → `manager.close_tab()` | ❌ 仅关 grid | ❌ | ❌ `?` 短路 | **GAP5** |
| R3 | tab 元数据（`child_layouts`/`last_position_at`/`tab_idle_since`/`hibernated_tabs`/`tabs`/`active_tab`） | `AppState` 六个 `Mutex` | `AppState` | `close_tab` 内 `?` **之后**逐项 remove | ❌ | ❌ | ❌ webview 关闭失败即整体泄漏 | **GAP5** |
| R4 | grid 子进程 | `grid_manager.get_or_spawn()` | `GridManager.children` | `shutdown_all()`：`kill()`+`wait()` | ⚠️ 旁路直调 | ❌ | ✅ `let _ =` 吞错继续 | **GAP2/GAP3** |
| R5 | grid 部分创建残留 | `create_grid` 循环内 `get_or_spawn(i)?` → `request(...)?` | 无 | **无回滚**：第 i 格失败时 0..i-1 已 spawn 的进程继续存活 | ❌ | ❌ | ❌ | **GAP4** |
| R6 | grid 子 webview | 子进程内 `GridCmd::CreateTab` | 子进程 | `shutdown_all` → 进程销毁 | ⚠️ | ❌ | — | **GAP2** |
| R7 | PTY 会话（`writer`+`child`） | `bridge.rs::term_spawn` | `AppState.terminals` | `term_kill`：`child.kill()` **无 `wait()`** | ❌ | ❌ | ✅ 不存在即 `Ok(())` | **GAP6**（僵尸进程） |
| R8 | PTY reader 线程 | `term_spawn` 内 `thread::spawn` | 无句柄 | 被动：读到 `Ok(0)/Err` 才退出 | ❌ | ❌ | ✅ 读失败自退 | **GAP6**（无 join） |
| R9 | 后台线程 · layout enforcer | `bridge.rs:423` `thread::spawn(loop)` | 无句柄 | **无** | ❌ | ❌ | ✅ sleep 400ms | **GAP7** |
| R10 | 后台线程 · resource scanner | `bridge.rs:602` `thread::spawn(loop)` | 无句柄（`swap(true)` 防重复） | **无** | ❌ | ❌ | ✅ | **GAP7** |
| R11 | 后台线程 · hibernation sweeper | `bridge.rs:1856` `thread::spawn(loop)` | 无句柄 | **无** | ❌ | ❌ | ✅ sleep 60s | **GAP7** |
| R12 | 前端 xterm 实例 + FitAddon | `TerminalPane.vue` `onMounted` | Vue 组件 | `onBeforeUnmount` → `term.dispose()` | ✅ | ✅ | ✅ try/catch | 无（**唯一闭环项**） |
| R13 | 前端 `ResizeObserver` | `TerminalPane.vue:152` | Vue 组件 | `onBeforeUnmount` → `disconnect()` | ✅ | ✅ | — | 无 |
| R14 | 前端 rAF 帧采样循环 | `TerminalPane.vue:43-50` | 模块级 `m0FrameSampling` | 仅 `m0ReportAfterPaint` 置 false | ❌ 卸载不置 false | ❌ | ❌ 卸载后循环续跑 | **FE-1**（见 §3） |
| R15 | 前端 store 回调绑定 | `system.bindTermWriter` / `bindM0ThroughputStart` | `useSystemStore` | `onBeforeUnmount` 置 null | ✅ | ✅ | — | 无 |
| R16 | 统一退出核心 | — | **不存在** | — | — | — | — | **GAP1** |

## 2. 七个机器可检 GAP（夹具逐条对应源码）

| GAP | 源码证据 | 危害 | 关闭检查点 |
|-----|---------|------|-----------|
| `NO_UNIFIED_SHUTDOWN_CORE` | 全仓库无 `ShutdownCoordinator` | 退出逻辑分散，M1~M5 各命令会重复实现 | M0-2.b |
| `WINDOW_CLOSE_BYPASSES_UNIFIED_CORE` | `main.rs:811-813` `WindowEvent::CloseRequested => state.grid_manager.shutdown_all();` | 窗口关闭只收 grid，tab/PTY/线程全不收；且是核心之外的旁路 | M0-2.c |
| `SYSTEM_EXIT_HOOK_MISSING` | `main.rs` 末尾直接 `.run(tauri::generate_context!())`，**无 `RunEvent` 闭包**（无 `ExitRequested`/`Exit`/`cleanup_before_exit`） | 托盘退出、系统关机、`Cmd+Q`、Ctrl-C 均不触发任何清理 → 孤儿进程/FD 泄漏 | M0-2.c |
| `GRID_PARTIAL_CREATE_ROLLBACK_MISSING` | `create_grid` 循环：`mgr.get_or_spawn(index)?` 后 `mgr.request(...)?;`，无 try/rollback | 第 i 格启动失败时 0..i-1 子进程泄漏（契约 §5 `orphan_process_count` 风险来源） | M0-2.b/c |
| `TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP` | `bridge.rs:519` `close_tab`：`manager.close_tab(&id)...?;` 之后才 remove 六项元数据 | webview 关闭失败 → 元数据残留，layout enforcer 继续操作已销毁 webview | M0-2.b/c |
| `TERMINAL_KILL_NOT_WAITED` | `term_kill`：`let _ = s.child.kill();` 无 `wait()` | PTY 子进程变僵尸；reader 线程无 join | M0-2.b/c |
| `BACKGROUND_WORKERS_NOT_CANCELLABLE` | `bridge.rs:423/602/1856` 三处 `thread::spawn(move || loop`，无 `shutdown_requested`/`stop_token`/`cancel_token`/`is_cancelled` | 进程无法优雅退出，退出期与 `AppState` 锁竞争 | M0-2.b |

## 3. 未纳入机器夹具的缺口（前端，FE 前缀）

> 说明：任务卡 `PASS_CRITERIA` 固定默认模式输出**7 个** GAP；前端缺口只在此表冻结，不进夹具，避免超出验收口径。

- **FE-1**：`TerminalPane.vue:43-50` 的 rAF 采样循环以模块级 `m0FrameSampling` 为开关，`onBeforeUnmount`（166-171）未置 `false`。测量模式下若组件在采样中卸载，循环会持续调度至 `m0ReportAfterPaint` 被调用为止。归 M0-2.c（与前端退出协议一并处理）。
- **FE-2**：`term?.dispose()` 只销毁前端渲染器，不通知 Rust 侧 `term_kill`；PTY 会话所有权仍在 Rust（`AppState.terminals`）。前端卸载 ≠ 终端关闭，属所有权跨层分裂，归 M0-2.c 明确归属。

## 4. 判定口径

- `scripts/check-lifecycle-contract.py` 默认模式在当前提交**必须** `EXIT=1` 并列出上述 7 个 GAP；这是**现状夹具的预期结果**，不是门禁回归。
- `--expect-current-gaps` 用于 CI/pre-merge：缺口集合与本文一致则 `EXIT=0`；一旦 M0-2.b/c 修好某一项，本模式会转 `EXIT=1` 提示需同步更新本文与 `EXPECTED_GAPS`。
- 缺口全部关闭后，默认模式转 `EXIT=0`，M0-2 验收项「所有退出路径调用同一核心」方可判定。
