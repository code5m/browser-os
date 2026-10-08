#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const POLICY = JSON.parse(fs.readFileSync(path.join(ROOT, "docs/engineering/evidence-policy.json"), "utf8"));

function classify(p) {
  if (p.startsWith("logs/m0-baseline/") || p.startsWith("logs/m0-build-metrics/")) return "permanent";
  if (p.startsWith("logs/acceptance/") || p.startsWith("logs/m0-6c-gui-evidence/")) return "releaseEvidence";
  if (p.startsWith("logs/assist/") || p.startsWith("logs/checkpoints/") || p.startsWith("logs/research/")) return "historicalFrozen";
  if (p.startsWith("logs/")) return "historicalOther";
  if (p.startsWith("diagnostics/") || p.startsWith("artifacts/")) return "localEphemeral";
  return "unclassified";
}

function git(args) {
  const r = spawnSync("git", args, { cwd: ROOT, encoding: "utf8" });
  if (r.status !== 0) throw new Error(r.stderr || `git ${args.join(" ")} failed`);
  return r.stdout.trim();
}

if (process.argv.includes("--self-test")) {
  const tests = [
    ["logs/m0-baseline/x", "permanent"],
    ["logs/acceptance/x", "releaseEvidence"],
    ["logs/checkpoints/x", "historicalFrozen"],
    ["diagnostics/x", "localEphemeral"],
  ];
  const ok = tests.every(([a, b]) => classify(a) === b);
  console.log(`EVIDENCE_LIFECYCLE_SELF_TEST=${ok ? "PASS" : "FAIL"}`);
  process.exit(ok ? 0 : 1);
}

const tracked = git(["ls-files", "logs"]).split("\n").filter(Boolean);
const counts = {};
const bytes = {};
const missing = [];
for (const p of tracked) {
  const kind = classify(p);
  counts[kind] = (counts[kind] || 0) + 1;
  const abs = path.join(ROOT, p);
  try {
    // Use lstat rather than exists/stat. Some historical evidence entries are
    // symlinks; existsSync/stat follow the target and falsely report a tracked
    // evidence object as missing when the target is intentionally absent in CI.
    bytes[kind] = (bytes[kind] || 0) + fs.lstatSync(abs).size;
  } catch {
    missing.push(p);
  }
}
const trackedBytes = Object.values(bytes).reduce((a, b) => a + b, 0);

let addedFrozen = [];
try {
  const base = process.env.GITHUB_BASE_REF ? `origin/${process.env.GITHUB_BASE_REF}` : "HEAD^";
  const diff = git(["diff", "--name-status", base, "HEAD", "--", "logs"]);
  addedFrozen = diff.split("\n").filter(Boolean).map((line) => line.split(/\s+/)).filter(([status, p]) =>
    status.startsWith("A") && classify(p) === "historicalFrozen"
  ).map(([, p]) => p);
} catch {
  // Initial repositories/shallow local contexts can lack HEAD^; total-budget protection still applies.
}

const report = {
  schemaVersion: 1,
  trackedFiles: tracked.length,
  trackedBytes,
  budgetBytes: POLICY.trackedHistoryBudgetBytes,
  counts,
  bytes,
  missing,
  newHistoricalFrozenFiles: addedFrozen,
};
if (process.argv.includes("--json")) console.log(JSON.stringify(report, null, 2));
const errors = [];
if (missing.length) errors.push(`missing tracked evidence files: ${missing.length}: ${missing.slice(0, 10).join(", ")}`);
if (trackedBytes > POLICY.trackedHistoryBudgetBytes) errors.push(`tracked evidence exceeds budget: ${trackedBytes} > ${POLICY.trackedHistoryBudgetBytes}`);
if (addedFrozen.length) errors.push(`new files added to frozen historical evidence areas: ${addedFrozen.join(", ")}`);
console.log(`EVIDENCE_LIFECYCLE_RESULT=${errors.length ? "FAIL" : "PASS"} files=${tracked.length} bytes=${trackedBytes}`);
for (const error of errors) console.error("- " + error);
process.exit(errors.length ? 1 : 0);
