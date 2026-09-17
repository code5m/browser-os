# 01 · Semantic Inventory — 浏览器-OS 语义真源清单

> Agent A · READ-ONLY 语义治理审计
> 范围：`src/stores/**`、`src/composables/**`、`src/components/**`、`src/utils/**`、Rust 后端 `src-tauri/src/**`
> 断言纪律：每条证据标注 FACT / INFERENCE / RECOMMENDATION / UNVERIFIED，设计建议不伪装成现状。
> 治理五原则：ONE STATE TRUTH / ONE INTENT ENTRYPOINT / ONE CONTRACT / ONE SIDE-EFFECT EXIT / ONE LIFECYCLE OWNER。

---

## 一、核心语义条目（SEM）

### SEM-001 · mainView（当前主视图）
- Name: mainView
- Business meaning: 当前主区域展示哪个一级/模块视图（home/browser/grid/files/term/...）。是几乎所有"该不该渲染/该不该定位 webview"判断的总开关。
- Current representations:
  - `useLayoutStore.ts:138` `const mainView = ref<MainView>("home")` —— 唯一真源。
  - 直接写入处：`setView()`（`useLayoutStore.ts:193`）、`useBrowserStore.openBrowser`/各 tab 动作（`useBrowserStore.ts:161,545,207,503`）、`useSessionStore.restoreSession:120`、`useWorkbenchStore.open:19-23`、`useSystemStore.openTerminalAt:326` 等。
- Authoritative candidate: `useLayoutStore.mainView`（SINGLE_SOURCE）。
- Readers: MainArea.vue、ActivityBar.vue、UnifiedTabBar.vue、BrowserHost.vue、useBrowserStore（isBrowserVisible / syncViewVisibility / syncFreeze）、useBrowserHost（schedulePosition/scheduleGrid）。
- Writers: `setView` + 约 6 处 store action 直接赋值（非集中）。
- Derived expressions: `isBrowserView()`（`useLayoutStore.ts:188`，= browser||grid）、`isBrowserVisible`（`useBrowserStore.ts:149`，= mainView==='browser'）、`bmPanelOpen`（`MainArea.vue:103`）、`navTopViews`/`navDensity`（computed）。
- Side effects: 切换会触发 useBrowserStore 的 `watch(mainView)` → `syncViewVisibility`（桥接 hideAllWebviews/relocate），以及 MainArea 的 compactMode/term 视图 watch。即"视图切换"会驱动原生 webview 显隐。
- Duplicate expressions: 无重复真源；但有多处直接赋值（非单一写入函数），属 MULTIPLE_WRITERS 倾向。
- Known conflicts: 见 SEM-003（isBrowserView 与 isBrowserVisible 不同义被混用风险）。
- Known incidents: 历史"切走视图后宫格/页签残留屏幕"正是 mainView 与 native 显隐不同步所致。
- Target canonical meaning: 单一视图枚举真源；所有"是否展示 X"判定必须 derive 自它，不得再私存 boolean。
- Recommended owner: `useLayoutStore`（已承担）。
- Risk: S1
- Evidence IDs: EVID-0001, EVID-0002, EVID-0003

### SEM-002 · activeTabId / activeTab（激活页签）
- Name: activeTabId（及 computed activeTab）
- Business meaning: 当前前台展示、接收键盘/填充/冻结解冻的浏览器页签。
- Current representations:
  - `useBrowserStore.ts:19` `activeTabId = ref("")` —— 前端真源。
  - `useBrowserStore.ts:148` `activeTab = computed(() => tabs.find(...))`。
  - Rust 后端镜像：`bridge.rs:269` `AppState.active_tab: Mutex<Option<String>>`，由 `tab_activate`（`bridge.rs:4663`）写入。
