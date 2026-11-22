# M0-2 资源所有权表（V1.4）

> 冻结时间：2026-08-30 ｜ 起始提交：`5872bc5` ｜ 契约：`logs/m0-baseline-contract-v1.md`
> 范围：盘点**现状所有权与缺口**；核心实现属 M0-2.b，调用方迁移属 M0-2.c。
> 机器夹具：`scripts/check-lifecycle-contract.py`（默认模式预期 `EXIT=1`，缺口清零后转 `EXIT=0`）。
>
> **V1.1（M0-2.b）**：`ShutdownCoordinator` 已落地（`src-tauri/src/shutdown.rs`，6 个生命周期测试通过），
> `NO_UNIFIED_SHUTDOWN_CORE` 已关闭并从夹具 `EXPECTED_GAPS` 移除，机器可检缺口 7 → **6**。
> R16 行状态更新为「核心已存在、尚无调用方」。其余缺口待 M0-2.c 迁移后复检。
>
> **V1.3（M0-2.d）**：M0-2.c 迁移完成后机器可检缺口 6 → **0**（默认模式 `PASS`）。
> 本版新增 §5「测试覆盖矩阵」，把 15 项 Rust 生命周期测试与历史 GAP 一一对应；
> 静态夹具补 6 个单缺口防回归用例（此前只在组合 fixture 中一起检出，检测器失效会被掩盖）。
>
> **V1.2（M0-2.c）**：主窗口关闭与 `RunEvent::ExitRequested` 已迁移到 `ShutdownCoordinator`；
> 已注册 stop-background-workers、close-tabs、kill-terminals、shutdown-grid 四类真实清理任务；
> 当时的批量退出路径在 tab 关闭失败后继续清理并汇总错误，PTY kill 后 wait，grid 创建失败回滚已创建子进程。
>
> **V1.4（2026-11-22）**：区分可恢复的普通关闭与最终退出。普通 `close_tab` 只有原生关闭成功后才删除 Owner 元数据/草稿，失败保留记录供重试；最终 `close-tabs` task 继续尝试全部子 WebView 并汇总错误。静态扫描必须跟随物理 bridge Owner 与 async 的实际阻塞委派，不能因函数迁出 facade 而停止检查。
> `scripts/check-lifecycle-contract.py` 默认模式现为 `EXIT=0`，机器可检缺口 **6 → 0**。

## 1. 所有权总表

| # | 资源 | 创建点 | 当前所有者 | 正常清理路径 | 窗口关闭覆盖 | 系统退出覆盖 | 单资源失败降级 | 现状 GAP |
|---|------|--------|-----------|-------------|-------------|-------------|---------------|---------|
| R1 | 主窗口 / `AppHandle` | `main.rs` `tauri::Builder` | Tauri 运行时 | Tauri 自动销毁 | ✅ 走 `ShutdownCoordinator` | ✅ `RunEvent::ExitRequested` | — | 已关闭 |
| R2 | tab 子 webview | Browser 创建入口与插件 `TabManagerState` | 插件 `TabManagerState` | 普通 `close_tab` 成功后提交清理；最终 `close-tabs` task 遍历真实插件 id | ✅ | ✅ | ✅ 普通失败保留 Owner 供重试；退出批量继续并汇总 | 已关闭 |
| R3 | tab 元数据与会话草稿 | `AppState` 对应 Owner 集合 | `AppState` | 普通关闭仅在原生成功后删除；最终退出由统一 task 清理 | ✅ | ✅ | ✅ 普通失败不得先删元数据或草稿，避免失去仍存活 WebView 的跟踪 | 已关闭 |
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
| ~~`TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP`~~ | 最终 `close-tabs` task 的错误提前返回 | 后续 WebView 未尝试关闭、批量清理中断 | ✅ V1.4 明确约束退出批次；普通关闭必须先传播原生失败以保留 Owner |
| ~~`TERMINAL_KILL_NOT_WAITED`~~ | ~~`term_kill` kill 后不 wait~~ | ~~PTY 子进程可能僵尸~~ | ✅ **M0-2.c 已关闭** |
| ~~`BACKGROUND_WORKERS_NOT_CANCELLABLE`~~ | ~~三个 `thread::spawn(move || loop)` 无取消信号~~ | ~~后台线程退出期继续触碰 AppState~~ | ✅ **M0-2.c 已关闭** |

