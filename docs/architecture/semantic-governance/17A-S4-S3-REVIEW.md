# 17A · S4 / S3 高风险结论复审（对抗第二轮 · Reviewer A）

> **透明性声明**：Reviewer A 的两次自动派发均因输出格式错误而未返回文档（无结论、无证据）。
> 为避免以"缺失"冒充"复核通过"，本文由 **Chief** 代笔，但**不采用第一轮的任何解释作为依据**——
> 所有 8 个复核项的证据均来自本轮 **B / C / D / E / F 五位复核者各自独立回源码建立的 SR-EVID**（他们对同一批符号做了交叉覆盖），
> 并由 Chief 逐条做了针对性再验证。结论与裁决由 Chief 承担。
> 若后续补派 Reviewer A 成功，应以 A 的独立结论覆盖本文。

**纪律**：TRUST CODE NOT DOCUMENTS；FACT ≠ INFERENCE；CAPABILITY ≠ CANONICAL INTENT API；MULTIPLE STATES ≠ MULTIPLE SOURCES OF TRUTH。
**裁决等级**：CONFIRMED / PARTIALLY_CONFIRMED / OVERSTATED / UNDERSTATED / CONTRADICTED / INSUFFICIENT_EVIDENCE。

---

## 0. 裁决总览

| # | 复核项 | 第一轮原判 | 本轮裁决 | 建议风险 | 主要反证来源 |
|---|---|---|---|---|---|
| A-01 | 离开宫格意图分叉 | S4 | **CONFIRMED** | **S4（保留）** | B / F |
| A-02 | gridOpen / mainView / gridToolbarOpen 是否重复真源 | S4 / 多源 | **OVERSTATED** | **S2（正交态 + 缺不变式 owner）** | C |
| A-03 | isBrowserView / isBrowserVisible 风险等级 | S4（08） vs S3（02） | **PARTIALLY_CONFIRMED** | **S2** | C |
| A-04 | 原生 WebView 显隐 Hidden State | S4 | **CONFIRMED** | **S4（保留）** | B |
| A-05 | 组件手工拼装 lifecycle | S3 | **UNDERSTATED** | **S3（且范围被低估约 7 倍）** | E |
| A-06 | Credential 缺 owner | S3 | **PARTIALLY_CONFIRMED** | **S2（红线已由类型层保证；直连属实）** | E / F |
| A-07 | Bookmark 双状态冲突 | S3 | **OVERSTATED** | **S2** | C |
| A-08 | position 蕴含 show | S4 | **CONFIRMED**（判级 INFERENCE → FACT） | **S3** | B / D / F |

**计数**：CONFIRMED 3 · PARTIALLY_CONFIRMED 2 · OVERSTATED 2 · UNDERSTATED 1 · CONTRADICTED 0 · INSUFFICIENT_EVIDENCE 0

---

## A-01 · 离开宫格意图是否真的存在分叉

**Original claim**（03/04，S4）：同一"离开宫格"的用户意图，经 `closeGridAll` = DESTROY，经 `ActivityBar` 导航 = HIDE。

**Independent evidence**
- file: `src/components/home/HomeLaunchers.vue` — symbol: `openArea` — L42-54（关键 **L47**）
  behavior: `if (view !== "grid" && browser.gridOpen) await browser.closeGridAll();` 之后才 `setView/openModule`。
- file: `src/stores/useBrowserStore.ts` — symbol: `closeGridAll` — L483-511
  behavior: `gridOpen=false`(486) → `bridge.closeGrid()`(487) → Rust `close_grid`（HideWindow→sleep100→CloseTab→`shutdown_all` kill+wait）→ `tabActivate`(499-501) → `mainView→browser`(502-504) → `syncFreeze` → `relocate`。
- file: `src/components/layout/ActivityBar.vue` — symbol: `onItem` — L150-177（grid 分支 167 / browser 分支 171-173）
  behavior: 非 grid 分支**只** `layout.setView(...)` / `openModule(...)`，**不调 closeGridAll**；grid 分支 `if (browser.gridOpen) layoutGrid(); else buildGrid();`。
- file: `src/stores/useLayoutStore.ts` — symbol: `setView` — L193-199
  behavior: 仅改 `mainView` / `fileEditorOpen` / `navSection`，**零 bridge 调用**。
