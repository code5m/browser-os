#!/usr/bin/env node
// Architecture boundary checker (Phase 03).
//
// Enforces the component → bridge → invoke boundary documented in
// AGENTS.md §6 and docs/AI/00-Architecture.md §2.1.
//
// Checks:
//   A1  Components/stores/composables/utils must not import invoke.        (P1)
//   A2  Components/stores/composables/utils must not call invoke().        (P1)
//   A3  src/bridge.ts is the sole invoke import point in src/.             (P1)
//   A4  Stores must not import native Tauri modules directly.              (WARN legacy)
//   A5  Components must not bypass the bridge boundary.                    (WARN legacy)
//   A6  Direct __TAURI__ global access is a bridge bypass.                 (WARN legacy)
//   A7  BrowserRuntime/MockRuntime/BrowserScene/syncScene targets.         (WARN not-yet)
//   A8  mvp_core boundary delegated to check-core-boundary.py.             (WARN if missing)
//
// CLI:
//   node scripts/check-architecture.mjs
//   node scripts/check-architecture.mjs --help
//   node scripts/check-architecture.mjs --json
//   node scripts/check-architecture.mjs --strict
//   node scripts/check-architecture.mjs --self-test
//
// Exit codes:
//   0  PASS         no blocking findings
//   1  FAIL         one or more blocking findings
//   2  USAGE_ERROR  invalid CLI arguments

import {
  readFileSync,
  readdirSync,
  statSync,
  existsSync,
} from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../", import.meta.url));
const CHECK_NAME = "ARCHITECTURE";

// ---------------------------------------------------------------------------
// File scanning helpers
// ---------------------------------------------------------------------------

function walkDir(dir, exts, excludePrefixes = []) {
  const results = [];
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return results;
  }
  for (const entry of entries) {
    const full = join(dir, entry);
    const rel = relative(ROOT, full);
    if (excludePrefixes.some((p) => rel === p || rel.startsWith(p + "/"))) continue;
    let stat;
    try {
      stat = statSync(full);
    } catch {
      continue;
    }
    if (stat.isDirectory()) {
      results.push(...walkDir(full, exts, excludePrefixes));
    } else if (exts.some((ext) => full.endsWith(ext))) {
      results.push(full);
    }
  }
  return results.sort();
}

function readFileContent(filePath) {
  return readFileSync(filePath, "utf8");
}

// Extract import statements as { module, names, line }.
// Skips single-line comment lines. Handles `import { a, b as c } from "m"`
// and `import defaultExport from "m"` and `import { type a, b } from "m"`.
function extractImports(content) {
  const imports = [];
  const lines = content.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const trimmed = raw.trimStart();
    if (trimmed.startsWith("//") || trimmed.startsWith("*")) continue;
    const match = raw.match(
      /^\s*import\s+(?:type\s+)?(?:\{([^}]*)\}|(\w+))\s+from\s+["']([^"']+)["']/,
    );
    if (!match) continue;
    const namesStr = match[1];
    const defaultName = match[2];
    const module = match[3];
    let names = [];
    if (namesStr) {
      names = namesStr
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean)
        .map((s) => s.replace(/^type\s+/, "").split(/\s+as\s+/)[0].trim());
    } else if (defaultName) {
      names = [defaultName];
    }
    imports.push({ module, names, line: i + 1 });
  }
  return imports;
}

// Return non-comment lines with their 1-based line numbers.
// Handles `//` single-line and `/* */` multi-line comments.
function getNonCommentLines(content) {
  const lines = content.split("\n");
  const result = [];
  let inBlock = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    let visible = "";
    let j = 0;
    while (j < line.length) {
      if (inBlock) {
        if (line[j] === "*" && line[j + 1] === "/") {
          inBlock = false;
          j += 2;
        } else {
          j++;
        }
      } else {
        if (line[j] === "/" && line[j + 1] === "/") {
          break;
        } else if (line[j] === "/" && line[j + 1] === "*") {
          inBlock = true;
          j += 2;
        } else {
          visible += line[j];
          j++;
        }
      }
    }
    if (visible.trim()) result.push({ line: i + 1, text: visible });
  }
  return result;
}

function rel(filePath) {
  return relative(ROOT, filePath);
}

// ---------------------------------------------------------------------------
// Rules — each returns an array of findings.
// ---------------------------------------------------------------------------

// A1/A3: No `invoke` import outside bridge.ts.
function checkInvokeImport(content, filePath, isBridge) {
  if (isBridge) return [];
  const findings = [];
  for (const imp of extractImports(content)) {
    if (imp.module === "@tauri-apps/api/core" && imp.names.includes("invoke")) {
      findings.push({
        severity: "P1",
        file: rel(filePath),
        line: imp.line,
        message:
          'Direct "invoke" import from @tauri-apps/api/core; must route through src/bridge.ts',
        rule: "AGENTS.md §6 / 00-Architecture.md §2.1",
      });
    }
  }
  return findings;
}

