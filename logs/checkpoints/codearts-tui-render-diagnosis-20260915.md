# CodeArts TUI 渲染排查实录（2026-09-15）

## 结论（TL;DR）

**根因不在终端渲染链路。** CodeArts CLI 26.8.12 启动流程被其**更新确认对话框**阻塞：

```
发现新版本，是否更新…
当前版本: 26.8.12 → 26.9.3
是否继续启动旧版本？  ● Yes / ○ No
```

- 对话框在打印 True Color 兼容性提示之后弹出（stdout+stderr 混合输出，stderr 含 `ESC[2J ESC[3J ESC[H` 清屏序列），**等待输入期间输出完全停止**——屏幕上停留在 True Color 提示，即为"卡住"。
- 更新检查走网络；**网络失败路径下 CLI 会静默退出**（实测 Run1 无输入约 2 分钟后进程消失），此时按回车无效（进程已死，只是 shell 提示符重绘）。
- 按 Enter 放行后（Run3）：**+33KB TUI 洪流、进程存活、用户目检 TUI 真实显示**。

渲染链路五层对账**全程恒等**：

```
pty.read == flush == chan.send == fe.recv(+2B) == xterm.done(+2B)
```

CodeArts TUI 洪流（24-bit 色、`ESC[?2026h` 同步输出、光标寻址、alternate buffer）下：
**零丢帧、零 queue_full、零 send_fail、零 utf8_valid=false、零 U+FFFD 替换、零 resize 事件**。
唯一 2 字节差 = 首帧在 Channel handler 接线前发出的启动竞态（全程恒定，不增长）。

## 实验对账表（真实客户端 probe，窗口 164×48）

| 实验 | PTY bytes（增量） | xterm.done（增量） | queue drop | UTF-8 异常 | alternate |
|---|---|---|---|---|---|
| A 普通 shell | 1,052 | 1,052 | 0 | 0 | — |
| B vim | 17,659 | 17,659 | 0 | 0 | ✅ |
| C nano（htop 未安装） | 686 | 686 | 0 | 0 | — |
| D CodeArts（Enter 后） | 39,991 | 39,991 | 0 | 0 | ✅ |

对照实验结论：**vim/nano/shell 全部正常 → 通用 TUI 渲染健康**；CodeArts 差异不在 ANSI 序列处理，在其自身启动流程。

## 已排除的假说（前序 + 本轮）

| 假说 | 判定 | 证据 |
|---|---|---|
| True Color 环境变量 | 排除（既有） | env 已生效 |
| CodeArts 等待回车确认 | 部分相关 | 等的是**更新对话框**，非 True Color 提示 |
| 自动确认 autoConfirmCli | **排除且证伪** | 其匹配模式（兼容性提示+按回车继续）在真实输出中不存在，从未触发；已从工作区剥离暂存（死代码，见下） |
| 8KB Channel 消息卡住 | 排除（既有） | 分块对照无效 |
| 队列丢帧 | 排除 | queue_full=0、drop=0 |
| Tauri Channel 丢数据 | 排除 | chan.send==fe.recv（-2B 启动竞态） |
| UTF-8 边界 lossy 替换 | 排除 | utf8_valid=true × 全部帧、replacement_count=0 |
| xterm 解析/渲染卡死 | 排除 | xterm.done 全量完成、buf=alternate |
| resize/SIGWINCH 异常 | 排除 | CodeArts 期间零 resize 事件（0×0 事件仅发生在早期视图切换且有守卫） |
| WebKitGTK 绘制层 | **排除（用户目检）** | TUI 真实显示 |

## 证据文件

- probe 快照：`/tmp/probe-{baseline,A-shell,B-vim,C-nano,D2-codearts-tui-alive}.log`（临时目录，重启即失；关键数字已录入本文件）
- CodeArts stderr 捕获：`/tmp/ca-err.log`（11B 清屏序列）
- 受控 PTY 完整会话：`/tmp/ca-full.log`（18.8KB，dialog→Enter→完整 TUI）

## 资产清单（本次产出）

