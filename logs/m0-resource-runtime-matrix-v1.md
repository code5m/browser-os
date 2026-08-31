# M0-5 运行时资源矩阵（V1.0）

> 生成时间：2026-08-31 11:11 CST
> 检查点：`M0-5.a`
> 执行器：Codex 当前会话；用户已切换强模型；界面完整模型名工具不可见
> 源提交：`68c78d7`
> 证据目录：`logs/m0-baseline/20260831T105149+0800_68c78d7_release_x11/`
> 分析文件：`logs/checkpoints/M0-5.a-analysis-20260831-1111.json`、`logs/checkpoints/M0-5.a-analysis-20260831-1111.md`

## 1. 结论

`M0-5.a = PASS`：当前环境可以真实启动 GUI 并完成应用采样，不需要按 BLOCKED 处理。40-cycle 诊断显示：孤儿进程为 0，RSS 首末不超过 10% 增长，tab 的旧 20-cycle 持续正增长没有升级为明确线性泄漏；但 FD 仍有首末正增长，必须交给 `M0-5.b` 修复或归因后复测。

本结论不是 `M0-5.c` 正式验收。该 run 使用 `M0_RUN_MODE=smoke`、`VR_CYCLE_SAMPLES=40`、`VR_IDLE_SECONDS=5`，用于 M0-5.a 诊断和失败复现；M0-5.c 仍必须回到正式 profile 的 5 次预热 + 20 次测量并归档。

## 2. 源码资源矩阵

| 资源 | 创建点 | 关闭/清理路径 | 运行时 owner | 本轮观测 |
|---|---|---|---|---|
| tab 子 webview | `src-tauri/src/bridge.rs:451` `create_tab` -> `spawn_child_window` | `src-tauri/src/bridge.rs:520` `close_tab`；退出时 `close-tabs` task | Tauri 插件 `TabManagerState` + WebKit 子进程 | 40-cycle driver PASS；关闭态 3 个成员稳定 |
| tab 元数据 | `AppState.tabs` / `child_layouts` / `tab_idle_since` / `hibernated_tabs` | `close_tab` 和 `close-tabs` task 清理 | Rust `AppState` | 元数据清理路径已接入；本轮未见 orphan |
| grid 子进程 | `src-tauri/src/main.rs:655` `create_grid(app, 4)` | `src-tauri/src/grid_process.rs:740` `shutdown_all()` kill+wait | `GridProcessManager.children` | 打开态候选进程数稳定；关闭态 orphan 0 |
| terminal PTY child | `src-tauri/src/main.rs:663` `term_spawn` | `src-tauri/src/bridge.rs:2400` `term_kill` kill+wait | `AppState.terminals` | 40-cycle driver PASS；关闭态 orphan 0；FD +2 |
| terminal reader 线程 | `src-tauri/src/bridge.rs:2341` | child kill 后 reader EOF/Err 自退 | 无句柄，依赖 PTY 生命周期 | 未见 orphan；仍需 M0-5.b 用 FD 目标差分确认 reader/PTY FD 是否完全回收 |
| 后台资源扫描线程 | `src-tauri/src/bridge.rs:747` | `shutdown_requested` 标志退出 | `AppState.shutdown_requested` | 源码已可取消；本轮不是整进程退出压测 |
| 休眠清扫线程 | `src-tauri/src/bridge.rs:2121` | `shutdown_requested` 标志退出 | `AppState.shutdown_requested` | 源码已可取消；最长 60s 退出窗口仍按 M0-2 记录 |
| M0 driver 线程 | `src-tauri/src/main.rs:526` | 测量完成后 `process::exit(code)` | 测量专用线程 | 本轮 app 采样依赖它完成四阶段握手 |
| 主窗关闭 | `src-tauri/src/main.rs:835` | `ShutdownCoordinator.shutdown()` | 统一生命周期核心 | M0-5.a 未做手工关窗；由 M0-6 覆盖 GUI 场景 |

## 3. 40-cycle 运行时矩阵

| 场景 | 正式样本 | RSS 首末 | RSS 总斜率 | RSS 后 20 斜率 | FD 首末 | FD delta | orphan max | 判定 |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| tab | 40 | `412304 -> 410976 KiB`（`-0.32%`） | `+10.06 KiB/cycle` | `+167.19 KiB/cycle` | `102 -> 103` | `+1` | `0` | RSS 未证明硬泄漏；FD 增长待修 |
| grid | 40 | `404880 -> 405484 KiB`（`+0.15%`） | `-1.34 KiB/cycle` | `-0.17 KiB/cycle` | `108 -> 109` | `+1` | `0` | RSS 平台；FD 增长待修 |
| terminal | 40 | `482084 -> 453432 KiB`（`-5.94%`） | `-871.09 KiB/cycle` | `+24.58 KiB/cycle` | `104 -> 106` | `+2` | `0` | RSS 平台；FD 增长待修 |

进程成员拆分：

| 场景 | root RSS/FD 变化 | WebKitNetwork RSS/FD 变化 | WebKitWebProcess RSS/FD 变化 |
|---|---|---|---|
| tab | RSS `185620 -> 183192 KiB`；FD `44 -> 45` | RSS `48484 -> 48492 KiB`；FD `29 -> 29` | RSS `178200 -> 179292 KiB`；FD `29 -> 29` |
| grid | RSS `177968 -> 178148 KiB`；FD `50 -> 50` | RSS `48500 -> 48500 KiB`；FD `29 -> 30` | RSS `178412 -> 178836 KiB`；FD `29 -> 29` |
| terminal | RSS `254620 -> 224112 KiB`；FD `46 -> 47` | RSS `48524 -> 48512 KiB`；FD `29 -> 30` | RSS `178940 -> 180808 KiB`；FD `29 -> 29` |

## 4. 失败复现

复现命令：

```bash
M0_RUN_MODE=smoke VR_CYCLE_SAMPLES=40 VR_IDLE_SECONDS=5 bash scripts/verify-resources.sh
```

关键证据：

- `summary.json`：`status=EXPLORATORY`、`measurements.ok=true`、`worktree_clean_at_start=true`、hooks `READY/READY`。
- `SHA256SUMS`：校验通过。
- `tab_cycle.json`：FD `102 -> 103`，orphan 全部为 0。
- `grid_cycle.json`：FD `108 -> 109`，orphan 全部为 0。
- `terminal_cycle.json`：FD `104 -> 106`，orphan 全部为 0。
- `M0-5.a-analysis-20260831-1111.md`：包含按进程角色拆分的 root/WebKitNetwork/WebKitWebProcess RSS/FD 矩阵。

## 5. M0-5.b 接手口径

`M0-5.b` 不应重复实现 M0-2 的统一关闭核心。下一步先做 FD 目标差分，再按 owner 修复：

1. 给资源采样补 FD target 差分或等价 lsof 输出，明确 `+1/+2` 来自 root、WebKitNetwork、PTY master/slave、socket、pipe 还是 audit/log 文件。
2. 对 tab：优先查 root 侧 `45-44` 的持有者；RSS 只作为复核项，不能先按内存泄漏大改。
3. 对 grid：优先查 WebKitNetwork `30-29` 的持有者，确认是否为平台缓存 FD 还是真实未关闭句柄。
4. 对 terminal：同时查 root `47-46` 与 WebKitNetwork `30-29`；PTY child 已 kill+wait，但 reader/Writer/前端绑定仍需用 FD target 证明闭环。
5. 修复后至少重跑 40-cycle 诊断，要求 FD delta 全部为 0、orphan max 全部为 0，再进入 `M0-5.c` 正式 20-cycle 归档。
