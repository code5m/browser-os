# 踩坑记录：Tauri 子 Webview 撑不满（WebKitGTK 卡 400px）——完整诊断与修复

> 项目：mvp-browser-os-v3（Tauri 2 + WebKitGTK 2.52.3 + Linux/X11 + Vue3 前端）
> 关联文档：`Tauri子Webview尺寸问题诊断.md`、`多页签卡死问题-跨专家圆桌纪要.md`
> 修复者：AI 助手（依据日志线索查 wry 源码定位根因）
> 日期：2026-08-15

---

## 一、问题现象（用户视角）

浏览器 OS 融合应用里，打开"百度"页签后：

1. **子 webview 内容区没有撑满**：页签栏 + 地址栏 + 状态栏（前端 Vue UI）位置正常，但百度网页只显示了上面约 400px 高的一截，下方大片空白（露出主 UI 背景）。
2. 窗口 resize、反复切换页签后依然如此，**永远卡在 400px 高度**。
3. 曾经怀疑过 Vue 框架/前端布局问题，甚至考虑过"去掉 Vue 换原生 Rust UI 控件"——**最终被证明与前端框架完全无关**。

---

## 二、首次修复（v0.2.0，部分有效但埋雷）

> ⚠️ 本节是**初次诊断结论**，其根因描述有误、修复手段引入了二次 bug。正确结论见第四节「二次翻车与最终根因」。

初次判断（**有误**）：wry 0.55.1 在 Linux/WebKitGTK 下对 `GtkFixed` 子 webview 执行 `set_bounds` 时只 `size_allocate` 不更新 `size_request`，导致被布局循环打回旧尺寸。

初次修复（`force_allocation` 里加了两行）：
1. `set_size_request(期望宽, 期望高)`；
2. `size_allocate` 后 `queue_resize()`。

这个修复让初始撑满生效了，但**制造了更严重的死循环**（见第四节）。

---

## 三、完整诊断时间线（逐步记录，含所有弯路）

### 第 1 轮：发现"子 webview 撑不满"

- 前端日志 `apply_bounds logical=(0,109,1200,665)` 完全正确（页签内容区从 y=109 开始、高 665），说明**前端计算的矩形没问题**，问题在原生层。
- 起初怀疑坐标系混乱（CSS 像素 vs 物理像素 ×devicePixelRatio）。修复了前端两处 `× devicePixelRatio`，统一为全链路 Logical(CSS) 坐标。**症状缓解但没根治**。

### 第 2 轮：查 wry 的 `add_child` 实现

- 读 wry 0.55.1 源码（`~/.local/share/cargo/registry/src/.../wry-0.55.1/src/webkitgtk/mod.rs`）：
  - `add_to_container`（590-628 行）：对 `GtkFixed` 只做 `set_size_request(w,h)` + `gtk::Fixed::put(webview, x, y)`，**不保证 allocation**。
  - `set_bounds`（853-875 行）：对 `is_in_fixed_parent` 的子控件只做 `webview.size_allocate(...)`，**不更新 size_request**。
- 初步结论：wry 的 GtkFixed 路径只做 set_size_request + put，allocation 无保证。当时据此写入了 TROUBLESHOOTING 第一条。

### 第 3 轮：主 webview 回归标准 WebviewWindow

- 当时主 UI 也用裸 `WindowBuilder` + `add_child` 承载，同样中招（主 webview CSS 视口卡在 ~400px，状态栏悬在窗口中间）。
- 修复：主 UI 改用标准 `WebviewWindowBuilder`（由 tao/wry 正常管理尺寸），页签/宫格仍走插件 `add_child`。
- 结果：主 UI 撑满解决，**但子 webview（页签）依旧卡 400px**。

### 第 4 轮：插件强制 `size_allocate`（第一版修复，治标）

- 在插件 linux 平台层加 `force_allocation`：对比 GTK allocation 与期望尺寸，不匹配则手动 `size_allocate` + `queue_draw`。
- 观察日志：
  ```
  [browser-tabs] force size_allocate: (1x1) -> (1200x665)
  [browser-tabs] force size_allocate: (1200x400) -> (1200x665)   ← 关键线索！
  ```
- **第二次 force 从 (1200x400) 开始**：说明第一次 force 到 665 之后，GTK 布局循环又把子 webview 分配回了 (1200x400)。**这证明 force 是临时的，布局循环会撤销它。**
- 之后 force 不再触发（allocation 稳定 1200x665），**但视觉仍没撑满**——说明 GTK allocation 对了 ≠ WebKit CSS 视口对了。两层是分离的。

### 第 5 轮：临时 viewport 探针，证实"视口与 allocation 分离"

- 在 `apply_bounds` 后延迟 eval JS 读取 `window.innerWidth/innerHeight` 经 `report_title` 回传打日志：
  ```js
  var vp = window.innerWidth + 'x' + window.innerHeight;
  if (window.__vpReported !== vp) { window.__vpReported = vp;
    window.__TAURI__.core.invoke('report_title', { title: '[vp] ' + vp }); }
  ```
