// A4-R3 shell-state & persistence contract — self-contained reference implementation.
// Pure functions only (no bridge / no DOM / no product import). Runs under `node`.
//
//   node logs/research/M5-W18/A4-R3-shell-state-prototype.mjs
//
// This file is a RESEARCH PROTOTYPE. It is NOT wired into the app and must not be
// imported by any product source. W19 ports these functions 1:1 into src/stores.

// ---------- constants ----------
const SCHEMA_VERSION = 1;
const MAX_DOCUMENTS = 200;
const MAX_TOOL_WINDOWS = 32;
const EDGES = ["left", "right", "bottom"];
const DEFAULT_VIEW = "home";
const VALID_MAIN_VIEWS = new Set([
  "home", "browser", "files", "clip", "arts", "grid", "apps", "term", "repo",
  "audit", "scripts", "commands", "tools", "db", "tasks", "skills", "agents",
  "graph", "plugin", "editor",
]);

// ---------- 7.1 migration from current tabs ----------
export function migrateFromLegacy(legacy) {
  const documents = [];
  const seen = new Map(); // ref -> doc id (de-dup)
  const push = (kind, ref, title, meta) => {
    if (seen.has(ref)) return seen.get(ref);
    const id = "doc-" + (documents.length + 1);
    const doc = { id, kind, ref, title: title || ref };
    if (meta) doc.meta = meta;
    documents.push(doc);
    seen.set(ref, id);
    return id;
  };
  for (const t of legacy.browserTabs || []) {
    push("browser", "browser:" + t.url, t.title);
  }
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
    // tie-break: pinned first, then highest last_activated_ms
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
// Creates the window if absent (the product's window registry may be absent at call time),
// then applies the edge rule.
export function openToolWindow(toolWindows, edge, id, nowMs = Date.now(), view) {
  const out = toolWindows.map((w) => ({ ...w }));
  let target = out.find((w) => w.id === id);
  if (!target) {
    target = { id, edge, view: view || id, state: "stashed", primary: false, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: nowMs };
    out.push(target);
  } else if (target.edge !== edge) {
    target.edge = edge;
  }
  const currentPrimary = out.find((w) => w.edge === edge && w.primary && w.id !== id);
  if (currentPrimary && currentPrimary.pinned) {
    // pinned primary wins: it stays primary, the incoming opens as stashed (available)
    target.primary = false;
    target.state = "stashed";
  } else {
    for (const w of out) if (w.edge === edge) w.primary = false;
    target.primary = true;
    target.state = "open";
  }
  target.edge = edge;
  target.last_activated_ms = nowMs;
  return enforceEdgeInvariant(out);
}

// ---------- 7.5 collapse all ----------
export function collapseAll(toolWindows) {
  return toolWindows.map((w) =>
    w.pinned ? { ...w } : { ...w, primary: false, state: "hidden" }
  );
}

// ---------- 5 / 7.2 normalize + unknown-view fallback ----------
export function normalizeShellState(raw, ctx) {
  const dropped = [];
  const availableViews = ctx.availableViews || VALID_MAIN_VIEWS;
  const availableToolViews = ctx.availableToolViews || new Set();
  const availableConnIds = ctx.availableConnIds || new Set();

  // schema guard
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
      (d.kind !== "db-console"); // editor/git/graph-node/note: no live registry check in R3
    if (!ok) {
      dropped.push({ reason: "unknown-conn", ref: d.ref });
      continue;
    }
    documents.push({ ...d });
  }

  // active document reset if dropped
  let active = srcWs.active_document_id || null;
  if (active && !documents.find((d) => d.id === active)) active = documents[0]?.id || null;

  // tool windows: unavailable view -> hidden
  const tool_windows = (srcWs.tool_windows || []).map((w) => {
    if (w.view && availableToolViews.size && !availableToolViews.has(w.view)) {
      dropped.push({ reason: "unknown-tool-view", view: w.view });
      return { ...w, state: "hidden", primary: false };
    }
    return { ...w };
  });

  // main view fallback
  let mainView = srcWs.layout?.main_view || DEFAULT_VIEW;
  if (!availableViews.has(mainView)) {
    dropped.push({ reason: "unknown-view", view: mainView });
    mainView = DEFAULT_VIEW;
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
        layout: { ...defaultLayout(), ...(srcWs.layout || {}), main_view: mainView },
        splitters: { left: 300, right: 320, bottom: 220, ...(srcWs.splitters || {}) },
      },
    },
  };
  // capacity cap
  capDocuments(state.workspaces[wsId]);
  return { state, dropped };
}

