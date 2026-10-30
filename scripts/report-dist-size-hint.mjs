import { readdir, stat, readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

async function bytesIn(directory) {
  let bytes = 0;
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error('symbolic links are not counted');
    if (entry.isDirectory()) bytes += await bytesIn(path);
    else if (entry.isFile()) bytes += (await stat(path)).size;
  }
  return bytes;
}

try {
  const policy = JSON.parse(await readFile('docs/engineering/coding-editor-budget.json', 'utf8'));
  const limit = policy.limitsBytes.total_bytes;
  if (!Number.isSafeInteger(limit) || limit <= 0) throw new Error('invalid total budget');
  const bytes = await bytesIn('dist');
  const remaining = limit - bytes;
  console.log(`DIST_SIZE_HINT=${remaining < 0 ? 'OVER_LIMIT' : remaining < limit * 0.1 ? 'NEAR_LIMIT' : 'WITHIN_LIMIT'} bytes=${bytes} total_limit=${limit} remaining=${remaining}`);
  console.log('DIST_SIZE_HINT_SCOPE=existing dist only; freshness and ownership unverified; not a budget gate');
} catch (error) {
  console.log(`DIST_SIZE_HINT=UNAVAILABLE (${error.message}); run the full merge budget audit for verified measurements`);
}
