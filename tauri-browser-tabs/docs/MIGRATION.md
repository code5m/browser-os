# 迁移指南：从手写 add_child 到 tauri-browser-tabs

本指南以一个真实项目（mvp-browser-os，多页签 + 宫格浏览器）的迁移为例。

## 迁移前的典型代码（问题模式）

```rust
// 创建：Logical 坐标，1x1 占位
window.add_child(
    WebviewBuilder::new(label, WebviewUrl::External(url)),
    LogicalPosition::new(0.0, 0.0),
    LogicalSize::new(1.0, 1.0),
)?;

// 定位：Physical 坐标（前端乘了 devicePixelRatio）
webview.set_position(Position::Physical(PhysicalPosition { x, y }))?;
webview.set_size(Size::Physical(PhysicalSize { width, height }))?;
```

```ts
// 前端：乘 dpr 转成"物理像素"
bridge.tabPosition(id, {
  x: Math.round(x * dpr),
  y: Math.round(y * dpr),
  width: Math.round(w * dpr),
  height: Math.round(h * dpr),
});
```

问题：坐标语义割裂（创建 Logical / 定位 Physical），Linux 下 1x1 初始尺寸被 WebKitGTK 缓存导致 resize 失效。

## 迁移步骤

### 1. 引入插件

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["unstable"] }
tauri-plugin-browser-tabs = "0.1"
```

```rust
// src-tauri/src/main.rs
tauri::Builder::default()
    .plugin(tauri_plugin_browser_tabs::init()) // 默认宿主窗口 label = "main"
    // 或 .plugin(tauri_plugin_browser_tabs::init_with_host("my-window"))
```

```json
// src-tauri/capabilities/default.json
{ "permissions": ["core:default", "browser-tabs:default"] }
```

### 2. 后端：替换创建/定位/关闭实现

```rust
use tauri_plugin_browser_tabs::{CreateTabOptions, LogicalRect, TabManagerState};

// 创建（带初始化脚本注入；window.open 由插件统一拦截并发事件）
let manager = app.state::<TabManagerState>();
manager.create_tab(CreateTabOptions {
    id: "tab-1".into(),
    url: "https://example.com".into(),
    rect: LogicalRect::new(0.0, 0.0, 1.0, 1.0),
    visible: false,
    auto_resize: true,
    user_agent: None,
    transparent: false,
    initialization_script: Some(include_str!("../injected/collect.js").into()),
})?;

// 定位 + 显示
manager.update_rect(&"tab-1".into(), LogicalRect::new(x, y, w, h))?;
manager.set_visible(&"tab-1".into(), true)?;

// 隐藏 / 关闭
manager.set_visible(&"tab-1".into(), false)?;
manager.close_tab(&"tab-1".into())?;
```

要点：

- 插件方法可从 command 线程直接调用，**不再需要** `run_on_main_thread`。
- 50ms 去重等业务层防抖逻辑保留在应用侧。

### 3. 前端：去掉 dpr 乘法

```ts
// 迁移后：直接传 getBoundingClientRect 的 CSS 像素
bridge.tabPosition(id, { x, y, width: w, height: h });
```

### 4. window.open 拦截的事件转发

插件把 `on_new_window` 统一为 `browser-tabs://event`（type = `newWindowRequested`）。若你的前端已有自己的事件名，在后端转发一次即可保持前端零改动：

```rust
use tauri::{Emitter, Listener};
let forward = app.handle().clone();
app.listen("browser-tabs://event", move |event| {
    let payload: serde_json::Value = serde_json::from_str(event.payload()).unwrap();
    if payload["type"] == "newWindowRequested" {
        let _ = forward.emit("new-tab-request",
            serde_json::json!({ "url": payload["url"] }));
    }
});
```

### 5. 注意事项

- **宿主窗口 label**：插件默认找 `main`。若你的主窗口用 `WindowBuilder::new(app, "main")` 手动创建（裸 Window 模式），插件的懒绑定机制完全兼容，无需额外处理。
- **eval / get_webview**：插件创建的子 webview 照常注册在 Tauri 的 webview 管理器里，`app.get_webview(label)` + `eval()` 的既有代码继续可用。
- **验收清单**：`cargo check` 通过 → 前端 build 通过 → 运行后子 webview 尺寸跟随布局 → DPR > 1 环境（如有）复测。
