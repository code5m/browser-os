// A8 M5-W18-R2 evidence: execute FTS / vector / hybrid routes on synthetic non-secret data
// using the @zvec/zvec native engine directly (no embedding model needed; we supply
// ground-truth synthetic vectors). Records: indexing time, index size, cold/warm
// latency, peak RSS, result quality (recall@k), tokenizer behavior.
import {
  ZVecInitialize,
  ZVecSetDefaultJiebaDictDir,
  ZVecCollectionSchema,
  ZVecCreateAndOpen,
  ZVecOpen,
  ZVecIndexType,
  ZVecDataType,
  ZVecMetricType,
} from '@zvec/zvec';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { execSync } from 'node:child_process';

const COLL = path.join(os.tmpdir(), `m5w18-a8-zvec-coll-${Date.now()}`);
fs.rmSync(COLL, { recursive: true, force: true });

const JIEBA = path.join('node_modules', '@zvec', 'bindings-linux-x64', 'jieba_dict');
ZVecSetDefaultJiebaDictDir(JIEBA);
ZVecInitialize({ logLevel: 3 }); // ERROR

const N = 1000;
const DIM = 256;
const TOPICS = ['auth', 'payment', 'search', 'cache', 'network'];
const ZH = { auth: '认证', payment: '支付', search: '检索', cache: '缓存', network: '网络' };

// deterministic PRNG
let seed = 123456789;
function rnd() { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; }
function randVec() { const v = new Array(DIM); for (let i = 0; i < DIM; i++) v[i] = rnd() * 2 - 1; return v; }
const topicCenters = TOPICS.map(() => randVec());
function addNoise(center, scale) { return center.map(c => c + (rnd() * 2 - 1) * scale); }

const schema = new ZVecCollectionSchema({
  name: 'evidence',
  vectors: { name: 'vec', dataType: ZVecDataType.VECTOR_FP32, dimension: DIM, indexParams: { indexType: ZVecIndexType.HNSW, metricType: ZVecMetricType.COSINE, m: 16, efConstruction: 200 } },
  fields: [
    { name: 'text', dataType: ZVecDataType.STRING, indexParams: { indexType: ZVecIndexType.FTS, tokenizerName: 'jieba', filters: ['lowercase'], extraParams: '' } },
    { name: 'topic', dataType: ZVecDataType.STRING },
  ],
});

const coll = ZVecCreateAndOpen(COLL, schema);
const docs = [];
for (let i = 0; i < N; i++) {
  const t = i % TOPICS.length;
  const topic = TOPICS[t];
  const en = `The ${topic} module handles ${topic} requests and ${topic} lifecycle for user ${i}.`;
  const zh = `${ZH[topic]}服务处理第${i}条${ZH[topic]}请求与${ZH[topic]}流程。`;
  docs.push({ id: `doc-${i}`, vectors: { vec: addNoise(topicCenters[t], 0.15) }, fields: { text: en + ' ' + zh, topic } });
}

// indexing time
const t0 = performance.now();
const BATCH = 100;
for (let i = 0; i < N; i += BATCH) coll.upsertSync(docs.slice(i, i + BATCH));
const tIndex = performance.now() - t0;

// index size on disk
const indexBytes = Number(execSync(`du -sb ${JSON.stringify(COLL)}`).toString().split('\t')[0]);

// peak RSS sampling helper
function rss() { return process.memoryUsage().rss; }
let peak = rss();

function timed(fn) { const a = performance.now(); const r = fn(); const b = performance.now(); peak = Math.max(peak, rss()); return { ms: b - a, r }; }

// VECTOR route: query = topic 0 center
const qVec = topicCenters[0];
const vCold = timed(() => coll.querySync({ fieldName: 'vec', vector: qVec, topk: 10, params: { indexType: ZVecIndexType.HNSW, ef: 200 } }));
const vWarm = timed(() => coll.querySync({ fieldName: 'vec', vector: qVec, topk: 10, params: { indexType: ZVecIndexType.HNSW, ef: 200 } }));
const vRecall = vWarm.r.filter(d => d.fields.topic === 'auth').length / 10;

// FTS route (English keyword)
const fCold = timed(() => coll.querySync({ fieldName: 'text', fts: { queryString: 'payment' }, topk: 10, params: { indexType: ZVecIndexType.FTS } }));
const fWarm = timed(() => coll.querySync({ fieldName: 'text', fts: { queryString: 'payment' }, topk: 10, params: { indexType: ZVecIndexType.FTS } }));
const fExpected = docs.filter(d => d.fields.topic === 'payment').length;
const fRecall = fWarm.r.length ? fWarm.r.filter(d => d.fields.topic === 'payment').length / fWarm.r.length : 0;

// FTS route (Chinese keyword via jieba)
const zCold = timed(() => coll.querySync({ fieldName: 'text', fts: { queryString: '网络' }, topk: 10, params: { indexType: ZVecIndexType.FTS } }));
const zRecall = zCold.r.filter(d => d.fields.topic === 'network').length / (zCold.r.length || 1);

// HYBRID route: vector + fts, RRF K=60 (engine default)
const hCold = timed(() => coll.multiQuerySync({
  queries: [
    { fieldName: 'vec', vector: qVec, numCandidates: 20, params: { indexType: ZVecIndexType.HNSW, ef: 200 } },
    { fieldName: 'text', fts: { queryString: 'auth' }, numCandidates: 20, params: { indexType: ZVecIndexType.FTS } },
  ],
  topk: 10,
  rerank: { type: 'rrf', rankConstant: 60 },
}));
const hWarm = timed(() => coll.multiQuerySync({
  queries: [
    { fieldName: 'vec', vector: qVec, numCandidates: 20, params: { indexType: ZVecIndexType.HNSW, ef: 200 } },
    { fieldName: 'text', fts: { queryString: 'auth' }, numCandidates: 20, params: { indexType: ZVecIndexType.FTS } },
  ],
  topk: 10,
  rerank: { type: 'rrf', rankConstant: 60 },
}));
const hScore0 = hWarm.r[0]?.score;

coll.closeSync();
const summary = {
  corpus_docs: N,
  vector_dim: DIM,
  indexing_ms: +tIndex.toFixed(1),
  index_bytes: indexBytes,
  peak_rss_bytes: peak,
  vector_route: { cold_ms: +vCold.ms.toFixed(2), warm_ms: +vWarm.ms.toFixed(2), recall_at10: +vRecall.toFixed(2) },
  fts_en_route: { cold_ms: +fCold.ms.toFixed(2), warm_ms: +fWarm.ms.toFixed(2), expected_matches: fExpected, recall_of_returned: +fRecall.toFixed(2) },
  fts_zh_route: { cold_ms: +zCold.ms.toFixed(2), recall_of_returned: +zRecall.toFixed(2) },
  hybrid_rrf_route: { cold_ms: +hCold.ms.toFixed(2), warm_ms: +hWarm.ms.toFixed(2), top_score: hScore0 },
};
console.log(JSON.stringify(summary, null, 2));
