# 16 · REVIEW-REPORT — 语义治理审计独立复核

> Review Agent（独立复核，READ-ONLY）。对 `docs/architecture/semantic-governance/` 下 01–09 与 `_findings/F-FEATURES.md` 的全部断言，重新对实际源码（`src/**`、`src-tauri/**`、`scripts/**`）抽样核验，不接受其它 Agent 的结论为事实。

## 1. 方法论说明（Methodology）

- 本复核**未信任**任何 Agent 的 "FACT" 标注；对每条 S4 / 所有生命周期红线 / 所有 IPC DRIFT_RISK / 所有 MULTIPLE_SOURCE_OF_TRUTH / 所有隐藏副作用 / 跨检查器冲突，均用 `search_content` / `read_file` 重读符号与行区间。
- 重点核验了：状态真源（`useBrowserStore.ts` / `useLayoutStore.ts` / `useWorkspaceStore.ts`）、生命周期与 IPC 后端（`bridge.rs` / `main.rs`）、DOM 双层（`BrowserHost.vue` / `MainArea.vue` / `ActivityBar.vue` / `UnifiedTabBar.vue` / `useBrowserHost.ts`）、检查器（`check-command-set-consistency.py` / `check-grid-close-logic.mjs` / `check-ui.mjs` / `check-session-persistence-policy.py` / `check-native-webview-overlay.mjs`）。
- 行号以**复核当日源码**为准；部分与文档引用行号有 ±几行偏移，但符号与语义一致。
- 复核结论中 "CONFIRMED" 指：复核抽样与 Agent 描述**一致**；"PARTIALLY_CONFIRMED" 指事实成立但严重程度/范围需修正；"CONTRADICTED" 指与源码不符；"INSUFFICIENT_EVIDENCE" 指未能在抽样范围内核实。

---

## 2. 逐条裁定（Per-Claim Verdicts）

### 2.1 各文档 S4 发现

**CLAIM-S4-01 (01 SEM-012 / 02 S4)：原生 webview 显隐为 HIDDEN_STATE，无前端响应式镜像，是历史遮屏/空白事故根因。**
- Agent verdict：S4 / CONFIRMED（FACT）。
- Reviewer finding：复核确认。`syncViewVisibility`（useBrowserStore.ts:646-654）在 `watch(mainView)`（:658-664）里命令式下发 `hideAllWebviews`/`relocate`；`BrowserHost.vue:15` 仅用 CSS `visibility` 控占位壳，真实显隐由 Rust `update_rect(-30000)` 与 `set_visible(true)` 维护，前端无响应式镜像。两层不同步确为事故机理。
- Evidence re-found：`useBrowserStore.ts:646-664`；`BrowserHost.vue:15`；`bridge.rs:531-564`(set_visible)、`570-597`(hide_bounds)。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-S4-02 (03 / 04 S4)：grid→browser 切换存在"销毁 vs 隐藏"分叉（CASE-002），同一用户意图"离开宫格"经 closeGridAll=DESTROY，经 ActivityBar 导航=HIDE，属 S4。**
- Agent verdict：S4 / CONFIRMED。
- Reviewer finding：分叉**事实成立**。`ActivityBar.onItem`（ActivityBar.vue:150-177）在 `v==="browser"` 走 `layout.setView("browser")`（:172）仅改 `mainView`，经 watcher→`syncViewVisibility`→`hideAllWebviews`（HIDE，gridOpen 保持 true）；而 closeGridAll（useBrowserStore.ts:483-511）→`bridge.closeGrid()`→`close_grid`（DESTROY，kill 子进程）。两条路径生命周期结果不同。
- Evidence re-found：`ActivityBar.vue:171-173`；`useBrowserStore.ts:483-511`；`bridge.rs:3915-3944`；`useBrowserStore.ts:646-654`。
- Verdict：CONFIRMED（分叉事实）· Confidence：HIGH
- 注意：该"分叉"被 03/04 标 S4，但 05 把"VIEW SWITCH != DESTROY"本身判为未违反（S4=0）。二者衡量对象不同（见 CONFLICT-03）。

