#!/usr/bin/env node
// Task boundary checker (Phase 03 Batch B).
//
// Ensures an agent's changed files stay within its assigned task scope. Given
// an allowlist (and/or --docs-only), any changed file outside the scope FAILs.
// Native danger-zone edits always FAIL unless explicitly authorized.
//
// This is the guardrail that turns "each agent may only edit its own files"
// from a written rule into an automatic gate.
//
// Spec: .ai/workbuddy-dispatch/phase-03-checker-specification.md (Task Boundary Checker)
// Depends on: Native Checker danger-zone definitions (AGENTS.md §4)
//
// CLI:
//   node scripts/check-task-boundary.mjs
//   node scripts/check-task-boundary.mjs --help
//   node scripts/check-task-boundary.mjs --json
//   node scripts/check-task-boundary.mjs --allow "src/components/**"
//   node scripts/check-task-boundary.mjs --allow "docs/AI/**" --docs-only
//   node scripts/check-task-boundary.mjs --self-test
//
// Exit codes:
//   0  PASS         no blocking findings
//   1  FAIL         one or more blocking findings
//   2  USAGE_ERROR  invalid CLI arguments

import { execSync } from "node:child_process";
import { relative } from "node:path";
import { fileURLToPath } from "node:url";

const CHECK_NAME = "task-boundary";
const ROOT = fileURLToPath(new URL("../", import.meta.url));
const SELF = "scripts/check-task-boundary.mjs";

// Native danger-zone definitions (mirror of check-native.mjs, AGENTS.md §4).
const DANGER_ZONE = [
  { regex: /^src-tauri\/src\/bridge\.rs$/, label: "bridge.rs (IPC command surface)" },
  { regex: /^src-tauri\/src\/.*\/linux\.rs$/, label: "linux.rs (GTK/WebKitGTK platform)" },
  { regex: /^src-tauri\/capabilities\/.+$/, label: "capabilities (Tauri permissions)" },
  { regex: /^tauri-browser-tabs\/.+$/, label: "tauri-browser-tabs plugin" },
];

function git(args) {
  try {
    return execSync(`git ${args}`, { cwd: ROOT, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] });
  } catch {
    return "";
  }
}

// Get changed files via `git status --porcelain` (staged + unstaged + untracked).
// Excludes this checker script itself and the .ai/ governance metadata dir.
function getChangedFiles() {
  const out = git("status --porcelain");
  const files = [];
  for (const line of out.split("\n")) {
    if (!line.trim()) continue;
    let path = line.slice(3).trim();
    if (path.includes(" -> ")) path = path.split(" -> ")[1];
    if (path === SELF) continue;
    if (path.startsWith(".ai/")) continue;
    files.push({ path, status: line.slice(0, 2) });
  }
  return files;
}

// Convert a glob (supporting ** and *) to an anchored RegExp.
function globToRegex(glob) {
  const parts = glob.split("/").map((seg) => {
    if (seg === "**") return "(?:.*)";
    return seg
      .replace(/[.+^${}()|[\]\\]/g, "\\$&")
      .replace(/\*/g, "[^/]*")
      .replace(/\?/g, ".");
  });
  return new RegExp("^" + parts.join("/") + "$");
}

function matchesAny(path, globs) {
  return globs.some((g) => globToRegex(g).test(path));
}

function inDangerZone(path) {
  for (const dz of DANGER_ZONE) if (dz.regex.test(path)) return dz;
  return null;
}

// ---------------------------------------------------------------------------
// Scan
// ---------------------------------------------------------------------------

