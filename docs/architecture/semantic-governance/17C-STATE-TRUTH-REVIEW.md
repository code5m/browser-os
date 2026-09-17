# 17C · State Truth / Orthogonality Review（对抗第二轮）

> Reviewer: **Agent C**（READ-ONLY）
> 对象：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> 输入：`docs/architecture/semantic-governance/01-SEMANTIC-INVENTORY.md`、`02-STATE-SOURCES.md`（轮次 1）
> 使命：对状态组重新判定 DUPLICATE_SOURCE_OF_TRUTH vs ORTHOGONAL_STATE，纠正轮次 1 的误判。
> 方法论：**多个状态 ≠ 多个真源；派生状态 ≠ 第二真源**。只有当两个变量回答**同一个问题**、且**互不派生**、且**都被至少一个消费者当作答案**时，才是 DUPLICATE_SOURCE_OF_TRUTH。
> 全程只读，未修改 `src/`、`src-tauri/`、`scripts/` 及 00-16 号文档。

---

## 0. 结论速览

| # | 状态组 | 本轮判定 | 轮次1 判定 | 轮次1 裁决 | 是否"重复真源" |
|---|---|---|---|---|---|
| 1 | mainView | CANONICAL_STATE（多 writer） | SINGLE_SOURCE / S1 | CONFIRMED（略 UNDERSTATED） | 否 |
| 2 | gridOpen | **ORTHOGONAL_STATE** | SINGLE_SOURCE(MIXED)/S2 | PARTIALLY_CONFIRMED | **否** |
| 3 | gridToolbarOpen | ORTHOGONAL_STATE（MULTIPLE_WRITERS） | MULTIPLE_WRITERS / S2 | PARTIALLY_CONFIRMED（"双真源"OVERSTATED） | 否 |
| 4 | isBrowserView() | DERIVED_STATE（browser‖grid） | DERIVED_STATE / S3 | CONFIRMED | 否 |
| 5 | isBrowserVisible | DERIVED_STATE（严格 browser） | DERIVED_STATE(注释过时)/S3 | CONFIRMED | 否 |
| 6 | activeTabId（前端） | CANONICAL_STATE | SINGLE_SOURCE / S2 | CONFIRMED | 否 |
| 7 | Rust `AppState.active_tab` | **STALE_MIRROR** | SINGLE_SOURCE（镜像）/S2 | PARTIALLY_CONFIRMED（标签自相矛盾） | 否（是镜像） |
| 8 | panelOpen | CANONICAL_STATE | SINGLE_SOURCE / S1 | CONFIRMED | 否 |
| 9 | bmPanelOpen | DERIVED_STATE（消费点不一致≠重复真源） | DERIVED_STATE_REIMPLEMENTED / S3 | **OVERSTATED**（S3→S2） | 否 |
| 10 | activeTermId（文档称 activeTerminal） | CANONICAL_STATE | SINGLE_SOURCE / S1 | CONFIRMED（符号不存在） | 否 |
| A | currentLocalPath | DERIVED_STATE（优先级聚合） | DERIVED_STATE_REIMPLEMENTED / S2 | **OVERSTATED**（S2→S1） | 否 |
| B | layout.aiNavOpen | DUPLICATE_SOURCE_OF_TRUTH（**潜势 / 今日零读者**） | MULTIPLE_SOURCES_OF_TRUTH / S2 | PARTIALLY_CONFIRMED（S2→S1） | 否（现在还不是） |
| C | hibernated_tabs | HIDDEN_STATE（默认关闭） | HIDDEN_STATE / S2 | PARTIALLY_CONFIRMED（S2→S1~S2） | 否 |

**判定计数**：CONFIRMED 5 · PARTIALLY_CONFIRMED 5 · OVERSTATED 2 · UNDERSTATED 1 · CONTRADICTED 0 · INSUFFICIENT_EVIDENCE 0

