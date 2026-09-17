# 11 · Checker Design — 机器可强制的语义看门狗
> Agent H · READ-ONLY 审计 · 配套 `10-TARGET-SEMANTIC-CONTRACTS.md`
> 每条规则给出：RULE-ID / Semantic rule / Why / Scope / Detection strategy / False-positive risk / Checker owner / Architecture·UI·Native / Autofix allowed。
> 全部为“最小、低误报、可静态 grep/parse”的契约守护；不修改 src/、src-tauri/、scripts/、package.json、Cargo.toml、AGENTS.md（本文件仅描述设计，落地由后续任务执行）。

---

## 通用约定
- 正则统一以 `multiline` 思路匹配（行级 + 函数体括号匹配）。
- 所有“禁止裸赋值”规则都对 **外部访问 `layout.xxx =`** 生效，`xxx.value =`（store 内部）不命中，避免误伤 setView。
- Autofix 一律 NO：本批契约涉及语义，自动改写风险高。

---

## RULE-001 · gridOpen 仅由 buildGrid/closeGridAll 赋值
- **Semantic rule**: `gridOpen.value` 只能赋 true（buildGrid）与 false（closeGridAll）；任一文件/函数越界即违规。
- **Why**: SMF-003 / DUP-006——任何“只置 false 不 closeGrid”的 hide 会留孤儿子进程；唯一生命周期 owner 必须收敛。
- **Scope**: `src/**/*.ts`（尤其 useBrowserStore.ts:319 / :486）。
- **Detection strategy**:
  - 正则抓取所有赋值点：`gridOpen\.value\s*=\s*(true|false)`。
  - 断言：所有命中行必须位于 `buildGrid` 或 `closeGridAll` 函数体内（用括号匹配提取函数）。
  - 等价断言：`= true` 仅 1 处（buildGrid），`= false` 仅 1 处（closeGridAll）。
- **False-positive risk**: 低。当前仅 2 处，且已收敛。
- **Checker owner**: 扩展 `scripts/check-grid-close-logic.mjs`（新增 G4）或新建 `scripts/check-grid-lifecycle.mjs`。
- **Architecture / UI / Native**: Architecture。
- **Autofix allowed**: NO。

## RULE-002 · isBrowserVisible 公式不得含 gridOpen
- **Semantic rule**: `isBrowserVisible` computed 必须严格等于 `layout.mainView === "browser"`，禁止出现 `gridOpen` 标记。
- **Why**: SMF-004/007、DUP-004——重新引入 `!gridOpen` 会在“宫格仅 flag 未销毁”时误隐藏浏览器 webview，复现 B9-4 空白。
- **Scope**: `src/stores/useBrowserStore.ts:149-151`（computed 体）。
- **Detection strategy**:
  - 提取 `const isBrowserVisible = computed(() => ... )` 体（括号匹配至 `)`）。
  - 断言体 **不含** `gridOpen`，且 **包含** `layout.mainView === "browser"`。
  - 同时修 `scripts/check-grid-close-logic.mjs` G2（L112）：把断言从 `!gridOpen.value && layout.mainView === "browser"` 改为仅 `layout.mainView === "browser"`。
  - 另：扫描 closeGridAll 注释（L491-494）含 `!gridOpen && mainView==="browser"` 的旧式描述 → 标记 STALE_COMMENT 待人工改注释。
- **False-positive risk**: 低。函数体极短，正则精确。
- **Checker owner**: `scripts/check-grid-close-logic.mjs`（改 G2）。
- **Architecture / UI / Native**: UI。
- **Autofix allowed**: NO（注释需人工措辞）。

