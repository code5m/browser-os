'use strict';

const readline = require('node:readline');

// Backpressure fixture: keep the event loop alive without consuming stdin.
if (process.argv.includes('blocked')) {
  setInterval(() => {}, 1000);
} else {
  const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
  input.on('line', (line) => {
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      return;
    }
    if (message.method === 'notifications/initialized') return;
    let result;
    if (message.method === 'initialize') {
      result = {
        protocolVersion: '2024-11-05',
        capabilities: { tools: {} },
        serverInfo: { name: 'browseros-windows-fixture', version: '1.0.0' },
      };
    } else if (message.method === 'tools/list') {
      result = {
        tools: [
          { name: 'echo', description: 'echo fixture', inputSchema: { type: 'object' } },
          { name: 'echo_env', description: 'echo secret fixture', inputSchema: { type: 'object' } },
        ],
      };
    } else if (message.method === 'tools/call') {
      if (message.params.name === 'echo') {
        result = { content: [{ type: 'text', text: message.params.arguments.text || '' }] };
      } else if (message.params.name === 'echo_env') {
        result = { content: [{ type: 'text', text: process.env.FIXTURE_TOKEN || '' }] };
      } else {
        process.stdout.write(`${JSON.stringify({ jsonrpc: '2.0', id: message.id, error: { code: -32601, message: 'unknown tool' } })}\n`);
        return;
      }
    } else {
      process.stdout.write(`${JSON.stringify({ jsonrpc: '2.0', id: message.id, error: { code: -32601, message: 'unsupported' } })}\n`);
      return;
    }
    process.stdout.write(`${JSON.stringify({ jsonrpc: '2.0', id: message.id, result })}\n`);
  });
}
