# Troubleshooting

本文件收录子 webview 集成中真实遇到并解决的问题，按症状索引。

## 症状：前端 UI 整体被压缩（如 100vh 只有约 400px），状态栏悬在窗口中间

**环境**：Linux + WebKitGTK，主 UI 通过 `Window::add_child` 承载（裸 `WindowBuilder` 窗口）。

**根因**：wry 的 GTK 后端对 `add_child` 的 webview 只做 `set_size_request`（最小尺寸请求）+ `gtk::Fixed::put`，不保证最终 allocation。主 UI webview 的 CSS 视口可能停在中间态（如约 400px），整个前端布局被压缩。这是 wry 0.55 GtkFixed 路径的结构性行为，**与前端框架无关**（Vue/React/原生 JS 都会中招）。

**修复**：主 UI webview 不要用 `add_child` 承载。改用标准 `WebviewWindowBuilder`（主 webview 作为窗口主内容，由 tao/wry 正常管理尺寸）；只有"页签/宫格"这类**附加**子 webview 才用 `add_child`（本插件）：

```rust
// 推荐：主 UI 用标准 WebviewWindow
let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
    .build()?;

// 页签仍由插件 add_child 到该窗口（不是多顶层窗口，无 X11 死锁问题）
```

**自查**：在前端 devtools 执行 `window.innerHeight`，若远小于窗口内容区高度，即为此问题。

## 症状：子 webview 高度卡在固定值（如 400px），resize 无效

**环境**：Linux + WebKitGTK。

**根因**：以 1x1（或较小的初始尺寸）创建子 webview 后，WebKitGTK 会缓存底层 GTK widget 的首次 `size_allocate`，后续 `set_size` 只改 wry 层的状态，不触发 GTK 重新分配。

**修复**：本插件已内置两种对策：

1. `create_tab` 直接以目标 rect 创建，避免 1x1 占位。
2. `update_rect` 后强制 `size_allocate` + `queue_draw`（`platform/linux.rs`）。

**自查**：若你绕开插件直接调 `add_child`，请确认没有"先 1x1 创建、再 set_size"的模式。

**辅助 API**：对非插件创建的 webview（如历史遗留的 add_child 主 UI），可用 `tauri_plugin_browser_tabs::ensure_native_layout(&webview)` 强制其原生 allocation 匹配 wry 层记录的尺寸。但治本方案仍是上一条：主 UI 回归标准 WebviewWindow。

## 症状：子 webview 位置/尺寸被放大或偏移（DPR > 1 的环境）

**根因**：前端把 CSS 坐标乘了 `devicePixelRatio` 传给后端，或后端把逻辑坐标当物理坐标又乘了一次 `scale_factor`，导致双重缩放。

**修复**：全链路只用 Logical 坐标。前端传 `getBoundingClientRect()` 原值，后端用 `LogicalPosition`/`LogicalSize`。不要乘任何 dpr。

**自查**：在 devtools 里执行 `window.devicePixelRatio`。若为 1 时表现正常、大于 1 时错位，基本就是双重缩放。

## 症状：Wayland 下启动数秒后崩溃 / 白屏

**根因**：WebKitGTK 在 Wayland 下默认启用 DMA-BUF 渲染器，存在静默崩溃问题。

**修复**（应用侧，非插件职责）：

```rust
std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
```

## 症状：Wayland 下子 webview 定位错位

**修复**（应用侧）：强制 X11 后端。

```rust
std::env::set_var("GDK_BACKEND", "x11");
```

## 症状：创建第二个 webview 时主线程卡死

**根因**：在主线程回调（如 `on_window_event`、listen 回调）里同步调用会阻塞主线程的 API，形成自等。

**修复**：插件的公开方法均可从任意线程调用（内部走 dispatcher）。但 `create_tab` 会同步等待主线程 build 完成——请在 Tauri command（独立线程池）中调用，不要在主线程回调里调。

## 症状：window.open / target="_blank" 导致死锁或无反应

**根因**：在 `on_new_window` 回调里同步创建窗口会破坏 GTK/X11 的 webview 创建流程。

**修复**：插件始终 Deny 新窗口并 emit `newWindowRequested` 事件，由宿主应用异步开新页签。监听示例：

```rust
app.listen("browser-tabs://event", |event| {
    // 解析 payload，type == "newWindowRequested" 时自行创建 tab
});
```

## 症状：窗口 resize 后子 webview 尺寸不更新

**自查清单**：

1. 前端宿主元素是否有 `ResizeObserver`（core 包 `BrowserTabManager` 默认开启）。
2. 布局链上是否有元素高度塌陷（flex 子项缺 `min-height: 0`）导致 `getBoundingClientRect()` 高度为 0。
3. 若宿主窗口在插件 setup 之后才创建，`browser-tabs://window-resized` 不会注册——此时完全靠前端的 ResizeObserver 即可。

## 症状：高频拖动分栏时界面卡顿

**根因**：每次 resize 都同步 IPC 到后端。

**修复**：core 包已内置 16ms 防抖 + rect 相等跳过。若自行实现，请至少保留 50ms 以内的去重。
