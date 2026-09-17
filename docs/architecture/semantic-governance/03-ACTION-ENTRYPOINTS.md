# 03-ACTION-ENTRYPOINTS.md

> Agent B · READ-ONLY 语义治理审计（ONE INTENT ENTRYPOINT / ONE LIFECYCLE OWNER / ONE SIDE-EFFECT EXIT）
> 项目：`mvp-browser-os-v3`（Tauri2 + Vue3）
> 断言纪律：FACT / INFERENCE / RECOMMENDATION / UNVERIFIED 分离；设计建议不伪装成现状。

---

## 1. 方法
扫描 `src/components/**`, `src/stores/**`, `src/composables/**`, `src/bridge.ts` 中的动作动词：
`open* close* create* destroy* show* hide* activate* deactivate* switch* toggle* sync* handle* refresh* relocate* layout* position*`。
每个动作回答：用户意图、入口、owner、状态变更、原生副作用、重复入口、风险。

## 2. 动作清单（Action → User Intent → Entrypoints → Owner → State Mutation → Native Effects → Duplicate → Risk）

| Action | User Intent | Entrypoints | Owner | State Mutation | Native Effects | Duplicate | Risk |
|---|---|---|---|---|---|---|---|
| setView/openModule | 切主视图 | `useLayoutStore.setView:193`, `openModule:253`, `openDirTab:268`, `closeModTab:300` | `useLayoutStore` | `mainView`, `modTabs`, `activeModTab` | watcher→`syncViewVisibility:646` (hide/relocate) | many | S2 |
| View-switch webview sync | 隐藏/显示原生webview | `useBrowserStore.syncViewVisibility:646` (watch :658); `useBrowserHost.schedulePosition:59`/`scheduleGrid:97` | browser store + `useBrowserHost` | (no `gridOpen` change) | `bridge.hideAllWebviews` (HIDE), `tabPosition`, `gridPosition` | — | S2/S3 |
| Open terminal | 开终端 | `useSystemStore.spawnTerm`/`openTerminalAt:325`/`toggleTerminal:311`; `App.vue:138/192`, `FilePanel.ctxOpenInTerminal`→`ws.ctxOpenInTerminal:395`→`system.openTerminalAt` | `useSystemStore` (also `TerminalPane.restart/closePane`) | `terminalOpen`, `termPanes`, `termWriters` | `bridge.termSpawnChannel`/`termWrite`/`termKill` | component also kills | S2 |
| Restore session | 恢复会话 | `useSessionStore.restoreSession:114`; `SessionPanel.vue:102` | `useSessionStore` | `tabs`/`activeTabId` (via tab_new) | `bridge.sessionRestore`→`tab_new` | — | S2 |
| Credential fill/import | 填/导入凭据 | `CredentialList.vue:44/79`; `bridge.importBrowserCredentials:478` | NO store (direct bridge) | none in FE | `list/fill/import_browser_credentials` (keyring, Rust) | multiple direct callers | S3 |
| buildGrid | 开宫格 | `useBrowserStore.buildGrid:300` + 6 UI triggers (`ActivityBar.onItem:167`, `setGridCount:134`, `setGridLayout:123`, `toggleGridToolbar`, `HomeLaunchers.openArea:47`, `App.vue:199`, `useWorkbenchStore:22`, `UnifiedTabBar:170`) | `useBrowserStore` (UI-leaked) | `gridSession`, `gridOpen`, `gridCount`, `mainView` | `create_grid` (CREATE/DESTROY+CREATE), `gridPosition`, `gridSetZoom`, `syncFreeze` | `closeGridOne` rebuild | S3 |
| closeGridAll | 关宫格 | `useBrowserStore.closeGridAll:483` + `toggleGridToolbar:214`, `HomeLaunchers:47`, `ActivityBar:398`, `closeGridOne:522` | `useBrowserStore` (triggered by layout+components) | `gridOpen=false`, `gridToolbarOpen=false`, `gridRects` clear, `mainView→browser`, reactivate tab | `close_grid` DESTROY, `tabActivate` show, relocate, syncFreeze | `closeGridOne` | S3 |
| grid→browser switch | 离开宫格 | destroy path: `closeGridAll:502-504`; hide path: `ActivityBar.onItem:172/176`/`setView`→watcher | SPLIT (store vs layout+watcher) | destroy: `gridOpen`+`mainView`; hide: only `mainView` | destroy: `close_grid`+`tabActivate`; hide: `hideAllWebviews` | two outcomes | **S4** |
| toggleGridToolbar | 收起宫格条 | `useLayoutStore.toggleGridToolbar:210` | layout store→delegates | `gridToolbarOpen`→`gridOpen`/`mainView` | OFF→`closeGridAll` DESTROY; ON→`buildGrid` CREATE | — | S3 |
| openBrowser/tabNew | 开网址 | `useBrowserStore.openBrowser:542`/`tabNew:158` + multiple UI | `useBrowserStore` | `url`,`tabs`,`activeTabId`,`mainView` | `tab_new` spawn webview, `tabActivate`, reposition | many funnel into one | S2 |
| tabClose | 关页签 | `useBrowserStore.tabClose:188`→`closeTabNow:193`; `App.vue:181` | `useBrowserStore` | `tabs`,`recentlyClosed`,`activeTabId` | `tab_close` DESTROY webview | — | S2 |
| tabSwitch | 切页签 | `useBrowserStore.tabSwitch:175`; `App.vue:186/191` | `useBrowserStore` | `activeTabId` | `tabActivate` show/hide, relocate | — | S2 |
| openFile/openFileInline | 打开文件 | `useWorkspaceStore.openFile:892`, `openFileInline:707` | `useWorkspaceStore` | `filePath`/`inlineFile`/`previewDir`/`fileEditorOpen` | none | divergent entrypoints | S2 |