## RULE-003 · mainView 外部裸赋值一律禁止
- **Semantic rule**: 禁止 `layout.mainView = "..."` 外部裸写；视图切换只经 `setView` / `openModule` / `closeModTab`。
- **Why**: DUP-003 / SMF-010——裸赋值绕过 setView 的 fileEditorOpen/navSection 清理，留下 stale 覆盖层。
- **Scope**: `src/**/*.vue` + `src/**/*.ts`（匹配 `layout\.mainView\s*=\s*['"]`）。
- **Detection strategy**:
  - 正则 `layout\.mainView\s*=\s*['"]`。
  - 排除：位于 `setView`/`openModule`/`closeModTab`/`buildGrid` 函数体内的（buildGrid 的 `mainView='grid'` 受 `!== 'grid' && !== 'browser'` 守卫，列入白名单函数）。
  - 当前应报站点（需整改）：FileEditor.vue:11、TopBar.vue:21、useBrowserStore.ts:161/207/503/545、useSessionStore.ts:120、useWorkspaceStore.ts:912/924。
- **False-positive risk**: 中。buildGrid 内的 `mainView='grid'` 需白名单；新函数若合法写 mainView 需登记。建议默认报，靠 whitelist 收敛。
- **Checker owner**: 新建 `scripts/check-view-switch.mjs`。
- **Architecture / UI / Native**: UI。
- **Autofix allowed**: NO。

## RULE-004 · 组件不得手拼宫格生命周期
- **Semantic rule**: `*.vue` 组件不得直接调用 `buildGrid`/`closeGridAll`/`closeGridOne`，不得先写 `browser.gridMode/gridLayout/gridCount` 再调 `buildGrid`。
- **Why**: CASE-008 / DUP-006——组件编排生命周期会绕过单一 owner 裁决（hide vs destroy 错配，S4）。
- **Scope**: `src/components/**/*.vue`。
- **Detection strategy**:
  - 正则 A：`(?:buildGrid|closeGridAll|closeGridOne)\s*\(` 在 vue 中命中即报（经 store 意图调用的应改 `exitGrid`/`openModule('grid')`）。
  - 正则 B（同一函数体内顺序组合）：`browser\.grid(Count|Layout|Mode)\s*=[\s\S]{0,120}?buildGrid\s*\(` → 报。
- **False-positive risk**: 中。UnifiedTabBar.vue:170 的 `browser.buildGrid()`/`browser.layoutGrid()` 属当前合法但待收敛点，需白名单过渡。
- **Checker owner**: 新建 `scripts/check-grid-exit-intent.mjs`（或并入 check-grid-lifecycle.mjs）。
- **Architecture / UI / Native**: UI。
- **Autofix allowed**: NO。

## RULE-005 · IPC 三源闭包 + typed-invoke 覆盖
- **Semantic rule**: 每个 FE `invoke` 必须有匹配 Rust 命令且 ACL 放行；看门狗正则必须覆盖 typed invoke。
- **Why**: EVID-013/015/027——5 个 agent/skill typed 占位当前对 gate 不可见，零保护；create_grid 半改会让 FE/Rust 在 IPC 字符串边界失配（SMF-002）。
- **Scope**: `src/bridge.ts`、`src-tauri/injected/collect.js`、`src-tauri/src/main.rs`、`src-tauri/permissions/*.toml`。
- **Detection strategy**:
  - 修 `scripts/check-command-set-consistency.py:83`：`invoked()` 正则改为 `invoke(?:<[^>]*>)?\(\s*["\']([^"\']+)["\']`。
  - 把 `skill_list|agent_list|skill_install|agent_install|skill_run` 加入 `KNOWN["main_invoke_not_registered"]`（或落地 Rust 后端后移出 KNOWN）。
  - 保留既有 4 个 untyped 占位（agent_chat/agent_chat_cancel/confirm_skill_install/confirm_agent_install）于 KNOWN。
  - 额外断言：`bridge.createGrid(` 调用处必须同时传 `n` 与 `urls`（桥接 `createGrid:534-535`）；`buildGrid` 必须保留 `if (degraded) gridCount.value = created`（:317）。
- **False-positive risk**: 低。正则扩写仅增加覆盖，不改变既有通过集。
- **Checker owner**: `scripts/check-command-set-consistency.py`（改 `invoked()` + KNOWN）。
- **Architecture / UI / Native**: Native（IPC 边界）。
- **Autofix allowed**: NO。

