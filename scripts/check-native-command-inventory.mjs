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

// Mask comments and literals for attribute discovery, while retaining literal
// values in the separate source view used to parse cfg(target_os = "...").
// Offsets and newlines stay aligned; examples inside strings/docs are not code.
function rustAttributeViews(text) {
  const source = [];
  const syntax = [];
  let from = 0;
  let i = 0;
  const blank = value => value.replace(/[^\r\n]/g, ' ');
  const take = (end, comment) => {
    source.push(text.slice(from, i), comment ? blank(text.slice(i, end)) : text.slice(i, end));
    syntax.push(text.slice(from, i), blank(text.slice(i, end)));
    i = end;
    from = end;
  };
  while (i < text.length) {
    if (text.startsWith('//', i)) {
      const end = text.indexOf('\n', i);
      take(end < 0 ? text.length : end, true);
      continue;
    }
    if (text.startsWith('/*', i)) {
      let end = i + 2;
      let depth = 1;
      while (end < text.length && depth) {
        if (text.startsWith('/*', end)) { depth++; end += 2; }
        else if (text.startsWith('*/', end)) { depth--; end += 2; }
        else end++;
      }
      if (depth) throw new Error('Unterminated Rust block comment');
      take(end, true);
      continue;
    }
    const rawPrefix = (text[i] === 'r' && (text[i + 1] === '"' || text[i + 1] === '#'))
      || ((text[i] === 'b' || text[i] === 'c') && text[i + 1] === 'r');
    const raw = rawPrefix ? /^(?:br|cr|r)(#*)"/.exec(text.slice(i, i + 260)) : null;
    if (raw) {
      const marker = '"' + raw[1];
      const end = text.indexOf(marker, i + raw[0].length);
      if (end < 0) throw new Error('Unterminated Rust raw string');
      take(end + marker.length, false);
      continue;
    }
    if (text[i] === '"') {
      let end = i + 1;
      while (end < text.length && text[end] !== '"') {
        end += text[end] === '\\' ? 2 : 1;
      }
      if (end >= text.length) throw new Error('Unterminated Rust string');
      take(end + 1, false);
      continue;
    }
    // A lifetime such as 'a is not a character literal.
    if (text[i] === "'") {
      const character = /^'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]+\}|.)|[^'\\\r\n])'/u.exec(text.slice(i));
      if (character) { take(i + character[0].length, false); continue; }
    }
    i++;
  }
  source.push(text.slice(from));
  syntax.push(text.slice(from));
  return { source: source.join(''), syntax: syntax.join('') };
}

// Only direct target_os predicates and Rust's all/any/not operators can prove
// platform exclusivity here. Unknown/malformed cfg or cfg_attr is conservative:
// it cannot exempt a duplicate. Values come from source, not an OS allowlist.
function parseTargetOsCfg(expression) {
  let offset = 0;
  const space = () => { while (/\s/.test(expression[offset] || '') && offset < expression.length) offset++; };
  function predicate(depth = 0) {
    if (depth > 64) throw new Error('cfg nesting limit');
    space();
    const atom = /^target_os\s*=\s*"([A-Za-z0-9_-]+)"/.exec(expression.slice(offset));
    if (atom) { offset += atom[0].length; return { kind: 'os', value: atom[1] }; }
    const call = /^(all|any|not)\s*\(/.exec(expression.slice(offset));
    if (!call) throw new Error('unsupported cfg predicate');
    offset += call[0].length;
    const args = [];
    space();
    while (expression[offset] !== ')') {
      args.push(predicate(depth + 1));
      space();
      if (expression[offset] === ')') break;
      if (expression[offset] !== ',') throw new Error('invalid cfg separator');
      offset++;
      space();
    }
    offset++;
    if (call[1] === 'not' && args.length !== 1) throw new Error('invalid cfg not arity');
    return { kind: call[1], args };
  }
  try {
    const result = predicate();
    space();
    return offset === expression.length ? result : null;
  } catch {
    return null;
  }
}

function parseDefinedSource(text, file) {
  const { source, syntax } = rustAttributeViews(text);
  const attrs = [];
  const start = /#\[/g;
  let match;
  while ((match = start.exec(syntax))) {
    let end = match.index + 2;
    let depth = 1;
    while (end < syntax.length && depth) {
      if (syntax[end] === '[') depth++;
      else if (syntax[end] === ']') depth--;
      end++;
    }
    if (depth) throw new Error('Unterminated Rust attribute in ' + file);
    attrs.push({ start: match.index, end, body: source.slice(match.index + 2, end - 1).trim() });
    start.lastIndex = end;
  }
  const results = [];
  for (let index = 0; index < attrs.length; index++) {
    const command = /^tauri\s*::\s*command(?:\s*\(([\s\S]*)\))?$/.exec(attrs[index].body);
    if (!command) continue;
    let first = index;
    let last = index;
    while (first > 0 && !source.slice(attrs[first - 1].end, attrs[first].start).trim()) first--;
    while (last + 1 < attrs.length && !source.slice(attrs[last].end, attrs[last + 1].start).trim()) last++;
    const fn = /^\s*(pub(?:\s*\([^)]*\))?\s+)?(async\s+)?fn\s+([A-Za-z_]\w*)/.exec(source.slice(attrs[last].end));
    if (!fn) throw new Error('Unparsed Tauri command declaration in ' + file);
    const rename = (command[1] || '').match(/\brename\s*=\s*"([^"]+)"/)?.[1] || null;
    const cfg = [];
    let unsupportedCfg = false;
    for (const attr of attrs.slice(first, last + 1)) {
      if (/^cfg_attr\b/.test(attr.body)) unsupportedCfg = true;
      if (!/^cfg\b/.test(attr.body)) continue;
      const expression = /^cfg\s*\(([\s\S]*)\)$/.exec(attr.body);
      const parsed = expression ? parseTargetOsCfg(expression[1]) : null;
      if (parsed) cfg.push(parsed);
      else unsupportedCfg = true;
    }
    results.push({
      fn: fn[3],
      command: rename || fn[3],
      rename,
      file,
      vis: fn[1] ? 'pub' : 'private',
      async: fn[2] ? 'async' : 'sync',
      cfg: unsupportedCfg ? null : cfg,
    });
  }
  return results;
}

