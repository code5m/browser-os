# 04 — Native Visibility (Agent D)

## 1. 现有 Native 显隐手段（证据）

| 前端调用 | bridge → Rust 命令 | 作用 |
|----------|-------------------|------|
| `bridge.tabPosition(id, rect)` | `tab_position` | 定位浏览器页签 webview（在屏=可见，离屏=隐藏） |
| `bridge.gridPosition(i, rect)` | `grid_position` | 定位 Grid 子窗 |
| `bridge.hideWebview(id)` | `hide_webview` | 单窗移出屏幕 |
| `bridge.hideAllWebviews()` | `hide_all_webviews` | 全部子窗移出屏幕（无去重、强制） |
| `bridge.closeGrid()` | `close_grid` | 销毁 Grid 原生窗 |
| `bridge.createGrid(n, urls)` | `create_grid` | 创建 Grid 原生窗 |

关键事实：**Rust 侧没有 `show_*` 命令**。显示 = 用**在屏 bounds** 调 `tab_position`/`grid_position`；隐藏 = 用**离屏 bounds**（`-30000` 等）调定位或 `hide_*`。
即：**position 同时承担 show/hide 语义**（on-screen = show, off-screen = hide）。

## 2. Native visibility 是什么？

**裁决：native visibility 是 domain 状态的执行结果，不是域真源。**

证据链：
- 域真源只有 `mainView` + `gridOpen`。
- `syncViewVisibility()`（useBrowserStore.ts:646-655）依 `mainView` 决定调 `hideAllWebviews`（其它视图）还是 `relocate`（browser/grid）。
- `watch(mainView)`（useBrowserStore.ts:658-664）→ `nextTick(syncViewVisibility)`。
- `useBrowserHost.schedulePosition` / `scheduleGrid`（composable）把 `mainView`+`gridOpen` 翻译成 `tab_position`/`grid_position` bounds。
- `isBrowserVisible` 经 CSS `visibility` 显隐浏览器 webview（BrowserHost.vue:15），也是派生。

故 native 的 show/hide/position 是 **Visibility Controller 的输出**，不应被任何模块当作"真相"回读。

→ **Q4 = YES**（native visibility 只是执行结果）。

## 3. 目标模型

```
Desired State  (mainView + gridOpen)
      │
      │  isBrowserView() / desiredGridVisibility / isBrowserVisible
      ▼
Visibility Controller   ← 新增薄层（纯函数，可单测）
      │  输入：mainView, gridOpen, activeTabId
      │  输出：每个 webview 的 desired native 状态（browser: show/hide/offscreen; grid: show/hide）
      ▼
Native Effect   (bridge.tab_position / grid_position / hide_all_webviews / close_grid)
```

Visibility Controller 职责（不新增 Rust 命令，仅编排既有 bridge 调用）：
- `mainView==="browser"` → 浏览器 webview 在屏（`schedulePosition`）；Grid 若在 `gridOpen` 则叠加在屏（`scheduleGrid`）。
- `mainView==="grid"` → Grid 在屏（`scheduleGrid`）；浏览器 webview `visibility:hidden` 但仍留 rect（isBrowserVisible=false）。
- `mainView` 为其它 → `hideAllWebviews`（浏览器 + Grid 均离屏，资源存活）。

## 4. `position → show` 契约裁决

当前契约：`gridPosition`/`tabPosition` 既定位又隐式 show（在屏 bounds）。

是否拆成 `position` / `show` 两个命令？按以下维度裁决：

| 维度 | 保留契约 | 拆分 |
|------|----------|------|
| 风险 | 低（已稳定，B9-4 修复依赖它） | 高（需新 Rust 命令 + 全量迁移 + 去重缓存改造） |
| 兼容性 | 保持 | 破坏既有 `create_grid`/`close_grid`/定位去重逻辑 |
| 调用数量 | 不变 | +1 命令/窗，定位与 show 需配对，易漏 |
| 小模型理解 | 直观（on-screen=show） | 需理解两命令配对语义 |
| 测试难度 | 已有 `check-grid-close-logic.mjs` 覆盖 | 需重写定位+show 双路径测试 |

**裁决：保留 `position → show` 契约。** 不因为函数名"不好看"就重构（违反 §2 禁止预设/不扩大重构）。新增的是**前端 Visibility Controller 派生层**，不是新的 Rust 命令。

→ **不引入新 Rust show/hide 命令**（见 07-ALTERNATIVES）。
