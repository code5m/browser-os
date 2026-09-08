# A1 · M5-W18-R3 Checkpoint（终稿，消费 A2-A10）

```text
LANE=A1
DISPATCH=M5-W18-R3-UX
STATUS=FINAL_READY_FOR_A0_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=200f0f1
HEAD=<pending commit>
CONSUMED_PEERS=A2(63b0a6e), A3(0258c5b), A4(8e77799), A6(d5eb144), A7(6e1f2ba), A8(0db481a), A9(23d12cd), A10(2a0722b) — A5(200f0f1,0 commits ahead: no R3 delta)
FILES=logs/research/M5-W18/A1-R3-browser-workbench.md, logs/research/M5-W18/A1-R3-prototype.html, logs/research/M5-W18/A1-R3-checkpoint.md
CORRECTIONS=R2B prototype rejected (too dense); R3 returns to Chrome-like content-first + IDEA progressive disclosure; final consumes A2-A10 all findings
VERIFY=HTML prototype browser-openable; A10 viewport-budget.py recomputed PASS; A3 model 58 assertions PASS; A5 model 78 assertions PASS; no build or product test run
PROPOSED_SLICES=none (R3 is prototype research, not coding authorization)
OPEN_DECISIONS=A0 rules on F05(size budget ~346B)/F12(WebView owner)/F01(rebased license); W19 prereqs: A6 registry + A7 Git backend
NEXT=A0 integration review; A11 acceptance packaging
NO_PRODUCT_CODE=true
NO_PUSH=true
```

## What A1 R3 delivered (final, consuming A2-A10)

### 1. Browser-first replacement shell prototype (`A1-R3-prototype.html`)
- Chrome-like: A2 8-control top row (down from 19) + compact tabs + address bar + large content area
- IDEA-style progressive disclosure: A3 CR-1~CR-12 tool window state machine, NARROW policy
- 7 modes: normal / collapsed / focus / database / knowledge / Git(central) / Git(bottom) / context-menu / command-palette
- 4 sizes: 1920×1080 / 1440×900 / 1366×768 / 1024×720 (800×600 dropped per A10 F13: min_inner_size=900×600)
- 4 states: empty / loading / error / disabled
- Git workflow: A7 14 units (COPY=0/ADAPT=5/REIMPLEMENT=9), D-R1 central↔bottom reversible
- Context menus: A6 CommandDef schema (id/scope/safetyClass/confirmTier), A9 4 safety classes
- Command palette: A6 registry + A7 classification (ADAPT/REIMPLEMENT + SAFE/MUTATES/DESTRUCTIVE)
- Visual tokens: A8 single accent #2b6cb0, 4px spacing scale, 3 corner radii, 1px hairline

### 2. Design report (`A1-R3-browser-workbench.md`)
- §2 shell design: A2 8-control top chrome ≤80px, A3 edge strip 28px, content-first, A3 tool windows, status bar
- §3 mode specs: 8 modes with A3/A5/A6/A7/A9 behavior references
- §4 viewport measurements: A2 measured + A3 model + A10 viewport-budget.py recomputed PASS
- §5 size adaptation: 4 sizes + 800×600 dropped (A10 F13)
- §7 Git workflow: A7 14-unit classification (COPY=0/ADAPT=5/REIMPLEMENT=9)
- §8 diff from rejected R2B prototype
- §9 peer consumption summary (A2-A10)

### 3. Constraint compliance (A10 G1-G8 gates)
- G1 reuse evidence: no COPY declared (A7 COPY=0) ✅
- G2 JetBrains source: all REIMPLEMENT_FROM_BEHAVIOR ✅
- G3 no branding/assets: prototype uses synthetic UI only ✅
- G4 action落地: context menu items have A6 command IDs + PROPOSED_NEW tags ✅
- G5数字可核实: viewport numbers source-derived (A2/A3/A10) ✅
- G6 viewport算术: A10 viewport-budget.py recomputed PASS ✅
- G7 state唯一归属: A3 owns tool windows, A4 owns persistence, A6 owns registry ✅
- G8 prototype自包含: no product source import, no dependency, no ACL, no native call ✅

## Corrections from R2B

| # | R2B (rejected) | R3 final (replacement) | Source |
|---|---|---|---|
| 1 | Permanent multi-panel squeeze | Content-first, on-demand tool windows | A2 §3 |
| 2 | 48px activity bar with 9 icons | 28px edge strip, icon-only | A2 §7.3 |
| 3 | Always-visible feature buttons | Context menus + command palette | A6 |
| 4 | Git as small status panel | Full Git workflow 14 units, central/bottom reversible | A7 |
| 5 | No context menus | A6 CommandDef + A9 safety classes | A6/A9 |
| 6 | No command palette | Ctrl+K unified entry with A7 classification | A6/A7 |
| 7 | 800×600 in target sizes | Dropped (min_inner_size=900×600) | A10 F13 |
| 8 | Estimated viewport numbers | Source-derived (A2 measured + A3 model + A10 script) | A2/A3/A10 |

## Boundary compliance

- Zero product-code changes; no dependency install; no build/test run; no push.
- All output under `logs/research/M5-W18/A1-R3-*` only.
- Prototype HTML is self-contained with synthetic data only.
- No Rebased branding, assets, or JetBrains platform code copied (A10 G2/G3).
- R1/R2/R2B reports preserved (integrated into master by A0).

## Unresolved questions dispatched to A0

- U1 (F05: size budget ~346B remaining) → A0 ruling
- U2 (F12: native WebView owner unassigned) → A0 assignment
- U3 (F01: rebased license = JetBrains Build Terms v1.3) → A0 legal acceptance
- U4 (W19 prereq: A6 command registry) → W19 S1
- U5 (W19 prereq: A7 Git backend 9 REIMPLEMENT units) → W19 S2/S3
