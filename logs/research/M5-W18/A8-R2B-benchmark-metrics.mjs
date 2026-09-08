// A8 R2B metrics correction benchmark.
// Fixes A0 R2B findings #7 (precision vs recall naming) and #8 (RSS boundary).
// Uses the npm @zvec/zvec 0.7.0 Node N-API binding (one host runtime of the
// alibaba/zvec engine v0.7.0). The engine is also reachable from Rust via the
// official `zvec-rust` 0.7.0 crate (see A7 f157eb4) — route behavior is
// engine-level and identical across bindings; only the host runtime differs.
import {
  ZVecInitialize, ZVecSetDefaultJiebaDictDir, ZVecCollectionSchema, ZVecCreateAndOpen,
  ZVecIndexType, ZVecDataType, ZVecMetricType,
} from '@zvec/zvec';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execSync } from 'node:child_process';

const COLL = path.join(os.tmpdir(), `m5w18-a8-r2b-${Date.now()}`);
const JIEBA = path.join('node_modules', '@zvec', 'bindings-linux-x64', 'jieba_dict');
ZVecSetDefaultJiebaDictDir(JIEBA);
ZVecInitialize({ logLevel: 3 });

const N = 1000, DIM = 256;
const TOPICS = ['auth', 'payment', 'search', 'cache', 'network'];
const ZH = { auth: '认证', payment: '支付', search: '检索', cache: '缓存', network: '网络' };
const CODE = { auth: 'fn authenticate(req)', payment: 'fn charge_card()', search: 'fn query_index()', cache: 'fn get_cache()', network: 'fn send_packet()' };

let seed = 987654321;
const rnd = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; };
const randVec = () => Array.from({ length: DIM }, () => rnd() * 2 - 1);
const topicCenters = TOPICS.map(() => randVec());
const addNoise = (c, s) => c.map(v => v + (rnd() * 2 - 1) * s);

// deterministic corpus with labeled relevant sets
const docs = [];
const relevant = { fts_en: [], fts_zh: [], fts_code: [], vec_auth: [] };
for (let i = 0; i < N; i++) {
  const t = i % TOPICS.length;
  const topic = TOPICS[t];
  const text = `The ${topic} module handles ${topic} requests. ${ZH[topic]}服务处理第${i}条${ZH[topic]}请求. ${CODE[topic]} // unit ${i}`;
  docs.push({ id: `doc-${i}`, vectors: { vec: addNoise(topicCenters[t], 0.15) }, fields: { text, topic } });
  if (topic === 'payment') { relevant.fts_en.push(`doc-${i}`); relevant.fts_zh.push(`doc-${i}`); relevant.fts_code.push(`doc-${i}`); }
  if (topic === 'auth') relevant.vec_auth.push(`doc-${i}`);
}

const schema = new ZVecCollectionSchema({
  name: 'r2b',
  vectors: { name: 'vec', dataType: ZVecDataType.VECTOR_FP32, dimension: DIM, indexParams: { indexType: ZVecIndexType.HNSW, metricType: ZVecMetricType.COSINE, m: 16, efConstruction: 200 } },
  fields: [
    { name: 'text', dataType: ZVecDataType.STRING, indexParams: { indexType: ZVecIndexType.FTS, tokenizerName: 'jieba', filters: ['lowercase'], extraParams: '' } },
    { name: 'topic', dataType: ZVecDataType.STRING },
  ],
});
const coll = ZVecCreateAndOpen(COLL, schema);
for (let i = 0; i < N; i += 100) coll.upsertSync(docs.slice(i, i + 100));

const rssSamples = [process.memoryUsage().rss];
const vmhwm = () => { try { const s = fs.readFileSync('/proc/self/status', 'utf8'); const m = s.match(/VmHWM:\s+(\d+)\s+kB/); return m ? Number(m[1]) * 1024 : null; } catch { return null; } };

function runQuery(q) {
  const r = coll.querySync(q);
  return r; // array of {id, score, fields}
}
function metrics(ids, relSet, k) {
  const rel = new Set(relSet);
  const top = ids.slice(0, k);
  const hit = top.filter(id => rel.has(id)).length;
  const precision = k > 0 ? hit / k : 0;
  const recall = relSet.length > 0 ? hit / relSet.length : 0;
  let rank = 0;
  for (let i = 0; i < ids.length; i++) { if (rel.has(ids[i])) { rank = i + 1; break; } }
  return { precision, recall, firstRelRank: rank };
}
function timing(fn, n) {
  const out = [];
  for (let i = 0; i < n; i++) { const a = performance.now(); fn(); out.push(performance.now() - a); rssSamples.push(process.memoryUsage().rss); }
  const mean = out.reduce((s, x) => s + x, 0) / out.length;
  const std = Math.sqrt(out.reduce((s, x) => s + (x - mean) ** 2, 0) / out.length);
  return { mean, std, min: Math.min(...out), max: Math.max(...out) };
}