## RULE-006 · 原生 webview 显隐/定位仅经授权适配
- **Semantic rule**: `bridge.tabPosition/gridPosition/hideWebview/hideAllWebviews/tabActivate` 只允许在 useBrowserStore.ts 与 useBrowserHost.ts 调用（单一 scene adapter，07 CASE-009）。
- **Why**: 07 CASE-009 / G3——scene-sync 逻辑已在 store+composable 重复；组件直连会绕过统一显隐时序，复现遮屏/空白。
- **Scope**: `src/**/*.vue`、`src/**/*.ts`（排除 useBrowserStore.ts、useBrowserHost.ts）。
- **Detection strategy**:
  - 正则：`bridge\.(tabPosition|gridPosition|hideWebview|hideAllWebviews|tabActivate)\s*\(`。
  - 命中且文件非上述两文件 → 报。
  - 当前预期：0 命中（tab 调用已在 useBrowserStore；webview hide/position 已在两文件）。
- **False-positive risk**: 低。当前已收敛。
- **Checker owner**: 新建 `scripts/check-native-scene-adapter.mjs`（或并入 check-native-webview-overlay.mjs 的守卫）。
- **Architecture / UI / Native**: Native。
- **Autofix allowed**: NO。

## RULE-007 · 禁止 position-as-hide hack
- **Semantic rule**: 不得用 `bridge.tabPosition(id, {x:-30000,...})` 之类“移屏外当 hide”的写法；隐藏统一走 `hideWebview`/`hideAllWebviews`。
- **Why**: EVID-BYPASS-007 / 07 CASE-007——位置当 hide 用会绕过无去重的 hide_bounds，且语义隐晦。
- **Scope**: `src/**/*.ts`（重点 useBrowserStore.ts:170）。
- **Detection strategy**:
  - 正则：`bridge\.tabPosition\(\s*\w+\s*,\s*\{[^}]*?-30000` 或 `\bx:\s*-30000\b` 出现在 position 调用上下文。
  - 当前唯一命中 useBrowserStore.ts:170（tabNew 的黑闪修复 hack）→ 标记待重构为 `bridge.hideWebview` 或授权 hide 帮助函数。
- **False-positive risk**: 低。-30000 为项目既定“屏外”常量，仅此一处用于 position。
- **Checker owner**: 新建 `scripts/check-native-scene-adapter.mjs`（与 RULE-006 同文件）。
- **Architecture / UI / Native**: Native。
- **Autofix allowed**: NO。

## RULE-008 · activateWeb 不得误用 isBrowserView()
- **Semantic rule**: `UnifiedTabBar.vue` 的 `activateWeb` 函数体不得引用 `isBrowserView()`；必须用 `layout.mainView === "browser"`（精确）。
- **Why**: DUP-009 / SMF-001 / CASE-001——`isBrowserView()` 含 grid，grid 视图下会让点击页签失效。其自身注释（L149-151）已写明陷阱。
- **Scope**: `src/components/layout/UnifiedTabBar.vue`（函数 `activateWeb`，L147-154）。
- **Detection strategy**:
  - 提取 `async function activateWeb` 体（括号匹配），断言其中 **不含** `isBrowserView`。
  - 反向断言（可选）：`isActiveWeb`（L156-158）**应当**使用 `isBrowserView()`（不得改成精确 mainView）。
- **False-positive risk**: 极低。函数名与谓词名稳定。
- **Checker owner**: 新建 `scripts/check-tabbar-predicate.mjs`（或并入 check-client-navigation-logic.mjs）。
- **Architecture / UI / Native**: UI。
- **Autofix allowed**: NO。