- file: `src/stores/useBrowserStore.ts` — symbol: `watch(mainView)` → `syncViewVisibility` — L658-664 / 646-655
  behavior: 只 `hideAllWebviews` + `relocate`，**不触碰 close_grid / kill_child**。

**Independent call chain（两条）**
- HIDE 路径：`ActivityBar.onItem` → `setView` → `mainView` 变更 → `watch` → `syncViewVisibility` → `hide_all_webviews` → Rust `record_hidden` + `GridCmd::HideWindow` → `win.hide()`（进程存活，gridOpen 保持 true）。
- DESTROY 路径：`HomeLaunchers.openArea` → `closeGridAll` → `bridge.closeGrid` → `close_grid` → `shutdown_all`（kill+wait）→ `mainView→browser`。

**Finding: CONFIRMED**
**Correct interpretation**：分叉真实存在，且不是"两条不同意图"——两者对用户的可观测语义都是"我想离开宫格去看别的视图"。生命周期结局由**调用方组件**隐式选择，而非由生命周期 owner 裁决。同时源码自证：视图切换本身确实不销毁（`setView` 纯净、`syncViewVisibility` 只 hide），所以第一轮 05 的"红线成立"与 03/04 的"分叉 S4"**并不矛盾**——前者指视图切换机制，后者指意图出口。
**Recommended risk: S4（保留）**
**Impact**
- target contract：CONTRACT-GRID-EXIT 成立且必要；10 的转移矩阵必须补入 DESTROY 这条边。
- migration：Phase 1 必须显式纳入 `HomeLaunchers.openArea:47` 与 `toggleGridToolbar:214`，否则 ADR-GRID-001 在 Phase 1 结束时仍处于违反态。
- checker：RULE-004 需要，但目标 API `exitGrid` **不存在**（E 证实），先建 action 再落门禁。
- ADR：ADR-GRID-001 作为"现状断言"**不成立**，应改写为"带已知违反清单的目标政策"。

---

## A-02 · gridOpen / mainView / gridToolbarOpen — 重复真源 还是 合法正交状态？

**Original claim**（02 / 08-DUP-002，S4）：宫格可见由 3 个独立标志编码，无单一真源，属 MULTIPLE_SOURCES_OF_TRUTH。

**Independent evidence**（主要来自 Reviewer C 的独立回源）
- file: `src/stores/useBrowserStore.ts` — symbol: `buildGrid` — L300-343（关键 L319-324）
  behavior: `gridOpen.value = true`(319) 后紧跟带注释守卫 `if (mainView !== "grid" && mainView !== "browser") mainView = "grid"` —— **显式产出并保留** `gridOpen=true && mainView="browser"`。
- file: `src/stores/useBrowserStore.ts` — symbol: `syncFreeze` — L618-631
  behavior: `if (gridOpen.value)` 对每格下发 `mainView==='grid' ? UNFREEZE : FREEZE` —— 源码**已语义化**"实例存活但当前未展示"= 冻结。
- file: `src/composables/useBrowserHost.ts` — symbol: `schedulePosition` / `scheduleGrid` / `layoutGridNow` — L64-66 / L99 / L109
  behavior: 四处守卫 `gridOpen && mainView==='grid'`，不成立则退化为页签分支 —— 组合是**被处理的合法态**。
- file: `src/stores/useLayoutStore.ts` — symbol: `toggleGridToolbar` — L210-215
  behavior: UI 展开态翻转 → ON `buildGrid()` / OFF `closeGridAll()`；`buildGrid` **从不**写 `gridToolbarOpen`。

**Finding: OVERSTATED**
**Correct interpretation**：三者**不是**同一事实的多份拷贝（不互相派生、各答各的问题：实例是否存在 / 当前选中哪个视图 / 工具条是否展开）。`gridOpen=true && mainView="browser"` 是**合法表达式**（实例存活但隐藏），已被 `syncFreeze`、`schedulePosition`、`scheduleGrid` 正当处理。
真实缺陷是两点，均非"多真源"：
1. `gridVisible = gridOpen && mainView==='grid'` 这一派生**没有单一命名真源**，被 inline 复制到 ≥5 处（useBrowserHost L64/99/109、ActivityBar L358/369、useGridArchiveStore L48）；
2. `gridToolbarOpen ↔ gridOpen` 跨 store **双 writer**（useLayoutStore:211、useBrowserStore:488），耦合契约**无 invariant owner**。
**Recommended risk: S2**
**Impact**：target contract 应把"派生单点化"与"耦合契约 owner"作为目标，而非"消除重复真源"；migration 中 ADR-GRID-VISIBILITY-001 的"仅文档化"应降级为 PROPOSED；checker 宜守"派生点数量/写入点集合"，而非笼统禁多标志。

