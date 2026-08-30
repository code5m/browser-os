# M0-2 资源所有权表（V1.2）

> 冻结时间：2026-08-30 ｜ 起始提交：`5872bc5` ｜ 契约：`logs/m0-baseline-contract-v1.md`
> 范围：盘点**现状所有权与缺口**；核心实现属 M0-2.b，调用方迁移属 M0-2.c。
> 机器夹具：`scripts/check-lifecycle-contract.py`（默认模式预期 `EXIT=1`，缺口清零后转 `EXIT=0`）。
>
> **V1.1（M0-2.b）**：`ShutdownCoordinator` 已落地（`src-tauri/src/shutdown.rs`，6 个生命周期测试通过），
> `NO_UNIFIED_SHUTDOWN_CORE` 已关闭并从夹具 `EXPECTED_GAPS` 移除，机器可检缺口 7 → **6**。
> R16 行状态更新为「核心已存在、尚无调用方」。其余缺口待 M0-2.c 迁移后复检。
>
> **V1.2（M0-2.c）**：主窗口关闭与 `RunEvent::ExitRequested` 已迁移到 `ShutdownCoordinator`；
> 已注册 stop-background-workers、close-tabs、kill-terminals、shutdown-grid 四类真实清理任务；
> tab 关闭失败后仍清理元数据，PTY kill 后 wait，grid 创建失败回滚已创建子进程。
> `scripts/check-lifecycle-contract.py` 默认模式现为 `EXIT=0`，机器可检缺口 **6 → 0**。

## 1. 所有权总表

| # | 资源 | 创建点 | 当前所有者 | 正常清理路径 | 窗口关闭覆盖 | 系统退出覆盖 | 单资源失败降级 | 现状 GAP |
|---|------|--------|-----------|-------------|-------------|-------------|---------------|---------|
| R1 | 主窗口 / `AppHandle` | `main.rs` `tauri::Builder` | Tauri 运行时 | Tauri 自动销毁 | ✅ 走 `ShutdownCoordinator` | ✅ `RunEvent::ExitRequested` | — | 已关闭 |
| R2 | tab 子 webview | `bridge.rs::tab_new`（插件 `TabManagerState`） | 插件 `TabManagerState` | `close_tab` / `close-tabs` task → `manager.close_tab()` | ✅ | ✅ | ✅ 失败仍继续元数据清理并汇总错误 | 已关闭 |
| R3 | tab 元数据（`child_layouts`/`last_position_at`/`tab_idle_since`/`hibernated_tabs`/`tabs`/`active_tab`） | `AppState` 六个 `Mutex` | `AppState` | `close_tab` 与 `close-tabs` task 清理 | ✅ | ✅ | ✅ webview 关闭失败不阻断元数据清理 | 已关闭 |
| R4 | grid 子进程 | `grid_manager.get_or_spawn()` | `GridManager.children` | `shutdown-grid` task → `shutdown_all()`：`kill()`+`wait()` | ✅ | ✅ | ✅ `shutdown_all` 继续处理全部 child | 已关闭 |
| R5 | grid 部分创建残留 | `create_grid` 循环内 `get_or_spawn(i)` / `request(...)` | `GridManager.children` | 失败时反向 `kill_child` 回滚已创建子进程 | ✅ | ✅ | ✅ | 已关闭 |
| R6 | grid 子 webview | 子进程内 `GridCmd::CreateTab` | 子进程 | `shutdown-grid` task → 进程销毁 | ✅ | ✅ | — | 已关闭 |
| R7 | PTY 会话（`writer`+`child`） | `bridge.rs::term_spawn` | `AppState.terminals` | `term_kill` / `kill-terminals` task：`child.kill()` + `wait()` | ✅ | ✅ | ✅ 不存在即 `Ok(())`；批量清理汇总错误 | 已关闭 |
| R8 | PTY reader 线程 | `term_spawn` 内 `thread::spawn` | 无句柄 | child kill 后 reader 读到 EOF/Err 自退 | ✅ | ✅ | ✅ 读失败自退 | 已关闭 |
| R9 | 后台线程 · layout enforcer | `bridge.rs:423` `thread::spawn(loop)` | shutdown 标志 | `stop-background-workers` task 设置 `shutdown_requested` 后自然退出 | ✅ | ✅ | ✅ sleep 400ms 后退出 | 已关闭 |
| R10 | 后台线程 · resource scanner | `bridge.rs:602` `thread::spawn(loop)` | shutdown 标志（`swap(true)` 防重复） | `shutdown_requested` 后自然退出 | ✅ | ✅ | ✅ | 已关闭 |
| R11 | 后台线程 · hibernation sweeper | `bridge.rs:1856` `thread::spawn(loop)` | shutdown 标志 | `shutdown_requested` 后自然退出 | ✅ | ✅ | ✅ 最多 sleep 60s 后退出 | 已关闭 |
| R12 | 前端 xterm 实例 + FitAddon | `TerminalPane.vue` `onMounted` | Vue 组件 | `onBeforeUnmount` → `term.dispose()` | ✅ | ✅ | ✅ try/catch | 无（**唯一闭环项**） |
| R13 | 前端 `ResizeObserver` | `TerminalPane.vue:152` | Vue 组件 | `onBeforeUnmount` → `disconnect()` | ✅ | ✅ | — | 无 |
| R14 | 前端 rAF 帧采样循环 | `TerminalPane.vue:43-50` | 模块级 `m0FrameSampling` | `m0ReportAfterPaint` 或组件卸载置 false | ✅ | ✅ | ✅ 卸载后停止下一帧调度 | 已关闭 |
| R15 | 前端 store 回调绑定 | `system.bindTermWriter` / `bindM0ThroughputStart` | `useSystemStore` | `onBeforeUnmount` 置 null | ✅ | ✅ | — | 无 |
| R16 | 统一退出核心 | `main.rs` `.manage(shutdown::ShutdownCoordinator::new())` | `ShutdownCoordinator` | `shutdown()` 幂等执行已注册任务 | ✅ | ✅ | ✅ 失败/panic 均隔离 | 已关闭 |