**CLAIM-S4-03 (05 S4=0)：视图切换≠销毁、隐藏≠关闭、激活≠导航、终端 PTY 存活于视图切换——无任何运行时红线违反。**
- Agent verdict：S4=0 / CONFIRMED（FACT）。
- Reviewer finding：复核支持。`syncViewVisibility` 对 grid 仅 `hideAllWebviews`（屏外移，不 kill）；`close_tab`/`close_grid` 不由视图切换触发；`tab_activate` 不含 navigate；`TerminalPane.onBeforeUnmount` 不 kill。
- Evidence re-found：`useBrowserStore.ts:646-654`；`bridge.rs:4663-4741`（activate 无 navigate）；`bridge.rs:794-823`（close_tab 独立）。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-S4-04 (06 S4=0)：所有 grid/tab/terminal/credential/bookmark/workspace/file 主题命令 IPC 契约一致，BROKEN=0。**
- Agent verdict：S4=0 / CONSISTENT。
- Reviewer finding：复核支持（核心契约一致，见 2.3 CASE-006/005）。但 06 将 5 个未落地 agent/skill 命令 + 治理文档 `sync_browser_scene` 矛盾标为 DRIFT_RISK(S3)，不计入 S4，符合其口径。
- Verdict：CONFIRMED（按 06 定义）· Confidence：HIGH

**CLAIM-S4-05 (07 S4=3)：position 蕴含 show（EVID-005-TAB/GRID + EVID-SHOW-007）为隐藏副作用 S4。**
- Agent verdict：S4 / CONFIRMED（FACT）。
- Reviewer finding：复核确认。`apply_bounds_inner`（bridge.rs:555）`set_visible(true)`；`GridCmd::UpdateRect`（main.rs:391-424，:423 `win.show()`）含 show。确实无独立 `show_webview` 命令。此为真实隐藏副作用。
- Evidence re-found：`bridge.rs:531-564`（:555）；`main.rs:391-424`（:423）；`bridge.rs` 全文无 `fn show_*`。
- Verdict：CONFIRMED · Confidence：HIGH
- 注意：与 05（把 POSITION==SHOW 标为 INFERENCE，非违规）及 03（标 POSITION != SHOW ✓）存在语义口径冲突，见 CONFLICT-01/02。

**CLAIM-S4-06 (08 S4=4：DUP-001 / DUP-002 / DUP-006 / DUP-009)：四个重复/歧义语义，模型易改错且静默出错。**
- Agent verdict：S4 / CONFIRMED。
- Reviewer finding：四个均复核成立：
  - DUP-001 `isBrowserVisible`（useBrowserStore.ts:149-151，仅 browser）vs `isBrowserView()`（useLayoutStore.ts:188-190，browser+grid）——两套不同集合，确认。
  - DUP-002 宫格可见拆为 `gridOpen`(:20)+`mainView==='grid'`(:138)+`gridToolbarOpen`(:144)，无单一真源——确认。
  - DUP-006 无"隐藏但不销毁"原语，closeGridAll 同时翻 `gridOpen=false` 且 kill 子进程——确认（bridge.rs:3915-3944）。
  - DUP-009 `activateWeb`（UnifiedTabBar.vue:147-154 用 `mainView!=="browser"`）vs `isActiveWeb`（:156-158 用 `isBrowserView()`）——确认，且 :149-151 注释已写明陷阱。
- Evidence re-found：上述行号均已核实。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-S4-07 (09 S4=4：SMF-001/002/003/004)：四个小模型失效模式。**
- Agent verdict：S4 / CONFIRMED（重验证）。
- Reviewer finding：四个失效模式对应的**源码陷阱均存在**：
  - SMF-001 activateWeb↔isBrowserView 陷阱：UnifiedTabBar.vue:147-158 确认。
  - SMF-002 create_grid 内存预算返回与 FE 重赋值：buildGrid `created = await bridge.createGrid(n,urls)`（useBrowserStore.ts:314）、`if(degraded) gridCount.value=created`（:317），且 Rust `create_grid` 返回 `Ok(n)` 预算值（bridge.rs:3852-3908）——确认契约与降级重赋值均存在。
  - SMF-003 宫格"最小化"会 orphan 450MB 子进程：closeGridAll 单点销毁（bridge.rs:3915-3944）+ 无独立 hide 原语——确认。
  - SMF-004 删 closeGridAll 的 mainView 复位→浏览器空白：closeGridAll:491-504 注释与代码确认（mainView 复位是 B9-4 修复关键）。
