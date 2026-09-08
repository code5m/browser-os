# M5-W18-R3 · A6 — Context-Menu & Command Registry

> LANE=A6 · Mode=`RESEARCH_AND_PROTOTYPE` · REBASED ONTO `origin/master`
> Deliverable owner: A6. Consumed by: A1 (consolidated prototype), A9 (safety review), A10 (provenance), A11 (acceptance).
> No product source, manifest, ACL, capability, or user-data changed. Research artifact only.

```text
LANE=A6
STATUS=PASS_WITH_DEBT
BASE=200f0f1cc032eb7dbd0229ab53a711a5ff1e3d6f
HEAD=898e4e4ac495b1cc648af7b899140078b6719665
REFERENCE_EVIDENCE=CURRENT_PRODUCT (file:line below); REFERENCE_SOURCE (DetachHead/rebased, JetBrains IDEA action system, Obsidian note/graph menus)
FILES=logs/research/M5-W18/A6-R3-context-menu-command-registry.md, logs/research/M5-W18/A6-R3-checkpoint.md
SOURCE_MAP=see §10
CLASSIFICATION=COPY=0, ADAPT=4, REIMPLEMENT_FROM_BEHAVIOR=5, DEFER=1, REJECT=0
VERIFY=static source read + structural self-check (no runtime/build)
CHECKPOINT=logs/research/M5-W18/A6-R3-checkpoint.md
MERGE_NOTES=A7 Git action map + A9 safety/audit gate + A3 tool-window focus must land before W19 implementation; A6 only specifies the registry contract.
NEXT=W19 slice S1 (shell + command registry bootstrap) and S2/S3/S4 (domain context menus) implementation cards.
```

---

## 1. Executive summary

R3 preserves the Chrome-like content-first shell and borrows IDEA-style progressive disclosure. A6's assigned slice is the **unifying command registry and the context-menu contract** that every other lane's actions must flow through.

Today the product has **no command registry** and **no command palette**. Context menus are two hand-rolled reactive states (`ws.fileCtx`, `ws.ctxMenu`) with inline `@click` handlers and no disabled-reason, audit, or safety metadata. This design introduces a single `CommandRegistry` as the *only* source of truth for:

1. every command's identity, scope, group, label, icon, shortcut, availability;
2. every context-menu's grouping and item availability;
3. every command-palette entry (the new "searchable command entry" required by the R3 global constraints);
4. disabled reasons, destructive-confirmation tiers, and audit class.

A context-menu action and a palette entry are the **same registry row** viewed two ways. This satisfies the R3 global constraint: *"Every context-menu action also has a command-registry identity, disabled reason, keyboard/accessibility path and safety class."*

`STATUS=PASS_WITH_DEBT`: the contract is complete enough for W19 implementation cards, but A6 cannot freeze Git-object action ids until A7 publishes its exact Rebased action map, and the destructive/audit tiers must be reconciled with A9's safety gate before any write-capable command ships.

---

## 2. Current product gap inventory (CURRENT_PRODUCT)

All claims cite exact file:line in the rebased tree (`origin/master` 200f0f1).

| # | Fact | Evidence | Gap |
|---|---|---|---|
| G1 | Two ad-hoc context-menu states, no registry | `src/stores/useWorkspaceStore.ts:208-226` (`fileCtx`, `ctxMenu` reactive objects) | No central command identity |
| G2 | Menu items hardcoded inline, bound to store methods | `src/components/workspace/FilePanel.vue:68-98` (`ctx-menu`, `ctx-item`, `ctx-sep`, `danger`); `src/components/workspace/ArtifactPanel.vue:19` (`onArtifactContext`) | No registry→menu projection |
| G3 | Danger items only styled, no disabled-reason/confirmation tier model | `FilePanel.vue:95` `class="ctx-item danger"`; `useWorkspaceStore.ts:287-301` (`ctxDelete` uses raw `confirm()`) | No confirmation-tier contract; no audit |
| G4 | Keyboard is a fixed 9-action keymap, not registry-driven | `src/stores/useSettingsStore.ts:8-54` (`KEYMAP_SCHEMES`, `ACTION_LABELS`); `src/App.vue:163-207` (`onGlobalKeydown`, `matchKey`) | Not every command has a keybinding; trees have none |
| G5 | No command palette / searchable command entry | `src/components/workspace/CommandSnippetPanel.vue:78-81` ("命令库" is a snippet library, not a command registry); `StatusBar.vue:19-24` labels `commands: 命令库` | New capability required by R3 |
| G6 | No tree bulk operations | grep `collapseAll|expandAll|collapse all|expand all` → 0 matches | No collapse-all / expand-one-level / bounded-expand-all |
| G7 | Context menus not scoped to a typed object | `useWorkspaceStore.ts:227-236` (`onFileContext` sets `entry` but menu is positional, not object-typed) | No "scoped to object under pointer" model |
| G8 | Right-click targets limited to file tree + artifacts | only `FileTreeNode.vue:29` and `ArtifactPanel.vue:19` bind `@contextmenu` | Tab/page, note/link/graph, database, Git, terminal, tool-window scopes absent |

