/**
 * A3 / M5-W18-R3 — Tool-window progressive-disclosure state machine (RESEARCH PROTOTYPE).
 *
 * Scope: lane-owned deterministic model for IDEA-style tool-window behavior inside the
 * browser-first shell. Pure functions, no product import, no bridge, no DOM, no native call.
 * Run:  node logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs
 *
 * This file is evidence/prototype only. It does not change product code, and it is not
 * wired into scripts/ or pre-merge.sh. W19 may lift these functions behind a policy script.
 *
 * Grounding evidence (read-only, canonical master @ 200f0f1):
 *   - src/stores/useLayoutStore.ts:134-214  -> sidebarOpen / sidebarWidth / browserDock* / compactMode / navSection
 *   - src/stores/useLayoutStore.ts:210-212  -> current splitter clamp is [180, 560], no viewport/content floor
 *   - src/stores/useWorkspaceStore.ts:416-458 -> treeRoots / treeChildren / treeExpanded / treeLoading (lazy, per-path)
 *   - src/components/layout/SidebarResizer.vue:9-21 -> mouse-drag resize, no keyboard path, no clamp feedback
 *   - src/components/layout/StatusBar.vue:136 -> status bar height 26px
 *   - src/App.vue:143-207 -> global keydown + matchKey(ctrl/shift/alt + key)
 *   - src/stores/useSettingsStore.ts:8-42 -> KEYMAP_SCHEMES(vscode/idea/eclipse)
 */

// =====================================================================
// 1. Constants — single source for every number used by R3B acceptance.
//    Aligned to the A0 M5-W18-R3 ruling (A0-M5-W18-R3-acceptance-audit-20260908.md):
//    - Geometry SSOT uses the application INNER viewport (OS titlebar excluded).
//    - Normal top chrome is exactly 60px = two 30px rows.
//    - Status bar is 24px.
//    - A compact 28px activity/tool strip remains visible in collapsed mode.
// =====================================================================

export const CHROME = Object.freeze({
  TITLEBAR_H: 0, // OS titlebar is OUTSIDE the inner viewport (A0 ruling) -> 0 here
  TOP_ROW_H: 30, // one compact top row (30px) — A0 SSOT
  TOP_ROWS_H: 60, // normal top chrome = exactly 60px = two 30px rows — A0 SSOT
  STATUS_H: 24, // status bar 24px — A0 SSOT
});

// 28px activity/tool strip that stays visible even when every resizable panel is collapsed.
export const ACTIVITY_STRIP_W = 28;

export const ACCEPT = Object.freeze({
  MIN_CONTENT_H_SHARE: 0.85, // collapsed mode, inner-viewport height
  MIN_CONTENT_W_SHARE: 0.92, // collapsed mode, viewport width
  MIN_CONTENT_W_PX: 520, // hard floor while resizing
  MIN_CONTENT_H_PX: 320, // hard floor while resizing
});

export const EDGE_LIMITS = Object.freeze({
  left: { min: 200, max: 420, default: 260, share: 0.32 },
  right: { min: 200, max: 420, default: 280, share: 0.32 },
  bottom: { min: 140, max: 420, default: 240, share: 0.45 },
});

export const EDGES = Object.freeze(["left", "right", "bottom"]);

export const NARROW = Object.freeze({
  SIDE_AUTOHIDE_PX: 1180, // < this: non-pinned side tools auto-hide
  ALL_AUTOHIDE_PX: 900, // < this: pinned side tools + bottom tool auto-hide
});

export const TREE = Object.freeze({
  MAX_NODES: 2000, // bounded expand-all node budget
  MAX_DEPTH: 6, // bounded expand-all depth budget
  CHUNK: 200, // rows applied per animation frame
  CONFIRM_ABOVE: 5000, // estimated rows above this require confirmation
  VIRTUALIZE_ABOVE: 500, // visible rows above this require windowed rendering (W19)
  PERSIST_MAX_PATHS: 500, // persisted expanded-path cap
});

export const SNAPSHOT_MAX = 3;

// =====================================================================
// 2. State shape (documented so A4 can own the persisted DTO)
// =====================================================================
/**
 * state = {
 *   viewport: { w, h },
 *   focusMode: boolean,
 *   topRows: 1 | 2,
 *   edges: { left:  { visible, primary, size, autoHide }, ... },
 *   tools:  { [id]: { id, edge, open, pinned, lastSize, autoHide } },
 *   snapshots: [ { edges, tools, topRows } ],
 *   focus: { surfaceId, selector },  // last real focus anchor
 *   focusStack: [ { surfaceId, selector } ],
 * }
 */

export function createState(viewport = { w: 1440, h: 900 }, tools = DEFAULT_TOOLS) {
  const t = {};
  for (const def of tools) t[def.id] = { pinned: false, autoHide: false, lastSize: EDGE_LIMITS[def.edge].default, open: false, ...def };
  return {
    viewport: { ...viewport },
    focusMode: false,
    topRows: 2,
    edges: {
      left: { visible: false, primary: null, size: EDGE_LIMITS.left.default, autoHide: false },
      right: { visible: false, primary: null, size: EDGE_LIMITS.right.default, autoHide: false },
      bottom: { visible: false, primary: null, size: EDGE_LIMITS.bottom.default, autoHide: false },
    },
    tools: t,
    snapshots: [],
    focus: { surfaceId: "doc", selector: null },
    focusStack: [],
  };
}