**结论先行**：轮次 1 在**目录学**（行号/读写者）上高度准确，但在**分类学**上有系统性偏差：它把「同一 canonical 源上的多个正交谓词/派生值」与「缺少单一不变式 owner 的耦合态」一并计入"多真源"。本轮认为：**本仓在当前代码状态下不存在语义意义上的 DUPLICATE_SOURCE_OF_TRUTH**（唯一够格者 `layout.aiNavOpen` 零读者，属潜势）。真正问题是 **耦合契约没有 invariant owner + 1 处镜像无重同步协议**。

---

## 1. 核心裁决：`gridOpen === true` AND `mainView === "browser"` 是否非法？

### 裁决：**合法表达式 = "宫格实例存活但当前视图未展示宫格"**

**SR-EVID-0201** Reviewer: C Classification: FACT
File: `src/stores/useBrowserStore.ts` Symbol: `buildGrid` Line: 300-343（L319-324）
Observed: `gridOpen.value = true;`(319) 后紧跟带注释守卫：
`// 不要在这里覆盖 mainView…避免 "grid"->"browser" 二次覆盖打乱 watch 时序`
`if (layout.mainView !== "grid" && layout.mainView !== "browser") { layout.mainView = "grid"; }`
Caller: toggleGridToolbar(L213)/ActivityBar onItem('grid')(167)/UnifiedTabBar(170)/useWorkbenchStore(22)/App(199)
Why: 这是**显式、带注释意图**的分支，直接产出 `gridOpen=true && mainView="browser"`。它不是意外中间态，而是被设计保留的组合。 Confidence: HIGH

**SR-EVID-0202** Reviewer: C Classification: FACT
File: `src/stores/useLayoutStore.ts` Symbol: `toggleGridToolbar` Line: 210-215
Observed: 翻转 `gridToolbarOpen` → 开且 `!gridOpen` 则 `buildGrid()`（**不切视图**）→ 关且 `gridOpen` 则 `closeGridAll()`
Why: 在 browser 视图点开工具条 = 官方路径进入该组合，UI 期望**留在 browser 视图**。 Confidence: HIGH

**SR-EVID-0203** Reviewer: C Classification: FACT
File: `src/stores/useBrowserStore.ts` Symbol: `syncFreeze` Line: 618-631
Observed: `const inBrowserView = …`(L620，**未被使用**，死变量)；页签 `visible = mainView==='browser' && id===activeTabId`(622)；`if (gridOpen.value)` 每格下发 `mainView==='grid' ? UNFREEZE : FREEZE`(628)
Why: 源码**已语义化**该组合 = 格子全部冻结（alive-but-hidden ⇒ frozen），而非未定义行为。 Confidence: HIGH

**SR-EVID-0204** Reviewer: C Classification: FACT
File: `src/composables/useBrowserHost.ts` Symbol: `schedulePosition`/`scheduleGrid`/`layoutGridNow` Line: 59-69 / 97-99 / 108-109
Observed: 四处守卫 `browser.gridOpen && layout.mainView === 'grid'`，不成立则退化为页签分支
Why: 组合是**被处理的合法态**；真实缺陷是守卫逻辑复制到 4 个点而**没有命名派生量**（`gridVisible`）。 Confidence: HIGH

**SR-EVID-0205** Reviewer: C Classification: FACT
File: `src/components/layout/MainArea.vue` Line: 139, 151
Observed: `<div class="viewport" :class="{ 'grid-mode': browser.gridOpen }">`，外层受 `v-else-if="mainView==='browser' || 'grid'"`(139) 限制
Why: `grid-mode` 在 browser 视图下随 gridOpen 生效，属**命名/用法瑕疵**，非"两个答案打架"。 Confidence: HIGH

**SR-EVID-0206** Reviewer: C Classification: FACT/INFERENCE
File: `src/components/layout/ActivityBar.vue` Symbol: `onItem` Line: 155-169；`closeGridAll` 调用点全集（ActivityBar L398、HomeLaunchers L47、useLayoutStore L214、useBrowserStore L522）
Observed: 注释称"离开宫格视图时自动关闭宫格：gridOpen 悬挂为 true 会让浏览视图定位走错分支…"，但 `onItem` 非 grid 分支**从不调用 closeGridAll**
Why: **STALE 注释**；"离开 grid 即关宫格"的不变式**没有 owner** ⇒ gridOpen 会长期悬挂。真实缺陷是缺单一不变式出口，不是"gridOpen 与 mainView 重复"。 Confidence: MEDIUM-HIGH

