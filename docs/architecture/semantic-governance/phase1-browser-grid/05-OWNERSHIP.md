# 05 — Ownership / Component Boundary (Agent E)

## 1. 现状：谁在编排 lifecycle？

扫描直接写 `mainView` / 调 `buildGrid`/`closeGridAll` 的点（证据，来自 `rg`）：

| 文件:行 | 直接动作 |
|---------|----------|
| useBrowserStore.ts:161 | `layout.mainView = "browser"`（tabNew） |
| useBrowserStore.ts:207 | `layout.mainView = "browser"`（closeTabNow 无页签时） |
| useBrowserStore.ts:323 | `layout.mainView = "grid"`（buildGrid） |
| useBrowserStore.ts:502-503 | `layout.mainView = "browser"`（closeGridAll，B9-4） |
| useBrowserStore.ts:545 | `layout.mainView = "browser"`（openBrowser） |
| useSessionStore.ts:120 | `layout.mainView = "browser"` |
| useWorkbenchStore.ts:19 | `layout.setView("browser")` |
| useWorkspaceStore.ts:912,924 | `layout.mainView = "editor"` |
| useHomeStore.ts:238 | `layout.setView("browser")` |
| useSystemStore.ts:326 | `layout.setView("term")` |
| App.vue:82,142,195 | `layout.setView("term")` / `toggleBrowserDock` |
| FileEditor.vue:11 | `layout.mainView = "browser"` |
| HomeLaunchers.vue:49 | `layout.setView("browser")` |
| MainArea.vue:151,155 | 用 `browser.gridOpen` 决定 `grid-mode` class / `v-show` |

结论：**`mainView` 写入点散落 10+ store/组件**，且 `useBrowserStore` 既管资源又直接改视图——意图边界混乱。

## 2. 组件当前如何"自己拼" lifecycle

- `MainArea.vue` 直接读 `browser.gridOpen` 决定 `grid-mode` class 与 `BrowserHost` 的 `v-show`（布局耦合）。
- `useBrowserStore.buildGrid`/`closeGridAll` 内部直接 `layout.mainView = ...`（store 跨边界改视图）。
- 各 store（session/workbench/workspace/system/home）按需直接置 `mainView`（绕过任何 owner）。

→ **Q7（CURRENT）= NO**（组件/store 直接编排 lifecycle，未统一经 Intent API）。

## 3. 目标：单一 canonical owner

**裁决：`useBrowserStore`（browser store）为 Browser/Grid 生命周期唯一 canonical owner。**

决策/执行拆分：

| 角色 | 模块 | 职责 |
|------|------|------|
| **Decision（owner）** | `useBrowserStore` | 拥有 `gridOpen` 真源；决定 build/destroy 时机；计算"应处哪个视图"（派生 `activateView` 调用）；暴露 canonical Intent API（`activateBrowser/activateGrid/openGrid/closeGrid/closeGridCell`） |
| **Execution（引擎）** | `useBrowserHost` composable | 定位/可见性引擎：`schedulePosition`/`scheduleGrid`/`layoutGridNow` 把 `mainView`+`gridOpen` 翻译成 `bridge.tab_position`/`grid_position`；绑定到 `useBrowserStore` 调度器 |
| **Execution（native）** | `bridge` → Rust | `create_grid`/`close_grid`/`grid_position`/`tab_position`/`hide_all_webviews`（仅执行，无决策） |
| **State holder（被授权）** | `useLayoutStore.setView` | 唯一被授权的 `mainView` 底层 setter；**仅**经 Intent API 调用，禁止 store/组件直接写 `mainView` |

`useLayoutStore` 仍持有 `mainView` 状态，但**不再做任何 Browser/Grid 生命周期决策**——它只是 `setView` 的执行点。所有"切视图"意图必须经由 `useBrowserStore` 的 `activateView`（owner 决策）而非散落写入。

## 4. 组件边界（TARGET）

```
Component (MainArea / UnifiedTabBar / HomeLaunchers / ActivityBar / FileEditor)
      │  只调用 Intent API
      ▼
useBrowserStore.activateView / openGrid / closeGrid / closeGridCell / switchTab
      │  owner 决策
      ▼
useBrowserHost (engine) + bridge (native)  ← 单一执行链
```

- `MainArea.vue` 不再直接读 `gridOpen` 决定 `v-show`/`grid-mode`；改为订阅由 Visibility Controller 产出的 `browserVisible`/`gridVisible` computed（派生，不新增真源）。
- 所有 store 删除直接 `layout.mainView = ...`；改为调 `activateView`。
- `closeGridAll` 删除 `mainView = "browser"`（B9-4 手动拨位）→ 改为：置 `gridOpen=false` 后，若 `mainView==="grid"` 调 `activateBrowser()`（由 owner 派生，统一入口）。

→ **Q7（TARGET）= YES**（组件只调 Intent API）；**Q8 = `useBrowserStore`**。

## 5. 为什么不是 `useLayoutStore` 当 owner

`useLayoutStore` 管全应用 20+ 视图导航（home/files/term/...），若让它兼管 Browser/Grid 资源 lifecycle，会把"资源 build/destroy"耦合进通用导航 store，且它当前不持有 `gridOpen`/`schedulePosition`——会反向依赖 `useBrowserStore`，形成循环。故 Browser/Grid 资源 lifecycle 归母属 store（`useBrowserStore` + `useBrowserHost` 引擎），`useLayoutStore` 退为"视图状态执行点"。
