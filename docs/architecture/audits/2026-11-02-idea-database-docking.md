# BrowserOS Database Studio: IDEA interaction audit and implementation

Date: 2026-11-02. Base: master at 520bfe08. Branch: feat/idea-database-workbench-20261102.

## Primary official documentation

- Arrange tool windows: https://www.jetbrains.com/help/idea/manipulating-the-tool-windows.html
- Tool windows: https://www.jetbrains.com/help/idea/tool-windows.html
- Database tool window: https://www.jetbrains.com/help/idea/database-tool-window.html
- Data Sources and Drivers: https://www.jetbrains.com/help/idea/data-sources-and-drivers-dialog.html
- Data editor and viewer: https://www.jetbrains.com/help/idea/data-editor-and-viewer.html

17 user-supplied IDEA screenshots show a SQL editor above Output, Database tree docked right, Services and Output below, draggable splitters, and connection configuration tabs General, Options, SSH/SSL, Schemas, Advanced and Kubernetes. Data-result controls include extractors, table/tree/text/transpose, column settings, transaction mode, and tool-window options. The screenshots can contain real environment metadata: **do not commit original screenshots to Git**.

## Audit of previous BrowserOS code

- DatabasePanel.vue: fixed 280px / content / 292px grid, independent toggles without draggable sizes, source-tree hierarchy was not integrated, and the connection dialog was nested inside the foldable navigator.
- DatabaseWorkspace.vue: SQL editor, results and history were a single vertical document; there was no independent Output splitter, maximize or alternative presentation; export was CSV/TSV/JSON.
- TableDataEditor.vue: guarded real table read/change/preview/commit was already present. Its user-visible rows were only a single table view.
- The Native database Owner already handles connection, SQL, metadata, transactional read/write controls and security gating. None of those gates should be replaced or bypassed for visual fidelity.

## Changes and safety

| Area | Implemented | Safety invariant |
| --- | --- | --- |
| Navigator / Inspector | Left/right navigator dock, independent folds, draggable width splitter, keyboard arrow resizing, reset | Only UI geometry/fold prefs saved; event listeners cleaned on stop/unmount |
| Data-source tree | Real saved datasource -> database -> Tables/Views -> actual objects, active-source lazy read | Other sources never silently queried |
| Source configuration | Independent resize-capable dialog with saved source list and General, Options, SSH/SSL, Schemas, Advanced, IDEA import | Credentials remain transient and keyring-backed; missing features explicitly described as unsupported |
| SQL Output | Standalone bottom dock with drag-to-resize, fold, maximize, batch-result tabs | Uses native query result only |
| Result viewer | Table/Tree/Text/Transpose, local filter/sort, column visibility, read-only cell inspector | No implicit SQL write |
| Data extractors | CSV/TSV/JSON/Markdown and SQL INSERT text in the existing typed Table Data Editor only | Only fetched rows; text is not executed; SQL UPDATE not faked |
| Table editor | Adds safe alternative read-only views, preserves guarded existing editable table mode | Existing transaction preview / confirmation / policy still required |

## Explicitly unsupported (no fake controls)

The global Services and browser-shell window dock, native detached multi-monitor windows, arbitrary JDBC driver properties, SSH tunnel, custom TLS client certs, Kubernetes forwarding, all cross-schema introspection patterns, SQL UPDATE extractors and Groovy script extractors are outside this UI pass. Future Native Owners and policies are required. Native real-effects, cold-start and resource usage cannot be marked passed by Vue tests.

## Source files

- src/capabilities/database/model/ideaDock.mjs: pure UI geometry, persistence and safe Markdown text extraction.
- src/capabilities/database/ui/DatabasePanel.vue: responsive tool-window geometry, hierarchical tree, dialog.
- src/capabilities/database/ui/DatabaseOutput.vue: independent results dock and presentation.
- src/capabilities/database/ui/DatabaseWorkspace.vue: hosts Output, keeps query Owner unchanged.
- src/capabilities/database/ui/TableDataEditor.vue: read-only Tree/Text/Transpose and Markdown export.
- tests/check-database-idea-dock.mjs: geometry, fail-closed storage, Markdown escaping and Vue SFC compilation.
- package.json: check:database-idea-dock runs in general npm check.

## Manual installed GUI acceptance (owner: user)

1. On startup show the right Database resource tree. Collapse/expand navigator and Inspector separately.
2. Drag navigator divider and Inspector divider, use arrow keys on focused splitter, switch navigator left/right by title drag and command; reload and reset.
3. Fold navigator and open Data Sources settings; it must be visible independently and list real saved connections. Exercise General, Options, SSL, Schemas, Advanced and IDEA import, without exposing a password.
4. Expand datasource -> database -> Tables/Views, select and double-click a real object; unrelated connection objects must not be queried.
5. Execute read-only SQL and inspect Output. Drag divider, collapse, maximize/restore, switch result tabs.
6. Change Table/Tree/Text/Transpose, toggle visible columns, use local filter/sort, inspect selected cell; export and verify only fetched data are exported.
7. Query Output has no ambiguous SQL INSERT target; use the existing Table Data Editor SQL INSERT exporter for real typed table data. Exported SQL must never auto-execute.
8. In authorized nonproduction table editing, keep an uncommitted draft while switching views and return to Table; no extra writes should happen.
9. Test focus, view cleanup, narrow window, per-session isolation and cancellation.
10. Run installed GUI PID/FD/RSS/PSS scenarios and physical Webview denial tests independently after merge.

**Status: only tests actually performed should be claimed as PASS. Installed GUI acceptance remains pending until the user runs it.**