### 合法/非法组合表

| gridOpen | mainView | 法律地位 | 谁产出 | 谁消费/处理 | 一致性维护者 |
|---|---|---|---|---|---|
| false | browser | 合法（默认稳态） | closeGridAll L502-504 | schedulePosition L68 | 无需维护 |
| true | grid | 合法（宫格可见） | buildGrid L322-324 / onItem('grid') | scheduleGrid L99 / syncFreeze L628 | buildGrid+openModule |
| **true** | **browser** | **合法（alive-but-hidden）** | **buildGrid L322-324 + toggleGridToolbar L213** | **schedulePosition L64/68、syncFreeze L628、MainArea L151** | **无单一 owner，4 处 guard 各自防守** |
| true | 其它（files/term…） | 合法但注释视为不宜 | 离开 grid 未关宫格 | schedulePosition L68 return；syncFreeze 冻结 | 无（缺不变式出口） |
| false | grid | **视图层可达成** | `openModule("grid")` 先于 buildGrid | ActivityBar L167/170 检测后 `buildGrid()` 补偿 | ActivityBar/App/UnifiedTabBar **各自**三元补救 |

> 最后一行（`false+grid`）才是真正需要 invariant 的组合，且三处调用点**各自写了同一句三元补救**（`gridOpen ? layoutGrid() : buildGrid()`）——属**多入口 DERIVED 修补**，不是重复真源。

---

## 2. 逐项再派生

### 2.1 mainView —— CANONICAL_STATE
- 回答："当前主区渲染哪个一级视图"。唯一定义 `useLayoutStore.ts:138`；写入：`setView`(193)、`useBrowserStore` L161/L207/L545、`useSessionStore` L120、`openModule/activateModTab/closeModTab`（内部走 setView）。
- **SR-EVID-0207** Classification: FACT。File: useLayoutStore L138/193/253-265/282-298/300-314；useBrowserStore L161,207,545；useSessionStore L119-120。
- Observed: `layout.mainView = "browser"` 出现在三个不同 store；useSessionStore 同时写 `browser.activeTabId` 与 `layout.mainView`。Side effects: 每次变更触发 `watch(mainView)`→`syncViewVisibility` 与 `invalidateAll()`。
- Why: 支撑"单真源"；同时说明**缺 ONE INTENT ENTRYPOINT**（§1 表最后一行正是多 writer 的直接后果）。Confidence: HIGH
- 裁决：CONFIRMED，但 **UNDERSTATED**：S1→S2（多 writer 直写使得"grid 视图 ⇒ gridOpen"无法在唯一出口强加）。

### 2.2 gridOpen —— ORTHOGONAL_STATE（生命周期）
- 回答："宫格子 webview 实例是否已建立且未拆除"。与 mainView **不同问题**；不可互相派生（buildGrid 明确保留 browser；closeGridAll 只在 mainView==='grid' 时才切回）。
- **SR-EVID-0208** Classification: INFERENCE。File: `useGridArchiveStore.ts` L48（`!browser.gridOpen` → throw GRID_NOT_OPEN）、MainArea L151、ActivityBar L358/369/393-394、useBrowserHost L64/99/109。
- Observed: `useGridArchiveStore` 用 gridOpen 表达"宫格窗口还开着"——在 alive-but-hidden 下语义上"没显示"但该 API 仍放行（弱 bug，读取仍可用）。
- Why: 轮次 1 "会被误判"成立但**后果轻微**；更重要的是证明缺一个共享派生量。Confidence: MEDIUM
- 建议：新增单一 `gridVisible = computed(() => gridOpen && mainView === 'grid')`，替换 inline 守卫；**不建议改名** gridOpen（成本 > 收益）。