---

## 3. Command Registry schema (the contract)

Proposed destination: new `src/stores/useCommandRegistry.ts` + `src/types.ts` `CommandDef`. A6 specifies the shape; W19 implements.

```ts
// Proposed CommandDef (spec only — not written to product)
type Scope =
  | "browser.tab" | "browser.page"
  | "workspace.file" | "workspace.tree"
  | "note" | "link" | "graph.node"
  | "db.connection" | "db.schema" | "db.table" | "db.cell" | "db.result"
  | "git.repo" | "git.branch" | "git.commit" | "git.file" | "git.hunk"
  | "terminal" | "toolwindow";

type SafetyClass = "safe" | "mutating" | "destructive" | "dangerous";
type AuditClass   = "none" | "ui" | "data-read" | "data-write" | "credential" | "admin";
type ConfirmTier  = "T0" | "T1" | "T2" | "T3";

interface CommandDef {
  id: string;            // stable, dotted: "<scope>.<verb>"  e.g. "file.rename"
  scope: Scope;
  group: string;         // menu group key, e.g. "edit", "git", "view"
  label: string;         // localized display
  icon?: string;
  shortcut?: string;     // default binding; resolved per keymap scheme (G4)
  searchTerms: string[]; // palette fuzzy terms (incl. English for Git verbs)
  runnable: (ctx: CommandContext) => boolean;   // availability predicate
  disabledReason?: (ctx: CommandContext) => string | null; // null = enabled
  safetyClass: SafetyClass;
  auditClass: AuditClass;
  confirmTier: ConfirmTier;
  paletteVisible: boolean;   // false for purely structural items
  execute: (ctx: CommandContext) => Promise<void> | void;
}
```

**Rules (binding for W19):**
- Every context-menu item renders only from a `CommandDef` row; no inline `@click` to store methods (kills G2/G7).
- `disabledReason` returns a human string; UI greys the item and exposes the reason via `aria-disabled` + `title` tooltip ("为什么不可用").
- `confirmTier >= T2` blocks execution behind the confirmation contract (§7).
- `auditClass !== "none"` emits a redacted audit event (§8).
- `shortcut` + `paletteVisible` guarantee keyboard/command-palette parity (§9).

---

## 4. Object-scope inventory & action groups

The R3 card enumerates these scopes. Each maps to a `Scope` and a set of grouped `CommandDef`s. Group keys drive menu section headers and palette filtering.

### 4.1 browser.tab / browser.page
- `tab` group: `tab.new`, `tab.close`, `tab.closeOthers`, `tab.closeRight`, `tab.pin`, `tab.reload`, `tab.duplicate`, `tab.mute`(n/a), `tab.bookmark`.
- `page` group: `page.copyUrl`, `page.openInNewTab`, `page.findInPage`, `page.print`, `page.screenshot`, `page.collectSelection` (existing `collectSelection` in `useWorkspaceStore.ts:318`).

### 4.2 workspace.file / workspace.tree
- `file` group: `file.newFile`, `file.newDir`, `file.rename`, `file.delete`, `file.copyPath`, `file.reveal` (existing `ctxReveal` pattern), `file.openExternal`.
- `tree` group (bulk, §6): `tree.collapseAll`, `tree.expandOneLevel`, `tree.expandAllBounded`, `tree.refresh`, `tree.newFileHere`, `tree.newDirHere`.

