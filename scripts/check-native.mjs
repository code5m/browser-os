#!/usr/bin/env node
// Native boundary checker (Phase 03).
//
// Enforces the native danger-zone boundary defined in AGENTS.md §4 and
// PROJECT-RULES.md. Detects:
//   1. Native danger-zone files in current diff OR untracked (FAIL by default).
//   2. Deprecated native patterns introduced in new diff lines (FAIL).
//   3. Deprecated native patterns anywhere in full source (WARN — historical).
//
// Spec: .ai/workbuddy-dispatch/phase-03-checker-specification.md
// Rules: PROJECT-RULES.md 规则 1 / 3.5 / 3.6, AGENTS.md §4

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { execSync } from "node:child_process";
import { resolve, relative, join } from "node:path";
import { fileURLToPath } from "node:url";

const CHECK_NAME = "check-native";
const ROOT = resolve(fileURLToPath(new URL("../", import.meta.url)));
const rel = (p) => relative(ROOT, p);

// ---------------------------------------------------------------------------
// Danger-zone definitions (AGENTS.md §4)
// ---------------------------------------------------------------------------
const DANGER_ZONE_PATTERNS = [
  { regex: /^src-tauri\/src\/bridge\.rs$/, label: "bridge.rs (IPC command surface)" },
  { regex: /^src-tauri\/src\/.*\/linux\.rs$/, label: "linux.rs (GTK/WebKitGTK platform)" },
  { regex: /^src-tauri\/capabilities\/.+$/, label: "capabilities (Tauri permissions)" },
  { regex: /^tauri-browser-tabs\/.+$/, label: "tauri-browser-tabs plugin" },
];

