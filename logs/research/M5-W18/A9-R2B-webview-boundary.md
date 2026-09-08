# A9 R2B — 原生 WebView 边界验证（含 A11 可执行场景）

- **Lane**: A9
- **Dispatch**: R2B §10「验证新布局的原生 WebView 边界：坐标、DPI、菜单/搜索/弹窗遮挡、焦点、切换隐藏、宫格子进程」
- **方法**: 静态源码/锁规则审计（W18-R，无 GUI 执行）；native 验收记账 = **`NOT_RUN`**（缺 GUI 通道，按蓝图 §7 不得写 PASS）
- **权威契约**: `PROJECT-RULES.md` 规则 1/2/3/3.5/3.6；`WORKBENCH_BLUEPRINT-20260908.md` §3、§6（S1）、§7

> 本文件给出 A11 可执行的验收场景与「源码已锁定的正确做法」；**实际 GUI 通过/失败由 A11 在 native 通道跑，本 lane 只记录设计与反例，不代判 PASS**。

---

## 1. 已锁定的原生 WebView 边界事实（来自 `PROJECT-RULES.md`，实测验证，禁止改动）

| 维度 | 锁定做法 | 禁止做法 | 来源 |
|---|---|---|---|
| 定位定尺寸 | `gtk_fixed_move(child,x,y)` + `size_allocate(Allocation(x,y,w,h))` + `queue_draw()` | `set_size_request` / `queue_resize`（GtkFixed 布局循环、越拉越大） | 规则 1 |
| auto_resize | Linux WebKitGTK 后端未实现，no-op；尺寸完全由前端 `tab_position` + gtk_fixed_move/size_allocate 控制 | 依赖 `auto_resize` 做任何事 | 规则 2 |
| 前端坐标 | 纯 CSS 像素，`Math.round(r.left/top/w/h)`，**绝不乘 devicePixelRatio** | 坐标乘 DPI | 规则 3 |
| 隐藏/移出 | `update_rect(id,(-30000,原y,原w,原h))` 只移 x 到屏外，尺寸不变 | `webview.hide()`（死锁）/ 缩到 1×1 / 用 -100000（超 X11 int16） | 规则 3.5 |
| 远程命令同步 | 子 webview 新增 `invoke` 命令必须同步加进 `remote-collect.toml` + `generate_handler!` | 漏加（报 `xxx not allowed`） | 规则 3.6 |

## 2. 新布局下须验证的边界项（R2B §10）

1. **坐标 / DPI**：原生子 webview 坐标仍遵守 CSS 像素契约（规则 3）；DPI=1/1.5/2 下 `gtk_fixed_move`/`size_allocate` 传入值应为前端 CSS px（Tauri/wry 统一做 DPI 换算）。窄窗 1024×720 / 800×600 下重要控件可达、文本不遮挡（蓝图 §3 尺寸约束）。
2. **菜单 / 搜索 / 弹窗遮挡**：原生上下文菜单、统一搜索框、模态弹窗必须绘制在 tab/grid webview **之上**，不被 WebKit 子控件覆盖（GtkFixed 子控件 z-order 由添加序/容器决定，菜单/弹窗须为独立顶层或更高层级）。
3. **焦点**：切换到隐藏页签（移到 -30000）再切回，焦点须正确归还（规则 3.5 只移位置不改尺寸 → 视口不重布局 → 焦点态保留）。
4. **切换隐藏**：多 tab / grid 子进程切换，隐藏用屏外位移而非 `webview.hide()`（规则 3.5 死锁红线）。
5. **宫格子进程**（`grid-*` / `grid-child-*`）：`browser-remote.json:5` 已把 `grid-child-*` 列入 windows、`grid-*` 列入 webviews 且 remote urls 通配 → 宫格子 webview 同样受 `remote-collect` 三命令约束；新增宫格命令须同步进 `remote-collect.toml`（规则 3.6）。

## 3. A11 可执行验收场景（记账 `NOT_RUN`，待 A11 native 通道）

每个场景写：前置 / 入口 / 动作 / 预期 / 失败注入 / 证据路径 / 来源。

**S-WV-1 菜单不被 webview 遮挡**
- 前置：debug（`localhost:1421`）与 release（`tauri://localhost`）各装一次；打开≥2 个浏览器 tab。
- 入口：在 tab 区域右键唤起原生上下文菜单；打开统一搜索（`omni`）。
- 动作：菜单/搜索出现时，悬停到 tab webview 覆盖区域。
- 预期：菜单/搜索绘制在 webview 之上，可点击；tab 不抢 z-order。
- 失败注入：菜单被 tab 内容遮挡、点击穿透到网页。
- 证据：`RUST_LOG=tauri=debug` + 截图；来源 `PROJECT-RULES.md` 规则 1。
- 验收：`NATIVE_PASS` / `NOT_RUN`。

**S-WV-2 DPI 1/1.5/2 坐标稳定**
- 前置：系统缩放分别设 100%/150%/200%。
- 入口：新建 tab + 宫格。
- 动作：记录 `tab_position` 传入 CSS px；比对实际渲染位置。
- 预期：三档下 tab/宫格均落在前端指定 CSS 位置，无偏移/沾满（规则 1 不被 DPI 破坏）。
- 失败注入：DPI=2 时子 webview 尺寸翻倍（误乘 DPI）。
- 证据：截图 + `force size_allocate` 日志（规则 5：单次 1×1→容器尺寸为正常初始化，反复刷屏非死循环）。
- 验收：`NATIVE_PASS` / `NOT_RUN`。

**S-WV-3 隐藏→切回焦点归还**
- 前置：tab A 有输入焦点（地址栏）。
- 动作：切到 tab B（A 移到 -30000）；再切回 A。
- 预期：A 恢复焦点且内容/滚动/选区不丢（蓝图 J1）；无死锁（未调 `webview.hide()`）。
- 失败注入：切回时卡死（旧 `webview.hide()` 死锁根因）/ 焦点丢失。
- 证据：操作录屏 + 进程不阻塞。
- 验收：`NATIVE_PASS` / `NOT_RUN`。

**S-WV-4 宫格子进程远程命令边界**
- 前置：打开宫格（`grid-*` webview，外部 http(s) 域）。
- 动作：在宫格页内尝试 `invoke('save_note',...)`（无意图令牌）。
- 预期：被拒（不在 `remote-collect`；且 `issue_intent` 仅 `main` 可签发，`bridge.rs:1985`）。
- 失败注入：宫格子 webview 能写盘/开终端。
- 证据：`remote-collect.toml:8` + `bridge.rs:1985-1987` 静态核对 + 运行时拒绝日志。
- 验收：`NATIVE_PASS` / `NOT_RUN`（静态已证，运行时复核）。

**S-WV-5 IME 不误触全局快捷键**
- 前置：地址栏/搜索框输入中文（IME 开启）。
- 动作：输入含候选切换的字符。
- 预期：IME 组合键不触发全局快捷键（蓝图 §7）。
- 失败注入：IME 上屏时误关弹窗/切换 tab。
- 证据：输入录屏。
- 验收：`NATIVE_PASS` / `NOT_RUN`。

## 4. 判定

- 原生 WebView 边界的**源码级正确做法已锁定**（规则 1/2/3/3.5/3.6），A9 不主张改动。
- 5 个 A11 场景已登记；**native 验收本批 `NOT_RUN`**（A9 无 GUI 通道）。
- 宫格子进程边界已在 capability 层静态闭环（S-WV-4）；其余 4 项交 A11 在 debug/release 双通道跑。
