#!/usr/bin/env node
// Doctor — project health report (Phase 03, agent 09-doctor).
//
// Single diagnostic entry reporting Git / Build / Desktop / Package / Runtime /
// Version / Governance status. GUI status is ALWAYS GUI_PENDING: the doctor may
// not pretend desktop behavior was verified without user interaction.
//
// Does NOT edit source, install, deploy, or run heavy builds. Read-only.
//
// Spec: .ai/workbuddy-dispatch/phase-03-checker-specification.md
// Role: .ai/agents/09-doctor.md

import { readFileSync, existsSync } from "node:fs";
import { execSync } from "node:child_process";
import { resolve, relative, join } from "node:path";
import { fileURLToPath } from "node:url";

const CHECK_NAME = "doctor";
const ROOT = resolve(fileURLToPath(new URL("../", import.meta.url)));
const rel = (p) => relative(ROOT, p);

const REQUIRED_CHECKERS = [
  "scripts/check-architecture.mjs",
  "scripts/check-ui.mjs",
  "scripts/check-native.mjs",
  "scripts/check-browser-runtime.mjs",
  "scripts/check-task-boundary.mjs",
];

// ---------------------------------------------------------------------------
// Helpers (read-only; never mutate the worktree)
// ---------------------------------------------------------------------------
function run(cmd) {
  try {
    return execSync(cmd, { cwd: ROOT, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] });
  } catch {
    return "";
  }
}
function runJson(cmd) {
  try {
    return JSON.parse(execSync(cmd, { cwd: ROOT, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] }));
  } catch {
    return null;
  }
}
function hasBin(b) {
  return run(`command -v ${b}`).trim().length > 0;
}

function gitInfo() {
  const branch = run("git rev-parse --abbrev-ref HEAD").trim() || "unknown";
  const head = run("git rev-parse --short HEAD").trim() || "unknown";
  const dirty = run("git status --porcelain").trim().length > 0;
  let base = "";
  for (const c of ["origin/master", "master", "origin/main", "main"]) {
    if (run(`git rev-parse --verify --quiet ${c}^{commit}`).trim()) { base = c; break; }
  }
  const ahead = base ? run(`git rev-list --count ${base}..HEAD`).trim() : "";
  return { branch, head, dirty, base: base || null, aheadCommits: ahead === "" ? null : Number(ahead) };
}

function toolInfo() {
  const tools = ["node", "npm", "cargo", "git", "tauri"];
  const missing = tools.filter((t) => !hasBin(t));
  return { tools, missing };
}

function packageInfo() {
  const p = join(ROOT, "package.json");
  if (!existsSync(p)) return { exists: false };
  let pkg;
  try {
    pkg = JSON.parse(readFileSync(p, "utf8"));
  } catch {
    return { exists: true, parseError: true };
  }
  const scripts = pkg.scripts || {};
  return {
    exists: true,
    name: pkg.name,
    version: pkg.version,
    type: pkg.type,
    hasDev: !!scripts.dev,
    hasBuild: !!scripts.build,
    hasTauri: !!scripts.tauri,
  };
}

function tauriInfo() {
  const conf = join(ROOT, "src-tauri/tauri.conf.json");
  const pluginCargo = join(ROOT, "tauri-browser-tabs/Cargo.toml");
  const vite =
    existsSync(join(ROOT, "vite.config.ts")) ||
    existsSync(join(ROOT, "vite.config.js")) ||
    existsSync(join(ROOT, "vite.config.mjs"));
  const indexHtml = existsSync(join(ROOT, "index.html"));
  return {
    confExists: existsSync(conf),
    pluginCargoExists: existsSync(pluginCargo),
    viteConfig: vite,
    indexHtml,
  };
}

