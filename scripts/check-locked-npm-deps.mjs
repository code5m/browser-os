import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Check direct dependencies before a release build. Checking for one package
// (e.g. terser) misses a stale node_modules tree left by a branch switch.
const root = resolve(process.argv[2] ?? fileURLToPath(new URL('..', import.meta.url)));
function readJson(path) {
  try { return JSON.parse(readFileSync(path, 'utf8')); }
  catch { return null; }
}
const manifest = readJson(join(root, 'package.json'));
const lock = readJson(join(root, 'package-lock.json'));
if (!manifest || !lock?.packages) {
  console.error('LOCKED_NPM_DEPS=FAIL: missing package.json or package-lock.json');
  process.exit(1);
}
let failures = 0;
for (const name of Object.keys({ ...manifest.dependencies, ...manifest.devDependencies }).sort()) {
  const expected = lock.packages[`node_modules/${name}`]?.version;
  const actual = readJson(join(root, 'node_modules', name, 'package.json'))?.version;
  if (!expected || actual !== expected) {
    console.error(`[npm-lock] ${name}: expected ${expected ?? '<not locked>'}, installed ${actual ?? '<missing>'}`);
    failures++;
  }
}
if (failures) {
  console.error(`LOCKED_NPM_DEPS=FAIL (${failures} direct dependency mismatches)`);
  process.exitCode = 1;
} else {
  console.log('LOCKED_NPM_DEPS=PASS (direct dependencies match package-lock.json)');
}
