# M5-W18 Workbench Prototype Review

Status: `REVISE`

## User verdict

The first prototype is rejected as the product shell. It makes the application feel smaller and busier than the current Chrome-like layout.

- Preserve the familiar browser-first structure and large content viewport.
- Remove always-visible feature buttons; the default state must be simpler and more spacious.
- Borrow IDEA's discoverability and progressive disclosure rather than its dense permanent chrome: tool windows collapse all/restore/maximize, trees collapse all/expand one level/bounded expand all, plus pin/unpin and auto-hide.
- Put complete, scope-aware actions in right-click menus and the command entry.
- Git must become a full workflow modeled primarily on `DetachHead/rebased`, not a small status panel: stage by file/hunk, diff, log/graph, branches, worktrees, stash, merge, rebase/interactive rebase, cherry-pick, conflict resolution and command history.
- R3 task cards and replacement prototypes are defined in `M5-W18-R3-UX-TASKS-20260908.md`.

This review decides the interaction direction before product UI code is changed. The prototypes use synthetic data and do not invoke native commands.

## Review inputs

1. Overall IDEA-style shell: `logs/research/M5-W18/A1-wireframe-prototype-R2B.html`
2. Database workbench detail: `logs/research/M5-W18/A5-R2B-wireframe.html`
3. Product intent and six daily workflows: `WORKBENCH_BLUEPRINT-20260908.md`

## What to judge

- **Shell:** a quiet IDEA-like workbench, not the current row of large top-level buttons.
- **Navigation:** project and tool windows at the sides, documents in the center, terminal/problems at the bottom, contextual details on the right.
- **Continuity:** opening a database console, note, graph node, search result, browser page, file, or terminal preserves a stable return position.
- **Database:** connection tree, multiple SQL documents, execution/cancel state, bounded result grid, history and export are one coherent daily loop.
- **Knowledge:** notes, backlinks, local graph and search are connected views of the same workspace, without copying Obsidian branding or proprietary assets.
- **Search:** one entry for files/text/notes/actions, with visible source, scope, freshness and truncation; exact local matching ships before semantic search.
- **Responsive behavior:** 1440x900 is full layout, 1024x720 collapses secondary detail, 800x600 keeps the active document and essential navigation usable.

## Decision rule

- `ACCEPT`: A0 may open named W19 slices, starting with S0 and S1.
- `REVISE`: record concrete layout/workflow changes and return only the affected design lane; product UI stays closed.
- `REJECT`: return to shell architecture, preserving all accepted domain research.

W19 remains `CLOSED` until this review is recorded. Backend correctness work is also held so that one explicit opening checkpoint controls the next wave.
