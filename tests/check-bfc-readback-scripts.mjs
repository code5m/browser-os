import assert from 'node:assert/strict';
import { Script, runInNewContext } from 'node:vm';
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

// Exercise the JavaScript that is actually injected, not only the outer
// template source: escaping mistakes previously disabled numeric filtering.
const metricStart=favorites.indexOf('const isMetric = (value) => {');
const metricEnd=favorites.indexOf('const cleanTitle = (value) => {',metricStart);
const titleEnd=favorites.indexOf('for (const anchor of',metricEnd);
assert.ok(metricStart>0&&metricEnd>metricStart&&titleEnd>metricEnd);
const titleFilter=runInNewContext('(()=>{' + favorites.slice(metricStart,titleEnd) + 'return cleanTitle;})()');
for(const metric of [
  '5.2万 00:28','3.6万 13:00:52','3968 02:46','8533 2 01:22',
  '00:28','3968','播放 3968','评论数 29','收藏 128',
  'BV1xxxxxxxxx','BV1xxxxxxx'
]) assert.equal(titleFilter(metric),'','metric should not become a title: '+metric);
for(const title of [
  '如何使用 AI 自动整理收藏夹','从零实现一个浏览器工作台',
  '5.2 万用户都在看的教程'
]) assert.equal(titleFilter(title),title,'real title must be preserved: '+title);
assert.match(favorites,/BFC_START_FROM_FIRST_PAGE/);

console.log('BFC_READBACK_SCRIPTS=PASS (DOM-only, bounded, cancellable, no cookie/API access)');