## RULE-009 · 页签命令不得裸调（store 外）
- **Semantic rule**: `bridge.tabClose/tabNew/tabOpen/tabActivate/tabReload` 只允许在 useBrowserStore.ts 调用；组件必须经 store action。
- **Why**: DUP-008 / SMF-008——裸调绕过 recentlyClosed 记录与 tabs[] 维护，破坏恢复栈。
- **Scope**: `src/**/*.vue`、`src/**/*.ts`（排除 useBrowserStore.ts；BookmarkPanel.vue:51 为注释，正则不匹配调用）。
- **Detection strategy**:
  - 正则：`bridge\.(tabClose|tabNew|tabOpen|tabActivate|tabReload)\s*\(`。
  - 命中且文件非 useBrowserStore.ts → 报。
  - 当前预期：0 命中（tab 调用已全在 store）。
- **False-positive risk**: 低。
- **Checker owner**: 新建 `scripts/check-tab-intent.mjs`（或并入 RULE-006 文件）。
- **Architecture / UI / Native**: UI。
- **Autofix allowed**: NO。

## RULE-010 · 破坏型文件系统命令仅限 useWorkspaceStore
- **Semantic rule**: `bridge.deletePath/writeFile/renamePath/movePath/createFile` 只允许在 useWorkspaceStore.ts 调用（`revealPath` 为只读展示，单列豁免）。
- **Why**: SMF-006——FilePanel 等若直连破坏型命令会成为全局数据丢失路径，且无语义看门狗。
- **Scope**: `src/**/*.vue`、`src/**/*.ts`（排除 useWorkspaceStore.ts）。
- **Detection strategy**:
  - 正则：`bridge\.(deletePath|writeFile|renamePath|movePath|createFile)\s*\(`。
  - 命中且文件非 useWorkspaceStore.ts → 报（revealPath 不在此列）。
  - 当前预期：0 命中（writeFile/deletePath/renamePath/movePath/createFile 已全在 store；UnifiedTabBar.vue:125 仅 revealPath，豁免）。
- **False-positive risk**: 低。
- **Checker owner**: 新建 `scripts/check-fs-intent.mjs`。
- **Architecture / UI / Native**: Native（FS 边界）。
- **Autofix allowed**: NO。

## RULE-011 · 凭据命令必经 facade，且前端不持密码
- **Semantic rule**: `bridge.fillBrowserCredential/listBrowserCredentials/importBrowserCredentials` 只可在凭据 facade（目标 `useCredentialStore`）调用；任何代码不得把 `password` 写入 Pinia ref / localStorage / 日志。
- **Why**: FEAT-005/009、04（STORE_OWNERSHIP_LEAK + NATIVE_POLICY_LEAK）——密码红线；当前 CredentialList.vue 直连 bridge，无 store _owner。
- **Scope**: `src/**/*.ts`、`src/**/*.vue`（排除未来的 useCredentialStore.ts 与 CredentialList.vue 内经 facade 的调用）。
- **Detection strategy**:
  - 正则 A：`bridge\.(fillBrowserCredential|listBrowserCredentials|importBrowserCredentials)\s*\(` → 命中且不在 facade 文件 → 报（过渡期允许 CredentialList.vue，但标注 TODO 收敛）。
  - 正则 B（红线）：`(ref|reactive|localStorage\.(set|get)Item|console\.(log|error|warn))\b[\s\S]{0,80}?password` → 命中即报（前端不得持有明文密码）。
- **False-positive risk**: 中。正则 B 可能命中“password 字段名”的安全日志过滤；建议 B 仅对 `password\s*[:=]` 赋值/存储型命中，且排除显式脱敏注释上下文。
- **Checker owner**: 新建 `scripts/check-credential-owner.mjs`。
- **Architecture / UI / Native**: Native（keyring）+ UI。
- **Autofix allowed**: NO。

