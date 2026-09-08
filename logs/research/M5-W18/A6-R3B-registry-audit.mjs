// A6 · M5-W18-R3B — Machine-checkable registry audit (RESEARCH ARTIFACT ONLY)
//
// Runs the invariant checks that the R3B Lane A6 acceptance requires:
//   1. Ctrl+K / Ctrl+Shift+P frozen to the two designated commands; no other
//      command claims those chords; no duplicate chords.
//   2. Every command has a stable id and a scoped (object-typed) menu placement.
//   3. Every command declares a disabled-reason hook (null => always enabled).
//   4. Every command has a keyboard or accessible path (reachable non-empty,
//      and never toolbar-only).
//   5. Every command carries a valid safety class, audit class and confirm tier.
//   6. Every command exposes accessibility attributes (role=menuitem, focus return).
//   7. All 14 Git workflow units (A0 ruling 43) have stable ids + full fields.
//   8. The explicitly required visible Git units (amend/reset/revert/…) exist.
//   9. No permanent toolbar-button expansion (no toolbarOnly:true anywhere).
//
// Usage:  node logs/research/M5-W18/A6-R3B-registry-audit.mjs
// Exit:   0 = PASS, 1 = FAIL (machine gate for A10/A11).

import {
  COMMANDS, SHORTCUTS, SAFETY_CLASSES, AUDIT_CLASSES, CONFIRM_TIERS,
  REACHABLE_PATHS, SCOPE_PREFIXES, GIT14, GIT_VISIBLE,
} from "./A6-R3B-registry.mjs";

let failures = 0;
const errors = [];
const note = [];

function check(cond, msg) {
  if (!cond) { failures++; errors.push(msg); }
}

// ---------------------------------------------------------------------------
// 1. Frozen global shortcuts (A0 ruling 41)
// ---------------------------------------------------------------------------
const frozenKeys = Object.keys(SHORTCUTS).sort();
check(
  frozenKeys.length === 2 &&
    frozenKeys.includes("Ctrl+K") && frozenKeys.includes("Ctrl+Shift+P"),
  `R3B-06: exactly two frozen chords expected (Ctrl+K, Ctrl+Shift+P), got [${frozenKeys.join(", ")}]`,
);
check(
  SHORTCUTS["Ctrl+K"] === "browser.focusAddressSearch",
  `R3B-06: Ctrl+K must map to browser.focusAddressSearch, got ${SHORTCUTS["Ctrl+K"]}`,
);
check(
  SHORTCUTS["Ctrl+Shift+P"] === "app.openCommandPalette",
  `R3B-06: Ctrl+Shift+P must map to app.openCommandPalette, got ${SHORTCUTS["Ctrl+Shift+P"]}`,
);

const byId = new Map(COMMANDS.map((c) => [c.id, c]));
check(byId.has("browser.focusAddressSearch"), "frozen Ctrl+K target command exists");
check(byId.has("app.openCommandPalette"), "frozen Ctrl+Shift+P target command exists");

// No other command may use the two frozen chords.
for (const c of COMMANDS) {
  if (c.shortcut === "Ctrl+K" && c.id !== "browser.focusAddressSearch")
    check(false, `command ${c.id} illegally claims Ctrl+K`);
  if (c.shortcut === "Ctrl+Shift+P" && c.id !== "app.openCommandPalette")
    check(false, `command ${c.id} illegally claims Ctrl+Shift+P`);
}

// No duplicate shortcut chords across commands.
const chordOwners = new Map();
for (const c of COMMANDS) {
  if (!c.shortcut) continue;
  if (chordOwners.has(c.shortcut))
    check(false, `duplicate shortcut ${c.shortcut} used by ${chordOwners.get(c.shortcut)} and ${c.id}`);
  chordOwners.set(c.shortcut, c.id);
}

