# A8 · M5-W18-R · Lane Checkpoint

```
LANE=A8
STATUS=PASS
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd   (origin/master)
HEAD=<未提交的研究产物；W18-R 边界要求不 push，A0 集成时提交>
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src (upstream zvec-ai/zvec-grep @ 52653951b24617762f4ab0c71c34d594e5001617, Apache-2.0)
FILES=logs/research/M5-W18/A8-zvec-grep-retrieval.md, logs/research/M5-W18/A8-checkpoint.md, logs/research/M5-W18/A8-benchmark-ripgrep.mjs
SOURCE_MAP=src/engine/service/zvec-grep.ts (context/contextFromRg/selectAndRankContextItems/contextGlobalRrfScore/fileFreshnessStatus/requireEmbeddingModel), src/engine/service/lexical.ts (runRgSearch/buildRipgrepArgs/runCommand), src/cli/managed-rg.ts (parseManagedRgCommand/assertRootScopedPath), package.json (deps), docs/04-pipeline.md, docs/05-architecture.md, docs/07-embedding.md
CLASSIFICATION=COPY=0 ADAPT=4 (R1,R2,R4,R8-if-enabled) REIMPLEMENT_FROM_BEHAVIOR=2 (R3,R5) DEFER=2 (R6,R7) REJECT=1 (R8-default / R7-default-remote)
VERIFY=node logs/research/M5-W18/A8-benchmark-ripgrep.mjs -> managed-ripgrep 11-17ms/420 files (synthetic); rg/vector/hybrid 其余路线因禁装依赖/禁下模型仅源码级论证
CHECKPOINT=logs/research/M5-W18/A8-checkpoint.md
MERGE_NOTES=依赖 A1(产品基线 gaps)、A7(index/ingestion 架构)、A9(信任边界)；A0 据本建议决定是否 W19 先行 ADAPT managed-ripgrep 路线（低风险）
NEXT=待 A10 源移植台账 + A3 采纳裁决后，方可 COPY @zvec/zvec 核心做 FTS/vector；向量路线默认 local-only 或置于 W19 之外
```

## 一句话结论

zvec-grep 检索层有四条路线：**managed ripgrep（零索引/零模型/亚 20ms/UTF-8 安全）** 可在 W19 低风险 **ADAPT** 先行；**FTS-BM25 / vector / hybrid-RRF（K=60）** 受「禁装依赖/禁下模型」约束仅完成源码级逆向，**DEFER** 至 A10 台账 + A3 裁决后采纳；向量路线默认 **REJECT** 远端嵌入，保持 local-only；**unavailable-model 必须硬失败（非静默）** 的契约须复刻。

## 关键证据

- 路线与 RRF：`src/engine/service/zvec-grep.ts` — primary query → routes `[{fts},{vector}]` 默认 hybrid；`CONTEXT_GROUP_RRF_K=60`；`contextGlobalRrfScore = Σ 1/(60+rank)`。
- rg 包装：`src/engine/service/lexical.ts` — `spawn(rg)`, `--json -n -c --with-filename --color never`, 流式解析, `limit` 超界 `child.kill()`（rg_truncated）；多字节列偏移 `Buffer.byteLength`。
- rg 沙箱：`src/cli/managed-rg.ts` — 拒绝 `| > & ; < > ( )`/shell 扩展, `assertRootScopedPath` 防 `..` 逃逸。
- 模型缺失：`requireEmbeddingModel` 抛 `EMBEDDING_MODEL_REQUIRED`（硬失败）。
- 新鲜度：`fileFreshnessStatus` → `fresh`/`possibly_stale`（结果照常返回）。
- 真实基准（A8-benchmark-ripgrep.mjs）：420 文件 literal 11.9ms / 中文 11.5ms / 正则 16.6ms。

## 交付物（均在 logs/research/M5-W18/，未 push）

- `A8-zvec-grep-retrieval.md` — 完整研究报告（缺口/源码映射/路线/持久化/并发/安全/性能/依赖/失败模式/stale/多语言/分类与采纳建议/未决问题）。
- `A8-checkpoint.md` — 本 checkpoint。
- `A8-benchmark-ripgrep.mjs` — 可复现的 managed-ripgrep 基准脚本（无依赖）。

## 研究边界合规

- 未编辑 `src/`、`src-tauri/`、`scripts/`、权限/清单/锁文件等任何产品代码路径。
- 未安装依赖、未下载模型、未起 daemon/MCP、未连远端、未改动用户 Obsidian 库。
- 仅写 `logs/research/M5-W18/` 自有报告 + checkpoint；未 commit、未 push（W18-R 边界 + WORKSPACE_IDENTITY 主副本不污染约定）。