- Authoritative candidate: 前端 `activeTabId` 为主，后端 `active_tab` 为其命令式镜像（并驱动原生 hide 其他页签）。
- Readers: tabSwitch/tabClose/relocate/syncFreeze、CredentialList（pageOrigin）、UnifiedTabBar（isActiveWeb）、ActivityBar。
- Writers: tabSwitch、closeTabNow、tabNew（新开即激活）、restoreSession、buildGrid/closeGridAll（重激活）。
- Derived expressions: `activeTab`。
- Side effects: 每次写入都触发 bridge.tabActivate → 后端 hide 其他 webview + idle 计时会话。
- Duplicate expressions: 后端 `active_tab` 与前端 `activeTabId` 双重记录同一事实。
- Known conflicts: 休眠重建路径：`tab_activate` 会先 `hibernated_tabs.remove` 再 spawn 重建，但前端 `activeTabId` 在 tabClose 后可能指向已休眠但仍在 `tabs` 列表的 id。
- Target canonical meaning: 单一激活页签真源，前端是唯一 writer；后端 `active_tab` 仅为原生布局镜像，不应被反向当作真源。
- Recommended owner: `useBrowserStore`。
- Risk: S2
- Evidence IDs: EVID-0007, EVID-0008, EVID-0017

### SEM-003 · isBrowserView vs isBrowserVisible（CASE-001 复核）
- Name: 两个"是否浏览器视图"判定
- Business meaning:
  - `isBrowserView()` = "浏览器类视图"（browser **或** grid 都算），用于"地址栏/页签条该不该显示"等 UI 范畴。
  - `isBrowserVisible`（computed）= "严格浏览器视图且可见"（仅 mainView==='browser'，不含 grid），用于 BrowserHost 的 CSS visibility。
- Current representations:
  - `useLayoutStore.ts:188-190` `isBrowserView()`。
  - `useBrowserStore.ts:149-151` `isBrowserVisible = computed(() => layout.mainView === "browser")`。
- Authoritative candidate: 两者都 derive 自 `mainView`，无独立真源；它们是**不同语义**的两个派生布尔，不是同一事实的两份拷贝。
- Readers:
  - `isBrowserView`：UnifiedTabBar.isActiveWeb（L157）、isActiveMod（L161）、ActivityBar（active 高亮）、browser 多处以它判断"浏览器类视图"。
  - `isBrowserVisible`：BrowserHost.vue:15（CSS visibility）、MainArea 注释引用。
- Writers: 无（均为 computed / 纯函数）。
- Known conflicts（复核结论）: **不是同一事实**。二者刻意不同义：`isBrowserView` 含 grid，`isBrowserVisible` 不含。但存在两类隐患：(1) 调用方可能因命名相近而误用（UnifiedTabBar.activateWeb 已专门注释"不能用 isBrowserView()（会把 grid 算入）"而改用 `mainView !== 'browser'`，L149-152，证明这是真实踩坑点）。(2) useBrowserStore.closeGridAll 注释里写了一句 `isBrowserVisible = !gridOpen && mainView==="browser"`（L491-494），与真实 computed（不含 !gridOpen）不一致——**注释已过时/错误**。
- Target canonical meaning: 保留两个派生判定，但名称需消除歧义（建议 `isBrowserLikeView` vs `isStrictBrowserVisible`），并在所有调用点用对。
- Recommended owner: 派生函数归属各自 store（现状）。
- Risk: S3
- Evidence IDs: EVID-0002, EVID-0003, EVID-0004

### SEM-004 · gridOpen（CASE-003 复核）
- Name: gridOpen
- Business meaning: **宫格子 webview 实例/生命周期是否存在**（createGrid 成功后置 true；关闭后置 false）。它**不是**"宫格当前是否可见"。
- Current representations:
  - `useBrowserStore.ts:20` `gridOpen = ref(false)`。
- Authoritative candidate: `useBrowserStore.gridOpen`（实例生命周期真源）。
- Writers:
  - `buildGrid` 成功路径 L319 置 `true`。
  - `closeGridAll` L486 置 `false`。
  - `closeGridOne` 在 count 2→1 时调 closeGridAll（L520-523），否则不改动 gridOpen。
