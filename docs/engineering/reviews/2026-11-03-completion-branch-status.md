# 2026-11-03 BrowserOS completion branch — verified scope and open gaps

Baseline: `master` commit `d65ae69360c94974fbe4224e779ea3727f5dfe2f`. Branch: `fix/completion-p0-p1-20261103`.

This is an **incomplete** batch. Do not interpret it as completion of all user requirements, a release, or installed GUI acceptance. The local untracked directories `需要解决的问题/` and `.bevel/` are explicitly excluded from commits.

## Implemented

- Workspace overlay editor now uses `editorFilePath` distinct from directory navigation `filePath`; quick-create uses the explicit active editor file parent.
- Opening another overlay document or entering a directory checks for unsaved overlay edits and does not discard when refused.
- A document read request revision and snapshot check prevents stale I/O from overwriting a newer document or keystrokes.
- Overlay saves compare the disk's current text with the last loaded/saved baseline and refuse an observed external conflict, preserving the draft. Save errors preserve the draft. **This is non-atomic; a different process can modify the file after the read and before the write.**
- Overlay close button and TopBar return-to-browser action prompt on unsaved edits. A browser `beforeunload` guard covers main and inline dirty editors when the store has been instantiated.
- Added main-editor guard regression and migrated quick-create regression to the new document identity contract.

## Not completed / not certified

| Request | Evidence-backed status and next gate |
|---|---|
| Cross-file F12 navigation | The current master test exists; browser automation depends on an available Playwright runtime. No claim of actual installed WebView acceptance. |
| WeChat public-account draft upload | Not implemented. Needs official endpoint/permission verification, secure account credential integration, error/retry model and a real authorized draft+image check. Never publish or mass-send automatically. |
| Ctrl+N all focus contexts | Existing frontend shortcut logic test is separate from terminal/native WebView/GTK actual focus testing. |
| Toolbar/dock collapse | Existing persisted folding tests do not prove actual native WebView rectangle or desktop keyboard behavior. |
| File data protection P0 | **PARTIAL**: overlay guard + advisory conflict check. Native cross-process atomic compare-and-swap, durable draft journal, crash recovery, every module navigation path, symlink/race policy remain **NOT DONE**. Don't call it CAS. |
| Database | Existing SQLite/i64 and MySQL mapping/unit checks are not full MySQL real roundtrip across extremes, binary lossless export, PostgreSQL implementation or installed GUI acceptance. |
| Agent / capability lifecycle | Multi-file write recovery journal, durable sessions, terminal cleanup, cancel races and real service failure injection still need implementation/evidence. |
| Native Grid/Browser/Terminal/Git | No native GTK/WebView changes made. Main-thread isolation, resource ownership, installed performance measurements and close-path process resource checks remain open. |
| Final acceptance | No current-SHA installed .deb GUI acceptance, real WeChat draft acceptance or full real-service matrix established by this branch. |

## Required final acceptance

Run frontend quick and full checks plus Rust gates after implementation has converged; then produce source-SHA matched MySQL service runs, deb package inspection, installed GUI behavior and resource before/after evidence. Never call `CODE_PASS`, `PACKAGE_PASS`, or `GUI_PASS` without the corresponding completed gate; `GUI_PASS` requires explicit user real desktop acceptance under `AGENTS.md`.