### 4.3 note / link / graph.node
- `note` group: `note.open`, `note.openInNew`, `note.rename`, `note.delete`, `note.copyLink` (wikilink), `note.backlinks`, `note.localGraph`.
- `link` group: `link.follow`, `link.copy`, `link.unresolvedCreate`.
- `graph.node` group: `graph.node.open`, `graph.node.localGraph`, `graph.node.expandNeighbors`, `graph.node.hide`, `graph.node.delete`(if user-owned).

### 4.4 db.connection / schema / table / cell / result
- `db.connection` group: `db.connect`, `db.disconnect`, `db.refresh`, `db.properties`.
- `db.schema`/`db.table` group: `db.openConsole`, `db.viewData`, `db.copyName`, `db.truncate`(destructive), `db.drop`(destructive).
- `db.cell` group: `db.cell.copy`, `db.cell.edit`(mutating), `db.cell.null`.
- `db.result` group: `db.result.export`, `db.result.copyRow`, `db.result.cancel`(maps to A4/A6 cancellation contract), `db.result.truncateToggle`.

### 4.5 git.repo / branch / commit / file / hunk  *(A6 enumerates; A7 freezes exact ids)*
- `git.repo` group: `git.status`, `git.fetch`, `git.pull`, `git.push`, `git.stash`, `git.stashPop`, `git.worktreeNew`, `git.logGraph`.
- `git.branch` group: `git.branchNew`, `git.branchCheckout`, `git.branchMerge`, `git.branchDelete`(destructive).
- `git.commit` group: `git.commitAmend`, `git.commitCherryPick`, `git.commitRevert`, `git.commitReset`(destructive), `git.commitCheckout`.
- `git.file` group: `git.fileStage`, `git.fileUnstage`, `git.fileDiff`, `git.fileHistory`(blame), `git.fileIgnore`.
- `git.hunk` group: `git.hunkStage`, `git.hunkUnstage`, `git.hunkDiscard`(destructive).
- A6 marks these **DEFER-to-A7**: exact ids, labels (EN/zh), and safety classes are owned by A7's Rebased action map (see §10 D1).

### 4.6 terminal
- `terminal` group: `term.newTab`, `term.close`, `term.clear`, `term.copy`, `term.paste`, `term.sendEof`, `term.search`.

### 4.7 toolwindow (consumes A3)
- `toolwindow` group: `tw.open`, `tw.close`, `tw.collapseAll`, `tw.restoreLayout`, `tw.maximizeActive`, `tw.pin`, `tw.unpin`, `tw.autoHide`, `tw.resize`(structural).

---

## 5. Context-menu group taxonomy (rendering contract)

Each scope's right-click renders sections in `group` order, separated by a `ctx-sep` (existing pattern, `FilePanel.vue:93`). The menu is **scoped to the object under the pointer** (G7): `onContextMenu(scope, objectId, x, y)` opens `CommandRegistry.menuFor(scope, objectId)`, never a catch-all menu.

Menu item states:
- **enabled** → `ctx-item`, `@click` → `registry.execute(id, ctx)`.
- **disabled** → `ctx-item[aria-disabled="true"]`, greyed, `title=disabledReason(ctx)`, no click.
- **destructive** (`safetyClass=destructive|dangerous`) → `ctx-item.danger` (keeps current `danger` style, `FilePanel.vue:95`) + confirm tier (§7).
- **structural** (`paletteVisible=false`, e.g. a submenu header) → not a palette entry.

Every menu action's row is identical to its palette row → parity by construction.

---

## 6. Tree expand / collapse command model

No current implementation (G6). Model on IDEA/Obsidian bulk tree ops, exposed as registry commands **and** keyboard shortcuts (§9).

| Command | Behavior | Bounded? | Default shortcut (vscode/idea) |
|---|---|---|---|
| `tree.collapseAll` | Collapse every node in active tree | Yes (instant) | `Ctrl+Shift+-` / `Ctrl+NumPad -` |
| `tree.expandOneLevel` | Expand only depth = currentMax+1 | Yes | `Ctrl+Shift++` / `Ctrl+NumPad +` |
| `tree.expandAllBounded` | Expand to `BOUND_DEPTH` (default 4, matches `GRAPH_MAX_DEPTH=4`, A7 W5) then stop; show "仅展开前 N 层" toast | **Yes — hard cap** | `Ctrl+Alt+Shift+E` / `Ctrl+Shift+*` |
| `tree.refresh` | Re-read filesystem / remote | n/a | `F5` / `F5` |

