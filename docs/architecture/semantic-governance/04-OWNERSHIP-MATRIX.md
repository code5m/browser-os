# 04-OWNERSHIP-MATRIX.md

> Semantic governance audit — OWNERSHIP MATRIX (ONE LIFECYCLE OWNER). Agent B (READ-ONLY).
> Classifications: `CLEAR_OWNER` / `MULTIPLE_OWNER` / `UI_ORCHESTRATION_LEAK` / `STORE_OWNERSHIP_LEAK` / `NATIVE_POLICY_LEAK`.

## 1. Ownership Matrix (domains → who owns)

| Domain | Classification | Owner (file:symbol) | Supporting Evidence |
|---|---|---|---|
| mainView (which surface is shown) | **CLEAR_OWNER** | `useLayoutStore` (`mainView` ref `:138`, `setView:193`, `isBrowserView:189`) | EVID-022 |
| Browser tab lifecycle (new/switch/close) | **CLEAR_OWNER** (store) + 1 native caller | `useBrowserStore` (`tabNew:158`, `tabSwitch:175`, `tabClose:188`, `closeTabNow:193`); native `close_browser` on `App.vue:218`, Rust `close_browser:465` | EVID-023, EVID-016 |
| Grid lifecycle (open/rebuild/close-one/close-all) | **MULTIPLE_OWNER** + **UI_ORCHESTRATION_LEAK** | Action body in `useBrowserStore` (`buildGrid:300`, `closeGridAll:483`, `closeGridOne:513`); triggered from `useLayoutStore.toggleGridToolbar:210`, `ActivityBar.onItem:167/setGridCount:134/setGridLayout:123`, `HomeLaunchers.openArea:47`, `App.vue:199`, `UnifiedTabBar:170`, `useWorkbenchStore:22` | EVID-005, EVID-006, EVID-024 |
| BrowserHost visibility (HTML host show/hide) | **CLEAR_OWNER** (derived) | Derived from `isBrowserVisible` (`useBrowserStore:149`) consumed by `BrowserHost.vue:15` | EVID-012, EVID-018 |
| Native WebView bounds (tab/grid positioning) | **CLEAR_OWNER** (composable) | `useBrowserHost` (`schedulePosition:59`→`tabPosition:91`; `scheduleGrid:97`→`gridPosition:141`); invoked via `useBrowserStore.relocate:560` | EVID-019, EVID-025 |
| Navigation (navigate/reload/back/forward) | **CLEAR_OWNER** (store) | `useBrowserStore` (`tabNavigate:238`, `tabReload:230`, `goBack:248`, `goForward:252`, `reloadActive:256`); `grid_open` for cells | EVID-026 |
| Session restore | **CLEAR_OWNER** | `useSessionStore.restoreSession:114` → `bridge.sessionRestore`; sole UI `SessionPanel.vue:102` | EVID-027 |
| Workspace lifecycle (dir tree/files) | **CLEAR_OWNER** | `useWorkspaceStore` (`enterDir`, `loadTree`, `refresh`, `startDirs`) | EVID-028 |
| Terminal lifecycle (spawn/kill/resize) | **MULTIPLE_OWNER** | `useSystemStore` (`spawnTerm`, `termKill:259`, `openTerminalAt:325`, `toggleTerminal:311`); ALSO `TerminalPane.restart/closePane` call `system.killTerm` directly (:215-221) | EVID-020 |
| Credential lifecycle (browser creds/keyring) | **STORE_OWNERSHIP_LEAK** + **NATIVE_POLICY_LEAK** | No Pinia store; `CredentialList.vue` calls `bridge.listBrowserCredentials:44` / `fillBrowserCredential:79` directly; `importBrowserCredentials` `src/bridge.ts:478`; storage in Rust keyring | EVID-021, EVID-030 |

## 2. Action Table (required schema)