### 2.3 gridToolbarOpen —— ORTHOGONAL_STATE（UI 展开态），耦合契约缺 owner
- 回答："活动条宫格设置扩展行是否展开"。与 gridOpen **不同事实**（buildGrid 从不写它）。不可互相派生。
- 谁维护：`toggleGridToolbar`(useLayoutStore 210-215) + `closeGridAll`(useBrowserStore 488)——**跨 store 双 writer，无第三处定义不变式**。
- **SR-EVID-0209** Classification: FACT。File: useLayoutStore L144/210-215；useBrowserStore L486-489。
- Observed: `closeGridAll` 内 `gridOpen=false` 先翻（早于 await IPC），再 `layout.gridToolbarOpen = false`。
- Why: 轮次 1 写"UI 展开态与实例态**双真源**"措辞错误（两者不答同一问题），但技术建议（加不变式）正确。Confidence: HIGH
- 裁决：PARTIALLY_CONFIRMED（标签 OVERSTATED；S2 维持，归因改为"耦合契约无 owner"）。
- 未处理组合：`gridToolbarOpen=true && mainView≠browser/grid`（`setView` 只重置 navSection，不重置它）——纯 UI 遗留，无害。

### 2.4 / 2.5 isBrowserView() vs isBrowserVisible —— 两个正交派生谓词（CONFIRMED）
- `isBrowserView()`（useLayoutStore L188-190）= `browser ‖ grid` → "该不该显示浏览器类 UI"
- `isBrowserVisible`（useBrowserStore L149-151）= `mainView === "browser"` → "BrowserHost 是否 visible"
- **SR-EVID-0210** Classification: FACT。File: useLayoutStore L188-190；useBrowserStore L149-151；UnifiedTabBar L147-152（注释「不能用 isBrowserView()」→ 改用 `mainView !== "browser"`）；UnifiedTabBar L157（`isActiveWeb` **合法**用 isBrowserView）；BrowserHost L15（唯一消费者）；MainArea L155。
- Why: **CONFIRMED** 轮次 1 CASE-001。真正成本是命名相近的维护负担（S3 偏高；单消费者 + 防呆注释 ⇒ 实际 S2）。Confidence: HIGH
- 顺带确认 `useBrowserStore` L490-494 注释 `isBrowserVisible = !gridOpen && mainView==="browser"` **确为过时**（真实 computed 不含 !gridOpen）——EVID-0004 CONFIRMED；该注释是错误直觉源头，建议优先修。

### 2.6 activeTabId —— CANONICAL_STATE
- 定义 `useBrowserStore.ts:19`；写入 tabNew L164 / tabSwitch L176 / closeTabNow L201 / restoreSession L119（跨 store）。
- **SR-EVID-0211** Classification: FACT。全仓 `activeTabId\s*=` 命中：定义 L19、useSessionStore L119；读取 UnifiedTabBar L157。外部**无**组件直写。Confidence: HIGH
- 裁决：CONFIRMED（S2 保留，理由在镜像侧 §2.7）。

### 2.7 Rust `AppState.active_tab` —— STALE_MIRROR
- 回答（对原生层）："哪个页签是原生侧当前焦点"。被 `close_browser`(466)、scanner(1974)、`tab_title` 回传(1935)、关闭重选(818-821)读取 ⇒ **是**镜像而非正交态。
- 后端**无法**从自身派生，只能由前端写穿。
- **SR-EVID-0212** Classification: FACT+INFERENCE。File: bridge.rs L269（定义）/757（create 写）/818-821（close 后取 `tabs.keys().next()`—HashMap 任意序）/890（退出清空）/4663-4685；useBrowserStore L179（**无 catch**）/L500（**静默** `.catch`）/L204。
- Observed: IPC 失败时 tabSwitch 抛错、closeGridAll 静默；**无重试或对账**，只有用户再次手动切换才收敛。close 时后端自行挑 successor（HashMap 顺序），若前端未调 tabActivate，两端顺序不一定一致。
- Why: 轮次 1 标 "SINGLE_SOURCE（镜像）" 自相矛盾且忽略两点：(a) 它是原生侧运行时**唯一判据**，非纯拷贝；(b) 无 reconciliation ⇒ 具备 stale 通道。Confidence: MEDIUM-HIGH
- 裁决：PARTIALLY_CONFIRMED → 改判 **STALE_MIRROR**。

