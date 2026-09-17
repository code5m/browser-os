# 02 · State Sources — 状态源分类与风险

> Agent A · READ-ONLY 语义治理审计
> 分类法：SINGLE_SOURCE / MULTIPLE_WRITERS / MULTIPLE_SOURCES_OF_TRUTH / DERIVED_STATE_REIMPLEMENTED / MIXED_SEMANTICS / HIDDEN_STATE / STALE_DERIVED_STATE
> 风险：S0(无) S1(低) S2(中) S3(高) S4(严重/历史事故根因)

---

## 一、分类图例
- SINGLE_SOURCE：只有一个真源变量，所有读写集中。
- MULTIPLE_WRITERS：单真源但被多处直接写入（应收敛到 action）。
- MULTIPLE_SOURCES_OF_TRUTH：同一事实存在两个独立真源（危险）。
- DERIVED_STATE_REIMPLEMENTED：本应派生的状态被多处各自重算/或底层多源汇聚无单一写入。
- MIXED_SEMANTICS：一个变量名承载两种语义（如"存在"vs"可见"）。
- HIDDEN_STATE：状态真实存在但无响应式镜像，靠命令式副作用维护。
- STALE_DERIVED_STATE：派生/注释与实现不符。

---

## 二、状态源总表

| Semantic | State | Owner | Writers | Readers | Derived From | Duplicate | Classification | Risk |
|---|---|---|---|---|---|---|---|---|
| 主视图 | mainView | useLayoutStore | setView + ~6 处 store 直接赋值 | MainArea/ActivityBar/UnifiedTabBar/useBrowserStore/useBrowserHost | — | 无 | SINGLE_SOURCE（偏 MULTIPLE_WRITERS） | S1 |
| 激活页签 | activeTabId | useBrowserStore | tabSwitch/closeTabNow/tabNew/restoreSession/buildGrid/closeGridAll | tabSwitch/relocate/syncFreeze/CredentialList/UnifiedTabBar | — | 后端 active_tab（镜像） | SINGLE_SOURCE | S2 |
| 后端激活页签 | active_tab | bridge.rs AppState | tab_activate（由前端驱动） | scanner/hibernation/hide 逻辑 | 前端 activeTabId | 前端 activeTabId | SINGLE_SOURCE（镜像） | S2 |
| 浏览器类视图 | isBrowserView() | useLayoutStore | —（纯函数） | UnifiedTabBar/ActivityBar | mainView | 与 isBrowserVisible 同名近义 | DERIVED_STATE | S3 |
| 严格浏览器可见 | isBrowserVisible | useBrowserStore | —（computed） | BrowserHost.vue | mainView | 见上 | DERIVED_STATE（注释过时） | S3 |
| 宫格实例生命周期 | gridOpen | useBrowserStore | buildGrid(+)、closeGridAll(-)、closeGridOne(条件) | scheduleGrid/schedulePosition/syncFreeze/MainArea/ActivityBar/useGridArchiveStore | — | "grid visible"未单独存 | SINGLE_SOURCE（MIXED 语义风险） | S2 |
| 宫格工具条展开 | gridToolbarOpen | useLayoutStore | toggleGridToolbar、closeGridAll（跨 store） | ActivityBar/MainArea | — | 与 gridOpen 强耦合 | MULTIPLE_WRITERS | S2 |
| 收藏夹侧栏 | panelOpen | useBookmarkStore | togglePanel | ActivityBar(高亮)、MainArea | — | bmPanelOpen（派生） | SINGLE_SOURCE | S1 |
| 收藏夹可见(浏览器内) | bmPanelOpen | MainArea(派生) | —（computed） | MainArea(v-if) | panelOpen + mainView | 与 ActivityBar 高亮判据不一致 | DERIVED_STATE_REIMPLEMENTED | S3 |
| 浏览器 Dock | browserDockOpen/browserDockTab | useLayoutStore | toggleBrowserDock | MainArea/ActivityBar/App/UnifiedTabBar | — | 无 | SINGLE_SOURCE | S1 |
| 激活终端 | activeTermId | useSystemStore | setActiveTerm/spawnTerm/killTerm | termKeydown/onTermData/TerminalPane | — | 无 | SINGLE_SOURCE | S1 |
| 宫格配置 | gridMode/gridLayout/gridCount | useBrowserStore | ActivityBar.setGridLayout/setGridCount/buildGrid | MainArea/ActivityBar/useBrowserHost | — | 无 | SINGLE_SOURCE | S1 |
| 工作区当前位置 | currentLocalPath | useWorkspaceStore | —（computed，四级 fallback） | locateTo/FilePanel/relativeOf | inlineFile/previewDir/modTab.path/filePath | 底层 4 源 | DERIVED_STATE_REIMPLEMENTED | S2 |
| 凭据 | (无前端 store) | Rust keyring | 后端导入；前端仅 load/fill | CredentialList（只读镜像） | listBrowserCredentials；pageOrigin←activeTab.url | 无前端重复 | SINGLE_SOURCE（后端，前端 HIDDEN_STATE 镜像） | S1 |
| 原生 webview 显隐 | native bounds（Rust） | bridge.rs + useBrowserStore/useBrowserHost | syncViewVisibility/relocate/hideAllWebviews/schedulePosition | 无响应式读者（命令式下发） | mainView（watch） | 无前端镜像 | HIDDEN_STATE | S4 |
| BrowserHost DOM 显隐 | visibility(CSS)/v-show | BrowserHost.vue/MainArea | 渲染系统 | — | isBrowserVisible / mainView | 双层不一致风险 | MIXED_SEMANTICS（易碎双层） | S3 |
| 页签休眠 | hibernated_tabs | bridge.rs AppState | hibernate_tab/sweeper/tab_activate | hide_all_webviews/tab_activate | tabs + idle 计时 | 前端 tabs 未标记 | HIDDEN_STATE | S2 |
| AI 导航面板 | aiNavOpen | useBrowserStore(实际) / useLayoutStore(死重复) | gotoAI、ActivityBar、useWorkbenchStore | AINavPanel/ActivityBar | — | layout.aiNavOpen 重复 | MULTIPLE_SOURCES_OF_TRUTH（死代码） | S2 |
| 宫格会话号 | gridSession | useBrowserStore | buildGrid/forceGridRelayout | useBrowserHost/useGridArchiveStore | — | 无 | SINGLE_SOURCE | S1 |