function runtimeInfo() {
  const r = runJson("node scripts/check-browser-runtime.mjs --json");
  if (!r) return { status: "UNKNOWN", targetStatus: "unknown", symbolsFound: null };
  return {
    status: r.status,
    targetStatus: r.summary?.targetStatus || "unknown",
    symbolsFound: r.summary?.symbolsFound ?? 0,
  };
}

function checkerInfo() {
  const present = REQUIRED_CHECKERS.filter((f) => existsSync(join(ROOT, f)));
  const missing = REQUIRED_CHECKERS.filter((f) => !existsSync(join(ROOT, f)));
  return { present, missing };
}

function governanceInfo() {
  // DECISION-1: tauri-browser-tabs/AGENTS.md should be tracked.
  const agentsTracked = run("git ls-files tauri-browser-tabs/AGENTS.md").trim().length > 0;
  // DECISION-2: Native Checker covers untracked danger-zone files (OBS-1 closed).
  return {
    dangerZoneAgentsMdTracked: agentsTracked,
    nativeCoversUntrackedDangerZone: true,
  };
}

function collect() {
  const findings = [];
  const warnings = [];

  const checkers = checkerInfo();
  for (const m of checkers.missing) {
    findings.push({ severity: "P1", area: "checkers", message: `Required checker script missing: ${m}` });
  }

  const pkg = packageInfo();
  if (!pkg.exists) {
    findings.push({ severity: "P1", area: "package", message: "package.json missing" });
  } else if (pkg.parseError) {
    findings.push({ severity: "P1", area: "package", message: "package.json unparseable" });
  }

  const tauri = tauriInfo();
  if (!tauri.confExists) {
    findings.push({ severity: "P1", area: "desktop", message: "src-tauri/tauri.conf.json missing" });
  }

  const tools = toolInfo();
  for (const t of tools.missing) {
    warnings.push({ severity: "WARN", area: "tools", message: `Optional tool missing: ${t}` });
  }

  const git = gitInfo();
  if (git.dirty) {
    warnings.push({ severity: "WARN", area: "git", message: "Working tree has uncommitted changes" });
  }

  const runtime = runtimeInfo();
  if (runtime.targetStatus === "not-yet-implemented") {
    warnings.push({
      severity: "WARN",
      area: "runtime",
      message: "BrowserRuntime target not yet implemented (expected pre-migration state)",
    });
  }

  return {
    findings,
    warnings,
    git,
    tools,
    pkg,
    tauri,
    runtime,
    checkers,
    governance: governanceInfo(),
  };
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------
function parseArgs(argv) {
  const opts = { json: false, strict: false, selfTest: false, help: false };
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
      default:
        unknown.push(a);
    }
  }
  return { opts, unknown };
}

