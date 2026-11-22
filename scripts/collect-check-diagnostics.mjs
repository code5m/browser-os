#!/usr/bin/env node
// Failure-only diagnostics. The original npm gate remains failed regardless of
// these reruns. Execute only the simple commands already declared by that gate.
import { spawn } from 'node:child_process';
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { tmpdir } from 'node:os';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));
const reportPath = join(process.env.RUNNER_TEMP || tmpdir(), 'browseros-check-diagnostics.json');
const deadline = Date.now() + 12 * 60 * 1000;
let interrupted = false;
let stopCurrent = () => {};
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => {
  interrupted = true;
  stopCurrent('INTERRUPTED');
});
const selfTest = process.argv.length === 3 && process.argv[2] === '--self-test';
if (process.platform !== 'linux' || (!selfTest && process.argv.length !== 2)) {
  throw new Error('Check diagnostics require the Linux CI runner and no command arguments, or --self-test');
}

function parseCommand(text) {
  const args = text.trim().split(/\s+/);
  if (!args.length || args.some(arg => !/^[A-Za-z0-9_./:@=+-]+$/.test(arg))) {
    throw new Error('UNSUPPORTED_COMMAND_SYNTAX');
  }
  const [program, ...rest] = args;
  if (program === 'npm') {
    if (rest.length !== 2 || rest[0] !== 'run' || !Object.hasOwn(pkg.scripts, rest[1])
        || ['check', 'precheck'].includes(rest[1])) throw new Error('UNSUPPORTED_NPM_COMMAND');
  } else if (program === 'node' || program === 'python3') {
    if (!/^(scripts|tests)\/[A-Za-z0-9_./-]+\.(mjs|cjs|js|py)$/.test(rest[0] || '')
        || rest[0].split('/').includes('..') || !existsSync(join(root, rest[0]))) {
      throw new Error('UNSUPPORTED_CHECK_SCRIPT');
    }
  } else {
    throw new Error('UNSUPPORTED_CHECK_PROGRAM');
  }
  return { command: args.join(' '), program: program === 'node' ? process.execPath : program, args: rest };
}

const cases = [];
for (const phase of ['precheck', 'check']) {
  if (typeof pkg.scripts?.[phase] !== 'string') throw new Error('CHECK_CHAIN_MISSING');
  for (const [index, text] of pkg.scripts[phase].split('&&').entries()) {
    try {
      cases.push({ phase, index: index + 1, ...parseCommand(text), status: 'NOT_RUN' });
    } catch (error) {
      // Do not echo unsupported shell text; never eval it or pass it to a shell.
      cases.push({ phase, index: index + 1, command: phase + '[' + (index + 1) + ']',
        status: 'BLOCKED', reason: error.message });
    }
  }
}
async function persist() {
  await writeFile(reportPath, JSON.stringify({
    original_gate: 'FAILED', diagnostic_only: true,
    commands: cases.map(({ program, args, ...record }) => record),
  }, null, 2) + '\n');
}

function groupExists(pid) {
  try { process.kill(-pid, 0); return true; }
  catch (error) { if (error.code === 'ESRCH') return false; throw error; }
}
function signalGroup(pid, signal) {
  if (!pid) return;
  try { process.kill(-pid, signal); } catch (error) { if (error.code !== 'ESRCH') throw error; }
}
async function awaitNaturalGroupExit(pid) {
  // A normal worker can outlive its parent's exit notification while handling
  // EOF. Allow bounded shutdown before treating it as an escaped descendant.
  const until = Date.now() + 500;
  while (pid && groupExists(pid) && Date.now() < until) await delay(25);
  return !pid || !groupExists(pid);
}
async function execute(item, budget) {
  const started = Date.now();
  return await new Promise(resolveResult => {
    let reason = '';
    let head = '';
    let tail = '';
    let bytes = 0;
    let code = null;
    let finished = false;
    let escalation;
    const child = spawn(item.program, item.args, {
      cwd: root, shell: false, detached: true, stdio: ['ignore', 'pipe', 'pipe'],
    });
    const stop = why => {
      reason ||= why;
      signalGroup(child.pid, 'SIGTERM');
      escalation ||= setTimeout(() => signalGroup(child.pid, 'SIGKILL'), 1000);
    };
    stopCurrent = stop;
    const timer = setTimeout(() => stop('TIMEOUT'), budget);
    const finish = async () => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      // npm may exit before its owned child processes. Clean only this new
      // process group and verify it is gone before starting another command.
      if (child.pid && !(await awaitNaturalGroupExit(child.pid))) {
        reason ||= 'DESCENDANT_CLEANUP_REQUIRED';
        signalGroup(child.pid, 'SIGKILL');
        for (let attempt = 0; attempt < 40 && groupExists(child.pid); attempt++) await delay(50);
        if (groupExists(child.pid)) reason = 'CLEANUP_UNCONFIRMED';
      }
      clearTimeout(escalation);
      stopCurrent = () => {};
      resolveResult({ code, reason, duration_ms: Date.now() - started,
        output_bytes: bytes, head, tail });
    };
    const capture = data => {
      bytes += data.length;
      const text = data.toString('utf8').replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, '').replace(/\r/g, '');
      if (head.length < 4096) head += text.slice(0, 4096 - head.length);
      tail = (tail + text).slice(-8192);
      if (bytes > 8 * 1024 * 1024) stop('OUTPUT_LIMIT');
    };
    child.stdout.on('data', capture);
    child.stderr.on('data', capture);
    child.once('error', () => { reason = 'SPAWN_FAILED'; void finish(); });
    child.once('exit', exitCode => {
      code = exitCode;
      if (child.pid) void (async () => {
        // Also supervise a descendant that retains stdout/stderr and prevents
        // the close event. finish() owns cleanup if that event arrives first.
        if (!(await awaitNaturalGroupExit(child.pid)) && !finished) {
          stop('DESCENDANT_CLEANUP_REQUIRED');
        }
      })();
    });
    child.once('close', () => { void finish(); });
  });
}