### 2.8 / 2.9 panelOpen vs bmPanelOpen —— 单真源 + 单派生
- `panelOpen`（useBookmarkStore L54）：用户是否打开过侧栏（唯一 writer `togglePanel` L192-194）⇒ **CANONICAL_STATE**
- `bmPanelOpen`（MainArea L103）：`panelOpen && mainView==="browser"` ⇒ **DERIVED_STATE**（定义唯一，单消费者 L150）
- **SR-EVID-0213** Classification: FACT。File: MainArea L102-103/L150；ActivityBar L137-148（非 browser 视图点击先 setView("browser") 再确保 panelOpen=true）、L328；BookmarkStar L46-50（同用 panelOpen，但仅在 browser 视图渲染 ⇒ 无不一致）；useBookmarkStore L54/192-194。
- Observed: 从任意视图点击收藏夹 ⇒ 契约满足 ⇒ 高亮与挂载一致；仅"**先开面板、后切到 files/term**"才出现"按钮亮着但无面板"，回 browser 视图立即复现。
- Why: 轮次 1 "DERIVED_STATE_REIMPLEMENTED"（暗示第二份等价计算）**不成立**。Confidence: HIGH
- 裁决：**OVERSTATED**（撤回 REIMPLEMENTED；S3→**S2**，表述应为"消费点未复用派生量"）。
- 最小修法：ActivityBar L328 改 `panelOpen && mainView==='browser'`，或把派生提升进 useBookmarkStore 共用（后者更符合"一处定义"）。

### 2.10 activeTermId —— CANONICAL_STATE
- 定义 useSystemStore L128；写入 L241（spawn 首个即激活）/L264（kill 后回落 termPanes[0]）/L321-323（setActiveTerm）。
- **SR-EVID-0214** Classification: FACT。全仓 `activeTerminal|activeTermId` 命中仅 useSystemStore L128/241/264/285/321-323/383。**不存在名为 `activeTerminal` 的符号**（TerminalPane 以 prop 传 paneId）。
- Why: CONFIRMED（S1）。附加：01 文档标题写 `activeTermId / activeTerminal` 属**命名陈旧**（文档级 STALE）。Confidence: HIGH

---

## 3. 附加评估项

### 3.1 currentLocalPath —— DERIVED_STATE（优先级聚合），**不是** REIMPLEMENTED
- 四级来源各答**不同问题**：`inlineFile`（行内打开文件）、`previewDir`（预览目录）、`modTabs[activeModTab].path`（目录页签路径）、`filePath`（浏览根目录）⇒ **四个不同事实按优先级择优**，非同一事实的四份拷贝。
- **SR-EVID-0215** Classification: FACT。File: useWorkspaceStore L578-586（定义）、L631-634（locateCurrent 读）、L692-696（watch→locateTo）；UnifiedTabBar L109-110/L143（`relativeOf(abs)` 用传入的 p，**不参与** fallback）。
- Observed: 派生链单一、消费者两个。**无第二处等价计算**。
- Why: 轮次 1 "DERIVED_STATE_REIMPLEMENTED" **被证伪**。真正（轻量）问题是**优先级顺序是隐式约定**。Confidence: HIGH
- 裁决：**OVERSTATED**（改判 DERIVED_STATE；S2→**S1**）。

### 3.2 aiNavOpen —— layout.aiNavOpen **零读者**：DUPLICATE_SOURCE_OF_TRUTH（仅潜势）
- 谁 read `browser.aiNavOpen`：ActivityBar L338/L341、AINavPanel L10/L18、useWorkbenchStore L27/29/32（快照保存与恢复）。
- **SR-EVID-0216** Classification: FACT。File: 全仓 `aiNavOpen`：useLayoutStore（L146 声明/L338 返回）、useBrowserStore（L43/538/680）、ActivityBar、AINavPanel、useWorkbenchStore。
- Observed: `layout.aiNavOpen` **无任何读写点**（除声明与返回）。
- Why: 支撑"存在重复声明"（FACT），**挑战** "MULTIPLE_SOURCES_OF_TRUTH"——零读者变量不承载任何"答案"，今日不构成第二真源；是**潜势重复真源/死声明**（风险通道正是 useWorkbenchStore 这类快照代码）。Confidence: HIGH
- 裁决：PARTIALLY_CONFIRMED。建议：维持删除动议，分类改 **DEAD_DUPLICATE_DECLARATION（潜势）**，S2→S1，优先加静态检查禁止新增引用。

