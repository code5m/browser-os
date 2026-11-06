import assert from 'node:assert/strict';
import { Script } from 'node:vm';
import { buildFavoritesReadbackScript, buildLoginProbeScript } from '../src/capabilities/bfc/model/readbackScripts.mjs';

const login = buildLoginProbeScript('01234567-89ab-cdef-0123-456789abcdef');
const favorites = buildFavoritesReadbackScript('01234567-89ab-cdef-0123-456789abcdef', '123456');

assert.doesNotThrow(() => new Script(login), 'login probe must be valid injected JavaScript');
assert.doesNotThrow(() => new Script(favorites), 'favorites readback must be valid injected JavaScript');
for (const script of [login, favorites]) {
  assert.match(script, /bfc_readback_submit/);
  assert.doesNotMatch(script, /document\.cookie|fetch\s*\(|XMLHttpRequest|WebSocket|axios\./i);
  assert.doesNotMatch(script, /api\.bilibili\.com|\/x\/v\d+\/|passport\.bilibili\.com\/x/i);
}
assert.match(login, /logged_in/);
assert.match(login, /logged_out/);
assert.match(login, /access_limited/);
assert.match(favorites, /BFC_READBACK_TOTAL_MISMATCH/);
assert.match(favorites, /BFC_BILIBILI_ACCESS_LIMITED/);
assert.match(favorites, /__browserosBfcCancelScan/);

console.log('BFC_READBACK_SCRIPTS=PASS (DOM-only, bounded, cancellable, no cookie/API access)');