function diagnosticStatus(result) {
  return result.reason ? 'BLOCKED' : result.code === 0 ? 'PASSED' : 'FAILED';
}
if (selfTest) {
  const run = async (code, budget = 3000) => execute({
    program: process.execPath, args: ['-e', code],
  }, budget);
  assert.equal(diagnosticStatus(await run('process.exit(0)')), 'PASSED');
  assert.equal(diagnosticStatus(await run('process.exit(7)')), 'FAILED');
  const spawnChild = (body, stdio) =>
    "require('node:child_process').spawn(process.execPath, ['-e', " +
    JSON.stringify(body) + "], {stdio:" + JSON.stringify(stdio) + "}).unref()";
  const natural = await run(spawnChild('setTimeout(() => process.exit(0), 30)', 'ignore'));
  assert.equal(diagnosticStatus(natural), 'PASSED', 'normal worker shutdown must complete without forced cleanup');
  for (const stdio of ['ignore', 'inherit']) {
    const leaked = await run(spawnChild('setInterval(() => {}, 1000)', stdio));
    assert.equal(leaked.code, 0);
    assert.equal(leaked.reason, 'DESCENDANT_CLEANUP_REQUIRED', 'surviving descendants must be rejected and cleaned');
    assert.equal(diagnosticStatus(leaked), 'BLOCKED');
  }
  const timedOut = await run('setInterval(() => {}, 1000)', 100);
  assert.equal(timedOut.reason, 'TIMEOUT');
  assert.equal(diagnosticStatus(timedOut), 'BLOCKED');
  console.log('CHECK_DIAGNOSTICS_SELF_TEST=PASS cases=6 (real owned process groups)');
  process.exit(0);
}

await persist();
for (const item of cases) {
  if (item.status !== 'NOT_RUN') continue;
  if (interrupted || Date.now() >= deadline) break;
  item.status = 'RUNNING';
  await persist();
  console.log('::group::Check diagnostics: ' + item.phase + '[' + item.index + '] ' + item.command);
  const result = await execute(item, Math.min(120000, deadline - Date.now()));
  item.exit_code = result.code;
  item.duration_ms = result.duration_ms;
  item.output_bytes = result.output_bytes;
  item.status = diagnosticStatus(result);
  if (result.reason) item.reason = result.reason;
  console.log('DIAGNOSTIC_STATUS=' + item.status + ' exit=' + item.exit_code + ' duration_ms=' + item.duration_ms + (result.reason ? ' reason=' + result.reason : ''));
  if (item.status !== 'PASSED') {
    // GitHub's normal secret masker still applies. Never print environment or
    // credentials. Bound child output and disable workflow-command parsing.
    const token = randomUUID();
    console.log('::stop-commands::' + token);
    if (result.output_bytes > 8192) console.log(result.head + '\n[... bounded diagnostic tail ...]');
    console.log(result.tail);
    console.log('::' + token + '::');
  }
  console.log('::endgroup::');
  await persist();
  if (result.reason === 'CLEANUP_UNCONFIRMED') break;
}
const failures = cases.filter(item => item.status !== 'PASSED');
const summary = [
  '### Independent npm check diagnostics',
  '',
  'The original npm gate remains FAILED. These reruns do not replace it.',
  '',
  '| Phase | Command | Diagnostic status | Exit |',
  '| --- | --- | --- | --- |',
  ...cases.map(item => '| ' + item.phase + ' | ' + item.command + ' | ' + item.status
    + (item.reason ? ': ' + item.reason : '') + ' | ' + (item.exit_code ?? '') + ' |'),
  '',
];
if (process.env.GITHUB_STEP_SUMMARY) {
  const existing = await readFile(process.env.GITHUB_STEP_SUMMARY, 'utf8').catch(() => '');
  await writeFile(process.env.GITHUB_STEP_SUMMARY, existing + summary.join('\n'));
}
await persist();
console.log('CHECK_DIAGNOSTICS_COMPLETE commands=' + cases.length + ' nonpassing=' + failures.length + '; original gate remains FAILED');
process.exitCode = failures.length ? 1 : 0;
