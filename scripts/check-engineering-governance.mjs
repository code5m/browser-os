#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const governancePath = path.join(root, "docs/engineering/governance.json");
const errors = [];

const fail = (message) => errors.push(message);
const exists = (repoPath) => fs.existsSync(path.join(root, repoPath));

function uniqueIds(items, label) {
  const seen = new Set();
  for (const item of items ?? []) {
    if (!item?.id) {
      fail(`${label}: item missing id`);
      continue;
    }
    if (seen.has(item.id)) fail(`${label}: duplicate id '${item.id}'`);
    seen.add(item.id);
  }
}

function listFiles(dir, predicate = () => true) {
  const abs = path.join(root, dir);
  if (!fs.existsSync(abs)) return [];
  return fs.readdirSync(abs, { withFileTypes: true })
    .filter((entry) => entry.isFile() && predicate(entry.name))
    .map((entry) => path.posix.join(dir, entry.name))
    .sort();
}

function checkRegisteredPaths(items, label) {
  for (const item of items ?? []) {
    const paths = item.path ? [item.path] : item.paths ?? [];
    for (const repoPath of paths) {
      if (!exists(repoPath)) fail(`${label} '${item.id ?? "unknown"}' references missing path: ${repoPath}`);
    }
  }
}

if (!exists("docs/engineering/governance.json")) {
  console.error("ENGINEERING_GOVERNANCE_RESULT=FAIL missing docs/engineering/governance.json");
  process.exit(1);
}

let governance;
try {
  governance = JSON.parse(fs.readFileSync(governancePath, "utf8"));
} catch (error) {
  console.error(`ENGINEERING_GOVERNANCE_RESULT=FAIL invalid JSON: ${error.message}`);
  process.exit(1);
}

if (governance.schemaVersion !== 1) fail("schemaVersion must be 1");
if (governance.repository !== "code5m/browser-os") fail("repository must be code5m/browser-os");
if (governance.sourceOfTruth?.primary !== "GitHub") fail("GitHub must remain the primary source of truth");
if (governance.sourceOfTruth?.defaultBranch !== "master") fail("default branch governance must be master");
if (governance.sourceOfTruth?.mirror?.role !== "non_authoritative_backup") {
  fail("Gitee mirror must be explicitly classified as non_authoritative_backup");
}

for (const [label, items] of Object.entries({
  workflows: governance.workflows,
  gitHooks: governance.gitHooks,
  gateFamilies: governance.gateFamilies,
  standards: governance.standards,
  assets: governance.assets,
  peripheralEngineering: governance.peripheralEngineering,
})) {
  uniqueIds(items, label);
  checkRegisteredPaths(items, label);
}

for (const doc of governance.requiredHumanDocs ?? []) {
  if (!exists(doc)) fail(`required human document missing: ${doc}`);
}

const actualWorkflows = listFiles(".github/workflows", (name) => /\.ya?ml$/.test(name));
const registeredWorkflows = (governance.workflows ?? []).map((item) => item.path).sort();
if (JSON.stringify(actualWorkflows) !== JSON.stringify(registeredWorkflows)) {
  fail(`workflow registry drift: actual=${actualWorkflows.join(",")} registered=${registeredWorkflows.join(",")}`);
}

const actualHooks = listFiles(".githooks");
const registeredHooks = (governance.gitHooks ?? []).map((item) => item.path).sort();
if (JSON.stringify(actualHooks) !== JSON.stringify(registeredHooks)) {
  fail(`git hook registry drift: actual=${actualHooks.join(",")} registered=${registeredHooks.join(",")}`);
}

let pkg;
try {
  pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
} catch (error) {
  fail(`package.json unreadable: ${error.message}`);
}
if (pkg) {
  if (pkg.scripts?.["check:engineering-governance"] !== "node scripts/check-engineering-governance.mjs") {
    fail("package.json must expose check:engineering-governance");
  }
  if (!pkg.scripts?.check?.includes("check-engineering-governance.mjs")) {
    fail("npm run check must include check-engineering-governance.mjs");
  }
}

const governanceWorkflow = path.join(root, ".github/workflows/engineering-governance.yml");
if (fs.existsSync(governanceWorkflow)) {
  const source = fs.readFileSync(governanceWorkflow, "utf8");
  if (!source.includes("scripts/check-engineering-governance.mjs")) fail("engineering governance workflow must execute governance checker");
  if (!source.includes("- master")) fail("engineering governance workflow must cover master pushes");
  if (!source.includes("feature/**")) fail("engineering governance workflow must cover feature/** pushes");
}

for (const workflow of [
  ".github/workflows/capability-v2-ui-safety.yml",
  ".github/workflows/capability-v2-validation.yml",
  ".github/workflows/capability-v3-hotplug.yml",
]) {
  if (!exists(workflow)) continue;
  const source = fs.readFileSync(path.join(root, workflow), "utf8");
  if (!source.includes("- master")) fail(`${workflow} must run on master pushes`);
  if (!source.includes("feature/**")) fail(`${workflow} must cover feature/** pushes`);
}

if (governance.stableBaseline?.tag !== "capability-platform-v4-stable") {
  fail("stable baseline tag must remain capability-platform-v4-stable until an explicit later freeze updates governance");
}
if (governance.stableBaseline?.commit !== "a6ff92674db98ffad964b784167fdb8c9a98f3cc") {
  fail("v4 stable baseline commit drifted");
}

if (errors.length) {
  console.error("ENGINEERING_GOVERNANCE_RESULT=FAIL");
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(
  `ENGINEERING_GOVERNANCE_RESULT=PASS workflows=${actualWorkflows.length} hooks=${actualHooks.length} gates=${governance.gateFamilies.length} standards=${governance.standards.length}`,
);
