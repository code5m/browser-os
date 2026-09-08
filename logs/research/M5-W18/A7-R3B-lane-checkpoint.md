# A7 — M5-W18-R3B Lane Checkpoint

- **Lane:** A7 (Git reference map & 14-unit correction)
- **Wave:** M5-W18-R3B (`RESEARCH_AND_PROTOTYPE`, targeted correction)
- **Worktree:** `/home/ainfinit/.codex/worktrees/m5-w18-a7/mvp-browser-os-v3`
- **Branch:** `codex/m5-w18-a7` (rebased on `origin/master` `d96b9b5`)
- **Mode:** research only — **no product source / ACL / capability / dependency changes, no push**
- **Date:** 2026-09-08

## 1. R3B mandate (R3B card §A7 + A0 rulings)

1. Distinguish `v1.1.15 @ cee14e9` as **behavior SSOT** from `2896562e` as a **later research snapshot**.
2. Fix the 14-unit totals to **`COPY=0 / ADAPT_PRODUCT=4 / REIMPLEMENT_FROM_BEHAVIOR=10`**; make explicit that
   ADAPT_PRODUCT **extends this repository only**.
3. For each unit give target UI placement, existing product symbol, required future command/state boundary,
   tests, safety, license/provenance note.
4. No donor-source transplant.

## 2. Deliverables (this lane)

| File | Status | Notes |
|---|---|---|
| `logs/research/M5-W18/A7-R3-git-workflow-reference-map.md` | **revised (R3B)** | §0 pin reconciled; §2.0 per-unit 9-col matrix added; §4 tally fixed to 4/10; §7 R3B delta + cross-lane handoffs |
| `logs/research/M5-W18/A7-R3B-git-command-map.md` | **new** | frozen Git command-id namespace (44 ids) + `SafetyClass`/tier map (resolves A6 `D1`) + W19 gate re-scope note |
| `logs/research/M5-W18/A7-R3B-lane-checkpoint.md` | **new** | this file |

## 3. Changes vs the R3 report (closes A0 `R3B-07`)

- **Revision identity:** `cee14e9` fixed as behavior SSOT; `2896562e` explicitly labelled "later research snapshot only"
  (was absent in R3 → looked inconsistent with A10).
- **Classification total:** §4 was `ADAPT=5 / REIMPLEMENT=9`; corrected to `ADAPT_PRODUCT=4 / REIMPLEMENT_FROM_BEHAVIOR=10`.
  The 14-row matrix (§3) and new per-unit matrix (§2.0) both total 14, so tally now matches the table.
- **ADAPT_PRODUCT extends this repo:** stated in §4 + §2.0 — these 4 units (status, diff, branches, merge) are incremental
  extensions of `sync.rs`/`bridge.rs`; the 10 REIMPLEMENT units build from observed behavior via git2 APIs. No Kotlin copy.
- **Per-unit spec:** §2.0 delivers all R3B-required columns for all 14 units (placement / symbol `file:line` / future boundary /
  tests / safety / license).

## 4. Mechanical evidence (pre-commit)

- `python3 scripts/check-git-write-policy.py` → **ok (PASS)** (`GIT_WRITE_*` invariants hold).
- `python3 scripts/check-git-ui-policy.py` → **ok (PASS)**.
- `node scripts/check-git-ui-logic.mjs` → **53 PASS / ok** (run with `node_modules` linked from canonical tree; not committed).
- `git diff --check` on both research files → **clean**.
- No product files modified (`git status` shows only `logs/research/...` additions/edits).

## 5. Risks / open items for downstream lanes

- **W19 gate re-scope (R3B §3 handoff):** `check-git-write-policy.py` `FORBIDDEN_IN_WRITE_SECTION` currently blacklists
  `.merge(`, `reset(`, `stash`, `rebase`, `cherry[-_]pick`. To ship U08/U09/U10/U07 (and ADAPT merge) W19 must whitelist the
  *product's own* confirmable-job-gated `GitWriteOp` variants while keeping the raw-shell-git / ungated-API blacklist. A7 only
  records this requirement; it does not modify the gate.
- **Provenance (A10):** file-level provenance + NOTICE duties for git4idea behavior remain A10's pre-W19 duty; `cee14e9` is the pin.
- **A6 D1 resolved:** Git command ids/labels/`SafetyClass` are now frozen in `A7-R3B-git-command-map.md`; A6 must consume, not re-defer.

## 6. Dependencies

- Consumes: A0 audit rulings (SSOT pin, 4/10 totals, ADAPT_PRODUCT=this-repo), R3B card §A7, A10 `F01/F03/F04`.
- Feeds: A1 (placements, esp. U04 reversible log + U11 conflict window), A3/A4 (U04 toggle + U06/U09 persistence), A6 (command ids),
  A9 (safety classes/tiers), A10 (provenance), A11 (manifest totals + pin).

## 7. Commit

- Commit scope: `logs/research/M5-W18/A7-R3-git-workflow-reference-map.md`,
  `logs/research/M5-W18/A7-R3B-git-command-map.md`, `logs/research/M5-W18/A7-R3B-lane-checkpoint.md`.
- **Not pushed** (only A0 integrates/pushes per board).
