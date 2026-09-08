// A6 · M5-W18-R3B — Synthetic Command Registry fixture (RESEARCH ARTIFACT ONLY)
//
// This module is the single machine-readable source of truth for the A6 R3B
// "command registry, shortcuts and accessibility" closure. It is NOT product
// code: nothing in src/ or src-tauri/ imports it. It exists so that
// `A6-R3B-registry-audit.mjs` can mechanically verify the R3B A6 acceptance
// criteria (frozen shortcuts, stable ids, scoped menus, disabled reasons,
// keyboard/accessible paths, safety classes, no toolbar-only expansion).
//
// References:
//   - M5-W18-R3B-CORRECTION-TASKS-20260908.md  (Lane A6 assignment)
//   - logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md  (rulings 41, 43)
//   - A6-R3-context-menu-command-registry.md (prior R3 contract, §3/§4/§9)

// ---------------------------------------------------------------------------
// Scope taxonomy (object-typed; a context menu is scoped to the pointer object)
// ---------------------------------------------------------------------------
export const SCOPE_PREFIXES = [
  "browser", "workspace", "note", "link", "graph", "db", "git", "terminal",
  "toolwindow", "app",
];

// ---------------------------------------------------------------------------
// Frozen global shortcuts (A0 ruling 41)
//   Ctrl+K      -> focus the browser address / search field
//   Ctrl+Shift+P-> open the command palette
// No other command may claim these two chords.
// ---------------------------------------------------------------------------
export const SHORTCUTS = {
  "Ctrl+K": "browser.focusAddressSearch",
  "Ctrl+Shift+P": "app.openCommandPalette",
};

// Allowed values for the invariant fields.
export const SAFETY_CLASSES = new Set(["safe", "mutating", "destructive", "dangerous"]);
export const AUDIT_CLASSES = new Set(["none", "ui", "data-read", "data-write", "credential", "admin"]);
export const CONFIRM_TIERS = new Set(["T0", "T1", "T2", "T3"]);
export const REACHABLE_PATHS = new Set(["shortcut", "palette", "menu", "keyboard"]);

// ---------------------------------------------------------------------------
// Command factory. Every field the audit checks is mandatory here.
//   - menu: a *scoped* placement path (must start with a scope prefix)
//   - disabledReason: null => always enabled; otherwise a stable reason token
//   - reachable: at least one of shortcut/palette/menu/keyboard (never toolbar-only)
// ---------------------------------------------------------------------------
function cmd(id, scope, group, label, opts = {}) {
  const shortcut = opts.shortcut ?? null;
  const reachable = opts.reachable ?? (shortcut ? ["shortcut", "palette", "menu"] : ["palette", "menu"]);
  return {
    id,
    scope,
    group,
    label,
    shortcut,
    paletteVisible: opts.paletteVisible ?? true,
    safetyClass: opts.safetyClass ?? "safe",
    auditClass: opts.auditClass ?? "none",
    confirmTier: opts.confirmTier ?? "T0",
    menu: opts.menu, // required, scoped placement
    disabledReason: opts.disabledReason ?? null,
    accessible: {
      ariaRole: "menuitem",
      focusReturn: opts.focusReturn ?? true,
    },
    reachable,
    toolbarOnly: false, // hard rule: no permanent toolbar-button expansion
  };
}