- Readers: scheduleGrid（要求 `gridOpen && mainView==='grid'`，L99）、schedulePosition（L64 走宫格分支）、syncFreeze（L625 解冻所有格）、MainArea（`:class="{'grid-mode': browser.gridOpen}"` L151）、ActivityBar、useGridArchiveStore.extract（L48 校验 `!browser.gridOpen` 抛 GRID_NOT_OPEN）。
- Derived expressions: "宫格可见" = `gridOpen && mainView==='grid'`（**未单独存储**，到处 inline 组合）。
- Known conflicts（复核结论）: gridOpen 真义是"实例存在/生命周期"，与"可见"是两个独立事实。多处把 gridOpen 当作"可见"来短路逻辑（如 useGridArchiveStore 用 `!browser.gridOpen` 代表"窗口已关闭"）。若 gridOpen=true 但 mainView 不是 grid（例如留在 browser 视图但宫格实例未关），这些判定会误判"宫格可见"。closeGridAll 在切回 browser 前先翻 gridOpen=false（L486）就是为了不产生这种中间态。
- Target canonical meaning: 改名/加注释明确为 `gridInstanceAlive`；"grid visible" 作为独立 derived，避免混用。
- Recommended owner: `useBrowserStore`。
- Risk: S2
- Evidence IDs: EVID-0005, EVID-0009

### SEM-005 · gridToolbarOpen（宫格工具条展开）
- Name: gridToolbarOpen
- Business meaning: 活动条"宫格设置"扩展行是否展开（UI 态），并与宫格实例生命周期强耦合。
- Current representations: `useLayoutStore.ts:144` `gridToolbarOpen = ref(false)`。
- Authoritative candidate: `useLayoutStore.gridToolbarOpen`。
- Writers:
  - `toggleGridToolbar`（`useLayoutStore.ts:210-215`）：开 → 若 `!gridOpen` 则 buildGrid；关 → 若 `gridOpen` 则 closeGridAll。
  - `closeGridAll`（`useBrowserStore.ts:488`）：`layout.gridToolbarOpen = false`（跨 store 直接写）。
- Readers: ActivityBar（扩展行渲染 L371）、MainArea。
- Duplicate expressions: 与 `gridOpen` 近义但不同层（UI 展开 vs 实例存在），靠 toggleGridToolbar 维持同步，但 `buildGrid` 自身**不**设置 gridToolbarOpen，`closeGridAll` 才设。存在短暂不一致窗口。
- Known conflicts: 两个 writer（toggleGridToolbar、closeGridAll）跨 store 协作，若未来新增关闭宫格的路径却忘了清 gridToolbarOpen，会出现"工具条展开着但宫格没了"。
- Target canonical meaning: 明确"UI 展开态"与"实例态"双真源并加不变式测试。
- Recommended owner: `useLayoutStore`（但跨 store 写需收敛到 action）。
- Risk: S2
- Evidence IDs: EVID-0006

### SEM-006 · panelOpen / bmPanelOpen（收藏夹侧栏）
- Name: 收藏夹侧栏开关
- Business meaning: 收藏夹侧栏是否展开。
- Current representations:
  - `useBookmarkStore.ts:54` `panelOpen = ref(false)` —— 真源。
  - `MainArea.vue:103` `bmPanelOpen = computed(() => bookmarks.panelOpen && layout.mainView === "browser")` —— 派生（"在浏览器视图下才真挂载"）。
- Authoritative candidate: `useBookmarkStore.panelOpen`。
- Readers:
  - ActivityBar：用 `bookmarks.panelOpen` 直接做 **active 高亮**（L328、L141-148），未用 bmPanelOpen。
  - MainArea：用 `bmPanelOpen` 决定 `<BookmarkPanel v-if>` 挂载（L150）。
- Writers: `togglePanel`（`useBookmarkStore.ts:192`）；ActivityBar.onToggleBookmarkPanel 用条件 toggle 保证"任意视图点开都先切回 browser 再开"（L141-148）。
- Known conflicts（复核结论）: **高亮与挂载不一致**。ActivityBar 用 `panelOpen` 高亮（非 browser 视图也高亮），而 MainArea 用 `bmPanelOpen`（非 browser 视图不挂载）。结果：在 files/term 等视图点"收藏夹"，按钮高亮 active，但面板其实不渲染（需先切回 browser）。
- Target canonical meaning: 单一真源 panelOpen；active 高亮也应 derive 自 bmPanelOpen（或在非 browser 视图不显示 active）。
- Recommended owner: `useBookmarkStore`（真源）+ 组件统一使用派生。
- Risk: S3
- Evidence IDs: EVID-0011, EVID-0012