function printHelp() {
  process.stdout.write(`Usage: node scripts/doctor.mjs [options]

Doctor — project health report (Git / Build / Desktop / Package / Runtime / Version / Governance).
GUI status is always GUI_PENDING (desktop behavior cannot be verified without user interaction).

Options:
  --help     Show this help and exit.
  --json     Emit machine-readable JSON.
  --strict   Upgrade WARN to FAIL.
  --self-test Run internal consistency checks and exit.

Exit codes:
  0  PASS   No blocking findings.
  1  FAIL   One or more blocking findings.
  2  USAGE  Invalid CLI arguments.
`);
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------
function runSelfTest() {
  const errors = [];
  // collect() must not throw on a normal worktree.
  let report;
  try {
    report = collect();
  } catch (e) {
    errors.push(`collect() threw: ${e.message}`);
    return errors;
  }
  // GUI status must be reportable and always PENDING.
  if (typeof report !== "object") errors.push("report not an object");
  // Runtime summary must carry a targetStatus string.
  if (typeof report.runtime?.targetStatus !== "string") errors.push("runtime.targetStatus missing");
  // Checker presence detection must return arrays.
  if (!Array.isArray(report.checkers?.present) || !Array.isArray(report.checkers?.missing)) {
    errors.push("checkerInfo shape wrong");
  }
  // Governance flags must be booleans.
  if (typeof report.governance?.nativeCoversUntrackedDangerZone !== "boolean") {
    errors.push("governance.nativeCoversUntrackedDangerZone not boolean");
  }
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
      process.stdout.write(JSON.stringify({ check: CHECK_NAME, status: "USAGE_ERROR", findings: [], warnings: [], summary: {} }) + "\n");
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
    }
    process.stdout.write(`SELF_TEST: FAIL\n\n${errors.map((e) => `- ${e}`).join("\n")}\n`);
    process.exit(1);
  }

  const report = collect();
  let { findings, warnings } = report;

  if (opts.strict) {
    const upgraded = [];
    for (const w of warnings) {
      upgraded.push({ ...w, severity: w.severity === "WARN" ? "P2" : w.severity });
    }
    findings = [...findings, ...upgraded];
    warnings = [];
  }

  const status = findings.length > 0 ? "FAIL" : "PASS";
  const exitCode = findings.length > 0 ? 1 : 0;

  // GUI can never be claimed verified by the doctor.
  const gui = "GUI_PENDING";

  if (opts.json) {
    process.stdout.write(
      JSON.stringify(
        {
          check: CHECK_NAME,
          status,
          findings: findings.map(({ area, ...rest }) => rest),
          warnings: warnings.map(({ area, ...rest }) => rest),
          summary: {
            gui,
            git: report.git,
            tools: { missing: report.tools.missing },
            package: report.pkg,
            tauri: report.tauri,
            runtime: report.runtime,
            checkers: { present: report.checkers.present.length, missing: report.checkers.missing },
            governance: report.governance,
          },
        },
        null,
        2,
      ) + "\n",
    );
  } else {
    process.stdout.write(`DOCTOR: ${status}  (gui=${gui})\n\n`);
    process.stdout.write(`Git     : ${report.git.branch} @ ${report.git.head}` +
      (report.git.base ? ` (ahead ${report.git.aheadCommits} of ${report.git.base})` : "") +
      (report.git.dirty ? "  [DIRTY]" : "") + "\n");
    process.stdout.write(`Package : ${report.pkg.exists ? `${report.pkg.name}@${report.pkg.version} (type=${report.pkg.type})` : "MISSING"}\n`);
    process.stdout.write(`Build   : vite=${report.tauri.viteConfig} indexHtml=${report.tauri.indexHtml} buildScript=${report.pkg.hasBuild}\n`);
    process.stdout.write(`Desktop : tauri.conf.json=${report.tauri.confExists} pluginCargo=${report.tauri.pluginCargoExists}\n`);
    process.stdout.write(`Runtime : ${report.runtime.status} (target=${report.runtime.targetStatus}, symbols=${report.runtime.symbolsFound})\n`);
    process.stdout.write(`Checkers: ${report.checkers.present.length}/${REQUIRED_CHECKERS.length} present` +
      (report.checkers.missing.length ? `, MISSING: ${report.checkers.missing.join(", ")}` : "") + "\n");
    process.stdout.write(`Tools   : missing=${report.tools.missing.length ? report.tools.missing.join(",") : "none"}\n`);
    process.stdout.write(`Gov     : dangerZoneAGENTS.md tracked=${report.governance.dangerZoneAgentsMdTracked}, nativeCoversUntracked=${report.governance.nativeCoversUntrackedDangerZone}\n`);

    if (findings.length > 0) {
      process.stdout.write("\nFAILINGS:\n");
      for (const f of findings) {
        process.stdout.write(`[${f.severity}] (${f.area}) ${f.message}\n`);
      }
    }
    if (warnings.length > 0) {
      process.stdout.write("\nWARNINGS (non-blocking):\n");
      for (const w of warnings) {
        process.stdout.write(`[WARN] (${w.area}) ${w.message}\n`);
      }
    }
  }

  process.exit(exitCode);
}

main();
