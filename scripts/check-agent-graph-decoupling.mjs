#!/usr/bin/env node
// V2 hard gate: Agent and Graph may share payload identifiers, but must remain independently assemblable.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(ROOT, path), "utf8");
const failures = [];
const fail = (message) => failures.push(message);

const agentManifest = read("src/capabilities/agent/manifest.ts");
const graphManifest = read("src/capabilities/graph/manifest.ts");
const agentSource = read("src/capabilities/agent/index.ts") + read("src/capabilities/agent/state/useAgentStore.ts");
const graphSource = read("src/capabilities/graph/index.ts") + read("src/capabilities/graph/state/useGraphStore.ts");
const capabilities = read("docs/architecture/capability-registry/capabilities.yaml");
const dependencies = read("docs/architecture/capability-registry/dependencies.yaml");

if (/optionalDependencies\s*:\s*\[[^\]]*["']graph["']/.test(agentManifest)) {
  fail("agent manifest must not depend on graph");
}
if (/optionalDependencies\s*:\s*\[[^\]]*["']agent["']/.test(graphManifest)) {
  fail("graph manifest must not depend on agent");
}
if (/from:\s*agent[^\n}]*to:\s*graph/.test(dependencies) || /from:\s*graph[^\n}]*to:\s*agent/.test(dependencies)) {
  fail("dependencies.yaml must not contain agent<->graph runtime edges");
}

const agentBlock = capabilities.match(/\n  - id: agent\n[\s\S]*?(?=\n  - id: skill\n)/)?.[0] ?? "";
const graphBlock = capabilities.match(/\n  - id: graph\n[\s\S]*?(?=\n  - id: vault\n)/)?.[0] ?? "";
if (!/optionalDependencies:\s*\[\]/.test(agentBlock)) fail("capabilities.yaml agent optionalDependencies must be empty");
if (!/optionalDependencies:\s*\[\]/.test(graphBlock)) fail("capabilities.yaml graph optionalDependencies must be empty");

if (/from\s+["'][^"']*capabilities\/graph(?:\/|["'])/.test(agentSource)) {
  fail("agent must not import graph implementation");
}
if (/from\s+["'][^"']*capabilities\/agent(?:\/|["'])/.test(graphSource)) {
  fail("graph must not import agent implementation");
}

if (failures.length) {
  for (const failure of failures) console.error("[FAIL] " + failure);
  console.error("AGENT_GRAPH_DECOUPLING_RESULT=FAIL");
  process.exit(1);
}
console.log("AGENT_GRAPH_DECOUPLING_RESULT=PASS");