// ---------- helpers ----------
function connIdOf(ref) {
  const m = /^db-console:(.*)$/.exec(ref || "");
  return m ? m[1] : "";
}
function capDocuments(ws) {
  if (ws.documents.length <= MAX_DOCUMENTS) return;
  const keep = new Set();
  // drop oldest non-pinned first
  const sorted = [...ws.documents].sort((a, b) => (a.id > b.id ? 1 : -1));
  for (const d of sorted) {
    if (keep.size >= MAX_DOCUMENTS) break;
    if (d.pinned) keep.add(d.id);
  }
  for (const d of sorted) {
    if (keep.size >= MAX_DOCUMENTS) break;
    if (!d.pinned) keep.add(d.id);
  }
  ws.documents = ws.documents.filter((d) => keep.has(d.id));
  if (ws.active_document_id && !keep.has(ws.active_document_id)) {
    ws.active_document_id = ws.documents[0]?.id || null;
  }
}
function defaultGlobal() {
  return {
    theme: "dark",
    focus_mode_default: false,
    density_pref: "auto",
    sidebar_open: true,
    sidebar_width: 300,
    addr_mode: "url",
    compact_mode: false,
  };
}
function defaultLayout() {
  return { main_view: DEFAULT_VIEW, nav_section: "", density: "full", compact_mode: false, addr_mode: "url", sidebar_open: true, sidebar_width: 300, focus_mode: false };
}
export function freshState() {
  return {
    schema_version: SCHEMA_VERSION,
    saved_at_ms: 0,
    global: defaultGlobal(),
    active_workspace_id: "default",
    workspaces: { default: { documents: [], active_document_id: null, tool_windows: [], layout: defaultLayout(), splitters: { left: 300, right: 320, bottom: 220 } } },
  };
}

