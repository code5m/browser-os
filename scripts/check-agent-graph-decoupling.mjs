#!/usr/bin/env node
// V2 hard gate: Agent and Graph may share payload identifiers, but must remain independently assemblable.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { parseYaml } from "./check-capability-registry.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(ROOT, path), "utf8");
const failures = [];
const fail = (message) => failures.push(message);

function manifestHasEdge(text, target) {
  return [...text.matchAll(/\b(?:dependsOn|dependencies|optionalDependencies)\s*:\s*\[([^\]]*)\]/g)]
    .some(match => [...match[1].matchAll(/["']([^"']+)["']/g)].some(item => item[1] === target));
}
function registryHasEdge(text, id, target) {
  const record = parseYaml(text).capabilities?.find(item => item.id === id);
  if (!record || !Array.isArray(record.dependsOn) || !Array.isArray(record.optionalDependencies)) throw Error(`Missing dependency contract: ${id}`);
  return [...record.dependsOn, ...record.optionalDependencies].includes(target);
}
function selfTest() {
  for (const [id, target] of [['agent','graph'],['graph','agent']]) {
    for (const field of ['dependsOn','dependencies','optionalDependencies']) assert.equal(manifestHasEdge(`${field}: ["${target}"]`,target),true);
    assert.equal(manifestHasEdge('optionalDependencies: ["script", "git", "database", "browser"]',target),false);
    for (const field of ['dependsOn','optionalDependencies']) {
      const text=`capabilities:\n  - id: ${id}\n    dependsOn: ${field==='dependsOn'?`[${target}]`:'[]'}\n    optionalDependencies: ${field==='optionalDependencies'?`[${target}]`:'[script, git, database, browser]'}\n`;
      assert.equal(registryHasEdge(text,id,target),true);
    }
    assert.equal(registryHasEdge(`capabilities:\n  - id: ${id}\n    dependsOn: [bridge]\n    optionalDependencies: [script, git, database, browser]\n`,id,target),false);
    assert.throws(()=>registryHasEdge('capabilities: []',id,target));
  }
  console.log('AGENT_GRAPH_DECOUPLING_SELF_TEST=PASS required/optional/both-directions/other-dependencies/missing-contract');
}
selfTest();

const agentManifest = read("src/capabilities/agent/manifest.ts");
const graphManifest = read("src/capabilities/graph/manifest.ts");
const agentSource = read("src/capabilities/agent/index.ts") + read("src/capabilities/agent/state/useAgentStore.ts");
const graphSource = read("src/capabilities/graph/index.ts") + read("src/capabilities/graph/state/useGraphStore.ts");
const capabilities = read("docs/architecture/capability-registry/capabilities.yaml");
const dependencies = read("docs/architecture/capability-registry/dependencies.yaml");

if (manifestHasEdge(agentManifest, 'graph')) {
  fail("agent manifest must not depend on graph");
}
if (manifestHasEdge(graphManifest, 'agent')) {
  fail("graph manifest must not depend on agent");
}
if (/from:\s*agent[^\n}]*to:\s*graph/.test(dependencies) || /from:\s*graph[^\n}]*to:\s*agent/.test(dependencies)) {
  fail("dependencies.yaml must not contain agent<->graph runtime edges");
}

if (registryHasEdge(capabilities,'agent','graph')) fail("capabilities.yaml agent must not depend on graph");
if (registryHasEdge(capabilities,'graph','agent')) fail("capabilities.yaml graph must not depend on agent");

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
