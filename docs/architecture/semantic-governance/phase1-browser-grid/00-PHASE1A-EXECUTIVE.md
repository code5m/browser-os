# Phase 1A — Browser / Grid Architecture Design & Adjudication (Executive)

> 设计阶段（DESIGN ONLY）。本轮不修改任何 `src/**`、`src-tauri/**`、`scripts/**`、`package.json`、`Cargo.toml`、`AGENTS.md`。
> 仅落盘本目录下的 Markdown。Phase 1B 实施将在后续任务中独立派发。

## 0. 目标

彻底裁决 Browser / Grid 的：

- 单一语义（single semantics）
- 状态模型（state model）
- 生命周期（lifecycle）
- Intent API（canonical entrypoint）
- Owner 与 Native visibility 关系

并产出可并行的 Phase 1B 实施计划 + 静态可检 Checker 设计。

## 1. 八问裁决速览

| # | 问题 | 裁决 |
|---|------|------|
| Q1 | `mainView` 是否就是 desired visible main surface？ | **YES**（`mainView` = 用户当前意图可见的主表面；Grid 是 `mainView==="grid"` 这一可选**主表面**，**非**叠加维度 —— Final Reconciliation §2.2 修正） |
| Q2 | `gridOpen` 是否只表示 Grid resource existence？ | **YES**（存在 ≠ 可见；可见还需 `mainView`） |
| Q3 | `gridVisible` 是否需要？ | **NO**（派生：`desiredGridVisibility = gridOpen && mainView === "grid"`，无需存储 —— Final Reconciliation §2.1 冻结公式） |
| Q4 | native visibility 是否只是执行结果？ | **YES**（由 `syncViewVisibility` / `hideAllWebviews` / `schedulePosition` 产生，非域真源） |
| Q5 | Browser/Grid view switch 是否走 canonical View Intent？ | **YES（Phase 1B 已实施）**：view switch 统一经 canonical Intent；`mainView` 唯一写入点 = `useLayoutStore.setView` |
| Q6 | `closeGrid` 是否必须独立于 View Switch？ | **YES（Phase 1B 已实施）**：`closeGrid` 只销毁 resource；view→browser 由 owner 不变量守卫派生，组件不得手拼 `closeGrid(); setView()` |
| Q7 | Component 是否只能调用 Intent API？ | **YES（Phase 1B 已实施）**：组件只调 Intent API；全仓已无 `.mainView =` 直写（`check-view-intent.mjs` C1/C2 守护） |
| Q8 | lifecycle owner 是谁？ | **`useBrowserStore`（browser store）为唯一 canonical owner**；`useBrowserHost` composable = 定位/可见性引擎（execution）；`useLayoutStore.setView` = 唯一被授权的 `mainView` 底层 setter，仅经 Intent API 调用 |

## 2. 状态模型结论（Model B）

拒绝 Model A（`mainView`+`gridOpen`+`gridVisible`+`nativeVisible` 四真源）。
采用 Model B（减少真源）：

```
mainView          (useLayoutStore)  — 用户意图可见主表面
gridOpen          (useBrowserStore)  — Grid 资源是否存在
        │
        ├─ isBrowserView()      = mainView ∈ {browser, grid}
        ├─ isBrowserVisible     = mainView === "browser"        (CURRENT 公式)
        └─ desiredGridVisibility = gridOpen && mainView === "grid"  (派生，不存)
                │
                ▼ Visibility Controller
        Native Effect (show/hide/position via bridge)
```

## 3. 文档索引

- `01-STATE-MODEL.md` — Agent A：状态真源、合法状态、`gridVisible` 裁决
- `02-INTENT-MODEL.md` — Agent B：真实用户意图、canonical Intent API、方案 A/B 评估
- `03-LIFECYCLE.md` — Agent C：当前/目标生命周期 + 全转移矩阵
- `04-NATIVE-VISIBILITY.md` — Agent D：native visibility = 执行结果；position→show 契约保留
- `05-OWNERSHIP.md` — Agent E：Owner/Component 边界、decision vs execution
- `06-TEST-AND-CHECKER-DESIGN.md` — Agent F：验收场景 + Checker 规则（STATIC/RUNTIME/NOT_RELIABLE）
- `07-ALTERNATIVES.md` — 被拒设计：`gridVisible` / `exitGrid(mode)` / 新 Rust show/hide / `GridLifecycle` enum
- `08-ADR.md` — 正式架构决策记录
- `09-REVIEW.md` — 独立 Reviewer 裁决（攻击 + 结论）
- `10-IMPLEMENTATION-PLAN.md` — Phase 1B 可并行子任务 + 验收/回滚