// ---------------------------------------------------------------------------
// 1. SHELL — browser tab/page
// ---------------------------------------------------------------------------
const browser = [
  cmd("browser.focusAddressSearch", "browser.page", "view", "Focus Address / Search", {
    shortcut: "Ctrl+K", menu: "browser.addressBar", safetyClass: "safe",
    disabledReason: null, auditClass: "none",
  }),
  cmd("tab.new", "browser.tab", "tab", "New Tab", { menu: "browser.tab", safetyClass: "safe" }),
  cmd("tab.close", "browser.tab", "tab", "Close Tab", { shortcut: "Ctrl+W", menu: "browser.tab", safetyClass: "safe" }),
  cmd("tab.closeOthers", "browser.tab", "tab", "Close Others", { menu: "browser.tab", safetyClass: "safe", disabledReason: "single-tab" }),
  cmd("tab.closeRight", "browser.tab", "tab", "Close Tabs to the Right", { menu: "browser.tab", safetyClass: "safe", disabledReason: "last-tab" }),
  cmd("tab.pin", "browser.tab", "tab", "Pin Tab", { menu: "browser.tab", safetyClass: "safe" }),
  cmd("tab.reload", "browser.tab", "tab", "Reload", { menu: "browser.tab", safetyClass: "safe" }),
  cmd("tab.duplicate", "browser.tab", "tab", "Duplicate", { menu: "browser.tab", safetyClass: "safe" }),
  cmd("tab.bookmark", "browser.tab", "tab", "Bookmark Page", { menu: "browser.tab", safetyClass: "safe" }),
  cmd("page.copyUrl", "browser.page", "page", "Copy URL", { menu: "browser.page", safetyClass: "safe" }),
  cmd("page.openInNewTab", "browser.page", "page", "Open in New Tab", { menu: "browser.page", safetyClass: "safe" }),
  cmd("page.findInPage", "browser.page", "page", "Find in Page", { menu: "browser.page", safetyClass: "safe" }),
  cmd("page.print", "browser.page", "page", "Print", { menu: "browser.page", safetyClass: "safe" }),
  cmd("page.screenshot", "browser.page", "page", "Screenshot", { menu: "browser.page", safetyClass: "safe" }),
  cmd("page.collectSelection", "browser.page", "page", "Collect Selection", { menu: "browser.page", safetyClass: "safe", auditClass: "ui" }),
];

// ---------------------------------------------------------------------------
// 2. SHELL — workspace file / tree
// ---------------------------------------------------------------------------
const workspace = [
  cmd("file.newFile", "workspace.file", "file", "New File", { menu: "workspace.file", safetyClass: "safe" }),
  cmd("file.newDir", "workspace.file", "file", "New Directory", { menu: "workspace.file", safetyClass: "safe" }),
  cmd("file.rename", "workspace.file", "file", "Rename", { shortcut: "F2", menu: "workspace.file", safetyClass: "mutating", confirmTier: "T1" }),
  cmd("file.delete", "workspace.file", "file", "Delete", { menu: "workspace.file", safetyClass: "destructive", confirmTier: "T2", auditClass: "data-write" }),
  cmd("file.copyPath", "workspace.file", "file", "Copy Path", { menu: "workspace.file", safetyClass: "safe" }),
  cmd("file.reveal", "workspace.file", "file", "Reveal in File Manager", { menu: "workspace.file", safetyClass: "safe" }),
  cmd("file.openExternal", "workspace.file", "file", "Open With…", { menu: "workspace.file", safetyClass: "safe" }),
  cmd("tree.collapseAll", "workspace.tree", "tree", "Collapse All", { shortcut: "Ctrl+Shift+-", menu: "workspace.tree", safetyClass: "safe" }),
  cmd("tree.expandOneLevel", "workspace.tree", "tree", "Expand One Level", { shortcut: "Ctrl+Shift++", menu: "workspace.tree", safetyClass: "safe" }),
  cmd("tree.expandAllBounded", "workspace.tree", "tree", "Expand to Bounded Depth", { shortcut: "Ctrl+Alt+Shift+E", menu: "workspace.tree", safetyClass: "safe", disabledReason: null }),
  cmd("tree.refresh", "workspace.tree", "tree", "Refresh", { shortcut: "F5", menu: "workspace.tree", safetyClass: "safe" }),
  cmd("tree.newFileHere", "workspace.tree", "tree", "New File Here", { menu: "workspace.tree", safetyClass: "safe", disabledReason: "no-scope" }),
  cmd("tree.newDirHere", "workspace.tree", "tree", "New Directory Here", { menu: "workspace.tree", safetyClass: "safe", disabledReason: "no-scope" }),
];

