#!/usr/bin/env node
// check-native-command-inventory.mjs
//
// Native Command Freeze Audit — deterministic reconciliation of three sources of
// truth for Tauri commands, plus AppState field-ownership matrix revalidation.
//
//   REGISTERED  = generate_handler![...] entries in src-tauri/src/main.rs
//   DEFINED     = #[tauri::command] fn in src-tauri/src/**/*.rs
//   REGISTRY    = docs/architecture/native-boundary/native-commands.yaml
//
// Gate (freeze preconditions, see Native Command Freeze Audit req):
//   COMMAND_UNKNOWN      = 0  (REGISTERED_NOT_DEFINED + UNKNOWN_DEFINITION_LOCATION)
//   COMMAND_REGISTRY_DRIFT = 0 (REGISTERED_NOT_IN_REGISTRY + REGISTRY_NOT_REGISTERED)
//   APPSTATE_UNKNOWN     = 0  (MISSING_MATRIX_FIELD + STALE_MATRIX_FIELD)
//   DUPLICATE_COMMAND    = 0
//
// Modes:
//   (default)            scan the real repo, print report, exit 0 iff gate PASS
//   --self-test          run positive/negative fixtures, assert detector works
//   --json               emit machine-readable JSON only

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '..');
const SRC = path.join(REPO, 'src-tauri', 'src');
const MAIN_RS = path.join(SRC, 'main.rs');
const BRIDGE_RS = path.join(SRC, 'bridge.rs');
const YAML = path.join(REPO, 'docs', 'architecture', 'native-boundary', 'native-commands.yaml');
const MATRIX = path.join(
  REPO,
  'docs',
  'architecture',
  'native-physical-boundary',
  'NATIVE-PHYSICAL-BOUNDARY-MATRIX.md',
);

function read(p) {
  return fs.readFileSync(p, 'utf8');
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

// REGISTERED: every token inside generate_handler![ ... ] (both handlers).
function parseRegistered(text) {
  const out = [];
  const re = /generate_handler!\[([\s\S]*?)\]/g;
  let m;
  while ((m = re.exec(text))) {
    const body = m[1].replace(/\/\/[^\n]*/g, ''); // strip line comments
    const tokens = body
      .split(',')
      .map((s) => s.trim())
      .filter((s) => /^[\w]+(::[\w]+)*$/.test(s));
    for (const t of tokens) {
      const parts = t.split('::');
      const name = parts[parts.length - 1];
      const prefix = parts.length > 1 ? parts.slice(0, -1).join('::') : 'bridge';
      out.push({ name, prefix, raw: t });
    }
  }
  // unique by command name (a command registered in both handlers still = one)
  const seen = new Map();
  for (const r of out) if (!seen.has(r.name)) seen.set(r.name, r);
  return [...seen.values()];
}

// DEFINED: #[tauri::command] followed by the fn signature.
function parseDefined() {
  const results = [];
  function walk(dir) {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.rs')) {
        const text = read(full);
        const attrRe = /#\[tauri::command(?:\(([^\)]*)\))?\]/g;
        let am;
        while ((am = attrRe.exec(text))) {
          // Skip literal mentions of `#[tauri::command]` inside comment lines
          // (e.g. `//! 本模块不含 #[tauri::command]`), which are not real attributes.
          const lineStart = text.lastIndexOf('\n', am.index) + 1;
          const lineSoFar = text.slice(lineStart, am.index).trimStart();
          if (lineSoFar.startsWith('//')) continue;

          const inner = am[1] || '';
          const renameM = inner.match(/rename\s*=\s*"([^"]+)"/);
          const rename = renameM ? renameM[1] : null;
          const after = text.slice(am.index);
          const fnM = after.match(/\n\s*(pub\s+)?(async\s+)?fn\s+([A-Za-z_]\w*)/);
          if (!fnM) continue;
          const vis = fnM[1] ? 'pub' : 'private';
          const async = fnM[2] ? 'async' : 'sync';
          const fn = fnM[3];
          results.push({
            fn,
            command: rename || fn,
            rename,
            file: path.relative(REPO, full),
            vis,
            async,
          });
        }
      }
    }
  }
  walk(SRC);
  return results;
}