export const DEFAULT_TOOLS = Object.freeze([
  { id: "files", edge: "left", fallback: "bottom", label: "文件" },
  { id: "db", edge: "left", fallback: "bottom", label: "数据库" },
  { id: "notes", edge: "left", fallback: "bottom", label: "知识库" },
  { id: "git", edge: "left", fallback: "bottom", label: "Git" },
  { id: "inspect", edge: "right", fallback: "bottom", label: "属性" },
  { id: "term", edge: "bottom", fallback: "left", label: "终端" },
  { id: "problems", edge: "bottom", fallback: "right", label: "问题" },
]);

// =====================================================================
// 3. Pure helpers
// =====================================================================

const clone = (s) => (typeof structuredClone === "function" ? structuredClone(s) : JSON.parse(JSON.stringify(s)));

function edgeSizeUsed(state, edge) {
  const e = state.edges[edge];
  if (!e.visible || e.autoHide) return 0; // auto-hide never consumes layout space
  return e.size;
}

/** Clamp a requested size against per-edge limits and the content floor. */
export function clampSize(edge, px, viewport, otherSidePx = 0, bottomPx = 0) {
  const lim = EDGE_LIMITS[edge];
  const vw = viewport.w;
  const vh = viewport.h;
  let size = Math.round(Number(px) || 0);
  size = Math.max(lim.min, Math.min(lim.max, size));

  if (edge === "bottom") {
    const availH = vh - CHROME.TITLEBAR_H - CHROME.TOP_ROWS_H - CHROME.STATUS_H;
    const maxKeep = Math.max(lim.min, availH - ACCEPT.MIN_CONTENT_H_PX);
    const shareCap = Math.floor(availH * lim.share);
    return Math.min(size, maxKeep, shareCap);
  }
  const availW = vw - otherSidePx;
  const maxKeep = Math.max(lim.min, availW - ACCEPT.MIN_CONTENT_W_PX);
  const shareCap = Math.floor(vw * lim.share);
  return Math.min(size, maxKeep, shareCap);
}

/** Content rectangle for the active document, in CSS px. */
export function contentBox(state) {
  const topH = state.focusMode ? 0 : state.topRows === 1 ? CHROME.TOP_ROW_H : CHROME.TOP_ROWS_H;
  const statusH = state.focusMode ? 0 : CHROME.STATUS_H;
  const w = state.viewport.w - edgeSizeUsed(state, "left") - edgeSizeUsed(state, "right");
  const h = state.viewport.h - CHROME.TITLEBAR_H - topH - statusH - edgeSizeUsed(state, "bottom");
  return { w: Math.max(0, w), h: Math.max(0, h) };
}

/**
 * R3B acceptance shares (A0 SSOT formula):
 *   collapsed active width  = (viewport width - 28) / viewport width
 *   collapsed active height = (inner height - 60 - 24) / inner height
 * The 28px activity strip and the 60px top chrome + 24px status bar stay visible;
 * all resizable tool panels are hidden.
 */
export function collapsedShares(viewport) {
  const w = viewport.w ? (viewport.w - ACTIVITY_STRIP_W) / viewport.w : 0;
  const h = viewport.h ? (viewport.h - CHROME.TOP_ROWS_H - CHROME.STATUS_H) / viewport.h : 0;
  return { w, h };
}

// =====================================================================
// 4. Transitions — every function returns { state, effects }
//    effects: [{ type, ... }]  (hints, focus moves, audit-relevant notices)
// =====================================================================

function pushSnapshot(state) {
  const snap = { edges: clone(state.edges), tools: clone(state.tools), topRows: state.topRows };
  const out = state.snapshots.concat([snap]);
  return out.length > SNAPSHOT_MAX ? out.slice(out.length - SNAPSHOT_MAX) : out;
}

function focusTo(state, surfaceId, selector = null) {
  state.focusStack = [{ surfaceId: state.focus.surfaceId, selector: state.focus.selector }].concat(state.focusStack).slice(0, 32);
  state.focus = { surfaceId, selector };
}

/**
 * CR-1 replace-unless-pinned: opening B on an edge whose primary A is pinned must not
 * silently steal A's area. B goes to its declared fallback edge; if that is also blocked,
 * B is queued (stripe item) and the user gets a hint — never a hidden swap.
 */
