# Tauri v2 子 Webview 尺寸/布局问题诊断

> 状态：排查中  
> 关联：`src-tauri/src/bridge.rs`、`src/composables/useBrowserHost.ts`、`src/styles/global.css`  
> 环境：Linux + WebKitGTK + Tauri v2

---

## 1. 问题现象

运行 `npm run tauri dev` 后，浏览器页签内容区（子 Webview）的实际渲染高度被卡在 **约 400px**，而父窗口与前端宿主 div 的预期高度远大于此。

具体表现：

- 主窗口 inner size 为 `1200x800`，outer size 为 `1200x837`。
- 前端 `BrowserHost.vue` 中 `.browser-host` 的 CSS 尺寸经浏览器 devtools 查看约为 `1200x550`。
- 但 Rust 后端日志反复出现 `child_before_size=Some(PhysicalSize { width: 1200, height: 400 })`，子 Webview 实际只渲染出 400px 高。
- 前端传给后端的 `css_input` 一会儿是 `(0,109,1200,665)`，一会儿又被压成 `(0,109,1200,165)` 或 `(0,109,1200,415)`，说明**前端宿主 div 的真实可用高度本身就在抖动**。

---

## 2. 当前实现链路

```
BrowserHost.vue 渲染 <div class="browser-host">
        ↓
useBrowserHost.ts 用 getBoundingClientRect() 取 rect，
                  并通过 position_browser(rect) 发给后端
        ↓
bridge.ts 调用 Tauri command `position_browser`
        ↓
bridge.rs 接收 (x, y, w, h) 后，
          用 PhysicalPosition/PhysicalSize 直接设置子 Webview
```

关键 Rust 代码（`src-tauri/src/bridge.rs`）：

- `spawn_child_window` 用 `LogicalPosition` + `LogicalSize` 创建子 Webview。
- `apply_bounds` 用 `PhysicalPosition` + `PhysicalSize` 调用 `set_position` / `set_size`。

关键前端代码：

- `global.css` 中 `.browser-host` 用 `position: absolute; inset: 0` 填满 `.viewport`。
- `.viewport` 是 flex 布局中 `flex: 1` 的自适应区域。

---

## 3. 已确认的事实

| 观测项 | 数值/结论 |
|---|---|
| 主窗口 inner size | `1200x800` |
| 主窗口 outer size | `1200x837`（含标题栏/边框） |
| 标题栏 + 活动栏 + 地址栏 + 页签栏高度 | 约 109px（从 css_input 的 y 值看出） |
| 预期内容区高度 | `800 - 109 - 状态栏(26px) ≈ 665px` |
| 前端 rect 偶尔报出 | `1200x665`、`1200x415`、`1200x165`、`1200x550` |
| 后端 child_before_size 始终为 | `1200x400` |
| 显示器缩放 | 物理分辨率 2560x1600，`devicePixelRatio = 1`，GNOME scaling-factor = 0（未开启） |

---

## 4. 根因分析（当前最可能的解释）

### 4.1 坐标系/单位混用

Tauri v2 的 `Window::add_child` 文档语义是：

- 子 Webview 的 position/size 使用**父窗口内容区的逻辑坐标**（Logical）。
- 一旦子 Webview 被 add_child，后续对其调用 `set_position` / `set_size` 也应使用**逻辑坐标**，而不是物理像素。

当前代码的问题：

```rust
// apply_bounds 里直接用 PhysicalPosition/PhysicalSize
webview.set_position(PhysicalPosition::new(x, y))?;
webview.set_size(PhysicalSize::new(w, h))?;
```

如果 Tauri/Linux 后端把 `PhysicalSize` 里的数值当成逻辑坐标再次缩放，就会出现“数值被吞”或“被限制”的错觉。更关键的是，当系统缩放因子 ≠ 1 时，物理像素和逻辑像素不一致，会直接造成错位。

### 4.2 前端宿主 div 高度本身不稳定