### SEM-007 · browserDockOpen（浏览器右侧 Dock）
- Name: browserDockOpen / browserDockTab
- Business meaning: 浏览器视图右侧 Dock（文件/终端/资源/会话）是否展开及其当前 Tab。
- Current representations: `useLayoutStore.ts:151-152` `browserDockOpen`、`browserDockTab`。
- Authoritative candidate: `useLayoutStore`（SINGLE_SOURCE）。
- Writers: `toggleBrowserDock`（L317）、ActivityBar、App.vue onOpenTerminal、UnifiedTabBar 快捷键。
- Readers: MainArea（aside v-if L158, L166-171）、ActivityBar。
- Risk: S1
- Evidence IDs: EVID-0001

### SEM-008 · activeTermId / activeTerminal（激活终端）
- Name: activeTermId（activeTerminal）
- Business meaning: 当前聚焦的终端实例（多 PTY 实例）。
- Current representations: `useSystemStore.ts:128` `activeTermId = ref("")`。
- Authoritative candidate: `useSystemStore.activeTermId`（SINGLE_SOURCE）。
- Writers: `setActiveTerm`（L321）、`spawnTerm`（空时自动设为首个，L241）、`killTerm`（重算为下一个，L264）、`termKeydown`（默认 tid）。
- Readers: termKeydown、onTermData/onTermChannelMsg 路由、TerminalPane。
- Risk: S1
- Evidence IDs: EVID-0013

### SEM-009 · gridMode / gridLayout / gridCount（宫格配置）
- Name: 宫格模式/布局/格数
- Business meaning: 宫格对比浏览(browse) vs 多 AI 群发(ai)；horizontal/quad/grid 布局；格数。
- Current representations: `useBrowserStore.ts:35-37`（`gridLayout`、`gridMode`）、`gridCount` L24。
- Authoritative candidate: `useBrowserStore`（SINGLE_SOURCE）。
- Writers: ActivityBar.setGridLayout/setGridCount、buildGrid（读 gridCount）、forceGridRelayout。
- Side effects: 改 gridCount/gridLayout 若 gridOpen 则需 buildGrid 重建。
- Risk: S1
- Evidence IDs: EVID-0005

### SEM-010 · workspace"当前本地位置"（workspace state）
- Name: currentLocalPath（工作区当前打开位置）
- Business meaning: IDE 文件树"当前定位到哪"，是派生的"当前目录/文件"。
- Current representations:
  - `useWorkspaceStore.ts:579-586` `currentLocalPath = computed(...)` 由 `inlineFile > previewDir > activeModTab.path > filePath` 四级 fallback。
  - 底层真源分散：`inlineFile`(L68)、`previewDir`(L60)、`layout.modTabs[activeModTab].path`、`filePath`(L53)。
- Authoritative candidate: 无单一真源；它是一个 **DERIVED_STATE_REIMPLEMENTED**，聚合 4 个可能互相矛盾的来源。
- Readers: locateTo、FilePanel 高亮、relativeOf（UnifiedTabBar）。
- Known conflicts: 四级 fallback 优先级是隐式约定，任一层被写入都会改变"当前位置"而不互相通知。
- Target canonical meaning: 引入显式 `currentLocalPath`（写入即改）替代四级 fallback，或至少集中到一个写入入口。
- Recommended owner: `useWorkspaceStore`。
- Risk: S2
- Evidence IDs: EVID-0014

### SEM-011 · credential state（账号凭据）
- Name: 浏览器已导入账号 / 凭据
- Business meaning: 与当前网页同源的可填充账号列表；密码明文只存系统密钥库（keyring），前端绝不持有。
- Current representations:
  - **无前端 store**。状态仅存在于组件局部：`CredentialList.vue:20-23` `items/loading/error/fillingId`（组件 ref，非 Pinia）。
  - 源数据：`bridge.listBrowserCredentials()`（CredentialList.vue:44）。
  - "当前网页 origin"：`CredentialList.vue:26-34` `pageOrigin = computed(new URL(activeTab.url).origin)` —— 派生自 `browser.activeTab.url`。
