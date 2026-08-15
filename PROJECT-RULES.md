# 项目规范（锁定规则，禁止随意改动）

> 本文件记录**已经过实测验证的正确做法**。凡标注【锁定】的规则，改动前必须先有充分的实测证据（日志 + 截图），并回滚验证，否则一律拒绝。
> 背景：本仓库曾因"凭猜测修改已验证的正确代码"导致回退性 bug，故建立本规范。

---

## 规则 1【锁定】子 WebView 定位定尺寸：gtk_fixed_move + size_allocate，绝不 set_size_request / queue_resize

文件：`tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/linux.rs` 的 `ensure_size_allocated`

**正确做法（v0.5.0 经日志实锤验证）：**

1. `gtk_fixed_move(child, x, y)` —— 固定位置。wry 的 `set_bounds`/`set_position` 对 GtkFixed 子控件只 `size_allocate` 不 move，位置会漂。
2. `size_allocate(&Allocation::new(x, y, w, h))` —— 固定尺寸 + 位置，触发 `size-allocate` 信号让 WebKit 刷新 CSS 视口。
3. `queue_draw()` —— 触发重绘。

**绝对禁止（v0.3.0~v0.4.0 沾满 bug 的根因，日志实锤）：**
- ❌ **`queue_resize()`**：触发 GtkFixed 布局循环，按"剩余空间"把子控件越拉越大（实测 alloc y 从 91→46→24→1、高 709→799 逐步沾满全窗口）。
- ❌ **`set_size_request(w, h)`**：被 GtkFixed 布局循环忽略/覆盖，且与 WebKit 自然尺寸请求打架。
- ❌ 认为"必须 queue_resize 才能刷新 WebKit 视口"——错误。`size_allocate` 本身就发 `size-allocate` 信号，足够刷新视口。

> 历史教训：v0.3.0 的"三件套（set_size_request + queue_resize + auto_resize）"曾被误锁为正确，实测它是沾满 bug 的根源。v0.5.0 改为 gtk_fixed_move + size_allocate 后子 webview 稳定在正确位置。

---

## 规则 2 子 WebView 的 `auto_resize`

文件：`tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs` 的 `create_tab`

- wry 0.55 / tao 0.35 的 WebKitGTK 后端**没有实现 `auto_resize`**（grep 全 crate 无实现代码），该属性在 Linux 下是 no-op。
- 因此开不开都行，**不依赖它**做任何事。尺寸与位置完全由前端 `tab_position` + 规则 1 的 `gtk_fixed_move`/`size_allocate` 控制。

---

## 规则 3【锁定】前端坐标：纯 CSS 像素，绝不乘 devicePixelRatio

文件：`src/composables/useBrowserHost.ts` 的 `schedulePosition` / `scheduleGrid`

- 坐标一律 `Math.round(r.left/top/width/height)`，**绝不乘 `devicePixelRatio`**。
- DPI 换算由 Tauri / wry 统一处理。
- 占位 div 用 `visibility:hidden` 而非 `display:none`（保证 `getBoundingClientRect` 非零）。

---

## 规则 3.5【锁定】隐藏/移出子 WebView：只移位置，绝不改尺寸、绝不 webview.hide()

文件：`src-tauri/src/bridge.rs` 的 `hide_bounds`

**正确做法（v0.5.0 经卡死实锤验证）：**
- 隐藏非激活页签 = `update_rect(id, (-30000, 原y, 原w, 原h))` —— **只把 x 移到屏幕外，尺寸保持不变**。

**绝对禁止（实测卡死根因）：**
- ❌ **`webview.hide()` / `set_visible(false)`**：对正在渲染的 WebKitGTK 子 webview 调 hide 会阻塞主线程事件循环，死锁（新建第二个页签时卡在 hide(旧页签)）。
- ❌ **隐藏时把尺寸缩到 1x1**：尺寸剧烈缩小触发 WebKit 视口重布局，与主线程死锁。
- ❌ **坐标用 -100000**：超出 X11 int16 范围（-32768~32767），协议异常。用 -30000。

> 原理：WebKit 视口重布局只在**尺寸变化**时触发。只移动位置（尺寸不变）则视口不变、不重布局、不卡死。

---

## 规则 4 修改"已验证正确"代码的流程（防止再次翻车）

当要修改本文件中标注【锁定】的代码时，必须：

1. **先复现问题**：用日志 + 截图证明现状有 bug，且 bug 与该锁定代码直接相关。
2. **提出假设并最小验证**：改 1 行，重新编译，实测确认假设成立；不成立立刻回滚。
3. **不得凭"直觉/代码看上去不对"删改**：尤其不得凭"怀疑是死循环/多余"就删 `gtk_fixed_move`/`size_allocate` 等已验证的修复，也不得擅自加回 `set_size_request`/`queue_resize`/`webview.hide()`（已实锤是 bug 根源）。
4. **改错立刻回滚**：若修改后网页撑不满 / 不能点击 / 前进后退失效，说明改动错误，必须 `git checkout` 恢复到上一版本，再重新分析。

---

## 规则 5 日志判读规则

- **一次 `force size_allocate: (1x1) -> (1200x665)`** = 正常初始化，无需处理。
- **反复 `force size_allocate`** 在 v0.3.0 中是**布局收敛过程**（`auto_resize` 把子 webview 拉到主窗高度，前端又精确到容器高度，最终稳定在容器高度），**不是死循环**，**不要据此改动代码**。
- 真正要警惕的是：**网页视觉没撑满 / 不能点击 / 前进后退失效**——这些才是回退信号，而非日志刷屏。

---

## 附：本仓库的实测正确版本基准

- `v0.3.0`（commit `7a87942` / `10c7813`）：子 webview 撑满 + 网页链接可点击 + 前进后退正常 + 地址栏同步。**这是基准正确版本。**
- `v0.3.1`（commit `b7500a5`）：错误回退版本，其代码改动（关 auto_resize / 删 queue_resize）**已撤销**，仅保留文档结构。今后以 v0.3.0 为准。