- Verdict：CONFIRMED · Confidence：HIGH

### 2.2 生命周期红线（05）全量核验

**CLAIM-LC-01 (05 CASE-002)：VIEW SWITCH == DESTROY？否。**
- Reviewer finding：CONFIRMED。`syncViewVisibility`（:646-654）对非浏览器/宫格视图仅 `hideAllWebviews()`（屏外移），不触发 `close_grid`/`close_tab`。
- Evidence re-found：`useBrowserStore.ts:646-654`；`bridge.rs:635-685`（hide_all_webviews）。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-LC-02 (05)：HIDE == CLOSE？否。**
- Reviewer finding：CONFIRMED。`hide_bounds`（bridge.rs:570-597）仅 `update_rect(-30000)` 屏外移，显式注释"不能用 `set_visible(false)`"（:567-569，WebKitGTK 死锁）；`HideWindow`（main.rs:426-431）仅 `win.hide()`，不调用 `close_tab`/`kill_child`。
- Evidence re-found：`bridge.rs:570-597`；`main.rs:426-431`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-LC-03 (05)：CLOSE == KILL？页签不适用；宫格 是（架构性耦合）。**
- Reviewer finding：CONFIRMED。`close_grid`（bridge.rs:3915-3944）顺序 HideWindow→CloseTab→`shutdown_all()`（:3936 杀全部子进程）；`close_tab`（:794-823）销毁 webview 并 `tabs.remove`（:817）。每格=一个 OS 子进程，关闭即 kill。
- Evidence re-found：`bridge.rs:3915-3944`；`bridge.rs:794-823`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-LC-04 (05 CASE-005)：POSITION == SHOW？是（机制性耦合）。**
- Reviewer finding：CONFIRMED。`apply_bounds_inner` 在 `update_rect` 后 `set_visible(true)`（bridge.rs:555）；`GridCmd::UpdateRect` 末行 `win.show()`（main.rs:423）。无独立 show 命令。
- Evidence re-found：`bridge.rs:531-564`（:555）；`main.rs:391-424`（:423）。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-LC-05 (05 / SEM-014)：HIBERNATE = CLOSE ≠ DESTROY？是。**
- Reviewer finding：CONFIRMED。`hibernate_tab`（bridge.rs:4559-4574）调用 `manager.close_tab`（销毁 webview）但**不**从 `state.tabs` 移除，反而 `hibernated_tabs.insert`（:4570）；`tab_activate`（:4663-4683）对 `hibernated_tabs` 命中的 id `remove` 后 `spawn_child_window` 按 URL 重建（:4680）。即"关 webview 但留条目+URL"。
- Evidence re-found：`bridge.rs:4559-4574`；`bridge.rs:4663-4683`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-LC-06 (05 EVID-GR-02)：create_grid 首行即 close_grid，创建隐式销毁旧宫格。**
- Reviewer finding：CONFIRMED。`create_grid`（bridge.rs:3854）`close_grid(app.clone())?` 幂等重建。
- Evidence re-found：`bridge.rs:3851-3909`（:3854）。
- Verdict：CONFIRMED · Confidence：HIGH

### 2.3 IPC 契约（06）BROKEN / DRIFT_RISK 全量核验

