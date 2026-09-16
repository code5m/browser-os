# 终端历史回归审计（HISTORICAL REGRESSION AUDIT）· 2026-09-15

## 结论

**A = NONE：未发现客户端历史代码的终端回归**；**B = 存在：CodeArts 自身启动交互阻塞**（更新确认对话框 + 浏览器 OAuth 授权等待），二者独立。
LAST_KNOWN_GOOD = `26e3586`（实测 GOOD）；FIRST_BAD_COMMIT = **NONE**。

## 终端路径变更全清单（5 核心文件 `git log --follow`）

| commit | 日期 | 终端语义变化 |
|---|---|---|
| 78ecec2 | 08-19 | xterm.js 终端引入（bindTermWriter 起源） |
| f1f5ca6 | 08-19 | 宫格双模式 + **终端保持挂载** |
| d77b849 | 09-05 | **M3.a 管道内核**（terminal.rs 诞生：worker→mpsc(128)→pump(16ms/64KiB)→Channel，引入 lossy 丢弃） |
| ba67248 | 09-05 | M3-review：真实 resize 取证 + 放锁后再终止 |
| 0dd4cf4 | 09-05 | M3.c：临时历史 40 条 + resize 静默窗口（140ms/500ms）+ proposeDimensions 守卫 |
| 8696935 | 09-09 | 仅 CSS（`.term-mod` height/overflow），无语义变化 |
| **2a96cb1** | 09-13 | **单终端→多终端重构（纯前端）**：termId→termPanes[]、termWriter→termWriters Map、termHistory→per-pane Map、TerminalPane 加 paneId props/small |
| 3a3b616 | 09-14 | 文件面板修复：**终端相关改动 0 行** |
| 31a9fad | 09-14 | 仅 AGENTS.md + release 脚本 |
| （未提交） | 09-15 | probe（诊断）+ autoConfirmCli（**从未提交**=问题出现后所加，属结果非原因） |

## 实测结果（固定标准：shell / vim / CodeArts）

| 版本 | shell | vim | CodeArts | 判定 |
|---|---|---|---|---|
| P3 `26e3586`（多终端前） | ✓ `echo P3_SHELL_OK`→输出 | ✓ 全屏完整 | ✓ 文本完整渲染；停在浏览器授权等待；进程存活 | **GOOD** |
| P4 `2a96cb1`（多终端后） | ✓ | ✓ | ✓ 与 P3 **逐项一致**（同停在授权等待） | **GOOD** |
| HEAD `31a9fad`+probe（前序已测） | ✓ | ✓ alternate | ✓ TUI 五层字节恒等 + 用户目检显示 | **GOOD** |

测试方法：隔离 worktree（`git worktree add --detach`，主工作区零改动）+ `XDG_DATA_HOME` 隔离 + 共享 CARGO_TARGET_DIR + 自写 `xwd2png.py`（XWD→PNG，绕过 Wayland 截图限制）实现自证视觉；输入用 XTEST + 每次 `xdotool windowfocus`。

## 静态验证（关键）

- `26e3586 → 2a96cb1` 后端 diff：`src-tauri/src/terminal.rs` / `bridge.rs` **0 行**；`bridge.ts` 仅 +7 行 → 多终端重构为**纯前端**。
- `onContainerResize` 守卫两版**逐字相同**（`if (!dims || dims.cols <= 0 || dims.rows <= 0) return;`），仅删了一行注释。
- 0×0 事件下 `proposed=NaNx1` 会通过 `<=0` 判断到达 `fit.fit()`（NaN 比较陷阱）——该瑕疵**新旧版本同样存在**（pre-existing 潜在隐患，非回归；实测有守卫恢复，未造成故障）。

## 审计覆盖边界（诚实声明）

- 实测覆盖 `26e3586`、`2a96cb1`、`31a9fad` 三个边界点 + 全量静态清单；中间窗口（08-19~09-05）未逐 commit 实测——但 `26e3586` 位于其后且 GOOD，故**当前 HEAD 不存在可利用的终端回归**。
- 若历史上存在"曾被改坏、后又修好"的中途窗口，本审计不覆盖（不影响当前问题归因）。

## 环境备注

- 主工作区全程未受影响：`git status` 保持 12 M + 2 ??（审计前后一致）。
- 我创建的 2 个临时 worktree 已 `git worktree remove --force` 清除；仓库内另有**前序存在**的 `~/.codex/worktrees/e0c9` 也指向 26e3586（他人/前序会话产物，未动）。
- 共享 target 目录当前二进制为 P4 版本；主树再次 `npm run dev`/`cargo build` 会自动重建，无需手工处理。