- Authoritative candidate: **Rust 端 keyring**（`bridge.fillBrowserCredential` 由 Rust 读 keyring 完成注入，前端只传 `credential_id + activeTabId`）。前端为只读、瞬时的局部镜像。
- Writers: 后端（导入/keyring）；前端只有 `load()` 拉取与 `fill()` 触发。
- Side effects: `fill()` 调 bridge.fillBrowserCredential —— 唯一副作用出口，符合 ONE SIDE-EFFECT EXIT。
- Duplicate expressions: 无前端重复真源（这是优点）；origin 匹配在 `matchesPage`（L36）与 `fill`（L73）各算一次（轻量重复）。
- Known conflicts: `pageOrigin` 派生自 `browser.activeTab.url`，若 activeTabId 指向休眠/重建中的页签，origin 可能瞬时错配导致填充被拒（ORIGIN_MISMATCH）。
- Target canonical meaning: 维持"密码不进前端"红线；凭据列表仍走后端单源，前端仅瞬时本地 ref。
- Recommended owner: Rust keyring（后端）+ CredentialList 组件（只读镜像）。
- Risk: S1
- Evidence IDs: EVID-0015

### SEM-012 · native webview hidden/visible（原生 webview 显隐，CASE-007）
- Name: 子 webview（浏览器页签 / 宫格子窗口）真实屏幕显隐
- Business meaning: 原生 GTK 子窗口当前是否在屏幕内（定位矩形 vs 移出屏幕 -30000）。这是"用户真正看到什么"的最终真源，但**前端没有对应的响应式变量镜像它**。
- Current representations:
  - 命令式：桥接 `hide_all_webviews`（`bridge.rs:635`）、`hide_webview`（`bridge.rs:608`）、`tab_position`（`bridge.rs:4299`）、`gridPosition`/`hideWebview` 等。
  - 前端驱动：`useBrowserStore.syncViewVisibility`（`useBrowserStore.ts:646-655`）在 `watch(mainView)`（L658-664）里分发；`useBrowserHost.schedulePosition/scheduleGrid` 经 rAF 去重后下发坐标。
- Authoritative candidate: 原生窗口实际 bounds（Rust 侧），由 mainView 经 watch 推导后命令式维护；**前端无响应式镜像**。
- Readers: 无响应式读者；只有命令式下发副作用。
- Writers: useBrowserStore（syncViewVisibility / relocate / hideAllWebviews / syncFreeze）+ useBrowserHost（定位调度器）。
- Side effects: 直接移动/隐藏原生窗口；50ms 去重缓存可能导致"移出屏幕"被去重吞掉（hide_* 特意用无去重的 hide_bounds 规避）。
- Known conflicts（复核结论，CASE-007）: **双层显隐**：
  - 第一层（DOM/CSS）：BrowserHost.vue 用 `visibility:hidden`（CSS）按 `isBrowserVisible`（=mainView==='browser'）控制。
  - 第二层（原生窗口）：靠 bridge 把 webview 移出/移回屏幕。
  两层由不同变量驱动，必须 keep-in-sync；历史上多次"切走视图后网页/宫格残留遮屏"或"切回后空白"均源于此不同步。
- Target canonical meaning: 把"原生显隐意图"提升为单一派生状态（例如 `nativeWebViewVisibleTarget`），由 ONE SIDE-EFFECT EXIT 统一下发，消除双层各自判断。
- Recommended owner: `useBrowserStore`（生命周期）+ `useBrowserHost`（定位），需收敛到一个出口。
- Risk: S4
- Evidence IDs: EVID-0009, EVID-0010

### SEM-013 · BrowserHost CSS visibility vs v-show（DOM 显隐双层，CASE-007 续）
- Name: BrowserHost 的 DOM 显隐
- Business meaning: BrowserHost 容器（提供 getBoundingClientRect 锚点）在 DOM 中的可见性。
- Current representations:
  - `BrowserHost.vue:12-16` 内层 `.browser-host` 用 `:style="{ visibility: browser.isBrowserVisible ? 'visible':'hidden' }"`（CSS visibility，非 display）。
  - `MainArea.vue:155` 外层 `<BrowserHost v-show="mainView==='browser' || 'grid'">`（v-show = display:none）。