### 3.3 hibernated_tabs —— HIDDEN_STATE（默认关闭 → 暴露面小于 S2）
- **SR-EVID-0217** Classification: FACT。File: bridge.rs L289/169-175/4558-4573/4585-4593/4663-4683/4507-4512；types.ts L238-242（`TabInfo` **无 hibernated 字段**）/L388-392（`ResourceStats.hibernated_count`）；StatusBar L105-107/L128（展示"休眠 N 个"）；useSettingsStore L65-67（`tabHibernation` 默认 **false**）/L121；SettingsPanel L82-86。
- Observed: **默认关闭** ⇒ 未开启用户永不见该状态；开启后前端 tabs/activeTabId 仍不区分，只能靠 IPC 报错被动得知；但已有**聚合级**可见性（计数）与明确错误信号。
- Why: 支持 HIDDEN_STATE 判定，同时**下调**严重度——不是"随时都在错"，而是"opt-in 后错"。Confidence: HIGH
- 裁决：PARTIALLY_CONFIRMED（分类保留；S2→**S1~S2**）。最小修复：`TabInfo` 增 `hibernated?: boolean`，由事件驱动。

---

## 4. 方法论裁决：为什么这些不是 DUPLICATE_SOURCE_OF_TRUTH

| 判据 | mainView vs gridOpen | gridOpen vs gridToolbarOpen | isBrowserView vs isBrowserVisible | panelOpen vs bmPanelOpen | FE activeTabId vs Rust active_tab | hibernated vs FE tabs |
|---|---|---|---|---|---|---|
| 回答同一问题？ | 否（选中 vs 存在） | 否（存在 vs UI 展开） | 否（类视图 vs 严格可见） | 否（意图 vs 挂载） | **是** | **是** |
| 可互相派生？ | 否（双向） | 否 | 同源但不等价 | 正向可，反向不可 | 后端不可派生 | 后端可派生，前端无字段 |
| 都被消费为答案？ | 是（各答各的） | 是（各答各的） | 是（各答各的） | 是（各答各的） | 是 ⇒ **镜像** | 前端无字段 ⇒ **隐藏态** |
| 结论 | ORTHOGONAL | ORTHOGONAL（耦合） | ORTHOGONAL（同源双谓词） | CANONICAL+DERIVED | **STALE_MIRROR** | HIDDEN_STATE |

> 唯一进入 DUPLICATE_SOURCE_OF_TRUTH 候选的 `layout.aiNavOpen`，因**零读者**落在"潜势/死声明"。

---

## 5. 轮次 1 误判清单

