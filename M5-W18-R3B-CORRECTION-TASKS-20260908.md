# M5-W18-R3B Targeted Prototype Closure

Version: 2026-09-08 15:07 CST
Mode: `RESEARCH_AND_PROTOTYPE`
NEXT: `M5-W18-R3B`
W19: `CLOSED`

## Shared inputs and frozen decisions

Read `WORKSPACE_IDENTITY.md`, `PARALLEL_COMMAND_BOARD.md`, `logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`, the R3 lane outputs and this card.

All lanes must preserve the current Chrome-like, browser-first shell and the A0 rulings in the audit. This is a correction wave, not a redesign restart. No product source, dependency, ACL, capability, native runtime or user data may change.

The final prototype must be self-contained, synthetic and open directly. Donor names, assets, fonts and branding must not appear in prototype UI. Reports may cite sources. No source copying is authorized.

## Lane assignments

### A1 - Corrected consolidated prototype

Revise the existing A1 report and HTML, consuming the corrected A2-A9 outputs. Keep two 30px browser rows, the 28px activity strip and the large content surface. Demonstrate all six target sizes, collapsed/restore/focus behavior, keyboard focus, context menus and failure states. The Git mode must visibly locate all 14 workflow units, including worktrees, amend, reset/revert, conflicts, patch and command log; advanced actions belong in menus/palette rather than permanent buttons. Add semantic roles and ARIA. Remove donor names from UI. Deliver a final checkpoint and no product code.

### A2 - Canonical geometry and minimum-size evidence

Replace divergent measurement conventions with the A0 formula. Measure the current baseline and revised prototype at 1920x1080, 1440x900, 1366x768, 1200x800, 1024x720 and 900x600. Verify 60px top chrome, 24px status bar, 28px activity strip, >=85% active height and >=92% active width. Explain measured versus derived values and update the executable measurement evidence.

### A3 - Collapse/restore and tree semantics

Align report, prototype and state machine with the A0 ruling: Collapse All hides all tool windows including pinned; Restore Layout restores visibility, dimensions and pin state from one snapshot; pin only changes ordinary replacement/auto-hide. Keep tree collapse/expand behavior separate. Add 900x600 and keyboard-focus assertions. Remove donor branding from prototype UI.

### A4 - Persistence contract reconciliation

Update the state/persistence contract and pure tests to exactly match A3 and the A0 ruling. Specify snapshot lifecycle, crash recovery, unknown-window fallback and workspace/global ownership. Prove Collapse All and Restore Layout are deterministic and do not persist temporary hover/overlay state.

### A5 - Database mode closure

Reconcile the database prototype with canonical geometry and A3/A4 state semantics. Remove donor font/branding strings. Verify connection tree, multiple SQL documents, results, history and properties remain discoverable through tool windows and scoped context menus, while focus mode restores the active editor/result surface. Add 900x600 evidence and rerun the 78-check model.

### A6 - Command registry, shortcuts and accessibility

Freeze `Ctrl+K` for address/search and `Ctrl+Shift+P` for command palette. Ensure all shell, database, knowledge and all 14 Git workflow actions have stable command IDs, scoped context-menu placement, disabled reasons, keyboard or accessible paths and safety classes. Provide a machine-checkable registry audit; no permanent toolbar-button expansion.

### A7 - Git reference and 14-unit correction

Correct the reference map to distinguish `v1.1.15/cee14e9` as behavior SSOT from `2896562e` as a later research snapshot. Correct the 14-unit totals to `COPY=0 / ADAPT_PRODUCT=4 / REIMPLEMENT_FROM_BEHAVIOR=10`, and make clear that ADAPT_PRODUCT extends this repository only. For each unit, give target UI placement, existing product symbol, required future command/state boundary, tests, safety and license/provenance note. No donor-source transplant.

### A8 - Visual and accessibility states

Revise all frames to canonical geometry and six target sizes. Define restrained light/dark tokens, keyboard focus, selected/hover/disabled states, menus and high-density 900x600 behavior. Check text fit and ensure the active content remains visually dominant. Provide objective pixel and ratio evidence.

### A9 - Complete Git and interaction safety matrix

Extend the safety review across all 14 Git units, especially amend, reset/revert, rebase continue/abort, cherry-pick continue/abort, conflict resolution, patch apply and command log. Freeze dirty-tree, protected-branch, confirmation, undo/abort, audit-redaction and credential boundaries. Verify context menus do not bypass command policy.

### A10 - Independent closure review

After A1-A9 commit, run the corrected self-containment checker and independently verify every A0 ruling, every target size, all 14 Git placements, accessibility, exact reference pins and classification totals. Treat a crashing checker as FAIL. Return a severity-ordered review; no high finding may remain open before A11 finalizes.

### A11 - Final acceptance package

After A10 and the corrected A1 prototype, update the manifest, matrix, user-review checklist and progress ledger. A report that merely finds words in a reference document does not prove prototype coverage: Git and accessibility probes must inspect the final A1 HTML. Final status requires 0 FAIL, 0 GAP_CONFIRMED, 0 unexplained WARN and 0 NOT_RUN except native desktop evidence explicitly reserved for user/A0 review.

## Sequence and merge order

1. A2-A9 work in parallel.
2. A1 consumes A2-A9 and publishes the corrected final prototype.
3. A10 reviews A1-A9 and publishes the independent verdict.
4. A11 consumes A1 and A10 and publishes the final package.
5. A0 integrates, runs all gates and opens the prototype for user review.

Merge order: `A2 -> A3 -> A4 -> A5 -> A6 -> A7 -> A8 -> A9 -> A1 -> A10 -> A11 -> A0`.

## One-line prompt

```text
LANE=A1；读取主仓库 /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/WORKSPACE_IDENTITY.md、PARALLEL_COMMAND_BOARD.md 和 M5-W18-R3B-CORRECTION-TASKS-20260908.md，推导并进入本lane的 m5-w18-aN 工作树与 codex/m5-w18-aN 分支，确认工作树干净后 fetch 并 rebase origin/master，按 R3B 卡完成整包修订、自检并提交本lane；不改产品代码、不改他lane文件、不push。
```

Only replace the first `A1` with the assigned lane id.
