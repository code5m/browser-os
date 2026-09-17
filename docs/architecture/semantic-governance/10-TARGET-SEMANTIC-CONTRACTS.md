# 10 · Target Semantic Contracts — 最小非大重构目标契约
> Agent H · READ-ONLY 语义治理审计（基于 01–09 + F-FEATURES + 源码复核）
> 原则：ONE STATE TRUTH / ONE INTENT ENTRYPOINT / ONE CONTRACT / ONE SIDE-EFFECT EXIT / ONE LIFECYCLE OWNER
> 断言纪律：每条标注 FACT（已对源码验证）/ INFERENCE / RECOMMENDATION / UNVERIFIED；建议不伪装成现状。
> 范围：仅定义“最小、非破坏性”的目标契约；落地方式见 `11-CHECKER-DESIGN.md`。

---

## 0. 复核结论（对 01–09 / F 的修订与确认）

- **确认 FACT**：`isBrowserVisible` 当前真义是 `layout.mainView === "browser"`（useBrowserStore.ts:149-151），**不含** `!gridOpen`。`check-grid-close-logic.mjs` G2（L112）与 closeGridAll 注释（L491-494）仍断言旧式 `!gridOpen && mainView==="browser"`，属 **STALE_DERIVED_STATE / checker 漂移**，需修脚本而非改源码。
- **确认 FACT**：`gridOpen` 仅两处赋值——`buildGrid:319`(=true) 与 `closeGridAll:486`(=false)；是“宫格实例/生命周期是否存在”，不是“可见”。可见 = `gridOpen && mainView==='grid'`，且此派生未单独存储。
- **确认 FACT**：`bridge.tabClose/tabNew/tabOpen/tabActivate/tabReload/tabPosition` 仅出现在 useBrowserStore.ts（BookmarkPanel.vue:51 为注释）；`bridge.{deletePath,writeFile,renamePath,movePath,createFile}` 仅出现在 useWorkspaceStore.ts。
- **确认 FACT**：`skill_list/agent_list/skill_install/agent_install/skill_run` 在 bridge.ts 为 **typed invoke 占位**（L401/403/418/421/432），Rust 端无 `#[tauri::command]` 实现；`AGENT_SKILL_COMMANDS_AVAILABLE=false`（bridge.ts:102）拦截调用，故为“休眠契约占位”，非运行期损坏。
- **确认 FACT**：`isBrowserView()`（layout，含 grid）= browser||grid；`isBrowserVisible`（browser，不含 grid）= mainView==='browser'。二者刻意不同义，UnifiedTabBar.activateWeb(L147-154) 显式用 `mainView !== "browser"` 而非 `isBrowserView()`。
- **UNVERIFIED→FACT 修正**：docs 03/08 称“gridOpen 视为可见”是历史误解；本契约以“生命周期 vs 可见”两独立事实为准。
- **RECOMMENDATION（非现状）**：下列契约描述“目标态”，当前源码部分仍未达标（已逐条标注 Current drift）。

---

## CONTRACT-GRID-LIFECYCLE · 宫格生命周期 vs 可见解耦

- **Classification**: FACT（状态语义）/ RECOMMENDATION（收敛方向）
- **唯一状态**：
  - `gridOpen`（useBrowserStore.ts:20）= 宫格子 webview 实例/进程 **生命周期是否存在**（CREATE 后置 true，DESTROY 后置 false）。
  - “宫格可见” = `gridOpen && mainView==='grid'`（派生，不得单独存储为第三个标志）。
- **唯一动作入口**：`buildGrid()`（CREATE，useBrowserStore.ts:300）= 唯一置 `gridOpen=true`；`closeGridAll()`（DESTROY，:483）= 唯一置 `gridOpen=false`。
- **唯一 owner**：`useBrowserStore`（生命周期）；`useBrowserHost` 仅做定位/调度，不决定生命周期。
- **允许的副作用**：
  - buildGrid → `bridge.createGrid(n, urls)`（幂等重建，内部先 close_grid 再 spawn）+ `gridSession++`。
  - closeGridAll → `gridOpen=false` + `bridge.closeGrid()`（Rust HideWindow→CloseTab→shutdown_all 杀进程）+ 复位 `mainView='browser'` + 重激活 activeTab + `relocate()`。
