# 项目规范（锁定规则，禁止随意改动）

> 本文件记录**已经过实测验证的正确做法**。凡标注【锁定】的规则，改动前必须先有充分的实测证据（日志 + 截图），并回滚验证，否则一律拒绝。
> 背景：本仓库曾因"凭猜测修改已验证的正确代码"导致回退性 bug，故建立本规范。

---

## 规则 1【锁定】子 WebView 尺寸撑满：三件套缺一不可

文件：`tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/linux.rs` 的 `force_allocation`

**必须同时存在，删任何一行都会回退 bug：**

1. `set_size_request(期望宽, 期望高)`
   - 作用：GtkFixed 布局循环按 `size_request` 分配子控件尺寸。wry 的 `set_bounds` 只直接 `size_allocate`、**不更新 size_request**，缺这行 → 窗口 resize / 重布局时子 webview 被打回旧尺寸（1x1 / 400px）。
2. `size_allocate(...)` + `queue_draw()`
   - 作用：立即生效当前 allocation。
3. `size_allocate` 后 `queue_resize()`
   - 作用：直接 `size_allocate` 只改 widget allocation，WebKitGTK 的 **CSS 视口刷新依赖完整 GTK 布局迭代**。缺这行 → 视口不刷新，网页显示区域错误。

**禁止的修改方向**（已被证明错误）：
- ❌ 以"避免死循环"为由删除 `set_size_request` 或 `queue_resize`
- ❌ 认为"wry 已正确更新 size_request，无需干预"（这是 0.3.1 误判的根因）

---

## 规则 2【锁定】子 WebView 必须开启 `auto_resize: true`

文件：`tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs` 的 `create_tab`

```rust
if options.auto_resize {
    builder = builder.auto_resize();
}
```

- 作用：让子 webview 跟随主窗 resize 自动调整，配合规则 1 的 `set_size_request` + `queue_resize` 保证撑满。
- **禁止**以"与前端精确定位冲突"为由关闭 `auto_resize`（0.3.1 误判的根因）。

---

## 规则 3【锁定】前端坐标：纯 CSS 像素，绝不乘 devicePixelRatio

文件：`src/composables/useBrowserHost.ts` 的 `schedulePosition` / `scheduleGrid`

- 坐标一律 `Math.round(r.left/top/width/height)`，**绝不乘 `devicePixelRatio`**。
- DPI 换算由 Tauri / wry 统一处理。
- 占位 div 用 `visibility:hidden` 而非 `display:none`（保证 `getBoundingClientRect` 非零）。

---

## 规则 4 修改"已验证正确"代码的流程（防止再次翻车）

当要修改本文件中标注【锁定】的代码时，必须：

1. **先复现问题**：用日志 + 截图证明现状有 bug，且 bug 与该锁定代码直接相关。
2. **提出假设并最小验证**：改 1 行，重新编译，实测确认假设成立；不成立立刻回滚。
3. **不得凭"直觉/代码看上去不对"删改**：尤其不得凭"怀疑是死循环/多余"就删 `set_size_request`/`queue_resize`/`auto_resize`。
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