**CLAIM-IPC-01 (06 CASE-006)：create_grid 契约一致（FE/Rust 均携带 n, urls）。**
- Agent verdict：CONSISTENT。
- Reviewer finding：CONFIRMED。FE `createGrid(n, urls)`（bridge.ts:534-535）→`invoke("create_grid",{n,urls})`；Rust `create_grid(app,n:usize,urls:Vec<String>)`（bridge.rs:3851）；注册于 main.rs:1440，ACL 放行 default-commands.toml:42。无半改契约。
- Evidence re-found：`bridge.ts:534-535`；`bridge.rs:3851`；`main.rs:1440`；`default-commands.toml:42`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-IPC-02 (06 CASE-005)：grid_position/UpdateRect 一致（含 position+size+show）。**
- Agent verdict：CONSISTENT。
- Reviewer finding：CONFIRMED（契约签名一致）。但需注意：该命令**实际携带 show 副作用**（main.rs:423 `win.show()`），这是 06（纯 layout IPC 建模）与 07（隐藏副作用）冲突的根源，见 CONFLICT-01。
- Evidence re-found：`bridge.ts:542-545`；`bridge.rs:4022-4054`；`main.rs:391-424`（:423）。
- Verdict：CONFIRMED（契约签名一致）· Confidence：HIGH

**CLAIM-IPC-03 (06 CASE-009)：sync_browser_scene 未实现，渲染场景散落在 ≥2 模块（DRIFT_RISK）。**
- Agent verdict：DRIFT_RISK。
- Reviewer finding：CONFIRMED。全仓 `grep sync_browser_scene|syncScene|BrowserRuntime` 命中 0（`src` 下 0 结果）；场景同步逻辑分散在 `useBrowserStore`（syncViewVisibility/relocate/syncFreeze 等）与 `useBrowserHost`（schedulePosition/scheduleGrid/onMounted 直发 `hideAllWebviews` :172）。治理文档要求的单一适配器并未落地。
- Evidence re-found：`src` 下 0 命中；`useBrowserStore.ts:646-664`；`useBrowserHost.ts:59-103,171-172`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-IPC-04 (06 EVID-014/015/027)：5 个未落地 agent/skill 命令（skill_list/agent_list/skill_install/agent_install/skill_run）无 Rust 实现，且不在 checker KNOWN 白名单、被正则盲区漏过。**
- Agent verdict：DRIFT_RISK（unguarded）。
- Reviewer finding：CONFIRMED。`src-tauri` 全仓搜索 `fn skill_list|fn agent_list|fn skill_install|fn agent_install|fn skill_run` 命中 0，且 main.rs 未注册这些命令；bridge.ts:401-438 以**带类型** `invoke<...>` 封装；checker `KNOWN["main_invoke_not_registered"]` 仅含 4 个无类型的占位（agent_chat/agent_chat_cancel/confirm_agent_install/confirm_skill_install），`invoked()` 正则 `invoke\(\s*["\']` 只匹配无类型调用——5 个带类型命令对门禁不可见。
- Evidence re-found：`bridge.ts:401-438`；`check-command-set-consistency.py:42-51,82-83`；`src-tauri` 命令搜索 0 命中。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-IPC-05 (06 EVID-013/027)：check-command-set-consistency.py 正则盲区。**
- Agent verdict：FACT（盲点）。
- Reviewer finding：CONFIRMED。正则 `re.findall(r'invoke\(\s*["\']([^"\']+)["\']', ...)`（:83）要求 `(` 与 `"` 间无类型参数；`invoke<number>("create_grid",...)` 被跳过。该盲区使 5 个未落地命令 + 所有带类型 invoke 逃过门禁。
- Evidence re-found：`check-command-set-consistency.py:82-83`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-IPC-06 (06 EVID-028)：5 命令因 `AGENT_SKILL_COMMANDS_AVAILABLE=false` 不运行时触发，为休眠占位。**
- Reviewer finding：CONFIRMED。`bridge.ts:102` `export const AGENT_SKILL_COMMANDS_AVAILABLE = false`，封装注释（:398）说明仅在该标志为 true 时调用。
- Evidence re-found：`bridge.ts:102,398`。
- Verdict：CONFIRMED · Confidence：HIGH

### 2.4 MULTIPLE_SOURCE_OF_TRUTH（02）核验

