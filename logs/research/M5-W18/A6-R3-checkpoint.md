# A6 · M5-W18-R3 Lane Checkpoint

```text
LANE=A6
STATUS=PASS_WITH_DEBT
BASE=200f0f1cc032eb7dbd0229ab53a711a5ff1e3d6f
HEAD=898e4e4ac495b1cc648af7b899140078b6719665
REFERENCE_EVIDENCE=CURRENT_PRODUCT src/stores/useWorkspaceStore.ts:208-226,227-316; src/components/workspace/FilePanel.vue:68-98; src/components/workspace/ArtifactPanel.vue:19; src/App.vue:163-207; src/stores/useSettingsStore.ts:8-54; src/components/workspace/CommandSnippetPanel.vue:78-81; src/components/layout/StatusBar.vue:19-24. REFERENCE_SOURCE DetachHead/rebased (Git), JetBrains IDEA action system (behavior), Obsidian note/graph menus (behavior).
FILES=logs/research/M5-W18/A6-R3-context-menu-command-registry.md, logs/research/M5-W18/A6-R3-checkpoint.md
SOURCE_MAP=see A6-R3-context-menu-command-registry.md §10
CLASSIFICATION=COPY=0, ADAPT=4, REIMPLEMENT_FROM_BEHAVIOR=5, DEFER=1, REJECT=0
VERIFY=static source read (file:line cited) + structural self-check (§13 of main doc). No build/runtime executed (research-only boundary).
CHECKPOINT=logs/research/M5-W18/A6-R3-checkpoint.md
MERGE_NOTES=A7 Git action map (D1) + A9 safety/audit gate + A3 tool-window focus must land before W19 implementation. A6 only specifies the registry contract; does not edit product code, other lanes' files, or push.
NEXT=W19 S1 (shell + CommandRegistry bootstrap), then S2/S3/S4 domain context menus consume this contract.
```

## Scope (R3 A6 card)
Context-menu and command registry: inventory object scopes (browser tab/page, workspace/file/tree, note/link/graph node, database connection/schema/table/cell/result, Git repo/branch/commit/file/hunk, terminal, tool window); define grouped menus, tree expand/collapse commands, disabled reasons, destructive confirmations, audit class, keyboard and command-palette parity.

## What was delivered
- A single `CommandRegistry` contract as the only source of truth for command identity, context menus, and the new command palette.
- Object-scope inventory with grouped `CommandDef`s for all 10 enumerated scopes.
- Tree bulk-op model with a hard `expandAllBounded` cap (mirrors `GRAPH_MAX_DEPTH=4` / `GRAPH_QUERY_LIMIT=1000` from A7 W5), rejecting unbounded `expandAll`.
- Disabled-reason hook, 4-tier destructive confirmation model (T0–T3), and redacted audit class.
- Keyboard + command-palette parity matrix proving every menu action is also a palette entry and (where defined) a keybinding.
- Reference classification (ADAPT/REIMPLEMENT/DEFER/REJECT) with explicit source identity.

## Current-product gaps confirmed (CURRENT_PRODUCT)
- No command registry; 2 ad-hoc context-menu states (`fileCtx`, `ctxMenu`) inline-rendered (G1–G2).
- Danger items only styled, no disabled-reason / confirmation-tier / audit model (G3).
- Fixed 9-action keymap, not registry-driven; trees have zero keybindings (G4).
- No command palette; "命令库" is a snippet library, not a searchable command entry (G5).
- No tree collapse-all / expand-all at all (G6); context menus not typed/scoped (G7); only file+artifact scopes wired (G8).

## Debt (PASS_WITH_DEBT)
- **D1**: Git-object `CommandDef` ids/labels/safety classes are placeholders pending A7's exact Rebased action map. Blocks W19 Git-menu implementation only.

## Boundary compliance
- No edits under `src/`, `src-tauri/`, manifests, lockfiles, ACLs, capabilities, or the user vault.
- Rebased onto `origin/master` (200f0f1); branch `codex/m5-w18-a6`; not pushed.

## Downstream consumers
- A1: consumes the registry contract for the consolidated prototype's menu/palette wiring.
- A9: reconciles T2/T3 confirmation + audit class with the safety threat model.
- A10: records provenance for ADAPT/REIMPLEMENT reference items.
- A11: verifies menu ⊆ palette ⊆ registry parity in the acceptance manifest.