1. **五层对账 probe**（已提交 `22a211d`，tag `diag/codearts-tui-render-20260915`）：
   - `src-tauri/src/terminal.rs`：probe_enabled() + TermProbe（PTY read/enq/drop/queue_full/水位、flush raw_bytes/utf8_valid/replacement_count/enc_bytes、chan.send ok/fail、summary 汇总行）+ `pty.openpty` 初始尺寸 + TERM_PROGRAM/CLICOLOR_FORCE env。
   - `src-tauri/src/bridge.rs`：`term.resize` 请求值与结果打点。
   - `src/bridge.ts`：`termSpawnChannel` 响应携带 `probe` 开关。
   - `src/stores/useSystemStore.ts`：`fe.recv` 层（seq/UTF-8 bytes/累计/去向 sink=xterm|buffer|exit）。
   - `src/components/system/TerminalPane.vue`：`xterm.write`/`xterm.done` 成对打点（bytes/累计/cols/rows/buf 类型）、`resize.evt`（容器 cw/ch/proposed）、`termResize.req`、mounted/unmounted 终值。
   - 开关：`MVP_TERMINAL_PROBE=1`（进程级 OnceLock 缓存），默认关闭，只记元数据。
2. **autoConfirmCli 死代码已剥离出暂存集**（保留在工作树未提交部分）：本次排查证伪——它针对"True Color 提示等回车"的模型是错误的，真凶是更新对话框。建议后续 lane 整体移除（checkbox UI + maybeAutoConfirmCli + localStorage 键）。
3. **2 字节启动竞态**（新发现小坑）：`term_spawn_channel` 返回前管道已启动，首帧可能在前端 Channel onmessage 接线前发出被丢弃。影响仅限首帧（shell 初始提示符），待后续 wave 评估修复（spawn 响应返回后再启动 pump，或前端 replay 补首帧）。

## 方法论沉淀（高复用）

排查"全屏 TUI 在某终端卡住"类问题的标准打法：

1. **先字节对账，再谈渲染**：`PTY → 队列 → flush(utf8) → Channel → 前端接收 → xterm.write/done` 六层 cumulative 恒等式，一分钟内定位断点层。xterm.write 的 callback = 解析完成（非绘制完成），绘制需目检或截图。
2. **受控 PTY 对照**：`script -qec 'cmd; echo EXIT=$?' log` + 管道喂输入，可完整读取会话内容（不受 probe 只记元数据限制），判断 CLI 本身行为（对话框/退出码）。
3. **GUI 自动化在本机（GNOME Wayland + XWayland）的坑**：
   - `xdotool type --window` / `key --window` 走 XSendEvent 合成事件，WebKitGTK **忽略**；必须用 XTEST（不带 --window）+ 真实焦点。
   - XTEST 点击能送达但不迁移焦点；`xdotool windowfocus` 可设 X 输入焦点，但会被 IDE/合成器抢回 → **每次输入前重新 windowfocus**。
   - `xdotool mousemove --window` 相对坐标在混合 DPI 双屏（4K@200% + 1080p@100%，窗口落在第二屏时）与预期不同，用 `getmouselocation` 实测映射再点击。
   - `/dev/uinput` 无权限（非 root 不可硬件级注入）；TIOCSTI 被 kernel 7.0 + `dev.tty.legacy_tiocsti=0` 封锁。
   - 截图：`xwd -id <win>` 可抓 XWayland 窗口（PIL 不认新版 XWD 时需其他转换器）；GNOME Wayland 原生截图/录屏被 AccessDenied。
4. **进程真相三件套**：`ps --ppid <shell_pid>`（前台进程是否还在）、`/proc/<pid>/io` 的 wchar（实际写出量 vs PTY 交付量）、退出码捕获（`cmd; echo $? > file`）。
5. **stdin 非 TTY 的 CLI 会走降级文本路径**（对话框变纯文本等待），与 TTY 路径行为不同——对照实验要控制 TTY 一致性。
6. **别急着升级依赖**：xterm/Tauri/WebKitGTK 升级可能"碰巧修好"但掩盖根因、弄坏其他链路。

## 后续行动建议（Phase 2 才动产品代码）

- 产品侧：CodeArts 的更新对话框不是本客户端能修的；可选：① 引导用户在对话框按 Enter；② 升级 CodeArts 到 26.9.3；③ 查 CLI 是否支持关闭自动更新检查（`codearts --help` / 配置项）。
- 客户端侧可做的最小修复：2 字节启动竞态（spawn 响应后再启 pump）。
- autoConfirmCli 死代码移除（见上）。
- probe 可在确认无需复测后移除（`git revert 22a211d` 或按需保留——开关默认关闭、零开销）。

## 状态

STATUS: ROOT_CAUSE_FOUND（用户已目检 TUI 真实显示，渲染链路 100% 排除）
分支：`debug/codearts-tui-render`（基于 31a9fad）；提交 `22a211d`；tag `diag/codearts-tui-render-20260915`；未 push。
