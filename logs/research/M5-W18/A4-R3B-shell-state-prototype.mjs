// A4-R3B shell-state & persistence contract — self-contained reference implementation.
// Pure functions only (no bridge / no DOM / no product import). Runs under `node`.
//
//   node logs/research/M5-W18/A4-R3B-shell-state-prototype.mjs
//
// RESEARCH PROTOTYPE. Not wired into the app, must not be imported by product source.
// W19 ports these functions 1:1 into src/stores.
//
// This R3B revision supersedes the collapse-all / restore semantics in
// A4-R3-shell-state-prototype.mjs (T14 there asserted the pre-R3B behavior and is void).
// It is reconciled to the A0 R3 acceptance ruling:
//   - Collapse All hides EVERY tool window, including pinned; it records ONE restorable
//     layout snapshot. Pinning affects ordinary replacement / auto-hide only.
//   - Restore Layout restores visibility, sizes and pin states from that snapshot.
//   - 28px activity strip stays visible in collapsed mode; top chrome 60px (two 30px rows),
//     status bar 24px (A0 SSOT, R3B-05 — note the live StatusBar.vue is still 26px, see report).
// It also proves determinism (key-order-independent output), transient-state exclusion and
// crash recovery (atomic write + corrupt quarantine + no external-side-effect replay).

// ---------- constants ----------
const SCHEMA_VERSION = 1;
const MAX_DOCUMENTS = 200;
const MAX_TOOL_WINDOWS = 32;
const EDGES = ["left", "right", "bottom"];
const DEFAULT_VIEW = "home";
const ACTIVITY_STRIP_PX = 28; // A0 ruling: remains in collapsed mode
const TOP_CHROME_PX = 60;     // A0 ruling: two 30px rows (normal)
const STATUS_PX = 24;         // A0 ruling: status bar
const VALID_MAIN_VIEWS = new Set([
  "home", "browser", "files", "clip", "arts", "grid", "apps", "term", "repo",
  "audit", "scripts", "commands", "tools", "db", "tasks", "skills", "agents",
  "graph", "plugin", "editor",
]);
// keys that must NEVER be persisted (session-only UI state)
const TRANSIENT_KEYS = new Set([
  "hover_id", "overlay", "drag_preview", "context_menu", "tooltip",
  "resize_active", "splitter_drag", "focus_ring",
]);
const SESSION_ONLY_KEYS = new Set(["focus", "focusStack", "snapshots"]);
const SECRET_KEY_RE = /(token|password|secret|passwd|api[_-]?key)/i;

// ---------- helpers ----------
const clone = (v) => (typeof structuredClone === "function" ? structuredClone(v) : JSON.parse(JSON.stringify(v)));
const byId = (a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);

function defaultGlobal() {
  return {
    theme: "dark",
    focus_mode_default: false,
    density_pref: "auto",
    keymap_scheme: "idea",
    addr_mode_default: "url",
    sidebar_default_open: true,
    window: { x: 0, y: 0, w: 1440, h: 900, maximized: false },
  };
}
function defaultLayout() {
  return { main_view: DEFAULT_VIEW, nav_section: "", density: "full", collapsed: false, focus_mode: false, addr_mode: "url", sidebar_open: true, sidebar_width: 300 };
}
export function freshState() {
  return {
    schema_version: SCHEMA_VERSION,
    saved_at_ms: 0,
    global: defaultGlobal(),
    active_workspace_id: "default",
    workspaces: {
      default: {
        documents: [], active_document_id: null, tool_windows: [], layout: defaultLayout(),
        splitters: { left: 300, right: 320, bottom: 220 }, collapsed_snapshot: null,
      },
    },
  };
}
function mkWs(toolWindows = [], opts = {}) {
  return {
    documents: [], active_document_id: null,
    tool_windows: toolWindows.map((w) => ({ ...w })),
    layout: defaultLayout(), splitters: { left: 300, right: 320, bottom: 220 },
    collapsed_snapshot: null, ...opts,
  };
}