- 结合第 4 轮现象确认：GTK widget allocation = 1200x665，但 WebKit 内部 viewportSize 可能仍停在 400px，需要完整布局迭代才能刷新。

### 第 6 轮：根因修复（治本）

对照 wry 源码补上两个缺口：

```rust
// tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/linux.rs
fn force_allocation(gtk_webview: &webkit2gtk::WebView, w: i32, h: i32) {
    // 1) 补 size_request：GtkFixed 布局循环按它分配，wry 从不更新它
    if gtk_webview.width_request() != w || gtk_webview.height_request() != h {
        gtk_webview.set_size_request(w, h);
    }
    // 2) allocation 不匹配才手动 force
    let allocation = gtk_webview.allocation();
    if allocation.width() != w || allocation.height() != h {
        gtk_webview.size_allocate(&gtk::Allocation::new(allocation.x(), allocation.y(), w, h));
        gtk_webview.queue_draw();
        // 3) 排队完整布局迭代：WebKit 视口刷新依赖 size-allocate 信号链
        gtk_webview.queue_resize();
    }
}
```

**修复后**：`force size_allocate` 日志只出现一次（说明不再被布局循环打回），百度页签内容完整撑满内容区。

### 第 7 轮：清理探针、收尾

- 删除临时 viewport 诊断代码（bridge.rs 的 `[vp]` 探针 + report_title 的 `[vp]` 拦截），只保留正式修复。
- 组件库 `tauri-browser-tabs` 从仓库外部复制进本仓库（排除 target/node_modules），Cargo.toml path 改为 `../tauri-browser-tabs/...`，`cargo check` 通过。

---

## 四、技术原理详解

### 4.1 GTK 布局机制（为什么 size_request 是隐形成员）

GTK3 中，容器（如 `GtkFixed`）在每次布局迭代（`check_resize`）时对子控件分配尺寸：

- 子控件的分配尺寸来源于 **`size_request`**（通过 `gtk_widget_set_size_request` 设置）；
- `gtk_widget_size_allocate()` 是**直接设置 allocation 并发出 `size-allocate` 信号**，**不会更新 size_request**；
- 因此：手动 `size_allocate` 到 665 → 下次布局循环按 size_request（旧的 400）重新分配 → **尺寸被打回**。

这就是为什么单独 force `size_allocate` 只能治标。

### 4.2 wry 0.55.1 的 GtkFixed 路径缺口

| wry 方法 | 对 GtkFixed 子控件做的事 | 缺口 |
|---|---|---|
| `add_child` → `add_to_container` | `set_size_request(w,h)` + `gtk::Fixed::put(x,y)` | 不保证 allocation |
| `set_bounds` | 仅 `webview.size_allocate(...)` | **从不更新 size_request** |

两个方法各管一半、互不衔接，形成结构性缺陷。

### 4.3 WebKitGTK 的 CSS 视口 ≠ GTK widget allocation

- GTK 层：widget 的 `allocation`（绘制区域）；
- WebKit 层：`WebKitWebViewBase` 内部的 `viewportSize`（决定页面 CSS 布局的 window.innerWidth/Height）；
- 两者的同步发生在 **WebKitWebViewBase 的 `size-allocate` 信号处理**里；
- 手动 `gtk_widget_size_allocate()` 虽会触发 `size-allocate` 信号，但 WebKit 的 backing store / 视口刷新在**完整布局迭代**路径下才可靠执行；
- 所以：`queue_resize()`（让 GTK 在下一轮主循环走正规 allocate 流程）是让视口真正跟上的关键一步。

### 4.4 "400px" 是什么

WebKitGTK 子 webview 在未完成首次完整布局时的**默认/缓存分配高度**（常见表现为 1200x400）。任何只改 allocation、不触发完整布局的 resize 都会卡在这个值上。

---

## 五、迁移到 browser-tabs 插件的要点（顺手记录）

1. **全链路 Logical(CSS) 坐标**：前端 `getBoundingClientRect` 原值直传，**不再乘 devicePixelRatio**；插件内部统一 `LogicalPosition`/`LogicalSize`。
2. **子 webview 用 `initialization_script` 注入采集脚本**（collect.js），而非 eval。
3. **`window.open` / `target="_blank"` 由插件 `on_new_window` 拦截为 `browser-tabs://event` (newWindowRequested)**，main.rs 转发为前端既有 `new-tab-request`。
4. **懒绑定宿主窗口**：插件在每次操作时重新 `get_window(host_label)`，兼容"插件 setup 先于窗口创建"的场景。
5. **Linux 特化处理全部收敛在 `platform/linux.rs`**，非 Linux 平台为空操作。

---

## 五、二次翻车与最终根因（v0.3.1，本轮回填）

### 现象
v0.2.0 修复后，用户实测反馈：**网页内链接不能点击、前进/后退按钮无效**（地址栏、页签栏这些前端 UI 本身可见）。日志出现持续刷屏：