// DEFINED keeps every real declaration. Mutually exclusive variants are
// reconciled below; no declaration is silently dropped to match a count.
function parseDefined() {
  const results = [];
  function walk(dir) {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.rs')) {
        results.push(...parseDefinedSource(read(full), path.relative(REPO, full)));
      }
    }
  }
  walk(SRC);
  return results;
}

function cfgMatches(cfg, os) {
  if (cfg.kind === 'os') return cfg.value === os;
  if (cfg.kind === 'all') return cfg.args.every(arg => cfgMatches(arg, os));
  if (cfg.kind === 'any') return cfg.args.some(arg => cfgMatches(arg, os));
  return !cfgMatches(cfg.args[0], os);
}

function definitionsCanOverlap(left, right) {
  if (left.cfg === null || right.cfg === null) return true;
  const leftCfg = left.cfg || [];
  const rightCfg = right.cfg || [];
  // All unmentioned OS values have the same truth assignment. A null sentinel
  // represents that class, including future targets and unsupported platforms.
  const targets = new Set([null]);
  const collect = cfg => {
    if (cfg.kind === 'os') targets.add(cfg.value);
    else cfg.args.forEach(collect);
  };
  [...leftCfg, ...rightCfg].forEach(collect);
  return [...targets].some(os =>
    leftCfg.every(cfg => cfgMatches(cfg, os)) && rightCfg.every(cfg => cfgMatches(cfg, os)));
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

  const byCommand = new Map();
  for (const definition of defined) {
    if (!byCommand.has(definition.command)) byCommand.set(definition.command, []);
    byCommand.get(definition.command).push(definition);
  }
  const duplicateCommandNames = [];
  const exclusivePlatformVariants = [];
  for (const [command, variants] of byCommand) {
    if (variants.length < 2) continue;
    const overlaps = variants.some((left, i) =>
      variants.slice(i + 1).some(right => definitionsCanOverlap(left, right)));
    (overlaps ? duplicateCommandNames : exclusivePlatformVariants).push(command);
  }
  duplicateCommandNames.sort();
  exclusivePlatformVariants.sort();

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
    exclusivePlatformVariants,
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

function runSelfTest({ quiet = false } = {}) {
  let failures = 0;
  const assert = (cond, msg) => {
    if (!cond) {
      failures++;
      console.error('  ✗ ' + msg);
    } else if (!quiet) {
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

  // Exercise source extraction and Boolean overlap together. These fixtures
  // use arbitrary command/OS names; no production command is exempted by name.
  const declaration = (cfg, name = 'platform_probe') =>
    cfg + '\n#[tauri::command]\npub fn ' + name + '() {}\n';
  const fromSource = source => {
    const defs = parseDefinedSource(source, 'cfg-fixture.rs');
    const names = [...new Set(defs.map(definition => definition.command))];
    return { defs, report: reconcile(
      names.map(name => ({ name, prefix: 'bridge', raw: 'bridge::' + name })),
      defs, names, known,
    ) };
  };
  const hasDuplicate = source => fromSource(source).report.duplicateCommandNames.length > 0;
  const linux = declaration('#[cfg(target_os = "linux")]');
  const windows = declaration('#[cfg(target_os = "windows")]');
  const fallback = declaration('#[cfg(not(any(target_os = "linux", target_os = "windows")))]');
  const exclusive = fromSource(linux + windows + fallback);
  assert(exclusive.defs.length === 3, 'cfg: retain all three platform declarations');
  assert(exclusive.report.duplicateCommandNames.length === 0
    && exclusive.report.exclusivePlatformVariants.join(',') === 'platform_probe',
  'cfg: Linux, Windows and the complete fallback are mutually exclusive');
  assert(hasDuplicate(linux + linux), 'cfg: same target_os still conflicts');
  assert(hasDuplicate(linux + declaration('')), 'cfg: unconditional and conditional definitions conflict');
  assert(hasDuplicate(linux + declaration('#[cfg(not(target_os = "windows"))]')),
    'cfg: not(windows) overlaps Linux');
  assert(hasDuplicate(windows + declaration('#[cfg(any(target_os = "linux", target_os = "windows"))]')),
    'cfg: any(...) overlap is rejected');
  assert(!hasDuplicate(windows + declaration(
    '#[cfg(all(not(target_os = "windows"), any(target_os = "linux", target_os = "future_os")))]')),
  'cfg: nested Boolean predicates are evaluated without an OS allowlist');
  assert(!hasDuplicate(declaration('#[cfg(target_os = "future_os")]') + declaration(
    '#[cfg(not(target_os = "future_os"))]')),
  'cfg: targets absent from the host remain part of the proof');
  assert(hasDuplicate(declaration('#[cfg(not(target_os = "linux"))]') + declaration(
    '#[cfg(not(target_os = "windows"))]')),
  'cfg: unmentioned targets expose overlap between two negative predicates');
  const adjacent = '#[tauri::command]\n#[cfg(\n all(\n target_os = "linux",\n not(target_os = "windows"),\n )\n)]\n'
    + '#[cfg(not(target_os = "future_os"))]\npub async fn platform_probe() {}\n';
  assert(!hasDuplicate(adjacent + windows + fallback),
    'cfg: multiline attributes on either side of the command are conjoined');
  assert(hasDuplicate(windows + declaration('#[cfg(feature = "optional")]')),
    'cfg: unknown feature constraints cannot prove exclusivity');
  assert(hasDuplicate(windows + declaration('#[cfg_attr(feature = "optional", cfg(target_os = "linux"))]')),
    'cfg: conditional attributes cannot silently exempt a definition');
  assert(hasDuplicate(windows + declaration('#[cfg(not(target_os = "windows", target_os = "linux"))]')),
    'cfg: malformed Boolean predicates fail closed');
  assert(hasDuplicate(windows + '#[cfg(target_os = "linux")]\nfn helper() {}\n' + declaration('')),
    'cfg: a helper function attribute does not leak to a later command');
  const commented = '// #[cfg(target_os = "linux")]\n/* #[cfg(target_os = "linux")] /* nested */ */\n'
    + declaration('');
  assert(hasDuplicate(windows + commented), 'cfg: comments cannot grant platform exclusivity');
  const examples = 'const SAMPLE: &str = r##"#[cfg(target_os = "linux")]\n'
    + '#[tauri::command]\nfn fake_command() {}"##;\n'
    + 'const STRING: &str = "#[tauri::command]";\n'
    + declaration('');
  assert(fromSource(examples).defs.length === 1 && hasDuplicate(windows + examples),
    'cfg: attributes and commands in raw/ordinary strings are ignored');
  const renamed = '#[cfg(target_os = "windows")]\n#[tauri::command(rename = "platform_probe")]\n'
    + 'pub fn another_name() {}\n';
  assert(hasDuplicate(windows + renamed), 'cfg: renamed overlapping commands still conflict');
  assert(!hasDuplicate(declaration('#[cfg(any())]') + declaration('#[cfg(all())]')),
    'cfg: Rust empty any/all predicates retain their Boolean semantics');

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
    if (!quiet) console.log('SELF_TEST: PASS');
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
  // The normal authority gate also executes these dependency-free fixtures.
  // Quiet mode preserves the --json interface consumed by semantic checks.
  if (!runSelfTest({ quiet: true })) process.exit(1);

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
    console.log('EXCLUSIVE_PLATFORM_VARIANTS:', reg.exclusivePlatformVariants.join(', ') || '(none)');
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