## 4. 约束遵守

- 禁止预设 `exitGrid(mode)` / `gridVisible` / 新 `GridLifecycle` enum / 新 Rust show/hide command —— 见 `07-ALTERNATIVES.md`。
- `position → show` 契约保留（不因为命名难看就拆）；新增薄 Visibility Controller 层派生 show/hide。
- 不扩大重构范围；仅新增 Intent API + 收敛 `mainView` 写入点。

## 5. Phase 1B 执行结果（已实施，未 commit/push）

### 5.1 落地改动
- `useLayoutStore` = **View Navigation owner**：新增 `activateView/activateBrowser/activateHome/activateFiles/activateTerm/activateEditor/activateWorkspace`；`setView` 为 `mainView` 唯一写入点。
- `useBrowserStore` = **Browser/Grid Resource Lifecycle owner**：新增 `openGrid/rebuildGrid/closeGrid/closeGridCell/activateGrid` + 纯派生 `computeDesiredVisibility` / `desiredGridVisibility` + 状态不变量守卫 watch。
- 组件迁移（只表达意图）：`App.vue` `ActivityBar.vue` `UnifiedTabBar.vue` `HomeLaunchers.vue` `BookmarkPanel.vue` `FileEditor.vue` `TopBar.vue` + `useWorkspaceStore/useSessionStore/useWorkbenchStore` 相关调用点。
- 全仓 `.mainView =` 直写 **0 处**；组件直调 `buildGrid/closeGridAll/gridCloseOne` **0 处**。

### 5.2 新增门禁
- `scripts/check-view-intent.mjs`（C1–C11 + T11/T12 契约仿真），已接入 `package.json` 的 `npm run check` 与 `scripts/pre-merge.sh`（默认门禁 + `--self-test`）。
- `scripts/check-grid-close-logic.mjs` G1 放宽为同时接受 `mainView = "browser"` 与 `setView("browser")`（改用 canonical 入口不算回归）。

### 5.3 独立 Review 抓出并已修复的 2 处真实回归
1. **不变量守卫与创建流程竞争**：`activateGrid` 同步置 `mainView="grid"`，而 `buildGrid` 的 `createGrid` 是异步 IPC（`gridOpen` 在其 resolve 后才为 true）；守卫按当前值判断会把**首次打开宫格弹回 browser**（须点两次）。
   → 修复：守卫改用 `gridOpen` 的**前值**，仅在 `true→false`（资源消失）时收敛；"创建中"是合法中间态。
2. **`openGrid()` 在 `gridOpen===true` 时为 no-op**，导致改格数 / 切四分不再重建（后端 webview 数量须与 `gridCount` 一致）。
   → 修复：新增 `rebuildGrid()`（真重建），`setGridCount/setGridLayout` 改走它；`openGrid()` 保持"确保资源存在"语义。

### 5.4 验证
- `npm run build` PASS；`npm run check` PASS；`check-view-intent.mjs` 11/11 + 契约仿真全绿；`check-grid-close-logic.mjs` 12/12 PASS；build metrics 22.77% ≤ 25.2%；`git diff --check` 干净。
- `pre-merge.sh --self-test` 的 2 项失败（`check-terminal-policy.py`、`check-terminal-ui-logic.mjs`）为**既有债**，与本次改动无关（本次未触碰任何终端文件）。

### 5.5 遗留债（显式，不静默消失）
- `closeGridCell` 定义但无调用方（单格关闭 UI 在当前版本已不存在，与 `closeGridOne` 同状态）。
- `useLayoutStore.toggleGridToolbar` 为死代码，且函数体内 `useBrowserStore()` **未 import**（既有问题，非本次引入；零调用者故不触发；改为静态 import 会形成 layout↔browser 循环依赖，故不擅自改）。
- 契约仿真为源码级镜像（未从 TS 导入），公式一致性由 C5 静态断言兜底。