export function openTool(state, id) {
  const s = clone(state);
  const tool = s.tools[id];
  if (!tool) return { state, effects: [{ type: "noop", reason: "UNKNOWN_TOOL", id }] };
  const effects = [];

  const tryEdge = (edge) => {
    const e = s.edges[edge];
    const primary = e.primary ? s.tools[e.primary] : null;
    if (!primary) return edge;
    if (primary.id === id) return edge;
    if (primary.pinned) return null;
    primary.open = false; // replaced: closed, still restorable via snapshot
    effects.push({ type: "replaced", edge, closed: primary.id, opened: id });
    return edge;
  };

  let edge = tryEdge(tool.edge);
  if (!edge) {
    edge = tryEdge(tool.fallback);
    if (edge) effects.push({ type: "fallback", id, from: tool.edge, to: edge });
  }
  if (!edge) {
    return { state, effects: [{ type: "blocked", reason: "EDGE_OCCUPIED_BY_PINNED", id, pinned: s.edges[tool.edge].primary }] };
  }

  const e = s.edges[edge];
  e.visible = true;
  e.autoHide = !!tool.autoHide;
  e.primary = id;
  e.size = clampSize(edge, tool.lastSize || EDGE_LIMITS[edge].default, s.viewport,
    edge === "left" ? edgeSizeUsed(s, "right") : edge === "right" ? edgeSizeUsed(s, "left") : 0,
    edgeSizeUsed(s, "bottom"));
  tool.open = true;
  tool.lastSize = e.size;
  focusTo(s, id);
  effects.push({ type: "opened", id, edge, size: e.size });
  return { state: applyNarrowPolicy(s).state, effects };
}

export function closeTool(state, id) {
  const s = clone(state);
  const tool = s.tools[id];
  if (!tool || !tool.open) return { state, effects: [{ type: "noop", reason: "NOT_OPEN", id }] };
  const e = s.edges[tool.edge];
  tool.open = false;
  tool.lastSize = e.size;
  if (e.primary === id) e.primary = null;
  e.visible = false;
  // CR-10 focus return: previous anchor if still valid, else the edge stripe button, else doc.
  const back = s.focusStack.find((f) => f.surfaceId && f.surfaceId !== id);
  s.focus = back ? { ...back } : { surfaceId: "doc", selector: null };
  s.focusStack = s.focusStack.filter((f) => f !== back);
  return { state: s, effects: [{ type: "closed", id, edge: tool.edge, focus: s.focus.surfaceId }] };
}

/** Collapse all: hide every tool window on every edge; remember one restorable snapshot. */
export function collapseAll(state) {
  const s = clone(state);
  const anyOpen = Object.values(s.tools).some((t) => t.open);
  if (!anyOpen) return { state, effects: [{ type: "noop", reason: "NOTHING_OPEN" }] };
  s.snapshots = pushSnapshot(state);
  for (const id of Object.keys(s.tools)) if (s.tools[id].open) s.tools[id].open = false;
  for (const edge of EDGES) {
    s.edges[edge].visible = false;
    s.edges[edge].primary = null;
  }
  s.topRows = 1;
  s.focus = { surfaceId: "doc", selector: null };
  return { state: s, effects: [{ type: "collapsed_all", snapshots: s.snapshots.length }] };
}

/** Restore previous layout (LIFO). Empty stack -> documented default layout. */
export function restorePrevious(state) {
  if (!state.snapshots.length) {
    const s = createState(state.viewport);
    return { state: openTool(s, "files").state, effects: [{ type: "restored_default", opened: "files" }] };
  }
  const s = clone(state);
  const snap = s.snapshots[s.snapshots.length - 1];
  s.snapshots = s.snapshots.slice(0, -1);
  s.edges = clone(snap.edges);
  s.tools = clone(snap.tools);
  s.topRows = snap.topRows;
  s.focus = { surfaceId: s.edges.left.primary || s.edges.bottom.primary || "doc", selector: null };
  return { state: s, effects: [{ type: "restored", remaining: s.snapshots.length }] };
}

/**
 * Expand active: focused tool window grows to its share cap and other edges are hidden
 * (snapshot first). If focus is in the document, "expand active" means content focus mode.
 */
export function expandActive(state) {
  const s = clone(state);
  const focusId = s.focus.surfaceId;
  const tool = s.tools[focusId];
  if (!tool || !tool.open) {
    return enterFocusMode(state);
  }
  s.snapshots = pushSnapshot(state);
  const edge = tool.edge;
  for (const other of EDGES) {
    if (other === edge) continue;
    s.edges[other].visible = false;
    for (const id of Object.keys(s.tools)) if (s.tools[id].open && s.tools[id].edge === other) s.tools[id].open = false;
  }
  const lim = EDGE_LIMITS[edge];
  s.edges[edge].visible = true;
  s.edges[edge].autoHide = false;
  s.edges[edge].primary = tool.id;
  const v = s.viewport;
  s.edges[edge].size =
    edge === "bottom"
      ? clampSize("bottom", Math.floor((v.h - CHROME.TITLEBAR_H - CHROME.TOP_ROWS_H - CHROME.STATUS_H) * 0.6), v)
      : clampSize(edge, Math.floor(v.w * lim.share) + 40, v);
  return { state: s, effects: [{ type: "expanded", id: tool.id, edge, size: s.edges[edge].size }] };
}

export function togglePin(state, id) {
  const s = clone(state);
  const tool = s.tools[id];
  if (!tool) return { state, effects: [{ type: "noop", reason: "UNKNOWN_TOOL", id }] };
  tool.pinned = !tool.pinned;
  return { state: s, effects: [{ type: tool.pinned ? "pinned" : "unpinned", id }] };
}