---

## A-03 · isBrowserView / isBrowserVisible 风险等级是否正确

**Original claim**：02 标 S3（SEM-003）；08 标 S4（DUP-001）；09 SMF-001 视为高危失效模式。

**Independent evidence**
- file: `src/stores/useLayoutStore.ts` — symbol: `isBrowserView` — L188-190 → `browser || grid`
- file: `src/stores/useBrowserStore.ts` — symbol: `isBrowserVisible` — L149-151 → `mainView === "browser"`
- file: `src/components/layout/UnifiedTabBar.vue` — symbol: `activateWeb` / `isActiveWeb` — L147-158
  behavior: `activateWeb` 用 `mainView !== "browser"`（注释 L149-151 明确写出"不能用 `isBrowserView()`"的踩坑理由）；`isActiveWeb`(157) **合法**使用 `isBrowserView()`。
- file: `src/components/browser/BrowserHost.vue` — L15：`isBrowserVisible` **唯一消费者**（CSS visibility）。
- file: `src/stores/useBrowserStore.ts` — L490-494：closeGridAll 注释仍写旧式 `isBrowserVisible = !gridOpen && mainView==="browser"`（**过时注释**）。

**Finding: PARTIALLY_CONFIRMED**
**Correct interpretation**：风险（命名相近易误用）**真实**，但等级被高估：二者均为零写入的 computed/纯函数，**同源同根**（mainView）且刻意不同义；`isBrowserVisible` 只有 1 个消费者，且陷阱点已有显式防呆注释。这不是"无防御的静默错误"，而是"维护负担"。
真正需要修的是 `closeGridAll` 那条**过时注释**（它正是错误直觉的源头）与命名消歧义。
**Recommended risk: S2**
**Impact**：08 的 DUP-001 由 S4 下调至 S2；checker（RULE-008）保留但定位为防回归守卫；ADR-BROWSER-001 维持 ACCEPTED（本轮唯一完全确认的两条之一）。

---

## A-04 · 原生 WebView visibility 是否真的存在 Hidden State

**Original claim**（01 SEM-012 / 02，S4）：原生 webview 显隐无前端响应式镜像，为 HIDDEN_STATE，是历史遮屏/空白根因。

**Independent evidence**（Reviewer B 独立回源）
- file: `src-tauri/src/bridge.rs` — symbol: `hide_bounds` / `apply_bounds_inner` — L566-597 / 531-564
  behavior: 隐藏 = `update_rect(-30000, y, w, h)`（保持原尺寸）；显示 = `update_rect` 后 `set_visible(true)`(L555)。注释 L566-569 明确**禁用** `set_visible(false)`（WebKitGTK 死锁）。
- file: `src-tauri/src/main.rs` — symbol: `GridCmd::UpdateRect` / `HideWindow` — L391-425 / 426-432
  behavior: 显示熔接在 `UpdateRect` 末尾 `win.show()`(L423)；`GridCmd` **无 ShowWindow 变体**。
- file: `src/stores/useBrowserStore.ts` — symbol: `syncViewVisibility` + `watch(mainView)` — L646-664
  behavior: 原生显隐**命令式**下发，前端**无**响应式变量镜像真实 bounds。
- file: `src/components/browser/BrowserHost.vue` — L15：仅 CSS `visibility` 控制**占位壳**，不控制真实 webview。

**Finding: CONFIRMED**
**Correct interpretation**：双层显隐（DOM 占位壳 CSS + 原生真实 bounds）真实存在，且前端无原生层响应式镜像，只能靠 `watch(mainView)` 命令式同步。历史上"切走残留遮屏 / 切回空白"正是两层不同步所致。
**Recommended risk: S4（保留）**
**Impact**：CONTRACT-NATIVE-SHOW 成立；ADR-NATIVE-SHOW-001 完全确认（4 个机制点逐一重验）；checker 应优先做"原生显隐出口收敛"（RULE-006/007），但注意 RULE-007 把官方 `-30000` 机制当 hack 属**误判**（见 E）。