- **绝对禁止**：
  - 任何“只置 `gridOpen=false` 却不调 `bridge.closeGrid()`”的 hide 操作（孤儿子进程，≈450MB/个）。
  - 把 `gridOpen` 当作“可见”短路判定（如 `!gridOpen` 推回可见公式）。
  - 新增第三个“grid visible”标志与 `gridOpen`/`mainView`/`gridToolbarOpen` 三态脱节。
  - `gridOpen.value` 赋值出现在 buildGrid/closeGridAll 之外的任何文件或函数。
- **Current drift（FACT）**：仅 2 处赋值且已收敛（:319 / :486），本契约当前 **满足**；需看守以防回归（SMF-003）。
- **合法转移矩阵**（来自 05 §2.2，FACT）：
  - ABSENT→VISIBLE: buildGrid/create_grid（含隐式 DESTROY 旧格）
  - VISIBLE→HIDDEN: 视图切换 → `hideAllWebviews`/`HideWindow`（**不 kill**）
  - HIDDEN→VISIBLE: 切回 grid → `gridPosition`+show()
  - VISIBLE/HIDDEN→DESTROYED: closeGridAll/closeGridOne → `close_grid`/`kill_child`

---

## CONTRACT-BROWSER-VISIBILITY · BrowserHost 可见性单一派生

- **Classification**: FACT（当前公式）/ RECOMMENDATION（收敛判定归一处）
- **唯一状态**：`isBrowserVisible`（useBrowserStore.ts:149-151）= `layout.mainView === "browser"`（**不含** `!gridOpen`）。它就是 BrowserHost 的 CSS `visibility` 真源（BrowserHost.vue:15）。
- **唯一动作入口**：`mainView` 经由 `setView` 写入（见 CONTRACT-VIEW-SWITCH）；可见性完全 derive，无独立 writer。
- **唯一 owner**：`useBrowserStore.isBrowserVisible`（派生）+ `useLayoutStore.mainView`（真源）。
- **允许的副作用**：CSS `visibility:hidden/visible` on `.browser-host`；原生显隐由 `syncViewVisibility`/`schedulePosition` 经 `mainView` watch 下发。
- **绝对禁止**：
  - 在 `isBrowserVisible` 公式中重新引入 `!gridOpen`（会令“宫格仅 flag 未销毁”时误隐藏浏览器 webview；SMF-004/007）。
  - 用 `gridOpen` 替代 `mainView==='browser'` 去驱动 BrowserHost 可见性或 schedulePosition 的 early-return。
  - 任何组件私存“浏览器是否可见”boolean 副本。
- **Current drift（FACT/STALE）**：源码公式已正确（仅 mainView）；但 `check-grid-close-logic.mjs` G2(L112) 与 closeGridAll 注释(L491-494) 仍断言旧式 `!gridOpen && mainView==="browser"` → **脚本当前 FAIL，需修脚本与注释，不得改源码**。

---

## CONTRACT-VIEW-SWITCH · 主视图切换唯一 writer

- **Classification**: RECOMMENDATION（目标态，当前未达标）
- **唯一状态**：`mainView`（useLayoutStore.ts:138）— 一级视图枚举真源。
- **唯一动作入口**：`setView(v)`（:193，含 navSection/fileEditorOpen 清理）；模块页签经 `openModule`/`closeModTab` 内部 funnel 到 `setView`。
- **唯一 owner**：`useLayoutStore`。
- **允许的副作用**：`setView` 清理 `fileEditorOpen`/`navSection` 并写 `mainView.value=v`，触发 `watch(mainView)` → `syncViewVisibility` → 原生 webview 重排/隐藏。
- **绝对禁止**：
  - 外部 `layout.mainView = "..."` 裸赋值（绕过 setView 的清理，留下 stale 覆盖层/菜单）。
  - 在组件内用 `layout.mainView = 'browser'` 代替 `layout.setView('browser')`（DUP-003 / SMF-010）。
- **Current drift（FACT，需整改）**：外部裸赋值现存于：
  - `src/components/workspace/FileEditor.vue:11` `layout.mainView = "browser"`
  - `src/components/layout/TopBar.vue:21` `layout.mainView = 'browser'; browser.relocate()`
  - `src/stores/useBrowserStore.ts:161,207,323,503,545`
  - `src/stores/useSessionStore.ts:120`
  - `src/stores/useWorkspaceStore.ts:912,924`
  （其中 buildGrid:322-324 的 `mainView='grid'` 受 `!== 'grid' && !== 'browser'` 守卫约束，也应收口为 setView/openModule 语义。）

---

## CONTRACT-GRID-EXIT · 宫格退出单一意图