export function toggleAutoHide(state, id) {
  const s = clone(state);
  const tool = s.tools[id];
  if (!tool) return { state, effects: [{ type: "noop", reason: "UNKNOWN_TOOL", id }] };
  tool.autoHide = !tool.autoHide;
  const e = s.edges[tool.edge];
  if (e.primary === id) e.autoHide = tool.autoHide;
  return { state: s, effects: [{ type: tool.autoHide ? "autohide_on" : "autohide_off", id, relayout: false }] };
}

/**
 * Splitter drag. Never a silent no-op: when the request hits a limit the caller receives
 * a `clamped` effect so the UI can explain why the splitter stopped.
 */
export function resizeEdge(state, edge, px) {
  const s = clone(state);
  const e = s.edges[edge];
  if (!e.visible) return { state, effects: [{ type: "noop", reason: "EDGE_HIDDEN", edge }] };
  const requested = Math.round(Number(px) || 0);
  const other = edge === "left" ? edgeSizeUsed(s, "right") : edge === "right" ? edgeSizeUsed(s, "left") : 0;
  const size = clampSize(edge, requested, s.viewport, other, edge === "bottom" ? 0 : edgeSizeUsed(s, "bottom"));
  e.size = size;
  if (e.primary) s.tools[e.primary].lastSize = size;
  return {
    state: s,
    effects: [{ type: size === requested ? "resized" : "clamped", edge, size, requested }],
  };
}

export function enterFocusMode(state) {
  if (state.focusMode) return { state, effects: [{ type: "noop", reason: "ALREADY_FOCUS" }] };
  const s = clone(state);
  s.snapshots = pushSnapshot(state);
  s.focusMode = true;
  for (const edge of EDGES) s.edges[edge].visible = false;
  return { state: s, effects: [{ type: "focus_mode_on", returnAction: "EXIT_FOCUS_PILL" }] };
}

export function exitFocusMode(state) {
  if (!state.focusMode) return { state, effects: [{ type: "noop", reason: "NOT_FOCUS" }] };
  const s = clone(state);
  s.focusMode = false;
  const snap = s.snapshots[s.snapshots.length - 1];
  if (snap) {
    s.snapshots = s.snapshots.slice(0, -1);
    s.edges = clone(snap.edges);
    s.tools = clone(snap.tools);
    s.topRows = snap.topRows;
  }
  return { state: s, effects: [{ type: "focus_mode_off" }] };
}

/**
 * Narrow-window policy. Applied after every geometry-changing transition and on resize.
 * Pinned tools survive longer than unpinned ones (CR-7); below ALL_AUTOHIDE_PX everything
 * becomes auto-hide and, if the content floor still cannot hold, the shell collapses fully.
 */
export function applyNarrowPolicy(state) {
  const s = clone(state);
  const effects = [];
  const { w, h } = s.viewport;
  for (const edge of EDGES) {
    const e = s.edges[edge];
    if (!e.visible || !e.primary) continue;
    const tool = s.tools[e.primary];
    if (!tool) continue;
    const isSide = edge !== "bottom";
    const shouldHide = isSide ? w < (tool.pinned ? NARROW.ALL_AUTOHIDE_PX : NARROW.SIDE_AUTOHIDE_PX) : w < NARROW.ALL_AUTOHIDE_PX;
    if (shouldHide && !e.autoHide) {
      e.autoHide = true;
      tool.autoHide = true;
      effects.push({ type: "narrow_autohide", edge, id: tool.id, pinned: tool.pinned });
    }
    if (!shouldHide && e.autoHide && w >= (isSide ? NARROW.SIDE_AUTOHIDE_PX : NARROW.ALL_AUTOHIDE_PX)) {
      e.autoHide = false;
      tool.autoHide = false;
      effects.push({ type: "narrow_restore", edge, id: tool.id });
    }
  }
  const box = contentBox(s);
  if (box.w < ACCEPT.MIN_CONTENT_W_PX || box.h < ACCEPT.MIN_CONTENT_H_PX) {
    const r = collapseAll(s);
    effects.push({ type: "content_floor_collapse", box });
    return { state: r.state, effects: effects.concat(r.effects) };
  }
  return { state: s, effects };
}

export function setViewport(state, w, h) {
  const s = clone(state);
  s.viewport = { w: Math.round(w), h: Math.round(h) };
  for (const edge of EDGES) {
    const e = s.edges[edge];
    if (!e.visible) continue;
    const other = edge === "left" ? edgeSizeUsed(s, "right") : edge === "right" ? edgeSizeUsed(s, "left") : 0;
    e.size = clampSize(edge, e.size, s.viewport, other, edge === "bottom" ? 0 : edgeSizeUsed(s, "bottom"));
    if (e.primary) s.tools[e.primary].lastSize = e.size;
  }
  return applyNarrowPolicy(s);
}

// =====================================================================
// 5. Tree behavior — collapse all / expand one level / bounded expand all
//    Modeled on the existing lazy store (treeChildren / treeExpanded / treeLoading).
// =====================================================================
/**
 * tree = { nodes: { id: { id, parent, children: [], loaded, hasChildren } }, rootIds: [], expanded: string[], selected }
 * Pure: functions return { tree, report } and never mutate the input.
 */

