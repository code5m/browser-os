# A8 · M5-W18-R2 · zvec-grep 检索层逆向与证据闭环（synthetic corpus）

> Lane A8（RESEARCH）· 按 `PARALLEL_COMMAND_BOARD.md` L1439 `M5-W18-R2 Evidence Closure Dispatch` 修订与补齐。
> 首轮（R1）草稿 `A8-zvec-grep-retrieval.md`（`b7a8615` 前形态）已保留；其被 A0 审计点名的结论在本文件 §0 纠正节**显式 retract/replace**，不静默改写。
> R2 证据契约：每条事实标注 `[CURRENT_PRODUCT]` / `[REFERENCE_SOURCE]` / `[OBSERVED_BEHAVIOR]` / `[OFFICIAL_DOC]` / `[EXECUTED_SYNTHETIC_TEST]` / `[INFERENCE]`，并附精确 path/symbol/line 或 command/result。

---

## 0. R2 纠正节（显式 retract R1 中被 A0 审计点名的陈述）

A0 审计（`logs/checkpoints/A0-M5-W18-R1-audit-20260908.md`）对 A8 的两条裁定：

- **(#7) 产品许可证陈述错误**：R1 旧文称「与产品（Apache-2.0）许可兼容」。
  **RETRACT**：产品根 `LICENSE` 实际上是 **MulanPSL-2.0（木兰宽松许可证 第2版）** `[CURRENT_PRODUCT: /home/.../mvp-browser-os-v3/LICENSE 首行「木兰宽松许可证，第2版」]`。
  **REPLACE**：`@zvec/zvec` 与 `zvec-grep` 为 Apache-2.0；把 Apache-2.0 组件引入 MulanPSL-2.0 产品属** inbound 兼容义务**，须经 Lane A10 的 provenance/NOTICE/修改声明台账，本 Lane 不自行判定合法性，仅记录事实。
- **(#8) 原生绑定可行性未证明**：R1 旧文未判定 `@zvec/zvec` 能否从 Rust 直链、能否仅经 Node/N-API、或是否共享稳定磁盘格式。
  **RETRACT/REPLACE**：已证明 `@zvec/zvec@0.7.0` 是**原生 N-API 插件**（`zvec_node_binding.node`，36,147,600 字节，仅 linux-x64 预编译），Node 端经由 `node-addon-api` 暴露 C 风格 API（`ZVecInitialize/ZVecCreateAndOpen/ZVecOpen/...`，见 `index.d.ts`）。
  **结论**：从本 npm 包**无法**「Rust 直接链接」；可移植路径只有 (a) Node sidecar 加载该插件，(b) 经 N-API FFI（napi-rs/nodejs-sys）在 Rust 侧调用，或 (c) 移植 alibaba/zvec 引擎源码（独立仓库，不在 npm 内）。详见 §4 / §9。

**保留未变项**：R1 的 managed-ripgrep 合成基准（`A8-benchmark-ripgrep.mjs`，420 文件 11–17ms）仍然有效且被 R2 复用；zg 沙箱/limit/stale/unavailable-model 源码映射仍准确。

---

## 1. 当前产品缺口 `[CURRENT_PRODUCT]`

在 `mvp-browser-os-v3/src` 全量检索 `ripgrep|bm25|tantivy|@zvec|semantic search|hybrid search|vector search`：**0 命中**；`*Search*.vue`：**0 文件** `[EXECUTED: grep -rn 上述模式 src/ → 0]`。
即产品当前**完全没有** BM25/向量/ripgrep 融合检索；现有检索为结构化/字面匹配（图谱、文件定位、命令面板）。任何 in-app 语义/全文检索均属绿地。
> A1 基线报告拥有权威 gap 清单；本 Lane 仅就「检索路线」维度补全。

---

## 2. 上游源码映射（exact files/symbols）`[REFERENCE_SOURCE]`

参考源：`/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src`（upstream zvec-ai/zvec-grep @ `52653951`，Apache-2.0）。

| 关注点 | 文件 | 关键符号 / 事实 |
|---|---|---|
| 检索编排/路由 | `src/engine/service/zvec-grep.ts` | `context`, `contextFromRg`, `contextFromWorkspaceIndex`, `selectAndRankContextItems`, `contextGlobalRrfScore`（`CONTEXT_GROUP_RRF_K=60`, `Σ 1/(60+rank)`） |
| 默认 hybrid 路由 | 同上 | `contextGroups` → routes `[{fts},{vector}]`；`normalizeContextRoutes` 仅允许 `fts`/`vector` |
| Managed ripgrep | `src/engine/service/lexical.ts` | `runRgSearch`, `buildRipgrepArgs`, `runCommand`（流式 + limit-kill）, `parseRipgrepJsonLine` |
| rg 命令沙箱 | `src/cli/managed-rg.ts` | `parseManagedRgCommand`, `scanManagedRgCommand`（拒绝 `| > & ; < > ( )`、shell 扩展、`assertRootScopedPath` 防 `..` 逃逸） |
| 新鲜度/陈旧 | `src/engine/service/zvec-grep.ts` | `fileFreshnessStatus`（mtime + contentHash → `fresh`/`possibly_stale`，结果照常返回） |
| 模型不可用硬失败 | 同上 | `requireEmbeddingModel`（L900-919）无模型抛 `ZVEC_GREP.ENGINE.SERVICE.EMBEDDING_MODEL_REQUIRED`；于 L800 在 vector 路径被调用 |
| 配置/依赖 | `package.json` | `@zvec/zvec ^0.7.0`, `@vscode/ripgrep ^1.18.0`, `@huggingface/transformers ^3.8.1`, `web-tree-sitter ^0.20.8`, `node-llama-cpp 3.18.1`(optional) |
| 本地模型清单 | `docs/07-embedding.md` | `local/potion-code-16m-v2`(Model2Vec FP16, 256-d, 1024-tok)、`local/all-minilm-l6-v2`(ONNX Q4, 384-d) 等；全部 cosine |

### 2.1 关键修正：`@zvec/zvec` 引擎本身即含 FTS + vector + hybrid `[REFERENCE_SOURCE + OBSERVED_BEHAVIOR]`

R1 旧文把 FTS/BM25 与 vector/hybrid 都挂在 zvec-grep 层。**实测 `@zvec/zvec@0.7.0` 引擎（C-API 包）原生支持** `[REFERENCE_SOURCE: node_modules/@zvec/zvec/src/index.d.ts]`：
- `ZVecIndexType.FTS = 11` + `ZVecFtsIndexParams`（tokenizer `standard`/`ngram`/`jieba`/`whitespace`，filters `lowercase`/`ascii_folding`/`stemmer`）；
- 向量索引 `HNSW`/`IVF`/`FLAT`/`DISKANN`/`HNSW_RABITQ`/`IVF_RABITQ`（`ZVecMetricType.COSINE` 等）；
- **单查询混合**：`ZVecQuery` 可同时给 `vector` + `fts` + `filter`；
- **多查询 RRF 融合**：`multiQuerySync` 自带 `rerank:{type:'rrf', rankConstant:60}`（默认 K=60），与 zvec-grep 的 `CONTEXT_GROUP_RRF_K=60` 一致。

即 FTS/BM25、vector、hybrid-RRF **三条路线在引擎层即可直接执行**，无需 zvec-grep 的 Node 编排层，也**无需嵌入模型**（引擎只接受已算好的向量）。这使得 R2「执行缺失路线证据」可在引擎层用合成向量完成（见 §3）。

---

## 3. R2 执行化证据：FTS / vector / hybrid 路线（synthetic 非机密语料）`[EXECUTED_SYNTHETIC_TEST]`

- 沙箱：`/tmp/m5-w18-a8-zvec`（一次性，已 `rm -rf`），pinned `@zvec/zvec@0.7.0`。
- 脚本：`logs/research/M5-W18/A8-benchmark-routes.mjs`（Node 内置 API，仅依赖 `@zvec/zvec@0.7.0`）；复现：在含 `@zvec/zvec@0.7.0` 的目录（如一次性沙箱 `/tmp/m5-w18-a8-zvec` 经 `npm i @zvec/zvec@0.7.0 --install-scripts` 后）运行 `node A8-benchmark-routes.mjs`。本 Lane 已在该沙箱执行并采集上表数字。
- 语料：1000 个合成文档，每文档含 1 个 STRING 文本字段（英文 + 中文 jieba 关键词，分 5 主题 `auth/payment/search/cache/network`）+ 1 个 256 维合成稠密向量（按主题中心 + 噪声聚类，作为向量路线真值）+ 1 个 STRING `topic` 字段。索引：文本字段建 `FTS(jieba)`，向量字段建 `HNSW(COSINE)`。
- 评估：向量路线用主题中心作查询向量；FTS 路线用主题关键词；hybrid 用 `[vector, fts]` 双子查询 + RRF K=60。recall@10 = 返回结果中属正确主题的比例。

**实测结果（单次运行，Node v26.7.0 / linux-x64）：**

| 指标 | 值 |
|---|---|
| 索引耗时（1000 docs 插入+建双索引） | **151.4 ms** |
| 索引磁盘体积（含 HNSW + FTS + 原始向量） | **6,571,302 B ≈ 6.57 MB** |
| 进程峰值 RSS（含 Node + jieba 词典 + 引擎） | **308,678,656 B ≈ 294 MB** |
| 向量路线 cold / warm 延迟 | 2.83 ms / 0.73 ms，recall@10 = **1.00** |
| FTS 英文路线 cold / warm | 2.56 ms / 3.18 ms，recall（返回集内）= **1.00**（200 个 payment 文档中取样） |
| FTS 中文路线（jieba `网络`） | 3.09 ms，recall（返回集内）= **1.00** |
| hybrid RRF cold / warm | 4.16 ms / 2.63 ms，top_score = **0.0283**（`Σ 1/(60+rank)` 与 K=60 一致） |

结论 `[EXECUTED_SYNTHETIC_TEST]`：
- 三条路线在引擎层**全部真实可执行且正确**（recall=1.0，RRF 分数与 K=60 算法吻合）。
- 延迟量级：千级文档子毫秒~数毫秒；索引体积 ~6.6 KB/文档（256-d 向量主导）；峰值 RSS ~294 MB（主要来自引擎 + jieba 词典常驻，与其余路线共享，非每查询线性增长）。
- 中文经 jieba 分词命中正确，证明 FTS 路线在中文代码注释/文档场景可用。

**unavailable-model 行为** `[REFERENCE_SOURCE]`：引擎层 `@zvec/zvec` 的导出键为 `ZVecInitialize/ZVecCreateAndOpen/ZVecOpen/ZVecCollectionSchema/ZVecDataType/ZVecIndexType/ZVecMetricType/.../isZVecError` `[OBSERVED_BEHAVIOR: import('@zvec/zvec') 键列表]`，**无任何 embedding/model 符号**——引擎只接收向量，模型概念仅存在于 zvec-grep 编排层。zvec-grep 的 vector 路线在缺失模型时由 `requireEmbeddingModel`（`src/engine/service/zvec-grep.ts:900-919`，调用点 :800）抛 `ZVEC_GREP.ENGINE.SERVICE.EMBEDDING_MODEL_REQUIRED`（带 hint：传 `--embedding` / 设 `ZVEC_GREP_EMBEDDING` / 配默认；examples：`local/potion-code-16m-v2`, `qwen/text-embedding-v4`）。即：**模型不可用时 vector 路线硬失败（非静默）**，错误码机器可识别。产品语义路线必须复刻此契约。

> 说明：unavailable-model 的**端到端**执行需构建完整 zvec-grep CLI + 下载模型，超出引擎层路线基准范围；本 Lane 以 `[REFERENCE_SOURCE]`（精确 file:line）提供契约证据，并以上述引擎层「无模型概念、仅接收向量」的 `[OBSERVED_BEHAVIOR]` 说明模型需求纯属编排层嵌入步骤。该契约独立于产品代码，不强制端到端重跑。

**stale-index 行为** `[REFERENCE_SOURCE]`：引擎层无 mtime/新鲜度概念（属于 zvec-grep 更高层的 `fileFreshnessStatus`）；因此 stale 检测是 zvec-grep 索引协调层职责，本 Lane 在 R1 已记录其「返回即标注 fresh/possibly_stale、不阻断」语义。

---

## 4. 依赖 / 许可 / 原生足迹（R2 修正后事实）`[OBSERVED_BEHAVIOR + CURRENT_PRODUCT + REFERENCE_SOURCE]`

- 产品根 `LICENSE` = **MulanPSL-2.0** `[CURRENT_PRODUCT]`。
- `@zvec/zvec@0.7.0`：Apache-2.0，作者 Alibaba，仓库 `github.com/alibaba/zvec`（npm 包 `github.com/zvec-ai/zvec-node`）`[REFERENCE_SOURCE: node_modules/@zvec/zvec/package.json]`。
- 原生绑定：`@zvec/bindings-linux-x64@0.7.0` 含 `zvec_node_binding.node` **36,147,600 B**，sha256 `a591609b520c9ef5b880d5bdc56ed20651173af57c5c563c7491abf5380dc0dc`；另有 darwin-arm64 / linux-arm64 / musl / win32-x64 等平台变体（npm 锁定哈希见 §附录）`[OBSERVED_BEHAVIOR: sha256sum + stat]`。
- **Rust 直链不可行**：npm 包仅交付 `.node` N-API 插件 + JS 包装，无 Rust crate、无静态库、无 C 头文件。要在 Rust/Tauri 产品内复用，唯一稳妥路径是 **Node sidecar**（打包 `@zvec/zvec` + 平台 binding，由 Rust command 经 stdio/IPC 调用）或 N-API FFI 封装；直接 `cargo` 链接不存在 `[INFERENCE: 基于 package 内容观察]`。
- 其余依赖：`@vscode/ripgrep` MIT、`@huggingface/transformers`+`tokenizers` Apache-2.0、`web-tree-sitter`+`tree-sitter-wasms` MIT、`zod`/`jsonc-parser` MIT/ISC —— 均 Apache/MIT/ISC，但与 MulanPSL-2.0 产品的 inbound 义务须由 A10 台账裁定 `[INFERENCE]`。

---

## 5. 路线分类与采纳建议（R2 修订）`[INFERENCE]`（基于 §2–§4 证据）

> 分类域：`COPY`（合法且技术可后续移植）/ `ADAPT`（改造后采用）/ `REIMPLEMENT_FROM_BEHAVIOR`（按行为重实现）/ `DEFER`（暂缓至 W19+）/ `REJECT`（当前拒绝）。R2 契约 #6：COPY 须给出上游文件+符号+传递依赖+外部 crate/包+测试+署名+目标落点，否则降级为 ADAPT/REIMPLEMENT。

| # | 路线/特性 | R2 分类 | 修订理由与 product 落点（destination-first） |
|---|---|---|---|
| R1 | **Managed ripgrep 包装**（流式解析 + limit-kill + 路径/根作用域沙箱） | **ADAPT** | 零索引零模型亚 20ms；Tauri Rust command 包 `rg`（系统或 `@vscode/ripgrep` 随包），照搬 `scanManagedRgCommand` 的 shell 操作符拒绝 + `assertRootScopedPath`。目标文件：`src-tauri/src/semantic_search.rs`（新建）或并入 `src-tauri/src/lexical_search.rs`；前端 `src/components/search/` + `src/stores/useSearchStore.ts`。 |
| R2 | **Filters / limits**（glob/type/ignore/depth/filesize/limit/context） | **ADAPT** | 与产品文件搜索过滤一一对应，直接映射。 |
| R3 | **RRF 融合（K=60）** | **REIMPLEMENT_FROM_BEHAVIOR** | 算法极小且众所周知；product 不应为融合依赖 `@zvec/zvec`。复刻 `matchedBy`/`fresh`/ranking-metadata schema。 |
| R4 | **Stale-index 新鲜度**（mtime+contentHash→fresh/possibly_stale，返回即标注） | **ADAPT** | 廉价友好，product 索引路线直接复用。 |
| R5 | **Unavailable-model 硬失败契约**（显式 `EMBEDDING_MODEL_REQUIRED`，非静默） | **REIMPLEMENT_FROM_BEHAVIOR** | 必须在 product 语义路线复刻，避免空结果误导。 |
| R6 | **Indexed FTS/BM25（引擎层 `ZVecFtsIndexParams`）** | **DEFER→ADAPT via sidecar** | 引擎层已证明可行（§3）；但引入需 Node sidecar 运行 `@zvec/zvec` + 平台 binding。目标：Rust command 经 IPC 调 sidecar 建/查 `.zvec` 索引。须经 A10 台账 + A3 采纳裁决。 |
| R7 | **Vector/semantic 路线（HNSW 等）** | **DEFER→默认 local-only / 否则 REJECT** | 需嵌入推理运行时（transformers.js WASM / node-llama-cpp GGUF）与 `@zvec/zvec` 引擎。原生足迹大、隐私面宽（远端发射）。除非 local-only 小模型，否则默认拒绝；远程嵌入需显式工作区签名授权。 |
| R8 | **Remote-embedding 授权模型**（签名 grant + loopback-only） | **ADAPT（若启用）/ REJECT（默认）** | 强契约；默认保持 local-only。 |

> **COPY=0**：本 Lane 审慎地**不将任何 `@zvec/zvec` 内部单元标为 COPY**——因 R2 契约 #6 要求逐函数 provenance + 依赖闭包 + 测试 + 署名 + 目标落点，而该引擎是闭源式 N-API 预编译插件，无法逐函数移植，只能整体以 sidecar 形态 ADAPT。这**修正了 R1 旧文把 `@zvec/zvec` 核心标为可 COPY 的过度乐观表述**。

### 采纳路线（route-selection recommendation）
1. **立即（低风险，可 W19 先行）**：ADOPT **managed ripgrep（R1+R2）**——零依赖零索引亚 20ms UTF-8 安全，关闭产品最大缺口；照搬沙箱与 limit 包装。确定性最高、回退最干净。
2. **后续（W19+，受 A10 台账约束）**：叠加 **RRF 融合（R3）+ FTS/BM25（R6 via sidecar）**，可选 local-only 小模型向量（R7）；融合层在 vector 路线模型缺失时优雅降级到 FTS→rg，永不静默（R5）。
3. **默认拒绝**：远端嵌入（R8）——保持 local-only。

---

## 6. 目标蓝图（R2 契约 #5：exact destination files/symbols）`[INFERENCE]`（采纳面待 A3 裁决）

**仅 R1（managed ripgrep）的确定落点（无引擎依赖）：**
- 新建 `src-tauri/src/lexical_search.rs`：Rust command `lexical_search(args)` → spawn `rg`（`--json -n -c --with-filename --color never` + 过滤），流式解析、超 `limit` 即 `child.kill()`（对应 rg_truncated）；复用既有 `check_invocation_source` + ACL。
- 复用 zvec-grep `managed-rg.ts` 的 shell 操作符拒绝 + `assertRootScopedPath` 逻辑（ADAPT 为 Rust）。
- 前端：`src/components/search/LexicalSearch.vue` + `src/stores/useSearchStore.ts` + `src/bridge.ts`/`src/types.ts` 暴露 + `default-commands.toml` ACL（插末条 `list_artifact_images` 前）。
- 门禁：`scripts/check-command-set-consistency.py` 三方比对 + 新增 `scripts/check-lexical-search-logic.mjs`（映射 `A8-benchmark-ripgrep.mjs` 断言：UTF-8 中文命中、limit 截断、路径逃逸拒绝）。
- 依赖闭包：系统 `rg`（或 `@vscode/ripgrep` 随包，MIT）；**无新增 Rust crate**。
- 生命周期：每次查询独立 spawn，无常驻 daemon；无索引文件。
- 稳定错误：`PATH_TRAVERSAL_REJECTED`（根逃逸）、`COMMAND_INJECTION_REJECTED`（shell 操作符）、`RG_TRUNCATED`（超限）。
- 迁移/回滚：纯新增文件 + 新 ACL 行；回滚 = 删除 command + ACL 行，不影响既有检索。
- 硬停：不接入任何 embedding/远端；不写 `.zvec` 索引；不改其他 lane 文件。

**R6/R7（FTS/vector，sidecar 形态，W19 外/受 A10 裁决）：**
- 目标：`src-tauri/src/semantic_index.rs` + sidecar `tools/zvec-sidecar/`（Node，依赖 `@zvec/zvec@0.7.0` + 平台 binding，打包进安装产物），Rust command 经 stdio/IPC 调 sidecar 的建索引/查询 API。
- 数据流向：前端 query → Rust command → sidecar（`ZVecCreateAndOpen`/`querySync`/`multiQuerySync`）→ 返回结构化命中（含 `matchedBy`/`fresh`/`score`）。
- 容量：千级文档 ~6.6KB/doc 索引；峰值 RSS ~294MB（sidecar 常驻或按需起停，需由 A3 定生命周期）。
- 测试：复用 `A8-benchmark-routes.mjs` 作为回归基准（recall@10、RRF 分数、中文 jieba 命中）。
- 硬停：远端嵌入默认关闭；模型缺失须硬失败（R5）；不改产品许可证。

---

## 7. 测试复用 `[REFERENCE_SOURCE]`
- `zvec-grep-src/test/unit/search.test.mjs`、`core.test.mjs`、`rg-cli.test.mjs`、`cli-format.test.mjs`：路线/RRF/limit/ranking 的 synthetic fixture 与期望，可作 product 单测形态参考（不复制依赖，仅借鉴断言结构）。
- `A8-benchmark-ripgrep.mjs`（R1，rg 路线，420 文件 11–17ms）与 `A8-benchmark-routes.mjs`（R2，引擎层 FTS/vector/hybrid）可作回归基准。

---

## 8. 未决问题（unresolved → 交 A0 / 跨 lane）`[INFERENCE]`
1. 产品检索**首要落点**（全局命令面板？文件内容？知识库？浏览器历史？）——决定 R1 集成面。**交 A3**。
2. FTS/vector 的**索引存储位置**（`.zvec-grep` 约定 vs product 自有 store）——需 A1/A10 对齐。
3. 是否采用 sidecar（ADAPT `@zvec/zvec`）还是另选轻量 BM25 库——待 A3 采纳裁决与 A10 台账。
4. 向量路线默认模型与设备在 product 目标平台的可用性——W19 真实基准（本地模型下载受网络约束，本沙箱未下载）。
5. 原生 RSS ~294MB 的常驻/按需策略与打包体积影响——交 A3/A10 评估。
6. **Apache-2.0 组件 inbound 进 MulanPSL-2.0 产品的 NOTICE/署名/修改声明义务**——交 A10 台账裁定，本 Lane 仅记录事实。

---

## 9. 诚实边界声明（HONEST BOUNDARY）`[EXECUTED_SYNTHETIC_TEST + REFERENCE_SOURCE]`
- managed-ripgrep 路线已**真实执行**基准（系统 `rg 14.1.0`）：420 文件 11–17ms，可复现。
- FTS / vector / hybrid-RRF 路线已**真实执行**基准（pinned `@zvec/zvec@0.7.0` 引擎，合成向量真值）：千级文档索引 151ms / 6.6MB / recall=1.0 / 子毫秒~数毫秒，可复现（`A8-benchmark-routes.mjs`）。
- 已**实测** `@zvec/zvec` 为原生 N-API 插件、记录绑定 sha256/体积、确认无 Rust 直链可能（修正 A0 #8）。
- 已**修正**产品许可证为 MulanPSL-2.0（修正 A0 #7）。
- 未端到端执行 zvec-grep CLI 的 unavailable-model 错误串（以 `[REFERENCE_SOURCE]` file:line 提供契约证据，见 §3）。
- 未下载任何嵌入模型（R2 授权但受网络约束未执行；本地模型基准留待 W19）。
- 未修改任何产品代码、未安装进产品依赖、未 push（符合 W18-R 研究边界）。

---

## 附录：pinned 依赖完整性（provenance）`[OBSERVED_BEHAVIOR]`
来自 `/tmp/m5-w18-a8-zvec/package-lock.json` 与 npm cache：
- `@zvec/zvec@0.7.0` sha512-`MT/M1CMnQ0k1w/Qh8iiCl8vXawonN0QwOr5dVq+92SMa6hMArHBVs`(p)
- `@zvec/bindings-linux-x64@0.7.0` sha512-`L+N/J5vPm1RHgvT6yNpc5nSEk0j8PcU0wv7y4P5sfKEAVvviQserI`(p)
- `@zvec/bindings-linux-x64` 内 `zvec_node_binding.node` sha256 `a591609b520c9ef5b880d5bdc56ed20651173af57c5c563c7491abf5380dc0dc`（36,147,600 B）
- `detect-libc@2.1.2` sha512-`Btj2BOOO83o3WyH59e8MgXsxEQVcarkUOpEYrubB0ur`(p)
- 安装脚本：`@zvec/zvec` 的 `scripts/install.js`（经 `npm install-scripts approve` 运行，产出本机绑定）已执行。
