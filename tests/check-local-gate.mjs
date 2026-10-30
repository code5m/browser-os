import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const help = spawnSync('bash', ['scripts/gate-local.sh', '--help'], { encoding: 'utf8' });
assert.equal(help.status, 0);
assert.match(help.stdout, /quick\|coding\|merge/);
assert.match(help.stdout, /No mode.*GUI acceptance/);
const invalid = spawnSync('bash', ['scripts/gate-local.sh', 'not-a-gate'], { encoding: 'utf8' });
assert.equal(invalid.status, 2, 'unknown gate names must not silently pass');
const script = readFileSync('scripts/gate-local.sh', 'utf8');
assert.match(script, /set -euo pipefail/);
for (const required of ['npm run check', 'cargo test', 'check-coding-ui-managed.mjs', 'scripts/pre-merge.sh', 'REAL_PROVIDER_MCP=NOT_VERIFIED PACKAGE=NOT_VERIFIED GUI=GUI_PENDING']) assert.ok(script.includes(required), `missing ${required}`);
assert.match(readFileSync('.githooks/pre-merge-commit', 'utf8'), /gate-local\.sh.*merge/);
assert.match(script, /coding\)[\s\S]*check:coding-workbench[\s\S]*check-coding-ui-managed\.mjs[\s\S]*CODING_CHECKS=PASS/);
assert.doesNotMatch(script, /report-dist-size-hint/);
console.log('LOCAL_GATE_CONTRACT=PASS (entry and scope contract; not a substitute for gate execution)');
