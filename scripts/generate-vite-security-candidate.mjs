// Run only in an explicitly requested CI diagnostic; never edits the checkout.
import { appendFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { isDeepStrictEqual } from 'node:util';

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const gitBlob = (bytes) => execFileSync('git', ['hash-object', '--stdin'], {
  input: bytes, encoding: 'utf8',
}).trim();
const sourceSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (!/^[0-9a-f]{40}$/.test(sourceSha) ||
    (process.env.GITHUB_SHA && process.env.GITHUB_SHA !== sourceSha)) {
  throw new Error('CANDIDATE_SOURCE_SHA_MISMATCH');
}
const inputs = Object.fromEntries(['package.json', 'package-lock.json'].map((name) => [
  name, execFileSync('git', ['show', sourceSha + ':' + name]),
]));
const expected = JSON.parse(inputs['package.json']);
if (expected.devDependencies?.vite !== '^5.2.0') {
  throw new Error('CANDIDATE_VITE_BASELINE_CHANGED');
}
expected.devDependencies.vite = '6.4.6';
const directory = mkdtempSync(join(process.env.RUNNER_TEMP || tmpdir(), 'browseros-vite-candidate-'));
writeFileSync(join(directory, 'package.json'), JSON.stringify(expected, null, 2) + '\n');
writeFileSync(join(directory, 'package-lock.json'), inputs['package-lock.json']);
const registry = '--registry=https://registry.npmjs.org';
const install = spawnSync('npm', [
  'install', '--package-lock-only', '--ignore-scripts', '--no-audit', '--no-fund', registry,
], { cwd: directory, stdio: 'inherit', timeout: 180_000 });
if (install.error || install.status !== 0) throw new Error('CANDIDATE_LOCK_RESOLUTION_FAILED');
const files = Object.fromEntries(['package.json', 'package-lock.json'].map((name) => [
  name, readFileSync(join(directory, name)),
]));
const generatedPackage = JSON.parse(files['package.json']);
const generatedLock = JSON.parse(files['package-lock.json']);
if (!isDeepStrictEqual(generatedPackage, expected) ||
    generatedLock.lockfileVersion !== JSON.parse(inputs['package-lock.json']).lockfileVersion ||
    generatedLock.packages?.['']?.devDependencies?.vite !== '6.4.6' ||
    generatedLock.packages?.['node_modules/vite']?.version !== '6.4.6') {
  throw new Error('CANDIDATE_LOCK_CONTRACT_MISMATCH');
}
// Reproduce the generated native declaration report in a separate temporary
// source tree. The normal checkout and ordinary check gates stay untouched.
const nativeReport = 'docs/architecture/native-boundary/NATIVE_OWNER_EFFECT_AUDIT_20261102.md';
const nativeInputs = [
  'scripts/check-native-owner-effect-audit.mjs',
  'docs/architecture/native-boundary/native-commands.yaml',
  'docs/architecture/native-boundary/native-permission-debt-baseline.json',
  'docs/engineering/governance.json',
  'src-tauri/capabilities/default.json',
  'src-tauri/capabilities/browser-remote.json',
  'src-tauri/permissions/default-commands.toml',
  'src-tauri/permissions/remote-collect.toml',
];
const nativeDirectory = mkdtempSync(join(process.env.RUNNER_TEMP || tmpdir(), 'browseros-native-report-'));
for (const name of nativeInputs) {
  const bytes = execFileSync('git', ['show', sourceSha + ':' + name]);
  inputs[name] = bytes;
  const destination = join(nativeDirectory, name);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, bytes);
}
execFileSync(process.execPath, [
  join(nativeDirectory, 'scripts/check-native-owner-effect-audit.mjs'), '--generate',
], { cwd: nativeDirectory, stdio: 'inherit', timeout: 30000 });
files[nativeReport] = readFileSync(join(nativeDirectory, nativeReport));
mkdirSync(dirname(join(directory, nativeReport)), { recursive: true });
writeFileSync(join(directory, nativeReport), files[nativeReport]);
const audit = spawnSync('npm', [
  'audit', '--include=dev', '--audit-level=low', '--json', registry,
], { cwd: directory, encoding: 'utf8', timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
writeFileSync(join(directory, 'npm-audit.json'), audit.stdout || '{}');
const metadata = {
  source_sha: sourceSha,
  target_vite: '6.4.6',
  source_sha256: Object.fromEntries(Object.entries(inputs).map(([name, bytes]) => [name, hash(bytes)])),
  candidate_sha256: Object.fromEntries(Object.entries(files).map(([name, bytes]) => [name, hash(bytes)])),
  candidate_git_blob: Object.fromEntries(Object.entries(files).map(([name, bytes]) => [name, gitBlob(bytes)])),
  npm_version: execFileSync('npm', ['--version'], { encoding: 'utf8' }).trim(),
  audit_status: audit.status,
  audit_error: Boolean(audit.error),
};
writeFileSync(join(directory, 'metadata.json'), JSON.stringify(metadata, null, 2) + '\n');
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, 'directory=' + directory + '\n');
console.log('NODE_DEPENDENCY_CANDIDATE_METADATA=' + JSON.stringify(metadata));
for (const [name, bytes] of Object.entries(files)) {
  if (bytes.length > 1024 * 1024) throw new Error('CANDIDATE_EXPORT_TOO_LARGE');
  const digest = hash(bytes);
  const blob = metadata.candidate_git_blob[name];
  if (!/^[0-9a-f]{40}$/.test(blob)) throw new Error('CANDIDATE_GIT_BLOB_INVALID');
  const encoded = bytes.toString('base64');
  const total = Math.ceil(encoded.length / 3072);
  console.log('NODE_DEPENDENCY_FILE_BEGIN source=' + sourceSha + ' file=' + name +
    ' git_blob=' + blob + ' sha256=' + digest + ' bytes=' + bytes.length + ' chunks=' + total);
  for (let index = 0; index < total; index++) {
    console.log('NODE_DEPENDENCY_FILE_CHUNK source=' + sourceSha + ' file=' + name +
      ' git_blob=' + blob + ' sha256=' + digest + ' part=' + (index + 1) + '/' + total +
      ' base64=' + encoded.slice(index * 3072, (index + 1) * 3072));
  }
  console.log('NODE_DEPENDENCY_FILE_END source=' + sourceSha + ' file=' + name + ' git_blob=' + blob + ' sha256=' + digest);
}
console.log('NODE_DEPENDENCY_CANDIDATE_AUDIT=' + (audit.stdout || '{}').trim());
if (audit.error || audit.status !== 0) process.exitCode = 1;
