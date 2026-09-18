# 03 — Lifecycle (Agent C)

## 1. Grid 真实状态（只用源码既有状态）

Grid 资源生命周期由 `gridOpen: boolean` + 原生窗存在性表达，无独立 enum。可映射为：

```
ABSENT     gridOpen=false，无原生窗
CREATED    gridOpen=true，create_grid 已建窗（可能隐藏）
VISIBLE    gridOpen=true 且 mainView∈{browser,grid}（窗口在屏）
HIDDEN     gridOpen=true 且 mainView∉{browser,grid}（窗口移屏隐藏，资源存活）
DESTROYED  gridOpen=false（= ABSENT，close_grid 已拆窗）
```

> 不新造 `GridLifecycle` enum（见 07-ALTERNATIVES）。`VISIBLE`/`HIDDEN` 是 `desiredGridVisibility` 的派生，不存储。

## 2. Browser 真实状态

Browser 主 webview 常驻（始终挂载于 MainArea）。其"可见"由 `isBrowserVisible`(=mainView==="browser") 经 CSS `visibility` 控制；"在屏/离屏"由 `schedulePosition`/`hideAllWebviews` 经原生 bounds 控制。Browser 资源**不随 Grid 开关销毁**——这是 B9-4 修复的核心（closeGridAll 只把 `mainView` 拨回 browser，不拆浏览器 webview）。

## 3. 当前生命周期（CURRENT）

| 转移 | 当前行为 | 代码 |
|------|----------|------|
| browser → grid | `buildGrid` 建窗 + `mainView="grid"`（若非 browser/grid） | useBrowserStore.ts:300-343 |
| grid → browser | 视路径：① `closeGridAll` 销毁 Grid 并 `mainView="browser"`；② 或 `setView("browser")` 仅切视图（Grid 仍叠加） | :483-511 / setView |
| grid → home | `setView("home")` → `hideAllWebviews` 隐藏 Grid（**资源存活**） | useBrowserStore.ts:646-655 |
| home → grid | `setView("grid")` + 若 gridOpen 则 `syncViewVisibility` 显示 | setView/watch |
| grid → workspace | `setView("files")` → 隐藏 Grid（资源存活） | 同上 |
| workspace → grid | `setView("grid")` | 同上 |
| explicit close grid | `closeGridAll`：销毁资源 + 翻位 + 拨 mainView | :483-511 |
| close single cell | `closeGridOne` → `gridCloseOne`；剩 1 格时调用 `closeGridAll` | :513-527 |
| app shutdown | Tauri 退出，所有 webview 释放 | — |

**CURRENT 问题**：
1. `mainView` 写入点散落 10+ 处 store/组件（见 05 §3），无单一入口。
2. `closeGridAll` 同时做"销毁"与"改 mainView"，意图耦合（Q6 要求解耦）。
3. `gridOpen=true, mainView=grid` 之后若 `closeGridAll` 漏拨 mainView → 浏览器区空白（B9-4 类）。目标态由 Visibility Controller 派生，从构造上消除。

## 4. 目标生命周期（TARGET）

原则：**VIEW SWITCH ≠ RESOURCE DESTROY。**

| 转移 | create? | show? | hide? | navigate? | destroy? | kill? |
|------|---------|-------|-------|-----------|----------|-------|
| browser → grid | 仅当 gridOpen=false 时 `openGrid` | Grid VISIBLE | （浏览器转 hidden 其下） | `activateGrid` | — | — |
| grid → browser | — | 浏览器 VISIBLE | Grid 仍叠加可见 | `activateBrowser` | — | — |
| grid → home | — | — | Grid HIDDEN | `activateHome` | — | — |
| home → grid | — | Grid VISIBLE | — | `activateGrid` | — | — |
| grid → workspace | — | — | Grid HIDDEN | `activateWorkspace` | — | — |
| workspace → grid | — | Grid VISIBLE | — | `activateGrid` | — | — |
| explicit close grid | — | — | — | （派生 `activateBrowser` 若 mainView==="grid"） | `closeGrid`：gridOpen=false + close_grid | — |
| close single cell | — | 其余重排 | — | — | `closeGridCell`：gridCloseOne | — |
| app shutdown | — | — | — | — | 全部 webview | Tauri quit |

硬性检查结论：
- **VIEW SWITCH != RESOURCE DESTROY**：`grid → home` / `grid → workspace` 只 hide 不 destroy（资源存活），已确认。
- `closeGrid` 是唯一 destroy 入口；它**不**改 `mainView`，仅置 `gridOpen=false`；若 `mainView==="grid"`，Visibility Controller 派生 `activateBrowser()`。
- `activate*` 只导航，绝不 destroy。

## 5. 不可重排不重建保证（来自 CURRENT 契约）

`check-grid-close-logic.mjs` 已固化：view switch 路径（含 `closeGridAll`）**不** recreate Grid、不 reload、不丢 URL、不丢页签输入。TARGET 必须保持：
- `activateBrowser()` 从 grid 视图切回：Grid 资源保留（`gridOpen` 不变），仅 `mainView` 变 → 原生 `hideAllWebviews`/`schedulePosition` 重定位，**不** `close_grid`。
- 只有 `closeGrid()` 才 `close_grid`。
