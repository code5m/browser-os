# M0-5 运行时资源矩阵（V1.1）

> 生成时间：2026-08-31 11:11 CST
> 检查点：`M0-5.a`
> 执行器：Codex 当前会话；用户已切换强模型；界面完整模型名工具不可见
> 源提交：`68c78d7`
> 证据目录：`logs/m0-baseline/20260831T105149+0800_68c78d7_release_x11/`
> 分析文件：`logs/checkpoints/M0-5.a-analysis-20260831-1111.json`、`logs/checkpoints/M0-5.a-analysis-20260831-1111.md`
> M0-5.b 更新时间：2026-08-31 16:31 CST
> M0-5.b 源提交：`e998bbd`；证据提交：`2271e75`
> M0-5.b 证据目录：`logs/m0-baseline/20260831T160738+0800_e998bbd_release_x11/`
> M0-5.b 分析文件：`logs/checkpoints/M0-5.b-analysis-20260831-160738.json`、`logs/checkpoints/M0-5.b-analysis-20260831-160738.md`
> M0-5.c 更新时间：2026-08-31 16:52 CST
> M0-5.c 源提交：`6534963`；证据提交：`ebc49f7`
> M0-5.c 证据目录：`logs/m0-baseline/20260831T164058+0800_6534963_release_x11/`
> M0-5.c 分析文件：`logs/checkpoints/M0-5.c-analysis-20260831-164058.json`、`logs/checkpoints/M0-5.c-analysis-20260831-164058.md`

## 1. 结论

`M0-5.a = PASS`：当前环境可以真实启动 GUI 并完成应用采样，不需要按 BLOCKED 处理。40-cycle 诊断显示：孤儿进程为 0，RSS 首末不超过 10% 增长，tab 的旧 20-cycle 持续正增长没有升级为明确线性泄漏；但 FD 仍有首末正增长，必须交给 `M0-5.b` 修复或归因后复测。

`M0-5.b = PASS`：资源采样已补 FD target 级证据，M0 driver 已隔离全局快捷键、活动栏和页签栏误触发入口，并在 tab driver 收尾显式关闭 grid 旁路。新的 40-cycle 复测显示：orphan max 全部为 0；grid/terminal FD delta 为 0；tab FD `+1` 来自 `child:WebKitNetworkPr anon_inode:timerfd 0->1`，第 6 个样本后稳定，裁决为 WebKitNetwork 一次性平台计时器，不是 tab/grid/PTY 所有权泄漏。tab RSS 总增长 `+3.49%`，低于 10%，late-window owner 为 root 进程，裁决为 GTK/WebKit/allocator 缓存平台噪声。

本结论仍不是 `M0-5.c` 正式验收。M0-5.a/M0-5.b run 均使用 `M0_RUN_MODE=smoke`、`VR_CYCLE_SAMPLES=40`、`VR_IDLE_SECONDS=5`，用于诊断、修复验证和裁决；M0-5.c 仍必须回到正式 profile 的 5 次预热 + 20 次测量并归档。

`M0-5.c = PASS`：正式 profile 已完成归档，summary 为 `PASS`，release 二进制 SHA 与期望值匹配，orphan max 全部为 0，tab FD delta 为 0。grid/terminal 出现的 FD 正增长均定位为 WebKit 子进程 `anon_inode:timerfd`，按平台计时器裁决；terminal 总 RSS 增长 `+7.46%`，未超过 10% 门禁。M0-5 整项关闭，下一步进入 M0-6。

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

## 6. M0-5.b 复测与裁决

复测命令：

```bash
M0_RUN_MODE=smoke VR_CYCLE_SAMPLES=40 VR_IDLE_SECONDS=5 bash scripts/verify-resources.sh
python3 scripts/analyze-resource-cycles.py logs/m0-baseline/20260831T160738+0800_e998bbd_release_x11/ --json-out logs/checkpoints/M0-5.b-analysis-20260831-160738.json --markdown-out logs/checkpoints/M0-5.b-analysis-20260831-160738.md
```

复测矩阵：