## 3. 未纳入机器夹具的缺口（前端，FE 前缀）

> 说明：以下为 M0-2.a 冻结过、但未纳入机器夹具的前端项；M0-2.c 后重新裁决。

- **FE-1 已关闭**：`TerminalPane.vue` 的 `onBeforeUnmount` 已将 `m0FrameSampling=false`，组件卸载后不会继续调度 rAF 采样。
- **FE-2 已裁决**：`term?.dispose()` 只销毁前端渲染器；Rust PTY 的唯一 owner 仍为 `AppState.terminals`。用户点击“隐藏”不应强杀 shell，应用退出时由 `kill-terminals` task 统一 kill+wait。若产品后续要求“隐藏即关闭”，需另开 UX 行为变更，不作为 M0-2 生命周期缺口。

## 4. 判定口径

- `scripts/check-lifecycle-contract.py` 默认模式在 M0-2.c 后必须 `EXIT=0`，输出 `LIFECYCLE_CONTRACT_RESULT=PASS`。
- `--expect-current-gaps` 仍用于 CI/pre-merge：当前 `EXPECTED_GAPS=()`，若新增或复发机器可检 gap，本模式转 `EXIT=1`。
- M0-2.c 关闭的是机器可检生命周期缺口；M0-2.d 已补重复退出、半初始化退出和失败降级的回归测试（见 §5）。

## 5. 测试覆盖矩阵（M0-2.d）

| 历史 GAP | 静态夹具 | Rust 测试 |
|---------|---------|----------|
| `NO_UNIFIED_SHUTDOWN_CORE` | `ShutdownCoordinator` 存在性 | 全部 15 项（核心为被测对象） |
| `WINDOW_CLOSE_BYPASSES_UNIFIED_CORE` | 单缺口用例 | `dual_exit_paths_run_cleanup_only_once`、`concurrent_exit_paths_run_cleanup_only_once` |
| `SYSTEM_EXIT_HOOK_MISSING` | 单缺口用例 | 同上（第二条入口） |
| `GRID_PARTIAL_CREATE_ROLLBACK_MISSING` | 源码模式（回滚分支存在性） | `shutdown_during_partial_registration_runs_only_registered_tasks`（半初始化语义等价） |
| `TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP` | 源码模式 | `grid_cleanup_failure_does_not_block_terminals_and_tabs` |
| `TERMINAL_KILL_NOT_WAITED` | 源码模式（`kill`+`wait` 成对） | `all_cleanups_failing_still_reports_every_task` |
| `BACKGROUND_WORKERS_NOT_CANCELLABLE` | 源码模式（取消信号） | `panicking_cleanup_does_not_block_remaining_tasks` |

> 说明：静态夹具只能证源码形态，行为语义（重复退出只执行一次、失败不重试、panic 隔离）由 Rust 测试保证。
> 二者互补，缺一不可——M0-2.b 曾出现「形态合规但语义错误」的 bug，只有行为测试能抓到。

## 6. V1.4 回归边界

`check-lifecycle-contract.py` 的当前 `EXPECTED_GAPS` 仍为空。它分别检查普通关闭的成功提交屏障与最终退出的错误汇总，不能把普通关闭失败后的状态保留当作应删除的残留。新增的反例包括忽略普通关闭错误、删除关闭 Owner/退出 task、任意单个后台线程缺取消信号，以及迁入 `grid_runtime.rs` 后阻塞实现重新出现无回滚的创建。

扫描读取 `bridge.rs` 及 `grid_runtime`、`terminal_commands`、`browser_commands`、`resource_commands`、`tab_runtime` 五个实际 Owner；`create_grid`/`term_kill` 仅在 wrapper 真正调用时追踪 `_blocking`/`_sync`，缺失文件或实现不能被视为无缺陷。正例保留现有取消/失败重试/元数据提交时序。静态夹具是形态约束，原生进程回收、WebView 关闭和重入行为仍由相应 Rust 与安装版测试证明。