**CLAIM-MT-01 (02 / SEM-015)：aiNavOpen 双定义，仅 browser.aiNavOpen 被消费，layout.aiNavOpen 为死重复真源。**
- Agent verdict：MULTIPLE_SOURCES_OF_TRUTH（死代码）。
- Reviewer finding：CONFIRMED。`useLayoutStore.ts:146` 定义并在 :338 返回 `aiNavOpen`，但全仓消费点均为 `browser.aiNavOpen`：AINavPanel.vue:10,18；ActivityBar.vue:338,341；useWorkbenchStore.ts:27,29,32。`layout.aiNavOpen` 确无任何读取点（死代码）。
- Evidence re-found：`useLayoutStore.ts:146,338`；`useBrowserStore.ts:43,538,680`；`AINavPanel.vue:10,18`；`ActivityBar.vue:338,341`；`useWorkbenchStore.ts:27,29,32`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-MT-02 (02 / 03 R3 / DUP-004 / SMF-007)：isBrowserVisible 公式漂移——源码 computed 已简化为仅 `mainView==="browser"`，但 closeGridAll 注释(:491) 与 check-grid-close-logic.mjs G2(:112) 仍断言 `!gridOpen && mainView==="browser"`。**
- Agent verdict：STALE_DERIVED_STATE / DRIFT。
- Reviewer finding：CONFIRMED（三向漂移）。live computed `useBrowserStore.ts:149-151` = `layout.mainView === "browser"`（无 !gridOpen）；同文件 :491 注释写 `"isBrowserVisible = !gridOpen && mainView==='browser'"`（过时）；checker `check-grid-close-logic.mjs:106-114` 的 G2（:112）断言公式含 `!gridOpen.value`。任一方"修复"都会引入耦合回归。
- Evidence re-found：`useBrowserStore.ts:149-151,491`；`check-grid-close-logic.mjs:106-114`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-MT-03 (02 / 08 DUP-002)："宫格是否可见"由 3 个独立标志编码（gridOpen + mainView==='grid' + gridToolbarOpen），无单一真源。**
- Agent verdict：S4 / 多源。
- Reviewer finding：CONFIRMED。`gridOpen`（useBrowserStore.ts:20）、`mainView`（useLayoutStore.ts:138）、`gridToolbarOpen`（:144）三者独立；"可见"=`gridOpen && mainView==='grid'` 仅 inline 组合（closeGridAll:502-504、scheduleGrid:99、syncFreeze:625 各取所需），非派生常量。
- Evidence re-found：`useBrowserStore.ts:20,319,486`；`useLayoutStore.ts:138,144`；`useBrowserStore.ts:99,502-504,625`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-MT-04 (02 SEM-006)：ActivityBar 用 panelOpen 高亮收藏夹，MainArea 用 bmPanelOpen(=panelOpen&&mainView==='browser') 挂载，非 browser 视图下"高亮但不渲染"。**
- Agent verdict：DERIVED_STATE_REIMPLEMENTED / S3。
- Reviewer finding：CONFIRMED。MainArea.vue:103 `bmPanelOpen = panelOpen && mainView==='browser'`，:150 `<BookmarkPanel v-if="bmPanelOpen">`；ActivityBar.vue:328 `:class="{active: bookmarks.panelOpen}"` 直接高亮。在 files/term 等视图点收藏夹时高亮 active 但面板不挂载。
- Evidence re-found：`MainArea.vue:103,150`；`ActivityBar.vue:328,144`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-MT-05 (02 SEM-010)：currentLocalPath 为 DERIVED_STATE_REIMPLEMENTED，inlineFile>previewDir>modTab.path>filePath 四级 fallback。**
- Agent verdict：S2 / DERIVED_STATE_REIMPLEMENTED。
- Reviewer finding：CONFIRMED。useWorkspaceStore.ts:579-586 四级 fallback 聚合 4 个可能矛盾的底层源。
- Evidence re-found：`useWorkspaceStore.ts:579-586`。
- Verdict：CONFIRMED · Confidence：HIGH

### 2.5 隐藏的销毁/杀进程副作用（07）核验

**CLAIM-SE-01 (07 EVID-005-TAB/GRID)：position 蕴含 show（隐藏副作用）。** — 见 CLAIM-S4-05 / CLAIM-LC-04，CONFIRMED。