## RULE-012 · layout.aiNavOpen 死重复真源须删除
- **Semantic rule**: 禁止访问 `layout.aiNavOpen`（死重复）；唯一消费的 `browser.aiNavOpen`（useBrowserStore.ts:43, :538 写，AINavPanel/ActivityBar/useWorkbenchStore 读）。
- **Why**: EVID-0018 / SEM-015——`useLayoutStore.aiNavOpen`（:146, 返回 :338）全仓无组件引用，是死重复真源，未来基于它做高亮/持久化会与 browser.aiNavOpen 分裂。
- **Scope**: `src/**/*.ts`、`src/**/*.vue`。
- **Detection strategy**:
  - 正则：`layout\.aiNavOpen` → 命中即报（含定义与返回，建议直接删除 useLayoutStore 内定义+返回）。
  - 反向断言（可选）：`browser\.aiNavOpen` 必须至少 1 处真实使用。
- **False-positive risk**: 极低。符号名唯一。
- **Checker owner**: 新建 `scripts/check-dead-state.mjs`（可扩展收纳其它死重复真源）。
- **Architecture / UI / Native**: UI。
- **Autofix allowed**: NO。

## RULE-013 · gridToolbarOpen 跨 store 写入收敛
- **Semantic rule**: `layout.gridToolbarOpen` 的写入只允许在 `toggleGridToolbar`（useLayoutStore.ts:210-215）；`closeGridAll` 跨 store 直写 `layout.gridToolbarOpen = false`（:488）需改为经 layout action。
- **Why**: FEAT-010 / EVID-0006 / SEM-005——两 writer 跨 store 协作，新增关格路径若忘清 gridToolbarOpen 会“工具条展开但宫格没了”。
- **Scope**: `src/**/*.ts`（匹配 `gridToolbarOpen\s*=\s*(true|false)` 或 `layout\.gridToolbarOpen\s*=`）。
- **Detection strategy**:
  - 抓取所有 `gridToolbarOpen` 写入点；断言仅位于 `toggleGridToolbar` 与（过渡期）`closeGridAll`，且最终收敛到单一 writer。
  - 当前命中：useLayoutStore.ts:211（toggleGridToolbar）、useBrowserStore.ts:488（closeGridAll 跨 store 写）。
- **False-positive risk**: 低。
- **Checker owner**: 并入 `scripts/check-grid-lifecycle.mjs`（RULE-001 同文件）。
- **Architecture / UI / Native**: UI。
- **Autofix allowed**: NO。

---

## 看门狗落点汇总（现有脚本改造 vs 新建）
| RULE | 落地脚本 | 改造/新建 |
|---|---|---|
| 001, 013 | check-grid-lifecycle.mjs | 新建（或并入 check-grid-close-logic.mjs） |
| 002 | check-grid-close-logic.mjs（改 G2 + 注释断言） | 改造 |
| 003 | check-view-switch.mjs | 新建 |
| 004 | check-grid-exit-intent.mjs | 新建 |
| 005 | check-command-set-consistency.py（改 invoked() + KNOWN） | 改造 |
| 006, 007 | check-native-scene-adapter.mjs | 新建（或并入 check-native-webview-overlay.mjs） |
| 008 | check-tabbar-predicate.mjs | 新建（或并入 check-client-navigation-logic.mjs） |
| 009 | check-tab-intent.mjs | 新建（或并入 006） |
| 010 | check-fs-intent.mjs | 新建 |
| 011 | check-credential-owner.mjs | 新建 |
| 012 | check-dead-state.mjs | 新建 |

## 当前门禁健康度（FACT，已对源码验证）
- 已满足（看守即可）：RULE-001（gridOpen 仅 2 处）、RULE-006（native 命令已收敛两文件）、RULE-009（tab 命令已收敛 store）、RULE-010（FS 命令已收敛 store）。
- 需修脚本（非源码）：RULE-002（checker G2 漂移）、RULE-005（regex 漏 typed invoke）。
- 需整改源码（命中即报）：RULE-003（9 处裸 mainView 赋值）、RULE-004（组件手拼宫格）、RULE-007（:170 -30000 hack）、RULE-011（CredentialList 直连 bridge，无 facade）、RULE-012（layout.aiNavOpen 死重复）、RULE-013（closeGridAll 跨 store 写 gridToolbarOpen）。
