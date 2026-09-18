# 06 — Tests / Checker Design (Agent F)

只设计验收与**静态可检**规则。不写测试代码。

## 1. 必覆盖验收场景

| ID | 场景 | 必须证明 |
|----|------|----------|
| T1 | browser → grid | 不 recreate grid、不 reload、不丢 URL、不丢页签输入 |
| T2 | grid → browser | 同上；Grid 资源保留（gridOpen 不变），仅 mainView 变 |
| T3 | grid → home | Grid 隐藏但资源存活（gridOpen=true）；回 grid 立即可见 |
| T4 | home → grid | 若 gridOpen 则直接显示；否则先 build 再显示 |
| T5 | browser ↔ grid ×20 | 反复切换无泄漏、无空白、无 URL 丢失 |
| T6 | explicit close grid | `closeGrid()` 后 `gridOpen=false` 且 `close_grid` 已调用；Grid 窗销毁 |
| T7 | app shutdown | 所有 webview 释放（`hideAllWebviews`/Tauri quit） |
| T8 | window resize | `schedulePosition`/`scheduleGrid` 重定位，网页不跑出容器 |
| T9 | maximize / restore | 同上，且 Grid 重新铺满 |
| T10 | close single cell | `closeGridCell(i)` 仅销毁该子窗，其余重排；剩 1 格触发 `closeGrid` |
| T11 | `gridOpen=true, mainView=grid` | 浏览器 webview `visibility:hidden` 但保留 rect（供定位） |
| T12 | `closeGrid` 后曾处 grid 视图 | 自动派生 `activateBrowser`，浏览器区不空白（B9-4 不回归） |

## 2. Checker 规则设计

每条标注可检性：**STATIC_CHECKABLE** / **RUNTIME_ONLY** / **NOT_RELIABLY_CHECKABLE**。

| # | 规则 | 可检性 | 说明 |
|---|------|--------|------|
| C1 | 组件禁止直接调 `closeGridAll`/`buildGrid`/`gridCloseOne`；须经 `useBrowserStore` Intent API | STATIC_CHECKABLE | 正则/import 图扫描（参考 `check-grid-close-logic.mjs` 的源码级断言手法） |
| C2 | 组件/store 禁止直接写 `mainView`（除 `useLayoutStore.setView` 内部） | STATIC_CHECKABLE | 扫描 `\.mainView\s*=` / `layout\.mainView =` 调用点，白名单仅 `useLayoutStore.ts` 自身 |
| C3 | `mainView` 只能通过 canonical intent/owner 修改 | STATIC_CHECKABLE | 同 C2，配合 owner 入口枚举 |
| C4 | View switch 路径禁止 destroy（无 `close_grid` 调用） | STATIC_CHECKABLE | 在 `setView`/`activateView` 实现内禁止引用 `bridge.closeGrid` |
| C5 | 不新增 `gridVisible` 存储字段（防第二真源） | STATIC_CHECKABLE | 扫描 `useBrowserStore`/`useLayoutStore` 新增 `gridVisible` ref/computed |
| C6 | 不新增 Rust `show_*` / `hide_*` 之外的原生 show 命令 | STATIC_CHECKABLE | 扫描 `src-tauri/src` 新增 `tauri::command` 名含 `_show`/`show_` |
| C7 | `isBrowserVisible` 公式保持 `mainView === "browser"`（不回退 `!gridOpen && ...`） | STATIC_CHECKABLE | 复用在 `check-grid-close-logic.mjs` 的契约断言 |
| C8 | `closeGrid` 不写 `mainView`（view→browser 由 Visibility Controller 派生） | STATIC_CHECKABLE | 在 `closeGridAll` 实现内禁止 `layout.mainView =` |
| C9 | `desiredGridVisibility` 派生 = `gridOpen && mainView === "grid"` | STATIC_CHECKABLE | 若有 computed 须如此定义；可静态校验（`check-view-intent.mjs` C5 已实现） |
| C10 | `gridOpen=true, mainView=grid` 时浏览器 webview 保留 rect（非 display:none） | RUNTIME_ONLY | 需 DOM/headless 验证 `BrowserHost` 仍 mounted 且 `getBoundingClientRect` 非零 |
| C11 | 反复切换 20 次无 URL 丢失 / 无空白 | RUNTIME_ONLY | 端到端/契约仿真（参考 `check-grid-close-logic.mjs` 契约仿真段） |
| C12 | native visibility 不作为域真源回读 | NOT_RELIABLY_CHECKABLE | 无单一静态签名；靠 owner 单一执行链 + Code Review 保证 |

## 3. 落地建议

- 把 C1–C9 并入既有 `scripts/check-grid-close-logic.mjs`（同款 `.mjs` 源码级断言），以及新增 `scripts/check-view-intent.mjs`（扫描 `mainView` 直写点）。
- C10–C11 由 Vitest + 真实 store/bridge 桩（headless）覆盖（参考 `check-ui.mjs` / `check-clipboard-persistence-logic.mjs` 的加载真实 store 手法）。
- C12 由 Reviewer 人工裁决，不入自动 checker。

## 4. 不引入的 Checker

- 不新增"检测 gridVisible 是否被使用"之外的运行时探针（避免过度测试）。
- 不为 `position→show` 契约新增运行时校验（既有定位去重已覆盖）。