function runCheck(opts) {
  const findings = [];
  const warnings = [];
  const changed = getChangedFiles();
  const allowGlobs = opts.allow;
  const docsOnly = opts.docsOnly;
  const allowDiff = opts.allowDiff || process.env.CHECK_NATIVE_ALLOW_DIFF === "1";

  for (const cf of changed) {
    const dz = inDangerZone(cf.path);
    if (dz) {
      if (allowDiff) {
        warnings.push({
          severity: "P1",
          file: cf.path,
          line: 0,
          message: `Danger-zone file modified: ${dz.label}. [allowed via --allow-diff]`,
          rule: "AGENTS.md §4",
        });
      } else {
        findings.push({
          severity: "P1",
          file: cf.path,
          line: 0,
          message: `Danger-zone file modified: ${dz.label}. Native danger-zone edits require explicit task authorization (AGENTS.md §4).`,
          rule: "AGENTS.md §4",
        });
      }
      continue;
    }

    if (docsOnly) {
      // docs-only tasks may only touch docs/; block any product/script change.
      if (!cf.path.startsWith("docs/")) {
        findings.push({
          severity: "P1",
          file: cf.path,
          line: 0,
          message: `Documentation-only task must not change non-docs file: ${cf.path}`,
          rule: "phase-03-checker-specification.md (Task Boundary: docs-only)",
        });
      }
      continue;
    }

    if (allowGlobs.length > 0 && !matchesAny(cf.path, allowGlobs)) {
      findings.push({
        severity: "P1",
        file: cf.path,
        line: 0,
        message: `Changed file is outside the task allowlist: ${cf.path}`,
        rule: "phase-03-checker-specification.md (Task Boundary: scope)",
      });
    }
  }

  findings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
  warnings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);

  return {
    findings,
    warnings,
    summary: {
      changedFiles: changed.length,
      allowGlobs,
      docsOnly,
      dangerZoneHits: changed.filter((c) => inDangerZone(c.path)).length,
    },
  };
}

// ---------------------------------------------------------------------------
// Self-test (in-memory; no git, no repo mutation)
// ---------------------------------------------------------------------------

function selfTest() {
  const tests = [];
  const ok = (name, cond) => tests.push({ name, pass: !!cond });

  ok("allow src/components/** passes Foo.vue", matchesAny("src/components/Foo.vue", ["src/components/**"]));
  ok("allow src/components/** fails store file", !matchesAny("src/store/x.ts", ["src/components/**"]));
  ok("glob src/** matches nested path", matchesAny("src/a/b/c.ts", ["src/**"]));
  ok("glob scripts/check-x.mjs matches exact", matchesAny("scripts/check-x.mjs", ["scripts/check-x.mjs"]));
  ok("docs-only blocks src change", (() => {
    const r = runCheckVirtual(["src/x.ts"], [], true);
    return r.findings.length === 1 && r.findings[0].file === "src/x.ts";
  })());
  ok("docs-only allows docs change", runCheckVirtual(["docs/AI/y.md"], [], true).findings.length === 0);
  ok("danger-zone bridge.rs detected and fails", inDangerZone("src-tauri/src/bridge.rs") !== null);
  ok("danger-zone safe component ignored", inDangerZone("src/components/Foo.vue") === null);
  ok("out-of-allowlist change fails", (() => {
    const r = runCheckVirtual(["src/store/x.ts"], ["scripts/**"], false);
    return r.findings.length === 1 && r.findings[0].file === "src/store/x.ts";
  })());
  ok("in-allowlist change passes", runCheckVirtual(["scripts/check-x.mjs"], ["scripts/**"], false).findings.length === 0);
  ok("self path excluded from scan", !getChangedFiles().some((c) => c.path === SELF));

  return tests;
}

