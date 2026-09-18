# 08 — Architecture Decision Records

## ADR-P1A-1 — mainView 是 desired visible main surface
- **状态**：ACCEPTED
- **背景**：`mainView` 全应用唯一主表面判定（useLayoutStore.ts:138,189）。
- **决策**：`mainView` 仅表示用户意图可见的主表面；Grid 叠加维度由 `gridOpen` 表达。
- **后果**：`gridOpen=true, mainView=browser` 合法（Grid 叠加在浏览器上）。
- **Q1 = YES**。

## ADR-P1A-2 — gridOpen 只表示 resource existence
- **状态**：ACCEPTED
- **背景**：`buildGrid`/`closeGridAll` 仅翻 `gridOpen`（useBrowserStore.ts:319,486）。
- **决策**：`gridOpen` 不表达可见性；可见性需 `mainView` 参与。
- **后果**：`gridOpen=true, mainView=files` 时 Grid 隐藏但资源存活。
- **Q2 = YES**。

## ADR-P1A-3 — 不引入 gridVisible 存储状态
- **状态**：ACCEPTED
- **背景**：`gridVisible` 永远 = `gridOpen && mainView === "grid"`（Final Reconciliation 冻结公式）。
- **决策**：采用 Model B，派生不存。
- **后果**：消除第二真源漂移风险。
- **Q3 = NO**。

## ADR-P1A-4 — native visibility 是执行结果
- **状态**：ACCEPTED
- **背景**：Rust 无 `show_*`，显示=在屏 bounds 定位（04 §1）。
- **决策**：native 显隐由 Visibility Controller 单向派生，不作域真源。
- **Q4 = YES**。

## ADR-P1A-5 — canonical View Intent API
- **状态**：ACCEPTED
- **背景**：当前 `mainView` 直写点散落 10+ 处。
- **决策**：`activateView/activateBrowser/activateGrid/openGrid/closeGrid/closeGridCell` 为唯一入口；组件只调 Intent API。
- **Q5 = TARGET YES / CURRENT NO**；**Q7 = TARGET YES**。

## ADR-P1A-6 — closeGrid 独立于 View Switch
- **状态**：ACCEPTED
- **背景**：`closeGridAll` 当前同时销毁+改 mainView（耦合）。
- **决策**：`closeGrid()` 只销毁资源（gridOpen=false + close_grid）；view→browser 由 Visibility Controller 派生。
- **Q6 = YES**。

## ADR-P1A-7 — useBrowserStore 为 lifecycle owner
- **状态**：ACCEPTED
- **背景**：资源 lifecycle 与视图导航分离；`useLayoutStore` 不宜反向依赖 `useBrowserStore`。
- **决策**：owner = `useBrowserStore`（决策）+ `useBrowserHost`（引擎）+ `bridge`（执行）+ `useLayoutStore.setView`（被授权状态执行点）。
- **Q8 = useBrowserStore**。

## ADR-P1A-8 — 保留 position→show 契约
- **状态**：ACCEPTED
- **背景**：现有定位即隐式 show，B9-4 修复依赖之。
- **决策**：不拆 position/show；新增前端 Visibility Controller 派生层，不新增 Rust 命令。
- **关联**：07-ALTERNATIVES §3。

## ADR-P1A-9 — 不新造 GridLifecycle enum
- **状态**：ACCEPTED
- **背景**：状态可全由 `gridOpen`+`mainView` 表达。
- **决策**：仅作文档化状态映射，不引入 enum 真源。
- **关联**：07-ALTERNATIVES §4。

## ADR-P1A-10 — isBrowserVisible 公式保持 mainView==="browser"
- **状态**：ACCEPTED
- **背景**：`schedulePosition` 仅 `mainView==="browser"` 下发（useBrowserHost.ts:68）；Grid 打开时浏览器 webview 仍需保留 rect。
- **决策**：沿用 CURRENT 公式，不回退 `!gridOpen && mainView==="browser"`。
- **关联**：01-STATE-MODEL §6；check-grid-close-logic.mjs 契约。