- Known conflicts（复核结论）: 外层 v-show 在非 browser/grid 视图给 display:none（此时 rect=0），内层 visibility 在 grid 视图为 hidden（仍占布局、rect 非零）。任何把外层改成 v-if 或在 grid 视图误设 display:none 的改动都会让宫格定位失效。
- Risk: S3
- Evidence IDs: EVID-0010

### SEM-014 · hibernated_tabs（休眠态：CLOSE≠DESTROY）
- Name: 页签休眠（webview 销毁但 URL 保留）
- Business meaning: 非激活页签空闲超时后销毁原生 webview（释放内存），但保留其 URL/元信息于 `tabs`；再次激活时按 URL 重建。即"关闭 webview"≠"关闭页签"。
- Current representations:
  - Rust：`bridge.rs:289` `hibernated_tabs: Mutex<HashSet<String>>`；`hibernate_tab`（L4558-4573）写；`start_hibernation_sweeper`（L4578+）每 60s 扫描；`tab_activate`（L4664-4683）重建。
  - 前端：`useBrowserStore.tabs`（reactive）仍包含休眠页签，`activeTabId` 可能指向休眠 id。
- Authoritative candidate: 后端 `hibernated_tabs` 集合（前端 `tabs` 列表未区分休眠/存活）。
- Known conflicts: 前端 `tabs` 与 `activeTabId` 不知道某 id 已休眠；激活时依赖后端 `tab_activate` 隐式重建。前端无"isHibernated"派生，属 HIDDEN_STATE。
- Target canonical meaning: 前端 `tabs` 增加 `hibernated:boolean` 字段（由后端事件同步），relocate 跳过休眠项。
- Risk: S2
- Evidence IDs: EVID-0017

### SEM-015 · aiNavOpen（重复真源 / 死代码）
- Name: aiNavOpen（AI 导航面板开关）
- Business meaning: AI 导航面板是否展开。
- Current representations（**重复**）:
  - `useLayoutStore.ts:146` `aiNavOpen = ref(false)`，返回 L338。
  - `useBrowserStore.ts:43` `aiNavOpen = ref(false)`，返回 L680；L538 `gotoAI` 中置 false。
- Authoritative candidate: 实际被消费的是 `browser.aiNavOpen`（AINavPanel.vue:10,18；ActivityBar.vue:338,341；useWorkbenchStore.ts:27,29,32 快照恢复都用 browser.aiNavOpen）。
- Duplicate expressions: `layout.aiNavOpen` 被定义并返回，但**全仓无任何组件引用**（仅 store 内部声明），是死重复真源。
- Known conflicts: 若未来有人基于 `layout.aiNavOpen` 做高亮/持久化，会与 `browser.aiNavOpen` 分裂，构成 MULTIPLE_SOURCES_OF_TRUTH。
- Target canonical meaning: 删除 `useLayoutStore.aiNavOpen`，仅保留 `useBrowserStore.aiNavOpen`。
- Risk: S2
- Evidence IDs: EVID-0018

### SEM-016 · gridSession（宫格会话号 / 缓存失效令牌）
- Name: gridSession
- Business meaning: 每次 buildGrid 重建 +1，用于让定位层/冻结层"作废上次发送的缓存、必须重发"。
- Current representations: `useBrowserStore.ts:23` `gridSession = ref(0)`；buildGrid L302、forceGridRelayout L568 自增；useBrowserHost / useGridArchiveStore 读取。
- Authoritative candidate: `useBrowserStore.gridSession`（SINGLE_SOURCE）。
- Risk: S1
- Evidence IDs: EVID-0016

---

## 二、MUST-VERIFY CASE 复核结论