`.browser-host` 依赖 flex 布局撑满 `.viewport`，而 `.viewport` 又依赖 `MainArea.vue` 中多层 flex 容器。任何一层因为：

- 状态栏未正确 `flex-shrink: 0`
- AI 侧边栏 `display: none` / `display: flex` 切换
- 活动栏动态高度
- `min-height: 0` 缺失

都会导致 `.browser-host` 的真实高度抖动，从而把不稳定的数据传给后端。

从日志看，`css_input` 的高度在 165/415/665 之间跳变，说明前端布局本身还没有稳定。

### 4.3 子 Webview 创建时的初始尺寸被“锁死”

`spawn_child_window` 创建子 Webview 时传的是 `(1,1)` 的极小尺寸 + `visible=false`：

```rust
let child = main_window.add_child(
    WebviewBuilder::new(label, url)
        .auto_resize(true)   // ← 已开启自动缩放
        .visible(false),
    LogicalPosition::new(1.0, 1.0),
    LogicalSize::new(1.0, 1.0),
)?;
```

即使后续调用 `set_size`，WebKitGTK 在某些情况下会把子 view 的“可见区域”按第一次有效布局时的尺寸缓存。这就是 `child_before_size` 一直卡在 `1200x400` 的原因——400px 是某次首次成功布局时的高度，之后再也没有被真正更新。

### 4.4 Linux WebKitGTK 的特殊行为

WebKitGTK 对 `WebKitWebView` 的 size_allocate 有以下特性：

- 它依赖 GTK widget 的 `allocate` 事件决定渲染尺寸。
- 如果父 GTK 容器没有正确触发 `size-allocate` 或 widget 的 `visible` 状态变更，WebView 会沿用旧的 allocation。
- Tauri v2 在 Linux 下对子 Webview 的 resize 支持相对较新，某些版本中存在“子 webview 尺寸不随 set_size 更新”的已知问题。

---

## 5. 为什么这个问题特别难

### 5.1 四层坐标系叠加

| 层级 | 单位/坐标系 | 备注 |
|---|---|---|
| CSS 像素 | 逻辑 px | 浏览器 devtools 看到的高度 |
| 前端 JS `getBoundingClientRect` | CSS 像素 | 与 CSS 一致 |
| Tauri Webview API | Logical / Physical 可选 | `LogicalPosition` vs `PhysicalPosition` |
| GTK/WebKitGTK | 物理像素 + widget allocation | 最终真正决定渲染 |
| 屏幕 | 物理像素 | 显示输出 |

任何一个环节单位假设错误，都会放大/缩小或偏移。调试时日志里的数字看起来“差不多”，但实际上已经在某个层级被转换过。

### 5.2 异步 + 多进程

Tauri 应用本质上是：

- Rust 主进程（管理 GTK 窗口和 WebKit）
- 前端 Webview 进程（Vite / Vue）
- 子 Webview 进程（每个页签一个）

前端布局变化 → JS 事件循环 → IPC → Rust 事件循环 → GTK 主线程 → WebKit 渲染线程。这条链路里有多个异步队列，导致：

- 前端已经 resize 完成，但后端命令还在排队。
- 后端设置尺寸时，前端布局又已变化。
- 日志时间戳无法精确对齐，难以判断因果。

### 5.3 平台差异

同样的代码在 macOS/Windows 上可能正常工作，因为：

- macOS WebKit 对子 view 的 frame 更新响应及时。
- Windows WebView2 有独立的 composition 系统。
- Linux WebKitGTK 直接嵌入 GTK container，对 `set_size` 的响应最慢、最挑剔。

这意味着在 Linux 下调试通过的方案，换平台可能还有另一套问题；反之亦然。

### 5.4 Tauri v2 子 Webview API 仍在演进

Tauri v2 的 `Window::add_child` 和子 Webview 的 `set_position`/`set_size` 是相对较新的 API：

- 文档对“子 webview 创建后如何更新尺寸”描述不够详尽。
- 社区 issue 中关于 Linux 子 webview resize 失效的报告并不少。
- `auto_resize(true)` 的行为在不同平台不一致。