```
[browser-tabs] force size_allocate: (1200x800) -> (1200x665)
[apply_bounds] label=tab-1 logical=(0,109,1200,665)
...（每帧重复几十次）...
```

### 真根因（推翻初次诊断）
1. **初次诊断的"wry 从不更新 size_request"是误判**。wry 0.55 的 `set_size()` 实际上**正确同时更新了物理 `size_allocate` 和 `size_request`**，WebKitGTK 的 CSS 视口也随 `set_size` 正常刷新。撑满问题在 `set_size` 层面本已解决。
2. **真正的元凶是我加的 `set_size_request` + `queue_resize`**：
   - 我把子 webview 的 `size_request` 设为固定值（如物理 665×scale），而父容器（主窗内容区 / `GtkFixed`）实际只给它分配 665 高；
   - 但 `auto_resize: true` 让子 webview 监听主窗 resize，把尺寸拉到主窗全高（日志里的 **800px**）；
   - 于是 `allocation(800) ≠ size_request(665)` → 我的 `force_allocation` 又补 `size_allocate(665)` → 又 `queue_resize()` → GTK 重布局 → 又按 size_request 分配 → 又被 auto_resize 拉成 800 → **无限 force 死循环**。
3. **"不能点击 / 前进后退失效"正是死循环的副作用**：每帧几十次 `queue_resize` + `size_allocate` 占满 GTK 主线程事件循环，输入事件（鼠标点击、按钮回调解）被严重延迟或饿死，表现为"点了没反应"。

### 最终修复（删代码，而非加代码）
| 位置 | 改动 |
|---|---|
| `platform/linux.rs` `force_allocation` | **删除** `set_size_request` 与 `queue_resize`。只在初始 allocation 仍是错的（≤1px 或 ≤400px，即 WebKit 初始缓存态）时补一次 `size_allocate` + `queue_draw`，且立刻返回，绝不重入布局。 |
| `commands.rs` `create_tab` | **关闭 `auto_resize`**（注释掉 `builder.auto_resize()`）。尺寸完全由前端 `ResizeObserver` + `tab_position` 精确控制，不再与 wry 的自动跟随打架。 |

### 验证预期
- 日志里**不应再出现反复 force**（最多一次初始 1x1→目标尺寸）；
- 主窗 resize 时子 webview 跟随前端重新 `tab_position`，无 800↔665 抖动；
- 网页链接可点击、前进/后退恢复（事件循环不再被拖死）。

---

## 六、经验总结（给后来者）

1. **前端 rect 对了而视觉不对 → 直接怀疑原生层**，别在 CSS/Vue 上浪费时间。用日志对比"前端计算的 rect"与"GTK allocation"与"JS window.innerWidth/Height"三层数值，能迅速分层定位。
2. **（修正）不要盲目给 GTK 子控件设 `set_size_request` + `queue_resize`**。wry 0.55 的 `set_size()` 本已正确更新 `size_allocate` 与 `size_request` 并触发 WebKit 视口刷新。手工干预 `size_request` 反而会与父容器布局循环互相覆盖，造成无限 `force` 死循环。只在"初始 allocation 仍是错的（≤1px/≤400px 缓存态）"时补一次 `size_allocate` + `queue_draw` 即可。
3. **（修正）`auto_resize` 与手动定位是天敌**：子 webview 一旦开 `auto_resize`，就会在主窗 resize 时拉到主窗全高，与前端按 Container 精确矩形定位互相覆盖。二选一——本项目选"前端精确控制，关闭 auto_resize"。
4. **"不能点击 / 按钮失效"往往是事件循环被拖死，不是链路断了**：持续刷屏的 `force size_allocate` 每帧几十次 `queue_resize`，会饿死 GTK 主线程输入事件。先看日志有没有死循环，再怀疑 IPC。
5. **加日志要看"重复频率"**：一次 force（1x1→目标）是正常初始化；每帧重复几十次才是死循环信号。
6. **读源码要读全，别凭片段下结论**：初次误判源于只看到 `set_bounds` 的 `size_allocate` 片段，没确认它配套更新了 `size_request`。
7. **分层验证三段式**：前端 rect → GTK allocation → JS viewport，任何一段不一致都能立即锁定问题层。
8. **修复后要删掉临时诊断代码**，只留正式修复 + 本文档。

---

## 七、相关代码位置

| 文件 | 内容 |
|---|---|
| `tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/linux.rs` | 根因修复（set_size_request + size_allocate + queue_resize） |
| `tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/mod.rs` | apply_rect / create_child_webview 的 Linux 钩挂点 |
| `tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs` | TabManager（create_tab/update_rect/set_visible/close_tab/navigate） |
| `src-tauri/src/main.rs` | 主窗口用标准 WebviewWindowBuilder；转发 newWindowRequested → new-tab-request |
| `src-tauri/src/bridge.rs` | spawn_child_window / apply_bounds 已迁移到插件 API |
| `src/composables/useBrowserHost.ts` | 前端去掉 ×devicePixelRatio，直传 CSS 像素 |