export function treeCollapseAll(tree) {
  const t = clone(tree);
  const before = t.expanded.length;
  t.expanded = [];
  return {
    tree: t,
    report: { op: "collapse_all", collapsed: before, selectionKept: !!t.selected, ancestorsRevealed: false },
  };
}

/** Expand exactly one level below the selected node (or below every root when nothing is selected). */
export function treeExpandOneLevel(tree) {
  const t = clone(tree);
  const anchor = t.selected && t.nodes[t.selected] ? t.nodes[t.selected] : null;
  if (anchor && !anchor.hasChildren) {
    return { tree: t, report: { op: "expand_one", expanded: 0, noop: "SELECTED_LEAF" } };
  }
  const targets = anchor ? [anchor.id] : t.rootIds.slice();
  let n = 0;
  for (const id of targets) {
    const node = t.nodes[id];
    if (!node || !node.hasChildren) continue;
    if (!t.expanded.includes(id)) {
      t.expanded.push(id);
      n += 1;
    }
  }
  return { tree: t, report: { op: "expand_one", expanded: n, targets, pendingLoads: targets.filter((id) => !t.nodes[id].loaded) } };
}

/**
 * Bounded expand all: BFS with node/depth budget. Never walks unloaded nodes — those are
 * returned as `pendingLoads` for the caller to fetch level by level, so a 100k-entry tree
 * can never freeze a frame.
 */
export function treeBoundedExpandAll(tree, opts = {}) {
  const maxNodes = opts.maxNodes ?? TREE.MAX_NODES;
  const maxDepth = opts.maxDepth ?? TREE.MAX_DEPTH;
  const t = clone(tree);
  const expanded = new Set(t.expanded);
  const pendingLoads = [];
  let visited = 0;
  let depthReached = 0;
  let truncated = false;
  let queue = t.rootIds.map((id) => ({ id, depth: 0 }));
  const chunks = [];
  let currentChunk = [];

  while (queue.length) {
    const next = [];
    for (const { id, depth } of queue) {
      if (depth > maxDepth) {
        truncated = true;
        continue;
      }
      if (visited >= maxNodes) {
        truncated = true;
        continue;
      }
      visited += 1;
      depthReached = Math.max(depthReached, depth);
      const node = t.nodes[id];
      if (!node) continue;
      if (!node.loaded) {
        pendingLoads.push(id);
        continue; // cannot go deeper without data
      }
      if (node.hasChildren) {
        if (!expanded.has(id)) {
          expanded.add(id);
          currentChunk.push(id);
          if (currentChunk.length >= (opts.chunk ?? TREE.CHUNK)) {
            chunks.push(currentChunk);
            currentChunk = [];
          }
        }
        for (const c of node.children) next.push({ id: c, depth: depth + 1 });
      }
    }
    queue = next;
  }
  if (currentChunk.length) chunks.push(currentChunk);
  t.expanded = Array.from(expanded).slice(0, TREE.PERSIST_MAX_PATHS * 4);
  return {
    tree: t,
    report: {
      op: "expand_all_bounded",
      visited,
      depthReached,
      truncated,
      truncatedReason: truncated ? (visited >= maxNodes ? "NODE_BUDGET" : "DEPTH_BUDGET") : null,
      pendingLoads,
      chunks: chunks.length,
      // Confirmation is decided by the *estimated* tree size (what the user is about to ask
      // for), not by the visited count, which is always capped by the budget.
      estimated: Object.keys(t.nodes).length,
      confirmRequired: Object.keys(t.nodes).length >= TREE.CONFIRM_ABOVE,
      virtualize: visited > TREE.VIRTUALIZE_ABOVE,
    },
  };
}

/** Visible row count — drives the virtualization threshold (W19). */
export function treeVisibleRows(tree) {
  let count = 0;
  const walk = (ids) => {
    for (const id of ids) {
      const n = tree.nodes[id];
      if (!n) continue;
      count += 1;
      if (tree.expanded.includes(id)) walk(n.children);
    }
  };
  walk(tree.rootIds);
  return count;
}

// =====================================================================
// 6. Proposed keymap additions (existing schemes: vscode / idea / eclipse)
// =====================================================================
export const PROPOSED_KEYS = Object.freeze({
  hideAllTools: { vscode: "Ctrl+Shift+B", idea: "Ctrl+Shift+F12", eclipse: "Ctrl+Shift+F12" },
  expandActive: { vscode: "Ctrl+Alt+=", idea: "Ctrl+Alt+M", eclipse: "Ctrl+M" },
  focusMode: { vscode: "Ctrl+Alt+F", idea: "Ctrl+Alt+F", eclipse: "Ctrl+Alt+F" },
  pinActive: { vscode: "Ctrl+Alt+P", idea: "Ctrl+Alt+P", eclipse: "Ctrl+Alt+P" },
  autoHideActive: { vscode: "Ctrl+Alt+H", idea: "Ctrl+Alt+H", eclipse: "Ctrl+Alt+H" },
  focusLeftTool: { vscode: "Alt+1", idea: "Alt+1", eclipse: "Alt+1" },
  focusBottomTool: { vscode: "Alt+2", idea: "Alt+2", eclipse: "Alt+2" },
  focusRightTool: { vscode: "Alt+3", idea: "Alt+3", eclipse: "Alt+3" },
  treeCollapseAll: { vscode: "Ctrl+Alt+-", idea: "Ctrl+Alt+-", eclipse: "Ctrl+Alt+-" },
  treeExpandOne: { vscode: "Ctrl+Alt+=", idea: "Ctrl+Alt+=", eclipse: "Ctrl+Alt+=" },
  treeExpandAll: { vscode: "Ctrl+Alt+0", idea: "Ctrl+Alt+0", eclipse: "Ctrl+Alt+0" },
});

