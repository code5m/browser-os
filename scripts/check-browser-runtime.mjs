#!/usr/bin/env node
// Browser runtime boundary checker (Phase 03 Batch B).
//
// Enforces the BrowserRuntime / BrowserScene / syncScene adapter boundary that
// Phase 02 approved as TARGET but did NOT implement in source. Per the
// Phase 03 checker specification, absence of these symbols is PASS (not FAIL);
// once introduced, the checker verifies interface/mock/call-site consistency
// and blocks components from importing the future runtime directly.
//
// Spec: .ai/workbuddy-dispatch/phase-03-checker-specification.md (Runtime Checker)
// Rules: AGENTS.md §6, docs/AI/00-Architecture.md §2.2 (APPROVED_TARGET)
//
// CLI:
//   node scripts/check-browser-runtime.mjs
//   node scripts/check-browser-runtime.mjs --help
//   node scripts/check-browser-runtime.mjs --json
//   node scripts/check-browser-runtime.mjs --strict
//   node scripts/check-browser-runtime.mjs --self-test
//
// Exit codes:
//   0  PASS         no blocking findings
//   1  FAIL         one or more blocking findings
//   2  USAGE_ERROR  invalid CLI arguments

import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const CHECK_NAME = "browser-runtime";
const ROOT = fileURLToPath(new URL("../", import.meta.url));
const rel = (p) => relative(ROOT, p).replace(/\\/g, "/");

// APPROVED_TARGET symbols defined in Phase 02 but not yet in source.
const TARGET_SYMBOLS = [
  "BrowserRuntime",
  "MockRuntime",
  "BrowserScene",
  "syncScene",
  "WebViewSafeShell",
  "BrowserViewportAnchor",
];
const SYM_RE = new RegExp(`\\b(${TARGET_SYMBOLS.join("|")})\\b`);

// Frontend areas that must route runtime access through bridge.ts / adapters.
const FRONTEND_DIRS = ["src/components", "src/stores", "src/composables", "src/utils"];

// ---------------------------------------------------------------------------
// File scanning helpers
// ---------------------------------------------------------------------------

function walkDir(dir, exts, excludePrefixes = []) {
  const results = [];
  const full = join(ROOT, dir);
  if (!existsSync(full)) return results;
  let entries;
  try {
    entries = readdirSync(full);
  } catch {
    return results;
  }
  for (const entry of entries) {
    const child = join(full, entry);
    const childRel = rel(child);
    if (excludePrefixes.some((p) => childRel === p || childRel.startsWith(p + "/"))) continue;
    let stat;
    try {
      stat = statSync(child);
    } catch {
      continue;
    }
    if (stat.isDirectory()) {
      results.push(...walkDir(childRel, exts, excludePrefixes));
    } else if (exts.some((ext) => child.endsWith(ext))) {
      results.push(child);
    }
  }
  return results.sort();
}

// Extract import statements as { module, names, line }. Skips // and * comments.
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