---

## 三、关键冲突说明

1. **mainView 多 writer（S1→需收敛）**：真源唯一但约 6 处 store action 直接 `layout.mainView = ...`，建议全部改走 `setView`/`openModule` 等集中入口。
2. **isBrowserView vs isBrowserVisible（S3）**：二者不同义却被命名混淆；closeGridAll 注释写错的等式（EVID-0004）是 STALE_DERIVED_STATE，应修正注释或改名。
3. **gridOpen 语义混淆（S2）**：它是"实例生命周期"非"可见"；"grid visible"未单存，建议显式派生 `gridVisible = gridOpen && mainView==='grid'`。
4. **bmPanelOpen 不一致（S3）**：ActivityBar 用 panelOpen 高亮、MainArea 用 bmPanelOpen 挂载，非 browser 视图下出现"高亮但不渲染"。
5. **原生 webview 显隐 HIDDEN_STATE（S4）**：无前端响应式镜像，靠 watch(mainView) 命令式下发；历史上遮屏/空白事故根因，建议提升为单一"原生显隐意图"派生态 + 单一出口。
6. **aiNavOpen 重复真源（S2）**：layout 与 browser 双定义，仅后者被消费，前者为死代码，应删除。
7. **hibernated_tabs HIDDEN_STATE（S2）**：前端 tabs/activeTabId 不知休眠，建议同步 `hibernated` 字段。

---

## 四、风险计数结论

- **S4 发现：1 项** —— SEM-012 / 原生 webview 显隐 HIDDEN_STATE（历史遮屏/空白事故根因）。
- **S3 发现：3 项** —— (1) SEM-003 isBrowserView vs isBrowserVisible 命名混淆；(2) SEM-006 bmPanelOpen 高亮/挂载不一致；(3) SEM-013 BrowserHost 双层 DOM 显隐。
- **S2 发现：6 项** —— activeTabId 后端镜像、gridOpen 语义、gridToolbarOpen 多 writer、currentLocalPath 派生重算、hibernated_tabs 隐藏态、aiNavOpen 重复真源。
- **S1 发现：7 项** —— mainView、panelOpen、browserDockOpen、activeTermId、gridMode/gridLayout/gridCount、gridSession、credential。

> 总数：S4=1，S3=3，S2=6，S1=7。
