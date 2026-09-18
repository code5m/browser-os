# 01 — State Model (Agent A)

研究文件：`src/stores/useLayoutStore.ts`、`src/stores/useBrowserStore.ts`、`src/composables/useBrowserHost.ts`、`src/components/layout/MainArea.vue`、`src/components/browser/BrowserHost.vue`、`src/bridge.ts`。

## 1. 现有状态真源（证据）

| 状态 | 类型 | 真源文件:行 | 含义 |
|------|------|-------------|------|
| `mainView` | `ref<MainView>` | useLayoutStore.ts:138 | 当前激活的 **Main Surface**（含 `browser`/`grid`/`home`/`files`/`term`/...） |
| `gridOpen` | `ref<boolean>` | useBrowserStore.ts:20 | Grid **资源**（原生窗）是否存在 |
| `gridToolbarOpen` | `ref<boolean>` | useLayoutStore.ts:144 | 宫格工具条是否展开（UI 入口开关） |
| `gridSession` | `ref<number>` | useBrowserStore.ts:23 | 每次 `buildGrid` +1，定位缓存失效键 |
| `activeTabId` | `ref<string>` | useBrowserStore.ts:19 | 当前激活浏览器页签 |
| `isBrowserView()` | fn | useLayoutStore.ts:189 | `mainView === "browser" \|\| mainView === "grid"` |
| `isBrowserVisible` | computed | useBrowserStore.ts:149-151 | **`mainView === "browser"`**（CURRENT 公式，与 `gridOpen` 解耦） |
| native 显隐 | — | bridge bounds | 执行结果，**非域状态** |

关键事实：
- `mainView` 是**全应用唯一**的"当前主表面"判定（`isNavActive` 也只用它，useLayoutStore.ts:101）。
- `isBrowserView()` 把 `browser` 与 `grid` 视为同一"浏览器类视图"——二者共用 `MainArea` 的同一模板与 `BrowserHost` 布局桩（MainArea.vue:139,155）。
- `BrowserHost` **始终挂载**，用 CSS `visibility` 而非 `display:none` 控制显隐（BrowserHost.vue:15），目的：保证 `getBoundingClientRect` 永远非零，供宫格定位（MainArea.vue:151-155 注释）。

## 2. `mainView` 到底表示什么？

**裁决：只表示"用户当前意图可见的 Main Surface"。** 不承载 lifecycle/owner 语义。

**Phase 1 Final Reconciliation（§2.2）修正**：Grid **不是叠加维度**，而是 `mainView` 的可选主表面之一。

`gridOpen=true` 且 `mainView==="browser"` 时，Grid 原生窗**不在屏**：`syncViewVisibility` 先 `hideAllWebviews()` 把全部子窗移出屏幕，随后只把浏览器 webview 重定位回屏；Grid 未被重定位（`scheduleGrid` 仅在 `mainView==="grid"` 时调用），故**隐藏且资源存活**（useBrowserStore.ts `syncViewVisibility` / useBrowserHost.ts `schedulePosition`）。

因此"是否看到 Grid" = `gridOpen && mainView === "grid"`（唯一冻结公式，见 §5）。

→ **Q1 = YES**（`mainView` = desired visible main surface；Grid 是 `mainView==="grid"` 这一可选**主表面**，不是 overlay）。

## 3. `gridOpen` 到底表示什么？

**裁决：只表示 Grid 资源（原生窗）是否存在，不表示可见性。**

证据：
- `buildGrid()` 在 `create_grid` 成功后置 `gridOpen.value = true`（useBrowserStore.ts:319）。
- `closeGridAll()` 先翻 `gridOpen.value = false`（useBrowserStore.ts:486），再 `bridge.closeGrid()`。
- 可见性还需 `mainView` 参与：`syncViewVisibility` 在 `mainView` 非 `browser/grid` 时 `hideAllWebviews()`（useBrowserStore.ts:646-655）——即 `gridOpen=true` 但用户在 `files` 视图时，Grid 窗口被移出屏幕（隐藏但资源存活）。

→ **Q2 = YES**（`gridOpen` 只表示 resource existence）。

## 4. 合法状态裁决

| `gridOpen` | `mainView` | 语义 | 合法？ |
|-----------|-----------|------|--------|
| false | browser | 普通浏览 | ✅ |
| false | grid | Grid 视图但无资源 | ❌ **目标态禁止**（见 §5）；当前可能瞬态出现（B9-4 类 bug） |
| true | browser | Grid 资源存活但**隐藏**（浏览器可见） | ✅ |
| true | grid | Grid 为主表面且可见 | ✅ |
| true | files | Grid 资源存活但隐藏 | ✅（资源持久，仅隐藏） |

`gridOpen=true, mainView=grid` 的语义明确为：**Grid 存活，但浏览器 webview 被 `isBrowserVisible=false` 置 `visibility:hidden`（仍保留 rect 供宫格定位）**。即"Grid 存活且当前为主表面"——完全合法，与"Grid 隐藏"是两回事。

## 5. `gridVisible` 裁决（Model A vs Model B）

### Model A
```
mainView, gridOpen, gridVisible, nativeVisible   ← 4 真源
```

### Model B
```
mainView, gridOpen

desiredGridVisibility = gridOpen && mainView === "grid"
        ↓ Visibility Controller
native show/hide
```

**采用 Model B（减少真源）。**

理由：
1. `gridVisible` 永远可由 `gridOpen && mainView === "grid"` 唯一定义，无独立自由度。
2. 存为独立状态会引入第二真源：任何 `gridOpen`/`mainView` 变更都须同步维护 `gridVisible`，否则漂移（正是 Phase 0 反复踩的"重复状态"坑）。
3. `nativeVisible` 是 `schedulePosition`/`gridPosition`/`hideAllWebviews` 的执行结果，更不应成为域状态。

→ **Q3 = NO**（`gridVisible` 不需要，派生即可）。

## 6. 当前 CURRENT 公式确认

`isBrowserVisible = mainView === "browser"`（useBrowserStore.ts:149-151，与 `gridOpen` 解耦）被 `check-grid-close-logic.mjs` 固化为契约。本设计**沿用**该公式，不回退到 `!gridOpen && mainView==="browser"`（旧耦合）。理由：Grid 打开时浏览器 webview 仍需保留 rect/可见性（隐藏靠 native 移屏，不靠 `visibility`），且 `schedulePosition` 仅在 `mainView==="browser"` 时下发（useBrowserHost.ts:68），故公式必须纯粹依赖 `mainView`。

## 7. 结论

- 状态真源 = `mainView`（useLayoutStore）+ `gridOpen`（useBrowserStore）。
- 派生量：`isBrowserView()`、`isBrowserVisible`、`desiredGridVisibility`（均函数/computed，不存）。
- 不引入 `gridVisible` 存储字段。