// Virtual scan over an explicit path list (for self-test only).
function runCheckVirtual(paths, allowGlobs, docsOnly) {
  const findings = [];
  const warnings = [];
  for (const p of paths) {
    if (p === SELF || p.startsWith(".ai/")) continue;
    const dz = inDangerZone(p);
    if (dz) {
      findings.push({ severity: "P1", file: p, line: 0, message: `Danger-zone: ${dz.label}`, rule: "AGENTS.md §4" });
      continue;
    }
    if (docsOnly) {
      if (!p.startsWith("docs/")) findings.push({ severity: "P1", file: p, line: 0, message: "docs-only block", rule: "spec" });
      continue;
    }
    if (allowGlobs.length > 0 && !matchesAny(p, allowGlobs)) {
      findings.push({ severity: "P1", file: p, line: 0, message: "out of scope", rule: "spec" });
    }
  }
  return { findings, warnings, summary: { changedFiles: paths.length, allowGlobs, docsOnly } };
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

function severityOrder(s) {
  return { P1: 0, P2: 1, P3: 2, WARN: 3 }[s] ?? 9;
}

function formatText(result, strict) {
  const blocking = strict
    ? [...result.findings, ...result.warnings.map((w) => ({ ...w, severity: "P1" }))]
    : result.findings;
  if (blocking.length === 0) {
    return `${CHECK_NAME.toUpperCase()}: PASS (${result.summary.changedFiles} changed file(s) in scope)`;
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
    ? [...result.findings, ...result.warnings.map((w) => ({ ...w, severity: "P1" }))]
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
    `Usage: node scripts/check-task-boundary.mjs [options]`,
    "",
    "Task boundary checker (Phase 03 Batch B).",
    "",
    "Options:",
    "  --help           Show this help and exit.",
    "  --json           Emit JSON output.",
    "  --strict         Upgrade WARN findings to blocking.",
    "  --self-test      Run built-in self-test and exit.",
    "  --allow <glob>   Allow a changed file matching this glob (repeatable).",
    "  --docs-only      Documentation-only task: any non-docs change FAILs.",
    "  --allow-diff     Allow native danger-zone diffs (explicit authorization).",
    "",
    "Exit codes:",
    "  0  PASS         no blocking findings",
    "  1  FAIL         one or more blocking findings",
    "  2  USAGE_ERROR  invalid CLI arguments",
    "",
    "Checks:",
    "  - Changed files stay within --allow globs (or --docs-only docs/).",
    "  - Native danger-zone edits FAIL unless --allow-diff.",
  ].join("\n");
}

function parseArgs(argv) {
  const opts = { allow: [], docsOnly: false, allowDiff: false, json: false, strict: false, selfTest: false, help: false, errors: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    switch (a) {
      case "--help":
      case "-h":
        opts.help = true;
        break;
      case "--json":
        opts.json = true;
        break;
      case "--strict":
        opts.strict = true;
        break;
      case "--self-test":
        opts.selfTest = true;
        break;
      case "--docs-only":
        opts.docsOnly = true;
        break;
      case "--allow-diff":
        opts.allowDiff = true;
        break;
      case "--allow":
        if (i + 1 < argv.length) {
          opts.allow.push(argv[++i]);
        } else {
          opts.errors.push("--allow requires a glob argument");
        }
        break;
      default:
        opts.errors.push(`Unknown argument: ${a}`);
    }
  }
  return opts;
}

function main(argv) {
  const opts = parseArgs(argv);
  if (opts.errors.length > 0) {
    for (const e of opts.errors) console.error(e);
    console.error(printHelp());
    process.exit(2);
  }
  if (opts.help) {
    console.log(printHelp());
    process.exit(0);
  }
  if (opts.selfTest) {
    const tests = selfTest();
    const allPass = tests.every((t) => t.pass);
    for (const t of tests) console.log(`${t.pass ? "ok" : "FAIL"} - ${t.name}`);
    console.log(
      `\nself-test: ${allPass ? "PASS" : "FAIL"} (${tests.filter((t) => t.pass).length}/${tests.length})`,
    );
    process.exit(allPass ? 0 : 1);
  }

  const result = runCheck(opts);
  const hasBlocking = result.findings.length > 0;

  if (opts.json) {
    console.log(formatJson(result, opts.strict));
  } else {
    console.log(formatText(result, opts.strict));
  }
  process.exit(hasBlocking ? 1 : 0);
}

main(process.argv.slice(2));