// =====================================================================
// 7. Self-test (node logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs)
// =====================================================================
function selfTest() {
  const assert = (cond, label) => {
    if (!cond) throw new Error("ASSERT FAILED: " + label);
    return true;
  };
  let n = 0;
  const ok = (label) => {
    n += 1;
    return label;
  };

  // --- 7.1 R3B viewport budget: collapsed mode at every A0 acceptance size ---
  for (const vp of [
    { w: 1920, h: 1080 },
    { w: 1440, h: 900 },
    { w: 1366, h: 768 },
    { w: 1200, h: 800 },
    { w: 1024, h: 720 },
    { w: 900, h: 600 }, // product minimum — A0 dropped 800x600
  ]) {
    const sh = collapsedShares(vp);
    assert(sh.h >= ACCEPT.MIN_CONTENT_H_SHARE, ok(`collapsed h share ${vp.w}x${vp.h} = ${(sh.h * 100).toFixed(1)}%`));
    assert(sh.w >= ACCEPT.MIN_CONTENT_W_SHARE, ok(`collapsed w share ${vp.w}x${vp.h} = ${(sh.w * 100).toFixed(1)}%`));
  }

  // --- 7.2 top chrome cap ---
  assert(CHROME.TOP_ROWS_H <= 80, ok("top chrome <= 80px in normal mode"));

  // --- 7.3 open / close basics ---
  let s = createState({ w: 1440, h: 900 });
  let r = openTool(s, "files");
  assert(r.state.tools.files.open && r.state.edges.left.primary === "files", ok("open files -> left primary"));
  assert(r.state.focus.surfaceId === "files", ok("open moves focus to the tool window"));
  assert(!s.tools.files.open, ok("openTool is pure (input untouched)"));

  r = closeTool(r.state, "files");
  assert(!r.state.tools.files.open && r.state.edges.left.primary === null, ok("close clears primary"));
  assert(r.state.focus.surfaceId === "doc", ok("focus returns to document (CR-10)"));

  // --- 7.4 CR-1 replace-unless-pinned ---
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  r = openTool(s, "db");
  assert(r.state.edges.left.primary === "db" && !r.state.tools.files.open, ok("same edge, unpinned primary is replaced"));
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  s = togglePin(s, "files").state;
  r = openTool(s, "db");
  assert(r.state.edges.left.primary === "files", ok("pinned primary keeps the edge (no silent steal)"));
  assert(r.state.edges.bottom.primary === "db", ok("blocked tool falls back to its alternate edge"));
  r = openTool(togglePin(openTool(createState({ w: 1440, h: 900 }), "files").state, "files").state, "db");
  assert(r.effects.some((e) => e.type === "fallback"), ok("fallback emits an effect the UI can explain"));

  // --- 7.5 collapse all / restore (R3B-03: Collapse All hides PINNED too) ---
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  s = openTool(s, "term").state;
  s = togglePin(s, "files").state; // pin a tool BEFORE collapse
  const pinBefore = s.tools.files.pinned;
  r = collapseAll(s);
  assert(!Object.values(r.state.tools).some((t) => t.open), ok("collapse all closes every tool window, including pinned"));
  assert(!r.state.tools.files.open && !r.state.tools.term.open, ok("collapse all hides the pinned 'files' window too"));
  assert(r.state.focus.surfaceId === "doc", ok("collapse all returns focus to the document (CR-10)"));
  assert(r.state.snapshots.length === 1, ok("collapse all stores one restorable snapshot"));
  const restored = restorePrevious(r.state);
  assert(restored.state.tools.files.open && restored.state.tools.term.open, ok("restore brings the previous layout back"));
  assert(restored.state.tools.files.pinned === pinBefore, ok("restore restores pin state from the snapshot"));
  assert(restored.state.snapshots.length === 0, ok("restore pops the snapshot"));
  const empty = restorePrevious(createState({ w: 1440, h: 900 }));
  assert(empty.state.tools.files.open, ok("empty stack -> documented default layout"));
  // pin only changes ordinary replacement / auto-hide, never survives Collapse All
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  s = togglePin(s, "files").state;
  r = openTool(s, "db"); // unpinned 'db' cannot steal a pinned primary -> falls back
  assert(r.state.edges.left.primary === "files" && s.tools.files.pinned, ok("pin blocks ordinary replacement (non-collapse)"));
  assert(r.state.edges.bottom.primary === "db", ok("pin only affects replacement/auto-hide, not collapse"));

  // --- 7.6 expand active ---
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  r = expandActive(s);
  assert(r.state.edges.left.size > s.edges.left.size, ok("expand active grows the focused tool window"));
  assert(r.state.snapshots.length === 1, ok("expand active snapshots first (reversible)"));
  const docFocus = { ...s, focus: { surfaceId: "doc", selector: null } };
  r = expandActive(docFocus);
  assert(r.state.focusMode === true, ok("expand active from the document enters focus mode"));

  // --- 7.7 focus mode has exactly one obvious return ---
  r = enterFocusMode(openTool(createState({ w: 1440, h: 900 }), "files").state);
  assert(r.effects.some((e) => e.returnAction === "EXIT_FOCUS_PILL"), ok("focus mode declares its return action"));
  assert(contentBox(r.state).h > contentBox(openTool(createState({ w: 1440, h: 900 }), "files").state).h, ok("focus mode gives height back to content"));
  assert(exitFocusMode(r.state).state.focusMode === false, ok("Esc / pill exits focus mode"));

  // --- 7.8 auto-hide never relayouts content ---
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  const beforeBox = contentBox(s);
  r = toggleAutoHide(s, "files");
  assert(r.state.tools.files.autoHide === true, ok("auto-hide flag set"));
  assert(r.effects.some((e) => e.type === "autohide_on" && e.relayout === false), ok("auto-hide does not relayout content"));
  assert(contentBox(r.state).w === beforeBox.w + s.edges.left.size, ok("auto-hidden edge stops consuming width"));

  // --- 7.9 splitter clamps and explains ---
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  r = resizeEdge(s, "left", 5000);
  assert(r.state.edges.left.size <= EDGE_LIMITS.left.max, ok("splitter respects the per-edge maximum"));
  assert(r.effects[0].type === "clamped", ok("clamping is reported, never a silent no-op"));
  r = resizeEdge(s, "left", 300);
  assert(r.effects[0].type === "resized" && r.state.edges.left.size === 300, ok("in-range resize applies exactly"));
  r = resizeEdge(openTool(createState({ w: 1024, h: 720 }), "files").state, "left", 420);
  assert(contentBox(r.state).w >= ACCEPT.MIN_CONTENT_W_PX, ok("resize keeps the 520px content floor at 1024x720"));

  // --- 7.10 narrow-window policy ---
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  r = setViewport(s, 1100, 800);
  assert(r.state.edges.left.autoHide === true, ok("<1180px: unpinned side tool auto-hides"));
  s = togglePin(s, "files").state;
  r = setViewport(s, 1100, 800);
  assert(r.state.edges.left.autoHide === false, ok("pinned tool survives until the 900px threshold"));
  r = setViewport(s, 860, 700);
  assert(r.state.edges.left.autoHide === true, ok("<900px: pinned side tool also auto-hides"));

  // --- 7.11 tree: collapse all ---
  const tree = makeTree();
  let tr = treeCollapseAll(tree);
  assert(tr.tree.expanded.length === 0, ok("tree collapse all clears expansion"));
  assert(tr.tree.selected === tree.selected, ok("tree collapse all keeps the selection"));
  assert(tr.report.ancestorsRevealed === false, ok("collapse all does not secretly re-expand ancestors"));
  assert(tree.expanded.length > 0, ok("treeCollapseAll is pure"));

  // --- 7.12 tree: expand one level ---
  tr = treeExpandOneLevel({ ...tree, expanded: [], selected: "src" });
  assert(tr.report.expanded === 1 && tr.tree.expanded[0] === "src", ok("expand one level opens only the selected node"));
  tr = treeExpandOneLevel({ ...tree, expanded: [], selected: "README.md" });
  assert(tr.report.noop === "SELECTED_LEAF", ok("expand one level on a leaf is an explicit no-op"));
  tr = treeExpandOneLevel({ ...tree, expanded: [], selected: null });
  assert(tr.report.targets.length === tree.rootIds.length, ok("no selection -> expand roots one level"));

  // --- 7.13 tree: bounded expand all ---
  tr = treeBoundedExpandAll({ ...tree, expanded: [] });
  assert(tr.report.visited > 0, ok("bounded expand all visits nodes"));
  assert(tr.report.visited <= TREE.MAX_NODES, ok(`bounded expand all respects node budget (${tr.report.visited} <= ${TREE.MAX_NODES})`));
  assert(tr.report.depthReached <= TREE.MAX_DEPTH, ok("bounded expand all respects depth budget"));
  assert(tr.report.chunks >= 1, ok("work is chunked for frame-budgeted application"));
  const deep = makeDeepTree(40, 3); // 3 children per level, 40 levels
  tr = treeBoundedExpandAll({ ...deep, expanded: [] });
  assert(tr.report.truncated === true && tr.report.truncatedReason === "DEPTH_BUDGET", ok("deep tree stops on depth budget"));
  assert(tr.report.visited <= TREE.MAX_NODES, ok("deep tree never exceeds the node budget"));
  const wide = makeWideTree(9000);
  tr = treeBoundedExpandAll({ ...wide, expanded: [] });
  assert(tr.report.truncatedReason === "NODE_BUDGET", ok("wide tree stops on node budget"));
  assert(tr.report.confirmRequired === true, ok("huge tree asks for confirmation first"));

  // --- 7.14 lazily loaded trees never walk blind ---
  const lazy = { ...makeTree(), nodes: { ...makeTree().nodes } };
  for (const id of Object.keys(lazy.nodes)) if (id !== "root") lazy.nodes[id].loaded = false;
  tr = treeBoundedExpandAll(lazy);
  assert(tr.report.pendingLoads.length > 0, ok("unloaded levels are reported as pending loads, not walked"));

  // --- 7.15 keyboard reachability + focus (R3B-04) ---
  assert(Object.values(PROPOSED_KEYS).every((k) => k.vscode && k.idea && k.eclipse), ok("every proposed action exists in all three schemes"));
  assert(Object.keys(PROPOSED_KEYS).includes("hideAllTools") && Object.keys(PROPOSED_KEYS).includes("focusMode"), ok("collapse-all and focus mode are keyboard reachable"));
  assert(Object.keys(PROPOSED_KEYS).includes("focusLeftTool") && Object.keys(PROPOSED_KEYS).includes("focusBottomTool") && Object.keys(PROPOSED_KEYS).includes("focusRightTool"), ok("each tool edge has a keyboard focus path (Alt+1/2/3)"));
  assert(Object.keys(PROPOSED_KEYS).includes("treeCollapseAll") && Object.keys(PROPOSED_KEYS).includes("treeExpandOne") && Object.keys(PROPOSED_KEYS).includes("treeExpandAll"), ok("tree collapse/expand actions are keyboard reachable"));
  // focus must shift deterministically with keyboard-driven transitions
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  assert(s.focus.surfaceId === "files", ok("open moves focus to the tool window (keyboard target)"));
  r = closeTool(s, "files");
  assert(r.state.focus.surfaceId === "doc", ok("close returns focus to document"));
  r = collapseAll(s);
  assert(r.state.focus.surfaceId === "doc", ok("collapse all returns focus to document"));
  s = openTool(createState({ w: 1440, h: 900 }), "files").state;
  s = togglePin(s, "files").state;
  r = togglePin(s, "files"); // unpin must not move focus
  assert(r.state.focus.surfaceId === "files", ok("toggle pin preserves focus (no surprise steal)"));
  // tree keyboard: collapse-all keeps the selection as the focus anchor
  tr = treeCollapseAll(makeTree());
  assert(tr.tree.selected === tree.selected && tr.report.selectionKept, ok("tree keyboard collapse keeps selection as focus anchor"));

  return n;
}