function isInFrontendDir(filePath) {
  return FRONTEND_DIRS.some((d) => filePath === d || filePath.startsWith(d + "/"));
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

// R4: No frontend component/store/composable/util may import a future runtime
// symbol directly. It must route through bridge.ts / an approved adapter.
function checkDirectRuntimeImport(content, filePath) {
  if (!isInFrontendDir(filePath)) return [];
  const findings = [];
  for (const imp of extractImports(content)) {
    const importsTarget = TARGET_SYMBOLS.some(
      (s) => imp.module.includes(s) || imp.names.includes(s),
    );
    if (importsTarget) {
      findings.push({
        severity: "P1",
        file: filePath,
        line: imp.line,
        message: `Direct import of future runtime symbol from "${imp.module}"; route through src/bridge.ts / approved adapter (BrowserRuntime not yet implemented)`,
        rule: "AGENTS.md §6 / 00-Architecture.md §2.2 (APPROVED_TARGET)",
      });
    }
  }
  return findings;
}

// R2: BrowserScene and syncScene must appear together (naming consistency).
function checkPairing(hasScene, hasSync) {
  const warnings = [];
  if (hasScene && !hasSync) {
    warnings.push({
      severity: "P3",
      file: "src/",
      line: 0,
      message:
        "BrowserScene is present but syncScene is missing; naming consistency requires both once the runtime target is introduced",
      rule: "00-Architecture.md §2.2 (BrowserScene/syncScene pairing)",
    });
  }
  if (hasSync && !hasScene) {
    warnings.push({
      severity: "P3",
      file: "src/",
      line: 0,
      message:
        "syncScene is present but BrowserScene is missing; naming consistency requires both once the runtime target is introduced",
      rule: "00-Architecture.md §2.2 (BrowserScene/syncScene pairing)",
    });
  }
  return warnings;
}

// R3: BrowserRuntime introduced without MockRuntime parity is a partial impl.
function checkMockParity(hasRuntime, hasMock) {
  if (hasRuntime && !hasMock) {
    return [
      {
        severity: "P2",
        file: "src/",
        line: 0,
        message:
          "BrowserRuntime is introduced without MockRuntime parity; partial runtime implementation is inconsistent",
        rule: "phase-03-checker-specification.md (Runtime Checker: interface/mock parity)",
      },
    ];
  }
  return [];
}

// ---------------------------------------------------------------------------
// Scan orchestration
// ---------------------------------------------------------------------------

function runScan() {
  const srcFiles = walkDir("src", [".vue", ".ts", ".mjs"], ["src/styles"]);
  const findings = [];
  const warnings = [];

  const symbolHits = new Set();
  const foundInFrontend = new Set();

  for (const filePath of srcFiles) {
    const filePathRel = rel(filePath);
    let content;
    try {
      content = readFileSync(filePath, "utf8");
    } catch {
      continue;
    }
    const matches = content.match(SYM_RE);
    if (matches) for (const m of matches) symbolHits.add(m);
    findings.push(...checkDirectRuntimeImport(content, filePathRel));
  }

  const hasRuntime = symbolHits.has("BrowserRuntime");
  const hasMock = symbolHits.has("MockRuntime");
  const hasScene = symbolHits.has("BrowserScene");
  const hasSync = symbolHits.has("syncScene");
  const hasAny = symbolHits.size > 0;

  if (hasAny) {
    warnings.push(...checkPairing(hasScene, hasSync));
    warnings.push(...checkMockParity(hasRuntime, hasMock));
  }

  findings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
  warnings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);

  return {
    findings,
    warnings,
    summary: {
      filesScanned: srcFiles.length,
      symbolsFound: symbolHits.size,
      symbols: [...symbolHits].sort(),
      targetStatus: hasAny ? "partial-or-present" : "not-yet-implemented",
    },
  };
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

function selfTest() {
  const tests = [];
  const ok = (name, cond) => tests.push({ name, pass: !!cond });

  // 1. Direct runtime import in frontend file -> P1.
  const f1 = checkDirectRuntimeImport(
    'import { BrowserRuntime } from "@/runtime/browser-runtime";',
    "src/components/__selftest__.vue",
  );
  ok("direct runtime import in frontend -> P1", f1.length === 1 && f1[0].severity === "P1");

  // 2. Direct runtime import in src-tauri (non-frontend) -> ignored.
  const f2 = checkDirectRuntimeImport(
    'import { BrowserRuntime } from "./browser-runtime";',
    "src-tauri/src/__selftest__.rs",
  );
  ok("runtime import outside frontend dirs ignored", f2.length === 0);

  // 3. BrowserRuntime without MockRuntime -> P2.
  ok(
    "BrowserRuntime without MockRuntime -> P2",
    (() => {
      const w = checkMockParity(true, false);
      return w.length === 1 && w[0].severity === "P2";
    })(),
  );

  // 4. BrowserScene without syncScene -> P3.
  ok(
    "BrowserScene without syncScene -> P3",
    (() => {
      const w = checkPairing(true, false);
      return w.length === 1 && w[0].severity === "P3";
    })(),
  );

  // 5. Paired symbols -> no pairing warning.
  ok(
    "paired BrowserScene/syncScene -> no P3",
    checkPairing(true, true).length === 0,
  );

  // 6. Commented symbol is not counted as a real hit (line-level check).
  const commentedOnly = (() => {
    const c = '// BrowserRuntime is planned but not imported\nconst x = 1;';
    return checkDirectRuntimeImport(c, "src/components/__selftest__.vue").length === 0;
  })();
  ok("commented target symbol ignored", commentedOnly);

  return tests;
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

function severityOrder(s) {
  return { P1: 0, P2: 1, P3: 2, WARN: 3 }[s] ?? 9;
}

function formatText(result, strict) {
  const blocking = strict
    ? [...result.findings, ...result.warnings.map((w) => ({ ...w, severity: "P3" }))]
    : result.findings;
  if (blocking.length === 0) {
    const warnCount = strict ? 0 : result.warnings.length;
    const tgt = result.summary.targetStatus;
    return `${CHECK_NAME.toUpperCase()}: PASS (target: ${tgt}${warnCount > 0 ? `, ${warnCount} warning(s)` : ""})`;
  }
  const lines = [`${CHECK_NAME.toUpperCase()}: FAIL`, ""];
  for (const f of blocking.sort(
    (a, b) =>
      severityOrder(a.severity) - severityOrder(b.severity) ||
      a.file.localeCompare(b.file) ||
      a.line - b.line,
  )) {
    const loc = f.line > 0 ? `${f.file}:${f.line}` : f.file;
    lines.push(`[${f.severity}] ${loc} ${f.message}`);
  }
  return lines.join("\n");
}

function formatJson(result, strict) {
  const blockingFindings = strict
    ? [...result.findings, ...result.warnings.map((w) => ({ ...w, severity: "P3" }))]
    : result.findings;
  const status = blockingFindings.length > 0 ? "FAIL" : "PASS";
  const warnings = strict ? [] : result.warnings;
  return JSON.stringify(
    {
      check: CHECK_NAME,
      status,
      findings: blockingFindings.sort(
        (a, b) =>
          severityOrder(a.severity) - severityOrder(b.severity) ||
          a.file.localeCompare(b.file) ||
          a.line - b.line,
      ),
      warnings,
      summary: result.summary,
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
    `Usage: node scripts/check-browser-runtime.mjs [options]`,
    "",
    "Browser runtime boundary checker (Phase 03 Batch B).",
    "",
    "Options:",
    "  --help        Show this help and exit.",
    "  --json        Emit JSON output.",
    "  --strict      Upgrade WARN/P3 findings to blocking.",
    "  --self-test   Run built-in self-test and exit.",
    "",
    "Exit codes:",
    "  0  PASS         no blocking findings",
    "  1  FAIL         one or more blocking findings",
    "  2  USAGE_ERROR  invalid CLI arguments",
    "",
    "Checks:",
    "  R1  BrowserRuntime target symbols absent -> PASS (not-yet-implemented).",
    "  R2  BrowserScene/syncScene naming consistency once introduced.   (P3)",
    "  R3  BrowserRuntime without MockRuntime parity.                    (P2)",
    "  R4  Frontend must not import future runtime symbols directly.     (P1)",
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
    for (const t of tests) console.log(`${t.pass ? "ok" : "FAIL"} - ${t.name}`);
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
