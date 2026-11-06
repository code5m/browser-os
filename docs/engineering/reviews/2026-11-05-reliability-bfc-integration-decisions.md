# BrowserOS reliability and BFC integration decisions

- Date: 2026-11-05
- Base: `master@ab1e172f42599365bf4169bf274968869d46d427`
- Single integration branch: `integration/browseros-reliability-bfc-20261105`
- Scope: the requested P0/P1/P2 items plus WeChat full-workspace layout and Bilibili login/readback flow.
- Repository rule: GitHub `code5m/browser-os` is the only writable product source in this integration branch. `code5m/bilibili-favorites-workbench` remains a read-only integration dependency; no second branch or PR will be created.

## Decisions

1. Preserve `.bevel/` and `需要解决的问题/` exactly as found. They are user-owned, untracked material and are not part of this branch.
2. Preserve the approved editor/core/shared byte caps and production asset attribution. Reduce emitted Core/Shared bytes; do not relabel shared code as editor-only or suppress files. The obsolete cumulative M0 percentage is no longer a current CI verdict (2026-11-06 update).
3. Workspace recovery keeps the source `session_id` from each selected encrypted journal. After a successful CAS save, clear only that selected journal and the current process journal. Never clear journals belonging to other sessions.
4. WeChat work-in-progress state is stored in a dedicated encrypted native draft store whose key remains in the OS keyring. AppID/AppSecret never enter frontend persistence. An ambiguous `draft/add` outcome remains a durable no-blind-retry state.
5. WeChat and BFC are full-width workbench surfaces in the existing main work area; no forced window maximization, overlay, or z-index workaround.
6. BFC integration does not modify the separate BFC repository. Login detection inspects only the rendered official Bilibili page and must distinguish logged-in, logged-out, access-limited/challenge, and unknown. No cookie extraction, direct hidden account API, proxy rotation, challenge bypass, or retry loop.
7. BFC readback is explicit and user initiated. It reads only rendered favorite-page entries in the user's authenticated BrowserOS tab, validates BVID/title bounds, and submits a completed snapshot to the existing local BFC service at `127.0.0.1:4198` using its existing local session/CSRF contract. HTTP 412 or challenge/unknown states stop the scan and are never reported as logged out or synced.
8. Grid window geometry updates use a bounded latest-value queue per grid cell. GTK window-event callbacks only enqueue; background workers perform UDS writes. Final geometry must win after rapid move/resize and reconnection.
9. Shutdown is a state machine: close/exit events are prevented while one background cleanup run executes; main-thread-only WebView closure is marshalled back to the main thread; the process exits only after the cleanup report is available. Failures are reported, not represented as successful resource release.
10. Coding task identity is derived from the actual workspace root and actual Coding session, and is carried through task lifecycle, cancellation and approval/recovery metadata. A task is not considered a durable session store.
11. Workspace CAS anchors all filesystem operations to an already-open authorized parent directory FD, uses no-follow operations and same-directory atomic replacement, and documents that arbitrary non-cooperating external writers cannot be made fully transactional by advisory locks alone.
12. Native Owner migration is incremental. New BFC behavior belongs to a dedicated BFC Owner; existing command ACL/source checks and native inventory must be updated in the same change. No broad command rewrite or unrelated ABI change.

## Acceptance gates

- Focused regression tests for every changed owner and failure path.
- `npm run check`, one production build, exact Core/Shared/editor budget audit, and same-run master incremental comparison on the same source SHA.
- `cargo fmt --all --check`, `cargo check --locked`, `cargo test --locked`, plus focused Linux filesystem, Grid queue, shutdown and Coding identity tests.
- `bash scripts/pre-merge.sh` and GitHub Full Validation on the exact branch head.
- Package/GUI checks are reported separately. No GUI_PASS claim without the real installed-client desktop evidence required by `AGENTS.md` and `PROJECT-RULES.md`.
- Merge the single PR only after all blocking automated gates pass and no unresolved correctness/security blocker remains. Otherwise keep the branch and report BLOCKED/GUI_PENDING accurately.
