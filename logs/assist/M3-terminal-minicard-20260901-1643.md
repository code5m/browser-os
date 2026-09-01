# M3 终端增强 强模型最小输入卡（覆盖 M3-1/2/3/4，≤80 行）

> 来源：详细设计与实施计划.md §5 + M3-4.a-assist-20260901-1624.md。仅给 AI:DEEP/R:high，本卡不实现、不签 PASS。

## 目标（M3 总览）
终端增强：mpsc+pump 解耦输出、term_kill 进程树回收、断线指数退避重试、体验项（E1/E2 低风险前端，E3/E4/E5 核心）。

## 必改文件
- `M3-1`：`src-tauri/src/bridge.rs` `m0_term_report`（:2684）→ 改 `mpsc(128)+pump task` 解耦同步 `Channel::send` 阻塞。
- `M3-2`：`bridge.rs` `term_kill`（:804）→ 进程组 `kill`（`process_group(0)`+SIGKILL）防 orphan；复用 M0-2 `kill-terminals` 范式。
- `M3-3`：`bridge.rs` 断线处理 → 指数退避重试 `RETRY_MAX_BACKOFF_MS=30_000`。
- `M3-4`：`src/stores/useSystemStore.ts` E1 临时历史上限 `TEMPORARY_HISTORY_LIMIT=40`；`TerminalPane.vue` E2 resize 静默 `TERMINAL_RESIZE_OUTPUT_QUIET_MS`（前端，低风险）。

## 禁止事项
- 不触 PTY spawn/kill 生命周期语义（M0-2 已定，M3 仅复核）。
- E1/E2 不改 `bridge.rs` 命令签名、不改 PTY、不改 `term_kill`。
- 不新建退出线程（回收须同步进 ShutdownCoordinator）。
- 不改性能基线 >10%（§9 纪律）。
- 不移动 `NEXT`。

## 实现要点
1. M3-1：输出经 `mpsc(128)` 缓冲 + 独立 pump task 转发事件，解耦前端背压（对标 fileterm）。
2. M3-2：`child.kill()` 前确保 `process_group(0)`，孙进程全回收；审计 `script_killed` 类。
3. M3-3：断线后指数退避（base*2^n，封顶 30s）重试；可取消（M0-2 协调器）。
4. M3-4：E1 仅 store 缓冲裁剪（不落盘）；E2 仅 ResizeObserver 防抖（不改 PTY）。

## 验收命令
```
cargo build --release
npm run build
bash scripts/pre-merge.sh                   # ALL_PASS
# 手动：大输出不卡 UI（M3-1）；cancel 后 ps 无 orphan（M3-2）；断网重连退避（M3-3）
# 前端：连续>40 行 termLines<=40（E1）；拖动窗口无抖动重渲（E2）
```

## 反向用例
- M3-1：10MB 输出 → UI 不卡（mpsc 缓冲生效）。
- M3-2：脚本 spawn 孙进程后 kill → `ps` 验证进程组全回收。
- M3-3：连续断线 → 退避间隔递增且封顶 30s，可中途取消。
- E1：termLines 超 40 自动裁剪，不崩。
- E2：拖动窗口 `termResize` 去抖，PTY 不重建。

## 失败动作
- M3-1 仍卡 → 确认 pump task 独立于 UI 线程，mpsc 容量合理。
- M3-2 orphan → 补 `process_group(0)`，复测。
- M3-3 退避异常 → 校验封顶常量，复测可取消。
- E1/E2 误改核心 → 回退仅前端/store 局部改动。