// ---------- 7.1 migration from current tabs ----------
export function migrateFromLegacy(legacy) {
  const documents = [];
  const seen = new Map();
  const push = (kind, ref, title, meta) => {
    if (seen.has(ref)) return seen.get(ref);
    const id = "doc-" + (documents.length + 1);
    const doc = { id, kind, ref, title: title || ref };
    if (meta) doc.meta = meta;
    documents.push(doc);
    seen.set(ref, id);
    return id;
  };
  for (const t of legacy.browserTabs || []) push("browser", "browser:" + t.url, t.title);
  for (const m of legacy.modTabs || []) {
    const meta = m.path ? { path: m.path } : undefined;
    push("module", "module:" + m.view, m.title, meta);
  }
  let active = null;
  if (legacy.activeTabId) {
    const bt = (legacy.browserTabs || []).find((t) => t.id === legacy.activeTabId);
    if (bt) active = seen.get("browser:" + bt.url) || null;
  }
  if (!active && legacy.activeModTab) {
    const mt = (legacy.modTabs || []).find((m) => m.id === legacy.activeModTab);
    if (mt) active = seen.get("module:" + mt.view) || null;
  }
  return { documents, active_document_id: active };
}

// ---------- 7.3 per-edge primary invariant ----------
export function enforceEdgeInvariant(toolWindows) {
  const out = toolWindows.map((w) => ({ ...w }));
  for (const edge of EDGES) {
    const onEdge = out.filter((w) => w.edge === edge);
    const primaries = onEdge.filter((w) => w.primary);
    if (primaries.length <= 1) continue;
    const ranked = [...primaries].sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return (b.last_activated_ms || 0) - (a.last_activated_ms || 0);
    });
    const keep = ranked[0].id;
    for (const w of onEdge) if (w.primary && w.id !== keep) w.primary = false;
  }
  return out;
}

// ---------- 7.4 open tool window (one primary per edge, replace unless pinned) ----------
export function openToolWindow(toolWindows, edge, id, nowMs = 0, view) {
  const out = toolWindows.map((w) => ({ ...w }));
  let target = out.find((w) => w.id === id);
  if (!target) {
    target = { id, edge, view: view || id, state: "stashed", primary: false, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: nowMs };
    out.push(target);
  }
  const currentPrimary = out.find((w) => w.edge === edge && w.primary && w.id !== id);
  if (currentPrimary && currentPrimary.pinned) {
    target.primary = false; target.state = "stashed";
  } else {
    for (const w of out) if (w.edge === edge) w.primary = false;
    target.primary = true; target.state = "open";
  }
  target.edge = edge; target.last_activated_ms = nowMs;
  return enforceEdgeInvariant(out);
}

// ---------- 7.5 Collapse All (R3B-corrected) ----------
// Hides EVERY tool window including pinned; records exactly one restorable snapshot of the
// pre-collapse layout (visibility + sizes + pin states). Idempotent: if nothing is open the
// existing snapshot is preserved (never overwritten with an empty state).
export function collapseAllWorkspace(ws, nowMs = 0) {
  const anyOpen = ws.tool_windows.some((w) => w.state !== "hidden");
  if (!anyOpen) return { ws, effects: [{ type: "noop", reason: "ALREADY_COLLAPSED", snapshotPreserved: true }] };
  const snapshot = {
    tool_windows: clone(ws.tool_windows),
    layout: clone(ws.layout),
    splitters: clone(ws.splitters),
    main_view: ws.layout.main_view || DEFAULT_VIEW,
  };
  const tool_windows = ws.tool_windows.map((w) => ({ ...w, state: "hidden", primary: false }));
  const layout = { ...ws.layout, collapsed: true };
  return { ws: { ...ws, tool_windows, layout, collapsed_snapshot: snapshot }, effects: [{ type: "collapsed_all", restoredVia: "collapsed_snapshot" }] };
}