**Freeze-guard (R3 global constraint):** "large trees must not freeze the UI." `expandAllBounded` MUST cap depth and node count (mirror `GRAPH_QUERY_LIMIT=1000`, `GRAPH_MAX_NODES=5000` from `domain.rs` A7 W5) and render asynchronously in chunks. A purely recursive `expandAll` is **REJECTED** as a registry command.

---

## 7. Destructive confirmation tiers

| Tier | Trigger | UX | Audit |
|---|---|---|---|
| T0 | safe / mutating-non-destructive | immediate | per `auditClass` |
| T1 | mutating with trivial undo (rename) | inline confirm chip on the item | `ui` |
| T2 | destructive (delete file, drop table, delete branch) | modal confirm with object name + consequence text | `data-write` |
| T3 | dangerous / protected (force push, reset --hard, protected-branch delete, credential wipe) | modal + typed confirmation / protected-branch policy check (A9) | `admin`/`credential` |

Current `ctxDelete` raw `confirm()` (`useWorkspaceStore.ts:288`) is replaced by T2 modal contract. A9 owns protected-branch / force policy (cross-ref A9 threat model).

---

## 8. Audit class

Every `CommandDef.auditClass !== "none"` emits a redacted audit event on execute (and on *denied* T2/T3). Audit fields: `ts`, `commandId`, `scope`, `objectRef` (redacted path/id, no secrets — per A6 R2B credential-flow findings and `B11-1` clipboard redaction precedent), `outcome`. Logs must **never** include credential material, full SQL, or full file bodies (WORKBENCH_BLUEPRINT §5 persistence rule + A6 R2B). `AuditPanel.vue` already exists as a consumer; registry feeds it.

---

## 9. Keyboard & command-palette parity matrix

Every registry row is reachable three ways: right-click menu, command palette (new), and a keybinding where one exists. The palette is a **new capability** (G5) modeled on IDEA Search Everywhere / Rebased command entry → `REIMPLEMENT_FROM_BEHAVIOR`.

Sample parity rows (full set in `useCommandRegistry.ts` W19):

| Command id | Scope | Menu group | Default shortcut | Palette | a11y path |
|---|---|---|---|---|---|
| `file.rename` | workspace.file | file | `F2` | ✅ | focus menu item + Enter |
| `file.delete` | workspace.file | file | `Delete` | ✅ | `aria-disabled` when protected |
| `tree.collapseAll` | workspace.tree | tree | `Ctrl+Shift+-` | ✅ | toolbar button + palette |
| `tree.expandAllBounded` | workspace.tree | tree | `Ctrl+Alt+Shift+E` | ✅ | palette only (no chord conflict) |
| `tab.close` | browser.tab | tab | `Ctrl+W` | ✅ | existing keymap (G4) |
| `db.result.cancel` | db.result | result | — | ✅ | button + palette |
| `git.hunkStage` | git.hunk | git | — | ✅ | menu + palette (A7 id) |
| `note.copyLink` | note | note | — | ✅ | menu + palette |
| `term.clear` | terminal | terminal | `Ctrl+L`(local) | ✅ | menu + palette |
| `tw.collapseAll` | toolwindow | toolwindow | — | ✅ | A3 focus model |

`KEYMAP_SCHEMES` (G4) is **extended** to drive the registry: each scheme maps `commandId → chord`, replacing the current 9 fixed actions. A11's acceptance checklist verifies parity = (menu item count) ⊆ (palette entries) ⊆ (registry rows).

---

## 10. Reference classification & source map

