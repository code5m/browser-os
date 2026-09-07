# M5-A11 · W17 Verification Matrix (assist note)

**Lane**: A11 · START VERIFICATION（board L1195）
**Verdict**: ⛔ **BLOCKED** — 2 hard blockers; do NOT push.

## Green (20+ gates)
| Gate | Result |
|---|---|
| cargo test (default) | **433 passed / 0 failed**; warnings 2 = baseline |
| cargo test --features mcp | **454 passed / 0 failed** (default 433 → +21, matches W15 "MCP feature 21/21") |
| Cargo feature | `[features]` L22, `mcp = []` L23 (default excludes) |
| cargo fmt --check | 0 diff |
| npm run build | PASS — ✓ built in 5.13s; `index-D521uF3k.js` 169.45 kB (gzip 60.78) |
| measure --self-test | ALL_PASS |
| core / mcp / agent-memory / agent-skill / graph / plugin / tools / database / scheduler / script-exec | ALL PASS (mcp ACTIVE=13 PENDING=0 + `--expect-current-gaps` PASS; graph ACTIVE=8; plugin ALL_PASS + `--expect-pending` OK) |
| **A4 W17 new** `check-home-client-policy.py` | default PASS ("all invariants hold", ACTIVE=3) + self-test PASS (ACTIVE=3/PENDING=3) |
| UI logic | a11y **PASS**, graph **113-0**, agent-skill **110-0** |
| `git diff --check` | worktree+index **EXIT=0**; `merge-base..HEAD` **EXIT=0** |

## BLOCKER-1 · Build metrics over budget (W17 AC5)
- Measured: `total_bytes_pct=25.55`, `exceeds_growth_limit=true`, `cargo_warnings` delta=0, `warnings_increased=false`.
- Limit: `TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.2` (`scripts/measure-build-metrics.py:39`; raised to 25.2 by A0 ONLY for the W15 accepted keyboard-accessibility fix).
- **Exact delta: +0.35pp over 25.2 ceiling; +0.41pp vs W15 accepted 25.14%.** Main chunk 169.45 kB (W10: 164.82 kB, +4.63 kB).
- Per AC5: **stopped and reported the delta; ceiling NOT raised, guard NOT bypassed.** Fix = shrink main chunk (lazy-load W17 home/nav logic & panels).
- pre-merge L81 `FAIL: build metrics regression`.

## BLOCKER-2 · `check-scheduler-ui-policy.py` SCHEDUI_PANEL_LAZY — **stale fixture, not a real regression**
- Policy (L178-182) demands exact string `defineAsyncComponent(() => import("../workspace/TaskPanel.vue"))`.
- `MainArea.vue` L40-41 actually uses the **object form** `defineAsyncComponent({ loader: () => import("../workspace/TaskPanel.vue"), ... })` → **lazy loading is CORRECT**; only the fixture anchor string is obsolete (self-test confirms: "变异锚点在 MainArea.vue 中不存在（夹具已过时）"). Same object form used for Database/SkillManager/AgentManager/Graph/PluginManager panels (L50/57/64/73/82).
- `MainArea.vue` last changed by `886ea29` (W14 integration, committed) → **NOT attributable to A7 W17 WIP** (file shows no local modification).
- Fix owner: the M4-8 scheduler-UI fixture owner — accept both arrow-shorthand and `{ loader: ... }` forms; update mutation samples.

## Not yet delivered (⏸ N/A, not fabricated)
- `scripts/check-home-store-logic.mjs` (A3) — missing
- `scripts/check-home-ui-logic.mjs` (A9) — missing
- `scripts/check-client-navigation-logic.mjs` (A6) — missing
→ W17 **AC1** (desktop start, no localhost:1421 refused) and **AC3** (navigation/narrow-window discoverability) cannot be statically closed by A11 yet; needs A2/A6/A9 delivery + A8 real-client QA.

## Point-in-time caveat
Shared worktree is being mutated concurrently by other lanes (`useHomeStore.ts` went from unlisted to ` M` between two `git status` runs; `homeUi.ts` untracked-new and imported by `useHomeStore.ts`). All readings are a **16:52 snapshot**; metrics may drift.

## Push readiness
⛔ **NOT READY.** Either blocker alone fails `pre-merge.sh` (L81 / L110-111) → A0 must not push. Fix BLOCKER-1 (shrink main chunk) and BLOCKER-2 (update fixture), then re-run `pre-merge.sh` + this matrix.

Full detail: `logs/checkpoints/A11-M5-W17-verification-20260907-1652.md`.
Patch: `logs/checkpoints/Lane-A11-M5-W17-20260907-1652.patch`. No product code changed by A11; not committed; not pushed.