// ---------------------------------------------------------------------------
// 2-6. Per-command invariants
// ---------------------------------------------------------------------------
const seenIds = new Set();
for (const c of COMMANDS) {
  // unique stable id
  check(!seenIds.has(c.id), `duplicate command id ${c.id}`);
  seenIds.add(c.id);

  // scoped (object-typed) menu placement
  const scoped = SCOPE_PREFIXES.some((p) => c.menu && c.menu.startsWith(p + "."));
  check(scoped, `command ${c.id}: menu '${c.menu}' is not scoped to a typed object`);

  // disabled-reason hook present (null or token)
  check(
    c.disabledReason === null || typeof c.disabledReason === "string",
    `command ${c.id}: disabledReason must be null or a reason token`,
  );

  // reachable (keyboard/accessible path) non-empty and valid
  check(Array.isArray(c.reachable) && c.reachable.length > 0,
    `command ${c.id}: reachable path set is empty (no keyboard/accessible path)`);
  for (const r of c.reachable)
    check(REACHABLE_PATHS.has(r), `command ${c.id}: invalid reachable path '${r}'`);

  // no permanent toolbar-button expansion
  check(c.toolbarOnly === false, `command ${c.id}: toolbarOnly must be false (no toolbar-only expansion)`);

  // valid classification enums
  check(SAFETY_CLASSES.has(c.safetyClass), `command ${c.id}: invalid safetyClass '${c.safetyClass}'`);
  check(AUDIT_CLASSES.has(c.auditClass), `command ${c.id}: invalid auditClass '${c.auditClass}'`);
  check(CONFIRM_TIERS.has(c.confirmTier), `command ${c.id}: invalid confirmTier '${c.confirmTier}'`);

  // accessibility attributes
  check(c.accessible && c.accessible.ariaRole === "menuitem",
    `command ${c.id}: accessible.ariaRole must be 'menuitem'`);
  check(c.accessible && typeof c.accessible.focusReturn === "boolean",
    `command ${c.id}: accessible.focusReturn must be boolean`);
}

// ---------------------------------------------------------------------------
// 7. 14 Git workflow units (A0 ruling 43)
// ---------------------------------------------------------------------------
for (const [unit, id] of Object.entries(GIT14)) {
  const c = byId.get(id);
  check(!!c, `GIT14 unit '${unit}' -> ${id} is missing from registry`);
  if (c) {
    check(SAFETY_CLASSES.has(c.safetyClass), `GIT14 ${id}: missing/invalid safetyClass`);
    check(c.disabledReason !== undefined, `GIT14 ${id}: missing disabledReason hook`);
    check(!!c.menu, `GIT14 ${id}: missing scoped menu placement`);
    check(c.reachable.length > 0, `GIT14 ${id}: no keyboard/accessible path`);
    check(c.accessible && c.accessible.ariaRole === "menuitem", `GIT14 ${id}: missing a11y attrs`);
  }
}

// ---------------------------------------------------------------------------
// 8. Explicitly required visible Git units
// ---------------------------------------------------------------------------
for (const id of GIT_VISIBLE) {
  check(byId.has(id), `required visible Git command ${id} is missing`);
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------
const counts = {
  total: COMMANDS.length,
  shell: COMMANDS.filter((c) => ["browser.tab", "browser.page", "workspace.file", "workspace.tree", "terminal", "toolwindow", "app"].includes(c.scope)).length,
  database: COMMANDS.filter((c) => c.scope.startsWith("db.")).length,
  knowledge: COMMANDS.filter((c) => ["note", "link", "graph.node"].includes(c.scope)).length,
  git: COMMANDS.filter((c) => c.scope.startsWith("git.")).length,
};

console.log("=== A6 R3B command-registry audit ===");
console.log(`commands: ${counts.total} (shell=${counts.shell}, database=${counts.database}, knowledge=${counts.knowledge}, git=${counts.git})`);
console.log(`frozen shortcuts: Ctrl+K->${SHORTCUTS["Ctrl+K"]}, Ctrl+Shift+P->${SHORTCUTS["Ctrl+Shift+P"]}`);
console.log(`GIT14 units mapped: ${Object.keys(GIT14).length}`);
console.log(`required visible Git units: ${GIT_VISIBLE.length}`);

if (failures > 0) {
  console.log(`\nRESULT: FAIL (${failures} finding(s))`);
  for (const e of errors) console.log(`  - ${e}`);
  process.exit(1);
} else {
  console.log("\nRESULT: PASS — all R3B A6 registry invariants satisfied.");
  process.exit(0);
}