1. **gridOpen ↔ mainView**：暗示该组合"误判可见"→ 方向对，但把正交生命周期态当重复/混合真源。真相：组合由 `buildGrid` L322-324 **显式产出**并被 4 处正当处理。真实缺陷是**派生未单点化** + **离开 grid 无统一出口**（ActivityBar L156-157 注释描述的行为不存在 ⇒ STALE）。改判 **ORTHOGONAL_STATE**。→ PARTIALLY_CONFIRMED
2. **gridToolbarOpen "双真源"** → **OVERSTATED**。改判 ORTHOGONAL_STATE（MULTIPLE_WRITERS）。→ PARTIALLY_CONFIRMED
3. **currentLocalPath "DERIVED_STATE_REIMPLEMENTED"** → **标签 CONTRADICTED**：全仓仅一份定义，`relativeOf` 用显式入参。改判 DERIVED_STATE，S2→S1。→ OVERSTATED
4. **bmPanelOpen "REIMPLEMENTED / S3"** → **OVERSTATED**：派生唯一、消费者唯一；缺陷在消费点且入口已补偿。S3→S2。→ PARTIALLY_CONFIRMED
5. **aiNavOpen "MULTIPLE_SOURCES_OF_TRUTH / S2"** → PARTIALLY_CONFIRMED：重复声明为 FACT，但零读者。改判潜势，S2→S1。
6. **后端 active_tab "SINGLE_SOURCE（镜像）"** → **标签自相矛盾**：既是写穿镜像又是原生运行时判据，且无 reconciliation。改判 **STALE_MIRROR**。→ PARTIALLY_CONFIRMED
7. **hibernated_tabs S2** → PARTIALLY_CONFIRMED 并下调：默认关 + 已有聚合计数 + 明确错误信号。S2→S1~S2。
8. **isBrowserView/isBrowserVisible**（唯一**未被误判**的争议项）：轮次 1 判定正确 → **CONFIRMED**；建议 S3→S2，并修 `closeGridAll` L490-494 过时注释。
9. **mainView**：CONFIRMED，但 **UNDERSTATED**——多 writer 跨 store 直写使"grid 视图 ⇒ gridOpen"无法在唯一出口强加，这是 §1 表中唯一真正不可达组合的根因。S1→S2。

---

## 6. 最终分类表

| 状态 | 回答的事实 | 可互派生 | 维护者 | 最终分类 | 建议风险 |
|---|---|---|---|---|---|
| `layout.mainView` | 当前主区渲染哪个一级视图 | 根不可派生 | setView + 6 处跨 store 直写 | **CANONICAL_STATE** | S1→S2 |
| `browser.gridOpen` | 宫格实例是否存活 | 否（双向） | buildGrid / closeGridAll | **ORTHOGONAL_STATE** | S2 |
| `layout.gridToolbarOpen` | 工具条是否展开 | 否 | toggleGridToolbar + closeGridAll（双 writer） | **ORTHOGONAL_STATE**（耦合无 owner） | S2 |
| `layout.isBrowserView()` | 是否浏览器**类**视图（含 grid） | 同源不等价 | computed/pure fn | **DERIVED_STATE** | S2 |
| `browser.isBrowserVisible` | 是否严格 browser（驱动 CSS） | 同源不等价 | computed | **DERIVED_STATE**（注释过时） | S2 |
| `browser.activeTabId` | UI 当前激活页签 | 后端不可派生 | tabNew/tabSwitch/closeTabNow/restoreSession | **CANONICAL_STATE** | S2 |
| Rust `AppState.active_tab` | 原生侧当前焦点页签 | 只能写穿 | tab_activate / create / close 兜底 | **STALE_MIRROR** | S2 |
| `bookmarks.panelOpen` | 用户是否打开过收藏夹 | 否（是根） | togglePanel | **CANONICAL_STATE** | S1 |
| `MainArea.bmPanelOpen` | 此刻是否挂载收藏夹 DOM | 完全派生 | computed | **DERIVED_STATE**（消费点未复用） | S2 |
| `system.activeTermId` | 当前聚焦终端实例 | 否 | spawnTerm/killTerm/setActiveTerm | **CANONICAL_STATE** | S1 |
| `ws.currentLocalPath` | 文件树应高亮的本地路径 | 由 4 个不同事实优先级聚合 | computed | **DERIVED_STATE** | S1 |
| `layout.aiNavOpen` | （无人问） | — | 无 writer / 无 reader | **DUPLICATE_SOURCE_OF_TRUTH（潜势/死声明）** | S1 |
| Rust `hibernated_tabs` | 哪些页签 webview 已销毁但 URL 保留 | 前端未镜像 | hibernate_tab/sweeper/tab_activate | **HIDDEN_STATE** | S1~S2 |

---