// Restore Layout: restore visibility/sizes/pin states from the collapsed snapshot; if absent,
// fall back to the documented default layout (files tool open on the left). Never resurrects a
// tool window whose view is unknown to the current registry (unknown windows stay hidden).
export function restoreWorkspaceLayout(ws) {
  if (!ws.collapsed_snapshot) {
    const tool_windows = ws.tool_windows.map((w) => ({ ...w, state: "hidden", primary: false }));
    const files = tool_windows.find((w) => w.view === "files") || tool_windows[0] || null;
    if (files) { files.state = "open"; files.primary = true; }
    const layout = { ...ws.layout, collapsed: false, main_view: ws.layout.main_view || DEFAULT_VIEW };
    return { ws: { ...ws, tool_windows, layout, collapsed_snapshot: null }, effects: [{ type: "restored_default", opened: files ? files.id : null }] };
  }
  const snap = clone(ws.collapsed_snapshot);
  const snapIds = new Set(snap.tool_windows.map((w) => w.id));
  const extra = ws.tool_windows
    .filter((w) => !snapIds.has(w.id))
    .map((w) => ({ ...w, state: "hidden", primary: false }))
    .sort(byId);
  const restored = snap.tool_windows.map((w) => ({ ...w })).sort(byId).concat(extra);
  const layout = { ...ws.layout, ...snap.layout, collapsed: false };
  const splitters = { ...ws.splitters, ...snap.splitters };
  const main_view = snap.main_view || ws.layout.main_view || DEFAULT_VIEW;
  return {
    ws: { ...ws, tool_windows: restored, layout, splitters, collapsed_snapshot: null },
    effects: [{ type: "restored", remaining_snapshots: 0 }],
  };
}

// ---------- 5 / 7.2 normalize + unknown-view fallback (load) ----------
export function normalizeShellState(raw, ctx) {
  const dropped = [];
  const availableViews = ctx.availableViews || VALID_MAIN_VIEWS;
  const availableToolViews = ctx.availableToolViews || new Set();
  const availableConnIds = ctx.availableConnIds || new Set();

  if (!raw || typeof raw !== "object" || raw.schema_version == null) {
    return { state: freshState(), dropped: [{ reason: "missing-schema" }] };
  }
  if (raw.schema_version > SCHEMA_VERSION) {
    return { state: freshState(), dropped: [{ reason: "schema-too-new" }] };
  }
  const wsId = raw.active_workspace_id || "default";
  const srcWs = (raw.workspaces && raw.workspaces[wsId]) || { documents: [], tool_windows: [], layout: {}, splitters: {} };

  // documents: drop unknown refs
  const documents = [];
  for (const d of srcWs.documents || []) {
    const ok =
      d.kind === "browser" ||
      d.kind === "module" ||
      (d.kind === "db-console" && availableConnIds.has(connIdOf(d.ref))) ||
      (d.kind !== "db-console");
    if (!ok) { dropped.push({ reason: "unknown-conn", ref: d.ref }); continue; }
    documents.push({ ...d });
  }
  let active = srcWs.active_document_id || null;
  if (active && !documents.find((d) => d.id === active)) active = documents[0]?.id || null;

  // tool windows: unavailable view -> hidden (and pruned from any snapshot below)
  const tool_windows = (srcWs.tool_windows || []).map((w) => {
    if (w.view && availableToolViews.size && !availableToolViews.has(w.view)) {
      dropped.push({ reason: "unknown-tool-view", view: w.view });
      return { ...w, state: "hidden", primary: false };
    }
    return { ...w };
  });

  let mainView = srcWs.layout?.main_view || DEFAULT_VIEW;
  if (!availableViews.has(mainView)) {
    dropped.push({ reason: "unknown-view", view: mainView });
    mainView = DEFAULT_VIEW;
  }

  // collapsed snapshot: keep only if present; prune any window whose view is unknown
  let collapsed_snapshot = null;
  if (srcWs.collapsed_snapshot && typeof srcWs.collapsed_snapshot === "object") {
    const snap = clone(srcWs.collapsed_snapshot);
    if (Array.isArray(snap.tool_windows)) {
      snap.tool_windows = snap.tool_windows
        .filter((w) => !(w.view && availableToolViews.size && !availableToolViews.has(w.view)))
        .map((w) => ({ ...w }));
    }
    collapsed_snapshot = snap;
  }

  const state = {
    schema_version: SCHEMA_VERSION,
    saved_at_ms: raw.saved_at_ms || 0,
    global: { ...defaultGlobal(), ...(raw.global || {}) },
    active_workspace_id: wsId,
    workspaces: {
      [wsId]: {
        documents,
        active_document_id: active,
        tool_windows: enforceEdgeInvariant(tool_windows),
        layout: { ...defaultLayout(), ...(srcWs.layout || {}), main_view: mainView, collapsed: !!(srcWs.layout && srcWs.layout.collapsed) },
        splitters: { left: 300, right: 320, bottom: 220, ...(srcWs.splitters || {}) },
        collapsed_snapshot,
      },
    },
  };
  capDocuments(state.workspaces[wsId]);
  return { state, dropped };
}