## 2. 机器可检 GAP（夹具逐条对应源码）

| GAP | 源码证据 | 危害 | 关闭检查点 |
|-----|---------|------|-----------|
| ~~`NO_UNIFIED_SHUTDOWN_CORE`~~ | ~~全仓库无 `ShutdownCoordinator`~~ | ~~退出逻辑分散~~ | ✅ **M0-2.b 已关闭**（`src-tauri/src/shutdown.rs`，6 测试通过；已从夹具 `EXPECTED_GAPS` 移除） |
| ~~`WINDOW_CLOSE_BYPASSES_UNIFIED_CORE`~~ | ~~窗口关闭直调 `grid_manager.shutdown_all()`~~ | ~~tab/PTY/线程不收；核心之外旁路~~ | ✅ **M0-2.c 已关闭** |
| ~~`SYSTEM_EXIT_HOOK_MISSING`~~ | ~~主进程无 `RunEvent::ExitRequested` 闭包~~ | ~~系统退出不触发清理~~ | ✅ **M0-2.c 已关闭** |
| ~~`GRID_PARTIAL_CREATE_ROLLBACK_MISSING`~~ | ~~`create_grid` 中 `get_or_spawn` / `request` 失败无回滚~~ | ~~部分子进程残留~~ | ✅ **M0-2.c 已关闭** |
| ~~`TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP`~~ | ~~`close_tab` 的 `?` 短路早于元数据清理~~ | ~~webview 关闭失败导致元数据残留~~ | ✅ **M0-2.c 已关闭** |
| ~~`TERMINAL_KILL_NOT_WAITED`~~ | ~~`term_kill` kill 后不 wait~~ | ~~PTY 子进程可能僵尸~~ | ✅ **M0-2.c 已关闭** |
| ~~`BACKGROUND_WORKERS_NOT_CANCELLABLE`~~ | ~~三个 `thread::spawn(move || loop)` 无取消信号~~ | ~~后台线程退出期继续触碰 AppState~~ | ✅ **M0-2.c 已关闭** |

## 3. 未纳入机器夹具的缺口（前端，FE 前缀）

> 说明：以下为 M0-2.a 冻结过、但未纳入机器夹具的前端项；M0-2.c 后重新裁决。

- **FE-1 已关闭**：`TerminalPane.vue` 的 `onBeforeUnmount` 已将 `m0FrameSampling=false`，组件卸载后不会继续调度 rAF 采样。
- **FE-2 已裁决**：`term?.dispose()` 只销毁前端渲染器；Rust PTY 的唯一 owner 仍为 `AppState.terminals`。用户点击“隐藏”不应强杀 shell，应用退出时由 `kill-terminals` task 统一 kill+wait。若产品后续要求“隐藏即关闭”，需另开 UX 行为变更，不作为 M0-2 生命周期缺口。

## 4. 判定口径

- `scripts/check-lifecycle-contract.py` 默认模式在 M0-2.c 后必须 `EXIT=0`，输出 `LIFECYCLE_CONTRACT_RESULT=PASS`。
- `--expect-current-gaps` 仍用于 CI/pre-merge：当前 `EXPECTED_GAPS=()`，若新增或复发机器可检 gap，本模式转 `EXIT=1`。
- M0-2.c 关闭的是机器可检生命周期缺口；M0-2.d 仍需补重复退出、半初始化退出和失败降级的更完整回归测试。