- CASE-001（isBrowserView vs mainView==='browser'）：**确认二者不是同一事实**。`isBrowserView()`=browser||grid（UI 范畴），`isBrowserVisible`=mainView==='browser'（严格可见，驱动 CSS）。二者刻意不同义，但命名接近+closeGridAll 过时注释造成混淆。风险 S3。
- CASE-003（gridOpen 真义）：**确认 gridOpen = 宫格实例/生命周期存在，而非可见**。"可见"= `gridOpen && mainView==='grid'`，未单独存储，多处 inline 组合。风险 S2。
- CASE-007（BrowserHost 可见性 / 原生 WebView 双层）：**确认双层显隐**。DOM 层 BrowserHost 用 CSS visibility 按 isBrowserVisible 控制，外层 v-show 按 browser||grid；原生层靠 bridge 把 webview 移屏/回屏，无前端响应式镜像。两层的不同步是历史遮屏/空白事故根因。风险 S4。
- CASE-008（组件是否直接编排生命周期）：**未发现组件自持生命周期标志位**。编排集中在 store action（buildGrid/closeGridAll/syncViewVisibility/tabSwitch 等）。组件仅作"意图入口"。结论：ONE LIFECYCLE OWNER 基本满足（store 层），但 grid 生命周期有 5+ 入口都 funnel 到 buildGrid，属正常"多意图入口→单动作"。风险 S2。

---

## 三、证据附录（Evidence）

### EVID-0001
- Claim: mainView 唯一真源在 useLayoutStore，且由 setView 与多处 store action 直接写入。
- Classification: FACT
- File: src/stores/useLayoutStore.ts  Symbol: useLayoutStore.mainView (ref, L138); setView (L193)
- Observed behavior: `const mainView = ref<MainView>("home")`；`setView(v){ mainView.value = v; ... }`；另有 useBrowserStore.openBrowser 等直接 `layout.mainView = "browser"`。
- Callers: MainArea/ActivityBar/UnifiedTabBar/useBrowserStore/useSessionStore/useWorkbenchStore/useSystemStore。Callees: syncViewVisibility watch、isBrowserView、isBrowserVisible。
- Why supports claim: 定义点唯一，所有视图渲染与 webview 定位均 reads 它。
- Confidence: HIGH

### EVID-0002
- Claim: isBrowserView() = mainView==='browser' || 'grid'（包含 grid）。
- Classification: FACT
- File: src/stores/useLayoutStore.ts  Symbol: isBrowserView (L188-190)
- Observed behavior: `return mainView.value === "browser" || mainView.value === "grid";`
- Confidence: HIGH

### EVID-0003
- Claim: isBrowserVisible computed = mainView==='browser'（严格，不含 grid）。
- Classification: FACT
- File: src/stores/useBrowserStore.ts  Symbol: isBrowserVisible (L149-151)
- Observed behavior: `const isBrowserVisible = computed(() => layout.mainView === "browser");`
- Confidence: HIGH

### EVID-0004
- Claim: closeGridAll 注释称 "isBrowserVisible = !gridOpen && mainView==='browser'"，与真实 computed（不含 !gridOpen）不一致——注释过时/错误。
- Classification: UNVERIFIED（作为"当前行为"不成立；作为"注释存在"为 FACT）
- File: src/stores/useBrowserStore.ts  Symbol: closeGridAll 注释 (L491-494)
- Why supports claim: 注释与实现不符，属 STALE_DERIVED_STATE 误导风险。
- Confidence: HIGH（注释存在为 FACT；等式作为行为为 UNVERIFIED/错误）

### EVID-0005
- Claim: gridOpen 表示宫格实例生命周期（存在/不存在），非可见；可见还需 mainView==='grid'。
- Classification: FACT
- File: src/stores/useBrowserStore.ts  Symbol: gridOpen (L20); buildGrid (L319); closeGridAll (L486); scheduleGrid (L99); syncFreeze (L625)
- Observed behavior: buildGrid 在 createGrid 后 `gridOpen.value = true`；closeGridAll 置 false；scheduleGrid 要求 `gridOpen && mainView==='grid'`；syncFreeze 仅当 gridOpen 才解冻各格。
- Confidence: HIGH

### EVID-0006
- Claim: gridToolbarOpen 由 toggleGridToolbar 与 closeGridAll 两处写入，且与 gridOpen 强耦合。
- Classification: FACT
- File: src/stores/useLayoutStore.ts  Symbol: toggleGridToolbar (L210-215); src/stores/useBrowserStore.ts Symbol: closeGridAll (L488)
- Confidence: HIGH

### EVID-0007
- Claim: activeTabId 前端唯一真源，activeTab 派生。
- Classification: FACT
- File: src/stores/useBrowserStore.ts  Symbol: activeTabId (L19); activeTab (L148)
- Confidence: HIGH