// ---------------------------------------------------------------------------
// 3. SHELL — terminal + tool window
// ---------------------------------------------------------------------------
const terminal = [
  cmd("term.newTab", "terminal", "terminal", "New Terminal", { menu: "terminal.pane", safetyClass: "safe" }),
  cmd("term.close", "terminal", "terminal", "Close Terminal", { menu: "terminal.pane", safetyClass: "safe" }),
  cmd("term.clear", "terminal", "terminal", "Clear", { shortcut: "Ctrl+L", menu: "terminal.pane", safetyClass: "safe" }),
  cmd("term.copy", "terminal", "terminal", "Copy", { menu: "terminal.pane", safetyClass: "safe" }),
  cmd("term.paste", "terminal", "terminal", "Paste", { menu: "terminal.pane", safetyClass: "safe" }),
  cmd("term.sendEof", "terminal", "terminal", "Send EOF", { menu: "terminal.pane", safetyClass: "safe" }),
  cmd("term.search", "terminal", "terminal", "Search", { menu: "terminal.pane", safetyClass: "safe" }),
];

const toolwindow = [
  cmd("tw.open", "toolwindow", "toolwindow", "Open Tool Window", { menu: "toolwindow.strip", safetyClass: "safe" }),
  cmd("tw.close", "toolwindow", "toolwindow", "Close Tool Window", { menu: "toolwindow.strip", safetyClass: "safe" }),
  cmd("tw.collapseAll", "toolwindow", "toolwindow", "Collapse All Windows", { shortcut: "Ctrl+K Ctrl+J", menu: "toolwindow.strip", safetyClass: "safe" }),
  cmd("tw.restoreLayout", "toolwindow", "toolwindow", "Restore Layout", { menu: "toolwindow.strip", safetyClass: "safe" }),
  cmd("tw.maximizeActive", "toolwindow", "toolwindow", "Maximize Active", { menu: "toolwindow.strip", safetyClass: "safe" }),
  cmd("tw.pin", "toolwindow", "toolwindow", "Pin", { menu: "toolwindow.strip", safetyClass: "safe" }),
  cmd("tw.unpin", "toolwindow", "toolwindow", "Unpin", { menu: "toolwindow.strip", safetyClass: "safe" }),
  cmd("tw.autoHide", "toolwindow", "toolwindow", "Auto Hide", { menu: "toolwindow.strip", safetyClass: "safe" }),
];

// ---------------------------------------------------------------------------
// 4. DATABASE (consumes A4/A5 contracts; cancellation maps to A4/A6)
// ---------------------------------------------------------------------------
const database = [
  cmd("db.connect", "db.connection", "db", "Connect", { menu: "db.connection", safetyClass: "safe", auditClass: "credential" }),
  cmd("db.disconnect", "db.connection", "db", "Disconnect", { menu: "db.connection", safetyClass: "safe", auditClass: "credential" }),
  cmd("db.refresh", "db.connection", "db", "Refresh", { menu: "db.connection", safetyClass: "safe" }),
  cmd("db.properties", "db.connection", "db", "Connection Properties", { menu: "db.connection", safetyClass: "safe" }),
  cmd("db.openConsole", "db.schema", "db", "Open SQL Console", { menu: "db.schema", safetyClass: "safe" }),
  cmd("db.viewData", "db.table", "db", "View Data", { menu: "db.table", safetyClass: "safe" }),
  cmd("db.copyName", "db.table", "db", "Copy Name", { menu: "db.table", safetyClass: "safe" }),
  cmd("db.truncate", "db.table", "db", "Truncate Table", { menu: "db.table", safetyClass: "dangerous", confirmTier: "T3", auditClass: "data-write" }),
  cmd("db.drop", "db.table", "db", "Drop Table", { menu: "db.table", safetyClass: "dangerous", confirmTier: "T3", auditClass: "admin" }),
  cmd("db.cell.copy", "db.cell", "db", "Copy Cell", { menu: "db.cell", safetyClass: "safe" }),
  cmd("db.cell.edit", "db.cell", "db", "Edit Cell", { menu: "db.cell", safetyClass: "mutating", confirmTier: "T1", auditClass: "data-write" }),
  cmd("db.cell.null", "db.cell", "db", "Set NULL", { menu: "db.cell", safetyClass: "mutating", confirmTier: "T2", auditClass: "data-write" }),
  cmd("db.result.export", "db.result", "db", "Export Result", { menu: "db.result", safetyClass: "safe", auditClass: "data-read" }),
  cmd("db.result.copyRow", "db.result", "db", "Copy Row", { menu: "db.result", safetyClass: "safe" }),
  cmd("db.result.cancel", "db.result", "db", "Cancel Query", { menu: "db.result", safetyClass: "safe", disabledReason: null }),
  cmd("db.result.truncateToggle", "db.result", "db", "Toggle Truncation", { menu: "db.result", safetyClass: "safe" }),
];

