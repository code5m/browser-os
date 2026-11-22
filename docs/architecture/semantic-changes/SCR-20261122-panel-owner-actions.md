# Semantic Change Request: panel Owner actions

## New Semantic

- Type: existing state writer enforcement; no new state or resource lifecycle.
- Classification: CURRENT_FACT for the existing state/Owner assignment; implementation and regression execution remain subject to the next CI result.
- Base: `c4e2df39505cb3b12285b90e9ddc48923af7183d`.
- Trigger: Full Validation run `45032187170`, validate job `137238824106`, reported 17 R9 cross-Owner writes.

## Why Existing Semantic Cannot Represent

The registry already assigns `sidebarOpen`, `browserDockOpen`, `browserDockTab` and `clipOpen` to Layout, and `aiNavOpen` to Browser. The implementation exposed toggle actions, but components and Workbench also need explicit close and snapshot restoration. A toggle cannot express repeated close or restoration of a known boolean safely.

The Dock's internal tab selection also differs from its menu toggle: selecting the visible tab keeps the Dock open; choosing that tab again from the menu collapses it. Contribution-defined view identifiers remain accepted.

## Existing Alternatives

| Existing action | Retained behavior | Missing operation |
| --- | --- | --- |
| Layout `toggleSidebar` | Inverts sidebar visibility | Explicit close or restore |
| Layout `toggleBrowserDock` | Menu opens/switches the Dock; same-tab repeat closes it | Select a Dock tab without changing visibility; close/restore explicitly |
| Browser `toggleAiNav` / `gotoAI` | Menu toggles; navigation closes the AI panel | Explicit close or restore through the public Owner |
| Layout `toggleClipboard` | Toggles the panel and retains its existing navigation behavior | Host-requested repeated close without reopening |
| Layout `closeNavSection` | Closes the activity expansion row | Already sufficient; Workbench reuses it |

## State / Intent / Owner Impact

Owners and stored state remain unchanged. The canonical write points become:

- `Layout.setSidebarOpen(boolean)`.
- `Layout.setBrowserDockOpen(boolean)`.
- `Layout.selectBrowserDockTab(string)`.
- `Browser.setAiNavOpen(boolean)`.
- Existing `Layout.toggleClipboard` and new `Layout.closeClipboard`.

Existing menu actions delegate to these writers. The six sidebar panels, AI panel, ActivityBar, MainArea, Workbench initialization/collapse/restore, and BrowserContext host adapter call Owner actions. The same adapter's clipboard close request also stops writing the Layout field directly.

The old registry note accepting AINavPanel's direct write is removed. No cross-Owner writer is added to a whitelist.

The same host adapter also reads Workbench's public Pinia boolean directly for Vault's collapsed-state port. The previous extra `.value` access returned `undefined`; the adapter must report the current `false` / `true` state without caching it.

## Second Source of Truth Risk

No second state copy, manager, or resource controller is introduced. Workbench keeps its existing persisted boolean snapshot and restores it through the same Owners. Panel actions change the existing reactive preference fields; they do not create or destroy native resources. Existing view guards, native completions, and fileEditorOpen ownership stay intact.

## Checker and Regression Impact

- Update `states.yaml` canonical writers and the contribution-based Dock selection description.
- Keep R9 and its negative fixtures unchanged.
- Extend `check-unified-tab-actions.mjs` with actual template click expressions and actual Layout/Browser implementations. Caller-side proxies reject direct writes; repeated closes, Dock selection versus menu toggling, legacy host adapter forwarding, and Vault's false/true/false Workbench-state readback are checked.
- Extend the existing real Pinia Workbench persistence regression with Owner action observation for collapse, restart initialization, and snapshot restoration.
- Both regression scripts already run in the full `npm run check` chain. No workflow or package command is added.

## ADR / Review

The existing unique-Owner decision remains in force (including `ADR-SEM-P6A-1` for AI navigation). No new Owner or lifecycle decision is proposed. This SCR records the concrete writer repair requested during the authorized CI failure review.

Reviewer: root integration review and workflow-scope peer review of Owner/registry/caller contracts; actual CI remains required.
Date: 2026-11-22.

## Verification Status

- Source changes and regression cases prepared against the exact base above.
- No local project command or local runtime executed.
- Next-SHA semantic self-test, full repository scan, frontend build, and behavior regressions: PENDING actual CI.
- Native/installed GUI acceptance is not inferred from these tests.