**CLAIM-SE-02 (07 EVID-ACTIVATE-007)：tab_activate 对休眠页签静默重建 webview。**
- Agent verdict：FACT / S3。
- Reviewer finding：CONFIRMED。`tab_activate`（bridge.rs:4663-4683）先 `hibernated_tabs.remove`（:4670），若命中则 `spawn_child_window`（:4680）按 URL 重建 1×1 隐藏 webview。前端 `activeTabId` 对此无感知。
- Evidence re-found：`bridge.rs:4663-4683`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-SE-03 (07 EVID-BYPASS-007)：tabNew 用 `tabPosition(-30000)` 直接隐藏新 webview，绕过 hide_webview。**
- Agent verdict：FACT / S3。
- Reviewer finding：CONFIRMED。useBrowserStore.ts:170 `bridge.tabPosition(t.id,{x:-30000,y:0,width:100,height:100})` 直发屏外（黑闪修复），注释明确"绕过 50ms 去重"。
- Evidence re-found：`useBrowserStore.ts:166-170`。
- Verdict：CONFIRMED · Confidence：HIGH

**CLAIM-SE-04 (07 EVID-HIDE-007 / CASE-007)：webview 隐藏有多处重叠出口（hide_webview / hide_all_webviews / tab_activate 内部 hide_bounds 循环 / 直发 tabPosition(-30000)）。**
- Agent verdict：INFERENCE / S3。
- Reviewer finding：CONFIRMED（多重出口属实）。`hide_webview`（bridge.rs:608）、`hide_all_webviews`（:635）、`tab_activate` 内部 `hide_bounds` 循环（:4710-4713）、`tabNew` 直发 `tabPosition(-30000)`（useBrowserStore.ts:170）并存。
- Evidence re-found：`bridge.rs:608,635,4710-4713`；`useBrowserStore.ts:170`。
- Verdict：CONFIRMED · Confidence：MEDIUM（出口列举完整度依赖更多调用点扫描，但核心四类已验证）

**CLAIM-SE-05 (07 EVID-CLOSE-META)：close_tab 除销毁 webview 外还大范围清除元数据。**
- Agent verdict：FACT / S2。
- Reviewer finding：CONFIRMED。bridge.rs:804-817 逐一 remove 并 `tabs.remove`（:817）。
- Evidence re-found：`bridge.rs:794-823`。
- Verdict：CONFIRMED · Confidence：HIGH

### 2.6 跨检查器冲突（SMF-005 / 08 末节）核验

**CLAIM-XC-01：check-ui.mjs 与 check-session-persistence-policy.py 仍将 SessionCloseDialog 标为 [CURRENT]，而 check-native-webview-overlay.mjs 明确禁止其存在（09 SMF-005 / 08 末节）。**
- Agent verdict：CONFIRMED 内部冲突。
- Reviewer finding：CONFIRMED（直接矛盾）。
  - check-ui.mjs:37,53,196,222-243,409：SessionCloseDialog 标 `[CURRENT]` 关闭协议，断言其必须保留。
  - check-session-persistence-policy.py:21,230,573-574：检查 App 仍挂载 SessionCloseDialog 为"正常"。
  - check-native-webview-overlay.mjs:6,25-26,33：`assert.doesNotMatch(app, /SessionCloseDialog/)` 且 webviewsSuspended 死开关已删（2026-09-12 撤销，useBrowserStore.ts:185-187 注释印证）。
  - 两方一禁一令，CI 自相矛盾；信任 check-ui.mjs 的模型会复活已撤销组件。
- Evidence re-found：`check-ui.mjs:37,53,196,222-243,409`；`check-session-persistence-policy.py:21,230,573-574`；`check-native-webview-overlay.mjs:6,25-26,33`；`useBrowserStore.ts:185-187`。
- Verdict：CONFIRMED · Confidence：HIGH

---

## 3. 冲突裁决（Conflict Resolution）