- **Classification**: RECOMMENDATION（目标态）
- **唯一状态**：退出意图 `exitGrid(mode: 'hide' | 'destroy')`；`mode` 决定仅 offscreen(keep child) 还是 DESTROY(kill child)。
- **唯一动作入口**：`exitGrid('hide')` → 仅 `syncViewVisibility`/HIDE 路径；`exitGrid('destroy')` → `closeGridAll()`。`toggleGridToolbar` 与两条导航路径一律经此意图，不得自行选择生命周期结果。
- **唯一 owner**：`useBrowserStore`（意图裁决）；`useLayoutStore.toggleGridToolbar` 委托。
- **允许的副作用**：同 CONTRACT-GRID-LIFECYCLE 的 buildGrid/closeGridAll 副作用集。
- **绝对禁止**：
  - 组件手拼 `setView(...) + buildGrid()` / `setView(...) + closeGridAll()`（HomeLaunchers.openArea、ActivityBar.onItem/setGridCount/setGridLayout 历史 pattern，CASE-008）。
  - 组件先写 `browser.gridMode/gridLayout/gridCount` 再调 `buildGrid()`（“先配参再建”应封装进意图）。
  - 新增“minimize grid to dock”仅置 `gridOpen=false` 而不 destroy（SMF-003 / DUP-006）。
- **Current drift（FACT）**：`ActivityBar.vue`、`HomeLaunchers.vue`、`useWorkbenchStore.ts` 等处仍手拼 buildGrid/closeGridAll（docs 03 CASE-008）；需逐步收敛到 `exitGrid`。

---

## CONTRACT-IPC-CONTRACT · 跨层 IPC 单一契约闭包

- **Classification**: FACT（现状）/ RECOMMENDATION（看门狗修正）
- **唯一状态**：每个 FE `invoke("cmd")` ↔ Rust `#[tauri::command]` ↔ ACL 三源闭包。
- **唯一动作入口**：`src/bridge.ts` 的封装函数（typed wrapper）为唯一 FE 调用表面；`check-command-set-consistency.py` 为三源门禁。
- **唯一 owner**：bridge.ts（FE 侧）+ main.rs `generate_handler!` + `permissions/*.toml`（后端侧）。
- **允许的副作用**：已注册且 ACL 放行的命令；契约占位（agent/skill）在未落地前由 `AGENT_SKILL_COMMANDS_AVAILABLE=false` 拦截。
- **绝对禁止**：
  - FE `invoke` 指向无 Rust 实现、且未登记到 KNOWN 的命令（新增漂移）。
  - `create_grid` 契约半改：FE 省略 `urls` 参数，或 Rust 签名去掉 `urls`（SMF-002）；`buildGrid` 必须保留 `if (degraded) gridCount.value = created` 降级回写（:317）。
- **Current drift（FACT，需修 checker）**：
  - `scripts/check-command-set-consistency.py:83` 的 `invoked()` 正则 `invoke\(\s*["\']([^"\']+)["\']` **只匹配 untyped invoke**，漏掉 5 个 typed 占位（skill_list/agent_list/skill_install/agent_install/skill_run，bridge.ts:401/403/418/421/432）→ 零门禁保护。
  - 修复：正则扩为 `invoke(?:<[^>]*>)?\(\s*["\']([^"\']+)["\']`；把 5 个命令加入 `KNOWN["main_invoke_not_registered"]`（或落地 Rust 后端）。
  - 已知 4 个 untyped 占位（agent_chat/agent_chat_cancel/confirm_skill_install/confirm_agent_install，bridge.ts:425/428/435/438）已在 KNOWN，继续保留。

---

## CONTRACT-NATIVE-SHOW · 原生 webview 显隐单一适配

- **Classification**: FACT（机制耦合）/ RECOMMENDATION（收敛出口）
- **唯一状态**：原生 webview 真实可见性 = Rust 侧 bounds（mainView 经 watch 推导后命令式维护）；前端无响应式镜像（07 CASE-007）。
- **唯一动作入口（canonical show/hide adapter）**：`useBrowserStore.syncViewVisibility`（:646）+ `useBrowserHost.schedulePosition/scheduleGrid`（隐藏经 `bridge.hideAllWebviews`/`hideWebview`，显示经 `position`→`set_visible(true)`/`win.show()`）。
- **唯一 owner**：`useBrowserStore`（生命周期显隐）+ `useBrowserHost`（定位）。
- **允许的副作用**：
  - show 隐式经 `tab_position`/`grid_position`→`UpdateRect` 内的 `set_visible(true)`/`win.show()`（POSITION==SHOW 机制耦合，EVID-005）。
  - hide 经 `hide_webview`/`hide_all_webviews`/`HideWindow`（**屏外 -30000 移出，绝不 `set_visible(false)`，避免 WebKitGTK 死锁**，bridge.rs:567-569）。