| Item | Class | Source | Notes |
|---|---|---|---|
| IDEA action system (id/keymap/context-availability) | ADAPT | JetBrains IDEA (experience reference, not code) | Reimplement schema in TS; no platform code copy |
| IDEA Search Everywhere / command palette | REIMPLEMENT_FROM_BEHAVIOR | IDEA (behavior) | No existing palette (G5); build from behavior |
| IDEA/Obsidian tree collapse-all/expand-all/bounded | REIMPLEMENT_FROM_BEHAVIOR | IDEA + Obsidian graph/local-graph | Cap depth (§6) |
| Obsidian note/link/graph right-click (open/new, copy link, backlinks, local graph) | ADAPT | Obsidian observable behavior | A2/A3 own exact semantics; A6 consumes |
| Rebased Git context actions (stage file/hunk, diff, log graph, branches, worktrees, stash, merge, rebase/I-rebase, cherry-pick, conflict, command log) | REIMPLEMENT_FROM_BEHAVIOR | `DetachHead/rebased` (primary Git ref, R3 card) | **DEFER exact ids to A7** (D1) |
| Current `FilePanel.vue` inline ctx-menu render + `ctx-sep`/`danger` styling | ADAPT | CURRENT_PRODUCT (`FilePanel.vue:68-98`) | Keep visual language; route through registry (kills G2) |
| Current `KEYMAP_SCHEMES` | ADAPT | CURRENT_PRODUCT (`useSettingsStore.ts:8-54`) | Extend to full registry |
| `CommandSnippetPanel.vue` snippet library | DEFER | CURRENT_PRODUCT | Distinct from palette; may embed palette later |
| JetBrains platform source / Rebased branding / Obsidian assets | REJECT | — | Forbidden by R3 card + license terms |

**D1 (debt):** Git-object `CommandDef` ids/labels/safety classes are specified here as placeholders; A7's Rebased action map is the authoritative source. A6 blocks W19 Git-menu implementation until A7 commits. `STATUS=PASS_WITH_DEBT` reflects D1 only.

---

## 11. State & responsive coverage (R3 global constraints)

- **Sizes:** 1920×1080, 1440×900, 1366×768, 1024×720 — menu opens within viewport; palette is a centered modal, width ≤ 520px, scrollable list.
- **Empty:** no selection → menu shows scope-appropriate root actions only (e.g. `tree.newFileHere` enabled, `file.rename` disabled with reason "未选择对象").
- **Loading:** async commands (`db.refresh`, `git.fetch`) show spinner; menu item disabled with reason "操作中…".
- **Error:** command throws → toast + audit `outcome=failed`; menu stays open for retry where safe.
- **Disabled:** greyed item + `title=disabledReason` (§3 rule).
- **Context-menu:** scoped to pointer object (§5), closes on outside click / `Esc` / execute.
- **Accessibility:** each item is a real `<button>`/role=`menuitem`, `aria-disabled`, focus moves into menu on open, `↑/↓` navigate, `Enter` executes, `Esc` closes and returns focus to origin (mirrors A6 W17 `ActivityBar` roving-focus contract).

---

## 12. Implementation hand-off (W19, no code here)

| Concern | Proposed destination (spec) | Owner |
|---|---|---|
| Registry store + `CommandDef` type | `src/stores/useCommandRegistry.ts`, `src/types.ts` | S1 |
| Menu projection component | new `src/components/shared/ContextMenu.vue` (replaces inline `FilePanel.vue:68-98`) | S1 |
| Palette component | new `src/components/shared/CommandPalette.vue` | S1 |
| Keymap → registry binding | extend `src/stores/useSettingsStore.ts` | S1 |
| Per-scope action rows | domain stores (`useWorkspaceStore`, `useDatabaseStore`, `useGraphStore`, future `useGitStore`) register `CommandDef`s | S2/S3/S4 |
| Git ids | A7 Rebased map → `useGitStore` | S2 (after A7) |
| Confirm tiers + audit | `src/composables/useConfirm.ts`, `AuditPanel.vue` feed | A9 reconcile |

This is a research contract; no file above is edited in this lane.

---

## 13. Self-check (structural, no runtime)

- [x] Every scope from the R3 A6 card (§1 line 69-71) is inventoried (§4).
- [x] Every context action has a registry identity, disabled reason hook, keyboard/palette path, safety class (§3 schema).
- [x] Right-click scoped to object under pointer, not catch-all (§5, G7).
- [x] Tree bulk ops bounded, `expandAll` rejected (§6).
- [x] Destructive confirmation tiers defined (§7).
- [x] Audit class + redaction rule defined (§8).
- [x] Keyboard + palette parity matrix present (§9).
- [x] State/responsive coverage per R3 global constraints (§11).
- [x] References classified COPY/ADAPT/REIMPLEMENT/DEFER/REJECT (§10).
- [x] No product source/manifest/ACL/capability/vault changed (this lane).

**Open for peer lanes:** D1 (A7 Git ids), A9 safety/audit gate reconciliation, A3 tool-window focus model, A1 consolidated prototype consumes this contract.
