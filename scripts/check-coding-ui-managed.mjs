import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';

// The listener is only used to choose a free port; Vite must still bind it strictly.
const listener = createServer();
await new Promise((resolve, reject) => listener.once('error', reject).listen(0, '127.0.0.1', resolve));
const port = listener.address().port;
await new Promise((resolve, reject) => listener.close(error => error ? reject(error) : resolve()));
const base = `http://127.0.0.1:${port}`;
const server = spawn(process.execPath, ['node_modules/vite/bin/vite.js', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { stdio: ['ignore', 'pipe', 'pipe'] });
let serverExit = null;
let log = '';
server.on('exit', code => { serverExit = code ?? -1; });
for (const stream of [server.stdout, server.stderr]) stream.on('data', chunk => { log = (log + chunk.toString()).slice(-8000); });
const tests = ['check-coding-workbench-ui.cjs', 'check-coding-command-ui.cjs', 'check-coding-operation-ui.cjs', 'check-coding-workspace-context.cjs', 'check-coding-lsp-ui.cjs', 'check-coding-typescript-ui.cjs', 'check-coding-typescript-workbench-ui.cjs', 'check-wechat-draft-ui.cjs'];
try {
  let ready = false;
  for (let attempt = 0; attempt < 100 && serverExit === null; attempt++) {
    try { if ((await fetch(`${base}/`)).ok) { ready = true; break; } } catch { /* wait for the owned server */ }
    await delay(100);
  }
  if (!ready) throw new Error(`Test server unavailable (exit=${serverExit}): ${log}`);
  for (const test of tests) {
    console.log(`[coding-ui] ${test}`);
    const child = spawn(process.execPath, [`tests/${test}`], { stdio: 'inherit', env: { ...process.env, CODING_UI_URL: base } });
    const code = await new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', (status, signal) => resolve(status ?? (signal ? 1 : 1))); });
    if (code !== 0) throw new Error(`${test} failed (${code})`);
  }
  console.log('CODING_UI_GATE=PASS (real components; simulated native transport; not installed GUI)');
} catch (error) {
  console.error(`CODING_UI_GATE=FAIL ${error.message}`);
  process.exitCode = 1;
} finally {
  if (serverExit === null) {
    server.kill('SIGTERM');
    await Promise.race([new Promise(resolve => server.once('exit', resolve)), delay(3000)]);
    if (server.exitCode === null && server.signalCode === null) server.kill('SIGKILL');
  }
}