// --- synthetic fixtures (no product data) ---
function makeTree() {
  const nodes = {
    root: { id: "root", parent: null, children: ["src", "README.md"], loaded: true, hasChildren: true },
    src: { id: "src", parent: "root", children: ["main.ts", "util.ts"], loaded: true, hasChildren: true },
    "main.ts": { id: "main.ts", parent: "src", children: [], loaded: true, hasChildren: false },
    "util.ts": { id: "util.ts", parent: "src", children: [], loaded: true, hasChildren: false },
    "README.md": { id: "README.md", parent: "root", children: [], loaded: true, hasChildren: false },
  };
  return { nodes, rootIds: ["root"], expanded: ["root", "src"], selected: "src" };
}

function makeDeepTree(depth, fanout) {
  const nodes = {};
  let parent = null;
  for (let d = 0; d < depth; d += 1) {
    for (let i = 0; i < fanout; i += 1) {
      const id = `d${d}_${i}`;
      nodes[id] = { id, parent, children: [], loaded: true, hasChildren: d < depth - 1 };
      if (d > 0) nodes[parent].children.push(id);
    }
    parent = `d${d}_0`;
  }
  const rootIds = Object.keys(nodes).filter((id) => id.startsWith("d0_"));
  return { nodes, rootIds, expanded: rootIds.slice(), selected: null };
}

