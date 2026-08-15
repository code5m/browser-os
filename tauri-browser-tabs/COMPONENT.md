# 组件：tauri-plugin-browser-tabs（子 WebView 浏览器插件）

> 可复用组件。本文件是**正确使用契约 + 禁忌清单**，由踩坑沉淀而来。
> 位置：`mvp-browser-os-v3/tauri-browser-tabs/`（已纳入主仓库 monorepo）

---

## 一、它解决什么问题

在**单个 Tauri 主窗口**内嵌入多个独立浏览器页签（每个页签是一个 `Webview`，作为主窗口的子控件 `add_child`）。支持：
- 多页签创建 / 切换 / 关闭 / 定位（Logical CSS 坐标）
- 页内导航事件回传（`NavigationFinished`）、`target=_blank` / `window.open` 拦截（`NewWindowRequested`）
- Linux/WebKitGTK 下子 webview 的尺寸正确撑满

---

## 二、正确用法（契约）

### 前端
1. 坐标一律用 **CSS 像素（Logical）**，**绝不乘 `devicePixelRatio`**——DPI 由 Tauri 统一处理。
2. 用 `ResizeObserver` + 主窗 `tauri://window-resized` 监听，任何布局变化都重新调用 `tab_position` 精确给出子 webview 的矩形（相对主窗内容区左上角）。
3. 子 webview 是独立 GTK 窗口叠在主窗之上，前端用一个空 `<div ref>` 占位（用 `visibility:hidden` 而非 `display:none`，保证 `getBoundingClientRect` 非零）。

### 后端（Rust 插件）
```rust
manager.create_tab(CreateTabOptions {
    id: "tab-1".into(),
    url: "https://baidu.com".into(),
    rect: LogicalRect::new(x, y, w, h),  // CSS 像素
    visible: false,
    auto_resize: false,   // ★ 必须 false，见禁忌 1
    transparent: false,
    initialization_script: Some(init_js),
})?;
// 随后 manager.update_rect(&id, rect) 定位，manager.set_visible(&id, true) 显示
```

---

## 三、禁忌清单（血泪，违反即翻车）

| # | 禁忌 | 后果 |
|---|---|---|
| 1 | **子 webview 开启 `auto_resize: true`** | 主窗 resize 时子 webview 被拉到主窗全高，与前端精确矩形定位互相覆盖，尺寸抖动 |
| 2 | **在 `force_allocation` 里调用 `set_size_request` + `queue_resize`** | 与父容器布局循环互相覆盖 → 无限 `force size_allocate` 死循环 → 拖死 GTK 主线程事件循环 → **网页不能点击 / 前进后退失效** |
| 3 | **前端坐标乘 `devicePixelRatio`** | 子 webview 被定位到 2x 偏移处，跑出主窗可见区 |
| 4 | **用 `display:none` 隐藏占位 div** | `getBoundingClientRect` 返回 0，子 webview 永远停在 (0,0,1,1) |
| 5 | **误判"wry 不更新 size_request"去补 `set_size_request`** | wry 0.55 `set_size()` 已正确更新 `size_allocate`+`size_request`+触发 WebKit 视口，手工干预是多余且有害的 |

---

## 四、Linux 尺寸修复的正确姿势（platform/linux.rs）

`force_allocation` 只做一件事：**当初始 allocation 仍是错的缓存态（≤1px 或 ≤400px）时，补一次 `size_allocate` + `queue_draw`，立刻返回**。

```rust
fn force_allocation(gtk_webview: &webkit2gtk::WebView, w: i32, h: i32) {
    let a = gtk_webview.allocation();
    let stuck = a.width() <= 1 || a.height() <= 1
             || a.width() <= 400 || a.height() <= 400;
    if (a.width() != w || a.height() != h) && stuck {
        let na = gtk::Allocation::new(a.x(), a.y(), w, h);
        gtk_webview.size_allocate(&na);
        gtk_webview.queue_draw();
    }
    // 注意：没有 set_size_request，没有 queue_resize
}
```

---

## 五、诊断三段式（定位问题层）

| 层 | 数值 | 不一致说明 |
|---|---|---|
| 前端 | `getBoundingClientRect()` 的 x/y/w/h | 计算错 → 前端问题 |
| GTK | `[apply_bounds] label=.. logical=(x,y,w,h)` 日志 | 与前端差很多 → 坐标换算错 |
| JS 视口 | `window.innerWidth/innerHeight` | 与 GTK 高度差很多 → WebKit 视口没刷新 |

---

## 六、事件流转（供前端订阅）

```
子 webview 内导航  → 插件 on_navigation → emit "browser-tabs://event"(NavigationFinished)
                   → 主程序 main.rs 监听 → 转发 "tab-navigated" → 前端 onTabNavigated 同步地址栏

target=_blank     → 插件 on_new_window(Deny) → emit "browser-tabs://event"(NewWindowRequested)
                   → 主程序 main.rs → 转发 "new-tab-request" → 前端 onNewTabRequest 开新页签
```