// ---------------------------------------------------------------------------
// 5. KNOWLEDGE — note / link / graph node (consumes A2/A3 semantics)
// ---------------------------------------------------------------------------
const knowledge = [
  cmd("note.open", "note", "note", "Open Note", { menu: "note.editor", safetyClass: "safe" }),
  cmd("note.openInNew", "note", "note", "Open in New Pane", { menu: "note.editor", safetyClass: "safe" }),
  cmd("note.rename", "note", "note", "Rename Note", { menu: "note.editor", safetyClass: "mutating", confirmTier: "T1" }),
  cmd("note.delete", "note", "note", "Delete Note", { menu: "note.editor", safetyClass: "destructive", confirmTier: "T2", auditClass: "data-write" }),
  cmd("note.copyLink", "note", "note", "Copy Link", { menu: "note.editor", safetyClass: "safe" }),
  cmd("note.backlinks", "note", "note", "Show Backlinks", { menu: "note.editor", safetyClass: "safe" }),
  cmd("note.localGraph", "note", "note", "Local Graph", { menu: "note.editor", safetyClass: "safe" }),
  cmd("link.follow", "link", "link", "Follow Link", { menu: "link.editor", safetyClass: "safe" }),
  cmd("link.copy", "link", "link", "Copy Link", { menu: "link.editor", safetyClass: "safe" }),
  cmd("link.unresolvedCreate", "link", "link", "Create Resolved Note", { menu: "link.editor", safetyClass: "mutating", confirmTier: "T1", disabledReason: "resolved" }),
  cmd("graph.node.open", "graph.node", "graph", "Open Node", { menu: "graph.node", safetyClass: "safe" }),
  cmd("graph.node.localGraph", "graph.node", "graph", "Local Graph", { menu: "graph.node", safetyClass: "safe" }),
  cmd("graph.node.expandNeighbors", "graph.node", "graph", "Expand Neighbors", { menu: "graph.node", safetyClass: "safe", disabledReason: null }),
  cmd("graph.node.hide", "graph.node", "graph", "Hide Node", { menu: "graph.node", safetyClass: "safe" }),
  cmd("graph.node.delete", "graph.node", "graph", "Delete Node", { menu: "graph.node", safetyClass: "destructive", confirmTier: "T2", disabledReason: "system-node", auditClass: "data-write" }),
];