**CONFLICT-01 — 06 vs 07：grid_position / UpdateRect 究竟是"纯布局 IPC"还是"携带 show 副作用"？**
- 06 把 `grid_position` 建模为纯布局契约（CASE-005 标 CONSISTENT，未列 show 为问题）；07 证明它经 `UpdateRect` 末端 `win.show()`（main.rs:423）实际 show。
- 裁决：二者都对各自口径成立——**契约签名一致（06 正确）**，但**命令确有非声明的可见性副作用（07 正确）**。这是 IPC 契约文档未声明隐藏语义的真实缺陷。建议：在 06 契约表补充"副作用列"，将 show 显式纳入 CASE-005。
- 剩余不确定性：无（两端源码均已核实）。

**CONFLICT-02 — 03(B) vs 05(C)/07(E)：是否存在独立"show webview"动作？POSITION != SHOW 是否成立？**
- 03 动作表隐含"显示"经 `isBrowserVisible`(CSS) 门控，标 `POSITION != SHOW ✓`（03:63）；05 与 07 证明显示熔接在 position/UpdateRect 内，无独立 show 命令（07 S4-005-SHOW-007）。
- 裁决：05/07 更准确。前端**没有**独立 show IPC；所谓"显示"由 position 命令隐式完成。**POSITION != SHOW 作为"代码层面有正交 show"的断言，应判为不成立**；03 的 ✓ 仅指"CSS visibility 与 native bounds 是两个独立层"，并非"存在独立 show 命令"。建议 03 修正措辞。
- 剩余不确定性：03 是否真枚举了独立 show action（03 表未列独立 show 行，其 R1 也未提；冲突为口径而非事实）。

**CONFLICT-03 — 03/04 vs 05：grid→browser 分叉到底是 S4 还是 S4=0？**
- 03/04 把"同一意图两种生命周期结果"标 S4；05 把"VIEW SWITCH != DESTROY 红线未被违反"标 S4=0。
- 裁决：事实无矛盾——**视图切换本身确实不销毁**（05 正确）；**但"离开宫格"这一用户意图因入口不同而有时销毁有时仅隐藏**（03/04 正确，见 CLAIM-S4-02）。真正的治理缺口是"离开宫格"意图无单一意图动作（05 自己 §8 也指出需 `exitGrid(mode)`）。建议统一 S 评级口径：S4 专指"红线违反/生产静默错误"，架构分叉风险改标 S3 并在冲突矩阵单列。
- 剩余不确定性：closeGridAll 是否所有"离开宫格"入口都可达——已确认仅 close 按钮/工具条关/Home/末格关触发销毁，导航入口仅隐藏。

**CONFLICT-04 — 02 vs 08：isBrowserVisible vs isBrowserView 应 S3 还是 S4？**
- 02 把该语义混淆标 S3（SEM-003）；08 把同名问题标 S4（DUP-001）。
- 裁决：风险本质相同（命名相近导致误用，已有 :149-151 注释踩坑点）。建议统一为 **S3**（已踩坑并有注释防御，非"无防御的静默错误"），08 的 S4 偏高。
- 剩余不确定性：无。

**CONFLICT-05 — 03 vs 07：CLOSE==KILL 对宫格是"违规(S3)"还是"架构性耦合(INFERENCE)"？**
- 03:62 把 `close_grid`/`close_tab` 混淆 CLOSE+DESTROY 标 S3（视为问题）；05/07 视为每格=子进程的固有架构约束（非 bug）。
- 裁决：二者一致——**事实是 close==kill==destroy，且为架构耦合**。05/07 的定性更准确；03 的 S3 仅强调"应抽取独立 hide 原语"。建议保留 S3 但注明"架构耦合，需补 hide 原语"。
- 剩余不确定性：无。

---

## 4. 无法核实的声明（INSUFFICIENT_EVIDENCE）