| Action | User Intent | Entrypoints | Owner | State Mutation | Native Effects | Duplicate | Risk |
|---|---|---|---|---|---|---|---|
| setView/openModule | 切主视图 | `useLayoutStore.setView:193`, `openModule:253`, `openDirTab:268` | `useLayoutStore` | `mainView`, `modTabs`, `activeModTab` | watcher→`syncViewVisibility:646` (hide/relocate) | many callers | S2 |
| buildGrid | 开宫格 | `useBrowserStore.buildGrid:300` + 6 UI triggers | `useBrowserStore` (UI-leaked) | `gridSession`, `gridOpen`, `gridCount`, `mainView` | `create_grid`, `gridPosition`, `gridSetZoom`, `syncFreeze` | `closeGridOne` rebuild | S3 |
| closeGridAll | 关宫格 | `useBrowserStore.closeGridAll:483` + `toggleGridToolbar:214`, `HomeLaunchers:47`, `ActivityBar:398`, `closeGridOne:522` | `useBrowserStore` (triggered by layout+components) | `gridOpen=false`, `gridToolbarOpen=false`, `gridRects` clear, `mainView→browser`, reactivate tab | `close_grid` DESTROY, `tabActivate` show, relocate, syncFreeze | `closeGridOne` | S3 |
| grid→browser switch | 离开宫格 | destroy path: `closeGridAll:502-504`; hide path: `ActivityBar.onItem:172/176`/`setView`→watcher | SPLIT (store vs layout+watcher) | destroy: `gridOpen`+`mainView`; hide: only `mainView` | destroy: `close_grid`+`tabActivate`; hide: `hideAllWebviews` | two outcomes | **S4** |
| toggleGridToolbar | 收起宫格条 | `useLayoutStore.toggleGridToolbar:210` | layout store→delegates | `gridToolbarOpen`→`gridOpen`/`mainView` | OFF→`closeGridAll` DESTROY; ON→`buildGrid` CREATE | — | S3 |
| openBrowser/tabNew | 开网址 | `useBrowserStore.openBrowser:542`/`tabNew:158` + multiple UI | `useBrowserStore` | `url`,`tabs`,`activeTabId`,`mainView` | `tab_new` spawn webview, `tabActivate`, reposition | many funnel into one | S2 |
| tabClose | 关页签 | `useBrowserStore.tabClose:188`→`closeTabNow:193`; `App.vue:181` | `useBrowserStore` | `tabs`,`recentlyClosed`,`activeTabId` | `tab_close` DESTROY webview | — | S2 |
| tabSwitch | 切页签 | `useBrowserStore.tabSwitch:175`; `App.vue:186/191` | `useBrowserStore` | `activeTabId` | `tabActivate` show/hide, relocate | — | S2 |
| syncViewVisibility (view-switch) | 隐藏/显示webview | `useBrowserStore.syncViewVisibility:646` (watch:658); `useBrowserHost.schedulePosition:59` | browser store + `useBrowserHost` | (none on `gridOpen`) | `hideAllWebviews` (HIDE), `tabPosition`, `gridPosition` | — | S2/S3 |
| openTerminal/spawnTerm | 开终端 | `useSystemStore.spawnTerm`/`openTerminalAt:325`/`toggleTerminal:311`; `App.vue:138/192`; `FilePanel`→`ws.ctxOpenInTerminal:395` | `useSystemStore` (component also kills) | `terminalOpen`,`termPanes`,`termWriters` | `termSpawnChannel`/`termWrite`/`termKill` | `TerminalPane` direct kill | S2 |
| restoreSession | 恢复会话 | `useSessionStore.restoreSession:114`; `SessionPanel.vue:102` | `useSessionStore` | `tabs`/`activeTabId` (via tab_new) | `sessionRestore`→`tab_new` | — | S2 |
| credential fill/import | 凭据填充/导入 | `CredentialList.vue:44/79`; `bridge.importBrowserCredentials:478` | NONE (direct bridge) | none in FE | `list/fill/import_browser_credentials` (keyring) | multiple direct callers | S3 |

## 3. Target Direction (RECOMMENDATION — not current state)
The intended architecture is **UI → intent API → single owner → native adapter**. Current gaps:
- Grid lifecycle violates it: UI components directly set `gridMode`/`gridLayout`/`gridCount` then call `buildGrid`, and choose DESTROY vs HIDE implicitly. RECOMMENDATION: collapse to one `grid` intent owner that decides lifecycle; UI passes only intent.
- Credential lifecycle has no store owner; UI talks to bridge directly. RECOMMENDATION: add a `useCredentialStore`/intent facade; keep keyring as native adapter.
- Terminal lifecycle split between `useSystemStore` and `TerminalPane.vue`. RECOMMENDATION: all spawn/kill through `useSystemStore`; component only renders.
- `closeGridAll` mixes state flip + native destroy + view switch + reactivate + relocate + freeze in one store action. RECOMMENDATION: split into "intent → owner decides → native adapter executes".

## 4. Counts & Conflicts
- **S4:** 1 (grid→browser switch divergence). **S3:** 4 (buildGrid, closeGridAll, toggleGridToolbar, credentials-no-owner). **S2:** 6.
- **Classification tally:** CLEAR_OWNER: 7 (mainView, tab lifecycle, BrowserHost visibility, native WebView bounds, navigation, session restore, workspace); MULTIPLE_OWNER: 2 (grid lifecycle, terminal); UI_ORCHESTRATION_LEAK: 1 (grid); STORE_OWNERSHIP_LEAK: 1 (grid orchestration / credentials); NATIVE_POLICY_LEAK: 1 (credentials).
- **Cross-artefact drift:** `scripts/check-grid-close-logic.mjs` G2 (:112) and in-code comment `useBrowserStore.ts:490-494` encode stale `isBrowserVisible = !gridOpen && mainView==="browser"`, but the live computed (`:149-151`) is `mainView==="browser"` only. The watchdog contract and source have diverged. This audit did not modify the checker, `src/`, `src-tauri/`, `Cargo.toml`, `package.json`, or `AGENTS.md`.