- **绝对禁止**：
  - “position-as-hide hack”：用 `bridge.tabPosition(id, {x:-30000,...})` 把 webview 移屏外当 hide 用（useBrowserStore.ts:170 现存 hack，EVID-BYPASS-007）→ 应改走 `bridge.hideWebview`/`hideAllWebviews`。
  - 组件直接调 `bridge.tabPosition/gridPosition/hideWebview/hideAllWebviews/tabActivate`（native adapter leak，07 CASE-009），除 useBrowserStore.ts 与 useBrowserHost.ts 外。
  - 前端的 `bridge.*.hide()` 类原生 hide 调用（Webview 死锁风险，已由 check-native.mjs 守 `.hide()` 模式）。
- **Current drift（FACT）**：native 命令调用已收敛于 useBrowserStore/useBrowserHost；唯 :170 的 -30000 位置当 hide 是待修 hack。CASE-005（position 蕴含 show）为 **架构性耦合，非 bug**，契约承认之、仅收敛出口。

---

## CONTRACT-TAB-INTENT · 页签生命周期单一入口

- **Classification**: FACT（现状良好）/ RECOMMENDATION（看守）
- **唯一状态**：`activeTabId`/`tabs`（useBrowserStore.ts:19/18）；后端 `active_tab` 仅为命令式镜像。
- **唯一动作入口（store actions）**：`tabNew`/`tabSwitch`/`tabClose`/`closeTabNow`/`tabReload`/`tabNavigate` 等；组件只调意图，不裸调 bridge。
- **唯一 owner**：`useBrowserStore`。
- **允许的副作用**：store action 内 `recordClose`→`tabs.splice`→`bridge.tabClose`→`tabActivate`→`relocate`→`syncFreeze` 全链路。
- **绝对禁止**：
  - 组件裸调 `bridge.tabClose(id)`/`bridge.tabNew(...)`（绕过 recentlyClosed 记录与 tabs[] 维护，DUP-008 / SMF-008）。
  - `UnifiedTabBar.activateWeb` 改用 `layout.isBrowserView()`（它会把 grid 计入，导致 grid 视图点页签不显示；必须用 `layout.mainView !== "browser"`，DUP-009 / SMF-001 / CASE-001）。
- **Current drift（FACT，已满足）**：`bridge.tabClose/tabNew/tabOpen/tabActivate/tabReload/tabPosition` 仅见于 useBrowserStore.ts；UnifiedTabBar 正确用 store action 与精确 `mainView` 判定。看守防回归。

---

## CONTRACT-CREDENTIAL-OWNER · 凭据所有权与红线

- **Classification**: FACT（无 store）/ RECOMMENDATION（加 facade）
- **唯一状态**：凭据权威在 Rust `keyring`（`fillBrowserCredential` 由 Rust 读 keyring 注入）；前端为只读、瞬时局部镜像（CredentialList.vue:20-23 `items/loading/error/fillingId` 为组件 ref，非 Pinia）。
- **唯一动作入口（intent facade）**：目标态——所有 `listBrowserCredentials/fillBrowserCredential/importBrowserCredentials` 经一个 `useCredentialStore`/intent facade；组件不得直连 bridge。
- **唯一 owner**：Rust keyring（后端）+ 前端 facade（只读镜像）；`pageOrigin` 派生自 `browser.activeTab.url`（CredentialList.vue:26-34）。
- **允许的副作用**：`fill()` 仅传 `{credential_id, activeTabId}` 给 `bridge.fillBrowserCredential`（唯一出口，符合 ONE SIDE-EFFECT EXIT）。
- **绝对禁止**：
  - 前端任何代码持有明文 password / 写入 Pinia ref / 进 localStorage / 进日志（FEAT-005 / FEAT-009 安全红线，CredentialList.vue:10-15）。
  - 组件直接 `bridge.fillBrowserCredential/listBrowserCredentials/importBrowserCredentials`（当前 CredentialList.vue:44/79 直连，**需加 facade**）。
  - 自动填充/自动提交/自动登录（仅用户主动触发 + exact origin 匹配）。
- **Current drift（FACT，需整改）**：无 `useCredentialStore`；CredentialList.vue 直连 bridge（04 标记 STORE_OWNERSHIP_LEAK + NATIVE_POLICY_LEAK）。