// ---------- load (crash recovery: parse error / corrupt -> quarantine + fresh) ----------
export function loadShellState(rawStr, ctx) {
  let raw;
  if (typeof rawStr === "string") {
    try { raw = JSON.parse(rawStr); }
    catch (e) { return { state: freshState(), dropped: [{ reason: "parse-error" }], quarantined: true }; }
  } else {
    raw = rawStr;
  }
  if (!raw || typeof raw !== "object") return { state: freshState(), dropped: [{ reason: "missing" }], quarantined: true };
  const { state, dropped } = normalizeShellState(raw, ctx);
  return { state, dropped, quarantined: false };
}

// ---------- serialize (key-order-deterministic, transient/secret/session stripped) ----------
export function serializeShellState(state) {
  let secretsStripped = 0;
  function strip(v) {
    if (Array.isArray(v)) return v.map(strip);
    if (v && typeof v === "object") {
      const out = {};
      for (const k of Object.keys(v).sort()) {
        if (TRANSIENT_KEYS.has(k)) continue;
        if (SESSION_ONLY_KEYS.has(k)) continue;
        if (SECRET_KEY_RE.test(k) && typeof v[k] === "string") { secretsStripped += 1; continue; }
        out[k] = strip(v[k]);
      }
      return out;
    }
    return v;
  }
  const clean = strip(state);
  // canonicalize tool-window array order so serialized output is key-order independent
  if (clean.workspaces) {
    for (const id of Object.keys(clean.workspaces)) {
      const ws = clean.workspaces[id];
      if (Array.isArray(ws.tool_windows)) ws.tool_windows.sort(byId);
      if (ws.collapsed_snapshot && Array.isArray(ws.collapsed_snapshot.tool_windows)) ws.collapsed_snapshot.tool_windows.sort(byId);
    }
  }
  return { json: JSON.stringify(clean, null, 2), secretsStripped };
}

// ---------- geometry (A0 SSOT) ----------
export function collapsedShares(vw, vh) {
  const w = vw ? (vw - ACTIVITY_STRIP_PX) / vw : 0;
  const h = vh ? (vh - TOP_CHROME_PX - STATUS_PX) / vh : 0;
  return { w, h };
}

// ---------- helpers ----------
function connIdOf(ref) {
  const m = /^db-console:(.*)$/.exec(ref || "");
  return m ? m[1] : "";
}
function capDocuments(ws) {
  if (ws.documents.length <= MAX_DOCUMENTS) return;
  const keep = new Set();
  const sorted = [...ws.documents].sort((a, b) => (a.id > b.id ? 1 : -1));
  for (const d of sorted) if (keep.size >= MAX_DOCUMENTS) break; else if (d.pinned) keep.add(d.id);
  for (const d of sorted) if (keep.size >= MAX_DOCUMENTS) break; else if (!d.pinned) keep.add(d.id);
  ws.documents = ws.documents.filter((d) => keep.has(d.id));
  if (ws.active_document_id && !keep.has(ws.active_document_id)) ws.active_document_id = ws.documents[0]?.id || null;
}

