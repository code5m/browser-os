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
const { useWorkspaceStore } = await import('../src/stores/useWorkspaceStore.ts');
setActivePinia(createPinia());
const workspace = useWorkspaceStore();
assert.equal(workspace.inlineFile, '');
workspace.locateCurrent();
assert.equal(workspace.locateTarget, '');
console.log('WORKSPACE_STARTUP=PASS');