---

## A-05 · 组件手工拼装 lifecycle 是否真实存在

**Original claim**（03 CASE-008，S3）：组件需手工组合 `setView` + `buildGrid`/`closeGridAll`；第一轮只点名 `UnifiedTabBar:170`。

**Independent evidence**（Reviewer E 全量扫描）
- 实测 **7 处 / 5 个文件**（round-1 仅承认 1 处，**低估约 7 倍**）：
  `App.vue:199`、`HomeLaunchers.vue:47`、`UnifiedTabBar.vue:170`、`ActivityBar.vue:123 / 134 / 167 / 398`。
- 同一惯用法 `if (gridOpen) layoutGrid(); else buildGrid();` 在 App.vue:199 / ActivityBar:167 / UnifiedTabBar:170 / useWorkbenchStore:22 **逐字重复 4 次**。
- `ActivityBar:398` 为模板内联 `@click="layout.navSection=''; browser.closeGridAll()"`。

**Finding: UNDERSTATED**
**Correct interpretation**：问题真实且比第一轮描述更广。但需区分两类：
- **纯触发**（`ActivityBar:398`、`HomeLaunchers:47`）—— 经 store action，owner 仍在 `useBrowserStore`，语义上**可接受**；
- **真编排**（`ActivityBar:123/134` 先写 `gridCount/gridLayout` 再 `buildGrid`）—— 组件决定了生命周期结局，这才是治理缺口。
此外 `exitGrid` 目标 API **不存在**（E 证实），故"先落门禁"会迫使小模型把 7 处加白名单，**反而固化债务**。
**Recommended risk: S3（保留，但迁移顺序必须先建 action）**
**Impact**：migration Phase 1 需先加 `exitGrid/ensureGrid` store action 再迁移 7 处；checker RULE-004 属 rejected-for-Phase-0（先有合规替代写法）。

---

## A-06 · Credential 是否缺少 owner

**Original claim**（04，S3 / STORE_OWNERSHIP_LEAK + NATIVE_POLICY_LEAK）：无 store，组件直连 bridge。

**Independent evidence**（Reviewer E / F）
- file: `src/components/browser/CredentialList.vue` — L44 `bridge.listBrowserCredentials()`、L79-82 `bridge.fillBrowserCredential(credential_id, activeTabId)`；L20-23 `items/loading/error/fillingId` 为**组件 ref，非 Pinia**；L10-15 红线注释。
- file: `src/types.ts` — L174-186：`BrowserCredentialItem` 只含 `url / username / has_password / credential_id / origin`，**无 password 字段** —— 红线已由**类型层**保证。
- file: `src/components/browser/BookmarkPanel.vue` — L109：`importBrowserCredentials` 直连（round-1 只点了 CredentialList，**漏了这里**）。
- 全仓**无** `useCredentialStore`。

**Finding: PARTIALLY_CONFIRMED**
**Correct interpretation**："缺 store owner + 组件直连"**属实**（3 处直连）。但"前端持有明文密码"这一风险**已由类型和后端边界保证**，用名字正则去复刻它属**伪规则**（E 实测拟议正则 B 唯一命中是 100% 误报：`<input ref="passwordInput" ...>`）。
另一修正：把所有凭据命令塞进 `useCredentialStore` 可能是**错误 owner 划分**——`importBrowserCredentials` 的调用方是**书签面板的 CSV 导入流**，其自然 owner 更像 bookmark/import store。
**Recommended risk: S2**
**Impact**：ADR-CREDENTIAL-001 维持 PROPOSED（分类正确）；RULE-011-B 应**否决**，改为类型断言（`BrowserCredentialItem` 无 password 字段）；RULE-011-A 需先澄清 owner 再建 facade。

---

## A-07 · Bookmark 双状态是否真实冲突

**Original claim**（02 SEM-006 / 03，S3）：ActivityBar 用 `panelOpen` 高亮、MainArea 用 `bmPanelOpen`（=`panelOpen && mainView==='browser'`）挂载，非 browser 视图"高亮但不渲染"。

