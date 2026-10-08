#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const REGISTRY = path.join(ROOT, "docs/architecture/native-boundary/native-commands.yaml");
const GOVERNANCE = path.join(ROOT, "docs/engineering/governance.json");

function parseRegistry(text) {
  const commands = [];
  let current = null;
  for (const line of text.split(/\r?\n/)) {
    const id = line.match(/^  ([a-z_]\w*):\s*$/);
    if (id) {
      current = { name: id[1] };
      commands.push(current);
      continue;
    }
    const field = line.match(/^    (file|owner|category|side_effect|resource|permission|allowed_callers):\s*(.*)$/);
    if (field && current) current[field[1]] = field[2].trim().replace(/^"|"$/g, "");
  }
  return commands;
}

function validateRegistry(commands) {
  const errors = [];
  const seen = new Set();
  for (const command of commands) {
    if (seen.has(command.name)) errors.push(`duplicate semantic command: ${command.name}`);
    seen.add(command.name);
    for (const field of ["file", "owner", "category", "side_effect", "resource", "permission"]) {
      if (!command[field]) errors.push(`${command.name}: missing ${field}`);
    }
    if (command.owner === "UNKNOWN") errors.push(`${command.name}: UNKNOWN owner`);
  }
  return errors;
}

function inventory() {
  const child = spawnSync(process.execPath, ["scripts/check-native-command-inventory.mjs", "--json"], {
    cwd: ROOT,
    encoding: "utf8",
  });
  if (child.status !== 0) throw new Error(child.stderr || child.stdout || "native command inventory failed");
  return JSON.parse(child.stdout);
}

function buildReport(inv, commands) {
  const owners = {};
  const resources = {};
  const permissions = {};
  for (const command of commands) {
    owners[command.owner] = (owners[command.owner] || 0) + 1;
    resources[command.resource] = (resources[command.resource] || 0) + 1;
    permissions[command.permission] = (permissions[command.permission] || 0) + 1;
  }
  return {
    schemaVersion: 1,
    authorities: {
      rustRegistration: "src-tauri/src/main.rs::generate_handler!",
      rustDefinitions: "src-tauri/src/**/*.rs::#[tauri::command]",
      nativeSemantics: "docs/architecture/native-boundary/native-commands.yaml",
      stateOwnership: "docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md",
      frontendIpc: "src/bridge.ts + scripts/check-command-set-consistency.py",
    },
    counts: inv.counts,
    gate: inv.gate,
    appState: {
      structFields: inv.appState.structFields,
      matrixFields: inv.appState.matrixFields,
    },
    owners: Object.fromEntries(Object.entries(owners).sort()),
    resources: Object.fromEntries(Object.entries(resources).sort()),
    permissions: Object.fromEntries(Object.entries(permissions).sort()),
    commands: commands
      .map(({ name, owner, category, resource, permission, side_effect, file, allowed_callers }) => ({
        name, owner, category, resource, permission, sideEffect: side_effect, declaredFile: file,
        ...(allowed_callers ? { allowedCallers: allowed_callers } : {}),
      }))
      .sort((a, b) => a.name.localeCompare(b.name)),
  };
}

function runSelfTest() {
  const good = parseRegistry(`commands:
  hello:
    file: "src-tauri/src/x.rs"
    owner: "framework"
    category: "FRAMEWORK_NATIVE_SERVICE"
    side_effect: "none"
    resource: "NONE"
    permission: "-"
`);
  const bad = parseRegistry(`commands:
  broken:
    owner: "UNKNOWN"
`);
  if (good.length !== 1 || validateRegistry(good).length !== 0) return false;
  const errors = validateRegistry(bad);
  return errors.some((x) => x.includes("UNKNOWN owner")) && errors.some((x) => x.includes("missing resource"));
}

if (process.argv.includes("--self-test")) {
  const ok = runSelfTest();
  console.log(`NATIVE_SEMANTIC_SELF_TEST=${ok ? "PASS" : "FAIL"}`);
  process.exit(ok ? 0 : 1);
}

let inv;
try {
  inv = inventory();
} catch (error) {
  console.error("NATIVE_SEMANTIC_RESULT=FAIL " + error.message);
  process.exit(1);
}

const commands = parseRegistry(fs.readFileSync(REGISTRY, "utf8"));
const errors = validateRegistry(commands);
const governance = JSON.parse(fs.readFileSync(GOVERNANCE, "utf8"));
const expected = governance.nativeSemantics || {};

if (!inv.gate.PASS) errors.push("existing native command inventory is not closed");
if (commands.length !== inv.counts.TOTAL_REGISTRY) errors.push("semantic registry parser count differs from native inventory");
if (expected.expectedRegisteredCommands !== inv.counts.TOTAL_REGISTERED) {
  errors.push(`registered command count drift: expected ${expected.expectedRegisteredCommands}, actual ${inv.counts.TOTAL_REGISTERED}`);
}
if (expected.expectedAppStateFields !== inv.counts.APPSTATE_STRUCT_FIELDS) {
  errors.push(`AppState field count drift: expected ${expected.expectedAppStateFields}, actual ${inv.counts.APPSTATE_STRUCT_FIELDS}`);
}

const report = buildReport(inv, commands);
const reportArg = process.argv.indexOf("--report");
if (reportArg >= 0) {
  const target = process.argv[reportArg + 1];
  if (!target) errors.push("--report requires a path");
  else {
    const abs = path.resolve(ROOT, target);
    fs.mkdirSync(path.dirname(abs), { recursive: true });
    fs.writeFileSync(abs, JSON.stringify(report, null, 2) + "\n");
  }
}
if (process.argv.includes("--json")) console.log(JSON.stringify(report, null, 2));
console.log(`NATIVE_SEMANTIC_RESULT=${errors.length ? "FAIL" : "PASS"} commands=${commands.length} appState=${inv.counts.APPSTATE_STRUCT_FIELDS}`);
for (const error of errors) console.error("- " + error);
process.exit(errors.length ? 1 : 0);