// ---------------------------------------------------------------------------
// 6. GIT — 14 workflow units (A0 ruling 43 classification)
//    ADAPT_PRODUCT=4 : status, diff, branches, merge
//    REIMPLEMENT_FROM_BEHAVIOR=10 : hunk staging, log graph, worktrees, stash,
//      rebase/I-rebase, cherry-pick, conflicts, history/blame, patch, command log
//    Plus the explicitly required visible units: amend, reset/revert.
// ---------------------------------------------------------------------------
const git = [
  // ADAPT_PRODUCT
  cmd("git.status", "git.repo", "git", "Status", { menu: "git.repo", safetyClass: "safe", auditClass: "data-read" }),
  cmd("git.diff", "git.file", "git", "Diff", { menu: "git.file", safetyClass: "safe", auditClass: "data-read" }),
  cmd("git.branches", "git.repo", "git", "Branches", { menu: "git.repo", safetyClass: "safe" }),
  cmd("git.branchNew", "git.branch", "git", "New Branch", { menu: "git.branch", safetyClass: "mutating", confirmTier: "T1" }),
  cmd("git.branchCheckout", "git.branch", "git", "Checkout Branch", { menu: "git.branch", safetyClass: "mutating", confirmTier: "T1", disabledReason: "dirty-tree" }),
  cmd("git.branchDelete", "git.branch", "git", "Delete Branch", { menu: "git.branch", safetyClass: "destructive", confirmTier: "T2", disabledReason: "protected-branch", auditClass: "data-write" }),
  cmd("git.merge", "git.branch", "git", "Merge Into Current", { menu: "git.branch", safetyClass: "mutating", confirmTier: "T2", disabledReason: "uncommitted", auditClass: "data-write" }),
  // REIMPLEMENT_FROM_BEHAVIOR
  cmd("git.hunkStage", "git.hunk", "git", "Stage Hunk", { menu: "git.hunk", safetyClass: "mutating", confirmTier: "T1", auditClass: "data-write" }),
  cmd("git.hunkUnstage", "git.hunk", "git", "Unstage Hunk", { menu: "git.hunk", safetyClass: "mutating", confirmTier: "T1" }),
  cmd("git.hunkDiscard", "git.hunk", "git", "Discard Hunk", { menu: "git.hunk", safetyClass: "destructive", confirmTier: "T2", auditClass: "data-write" }),
  cmd("git.logGraph", "git.repo", "git", "Log Graph", { menu: "git.repo", safetyClass: "safe", auditClass: "data-read" }),
  cmd("git.worktree", "git.repo", "git", "Worktrees", { menu: "git.repo", safetyClass: "safe" }),
  cmd("git.worktreeNew", "git.repo", "git", "New Worktree", { menu: "git.repo", safetyClass: "mutating", confirmTier: "T1" }),
  cmd("git.stash", "git.repo", "git", "Stash", { menu: "git.repo", safetyClass: "mutating", confirmTier: "T1", disabledReason: "clean-tree", auditClass: "data-write" }),
  cmd("git.stashPop", "git.repo", "git", "Pop Stash", { menu: "git.repo", safetyClass: "mutating", confirmTier: "T1", disabledReason: "no-stash" }),
  cmd("git.stashDrop", "git.repo", "git", "Drop Stash", { menu: "git.repo", safetyClass: "destructive", confirmTier: "T2", disabledReason: "no-stash", auditClass: "data-write" }),
  cmd("git.rebase", "git.branch", "git", "Rebase", { menu: "git.branch", safetyClass: "destructive", confirmTier: "T3", disabledReason: "dirty-tree", auditClass: "data-write" }),
  cmd("git.rebaseContinue", "git.branch", "git", "Continue Rebase", { menu: "git.branch", safetyClass: "mutating", confirmTier: "T1", disabledReason: "not-rebasing" }),
  cmd("git.rebaseAbort", "git.branch", "git", "Abort Rebase", { menu: "git.branch", safetyClass: "mutating", confirmTier: "T1", disabledReason: "not-rebasing" }),
  cmd("git.cherryPick", "git.commit", "git", "Cherry-Pick", { menu: "git.commit", safetyClass: "mutating", confirmTier: "T1", disabledReason: "no-selection", auditClass: "data-write" }),
  cmd("git.cherryPickContinue", "git.commit", "git", "Continue Cherry-Pick", { menu: "git.commit", safetyClass: "mutating", confirmTier: "T1", disabledReason: "not-cherry-picking" }),
  cmd("git.cherryPickAbort", "git.commit", "git", "Abort Cherry-Pick", { menu: "git.commit", safetyClass: "mutating", confirmTier: "T1", disabledReason: "not-cherry-picking" }),
  cmd("git.conflicts", "git.file", "git", "Resolve Conflicts", { menu: "git.file", safetyClass: "mutating", confirmTier: "T2", disabledReason: "no-conflict", auditClass: "data-write" }),
  cmd("git.conflictResolve", "git.file", "git", "Mark Resolved", { menu: "git.file", safetyClass: "mutating", confirmTier: "T1", disabledReason: "no-conflict" }),
  cmd("git.history", "git.commit", "git", "History", { menu: "git.commit", safetyClass: "safe", auditClass: "data-read" }),
  cmd("git.blame", "git.file", "git", "Blame", { menu: "git.file", safetyClass: "safe", auditClass: "data-read" }),
  cmd("git.patch", "git.repo", "git", "Create Patch", { menu: "git.repo", safetyClass: "safe", auditClass: "data-read" }),
  cmd("git.patchApply", "git.repo", "git", "Apply Patch", { menu: "git.repo", safetyClass: "mutating", confirmTier: "T2", disabledReason: "no-patch", auditClass: "data-write" }),
  cmd("git.commandLog", "git.repo", "git", "Command Log", { menu: "git.repo", safetyClass: "safe", auditClass: "ui" }),
  // Explicitly required visible units (amend, reset/revert)
  cmd("git.commitAmend", "git.commit", "git", "Amend Commit", { menu: "git.commit", safetyClass: "destructive", confirmTier: "T3", disabledReason: "no-commit", auditClass: "data-write" }),
  cmd("git.commitReset", "git.commit", "git", "Reset Commit", { menu: "git.commit", safetyClass: "dangerous", confirmTier: "T3", disabledReason: "protected-branch", auditClass: "admin" }),
  cmd("git.commitRevert", "git.commit", "git", "Revert Commit", { menu: "git.commit", safetyClass: "mutating", confirmTier: "T2", disabledReason: "no-commit", auditClass: "data-write" }),
];