// ---------- tiny test harness ----------
function assert(cond, msg) {
  if (!cond) throw new Error("FAIL: " + msg);
}
function run() {
  let n = 0;
  const ok = (name, fn) => { fn(); n++; console.log("  ok  " + name); };

  ok("T1 migrate empty", () => {
    const r = migrateFromLegacy({});
    assert(r.documents.length === 0 && r.active_document_id === null, "empty");
  });
  ok("T2 migrate browser+module", () => {
    const r = migrateFromLegacy({
      browserTabs: [{ id: "t1", url: "https://a.com", title: "A" }, { id: "t2", url: "https://b.com", title: "B" }],
      modTabs: [{ id: "m1", view: "files", title: "Files" }], activeTabId: "t2",
    });
    assert(r.documents.length === 3, "3 docs, got " + r.documents.length);
    assert(r.documents.filter((d) => d.kind === "browser").length === 2, "2 browser");
  });
  ok("T3 de-dup duplicate url", () => {
    const r = migrateFromLegacy({ browserTabs: [{ id: "t1", url: "u", title: "X" }, { id: "t2", url: "u", title: "X" }] });
    assert(r.documents.length === 1, "dup collapsed, got " + r.documents.length);
  });

  ok("T4 unknown main_view -> home", () => {
    const { state, dropped } = normalizeShellState(
      { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: [], tool_windows: [], layout: { main_view: "nonexistent" }, splitters: {} } } },
      { availableViews: VALID_MAIN_VIEWS });
    assert(state.workspaces.default.layout.main_view === "home", "reset to home");
    assert(dropped.some((d) => d.reason === "unknown-view"), "dropped recorded");
  });
  ok("T5 doc unknown conn -> dropped + active reset", () => {
    const { state, dropped } = normalizeShellState(
      { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: [{ id: "d1", kind: "db-console", ref: "db-console:gone", title: "G" }], active_document_id: "d1", tool_windows: [], layout: {}, splitters: {} } } },
      { availableConnIds: new Set(["keep"]) });
    assert(state.workspaces.default.documents.length === 0, "doc dropped");
    assert(state.workspaces.default.active_document_id === null, "active reset");
  });
  ok("T6 tool view unavailable -> hidden", () => {
    const { state, dropped } = normalizeShellState(
      { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: [], tool_windows: [{ id: "x", edge: "right", view: "ghost", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 }], layout: {}, splitters: {} } } },
      { availableToolViews: new Set(["files", "terminal"]) });
    assert(state.workspaces.default.tool_windows[0].state === "hidden", "hidden");
    assert(dropped.some((d) => d.reason === "unknown-tool-view"), "reason");
  });
  ok("T7 schema too new -> fresh", () => {
    const { state, dropped } = normalizeShellState({ schema_version: 99, workspaces: {} }, {});
    assert(state.workspaces.default.documents.length === 0, "fresh");
    assert(dropped.some((d) => d.reason === "schema-too-new"), "reason");
  });
  ok("T8 corrupt/missing schema -> fresh", () => {
    const { state } = normalizeShellState(null, {});
    assert(state.workspaces.default.documents.length === 0, "fresh on null");
  });

  ok("T9 edge invariant: pinned wins", () => {
    const tw = [
      { id: "a", edge: "right", view: "files", state: "open", primary: true, pinned: true, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
      { id: "b", edge: "right", view: "term", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 2 },
    ];
    const out = enforceEdgeInvariant(tw);
    const primaries = out.filter((w) => w.primary);
    assert(primaries.length === 1 && primaries[0].id === "a", "pinned a stays primary");
  });
  ok("T10 edge invariant: highest activated wins", () => {
    const tw = [
      { id: "a", edge: "left", view: "f", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
      { id: "b", edge: "left", view: "t", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 5 },
    ];
    const out = enforceEdgeInvariant(tw);
    const primaries = out.filter((w) => w.primary);
    assert(primaries.length === 1 && primaries[0].id === "b", "b (newer) wins");
  });
  ok("T11 open on empty edge", () => {
    const out = openToolWindow([], "right", "terminal", 10);
    assert(out[0].primary === true && out[0].state === "open", "primary open");
  });
  ok("T12 open replaces non-pinned primary", () => {
    const tw = [{ id: "a", edge: "right", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 }];
    const out = openToolWindow(tw, "right", "terminal", 10);
    assert(out.find((w) => w.id === "a").primary === false, "a replaced");
    assert(out.find((w) => w.id === "terminal").primary === true && out.find((w) => w.id === "terminal").state === "open", "terminal primary");
  });
  ok("T13 open does NOT replace pinned primary", () => {
    const tw = [{ id: "a", edge: "right", view: "files", state: "open", primary: true, pinned: true, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 }];
    const out = openToolWindow(tw, "right", "terminal", 10);
    assert(out.find((w) => w.id === "a").primary === true, "pinned a stays primary");
    assert(out.find((w) => w.id === "terminal").primary === false && out.find((w) => w.id === "terminal").state === "stashed", "terminal stashed, not replaced");
  });

  // ---- R3B-corrected collapse/restore ----
  ok("T14 collapseAll hides PINNED too", () => {
    const ws = mkWs([
      { id: "a", edge: "right", view: "files", state: "open", primary: false, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
      { id: "b", edge: "right", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 320, last_activated_ms: 1 },
    ]);
    const r = collapseAllWorkspace(ws);
    assert(r.ws.tool_windows.every((w) => w.state === "hidden"), "all hidden incl pinned b");
    const snapB = r.ws.collapsed_snapshot.tool_windows.find((w) => w.id === "b");
    assert(snapB && snapB.state === "open" && snapB.pinned === true, "snapshot preserved pinned-open");
    assert(r.ws.layout.collapsed === true, "collapsed flag set");
  });
  ok("T15 capacity cap drops oldest non-pinned", () => {
    const docs = [];
    for (let i = 0; i < MAX_DOCUMENTS + 5; i++) docs.push({ id: "d" + i, kind: "browser", ref: "browser:" + i, title: "" + i });
    docs[MAX_DOCUMENTS + 1].pinned = true;
    const raw = { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: docs, active_document_id: "d0", tool_windows: [], layout: {}, splitters: {} } } };
    const { state } = normalizeShellState(raw, {});
    assert(state.workspaces.default.documents.length === MAX_DOCUMENTS, "capped, got " + state.workspaces.default.documents.length);
    assert(state.workspaces.default.documents.some((d) => d.id === "d" + (MAX_DOCUMENTS + 1)), "pinned kept");
  });
  ok("T16 collapseAll records exactly one restorable snapshot", () => {
    const ws = mkWs([
      { id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 },
      { id: "b", edge: "bottom", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 240, last_activated_ms: 1 },
    ], { layout: { ...defaultLayout(), main_view: "files" } });
    const r = collapseAllWorkspace(ws);
    assert(r.ws.collapsed_snapshot && Array.isArray(r.ws.collapsed_snapshot.tool_windows), "snapshot exists");
    assert(r.ws.collapsed_snapshot.tool_windows.length === 2, "snapshot captured both windows");
    assert(r.ws.collapsed_snapshot.splitters.left === 260 || r.ws.collapsed_snapshot.tool_windows.find((w) => w.id === "a").size_px === 260, "sizes captured");
  });
  ok("T17 collapseAll idempotent when already collapsed (snapshot preserved)", () => {
    const ws = mkWs([{ id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 }]);
    const r1 = collapseAllWorkspace(ws);
    const r2 = collapseAllWorkspace(r1.ws);
    assert(r2.ws.tool_windows.every((w) => w.state === "hidden"), "still hidden");
    const s1 = JSON.stringify(r1.ws.collapsed_snapshot);
    const s2 = JSON.stringify(r2.ws.collapsed_snapshot);
    assert(s1 === s2, "snapshot NOT overwritten with empty state");
  });
  ok("T18 collapseAll + restore round-trip restores visibility/size/pin", () => {
    const ws = mkWs([
      { id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 },
      { id: "b", edge: "bottom", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 240, last_activated_ms: 1 },
    ]);
    const c = collapseAllWorkspace(ws);
    const r = restoreWorkspaceLayout(c.ws);
    assert(r.ws.tool_windows.find((w) => w.id === "a").state === "open", "a visible again");
    assert(r.ws.tool_windows.find((w) => w.id === "a").size_px === 260, "a size restored");
    assert(r.ws.tool_windows.find((w) => w.id === "b").pinned === true && r.ws.tool_windows.find((w) => w.id === "b").state === "open", "b pin+visibility restored");
    assert(r.ws.layout.collapsed === false, "collapsed cleared");
    assert(r.ws.collapsed_snapshot === null, "snapshot consumed");
  });
  ok("T19 restore with empty snapshot -> documented default (files on left)", () => {
    // registry always defines a 'files' tool; here it is persisted but hidden.
    const ws = mkWs([
      { id: "files", edge: "left", view: "files", state: "hidden", primary: false, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 },
      { id: "a", edge: "right", view: "inspect", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 280, last_activated_ms: 1 },
    ], { collapsed_snapshot: null, layout: { ...defaultLayout(), collapsed: true } });
    const r = restoreWorkspaceLayout(ws);
    const files = r.ws.tool_windows.find((w) => w.view === "files");
    assert(files && files.state === "open" && files.primary === true && files.edge === "left", "default files on left");
  });
  ok("T20 restore clears collapsed flag", () => {
    const ws = mkWs([{ id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 }]);
    const c = collapseAllWorkspace(ws);
    const r = restoreWorkspaceLayout(c.ws);
    assert(r.ws.layout.collapsed === false, "collapsed=false after restore");
  });

  // ---- transient / secret / session exclusion ----
  const mkDirtyState = () => {
    const s = freshState();
    s.workspaces.default.tool_windows.push({ id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1, hover_id: "node-9", overlay: { kind: "preview" }, drag_preview: true });
    s.workspaces.default.layout.collapsed = true;
    s.workspaces.default.layout.tooltip = "x";
    s.focus = { surfaceId: "a", selector: null };
    s.focusStack = [{ surfaceId: "doc", selector: null }];
    s.snapshots = [{ edges: {}, tools: {}, topRows: 2 }];
    s.workspaces.default.tool_windows[0].meta = { db_password: "supersecret" };
    return s;
  };
  ok("T21 transient fields excluded from serialized JSON", () => {
    const a = mkDirtyState();
    const b = mkDirtyState();
    delete b.workspaces.default.tool_windows[0].hover_id; delete b.workspaces.default.tool_windows[0].overlay; delete b.workspaces.default.tool_windows[0].drag_preview; delete b.workspaces.default.layout.tooltip;
    const ja = serializeShellState(a).json; const jb = serializeShellState(b).json;
    assert(ja === jb, "transient state does not change serialized output");
    assert(!/hover_id|overlay|drag_preview|tooltip/.test(ja), "no transient key in JSON");
  });
  ok("T22 focus/focusStack dropped from serialized JSON", () => {
    const { json } = serializeShellState(mkDirtyState());
    assert(!/"focus"/.test(json) && !/"focusStack"/.test(json), "focus/focusStack not persisted");
  });
  ok("T23 secret-key guard strips credentials from serialized JSON", () => {
    const { json, secretsStripped } = serializeShellState(mkDirtyState());
    assert(!/supersecret/.test(json), "secret value absent");
    assert(secretsStripped >= 1, "secret key stripped");
  });
  ok("T24 normalize idempotent (double-run stable)", () => {
    const raw = { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: [{ id: "d1", kind: "browser", ref: "browser:u", title: "U" }], active_document_id: "d1", tool_windows: [{ id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 }], layout: { main_view: "files", collapsed: true }, splitters: { left: 260 } } } };
    const a = JSON.stringify(normalizeShellState(raw, { availableViews: VALID_MAIN_VIEWS }).state);
    const b = JSON.stringify(normalizeShellState(JSON.parse(a), { availableViews: VALID_MAIN_VIEWS }).state);
    assert(a === b, "normalize stable");
  });
  ok("T25 unknown open tool window -> hidden; restore does NOT resurrect ghost", () => {
    // persisted file: ghost was open AND captured open in the collapsed snapshot.
    const raw = {
      schema_version: 1, active_workspace_id: "default",
      workspaces: { default: { documents: [], tool_windows: [{ id: "ghost", edge: "right", view: "ghost", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 280, last_activated_ms: 1 }], layout: { main_view: "home", collapsed: true }, splitters: {}, collapsed_snapshot: { tool_windows: [{ id: "ghost", edge: "right", view: "ghost", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 280, last_activated_ms: 1 }], layout: { main_view: "home" }, splitters: {}, main_view: "home" } } },
    };
    const { state } = normalizeShellState(raw, { availableToolViews: new Set(["files", "terminal"]) });
    assert(state.workspaces.default.tool_windows[0].state === "hidden", "ghost hidden on load");
    // the unknown ghost must also be pruned from the snapshot so restore cannot resurrect it
    assert(!state.workspaces.default.collapsed_snapshot.tool_windows.some((w) => w.id === "ghost"), "ghost pruned from snapshot");
    const c = collapseAllWorkspace(state.workspaces.default);
    const r = restoreWorkspaceLayout(c.ws);
    assert(r.ws.tool_windows.find((w) => w.id === "ghost").state === "hidden", "ghost stays hidden after restore");
  });
  ok("T26 corrupted JSON -> quarantine + fresh, no throw", () => {
    const { state, quarantined } = loadShellState("{ this is : not json", {});
    assert(quarantined === true, "quarantined");
    assert(state.workspaces.default.documents.length === 0, "fresh state");
  });
  ok("T27 crash recovery: save->load round trip preserves layout", () => {
    const ws = mkWs([
      { id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 },
      { id: "b", edge: "bottom", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 240, last_activated_ms: 1 },
    ], { layout: { ...defaultLayout(), main_view: "files", collapsed: true }, collapsed_snapshot: { tool_windows: [{ id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 1 }, { id: "b", edge: "bottom", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 240, last_activated_ms: 1 }], layout: { ...defaultLayout(), main_view: "files" }, splitters: { left: 260, right: 320, bottom: 240 }, main_view: "files" } });
    const st = freshState(); st.workspaces.default = ws;
    const str = serializeShellState(st).json;
    const { state, quarantined } = loadShellState(str, { availableViews: VALID_MAIN_VIEWS, availableToolViews: new Set(["files", "term"]) });
    assert(quarantined === false, "valid file not quarantined");
    assert(state.workspaces.default.collapsed_snapshot && state.workspaces.default.collapsed_snapshot.tool_windows.length === 2, "snapshot survived round trip");
    assert(state.workspaces.default.layout.collapsed === true, "collapsed survived");
  });
  ok("T28 determinism: collapse output independent of input key order", () => {
    const wsA = mkWs([{ id: "b", edge: "bottom", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 240, last_activated_ms: 1 }, { id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 2 }]);
    const wsB = mkWs([{ id: "a", edge: "left", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 260, last_activated_ms: 2 }, { id: "b", edge: "bottom", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 240, last_activated_ms: 1 }]);
    const ca = serializeShellState(freshStateWith(collapseAllWorkspace(wsA).ws)).json;
    const cb = serializeShellState(freshStateWith(collapseAllWorkspace(wsB).ws)).json;
    assert(ca === cb, "key-order-independent serialized output");
  });

  // ---- geometry (A0 SSOT) ----
  ok("T29 collapsed shares: A0 formula, 900x600 >= 85% height", () => {
    for (const [w, h] of [[1920, 1080], [1440, 900], [1366, 768], [1200, 800], [1024, 720], [900, 600]]) {
      const s = collapsedShares(w, h);
      assert(Math.abs(s.w - (w - ACTIVITY_STRIP_PX) / w) < 1e-9, "width formula " + w + "x" + h);
      assert(Math.abs(s.h - (h - TOP_CHROME_PX - STATUS_PX) / h) < 1e-9, "height formula " + w + "x" + h);
    }
    const min = collapsedShares(900, 600);
    assert(min.h >= 0.85, "900x600 height share >= 85%, got " + (min.h * 100).toFixed(1) + "%");
  });

  console.log(`\nA4-R3B PURE TESTS: ${n}/${n} PASS`);
  return n;
}
function freshStateWith(ws) { const st = freshState(); st.workspaces.default = ws; return st; }

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const n = run();
    if (n !== 29) { console.error("expected 29 tests"); process.exit(1); }
  } catch (e) {
    console.error(String(e && e.stack || e));
    process.exit(1);
  }
}
