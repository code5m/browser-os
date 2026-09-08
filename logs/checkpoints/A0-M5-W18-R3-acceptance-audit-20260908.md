# A0 M5-W18-R3 Acceptance Audit

Time: 2026-09-08 15:07 CST
Controller: A0
Verdict: `REVISE_TARGETED`
NEXT: `M5-W18-R3B`
W19: `CLOSED`

## Integrated evidence

A1-A11 R3 branches were clean, based on the R3 baseline and limited to research, prototype, test and checkpoint files. A0 integrated all 26 lane commits into canonical `master` without product-code changes.

Mechanical checks after integration:

- A3 tool-window state machine: `58 assertions PASS`.
- A4 shell-state prototype: exit 0.
- A5 database state model: `78 checks PASS`.
- A11 matrix: `56 PASS / 0 FAIL / 2 NOT_RUN / 2 GAP_CONFIRMED / 2 WARN`; this is evidence inventory, not A0 product acceptance.
- `git diff --check`: PASS.
- A10 prototype self-containment checker initially crashed because it assumed every regular expression had two capture groups. A0 corrected the checker. The corrected run is a real FAIL: only 4 of 7 prototypes are clean.

## Blocking findings

1. `R3B-01 Git coverage`: the consolidated A1 prototype visibly demonstrates only part of the required 14-unit Git workflow. Worktree, amend, reset/revert, conflict resolution, patch and command-log placement are not complete.
2. `R3B-02 prototype hygiene`: A1 and A3 display donor branding in prototype UI; A5 embeds a donor font name. The corrected A10 gate reports three prototype failures. Behavior references belong in reports, not product-facing prototype text.
3. `R3B-03 collapse semantics`: A3 and A4 disagree about pinned windows during Collapse All.
4. `R3B-04 accessibility`: the consolidated prototype has no complete `role`, `aria-*`, keyboard focus and disabled-reason demonstration.
5. `R3B-05 geometry`: A1/A2/A3/A8 use incompatible width and top-chrome measurement conventions.
6. `R3B-06 shortcut`: `Ctrl+K` and `Ctrl+Shift+P` are both used as command entry.
7. `R3B-07 reference identity`: A7 and A10 use different Rebased revisions and the A7 classification total does not match its 14-row table.
8. `R3B-08 minimum window`: acceptance does not cover the product minimum `900x600`; `800x600` is below the product minimum and is not a target.
9. `R3B-09 native feasibility`: child WebView focus, occlusion and resize behavior has not been validated in a real desktop client.

## A0 rulings

- The browser-first direction is accepted: two compact rows above a large active surface, with advanced features in tool windows, context menus and the command palette.
- Collapse All hides every tool window, including pinned windows, and records one restorable layout snapshot. Pinning affects ordinary replacement and auto-hide only. Restore Layout restores visibility, sizes and pin states.
- A compact 28px activity/tool strip remains visible in collapsed mode. All resizable panels are hidden.
- Geometry SSOT uses the application inner viewport, excluding the OS titlebar. Normal top chrome is exactly 60px (two 30px rows); status bar is 24px. Collapsed active width is `(viewport width - 28) / viewport width`; collapsed active height is `(inner height - 60 - 24) / inner height`.
- Acceptance sizes are `1920x1080`, `1440x900`, `1366x768`, `1200x800`, `1024x720` and minimum `900x600`. `800x600` is dropped.
- `Ctrl+K` focuses the browser address/search field. `Ctrl+Shift+P` opens the command palette.
- Rebased behavior SSOT is release `v1.1.15` at `cee14e9`. A10's `2896562e` is retained only as a later research snapshot. No source copying is authorized.
- The 14-unit classification is `COPY=0`, `ADAPT_PRODUCT=4` (status, diff, branches, merge) and `REIMPLEMENT_FROM_BEHAVIOR=10` (hunk staging, log graph, worktrees, stash, rebase/interactive rebase, cherry-pick, conflicts, history/blame, patch, command log). `ADAPT_PRODUCT` means extending this product's existing implementation, never transplanting Rebased/JetBrains source.
- W19 remains closed. R3B must end with a user-reviewable replacement prototype and an independent acceptance package.

## Exit gate for R3B

R3B may be presented to the user only when the corrected A10 self-containment checker passes every prototype, the A1 prototype visibly locates all 14 Git units, A2 verifies every target size including `900x600`, A3/A4 agree on collapse/restore, accessibility checks are mechanical, A10 has no open high finding, and A11 reports no unexplained gap or warning.