const K = [10, 20, 50];
const out = {};

// FTS English
{
  const q = () => runQuery({ fieldName: 'text', fts: { queryString: 'payment' }, topk: 50, params: { indexType: ZVecIndexType.FTS } }).map(d => d.id);
  const ids = q();
  out.fts_en = { relevant_total: relevant.fts_en.length, k: K.map(k => ({ k, ...metrics(ids, relevant.fts_en, k) })) };
  out.fts_en_latency = { cold: (() => { const a = performance.now(); q(); return performance.now() - a; })(), warm: timing(q, 5) };
}
// FTS Chinese (jieba)
{
  const q = () => runQuery({ fieldName: 'text', fts: { queryString: '网络' }, topk: 50, params: { indexType: ZVecIndexType.FTS } }).map(d => d.id);
  const ids = q();
  out.fts_zh = { relevant_total: relevant.fts_zh.length, k: K.map(k => ({ k, ...metrics(ids, relevant.fts_zh, k) })) };
  out.fts_zh_latency = { cold: (() => { const a = performance.now(); q(); return performance.now() - a; })(), warm: timing(q, 5) };
}
// FTS code token
{
  const q = () => runQuery({ fieldName: 'text', fts: { queryString: 'charge_card' }, topk: 50, params: { indexType: ZVecIndexType.FTS } }).map(d => d.id);
  const ids = q();
  out.fts_code = { relevant_total: relevant.fts_code.length, k: K.map(k => ({ k, ...metrics(ids, relevant.fts_code, k) })) };
  out.fts_code_latency = { cold: (() => { const a = performance.now(); q(); return performance.now() - a; })(), warm: timing(q, 5) };
}
// Vector route (synthetic topic cluster — MECHANISM TEST, NOT semantic)
{
  const q = () => coll.querySync({ fieldName: 'vec', vector: topicCenters[0], topk: 50, params: { indexType: ZVecIndexType.HNSW, ef: 200 } }).map(d => d.id);
  const ids = q();
  out.vector_topic = { note: 'synthetic clustered vectors; relevant=same-topic docs (200). Mechanism only, not real embedding semantics.', relevant_total: relevant.vec_auth.length, k: K.map(k => ({ k, ...metrics(ids, relevant.vec_auth, k) })) };
  out.vector_latency = { cold: (() => { const a = performance.now(); q(); return performance.now() - a; })(), warm: timing(q, 5) };
}

const sampledPeakRss = Math.max(...rssSamples);
const osPeakRss = vmhwm();
const indexBytes = Number(execSync(`du -sb ${JSON.stringify(COLL)}`).toString().split('\t')[0]);

out.meta = {
  corpus_docs: N, vector_dim: DIM,
  index_bytes: indexBytes,
  rss_sampled_peak_bytes: sampledPeakRss,
  rss_os_peak_bytes_VmHWM: osPeakRss,
  rss_note: 'sampled_peak = max of post-call process.memoryUsage().rss samples; os_peak = /proc/self/status VmHWM (kernel peak for this process). Single-process run only; not a fleet p95.',
  trials_per_query: 5,
  cold_def: 'first call immediately after open (page-in inclusive)',
  warm_def: 'subsequent calls (cache warm)',
  metric_denominators: 'precision@k = hits_in_topk / k ; recall@k = hits_in_topk / relevant_total ; firstRelRank = 1-based rank of first relevant hit',
  limitation: 'Lexical relevant set = docs containing the query term, so FTS precision@k=1.0 by construction. True SEMANTIC precision/recall needs human relevance judgments, which a synthetic corpus cannot supply. Vector route uses synthetic topic vectors; it proves nearest-neighbor retrieval returns same-cluster docs, NOT embedding semantic quality.',
  binding: 'npm @zvec/zvec@0.7.0 Node N-API addon (one host runtime of engine v0.7.0). Engine also has official Rust crate zvec-rust@0.7.0 (libzvec_c_api) per A7 f157eb4; route behavior identical across bindings.',
};
console.log(JSON.stringify(out, null, 2));
