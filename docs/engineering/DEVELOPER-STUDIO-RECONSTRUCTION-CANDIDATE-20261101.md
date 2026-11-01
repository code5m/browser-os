# BrowserOS Developer Studio and shared editor: integration candidate

Base: feature/perf-monaco-worker-internalization-20261031 at 363a6257307b84d23a91acdce7dfd89f8014bdbe.
Architecture review: research/architecture-reconstruction-audit-20261101.
Status: integration candidate only; this is NOT the entire requested reconstruction.

## Real changes in this candidate
- Shared lazy Monaco loader, language mapping, resource/service ownership and document model creation.
- CodingCodeEditor preserves Rust LSP providers and tool/AI behavior, now imports shared loader/model.
- Workspace inline and overlay editors now share a Monaco text surface while retaining Workspace's save owner.
- Existing import paths retained as compatibility facades; no new Rust commands or Tauri permissions.
- Independent document identity/version regression check.

## Not implemented or proved
- SQL editor still has its specialized 64 KiB parser/completion/query workflow; it must be migrated only after feature parity checks.
- No full Developer Studio composed view; current Shell Contribution Registry remains unchanged.
- Browser and module tabs share a visual strip today but not yet one canonical tab-state owner.
- Grid UDS backlog, blocking request timing, pending drain, RSS/p95 instrumentation require native instrumentation and installed-GUI evidence; existing locked GTK/WebView geometry behavior must remain untouched until reproduction.
- Browser/Database/Git/Coding agents do NOT yet share a common production Rust runtime.
- Capability async lifecycle, Hot-Plug rollback, permission and destroy deficiencies in research audit remain.
- No build, CI or installed Ubuntu acceptance was run against this tree at authorship.

## Next safe integration gates
1. node tests/check-shared-editor-contract.mjs
2. node tests/check-coding-script-model-sync.mjs
3. npm run check:coding-workbench and npm run check:developer-studio
4. npm run build and bash scripts/pre-merge.sh
5. Browser-based Monaco TS/JS, Workspace inline save/restore and Markdown preview tests.
6. Installed GUI cold start, WebView/IPC runtime sampling, then performance before/after comparison.

Do not merge without completing those checks and reviewing the existing locked native behavior.
