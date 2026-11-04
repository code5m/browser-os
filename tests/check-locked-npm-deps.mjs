import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const script = resolve('scripts/check-locked-npm-deps.mjs');
const root = mkdtempSync(join(tmpdir(), 'browseros-锁文件-'));
const name = '@tauri-apps/api';
function save(filename, value) {
  mkdirSync(join(root, filename, '..'), { recursive: true });
  writeFileSync(join(root, filename), JSON.stringify(value));
}
function check() {
  return spawnSync(process.execPath, [script, root], { encoding: 'utf8' });
}
try {
  save('package.json', { dependencies: { [name]: '^2.0.0' } });
  save('package-lock.json', { packages: { [`node_modules/${name}`]: { version: '2.11.1' } } });
  save(`node_modules/${name}/package.json`, { version: '2.11.1' });
  assert.equal(check().status, 0, 'matching lock must pass');
  save(`node_modules/${name}/package.json`, { version: '2.12.4' });
  const mismatch = check();
  assert.equal(mismatch.status, 1, 'outdated node_modules must fail');
  assert.match(mismatch.stderr, /expected 2\.11\.1, installed 2\.12\.4/);
  rmSync(join(root, 'node_modules'), { recursive: true });
  assert.equal(check().status, 1, 'missing dependencies must fail');
  save('package-lock.json', { packages: {} });
  assert.equal(check().status, 1, 'missing lock entry must fail');
  console.log('LOCKED_NPM_DEPS_TEST=PASS including non-ASCII workspace');
} finally {
  rmSync(root, { recursive: true, force: true });
}
