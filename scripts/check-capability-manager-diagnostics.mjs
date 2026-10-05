#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const panel = readFileSync(join(ROOT, "src/components/system/CapabilityManagerPanel.vue"), "utf8");
const runtime = readFileSync(join(ROOT, "src/capability/runtime.ts"), "utf8");
const failures = [];
const requireText = (needle, label) => { if (!panel.includes(needle)) failures.push(label); };

requireText("CAPABILITY_CATALOG", "Manager must read canonical catalog");
requireText("CAPABILITY_DEFINITIONS", "Manager must read runtime definitions");
requireText("contributionRegistry", "Manager must read live contributions");
requireText("peekCapabilityRuntime", "Manager must read live runtime");
requireText("manifest.id", "missing capability id diagnostic");
requireText("manifest.displayName", "missing display name diagnostic");
requireText("manifest.maturity", "missing maturity diagnostic");
requireText("manifest.semanticOwner", "missing semantic owner diagnostic");
requireText("manifest.dependencies", "missing dependencies diagnostic");
requireText("manifest.optionalDependencies", "missing optional dependencies diagnostic");
requireText("dependent", "missing dependents diagnostic");
requireText("contributions", "missing contribution diagnostic");
requireText("activationDurationMs", "missing activation duration diagnostic");
requireText("persistenceState", "missing persistence diagnostic");
requireText("lastError", "missing last error diagnostic");
requireText("sourceOwnership", "missing package/source ownership diagnostic");
requireText("blockedReason", "missing blocked reason diagnostic");
requireText("suspendable", "missing suspendable diagnostic");
requireText("disableable", "missing disableable diagnostic");

if (!runtime.includes("activationDurationMs") || !runtime.includes("lastError")) {
  failures.push("Runtime must own activation duration and last-error diagnostics");
}
if (/const\s+(?:capabilities|capabilityRows)\s*=\s*\[/.test(panel)) {
  failures.push("Manager must not introduce a handwritten capability list");
}

if (failures.length) {
  for (const failure of failures) console.error("[FAIL] " + failure);
  console.error("CAPABILITY_MANAGER_DIAGNOSTICS_RESULT=FAIL");
  process.exit(1);
}
console.log("CAPABILITY_MANAGER_DIAGNOSTICS_RESULT=PASS");