// REGISTRY: 2-space command keys under `commands:` in the YAML.
function parseRegistry(text) {
  const keys = [];
  const lines = text.split('\n');
  let inCommands = false;
  for (const line of lines) {
    if (/^commands:\s*$/.test(line)) {
      inCommands = true;
      continue;
    }
    if (inCommands) {
      if (/^\S/.test(line) && !/^commands:/.test(line)) break; // next top-level key
      const m = line.match(/^  ([A-Za-z_]\w*):\s*$/);
      if (m) keys.push(m[1]);
    }
  }
  return keys;
}

// Known module aliases declared in main.rs (for UNKNOWN_DEFINITION_LOCATION).
function parseKnownModules(text) {
  const set = new Set(['bridge']);
  for (const m of text.matchAll(/pub mod\s+([A-Za-z_]\w*)/g)) set.add(m[1]);
  for (const m of text.matchAll(/pub use\s+([^;]+);/g)) {
    const tail = m[1].trim();
    const asM = tail.match(/\bas\s+([A-Za-z_]\w*)\s*$/);
    if (asM) set.add(asM[1]);
    else {
      const last = tail.split('::').filter(Boolean).pop();
      if (last) set.add(last);
    }
  }
  return set;
}

// AppState field names from `pub struct AppState { ... }`.
function parseAppState(text) {
  const m = text.match(/pub struct AppState\s*\{([\s\S]*?)\n\}/);
  if (!m) return [];
  return [...m[1].matchAll(/pub\s+([A-Za-z_]\w*)\s*:/g)].map((x) => x[1]);
}