我们经常需要“试”而不是“查文档确定”。

### 5.5 UI 布局本身复杂

当前布局树：

```
.app (flex column)
  ├─ TitleBar
  ├─ ActivityBar
  ├─ .body (flex row)
  │    ├─ AINavPanel (条件渲染)
  │    └─ MainArea (flex column)
  │         ├─ AddressBar
  │         ├─ TabBar
  │         ├─ .viewport (flex:1)
  │         │    └─ .browser-host (absolute)
  │         └─ StatusBar
```

任何一层高度不稳定，都会传导到 `.browser-host`。而 AddressBar、AINavPanel、StatusBar 的显示/隐藏都可能变化，导致高度不是固定值，进一步加剧抖动。

### 5.6 视觉反馈滞后

子 Webview 渲染的是外部网页（如 baidu.com），我们无法直接在其内容上画标尺或边框。要确认“到底多高”，只能依赖：

- 后端日志
- 前端 devtools 看 `.browser-host` 的 rect
- 截图后用肉眼估算

三者经常不一致，增加了定位难度。

---

## 6. 下一步可选方案

### 方案 A：修正坐标系（优先尝试）

1. 前端传 `position_browser` 时，明确传**逻辑像素**（CSS 像素）。
2. Rust 端统一使用 `LogicalPosition` / `LogicalSize` 调用 `set_position` / `set_size`。
3. 移除 `PhysicalPosition`/`PhysicalSize` 的混用。
4. 同时确保 `.browser-host` 的真实高度稳定后再触发 IPC（加 `ResizeObserver` + `requestAnimationFrame` 防抖）。

### 方案 B：创建时就定好尺寸

不先创建 `1x1` 的子 Webview，而是在 `create_tab` 时就把目标矩形算好，直接 `add_child` 到正确尺寸。避免后续 resize 被 WebKitGTK 忽略。

### 方案 C：用多窗口替代子 Webview

如果子 Webview 在 Linux 下实在不稳定，回退到 Tauri 多窗口方案。每个页签一个独立窗口，由 Rust 直接控制窗口尺寸。缺点是失去“内嵌标签页”的视觉效果。

### 方案 D：监听主窗口 resize，后端主动同步

在 Rust 端监听主窗口 `WindowEvent::Resized`，由后端统一计算并更新所有子 Webview 的尺寸，而不是由前端驱动。减少 IPC 异步带来的时序问题。

---

## 7. 待验证假设

1. 把 `PhysicalPosition`/`PhysicalSize` 改成 `LogicalPosition`/`LogicalSize` 后，`child_before_size` 是否会变成 1200x665？
2. `auto_resize(true)` 是否反而是罪魁祸首？尝试关闭它，完全手动控制尺寸。
3. 子 Webview 在 `visible(false)` 状态下创建后，第一次 `set_size` 是否真的生效？尝试创建时直接 visible(true) 并给定正确尺寸。
4. 前端 `.browser-host` 的高度抖动是否由 StatusBar / AINavPanel 引起？用固定尺寸暂时替换 flex 布局后观察。

---

## 8. 结论

当前最可能的根因是 **Rust 后端使用了物理像素坐标设置子 Webview，而 Tauri/Linux 期望的是逻辑坐标**，叠加 **前端 flex 布局尚未稳定就把数据发给后端**，导致子 Webview 被锁在一个错误的初始高度。

这个问题的难点不在于某一段代码明显写错，而在于：

- 多个坐标系混用且文档不清；
- 跨进程/跨线程/跨平台的异步链路长；
- Linux WebKitGTK 对子 view resize 的支持最薄弱；
- UI 布局树复杂，高度不是固定值，容易被误传；
- 视觉反馈滞后，难以快速验证修改效果。

建议下一步先用 **方案 A（统一 Logical 坐标 + 防抖）** 做最小改动验证；如无效，再考虑 **方案 B（创建时直接定尺寸）** 或 **方案 D（后端主动同步）**。