// ---------------------------------------------------------------------------
// 7. App-level — command palette (frozen Ctrl+Shift+P)
// ---------------------------------------------------------------------------
const app = [
  cmd("app.openCommandPalette", "app", "app", "Command Palette", {
    shortcut: "Ctrl+Shift+P", menu: "app.commandPalette", safetyClass: "safe",
    disabledReason: null, auditClass: "none",
  }),
];

export const COMMANDS = [
  ...browser, ...workspace, ...terminal, ...toolwindow,
  ...database, ...knowledge, ...git, ...app,
];

// A0 ruling 43 — the 14 canonical workflow units and their primary command id.
export const GIT14 = {
  status: "git.status",
  diff: "git.diff",
  branches: "git.branches",
  merge: "git.merge",
  "hunk-staging": "git.hunkStage",
  "log-graph": "git.logGraph",
  worktrees: "git.worktree",
  stash: "git.stash",
  "rebase": "git.rebase",
  "cherry-pick": "git.cherryPick",
  conflicts: "git.conflicts",
  "history-blame": "git.history",
  patch: "git.patch",
  "command-log": "git.commandLog",
};

// The explicitly named "must be visibly located" units from the R3B A6/A1 card.
export const GIT_VISIBLE = [
  "git.worktree", "git.worktreeNew",
  "git.commitAmend", "git.commitReset", "git.commitRevert",
  "git.conflicts", "git.conflictResolve",
  "git.patch", "git.patchApply",
  "git.commandLog",
];