### EVID-0008
- Claim: Rust AppState.active_tab 是前端 activeTabId 的命令式镜像，由 tab_activate 写入，并驱动 hide 其他页签与 idle 计时；休眠重建时先 remove hibernated 再 spawn。
- Classification: FACT
- File: src-tauri/src/bridge.rs  Symbol: AppState.active_tab (L269); tab_activate (L4663-4685)
- Confidence: HIGH

### EVID-0009
- Claim: 原生 webview 显隐无前端响应式镜像，由 syncViewVisibility watch(mainView) 命令式下发 bridge.hideAllWebviews/relocate；历史注释记录"切走/切回不处理会空白或残留"。
- Classification: FACT
- File: src/stores/useBrowserStore.ts  Symbol: syncViewVisibility (L646-655); watch(L658-664)  src-tauri/src/bridge.rs Symbol: hide_all_webviews (L635), hide_webview (L608)
- Confidence: HIGH

### EVID-0010
- Claim: BrowserHost 双层 DOM 显隐：内层 CSS visibility 按 isBrowserVisible，外层 v-show 按 browser||grid。
- Classification: FACT
- File: src/components/browser/BrowserHost.vue (L12-16)  src/components/layout/MainArea.vue (L139-156)
- Confidence: HIGH

### EVID-0011
- Claim: 收藏夹 active 高亮用 panelOpen（ActivityBar），挂载用 bmPanelOpen=panelOpen&&mainView==='browser'（MainArea），两者不一致。
- Classification: FACT
- File: src/components/layout/MainArea.vue (L103, L150)  src/components/layout/ActivityBar.vue (L328, L141-148)
- Confidence: HIGH

### EVID-0012
- Claim: panelOpen 单一真源在 useBookmarkStore，togglePanel 唯一写入。
- Classification: FACT
- File: src/stores/useBookmarkStore.ts  Symbol: panelOpen (L54); togglePanel (L192-194)
- Confidence: HIGH

### EVID-0013
- Claim: activeTermId 单一真源在 useSystemStore，setActiveTerm/spawnTerm/killTerm 管理。
- Classification: FACT
- File: src/stores/useSystemStore.ts  Symbol: activeTermId (L128)
- Confidence: HIGH

### EVID-0014
- Claim: currentLocalPath 是 DERIVED_STATE_REIMPLEMENTED，由 inlineFile>previewDir>modTab.path>filePath 四级 fallback。
- Classification: FACT
- File: src/stores/useWorkspaceStore.ts  Symbol: currentLocalPath (L579-586)
- Confidence: HIGH

### EVID-0015
- Claim: 凭据无前端 store，仅 CredentialList 组件局部 ref 镜像后端；password 不进前端；pageOrigin 派生自 activeTab.url；权威在 Rust keyring。
- Classification: FACT（前端无 store）；INFERENCE（keyring 为权威）
- File: src/components/browser/CredentialList.vue  Symbol: items (L20), pageOrigin (L26-34), fill (L68-89)
- Confidence: HIGH

### EVID-0016
- Claim: gridSession 单一真源在 useBrowserStore，自增用于让定位/冻结缓存失效。
- Classification: FACT
- File: src/stores/useBrowserStore.ts  Symbol: gridSession (L23); buildGrid (L302); forceGridRelayout (L568)
- Confidence: HIGH

### EVID-0017
- Claim: 后端 hibernated_tabs 表示"webview 销毁但 URL 保留"；前端 tabs 列表与 activeTabId 不区分休眠，激活时由后端隐式重建。
- Classification: FACT
- File: src-tauri/src/bridge.rs  Symbol: AppState.hibernated_tabs (L289); hibernate_tab (L4558-4573); tab_activate (L4664-4683)
- Confidence: HIGH

### EVID-0018
- Claim: aiNavOpen 在 useLayoutStore 与 useBrowserStore 两处定义；仅 browser.aiNavOpen 被消费，layout.aiNavOpen 为死重复真源。
- Classification: FACT
- File: src/stores/useLayoutStore.ts (L146, 返回 L338)  src/stores/useBrowserStore.ts (L43, 返回 L680, 写 L538)
- Confidence: HIGH
