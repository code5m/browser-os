# 02 — Intent Model (Agent B)

先列真实用户意图，再设计 canonical Intent API。禁止先定 API 名。

## 1. 真实用户意图清单

| 用户动作 | 用户真正想要 | 当前代码路径 |
|----------|--------------|--------------|
| 点普通浏览器 Tab | 看该网页 | `tabSwitch(id)` → `tabActivate` + `relocate` |
| 点 Grid（宫格入口） | 打开/进入对比宫格 | `toggleGridToolbar()` / `buildGrid()`；`mainView="grid"` |
| 点 Home | 回主页快捷墙 | `setView("home")` |
| 点 Workspace/File | 进文件工作区 | `setView("files")` / `openDirTab` |
| 显式关闭 Grid | 销毁宫格资源 | `closeGridAll()`（`gridOpen=false` + `close_grid`） |
| 关闭单个 Grid cell | 销毁该子窗，其余保留 | `closeGridOne(i)`（`gridCloseOne`） |
| App shutdown | 释放所有 webview | Tauri 退出（`hideAllWebviews` 等） |
| Workspace teardown | 关闭文件工作区视图 | `setView(...)` 切走 → webview 移屏 |

观察：
- "看网页" 与 "进宫格" 是**两个独立意图**，不是同一动作的两阶段。
- "关闭 Grid" 的意图 = **销毁 Grid 资源**；"切回浏览" 是另一个意图（或关闭 Grid 的副作用）。
- 当前代码把 `buildGrid`/`closeGridAll` 同时做"资源变更 + 直接改 `mainView`"，混淆了意图边界。

## 2. Canonical Intent API（目标）

一个用户意图 → 一个 canonical entrypoint。设计：

```
// 视图导航（宿主表面切换）—— 唯一经此改 mainView
activateView(view: MainView)            // 替代散落 layout.mainView = "browser"
activateBrowser()  = activateView("browser")
activateGrid()     = activateView("grid")   // 若 gridOpen=false 则先 buildGrid
activateHome()     = activateView("home")
activateWorkspace()= activateView("files")

// Grid 资源生命周期（独立意图）
openGrid(opts?)   // buildGrid：创建 Grid 资源（gridOpen=true）；不直接改 mainView
closeGrid()       // closeGridAll：仅销毁 Grid 资源（gridOpen=false）；view→browser 是派生
closeGridCell(i)  // closeGridOne：销毁单格

// 页签意图（已有，归入同一 Intent 层）
switchTab(id) / closeTab(id) / newTab(url)
```

要点：
- `activateGrid()` 与 `openGrid()` 分离：`activateGrid` 是"我要看宫格"（若资源不在则建）；`openGrid` 是"确保宫格资源存在"（不一定切视图）。
- `closeGrid()` **只销毁资源**，不负责切视图。切回 `browser` 由 Visibility Controller 在 `desiredGridVisibility` 变 false 且 `mainView==="grid"` 时**自动派生**（见 03/04）。

## 3. 方案 A vs 方案 B 评估

### 方案 A
```ts
exitGrid("hide")    // 隐藏
exitGrid("destroy") // 销毁
```
多义万能 API：`exitGrid` 一个动词承担"隐藏/销毁/切视图"三义；`mode` 参数使小模型易误用（传错 mode = 资源泄漏或误销毁）；Checker 难静态分辨"本次是 hide 还是 destroy"。

### 方案 B
```ts
activateBrowser() / activateGrid() / activateHome() / activateWorkspace()
closeGrid() / shutdownGrid()
```
单一语义；每个 entrypoint 只做一件事；小模型误用概率低（无 mode 分支）；生命周期安全（closeGrid 永远销毁、activate* 永远导航）；调用方简单（无需记 mode 枚举）；Checker 可静态检测"closeGrid 未被当作 hide 用"。

### 维度评分

| 维度 | 方案 A | 方案 B |
|------|--------|--------|
| 单一语义 | ❌ 多义 | ✅ 单义 |
| 小模型误用概率 | 高（mode 错配） | 低 |
| 生命周期安全 | 中（hide/destroy 易混） | 高 |
| 调用方简单度 | 中 | 高 |
| Checker 可执行性 | 低（mode 难静态分辨） | 高 |
| Owner 清晰度 | 低 | 高 |

**采用方案 B。**

→ **Q5 = TARGET YES**（走 canonical View Intent）/ **CURRENT NO**（当前 `useBrowserStore`/`useSessionStore`/`App.vue` 等直接写 `mainView`，见 05 §3 证据）。
→ **Q6 = YES**（`closeGrid` 独立于 View Switch；view→browser 是派生后果，非 `closeGrid` 直接改 `mainView`）。

## 4. 与现有 API 映射

| 现有调用 | 目标替换 |
|----------|----------|
| `layout.mainView = "browser"`（useBrowserStore.ts:161,207,545 等） | `activateBrowser()` |
| `layout.mainView = "grid"`（useBrowserStore.ts:323） | `activateGrid()` |
| `layout.mainView = "browser"`（closeGridAll:503） | 删除：由 Visibility Controller 派生 |
| `layout.setView("term")`（App.vue:82,142） | `activateView("term")` |
| `toggleGridToolbar()`（useLayoutStore.ts:210） | `openGrid()` / `closeGrid()` 语义分流 |
