# A1 · M5-W18-R3B Checkpoint（终稿，消费 A2-A9 R3B 修正）

```text
LANE=A1
DISPATCH=M5-W18-R3B
STATUS=FINAL_READY_FOR_A10_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=d96b9b5
HEAD=<pending commit>
CONSUMED_PEERS=A2(bfe8e86), A3(8d8719a), A4(5d9e447), A5(3ff1f8b), A6(2bce4e3), A7(b96cd51), A8(ba92110), A9(d17091c)
FILES=logs/research/M5-W18/A1-R3B-browser-workbench.md, logs/research/M5-W18/A1-R3B-prototype.html, logs/research/M5-W18/A1-R3B-checkpoint.md
CORRECTIONS=R3B-01 Git 14 units; R3B-02 donor removed; R3B-03 collapse aligned; R3B-04 ARIA added; R3B-05 geometry SSOT; R3B-06 shortcuts frozen; R3B-07 ref pinned; R3B-08 6 sizes; R3B-09 NOT_RUN
VERIFY=HTML browser-openable; A2 measure.py PASS; A6 registry-audit PASS; A3 58 assertions PASS; A4 29/29 PASS; A5 78 PASS; no build run
PROPOSED_SLICES=none (prototype research, not coding authorization)
OPEN_DECISIONS=A0 rules on F05/F12/F01; W19 prereqs: A6 registry + A7 Git backend; R3B-09 native NOT_RUN
NEXT=A10 independent review; A11 acceptance packaging
NO_PRODUCT_CODE=true
NO_PUSH=true
```

## What A1 R3B delivered (final, consuming A2-A9 R3B corrections)

### 1. Corrected prototype (`A1-R3B-prototype.html`)
- **R3B-01 Git 14 units**: all 14 units visible in 3 locations (grid + context menu + command palette)
  - U01 status, U02 hunk, U03 diff, U04 log/graph, U05 branch, U06 worktree, U07 stash, U08 merge, U09 rebase, U10 cherry-pick, U11 conflict, U12 blame, U13 patch, U14 command log
  - Plus: amend, reset, revert, rebase continue/abort, push (disabled with reason)
- **R3B-02 donor branding**: 0 donor names in UI (Rebased/JetBrains/Obsidian/IntelliJ all removed)
- **R3B-03 collapse semantics**: Collapse All hides all (including pinned), records snapshot; Restore Layout deterministic; pin affects replacement/auto-hide only
- **R3B-04 accessibility**: 94 role=, 127 aria-*, 78 tabindex, :focus-visible, aria-disabled+title for disabled reasons, keyboard menu navigation (↑/↓/Enter/Esc)
- **R3B-05 geometry SSOT**: 60px top (2×30), 24px status, 28px strip as CSS hard constants; A0 formula encoded
- **R3B-06 shortcuts**: Ctrl+K = address/search, Ctrl+Shift+P = command palette (frozen, distinct, no collision)
- **R3B-07 reference**: SSOT = v1.1.15@cee14e9; classification COPY=0/ADAPT_PRODUCT=4/REIMPLEMENT=10
- **R3B-08 sizes**: 6 target sizes (1920/1440/1366/1200/1024/900×600), 800×600 dropped; all PASS ≥85% H / ≥92% W
- **R3B-09 native**: NOT_RUN (deferred to A9/A11)

### 2. Design report (`A1-R3B-browser-workbench.md`)
- §1 A0 audit findings + R3B task
- §2 per-finding corrections (R3B-01 ~ R3B-09)
- §3 mode coverage (9 modes)
- §4 state coverage (4 states)
- §5 peer consumption table (A2-A9 with SHAs)
- §6 diff from R3 final
- §7 compliance checklist
- §8 unresolved questions

### 3. Constraint compliance (A0 rulings)
- Browser-first direction preserved ✅
- Two 30px rows + 28px strip + large content ✅
- Collapse All includes pinned ✅
- 28px strip visible in collapsed ✅
- Geometry 60/24/28 hard constants ✅
- 6 sizes including 900×600 ✅
- Ctrl+K / Ctrl+Shift+P frozen distinct ✅
- Rebased SSOT cee14e9 ✅
- 14-unit 0/4/10 classification ✅
- No source copying ✅

## Corrections from R3 (ee2b8fd)

| # | R3 (rejected by audit) | R3B (corrected) | A0 finding |
|---|---|---|---|
| 1 | Git partial (missing 6 units) | All 14 units in 3 locations | R3B-01 |
| 2 | "Rebased" in UI | 0 donor names in UI | R3B-02 |
| 3 | A3/A4 collapse disagreement | Aligned to A0 ruling | R3B-03 |
| 4 | role/aria = 0 | 94 role, 127 aria, 78 tabindex | R3B-04 |
| 5 | Multiple geometry conventions | A0 SSOT 60/24/28 | R3B-05 |
| 6 | Ctrl+K/Ctrl+Shift+P conflict | Frozen distinct | R3B-06 |
| 7 | Mixed Rebased revisions | SSOT=cee14e9 only | R3B-07 |
| 8 | 4 sizes (missing 1200/900) | 6 sizes, 800×600 dropped | R3B-08 |
| 9 | Native not validated | NOT_RUN (explicit) | R3B-09 |

## Boundary compliance

- Zero product-code changes; no dependency install; no build/test run; no push.
- All output under `logs/research/M5-W18/A1-R3B-*` only.
- Prototype HTML is self-contained with synthetic data only.
- No donor branding/assets/source in prototype UI (R3B-02).
- R1/R2/R2B/R3 reports preserved.

## Unresolved questions dispatched to A0/A10/A11

- U1 (F05: size budget ~346B) → A0 ruling
- U2 (F12: native WebView owner) → A0 assignment
- U3 (F01: rebased license) → A0 legal acceptance
- U4 (W19 prereq: A6 command registry) → W19 S1
- U5 (W19 prereq: A7 Git backend 10 REIMPLEMENT) → W19 S2/S3
- U6 (R3B-09: native WebView validation) → A9/A11 NOT_RUN
