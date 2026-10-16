# PR35 regression closure

Source baseline: PR35 head 38b579e36b348a1080e4641e521f5ea01fec8c83. All current tracked files, including public/, and locked npm dependencies were exported read-only from GitHub. The source archive contains GitHub's synthetic PR merge commit; the four original patch files matched their recorded Git blob hashes exactly.

## Changes

GitStudio now invalidates repository-bound targets, previews and results synchronously on source changes. Same-source refresh retains user input; the history entry remains a history entry. The legacy GitPanel checker validates the real wrapper and executes the new SFC regression instead of assuming retired component internals.

The table add-on adds native SQLite and MySQL regressions for literal bound data, full-batch rollback following a late duplicate key, approval consumption, and release of row locks checked by a separate MySQL FOR UPDATE NOWAIT transaction. Existing transaction-control fixes remain unchanged; added tests are not production data operations.

The pinned build-only lightningcss declaration is reconciled with all three frozen policy maps. Negative coverage preserves rejection of unreviewed additions, deletion, version changes and runtime moves. No footprint limit, baseline, ACL or database production protection is relaxed.

## Isolated agent-container results

- Complete npm run check: PASS (Node 22.16 with experimental type stripping enabled to match CI Node 22 behavior).
- Git source isolation: 40 assertions PASS, actual Vue SFC setup/watch with native-IO fixtures.
- Existing Git UI logic check: PASS.
- Three dependency policy families: live checks and existing positive/negative self-tests PASS.
- Additional pinned-dependency contract: 3 tests PASS, each applicable negative case checked across all three families.
- Existing model/workbench/PR35 regressions: 19/58/47 assertions PASS.
- Frontend production build: PASS.

These results are not native Rust execution, real-MySQL execution, Tauri GUI evidence, or final CI. Added Rust tests must compile and pass in final CI. The user performs local installation/use testing separately; it is not a prerequisite for Agent-controlled merge. Original full-scope functionality remains required before declaring the PR complete.