- **SEM-011 / FEAT-009 凭据安全域**：未深读 `CredentialList.vue` 与 Rust keyring 注册路径，仅据 01/02 声明判断为合理；需补 `CredentialList.vue:20-89` 与 `keyring_store.rs` 抽样。
- **FEAT-006 收藏夹 vs 主页 favorite 双语义**：未读 `useHomeStore.favoriteDirectory/favoriteCurrentPage` 与 localStorage 路径；声明未核实。
- **FEAT-008 终端 resize 去重位置**：未读 `TerminalPane.vue:24-28` 注释与 `useTerminalResize.ts` 全文；"后端直通/前端去重"归属未独立核验。
- **SMF-006 FilePanel 破坏性操作无语义检查器**：未扫描 `FilePanel.vue` 是否直调 `bridge.writeFile/deletePath`；声明合理但未经源码确认。
- **EVID-WS-01 workspace 切换对页签/宫格生命周期影响**：05 自标 UNVERIFIED（Rust 无 `switchWorkspace` 命令）；复核未额外深挖，维持 UNVERIFIED。
- **SMF-002 内存爆炸量级（12×450MB≈5.4GB）**：预算降级守卫未逐项核算，量级为推断；核心契约与降级重赋值已核实。
- **FEAT-011 recentlyClosed vs recents 命名碰撞**：仅据声明，未抽样两符号的全部读写点确认"无交叉误用"。

---

## 5. 总体评估（Overall Assessment）

**审计证据可信度：高（HIGH），可进入 Phase 0 / Phase 1 语义收敛。**

理由：
1. 所有被要求核验的**事实性核心断言（S4 清单、05 全部生命周期红线、06 全部 IPC 契约与 DRIFT_RISK、02 全部多源真源、07 全部隐藏副作用、SMF-005 跨检查器冲突）均经独立重采样确认**，未发现与源码相矛盾的事实性误报。各 Agent 的"FACT"标注基本可靠。
2. 唯一实质性的"冲突"均为**评级口径/措辞差异**（CONFLICT-01/02/03/04/05），而非事实对立；建议统一 S 评级语义后再发布，但**不影响收敛路线图的启动**。
3. 最严重的、可直接指导 Phase 0 落地的真实缺口已坐实：
   - 宫格"离开"意图无单一入口（销毁/隐藏分叉，CLAIM-S4-02）；
   - 宫格可见无单一派生选择器（DUP-002 / gridOpen+mainView+gridToolbarOpen 三源）；
   - position 命令携带未声明的 show 副作用（跨 IPC/生命周期失配）；
   - 三个检查器互相矛盾（SessionCloseDialog 一禁一令）；
   - IPC 门禁正则盲区使 5 个未落地命令零保护（CLAIM-IPC-04/05）。

**进入 Phase 0/1 前建议优先修复（不阻塞，但应并行）：**
- 修 `check-command-set-consistency.py` 正则（捕获 `invoke<Type>`）+ 将 5 命令纳入 KNOWN 或落地 Rust；
- 统一 SessionCloseDialog 三方检查器口径（以 check-native-webview-overlay 的"已撤销"为准，清除 check-ui.mjs / check-session-persistence-policy.py 的 [CURRENT] 引用）；
- 在 06 契约表补"隐藏副作用"列（show/visibility）；
- 统一 S 严重度定义（红线违反=生产静默错误 才计 S4），消除 02/05/07/08/09 之间的评级漂移。

**结论：证据充分、事实可靠，建议 proceed to Phase 0/Phase 1 语义收敛。**

> **第一轮裁定已被第二轮推翻（重要更正）**：本文件（`16-REVIEW-REPORT.md`，第一轮）的 CONFLICT-01~05 与据此在 `00` 做的评级/回写，已被**第二轮对抗性终审（`17A–17F` + `18-SECOND-REVIEW-SYNTHESIS.md`）推翻/修正至少 8 处事实级错误**（见 `18` §8）：`BROKEN=0`、`05 S4=0`、三方检查器矛盾、`ADR-GRID-001` L18 覆盖声明等。最终权威以 `18` 为准——全局 `S4=2 / S3=8 / S2=6 / S1=3`，FINAL_S4 = GRID_EXIT_DIVERGENCE + NATIVE_VISIBILITY_HIDDEN_STATE。本文件保留为第一轮记录，**不得再作为评级/红线结论的引用源**；所有回写以 `18` + 更新后的 `00`/`05`/`06`/`14` 为准。