// AppState field names recorded in matrix §2.5 table (first backticked column).
function parseMatrixFields(text) {
  const lines = text.split('\n');
  let start = -1;
  let end = -1;
  for (let i = 0; i < lines.length; i++) {
    if (/^###?\s*2\.5|2\.5 AppState/.test(lines[i])) start = i;
    else if (start >= 0 && /^\s*#{2,3}\s/.test(lines[i]) && i > start) {
      end = i;
      break;
    }
  }
  if (start < 0) return [];
  const slice = end < 0 ? lines.slice(start) : lines.slice(start, end);
  const fields = [];
  const re = /\|\s*`([A-Za-z_]\w*)`\s*\|/g;
  for (const line of slice) {
    let mm;
    while ((mm = re.exec(line))) fields.push(mm[1]);
  }
  return fields;
}

// ---------------------------------------------------------------------------
// Reconciliation
// ---------------------------------------------------------------------------

function reconcile(registered, defined, registry, knownModules) {
  const regNames = new Set(registered.map((r) => r.name));
  const defCmds = new Set(defined.map((d) => d.command));
  const regKeys = new Set(registry);

  const registeredNotDefined = [...regNames].filter((n) => !defCmds.has(n)).sort();
  const definedNotRegistered = [...defCmds].filter((n) => !regNames.has(n)).sort();
  const registeredNotInRegistry = [...regNames].filter((n) => !regKeys.has(n)).sort();
  const registryNotRegistered = [...regKeys].filter((n) => !regNames.has(n)).sort();

  const counts = {};
  for (const d of defined) counts[d.command] = (counts[d.command] || 0) + 1;
  const duplicateCommandNames = Object.entries(counts)
    .filter(([, v]) => v > 1)
    .map(([k]) => k)
    .sort();

  const unknownDefinitionLocation = registered
    .filter((r) => !defCmds.has(r.name) && !knownModules.has(r.prefix))
    .map((r) => r.raw)
    .sort();

  return {
    registeredNotDefined,
    definedNotRegistered,
    registeredNotInRegistry,
    registryNotRegistered,
    duplicateCommandNames,
    unknownDefinitionLocation,
  };
}

function reconcileAppState(structFields, matrixFields) {
  const s = new Set(structFields);
  const m = new Set(matrixFields);
  const missingMatrixField = [...s].filter((f) => !m.has(f)).sort();
  const staleMatrixField = [...m].filter((f) => !s.has(f)).sort();
  return { missingMatrixField, staleMatrixField };
}

function summarize(reg, appDrift, counts) {
  const drift =
    reg.registeredNotInRegistry.length + reg.registryNotRegistered.length;
  const unknown =
    reg.registeredNotDefined.length + reg.unknownDefinitionLocation.length;
  const dup = reg.duplicateCommandNames.length;
  const app = appDrift.missingMatrixField.length + appDrift.staleMatrixField.length;
  return {
    drift,
    unknown,
    dup,
    app,
    PASS: unknown === 0 && drift === 0 && dup === 0 && app === 0,
  };
}

// ---------------------------------------------------------------------------
// Self-test (positive + negative fixtures)
// ---------------------------------------------------------------------------

function runSelfTest() {
  let failures = 0;
  const assert = (cond, msg) => {
    if (!cond) {
      failures++;
      console.error('  ✗ ' + msg);
    } else {
      console.log('  ✓ ' + msg);
    }
  };

  const posDefined = [
    { command: 'a', fn: 'a', file: 'x', vis: 'pub', async: 'sync' },
    { command: 'b', fn: 'b', file: 'x', vis: 'pub', async: 'sync' },
  ];
  const posReg = [
    { name: 'a', prefix: 'bridge', raw: 'bridge::a' },
    { name: 'b', prefix: 'bridge', raw: 'bridge::b' },
  ];
  const posReg2 = ['a', 'b'];
  const known = new Set(['bridge']);
  const posRec = reconcile(posReg, posDefined, posReg2, known);
  assert(posRec.registeredNotDefined.length === 0, 'positive: no REGISTERED_NOT_DEFINED');
  assert(posRec.definedNotRegistered.length === 0, 'positive: no DEFINED_NOT_REGISTERED');
  assert(posRec.registeredNotInRegistry.length === 0, 'positive: no REGISTERED_NOT_IN_REGISTRY');
  assert(posRec.registryNotRegistered.length === 0, 'positive: no REGISTRY_NOT_REGISTERED');
  assert(posRec.duplicateCommandNames.length === 0, 'positive: no DUPLICATE');
  const posApp = reconcileAppState(['f1', 'f2'], ['f1', 'f2']);
  assert(posApp.missingMatrixField.length === 0, 'positive: no MISSING matrix field');
  assert(posApp.staleMatrixField.length === 0, 'positive: no STALE matrix field');

  // negative: b registered but not defined; c in registry but not registered
  const negDefined = [
    { command: 'a', fn: 'a', file: 'x', vis: 'pub', async: 'sync' },
  ];
  const negReg = [
    { name: 'a', prefix: 'bridge', raw: 'bridge::a' },
    { name: 'b', prefix: 'ghost', raw: 'ghost::b' },
  ];
  const negReg2 = ['a', 'c'];
  const negRec = reconcile(negReg, negDefined, negReg2, known);
  assert(
    JSON.stringify(negRec.registeredNotDefined) === JSON.stringify(['b']),
    'negative: REGISTERED_NOT_DEFINED = [b]',
  );
  assert(
    JSON.stringify(negRec.registeredNotInRegistry) === JSON.stringify(['b']),
    'negative: REGISTERED_NOT_IN_REGISTRY = [b]',
  );
  assert(
    JSON.stringify(negRec.registryNotRegistered) === JSON.stringify(['c']),
    'negative: REGISTRY_NOT_REGISTERED = [c]',
  );
  assert(
    negRec.unknownDefinitionLocation.includes('ghost::b'),
    'negative: UNKNOWN_DEFINITION_LOCATION flags ghost::b',
  );

  // negative: duplicate command name
  const dupDefined = [
    { command: 'z', fn: 'z', file: 'p', vis: 'pub', async: 'sync' },
    { command: 'z', fn: 'z', file: 'q', vis: 'pub', async: 'sync' },
  ];
  const dupRec = reconcile([{ name: 'z', prefix: 'bridge', raw: 'bridge::z' }], dupDefined, ['z'], known);
  assert(
    JSON.stringify(dupRec.duplicateCommandNames) === JSON.stringify(['z']),
    'negative: DUPLICATE_COMMAND_NAMES = [z]',
  );

  // negative: appstate drift
  const negApp = reconcileAppState(['f1', 'f3'], ['f1', 'f2']);
  assert(
    JSON.stringify(negApp.missingMatrixField) === JSON.stringify(['f3']),
    'negative: MISSING_MATRIX_FIELD = [f3]',
  );
  assert(
    JSON.stringify(negApp.staleMatrixField) === JSON.stringify(['f2']),
    'negative: STALE_MATRIX_FIELD = [f2]',
  );

  if (failures === 0) {
    console.log('SELF_TEST: PASS');
    return true;
  }
  console.error(`SELF_TEST: FAIL (${failures} assertion(s))`);
  return false;
}

// ---------------------------------------------------------------------------
// Real scan + report
// ---------------------------------------------------------------------------

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--self-test')) {
    const ok = runSelfTest();
    process.exit(ok ? 0 : 1);
  }

  const mainText = read(MAIN_RS);
  const yamlText = read(YAML);
  const matrixText = read(MATRIX);

  const registered = parseRegistered(mainText);
  const defined = parseDefined();
  const registry = parseRegistry(yamlText);
  const knownModules = parseKnownModules(mainText);

  const reg = reconcile(registered, defined, registry, knownModules);

  const structFields = parseAppState(read(BRIDGE_RS));
  const matrixFields = parseMatrixFields(matrixText);
  const appDrift = reconcileAppState(structFields, matrixFields);

  const counts = {
    TOTAL_REGISTERED: registered.length,
    TOTAL_DEFINED: defined.length,
    TOTAL_DEFINED_DISTINCT: new Set(defined.map((d) => d.command)).size,
    TOTAL_REGISTRY: registry.length,
    APPSTATE_STRUCT_FIELDS: structFields.length,
    APPSTATE_MATRIX_FIELDS: matrixFields.length,
  };

  const sum = summarize(reg, appDrift, counts);

  if (args.includes('--json')) {
    console.log(
      JSON.stringify(
        {
          counts,
          reconciliation: reg,
          appState: { structFields, matrixFields, ...appDrift },
          gate: sum,
        },
        null,
        2,
      ),
    );
  } else {
    console.log('=== NATIVE COMMAND INVENTORY ===');
    console.log('REGISTERED (generate_handler!):', counts.TOTAL_REGISTERED);
    console.log('DEFINED (#[tauri::command]):', counts.TOTAL_DEFINED, '(distinct names:', counts.TOTAL_DEFINED_DISTINCT + ')');
    console.log('REGISTRY (yaml):', counts.TOTAL_REGISTRY);
    console.log('');
    console.log('--- THREE-WAY RECONCILIATION ---');
    console.log('REGISTERED_NOT_DEFINED     :', reg.registeredNotDefined.join(', ') || '(none)');
    console.log('DEFINED_NOT_REGISTERED     :', reg.definedNotRegistered.join(', ') || '(none)');
    console.log('REGISTERED_NOT_IN_REGISTRY :', reg.registeredNotInRegistry.join(', ') || '(none)');
    console.log('REGISTRY_NOT_REGISTERED    :', reg.registryNotRegistered.join(', ') || '(none)');
    console.log('DUPLICATE_COMMAND_NAMES    :', reg.duplicateCommandNames.join(', ') || '(none)');
    console.log('UNKNOWN_DEFINITION_LOCATION:', reg.unknownDefinitionLocation.join(', ') || '(none)');
    console.log('');
    console.log('--- APPSTATE MATRIX ---');
    console.log('STRUCT_FIELDS :', counts.APPSTATE_STRUCT_FIELDS);
    console.log('MATRIX_FIELDS :', counts.APPSTATE_MATRIX_FIELDS);
    console.log('MISSING_MATRIX_FIELD :', appDrift.missingMatrixField.join(', ') || '(none)');
    console.log('STALE_MATRIX_FIELD   :', appDrift.staleMatrixField.join(', ') || '(none)');
    console.log('');
    console.log('--- GATE ---');
    console.log('COMMAND_UNKNOWN       :', sum.unknown);
    console.log('COMMAND_REGISTRY_DRIFT:', sum.drift);
    console.log('DUPLICATE_COMMAND     :', sum.dup);
    console.log('APPSTATE_UNKNOWN      :', sum.app);
    console.log('GATE_PASS             :', sum.PASS ? 'YES' : 'NO');
  }

  process.exit(sum.PASS ? 0 : 2);
}

main();