**Independent evidence**（Reviewer C）
- file: `src/components/layout/MainArea.vue` — L102-103 定义、L150 唯一消费者 `<BookmarkPanel v-if="bmPanelOpen">` → **派生唯一，无第二处重算**。
- file: `src/components/layout/ActivityBar.vue` — L328 `:class="{ active: bookmarks.panelOpen }"`（未用派生）；L137-148 `onToggleBookmarkPanel` **已补偿**：非 browser 视图点击时先 `setView("browser")` 再确保 `panelOpen=true`。
- `BookmarkStar.vue` L46-50 同样用 `panelOpen`，但仅在 browser 视图渲染 → 无不一致。

**Finding: OVERSTATED**
**Correct interpretation**：不是"两个判据打架"，而是**消费点未复用派生量**。且主路径已被入口层补偿：从任意视图点击收藏夹 ⇒ 先切 browser 再打开 ⇒ 高亮与挂载一致。仅在"**先开面板、再切到 files/term**"这一窄边界下才出现"按钮亮着但无面板"，且回到 browser 视图立即复现（面板仍是用户意图态）。
**Recommended risk: S2**
**Impact**：修法最小——ActivityBar L328 改为 `panelOpen && mainView==='browser'`，或把派生提升进 `useBookmarkStore` 共用；不应作为 S3 进入 Phase 1 首批。

---

## A-08 · position 蕴含 show（补充 S4 复核项）

**Original claim**（07，S4；05 标 INFERENCE）：`position` 命令携带未声明的 show 副作用。

**Independent evidence**
- `src-tauri/src/bridge.rs` `apply_bounds_inner` **L555** `set_visible(true)`（B / D / F 三方独立确认）。
- `src-tauri/src/main.rs` `GridCmd::UpdateRect` **L423** `win.show()`；`GridCmd` 无 `ShowWindow` 变体（B / D / F）。
- `grid_position` → `record_rect` 会清 `hidden`/`blur_hidden` → POSITION ⇒ 解除隐藏 ⇒ SHOW（B，SR-EVID-0017）。

**Finding: CONFIRMED**（判级由 INFERENCE **升为 FACT**）
**Correct interpretation**：这是**机制性耦合**（隐藏靠屏外移，显示只能经重定位），不是 bug。但按 ADR-SEVERITY-001（S4 专指生产静默错误），它不产生静默错误，**应归 S3 并标注"架构耦合，需补正交 show/hide 原语"**。
另注意：RULE-007 把官方 `-30000` 隐藏机制当 "hack" 属**误判**（PROJECT-RULES 明文规定 `-30000` 为官方隐藏值，`bridge.hideWebview` 内部执行的正是它）；`:170` 那处是为绕过去重做黑闪修复，改为 `hideWebview` 有回归风险。

---

## 1. 第一轮错误清单（本项范围内）

1. **A-02 误判**：把三个**正交状态**标为 MULTIPLE_SOURCES_OF_TRUTH / S4。真相：合法正交 + 派生未单点化 + 耦合无 owner。
2. **A-03 偏高**：`isBrowserView/isBrowserVisible` 标 S4；实为同源双谓词、单消费者、已有防呆注释 → S2。
3. **A-05 低估**：组件手拼 lifecycle 只点名 1 处，实测 7 处；且未识别"纯触发 vs 真编排"的区分。
4. **A-06 误判**：把已由类型层保证的"不持密码"红线再做成名字正则（实测 100% 误报），并低估了 owner 划分问题（BookmarkPanel:109 漏记）。
5. **A-07 偏高**：`bmPanelOpen` 标 DERIVED_STATE_REIMPLEMENTED / S3；实为单派生、单消费者，边界窄 → S2。
6. **A-08 判级偏低**：position⇒show 被标 INFERENCE，实为可逐字读出的 FACT（但风险等级应 S4→S3）。

## 2. 对 RECOMMENDED_FIRST_BATCH 的复核结论

- **保留 S4 的仅 2 项**：A-01 离开宫格意图分叉、A-04 原生显隐 Hidden State。二者满足"语义歧义 + 小模型易错 + 可编译 + 运行时破坏 + 现有检查器难阻止"全部条件，应进入 Phase 1 首批。
- **降级的**：A-02（S4→S2）、A-03（S4→S2）、A-07（S3→S2）—— 均属"正交/派生"误判，不应以"消除重复真源"为目标做重构。
- **升级的**：A-05（范围 ×7）—— 但治理顺序必须先建 store action 再落门禁。
- **需改判为"伪规则"的**：A-06 的密码正则、A-08 相关的 RULE-007。