function matchDangerZone(filePath) {
  for (const dz of DANGER_ZONE_PATTERNS) {
    if (dz.regex.test(filePath)) return dz;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Deprecated native patterns (PROJECT-RULES.md 规则 1 / 3.5)
// ---------------------------------------------------------------------------
const DEPRECATED_PATTERNS = [
  {
    name: "set_size_request",
    regex: /set_size_request/,
    severity: "P1",
    rule: "PROJECT-RULES.md 规则 1",
    message: "set_size_request is deprecated (triggers GtkFixed layout loop). Use gtk_fixed_move + size_allocate.",
  },
  {
    name: "queue_resize",
    regex: /queue_resize/,
    severity: "P1",
    rule: "PROJECT-RULES.md 规则 1",
    message: "queue_resize is deprecated (triggers GtkFixed layout loop). Use size_allocate + queue_draw.",
  },
  {
    name: "webview_hide",
    regex: /\.hide\s*\(\s*\)/,
    severity: "P2",
    rule: "PROJECT-RULES.md 规则 3.5",
    message: "webview.hide() on a rendering WebKitGTK child can deadlock the main thread. Verify this is a legitimate initial-hide or set_visible API, not tab-switch hiding.",
  },
  {
    name: "offscreen_1x1",
    regex: /(LogicalRect|LogicalSize|spawn_child_window)[\s\S]*?1\.0,\s*1\.0\)|\b1,\s*1\)/,
    severity: "P1",
    rule: "PROJECT-RULES.md 规则 3.5",
    message: "Shrinking hidden WebView to 1x1 triggers WebKit reflow deadlock. Move offscreen (x=-30000) with size unchanged.",
  },
  {
    name: "dpr_multiply",
    regex: /device_pixel_ratio\s*\*|devicePixelRatio\s*\*/,
    severity: "P1",
    rule: "PROJECT-RULES.md 规则 3",
    message: "Multiplying frontend coordinates by devicePixelRatio is deprecated. DPI is handled by Tauri/wry.",
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Walk a directory tree and return all .rs file paths (relative to ROOT). */
function walkRs(dir, results = []) {
  const full = join(ROOT, dir);
  if (!existsSync(full)) return results;
  let entries;
  try {
    entries = readdirSync(full, { withFileTypes: true });
  } catch {
    return results;
  }
  for (const e of entries) {
    if (e.name === "target" || e.name === "node_modules" || e.name === ".git" || e.name === "examples") continue;
    const childRel = dir ? `${dir}/${e.name}` : e.name;
    if (e.isDirectory()) {
      walkRs(childRel, results);
    } else if (e.name.endsWith(".rs")) {
      results.push(childRel);
    }
  }
  return results;
}

/** Return true if the pattern match on this line is inside a Rust line comment. */
function isInsideComment(line, matchIndex) {
  const commentIdx = line.indexOf("//");
  return commentIdx !== -1 && matchIndex >= commentIdx;
}

/** Run a git command, returning stdout or empty string on failure. */
function git(args) {
  try {
    return execSync(`git ${args}`, { cwd: ROOT, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] });
  } catch {
    return "";
  }
}

/** Get TRACKED changed files relative to HEAD (staged + unstaged).
 *  Used for diff-line-aware checks (Check 2) and the changed-files summary.
 *  NOTE: does NOT include untracked files — danger-zone coverage of untracked
 *  files is handled separately by getUntrackedFiles() so a brand-new file
 *  inside a native danger-zone is still gated (see Check 1 / OBS-1). */
function getChangedFiles() {
  const out = git("diff --name-status HEAD");
  const files = [];
  for (const line of out.split("\n")) {
    if (!line.trim()) continue;
    const parts = line.split("\t");
    if (parts.length < 2) continue;
    const status = parts[0];
    // Rename (Rxxx): old -> new; take the new path (last field)
    const file = parts[parts.length - 1];
    files.push({ path: file, status });
  }
  return files;
}

/** Get UNTRACKED files via `git status --porcelain` (?? lines).
 *  A new file inside a native danger-zone must still be gated even though it
 *  has no git diff. Required to close the tracked-only blind spot (OBS-1). */
function getUntrackedFiles() {
  const out = git("status --porcelain");
  const files = [];
  for (const line of out.split("\n")) {
    if (!line.trim()) continue;
    const code = line.slice(0, 2);
    const file = line.slice(3).trim();
    if (code.startsWith("??")) files.push(file);
  }
  return files;
}

/** Get added lines from git diff (unstaged + staged) as { file, line, content }. */
function getDiffAddedLines() {
  const result = [];
  for (const cached of [false, true]) {
    const out = git(`diff ${cached ? "--cached" : ""} --unified=0 --diff-filter=AMRC`);
    if (!out) continue;
    let file = null;
    let newLine = 0;
    for (const line of out.split("\n")) {
      if (line.startsWith("+++ b/")) {
        file = line.slice(6);
        newLine = 0;
      } else if (line.startsWith("@@")) {
        const m = line.match(/\+(\d+)/);
        if (m) newLine = parseInt(m[1], 10) - 1;
      } else if (line.startsWith("+") && !line.startsWith("+++") && file) {
        newLine++;
        result.push({ file, line: newLine, content: line.slice(1) });
      } else if (line.startsWith(" ") && file) {
        newLine++;
      }
    }
  }
  return result;
}

/** Scan a file for deprecated patterns. Returns findings array. */
function scanFile(filePath, { warnOnly }) {
  const findings = [];
  const fullPath = join(ROOT, filePath);
  let content;
  try {
    content = readFileSync(fullPath, "utf8");
  } catch {
    return findings;
  }
  const lines = content.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    for (const pat of DEPRECATED_PATTERNS) {
      const m = pat.regex.exec(line);
      if (!m) continue;
      if (isInsideComment(line, m.index)) continue; // skip comment references
      findings.push({
        severity: warnOnly ? "WARN" : pat.severity,
        file: filePath,
        line: i + 1,
        message: pat.message,
        rule: pat.rule,
        pattern: pat.name,
      });
    }
  }
  return findings;
}

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------
function parseArgs(argv) {
  const opts = { json: false, strict: false, selfTest: false, help: false, allowDiff: false };
  const unknown = [];
  for (const a of argv) {
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
      case "--allow-diff":
        opts.allowDiff = true;
        break;
      default:
        if (a.startsWith("--allow-diff=")) {
          opts.allowDiff = a.slice("--allow-diff=".length) === "1";
        } else {
          unknown.push(a);
        }
    }
  }
  return { opts, unknown };
}

