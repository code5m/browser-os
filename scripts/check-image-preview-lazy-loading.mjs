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

const { bridge } = await import('../src/bridge.ts');
const { createPinia, setActivePinia } = await import('pinia');
const { useWorkspaceStore } = await import('../src/capabilities/workspace/state/useWorkspaceStore.ts');

const entries = Array.from({ length: 20 }, (_, i) => ({
  name: `image-${String(i).padStart(2, '0')}.jpg`,
  path: `/tmp/images/image-${String(i).padStart(2, '0')}.jpg`,
  is_dir: false,
  size: 1024,
}));

let reads = 0;
let active = 0;
let maxActive = 0;
bridge.listDir = async () => entries;
bridge.readImageDataUrl = async (path) => {
  reads += 1;
  active += 1;
  maxActive = Math.max(maxActive, active);
  await new Promise((resolve) => setTimeout(resolve, 5));
  active -= 1;
  return `data:image/jpeg;base64,${Buffer.from(path).toString('base64')}`;
};

setActivePinia(createPinia());
const workspace = useWorkspaceStore();

await workspace.openDirPreview({ name: 'images', path: '/tmp/images', is_dir: true, size: 0 });
assert.equal(reads, 0, 'opening a directory must not eagerly read image data URLs');

for (const entry of entries.slice(0, 10)) workspace.loadPreviewImage(entry);

for (let i = 0; i < 80; i += 1) {
  if (Object.keys(workspace.previewImages).length === 10) break;
  await new Promise((resolve) => setTimeout(resolve, 5));
}

assert.equal(Object.keys(workspace.previewImages).length, 10, 'lazy image loads should complete');
assert.ok(maxActive <= 4, `image preview concurrency exceeded: ${maxActive}`);

console.log('IMAGE_PREVIEW_LAZY_LOADING=PASS');