// A2: No invoke() call outside bridge.ts.
function checkInvokeCall(content, filePath, isBridge) {
  if (isBridge) return [];
  const findings = [];
  for (const { line, text } of getNonCommentLines(content)) {
    if (/^\s*import\s/.test(text)) continue;
    // Match invoke( preceded by start-of-string or a non-identifier non-dot char.
    if (/(^|[^\w.])invoke\s*\(/.test(text)) {
      findings.push({
        severity: "P1",
        file: rel(filePath),
        line,
        message: "Direct invoke() call; must route through src/bridge.ts",
        rule: "AGENTS.md §6 / 00-Architecture.md §2.1",
      });
    }
  }
  return findings;
}

// A4/A5: Non-invoke Tauri API import outside bridge.ts (legacy WARN).
function checkNonInvokeTauriImport(content, filePath, isBridge) {
  if (isBridge) return [];
  const warnings = [];
  for (const imp of extractImports(content)) {
    if (!imp.module.startsWith("@tauri-apps/")) continue;
    // invoke import is handled by A1/A3; skip it here.
    if (imp.module === "@tauri-apps/api/core" && imp.names.every((n) => n === "invoke")) continue;
    warnings.push({
      severity: "WARN",
      file: rel(filePath),
      line: imp.line,
      message: `Direct Tauri API import (${imp.module}:${imp.names.join(", ")}); route through src/bridge.ts to keep the IPC boundary unified`,
      rule: "00-Architecture.md §2.1 (bridge boundary)",
    });
  }
  return warnings;
}

// A6: Direct __TAURI__ / __TAURI_INTERNALS__ global access (legacy WARN).
function checkTauriGlobalAccess(content, filePath) {
  const warnings = [];
  for (const { line, text } of getNonCommentLines(content)) {
    if (/__TAURI_?INTERNALS?__|__TAURI__/.test(text)) {
      warnings.push({
        severity: "WARN",
        file: rel(filePath),
        line,
        message:
          "Direct access to Tauri internal global (__TAURI__); route through src/bridge.ts",
        rule: "00-Architecture.md §2.1 (bridge boundary)",
      });
    }
  }
  return warnings;
}

// ---------------------------------------------------------------------------
// Global checks (not per-file)
// ---------------------------------------------------------------------------

// A7: BrowserRuntime / MockRuntime / BrowserScene / syncScene /
//     WebViewSafeShell / BrowserViewportAnchor — APPROVED_TARGET, not yet in source.
function checkApprovedTargets(srcFiles) {
  const warnings = [];
  const targets = [
    "BrowserRuntime",
    "MockRuntime",
    "BrowserScene",
    "syncScene",
    "WebViewSafeShell",
    "BrowserViewportAnchor",
  ];
  const symbolRegex = new RegExp(`\\b(${targets.join("|")})\\b`);
  const found = new Set();
  for (const filePath of srcFiles) {
    let content;
    try {
      content = readFileContent(filePath);
    } catch {
      continue;
    }
    const matches = content.match(new RegExp(`\\b(${targets.join("|")})\\b`, "g"));
    if (matches) for (const m of matches) found.add(m);
  }
  for (const target of targets) {
    if (!found.has(target)) {
      warnings.push({
        severity: "WARN",
        file: "src/",
        line: 0,
        message: `${target} is APPROVED_TARGET but not yet in source (0 hits); bridge.ts remains the IPC boundary`,
        rule: "00-Architecture.md §2.2 (APPROVED_TARGET)",
      });
    }
  }
  return warnings;
}

// A8: mvp_core boundary is delegated to scripts/check-core-boundary.py.
function checkCoreBoundaryDelegate() {
  const warnings = [];
  const checkerPath = join(ROOT, "scripts", "check-core-boundary.py");
  if (!existsSync(checkerPath)) {
    warnings.push({
      severity: "WARN",
      file: "scripts/check-core-boundary.py",
      line: 0,
      message:
        "mvp_core boundary checker (check-core-boundary.py) is missing; mvp_core boundary (R-B1..R-B5) is unguarded",
      rule: "00-Architecture.md §2.1 (library boundary)",
    });
  }
  return warnings;
}

// ---------------------------------------------------------------------------
// Scan orchestration
// ---------------------------------------------------------------------------

function collectSrcFiles() {
  const srcDir = join(ROOT, "src");
  return walkDir(srcDir, [".vue", ".ts", ".mjs"], ["src/styles"]);
}

function runScan() {
  const srcFiles = collectSrcFiles();
  const findings = [];
  const warnings = [];

  const bridgePath = join(ROOT, "src", "bridge.ts");
  const bridgeExists = existsSync(bridgePath);

  if (!bridgeExists) {
    findings.push({
      severity: "P1",
      file: "src/bridge.ts",
      line: 0,
      message:
        "src/bridge.ts is missing; the unified IPC boundary is gone (AGENTS.md §6)",
      rule: "AGENTS.md §6 / 00-Architecture.md §2.1",
    });
  }

  for (const filePath of srcFiles) {
    const relPath = rel(filePath);
    const isBridge = relPath === "src/bridge.ts";
    let content;
    try {
      content = readFileContent(filePath);
    } catch {
      continue;
    }
    findings.push(...checkInvokeImport(content, filePath, isBridge));
    findings.push(...checkInvokeCall(content, filePath, isBridge));
    warnings.push(...checkNonInvokeTauriImport(content, filePath, isBridge));
    warnings.push(...checkTauriGlobalAccess(content, filePath));
  }

  warnings.push(...checkApprovedTargets(srcFiles));
  warnings.push(...checkCoreBoundaryDelegate());

  findings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
  warnings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);

  return { findings, warnings, filesScanned: srcFiles.length };
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

function selfTest() {
  const tests = [];
  const fakePath = join(ROOT, "src", "components", "__selftest__.vue");
  const ok = (name, cond) => tests.push({ name, pass: !!cond });

  // 1. invoke import → P1
  const f1 = checkInvokeImport(
    'import { invoke } from "@tauri-apps/api/core";',
    fakePath,
    false,
  );
  ok("invoke import detected as P1", f1.length === 1 && f1[0].severity === "P1");

  // 2. invoke() call → P1
  const f2 = checkInvokeCall('const r = await invoke("cmd");', fakePath, false);
  ok("invoke() call detected as P1", f2.length === 1 && f2[0].severity === "P1");

  // 3. bridge.ts invoke import allowed
  const f3 = checkInvokeImport(
    'import { invoke, Channel } from "@tauri-apps/api/core";',
    join(ROOT, "src", "bridge.ts"),
    true,
  );
  ok("bridge.ts invoke import allowed", f3.length === 0);

  // 4. bridge.ts invoke() call allowed
  const f4 = checkInvokeCall(
    'export async function tabPosition(id, x, y, w, h) { return invoke("tab_position", { id, x, y, w, h }); }',
    join(ROOT, "src", "bridge.ts"),
    true,
  );
  ok("bridge.ts invoke() call allowed", f4.length === 0);

  // 5. non-invoke Tauri import → WARN
  const w5 = checkNonInvokeTauriImport(
    'import { convertFileSrc } from "@tauri-apps/api/core";',
    fakePath,
    false,
  );
  ok(
    "convertFileSrc import detected as WARN",
    w5.length === 1 && w5[0].severity === "WARN",
  );

  // 6. @tauri-apps/api/window import → WARN
  const w6 = checkNonInvokeTauriImport(
    'import { getCurrentWindow } from "@tauri-apps/api/window";',
    fakePath,
    false,
  );
  ok(
    "@tauri-apps/api/window import detected as WARN",
    w6.length === 1 && w6[0].severity === "WARN",
  );

  // 7. __TAURI__ access → WARN
  const w7 = checkTauriGlobalAccess(
    '(window as any).__TAURI__?.event?.listen("tauri://focus", fn)',
    fakePath,
  );
  ok("__TAURI__ access detected as WARN", w7.length === 1 && w7[0].severity === "WARN");

  // 8. clean code → no findings
  const f8a = checkInvokeImport('import { bridge } from "../bridge";', fakePath, false);
  const f8b = checkInvokeCall("bridge.tabPosition(id, x, y, w, h);", fakePath, false);
  const w8 = checkNonInvokeTauriImport('import { bridge } from "../bridge";', fakePath, false);
  ok("clean bridge-based code has no findings", f8a.length === 0 && f8b.length === 0 && w8.length === 0);

  // 9. comment lines ignored
  const f9a = checkInvokeCall('// invoke("commented out")', fakePath, false);
  const f9b = checkInvokeCall('/* invoke("block commented") */', fakePath, false);
  ok("commented invoke() ignored", f9a.length === 0 && f9b.length === 0);

  // 10. .invoke() method call not flagged (e.g., obj.invoke() is not Tauri invoke)
  const f10 = checkInvokeCall("result = obj.invoke(arg);", fakePath, false);
  ok("method .invoke() not flagged as direct call", f10.length === 0);

  // 11. multi-line comment with invoke ignored
  const f11 = checkInvokeCall(
    "/* line one\n   invoke(\"inside\")\n   end */\ncode();",
    fakePath,
    false,
  );
  ok("multi-line comment invoke() ignored", f11.length === 0);

  return tests;
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

function formatText(result, strict) {
  const blocking = strict
    ? [
        ...result.findings,
        ...result.warnings.map((w) => ({ ...w, severity: "P3" })),
      ]
    : result.findings;
  const sorted = blocking.sort(
    (a, b) =>
      severityOrder(a.severity) - severityOrder(b.severity) ||
      a.file.localeCompare(b.file) ||
      a.line - b.line,
  );
  if (sorted.length === 0) {
    const warnCount = strict ? 0 : result.warnings.length;
    return `${CHECK_NAME}: PASS${warnCount > 0 ? ` (${warnCount} warning${warnCount > 1 ? "s" : ""}, --strict to upgrade)` : ""}`;
  }
  const lines = [`${CHECK_NAME}: FAIL`, ""];
  for (const f of sorted) {
    lines.push(`[${f.severity}] ${f.file}:${f.line} ${f.message}`);
  }
  if (!strict && result.warnings.length > 0) {
    lines.push("");
    lines.push(`${result.warnings.length} warning(s) not shown (use --strict to upgrade)`);
  }
  return lines.join("\n");
}

function severityOrder(s) {
  return { P1: 0, P2: 1, P3: 2, WARN: 3 }[s] ?? 9;
}

function formatJson(result, strict) {
  const blockingFindings = strict
    ? [...result.findings, ...result.warnings.map((w) => ({ ...w, severity: "P3" }))]
    : result.findings;
  const status = blockingFindings.length > 0 ? "FAIL" : "PASS";
  const warnings = strict ? [] : result.warnings;
  return JSON.stringify(
    {
      check: CHECK_NAME.toLowerCase(),
      status,
      findings: blockingFindings.sort(
        (a, b) =>
          severityOrder(a.severity) - severityOrder(b.severity) ||
          a.file.localeCompare(b.file) ||
          a.line - b.line,
      ),
      warnings,
      summary: {
        filesScanned: result.filesScanned,
        findingsCount: blockingFindings.length,
        warningsCount: warnings.length,
      },
    },
    null,
    2,
  );
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function printHelp() {
  return [
    `Usage: node scripts/check-architecture.mjs [options]`,
    "",
    "Architecture boundary checker (Phase 03).",
    "",
    "Options:",
    "  --help        Show this help and exit.",
    "  --json        Emit JSON output (see JSON Output Contract).",
    "  --strict      Upgrade WARN findings to blocking (P3).",
    "  --self-test   Run built-in self-test and exit.",
    "",
    "Exit codes:",
    "  0  PASS         no blocking findings",
    "  1  FAIL         one or more blocking findings",
    "  2  USAGE_ERROR  invalid CLI arguments",
    "",
    "Checks:",
    "  A1  No invoke import outside bridge.ts.                         (P1)",
    "  A2  No invoke() call outside bridge.ts.                         (P1)",
    "  A3  src/bridge.ts is the sole invoke import point in src/.      (P1)",
    "  A4  Stores must not import native Tauri modules directly.        (WARN)",
    "  A5  Components must not bypass the bridge boundary.              (WARN)",
    "  A6  Direct __TAURI__ global access is a bridge bypass.           (WARN)",
    "  A7  BrowserRuntime/MockRuntime/BrowserScene/syncScene targets.   (WARN)",
    "  A8  mvp_core boundary delegated to check-core-boundary.py.       (WARN)",
  ].join("\n");
}

function parseArgs(argv) {
  const args = { help: false, json: false, strict: false, selfTest: false, errors: [] };
  for (const arg of argv) {
    switch (arg) {
      case "--help":
      case "-h":
        args.help = true;
        break;
      case "--json":
        args.json = true;
        break;
      case "--strict":
        args.strict = true;
        break;
      case "--self-test":
        args.selfTest = true;
        break;
      default:
        args.errors.push(`Unknown argument: ${arg}`);
        break;
    }
  }
  return args;
}

function main(argv) {
  const args = parseArgs(argv);

  if (args.errors.length > 0) {
    for (const e of args.errors) console.error(e);
    console.error(printHelp());
    process.exit(2);
  }

  if (args.help) {
    console.log(printHelp());
    process.exit(0);
  }

  if (args.selfTest) {
    const tests = selfTest();
    const allPass = tests.every((t) => t.pass);
    for (const t of tests) {
      console.log(`${t.pass ? "ok" : "FAIL"} - ${t.name}`);
    }
    console.log(
      `\nself-test: ${allPass ? "PASS" : "FAIL"} (${tests.filter((t) => t.pass).length}/${tests.length})`,
    );
    process.exit(allPass ? 0 : 1);
  }

  const result = runScan();
  const hasBlocking = args.strict
    ? result.findings.length + result.warnings.length > 0
    : result.findings.length > 0;

  if (args.json) {
    console.log(formatJson(result, args.strict));
  } else {
    console.log(formatText(result, args.strict));
  }

  process.exit(hasBlocking ? 1 : 0);
}

main(process.argv.slice(2));