function printHelp() {
  process.stdout.write(`Usage: node scripts/check-native.mjs [options]

Native boundary checker — detects danger-zone edits and deprecated native patterns.

Options:
  --help        Show this help message and exit.
  --json        Emit machine-readable JSON instead of text.
  --strict      Upgrade WARN findings to FAIL (changes exit code).
  --self-test   Run internal consistency checks and exit.
  --allow-diff  Allow danger-zone diff (explicit CI/task authorization).
                  Also: --allow-diff=1 / env CHECK_NATIVE_ALLOW_DIFF=1.

Exit codes:
  0  PASS   No blocking findings.
  1  FAIL   One or more blocking findings.
  2  USAGE  Invalid CLI arguments.

Checks:
  1. Native danger-zone edits — TRACKED or UNTRACKED (FAIL unless --allow-diff).
     Danger zones:
       - src-tauri/src/bridge.rs
       - src-tauri/src/**/linux.rs
       - src-tauri/capabilities/**
       - tauri-browser-tabs/**
  2. Deprecated native patterns in new diff lines (FAIL):
       - set_size_request, queue_resize (PROJECT-RULES 规则 1)
       - webview.hide() on rendering child (PROJECT-RULES 规则 3.5)
       - offscreen 1x1 resize hiding (PROJECT-RULES 规则 3.5)
       - devicePixelRatio multiply (PROJECT-RULES 规则 3)
  3. Deprecated native patterns in full source (WARN — historical).
`);
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------
function runSelfTest() {
  const errors = [];
  // Danger-zone regexes match known paths.
  if (!matchDangerZone("src-tauri/src/bridge.rs")) errors.push("bridge.rs not matched");
  if (!matchDangerZone("src-tauri/src/core/linux.rs")) errors.push("core/linux.rs not matched");
  if (!matchDangerZone("src-tauri/capabilities/default.json")) errors.push("capabilities/default.json not matched");
  if (!matchDangerZone("tauri-browser-tabs/crates/foo.rs")) errors.push("tauri-browser-tabs/foo.rs not matched");
  // Danger-zone regexes reject safe paths.
  if (matchDangerZone("src/components/Foo.vue")) errors.push("safe component wrongly matched");
  if (matchDangerZone("scripts/check-native.mjs")) errors.push("script wrongly matched");
  // Deprecated patterns match known strings.
  if (!DEPRECATED_PATTERNS[0].regex.test("gtk_webview.set_size_request(w, h);")) errors.push("set_size_request pattern failed");
  if (!DEPRECATED_PATTERNS[1].regex.test("gtk_webview.queue_resize();")) errors.push("queue_resize pattern failed");
  if (!DEPRECATED_PATTERNS[2].regex.test("webview.hide()?;")) errors.push("webview.hide() pattern failed");
  // Comment detection.
  if (!isInsideComment("// do not set_size_request", 13)) errors.push("comment detection failed");
  if (isInsideComment("webview.hide(); // legit", 0)) errors.push("non-comment wrongly detected as comment");
  // Untracked danger-zone file is still in scope (OBS-1).
  const untrackedHit = matchDangerZone("tauri-browser-tabs/crates/new_untracked.rs");
  if (!untrackedHit) errors.push("untracked danger-zone path not matched by danger-zone regex");
  return errors;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
function main() {
  const { opts, unknown } = parseArgs(process.argv.slice(2));

  if (opts.help) {
    printHelp();
    process.exit(0);
  }

  if (unknown.length > 0) {
    if (opts.json) {
      process.stdout.write(JSON.stringify({
        check: CHECK_NAME,
        status: "USAGE_ERROR",
        findings: [],
        warnings: [],
        summary: { filesScanned: 0 },
      }) + "\n");
    } else {
      process.stderr.write(`${CHECK_NAME}: USAGE_ERROR\n\nUnknown argument(s): ${unknown.join(", ")}\nTry --help.\n`);
    }
    process.exit(2);
  }

  if (opts.selfTest) {
    const errors = runSelfTest();
    if (errors.length === 0) {
      process.stdout.write(`SELF_TEST: PASS (${CHECK_NAME} internal consistency)\n`);
      process.exit(0);
    } else {
      process.stdout.write(`SELF_TEST: FAIL\n\n${errors.map((e) => `- ${e}`).join("\n")}\n`);
      process.exit(1);
    }
  }

  const allowDiff = opts.allowDiff || process.env.CHECK_NATIVE_ALLOW_DIFF === "1";

  const findings = [];   // blocking (P1/P2)
  const warnings = [];   // non-blocking (WARN)

  // --- Check 1: danger-zone files (TRACKED + UNTRACKED) in current scope ---
  const changedFiles = getChangedFiles();
  const untrackedFiles = getUntrackedFiles();
  const dangerZoneHits = [];
  // Tracked modifications in danger-zone -> FAIL by default.
  for (const cf of changedFiles) {
    const dz = matchDangerZone(cf.path);
    if (dz) dangerZoneHits.push({ path: cf.path, label: dz.label, status: cf.status });
  }
  // Untracked NEW files in danger-zone -> also FAIL (close OBS-1 blind spot).
  for (const uf of untrackedFiles) {
    const dz = matchDangerZone(uf);
    if (dz) dangerZoneHits.push({ path: uf, label: dz.label, status: "?" });
  }
  if (dangerZoneHits.length > 0) {
    for (const hit of dangerZoneHits) {
      const f = {
        severity: "P1",
        file: hit.path,
        line: 0,
        message: `Danger-zone file modified: ${hit.label}. Native danger-zone edits require explicit task authorization (AGENTS.md §4).`,
        rule: "AGENTS.md §4",
      };
      if (allowDiff) {
        f.severity = "WARN";
        f.message += ` [allowed via --allow-diff]`;
        warnings.push(f);
      } else {
        findings.push(f);
      }
    }
  }

  // --- Check 2: deprecated patterns in new diff lines (tracked diff) ---
  const addedLines = getDiffAddedLines();
  for (const al of addedLines) {
    for (const pat of DEPRECATED_PATTERNS) {
      const m = pat.regex.exec(al.content);
      if (!m) continue;
      if (isInsideComment(al.content, m.index)) continue;
      findings.push({
        severity: pat.severity,
        file: al.file,
        line: al.line,
        message: `New code introduces deprecated native pattern: ${pat.message}`,
        rule: pat.rule,
        pattern: pat.name,
      });
    }
  }

  // --- Check 2b: deprecated patterns in UNTRACKED danger-zone .rs files ---
  // A brand-new file inside a danger-zone has no git diff, so scan its full
  // content as added lines to keep the content-level gate (OBS-1).
  for (const uf of untrackedFiles) {
    if (!matchDangerZone(uf)) continue;
    if (!uf.endsWith(".rs")) continue;
    const fileFindings = scanFile(uf, { warnOnly: false });
    for (const ff of fileFindings) {
      findings.push({
        ...ff,
        message: `New untracked file introduces deprecated native pattern: ${ff.message}`,
      });
    }
  }

  // --- Check 3: deprecated patterns in full source (WARN — historical) ---
  const rsFiles = [
    ...walkRs("src-tauri/src"),
    ...walkRs("tauri-browser-tabs/crates"),
  ];
  let filesScanned = 0;
  for (const f of rsFiles) {
    filesScanned++;
    const fileFindings = scanFile(f, { warnOnly: true });
    for (const wf of fileFindings) {
      warnings.push(wf);
    }
  }

  // --- --strict: upgrade WARN to FAIL ---
  if (opts.strict) {
    for (const w of warnings) {
      findings.push({ ...w, severity: w.severity === "WARN" ? "P2" : w.severity });
    }
    warnings.length = 0;
  }

  // --- Output ---
  const status = findings.length > 0 ? "FAIL" : "PASS";
  const exitCode = findings.length > 0 ? 1 : 0;

  if (opts.json) {
    process.stdout.write(JSON.stringify({
      check: CHECK_NAME,
      status,
      findings: findings.map(({ pattern, ...rest }) => rest),
      warnings: warnings.map(({ pattern, ...rest }) => rest),
      summary: {
        filesScanned,
        dangerZoneHits: dangerZoneHits.length,
        changedFiles: changedFiles.length,
        untrackedFiles: untrackedFiles.length,
        deprecatedInDiff: addedLines.filter((al) =>
          DEPRECATED_PATTERNS.some((p) => {
            const m = p.regex.exec(al.content);
            return m && !isInsideComment(al.content, m.index);
          })
        ).length,
      },
    }) + "\n");
  } else {
    process.stdout.write(`${CHECK_NAME}: ${status}\n`);
    if (findings.length > 0) {
      process.stdout.write("\n");
      for (const f of findings) {
        const loc = f.line > 0 ? `${f.file}:${f.line}` : f.file;
        process.stdout.write(`[${f.severity}] ${loc} ${f.message}\n`);
      }
    }
    if (warnings.length > 0 && !opts.strict) {
      process.stdout.write("\nWARNINGS (historical / non-blocking):\n");
      for (const w of warnings) {
        const loc = w.line > 0 ? `${w.file}:${w.line}` : w.file;
        process.stdout.write(`[WARN] ${loc} ${w.message}\n`);
      }
    }
    if (status === "PASS") {
      process.stdout.write(`  (scanned ${filesScanned} .rs files, ${changedFiles.length} changed, ${untrackedFiles.length} untracked, ${dangerZoneHits.length} danger-zone hits)\n`);
    }
  }

  process.exit(exitCode);
}

main();
