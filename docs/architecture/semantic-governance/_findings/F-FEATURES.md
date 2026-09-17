# F-FEATURES.md (Agent F — Cross-Feature Semantic Governance Audit)

Project: `mvp-browser-os-v3` (Tauri2 + Vue3). Principle audited: **ONE STATE TRUTH, ONE INTENT ENTRYPOINT**. Scope: BROWSER / GRID / BOOKMARK / WORKSPACE / TERMINAL / CREDENTIALS.

> Note: audit was read-only. Findings record symbol + file + line range; classification separates FACT vs INFERENCE vs RECOMMENDATION vs UNVERIFIED.

---

## FEAT-001 — "is a browser view" has two divergent predicates (`isBrowserVisible` vs `isBrowserView`)
Module: BROWSER / LAYOUT. Risk: S2.
Representations: `useBrowserStore.isBrowserVisible` (browser-only, :149-151) vs `useLayoutStore.isBrowserView()` (browser+grid, :188-190).
Evidence: EVID-0001, EVID-0002, EVID-0003.

## FEAT-002 — Grid "open/visible" state tracked in ≥4 representations that can drift
Module: GRID ↔ BROWSER ↔ LAYOUT ↔ Rust backend. Risk: S2.
Representations: `gridOpen` (FE) + `mainView==="grid"` + `gridToolbarOpen` + Rust `hidden`/`blur_hidden`/`last_rect`; no FE mirror of `blur_hidden`.
Evidence: EVID-0004, EVID-0005, EVID-0006, EVID-0007, EVID-0008, EVID-0026.

## FEAT-003 — Workspace "current/active path" has ≥4 competing representations
Module: WORKSPACE / FILEPANEL. Risk: S2.
Representations: `filePath` (overloaded dir/file), `previewDir`, `inlineFile`, derived `currentLocalPath` (4-way tie-break :579-586), split editor-open flags (`layout.fileEditorOpen` + `editingFile`/`mdPreview`).
Evidence: EVID-0009, EVID-0010, EVID-0011, EVID-0012.

## FEAT-004 — Webview position/rect dedup computed in two layers
Module: BROWSER ↔ Rust backend. Risk: S3.
FE dedupe (`useBrowserHost.ts` KeyDeduper/gridCache/hiddenIntent) vs BE dedupe (`apply_bounds` 50ms/rect-equal drop, bridge.rs:510-524). Both intentional; two caches can disagree.
Evidence: EVID-0013, EVID-0014.

## FEAT-005 — Credential exact-origin match computed in three layers
Module: CREDENTIALS ↔ BROWSER. Risk: S2.
Rust `credential_origin` → DTO `origin`; FE `pageOrigin` recompute+match; in-page `location.origin` check. FE uses `activeTab?.url || url` while Rust used stored credential URL.
Evidence: EVID-0019, EVID-0020, EVID-0021.

## FEAT-006 — "favorite" (HomeStore) vs "bookmark" (useBookmarkStore) — overlapping 收藏 semantics, two stores, two backends
Module: BOOKMARK / HOME / WORKSPACE. Risk: S3.
`useHomeStore.favoriteDirectory`/`favoriteCurrentPage` (localStorage shortcuts) vs `useBookmarkStore` bookmarks (backend JSON). Security-domain + persistence differ.
Evidence: EVID-0022.

## FEAT-007 — Two unrelated "grid" features collide in naming
Module: BROWSER (AI compare-grid) ↔ TERMINAL (multi-pane grid). Risk: S3.
`gridOpen`/`gridCount`/`gridLayout`/`gridMode` (browser) vs `termGrid`/`termGridCount` (terminal).
Evidence: EVID-0018.

## FEAT-008 — Terminal resize dedup: component comment attributes it to backend, but it lives in FE only
Module: TERMINAL. Risk: S3.
`TerminalPane.vue:24-28` comment claims backend `term_resize` does dedup+140ms+500ms; actual backend is pass-through (bridge.rs:4801-4813). FE `useTerminalResize` owns it.
Evidence: EVID-0015, EVID-0016, EVID-0017.

## FEAT-009 — Credentials nested inside BookmarkPanel despite separate security domain
Module: CREDENTIALS ↔ BOOKMARK. Risk: S3.
`BookmarkPanel.vue` renders "账号" tab mounting `CredentialList.vue`; keyring-backed creds vs benign bookmarks.
Evidence: EVID-0023.

## FEAT-010 — `gridToolbarOpen` (layout) and `gridOpen` (browser) are two coupled flags
Module: GRID ↔ LAYOUT. Risk: S3.
Two flags encode near-identical state across two stores; neither derived from the other.
Evidence: EVID-0004, EVID-0008.

## FEAT-011 — `recentlyClosed` (tabs) vs `recents` (opened urls/files) naming collision
Module: BROWSER ↔ WORKSPACE. Risk: S4.
`recentlyClosed` (closed tabs, in-memory, Ctrl+Shift+T) vs `recents` (recently opened url/file, localStorage). Opposite lifecycle events, same word "recent".
Evidence: EVID-0024.

## FEAT-012 — URL normalization duplicated/divergent across bookmark vs credential
Module: BOOKMARK ↔ CREDENTIALS. Risk: S3.
Bookmark `normalizeUrl` (scheme+host lower, strip trailing slash, keep search) vs credential `normalize_url`/`credential_origin` (port-strip, scheme/host only).
Evidence: EVID-0025.

---

## Summary
- Findings (FEAT): 12 — FEAT-001 … FEAT-012.
- Risk counts: **S2 = 4** (FEAT-001, FEAT-002, FEAT-003, FEAT-005); **S3 = 7** (FEAT-004, FEAT-006, FEAT-007, FEAT-008, FEAT-009, FEAT-010, FEAT-012); **S4 = 1** (FEAT-011).
- Strongest duplicate/ambiguous (feed 08): FEAT-002 (grid visible ≥4 reps), FEAT-003 (active path 4 reps), FEAT-005 (origin match 3 layers), FEAT-001 (isBrowserVisible vs isBrowserView), FEAT-006/012 (favorite vs bookmark + URL normalize divergence).
