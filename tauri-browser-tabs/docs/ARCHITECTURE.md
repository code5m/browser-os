# Architecture

本文档描述 tauri-browser-tabs 的设计决策与内部结构。

## 问题背景

Tauri v2 的 `Window::add_child` 允许在同一窗口内嵌入多个子 webview，但直接使用会遇到三类问题：

1. **坐标系混乱**：`LogicalPosition` vs `PhysicalPosition`，前端 CSS 像素、`devicePixelRatio`、Tauri `scale_factor` 之间如何换算没有明确范式，极易出现"创建用 Logical、定位用 Physical"的割裂用法。
2. **Linux WebKitGTK resize 失效**：以 1x1 创建的子 webview，后续 `set_size` 可能被忽略（底层 GTK widget 的 size_allocate 被缓存），表现为内容区卡在初始高度（如 400px）。
3. **时序与死锁**：GTK 窗口操作必须在主线程；command 同步上下文里直接 build webview 会与事件循环互等死锁。

## 设计原则

### 1. 单一坐标系：Logical（CSS 像素）

全链路只使用逻辑坐标：

```
前端 getBoundingClientRect()  ──CSS px──▶  IPC  ──CSS px──▶  LogicalPosition/LogicalSize
```

- 前端**不乘** `devicePixelRatio`。
- 后端**不读** `scale_factor` 做换算。
- DPI 缩放完全交给 Tauri/wry 内部处理。

这样消除了所有"该不该乘 dpr"的判断分支，代码路径唯一。

### 2. 平台隔离层（platform/）

```
platform::create_child_webview()   ── 直接以目标 rect 创建（不用 1x1 再 resize）
platform::apply_rect()             ── set_position + set_size
        └── [linux] ensure_size_allocated()  ── 强制 GTK size_allocate + queue_draw
```

- **macOS / Windows**：Tauri 标准 API 行为正确，无额外处理。
- **Linux**：通过 `with_webview` 拿到底层 `webkit2gtk::WebView`，对比当前 allocation 与期望尺寸，不一致时强制 `size_allocate`。

### 3. 懒绑定宿主窗口

`TabManager` 不缓存 `Window` 句柄，只保存 `AppHandle + host_label`，每次操作时 `app.get_window(label)`：

- 插件 `setup` 早于应用 `.setup()` 执行，而很多应用（如 mvp-browser-os）在 `.setup()` 里手动 `WindowBuilder` 创建裸窗口——此时宿主窗口尚不存在。
- 窗口可能被销毁重建，缓存句柄会失效。

### 4. 事件模型

| 通道 | 事件 | 说明 |
|---|---|---|
| `browser-tabs://event` | `navigationFinished` | 子 webview 导航完成 |
| `browser-tabs://event` | `newWindowRequested` | `window.open` / `target="_blank"` 被拦截（始终 Deny），由宿主决定开新页签 |
| `browser-tabs://window-resized` | — | 宿主窗口 resize（若插件 setup 时窗口已存在才注册） |

`on_new_window` 始终 Deny 的原因：在 webview 创建回调里同步创建新窗口会导致 GTK/X11 死锁；交给宿主应用异步处理是唯一安全路径。

### 5. 线程模型

- 插件所有公开方法（`create_tab` / `update_rect` / `set_visible` / `close_tab`）内部走 wry dispatcher，**可从任意线程调用**，无需调用方自行 `run_on_main_thread`。
- `create_tab` 底层 `add_child` 是同步等待主线程完成 build（channel 阻塞），在 Tauri command 线程调用安全；**不要在主线程回调内调用**，否则自等死锁。

## 组件分层

```
┌──────────────────────────────────────────────┐
│ @tauri-browser-tabs/vue                      │
│   BrowserHost.vue / useBrowserTabs()         │  Vue 组件 + composable
├──────────────────────────────────────────────┤
│ @tauri-browser-tabs/core                     │
│   BrowserTabManager                          │  ResizeObserver + 16ms 防抖 + rect 相等跳过
│   geometry.ts                                │  DOMRect → LogicalRect
├──────────────────────────────────────────────┤
│ tauri-plugin-browser-tabs (Rust)             │
│   commands.rs   ─ TabManager (懒绑定)        │
│   platform/     ─ 坐标统一 + Linux 修复      │
├──────────────────────────────────────────────┤
│ Tauri v2 (unstable: add_child/WebviewBuilder)│
└──────────────────────────────────────────────┘
```

## 依赖特性说明

- `tauri` features = `["unstable", "wry"]`：`add_child` / `WebviewBuilder` / `with_webview` 均为 unstable API。使用本插件的应用同样需要启用 `unstable`（cargo feature 会自动统一）。
- Linux 下额外依赖 `gtk 0.18` + `webkit2gtk 2.0`（与 wry 内部版本一致，共享同一份系统库）。

## 已知限制

- 仅支持桌面端（Windows / macOS / Linux）。移动端 Tauri 的 webview 模型不同，暂未适配。
- 依赖 Tauri unstable API，Tauri 升级时可能需要跟随调整。
- `browser-tabs://window-resized` 仅在插件 setup 时宿主窗口已存在才注册；手动晚创建窗口的应用应靠前端的 ResizeObserver 同步尺寸（core 包的 `BrowserTabManager` 默认已覆盖）。