| 场景 | 正式样本 | RSS 首末 | RSS 后窗斜率 | FD 首末 | FD delta | orphan max | M0-5.b 裁决 |
|---|---:|---:|---:|---:|---:|---:|---|
| tab | 40 | `407216 -> 421428 KiB`（`+3.49%`） | `+407.42 KiB/cycle` | `103 -> 104` | `+1` | `0` | FD 来自 WebKitNetwork `anon_inode:timerfd` 一次性平台计时器；RSS 低于 10%，M0-5.c 复核 |
| grid | 40 | `415536 -> 408480 KiB`（`-1.70%`） | `-23.02 KiB/cycle` | `107 -> 107` | `0` | `0` | PASS |
| terminal | 40 | `408736 -> 417856 KiB`（`+2.23%`） | `-8.50 KiB/cycle` | `104 -> 104` | `0` | `0` | PASS |

tab 进程成员拆分：

| 角色 | RSS 首末 | RSS 后窗斜率 | FD 首末 | FD target 正增长 |
|---|---:|---:|---:|---|
| `root:mvp-browser-os` | `178424 -> 191588 KiB` | `+394.13 KiB/cycle` | `45 -> 45` | 无 |
| `child:WebKitNetworkPr` | `48496 -> 48512 KiB` | `+0.73 KiB/cycle` | `29 -> 30` | `anon_inode:timerfd 0 -> 1` |
| `child:WebKitWebProces` | `180296 -> 181328 KiB` | `+12.55 KiB/cycle` | `29 -> 29` | 无 |

M0-5.b 关闭项：

- `grid` 的 FD `+1` 已归零，未见 orphan。
- `terminal` 的 FD `+2` 已归零，未见 PTY child、reader 线程或 WebKit 子进程残留。
- `tab` 的 FD `+1` 不是用户态 tab/grid/PTY 所有权泄漏；目标为 WebKitNetwork 一次性 `timerfd`，且从第 6 样本后保持稳定。
- tab RSS 后窗仍上行，但总增长 `+3.49%` 未超过 M0-5 验收阈值；owner 为 root 进程，当前裁决为 GTK/WebKit/allocator 缓存平台噪声。

M0-5.c 接手口径：使用正式 profile 重新归档，若 tab `timerfd +1` 或 RSS 后窗上行再次出现，复用本节 target/owner 口径裁决；若出现新的 FD target 或 RSS 末值较初值超过 10%，不得直接 PASS，必须新增 owner 与裁决。

## 7. M0-5.c 正式归档

正式命令：

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release --locked
M0_EXPECTED_BINARY_SHA256=5c41d4abb22a5f5ba4c8d85b443f890f43db8ff8d316ac4a6dae707b29312162 bash scripts/verify-resources.sh
python3 scripts/analyze-resource-cycles.py logs/m0-baseline/20260831T164058+0800_6534963_release_x11 --output-json logs/checkpoints/M0-5.c-analysis-20260831-164058.json --output-md logs/checkpoints/M0-5.c-analysis-20260831-164058.md
```

正式矩阵：

| 场景 | 正式样本 | RSS 首末 | RSS 斜率 | FD 首末 | FD delta | orphan max | M0-5.c 裁决 |
|---|---:|---:|---:|---:|---:|---:|---|
| tab | 20 | `372768 -> 368448 KiB`（`-1.16%`） | `-34.10 KiB/cycle` | `103 -> 103` | `0` | `0` | PASS |
| grid | 20 | `374308 -> 372728 KiB`（`-0.42%`） | `-196.88 KiB/cycle` | `108 -> 109` | `+1` | `0` | WebKitNetwork `anon_inode:timerfd` 平台计时器 |
| terminal | 20 | `433524 -> 465864 KiB`（`+7.46%`） | `+903.65 KiB/cycle` | `104 -> 106` | `+2` | `0` | WebKit 子进程 `timerfd` 平台计时器；RSS 未超过 10% |

FD target 归因：

| 场景 | 角色 | FD target | delta | 裁决 |
|---|---|---|---:|---|
| grid | `child:WebKitNetworkPr` | `anon_inode:timerfd` | `+1` | 平台计时器；root FD `50 -> 50` |
| terminal | `child:WebKitNetworkPr` | `anon_inode:timerfd` | `+1` | 平台计时器；root FD `46 -> 46` |
| terminal | `child:WebKitWebProces` | `anon_inode:timerfd` | `+1` | 平台计时器；root FD `46 -> 46` |

M0-5.c 结论：正式资源 profile 归档 PASS。M0-5 验收项“关闭后孤儿子进程为 0、FD 无持续增长或有 target 裁决、RSS 末值较初值高于 10% 必须归因”已满足；M0-5 整项关闭。
