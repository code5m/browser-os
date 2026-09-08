# A8 · M5-W18-R2 · Lane Checkpoint

```
LANE=A8
STATUS=PASS_WITH_DEBT
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd   (origin/master; 本分支已 rebase 至 d6127c4 = 最新 origin/master)
HEAD=<本 R2 提交；W18-R 边界要求不 push，A0 集成时拉取>
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src (upstream zvec-ai/zvec-grep @ 52653951, Apache-2.0)
SANDBOX=/tmp/m5-w18-a8-zvec (一次性；已 rm -rf 后重建；pinned @zvec/zvec@0.7.0)
FILES=logs/research/M5-W18/A8-zvec-grep-retrieval.md, logs/research/M5-W18/A8-checkpoint.md, logs/research/M5-W18/A8-benchmark-ripgrep.mjs, logs/research/M5-W18/A8-benchmark-routes.mjs
SOURCE_MAP=
  zvec-grep 编排层: src/engine/service/zvec-grep.ts (context/contextFromRg/selectAndRankContextItems/contextGlobalRrfScore=Σ1/(60+rank)/fileFreshnessStatus/requireEmbeddingModel@L900-919,调用@L800), src/engine/service/lexical.ts (runRgSearch/buildRipgrepArgs/runCommand), src/cli/managed-rg.ts (parseManagedRgCommand/assertRootScopedPath)
  @zvec/zvec 引擎层(Apache-2.0, 原生 N-API 插件): node_modules/@zvec/zvec/src/index.d.ts (ZVecIndexType.FTS=11 / HNSW/IVF/FLAT/DISKANN/RaBitQ; ZVecFtsIndexParams; ZVecQuery 可 vector+fts+filter; multiQuerySync 自带 RRF rankConstant=60)
  zvec-grep 依赖/模型: package.json (@zvec/zvec ^0.7.0, @vscode/ripgrep ^1.18.0, @huggingface/transformers ^3.8.1, web-tree-sitter ^0.20.8, node-llama-cpp 3.18.1 optional), docs/07-embedding.md (local/potion-code-16m-v2 等)
CLASSIFICATION=COPY=0 ADAPT=4 (R1,R2,R4,R8-if-enabled) REIMPLEMENT_FROM_BEHAVIOR=2 (R3,R5) DEFER=2 (R6,R7) REJECT=0
VERIFY=
  node logs/research/M5-W18/A8-benchmark-ripgrep.mjs -> managed-ripgrep 11-17ms/420 文件 (EXECUTED_SYNTHETIC_TEST, R1)
  node logs/research/M5-W18/A8-benchmark-routes.mjs -> FTS/vector/hybrid 引擎层真实执行: 1000 docs 索引151ms/6.57MB/峰值RSS~294MB, 向量 recall@10=1.0(cold2.83/warm0.73ms), FTS英 recall=1.0(~3ms), FTS中文(jieba) recall=1.0, hybrid RRF cold4.16/warm2.63ms top_score0.0283 (EXECUTED_SYNTHETIC_TEST, R2)
CORRECTIONS=R1 旧文两条被 A0 审计点名已显式 retract: (1) 产品许可非 Apache-2.0 实为 MulanPSL-2.0; (2) @zvec/zvec 原生 N-API 插件, 无法 Rust 直链, 仅可 sidecar/FFI/移植; COPY 由旧文过度乐观改为 COPY=0(整体只能 sidecar ADAPT)
MERGE_NOTES=依赖 A1(产品基线 gaps)、A3(检索落点/生命周期/采纳裁决)、A10(@zvec/zvec inbound Apache-2.0→MulanPSL-2.0 台账与 NOTICE/署名); A0 据本证据决定是否 W19 先行 ADAPT managed-ripgrep 路线(低风险)
NEXT=待 A10 源移植台账 + A3 采纳裁决后, 方可 sidecar ADAPT @zvec/zvec 做 FTS/vector; 向量路线默认 local-only 或置于 W19 之外
UNANSWERED=(1)产品检索首要落点(A3); (2)索引存储位置(A1/A10); (3)sidecar vs 轻量 BM25(A3/A10); (4)本地模型设备可用性(W19); (5)RSS~294MB 常驻策略(A3/A10); (6)Apache-2.0 inbound 许可义务(A10) -- 均为跨 lane, 不阻塞本 Lane 证据闭环
```

## 一句话结论

zvec-grep 检索层四条路线中，**managed ripgrep（R1）** 与 **FTS/BM25 / vector / hybrid-RRF（R6/R7 引擎层，R2 已真实执行）** 现均有证据支撑：rg 路线亚 20ms 零索引；引擎层千级文档索引 151ms、体积 6.6KB/doc、recall@10=1.0、RRF(K=60) 分数吻合。**关键修正（A0 审计 #7/#8）**：产品根许可是 **MulanPSL-2.0**（非 Apache-2.0），`@zvec/zvec@0.7.0` 是**原生 N-API 插件**（绑定 sha256 `a591609b…dc0dc`，36MB，仅 linux-x64），**无法从 Rust 直链**，只能以 Node sidecar 或 N-API FFI 形态 ADAPT，故 `COPY=0`。unavailable-model 硬失败契约已源码级确认（`zvec-grep.ts:900-919`）。采纳建议：W19 先行 ADAPT managed-ripgrep；FTS/vector 经 sidecar ADAPT，默认 local-only；远端嵌入默认拒绝。

## R2 证据类型分布

- `[CURRENT_PRODUCT]`：产品 0 检索、根 LICENSE=MulanPSL-2.0。
- `[REFERENCE_SOURCE]`：zvec-grep-src 编排/沙箱/unavailable-model、引擎层 `index.d.ts`、依赖与模型清单。
- `[OBSERVED_BEHAVIOR]`：安装并加载 `@zvec/zvec`、导出键列表、绑定 sha256/体积、引擎层 API 实测、RSS。
- `[EXECUTED_SYNTHETIC_TEST]`：`A8-benchmark-ripgrep.mjs` 与 `A8-benchmark-routes.mjs` 的真实数字。
- `[INFERENCE]`：分类、采纳路线、目标蓝图、跨 lane 未决项（均显式标注）。

## 交付物（均在 logs/research/M5-W18/，未 push）

- `A8-zvec-grep-retrieval.md` — R2 修订版研究报告（§0 纠正节 + §1–§9 + 附录 provenance）。
- `A8-checkpoint.md` — 本 checkpoint。
- `A8-benchmark-ripgrep.mjs` — R1 managed-ripgrep 基准（保留）。
- `A8-benchmark-routes.mjs` — R2 FTS/vector/hybrid 引擎层基准（新增，可复现）。

## 研究边界合规

- 未编辑 `src/`、`src-tauri/`、`scripts/`、权限/清单/锁文件等任何产品代码路径。
- 依赖仅装在一次性 `/tmp/m5-w18-a8-zvec`；未写入产品 `package.json`/锁文件；未下模型（受网络约束，留待 W19）。
- 未起 daemon/MCP；未连远端；未改动用户 Obsidian 库。
- 仅写 `logs/research/M5-W18/` 自有报告 + checkpoint；已 commit 至 `codex/m5-w18-a8`，未 push（W18-R 边界 + WORKSPACE_IDENTITY 主副本不污染约定）。