## 7. 最小修正建议（非现状断言）
1. 新增 `gridVisible = gridOpen && mainView === "grid"` 单一派生，替换 useBrowserHost L64/L99/L109、ActivityBar L358/L369、useGridArchiveStore L48 的近似判定。
2. 为"离开 grid 视图"建立单一出口（或在 setView 内统一施加不变式），消除 gridOpen 长期悬挂；同时修 ActivityBar L156-157 与 useBrowserStore L490-494 两处 STALE 注释。
3. `tabActivate` 失败需对账：把 closeGridAll L500 静默 catch 与 tabSwitch L179 无兜底统一为"记录 divergence + 下次视图切换重放"。
4. `bmPanelOpen` 提升进 useBookmarkStore，ActivityBar L328 与 MainArea L150 共用。
5. 删除 `layout.aiNavOpen`，加静态检查禁止复现。
6. `TabInfo` 增 `hibernated?: boolean`，由休眠/重建事件驱动。
7. 清理 `syncFreeze` L620 未使用变量；01 文档 `activeTerminal` 改为 `activeTermId`。

---

## 8. 证据索引（本轮）

| ID | File | Symbol / Line | 分类 | 置信 |
|---|---|---|---|---|
| SR-EVID-0201 | useBrowserStore.ts | buildGrid L300-343（319-324） | FACT | HIGH |
| SR-EVID-0202 | useLayoutStore.ts | toggleGridToolbar L210-215 | FACT | HIGH |
| SR-EVID-0203 | useBrowserStore.ts | syncFreeze L618-631（620 死变量/622/628） | FACT | HIGH |
| SR-EVID-0204 | useBrowserHost.ts | schedulePosition L59-69 / scheduleGrid L97-99 / layoutGridNow L108-109 | FACT | HIGH |
| SR-EVID-0205 | MainArea.vue | L139, L151 | FACT | HIGH |
| SR-EVID-0206 | ActivityBar.vue | onItem L155-169 + closeGridAll 调用点全集 | FACT/INFERENCE | MEDIUM-HIGH |
| SR-EVID-0207 | useLayoutStore/useBrowserStore/useSessionStore | mainView L138/193；161/207/545；119-120 | FACT | HIGH |
| SR-EVID-0208 | useGridArchiveStore/MainArea/ActivityBar | gridOpen 近似"可见"用法 | INFERENCE | MEDIUM |
| SR-EVID-0209 | useLayoutStore L144/210-215；useBrowserStore L486-489 | gridToolbarOpen ↔ gridOpen 耦合 | FACT | HIGH |
| SR-EVID-0210 | useLayoutStore L188-190；useBrowserStore L149-151；UnifiedTabBar L147-157；BrowserHost L15；MainArea L155 | 双谓词非同义 | FACT | HIGH |
| SR-EVID-0211 | 全仓 `activeTabId =` | 写入点全集 | FACT | HIGH |
| SR-EVID-0212 | bridge.rs L269/466/757/818-821/890/1935/1974/4663-4685；useBrowserStore L179/204/500 | active_tab 无 reconciliation 镜像 | FACT/INFERENCE | MEDIUM-HIGH |
| SR-EVID-0213 | MainArea L102-103/150；ActivityBar L137-148/321/328；BookmarkStar L46-50；useBookmarkStore L54/192 | bmPanelOpen 单点派生 | FACT | HIGH |
| SR-EVID-0214 | useSystemStore.ts L128/241/264/285/321-323/383 | 无 activeTerminal 符号 | FACT | HIGH |
| SR-EVID-0215 | useWorkspaceStore L578-586/631-634/692-696；UnifiedTabBar L109-143 | currentLocalPath 未重实现 | FACT | HIGH |
| SR-EVID-0216 | 全仓 aiNavOpen | layout.aiNavOpen 零读者 | FACT | HIGH |
| SR-EVID-0217 | bridge.rs L289/169-175/4558-4573/4585-4593/4663-4683/4507-4512；types.ts L238-242/388-392；StatusBar L105-128；useSettingsStore L65-67/121 | hibernated 默认关 + 聚合镜像 | FACT | HIGH |

> 本轮全程只读；未改动 `src/`、`src-tauri/`、`scripts/` 与 00-16 号文档；未执行任何 git 写操作。