---

## 3. MUST-CHECK CASE verification

### CASE-004 — `closeGridAll`: hide / close / destroy / kill?
- **Answer:** `closeGridAll` performs a true **DESTROY** of the grid's native child processes, plus UI view-switch back to browser and reactivation of the active browser tab.
- **FE caller:** `useBrowserStore.closeGridAll:483`. Triggered by `toggleGridToolbar:214`, `closeGridOne:522`, `HomeLaunchers.openArea:47`, `ActivityBar.vue:398`.
- **Rust command:** `bridge.closeGrid()` → `invoke("close_grid")` → `close_grid` `src-tauri/src/bridge.rs:3915`. Body: `HideWindow` → `CloseTab` → `mgr.shutdown_all()` → KILLS grid child processes.
- **Lifecycle transition:** `gridOpen:true→false` + main-view `grid→browser` + active tab `tabActivate` + native **destroy** of all grid child windows. NOT a mere hide.

### CASE-002 — Grid → Browser switch: which action, does it destroy the grid?
- **Destroying path:** `closeGridAll` (when `mainView==="grid"` resets to `"browser"` and calls `bridge.closeGrid()` → DESTROY). Reachable via close-grid button, toolbar toggle OFF, Home launcher, or final single-cell close.
- **Non-destroying path:** Clicking any non-grid navigation in `ActivityBar.onItem` (`setView("browser") :172` or `openModule`) only changes `mainView`; the watcher runs `syncViewVisibility` → `bridge.hideAllWebviews()` (offscreen HIDE). `gridOpen` stays `true`; grid preserved.
- **Does it destroy the grid?** **Depends on the entrypoint.** Same user intent ("leave the grid") → DESTROY via closeGridAll, but only HIDE via ActivityBar navigation. This divergence is the core S4 risk.

### CASE-008 — Does a simple user action require a Component to hand-combine setView+hide+activate+relocate+layout?
- **YES (HIGH risk).** Evidence:
  - `HomeLaunchers.openArea` (:46-54) hand-combines `closeGridAll()` then `layout.setView("browser")` / `openModule`.
  - `ActivityBar.onItem` (:159-168) hand-combines `browser.gridMode="ai"` + `layout.openModule("grid")` + `buildGrid()`/`layoutGrid()`.
  - `ActivityBar.setGridCount` (:127-135) hand-combines `browser.gridCount=n` + `browser.gridLayout` change + `buildGrid()`.
  - Inside the store, `closeGridAll` itself hand-combines 6+ steps. ⇒ STORE_OWNERSHIP_LEAK + UI_ORCHESTRATION_LEAK.

### grid hide vs closeGridAll confusion
- Two native grid exits with different semantics: (a) **HIDE** — `syncViewVisibility`→`hide_all_webviews` (no `gridOpen` change, grid survives); (b) **DESTROY** — `closeGridAll`→`close_grid` (kills child procs, `gridOpen=false`).

---

## 4. Caveat conformance notes
- VIEW SWITCH != DESTROY: confirmed — plain `setView` only HIDES offscreen. ✓
- HIDE != CLOSE: `hide_all_webviews`/HideWindow ≠ `close_grid`/`close_tab`. ✓
- CLOSE != DESTROY: in this codebase `close_grid`/`close_tab` conflate CLOSE+DESTROY (no separate "close-but-keep" for grid). Flagged as S3.
- POSITION != SHOW: `schedulePosition`/`tabPosition` only issue bounds; `isBrowserVisible` (CSS) gates actual display. ✓.

## 5. Recommendations (RECOMMENDATION — not current state)
- R1: Introduce one intent action `exitGrid(mode: "hide"|"destroy")` so UI never chooses the lifecycle outcome implicitly; route `toggleGridToolbar` and both navigation paths through it.
- R2: Move the `closeGridAll` hand-combined sequence into a single native adapter / store "grid lifecycle owner" so components only call intent, not `setView`+`buildGrid` combos.
- R3: Reconcile `isBrowserVisible` formula drift: either restore `!gridOpen` in the computed or update the store comment + `check-grid-close-logic.mjs` G2.

## 6. Counts & Conflicts
- **S4:** 1 (Grid→Browser switch divergence). **S3:** 4 (buildGrid, closeGridAll, toggleGridToolbar, credentials-no-owner). **S2:** 6.
- **Cross-artefact conflict:** `scripts/check-grid-close-logic.mjs` G2 and the in-code comment (`useBrowserStore.ts:490-494`) both still encode `isBrowserVisible = !gridOpen && mainView==="browser"`, but the live computed (`:149-151`) dropped `!gridOpen`. The watchdog and the source have drifted. This audit did NOT modify the checker.
