import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';

registerHooks({ resolve(specifier, context, next) {
  try { return next(specifier, context); } catch (error) {
    for (const ext of ['.ts', '.mjs', '.js']) {
      try { return next(specifier + ext, context); } catch {}
    }
    throw error;
  }
} });
globalThis.window = { setTimeout };
const { createPinia, setActivePinia } = await import('pinia');
const { useFileStore } = await import('../src/capabilities/workspace/state/useFileStore.ts');
setActivePinia(createPinia());
const files = useFileStore();
assert.equal(files.inlineFile, '');
files.locateCurrent();
assert.equal(files.locateTarget, '');
console.log('WORKSPACE_STARTUP=PASS');