// ---------- tiny test harness ----------
function assert(cond, msg) {
  if (!cond) throw new Error("FAIL: " + msg);
}
function run() {
  let n = 0;
  const ok = (name, fn) => { fn(); n++; console.log("  ok  " + name); };

  // T1
  ok("T1 migrate empty", () => {
    const r = migrateFromLegacy({});
    assert(r.documents.length === 0 && r.active_document_id === null, "empty");
  });
  // T2
  ok("T2 migrate browser+module", () => {
    const r = migrateFromLegacy({
      browserTabs: [{ id: "t1", url: "https://a.com", title: "A" }, { id: "t2", url: "https://b.com", title: "B" }],
      modTabs: [{ id: "m1", view: "files", title: "Files" }],
      activeTabId: "t2",
    });
    assert(r.documents.length === 3, "3 docs, got " + r.documents.length);
    assert(r.documents.filter((d) => d.kind === "browser").length === 2, "2 browser");
    assert(r.active_document_id && r.documents.find((d) => d.id === r.active_document_id).kind === "browser", "active browser");
  });
  // T3
  ok("T3 de-dup duplicate url", () => {
    const r = migrateFromLegacy({ browserTabs: [{ id: "t1", url: "u", title: "X" }, { id: "t2", url: "u", title: "X" }] });
    assert(r.documents.length === 1, "dup collapsed, got " + r.documents.length);
  });

  // T4
  ok("T4 unknown main_view -> home", () => {
    const { state, dropped } = normalizeShellState(
      { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: [], tool_windows: [], layout: { main_view: "nonexistent" }, splitters: {} } } },
      { availableViews: VALID_MAIN_VIEWS }
    );
    assert(state.workspaces.default.layout.main_view === "home", "reset to home");
    assert(dropped.some((d) => d.reason === "unknown-view"), "dropped recorded");
  });
  // T5
  ok("T5 doc unknown conn -> dropped + active reset", () => {
    const { state, dropped } = normalizeShellState(
      { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: [{ id: "d1", kind: "db-console", ref: "db-console:gone", title: "G" }], active_document_id: "d1", tool_windows: [], layout: {}, splitters: {} } } },
      { availableConnIds: new Set(["keep"]) }
    );
    assert(state.workspaces.default.documents.length === 0, "doc dropped");
    assert(state.workspaces.default.active_document_id === null, "active reset");
    assert(dropped.some((d) => d.reason === "unknown-conn"), "reason recorded");
  });
  // T6
  ok("T6 tool view unavailable -> hidden", () => {
    const { state, dropped } = normalizeShellState(
      { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: [], tool_windows: [{ id: "x", edge: "right", view: "ghost", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 }], layout: {}, splitters: {} } } },
      { availableToolViews: new Set(["files", "terminal"]) }
    );
    assert(state.workspaces.default.tool_windows[0].state === "hidden", "hidden");
    assert(dropped.some((d) => d.reason === "unknown-tool-view"), "reason");
  });
  // T7
  ok("T7 schema too new -> fresh", () => {
    const { state, dropped } = normalizeShellState({ schema_version: 99, workspaces: {} }, {});
    assert(state.workspaces.default.documents.length === 0, "fresh");
    assert(dropped.some((d) => d.reason === "schema-too-new"), "reason");
  });
  // T8
  ok("T8 corrupt/missing schema -> fresh", () => {
    const { state } = normalizeShellState(null, {});
    assert(state.workspaces.default.documents.length === 0, "fresh on null");
  });

  // T9
  ok("T9 edge invariant: pinned wins", () => {
    const tw = [
      { id: "a", edge: "right", view: "files", state: "open", primary: true, pinned: true, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
      { id: "b", edge: "right", view: "term", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 2 },
    ];
    const out = enforceEdgeInvariant(tw);
    const primaries = out.filter((w) => w.primary);
    assert(primaries.length === 1 && primaries[0].id === "a", "pinned a stays primary");
  });
  // T10
  ok("T10 edge invariant: highest activated wins", () => {
    const tw = [
      { id: "a", edge: "left", view: "f", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
      { id: "b", edge: "left", view: "t", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 5 },
    ];
    const out = enforceEdgeInvariant(tw);
    const primaries = out.filter((w) => w.primary);
    assert(primaries.length === 1 && primaries[0].id === "b", "b (newer) wins");
  });

  // T11
  ok("T11 open on empty edge", () => {
    const out = openToolWindow([], "right", "terminal", 10);
    assert(out[0].primary === true && out[0].state === "open", "primary open");
  });
  // T12
  ok("T12 open replaces non-pinned primary", () => {
    const tw = [
      { id: "a", edge: "right", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
    ];
    const out = openToolWindow(tw, "right", "terminal", 10);
    const a = out.find((w) => w.id === "a");
    const t = out.find((w) => w.id === "terminal");
    assert(a.primary === false, "a replaced");
    assert(t.primary === true && t.state === "open", "terminal primary");
  });
  // T13
  ok("T13 open does NOT replace pinned primary", () => {
    const tw = [
      { id: "a", edge: "right", view: "files", state: "open", primary: true, pinned: true, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
    ];
    const out = openToolWindow(tw, "right", "terminal", 10);
    const a = out.find((w) => w.id === "a");
    const t = out.find((w) => w.id === "terminal");
    assert(a.primary === true, "pinned a stays primary");
    assert(t.primary === false && t.state === "stashed", "terminal stashed, not replaced");
  });

  // T14
  ok("T14 collapseAll keeps pinned", () => {
    const tw = [
      { id: "a", edge: "right", view: "files", state: "open", primary: true, pinned: false, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
      { id: "b", edge: "right", view: "term", state: "open", primary: false, pinned: true, autoHide: false, maximized: false, size_px: 300, last_activated_ms: 1 },
    ];
    const out = collapseAll(tw);
    assert(out.find((w) => w.id === "a").state === "hidden", "a hidden");
    assert(out.find((w) => w.id === "b").state === "open", "b (pinned) preserved");
  });

  // T15
  ok("T15 capacity cap drops oldest non-pinned", () => {
    const docs = [];
    for (let i = 0; i < MAX_DOCUMENTS + 5; i++) docs.push({ id: "d" + i, kind: "browser", ref: "browser:" + i, title: "" + i });
    docs[MAX_DOCUMENTS + 1].pinned = true; // pin one of the overflow
    const raw = { schema_version: 1, active_workspace_id: "default", workspaces: { default: { documents: docs, active_document_id: "d0", tool_windows: [], layout: {}, splitters: {} } } };
    const { state } = normalizeShellState(raw, {});
    assert(state.workspaces.default.documents.length === MAX_DOCUMENTS, "capped to " + MAX_DOCUMENTS + ", got " + state.workspaces.default.documents.length);
    assert(state.workspaces.default.documents.some((d) => d.id === "d" + (MAX_DOCUMENTS + 1)), "pinned kept");
  });

  console.log(`\nA4-R3 PURE TESTS: ${n}/${n} PASS`);
  return n;
}

// run when executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const n = run();
    if (n !== 15) { console.error("expected 15 tests"); process.exit(1); }
  } catch (e) {
    console.error(String(e && e.stack || e));
    process.exit(1);
  }
}