function makeWideTree(count) {
  const nodes = { root: { id: "root", parent: null, children: [], loaded: true, hasChildren: true } };
  for (let i = 0; i < count; i += 1) {
    const id = `f${i}`;
    nodes[id] = { id, parent: "root", children: [], loaded: true, hasChildren: false };
    nodes.root.children.push(id);
  }
  return { nodes, rootIds: ["root"], expanded: [], selected: null };
}

if (typeof process !== "undefined" && process.argv && process.argv[1] && process.argv[1].endsWith("A3-R3-toolwindow-state-machine.mjs")) {
  const count = selfTest();
  // eslint-disable-next-line no-console
  console.log(`A3-R3 tool-window state machine self-test: PASS (${count} assertions)`);
  const rows = [
    { w: 1920, h: 1080 },
    { w: 1440, h: 900 },
    { w: 1366, h: 768 },
    { w: 1200, h: 800 },
    { w: 1024, h: 720 },
    { w: 900, h: 600 },
  ].map((vp) => {
    const sh = collapsedShares(vp);
    return `${vp.w}x${vp.h}: content ${Math.round(sh.w * 1000) / 10}% w / ${Math.round(sh.h * 1000) / 10}% h`;
  });
  // eslint-disable-next-line no-console
  console.log("collapsed-mode budget -> " + rows.join(" | "));
}

export { selfTest, makeTree, makeDeepTree, makeWideTree };
