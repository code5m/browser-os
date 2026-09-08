# A8 · M5-W18-R · zvec-grep 检索层逆向与基准（synthetic corpus）

> Lane A8（RESEARCH）· 按 M5-W18-R Research and Replication Blueprint Dispatch（PARALLEL_COMMAND_BOARD.md L1350-1438）。
> 范围：逆向工程 + 基准 zvec-grep 的**检索层**（managed ripgrep / FTS-BM25 / vector / hybrid-RRF、filters、limits、ranking metadata、stale-index、multilingual/code、latency/memory/index-size、unavailable-model failure），产出**基于证据的路线选择与采纳建议**。
> 研究边界（dispatch L1378-1385）：仅读源码/配置、仅跑 synthetic 非机密 fixture 的只读基准；**不装产品依赖、不下模型、不起 daemon/MCP、不连远端**。本报告严格遵守。

---

## LANE 元信息（dispatch L1414-1429 输出模板）

```
LANE=A8
STATUS=PASS
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd   (origin/master)
HEAD=<未提交的研究产物；W18-R 边界要求不 push，A0 集成时提交>
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src (upstream zvec-ai/zvec-grep @ 52653951b24617762f4ab0c71c34d594e5001617, Apache-2.0)
FILES=logs/research/M5-W18/A8-zvec-grep-retrieval.md, logs/research/M5-W18/A8-checkpoint.md, logs/research/M5-W18/A8-benchmark-ripgrep.mjs
SOURCE_MAP=见下文「上游源码映射」
CLASSIFICATION=COPY=0 ADAPT=4 REIMPLEMENT_FROM_BEHAVIOR=2 DEFER=2 REJECT=1（详见「路线分类与采纳建议」）
VERIFY=node logs/research/M5-W18/A8-benchmark-ripgrep.mjs -> managed-ripgrep 路线 11-17ms/420 文件（见「性能/容量证据」）；ripgrep/vector/hybrid 其余路线因禁装依赖/禁下模型仅源码级论证
CHECKPOINT=logs/research/M5-W18/A8-checkpoint.md
MERGE_NOTES=依赖 A1（产品基线 gaps）、A7（index/ingestion 架构）、A9（信任边界）；A0 据本建议决定是否在 W19 采纳 managed-ripgrep 路线（ADAPT，低风险可先行）
NEXT=待 A10 源移植台账 + A3 采纳裁决后，方可 COPY @zvec/zvec 核心做 FTS/vector；向量路线默认保持 local-only 或在 W19 之外
```

---

## 1. 当前产品缺口（current-product gap）

在 `mvp-browser-os-v3/src` 全量检索 `ripgrep|bm25|tantivy|@zvec|semantic search|hybrid search|vector search`：**0 命中**；`*Search*.vue`：**0 文件**。
即产品当前**完全没有** BM25 / 向量 / ripgrep 融合检索；现有检索为 (a) 知识图谱结构化查询、(b) 系统/文件定位、(c) 命令面板——均为结构化或字面匹配，**无跨工作区语义/全文检索能力**。任何未来 in-app 检索均属绿地（greenfield）。

> A1 基线报告拥有权威 gap 清单；本 Lane 仅就「检索路线」维度补全，不与 A1 重复。

---

## 2. 上游源码映射（exact files/symbols）

| 关注点 | 文件 | 关键符号 |
|---|---|---|
| 检索编排/路由 | `src/engine/service/zvec-grep.ts` | `ZvecGrepService.context`, `contextFromWorkspaceIndex`, `contextFromRg`, `contextFromOpenWorkspaceIndex`, `selectAndRankContextItems`, `contextGlobalRrfScore` |
| 路由分组 | 同上 | `contextGroups`（primary query → routes `[{fts},{vector}]`）、`normalizeContextRoutes`（仅允许 `fts`/`vector`）|
| FTS/BM25 + vector + RRF 引擎 | **外部依赖 `@zvec/zvec`（Apache-2.0）** | 本快照不含其源码（node_module）；经 `WorkspaceIndex.searchPlan({routes,limit,...})` 调用 |
| Managed ripgrep | `src/engine/service/lexical.ts` | `runRgSearch`, `buildRipgrepArgs`, `runRipgrep`, `runCommand`（流式 + limit-kill）, `parseRipgrepJsonLine` |
| rg 命令沙箱化 | `src/cli/managed-rg.ts` | `parseManagedRgCommand`, `scanManagedRgCommand`（拒绝 `| > & ; < > ( )`、拒绝 shell 扩展、根目录作用域校验 `assertRootScopedPaths`）|
| 新鲜度/陈旧 | `src/engine/service/zvec-grep.ts` | `fileFreshnessStatus`（mtime + contentHash → `fresh`/`possibly_stale`）|
| 模型不可用失败 | 同上 | `embeddingModelForSearch`（无 vector 路由即返 `undefined`）、`requireEmbeddingModel` → 抛 `EMBEDDING_MODEL_REQUIRED` |
| 嵌入模型/远端授权 | `src/engine/service/zvec-grep.ts`, `src/authorization/operation.ts`, `src/authorization/store.ts` | `createServiceEmbeddingModel`, `remoteEmbeddingAuthorizationGuard`, `RemoteEmbeddingAuthorizationStore` |
| 配置/依赖 | `package.json` | `@zvec/zvec ^0.7.0`, `@vscode/ripgrep ^1.18.0`, `@huggingface/transformers ^3.8.1`, `web-tree-sitter ^0.20.8`, `tree-sitter-wasms`, `node-llama-cpp`(optional) |
| 文档（行为契约） | `docs/04-pipeline.md`, `docs/05-architecture.md`, `docs/07-embedding.md` | 路线表、freshness、模型表、远端授权流程 |

---

## 3. 检索路线与数据/控制流

引擎对外暴露两条互补路径（架构文档）：

| 路径 | 最佳场景 | 数据源 | 是否需要索引/模型 |
|---|---|---|---|
| **Managed ripgrep** (`--rg`) | 已知文本/符号/路径/正则 | 直接扫描工作区文件 | 否（零索引） |
| **Indexed**（FTS/BM25 + vector + RRF） | 意图、相关概念、排名关键词 | `<root>/.zvec-grep/` 索引 | 是（需 embedding 模型建向量）|

控制流（`context()`）：
1. 若 `options.rg` → `contextFromRg` → `runRgSearch`（spawn rg）→ 结构富化 → `dedupeAndRerankContextItems`。覆盖 `rg_exhaustive` 或 `rg_truncated`。
2. 否则找最近 `<root>/.zvec-grep/` 索引（缺失 → 抛 `WORKSPACE_INDEX_NOT_FOUND`；disabled → 抛 `WORKSPACE_INDEX_DISABLED`）。
3. `contextFromOpenWorkspaceIndex`：每个 query group 调 `workspaceIndex.searchPlan({routes,limit,...})`，再把各组命中经 `selectAndRankContextItems` 融合排序。

**路由选择（关键）**：
- 默认 primary query → `contextGroups` 生成 routes `[{fts},{vector}]`：即**默认 hybrid**，同时跑 FTS + vector，用 **RRF（K=60）** 融合（`CONTEXT_GROUP_RRF_K=60`；`contextGlobalRrfScore = Σ 1/(60+rank)`）。
- `--fts` / `--vector` / `--rg` 显式覆盖单一路由；`--fuse` 将所有 group 合并为一个。
- `normalizeContextRoutes` 只允许 `fts`/`vector`（hybrid 由默认行为产生，非独立 mode）。

**Ranking metadata（每个 item）**：`rank`, `score`, `status`(fresh/possibly_stale), `matchedBy`∈{fts,vector,fts+vector}, `queryGroups[]`(每 group 的 id/query/role/rank/matchedBy), `entityId`, `trace`, `metadata`(symbol type 等)。优先级选择：coverage group 至多 `DEFAULT_CONTEXT_PRIORITY_LIMIT=6` → global fill → unprioritized。

---

## 4. 持久化格式

- 索引根：`<root>/.zvec-grep/`（`manifest.json` + `files.zvec` + `index.zvec`）。`.git` 与 `.zvec-grep` 始终排除。
- `manifest.json`：manifestVersion、rootPaths（含 include/exclude/glob/type/size/depth 过滤）、indexPolicy(`enabled`/`disabled`/`undecided`)、embedding schema（provider/model/**dimension**/**metric**）、embeddingRuntime（远端 provider 时含 apiKey 指纹）。
- 全局：`~/.zvec-grep/`（配置、daemon 状态、模型缓存 `~/.zvec-grep/models`）。
- 索引增量：基于 mtime + contentHash 协调（`fileFreshnessStatus`）；改模型/端点需 `--rebuild`（向量空间不可跨模型兼容，即便维度相同）。

---

## 5. 并发 / 生命周期

- `WorkspaceIndex` 分 `read`/`write` 模式；每次操作经 `acquireHomeLock(home,"read"|"write")` 文件锁。
- embedding 模型生命周期：`recoveredEmbeddingModels` 缓存（上限 `MAX_RECOVERED_EMBEDDING_MODELS=4`），空闲（`activeEmbeddingModelOperations===0`）时 `dispose` 退役模型；`close()` 释放全部。
- 自动刷新：`refreshWorkspaceIndexForContext` 在 `autoUpdate!==false` 时检测 `indexStatusNeedsRefresh` 并经 `withHomeWriteLock` 增量重建（需 daemon 写许可）。
- rg 包装层：`spawn` rg，流式解析 JSON；设 `limit` 时超出即 `truncated=true` 并 `child.kill()`（穷举但有界）。

---

## 6. 安全 / 隐私

- **远端嵌入是唯一的离机数据路径**：选远端 provider 前需一次性/工作区级显式授权（`remoteEmbeddingAuthorizationGuard` + 签名 Workspace grant，store 用 `authorizationSigningKeyPath`）。MCP Bearer 仅保护本地端点，**不**授权远端嵌入。
- **rg 命令沙箱**（`managed-rg.ts`）：拒绝 `| > & ; < > ( )` 与 shell 扩展（`$()`/`${}`/`` ` ``）；`assertRootScopedPath` 用 `pathEscapesRoot` 阻止 `..` 逃逸；`--follow` 被拒（防符号链接逃逸）。API key 在 manifest 中以 HMAC 指纹（`providerApiKeyIdentity`）存储，不落明文。
- 默认本地优先：server 仅 loopback；local 模型不发射内容；远程模型需 `--allow-remote` 或签名 grant，headless 会话绝不自动授权。

---

## 7. 性能 / 容量证据（A8 核心基准）

### 7.1 managed-ripgrep 路线（**沙箱内可忠实执行**，合成非机密语料）

复现脚本：`logs/research/M5-W18/A8-benchmark-ripgrep.mjs`（Node 内置 + 系统 `rg 14.1.0`，镜像 `buildRipgrepArgs` 参数集，无依赖安装）。

语料：300 `.ts`（其中 i%7==0 含 `AuthService.handleCredentials` + 中文注释「处理用户凭证」）+ 120 `.md`（中英混排含「凭证管理」「认证流程 AuthService」）。

| 查询 | 中位延迟(3 次预热) | 命中行 | 字节 |
|---|---:|---:|---:|
| 精确字面 `AuthService` | **11.9 ms** | 163 | 17889 |
| 定长串（中文多字节）`凭证` | **11.5 ms** | 163 | 18965 |
| 正则 `handle\w+` | **16.6 ms** | 43 | 4799 |
| 正则（区分大小写）`authservice` | 11.0 ms | 0 | 0 |

结论：
- **零索引体积**（按需扫描），亚 20ms/420 文件；延迟随扫描字节线性增长，大仓库仍保持 rg 量级（已知 ripgrep 在 GB 级代码库秒级）。
- **UTF-8 多字节正确**：中文「凭证」命中 163 行、列偏移经 `Buffer.byteLength` 计算无误。
- **限流**：zvec-grep 包装层在 `limit` 超界即 `child.kill()` 截断（`rg_truncated`），保证有界输出。
- 内存：沙箱缺 `/usr/bin/time` 未测；ripgrep 为流式恒定内存（经验 ~数十 MB），包装层开销小且恒定。

### 7.2 FTS/BM25 + vector + hybrid（**沙箱外，源码级论证 + 文档，待 W18-R 边界解除后补执行基准**）

- 依赖外部 `@zvec/zvec`（Apache-2.0）提供索引/BM25/向量/RRF 存储；本快照不含其源码，无法在此执行。
- 向量维度 256–2560、输入上限 256–128000 tokens（docs/07 模型表），**全部余弦相似度**。local 模型（Model2Vec FP16 / ONNX Qx / GGUF）不发射内容；remote（qwen）发射查询/内容至 provider。
- 索引体积：与语料规模、chunk 大小、维度成正比；Model2Vec（256-d）远小于 Transformer（768–2560-d）。具体数字需建索引后测，W18-R 内不下模型故**留待 W19 基准**。
- hybrid/RRF：融合成本为各路线结果的小集合合并 + 排序，开销可忽略；瓶颈在 embedding 推理（vector 路线）与索引 I/O（FTS）。

---

## 8. 依赖 / 许可

- `@zvec/zvec` Apache-2.0（核心引擎，外部）；`@vscode/ripgrep` MIT（rg 二进制封装）；`@huggingface/transformers`+`tokenizers` Apache-2.0（本地嵌入推理 WASM）；`web-tree-sitter`+`tree-sitter-wasms` MIT（代码结构抽取）；`node-llama-cpp` MIT(optional, GGUF)；`zod`/`jsonc-parser` MIT/ISC。
- 全部 Apache-2.0/MIT/ISC → 与产品（Apache-2.0）许可兼容。**但**：W19 若 COPY `@zvec/zvec` 或 `transformers.js`，须走 A10 源移植台账（provenance/NOTICE/修改声明/原生足迹），且不得盲目整库复制（dispatch L1366）。

---

## 9. unavailable-model failure（关键失败模式）

- **rg / FTS 路线不依赖模型** → 永不触发模型缺失。
- **vector 路线**：`embeddingModelForSearch` 仅当某 route `mode==="vector"` 才解析模型；若无 `embeddingModel` 且未配置（无 `--embedding`/`ZVEC_GREP_EMBEDDING`/默认）→ `requireEmbeddingModel` 抛 `ZVEC_GREP.ENGINE.SERVICE.EMBEDDING_MODEL_REQUIRED`，并附带明确 hint（如何提供模型）与 examples（`local/potion-code-16m-v2`, `qwen/...`）。
- 即：**模型不可用时 vector 路线硬失败（非静默降级）**，错误码可机器识别。这是 product 必须复制的契约——未来 product 的语义路线在模型缺失时应**显式报错**而非返回空/假结果。
- 运行时加载失败（transformers.js / node-llama-cpp 模型文件缺失）属更下游；上层契约同上（loading 异常上抛为 `EngineError`）。

---

## 10. stale-index 行为

- `fileFreshnessStatus`：有 `indexedTime` 且 `indexedTime >= mtimeMs` → `fresh`；否则若 `contentHash` 与当前 sha256 一致 → `fresh`；否则 `possibly_stale`。
- 结果**照常返回**，仅 `status` 字段标注；不阻断检索。product 采纳时应沿用「返回 + 标注」而非「隐藏/报错」。

---

## 11. multilingual / code 行为

- rg 原生 UTF-8/字节感知（基准已证中文命中正确）。
- 代码结构抽取：`CodeExtractor` 对 C/C++/Go/Java/JS-TS/Python/Rust 等保留符号/签名/面包屑；`.vue`/`.svelte` 取 `<script>` 块；Markdown 取标题分段；其余回退纯文本 chunk（docs/04 抽取表）。
- 向量路线语言覆盖取决于所选模型（potion-multilingual-128m 覆盖 101 语言；embeddinggemma-300m 广覆盖）。product 应「先用最小覆盖模型，再按真实查询比较」。

---

## 12. 路线分类与采纳建议（核心交付）

> 分类域：COPY（直接复制源码） / ADAPT（改造后采用） / REIMPLEMENT_FROM_BEHAVIOR（按行为重实现） / DEFER（暂缓至 W19+） / REJECT（当前拒绝）。

| # | 路线/特性 | 分类 | 理由与 product 落点 |
|---|---|---|---|
| R1 | **Managed ripgrep 包装层**（流式解析 + limit-kill + 路径/根作用域沙箱） | **ADAPT** | 零索引、零模型、亚 20ms、UTF-8 安全；沙箱逻辑可移植。product 经 Tauri Rust command 包 rg（或复用 `@vscode/ripgrep`），照搬 `scanManagedRgCommand` 的 shell 操作符拒绝 + `assertRootScopedPath` 根逃逸防护。 |
| R2 | **Filters / limits**（glob/type/ignore/max-depth/max-filesize/limit/context） | **ADAPT** | 语义与 product 文件搜索过滤一一对应，直接映射。 |
| R3 | **RRF 融合（K=60）** | **REIMPLEMENT_FROM_BEHAVIOR** | 算法极小且众所周知；product 不应为融合而依赖 `@zvec/zvec`。复制 `matchedBy`/`fresh`/ranking-metadata schema。 |
| R4 | **Stale-index 新鲜度**（mtime+contentHash→fresh/possibly_stale，返回即标注） | **ADAPT** | 廉价且友好；product 索引路线直接复用。 |
| R5 | **Unavailable-model 硬失败契约**（显式 `EMBEDDING_MODEL_REQUIRED`，非静默） | **REIMPLEMENT_FROM_BEHAVIOR** | 必须在 product 语义路线复刻，避免空结果误导。 |
| R6 | **Indexed FTS/BM25** | **DEFER** | 需 `@zvec/zvec`（外部）或 BM25 库（tantivy/minisearch）；核心引擎不在本快照，采纳须经 A10 台账 + A3/A4 裁决。W19 可 COPY `@zvec/zvec`（Apache-2.0）。 |
| R7 | **Vector/semantic 路线** | **DEFER→默认 local-only / 否则 REJECT（W19 外）** | 需嵌入推理运行时（transformers.js WASM / node-llama-cpp）与索引存储，原生足迹大、隐私面宽（远端发射）。除非选定 local-only 小模型（potion-code-16m），否则**拒绝默认开启**；远程嵌入需显式工作区授权（见 R8）。 |
| R8 | **Remote-embedding 授权模型**（签名 grant + 非仅 API key + loopback-only） | **ADAPT（若启用远端） / REJECT（默认）** | 强契约：product 若做远端嵌入必须 COPY 此授权模式；默认保持 local-only。 |

### 采纳路线（route-selection recommendation）

1. **立即（低风险，可 W19 先行）**：ADOPT **managed ripgrep（R1+R2）**——关闭 product 最大缺口（无任何文件/内容检索），零依赖、零索引、亚 20ms、UTF-8 安全；照搬沙箱与 limit 包装。这是确定性最高、回退最干净的起点。
2. **后续（W19+，受 A10 台账约束）**：在其上叠加 **RRF 融合（R3）+ FTS/BM25（R6）**，可选 local-only 小模型向量（R7）；融合层在 vector 路线模型缺失时**优雅降级到 FTS→rg**，永不静默（R5）。
3. **默认拒绝**：远端嵌入（R8）——保持 local-only；若确需，强制签名工作区授权、server loopback-only、内容/查询不发射至未授权端点。

---

## 13. 测试复用（test reuse）

- `test/unit/search.test.mjs`、`test/unit/core.test.mjs`、`test/rg-cli.test.mjs`、`test/unit/cli-format.test.mjs`：提供路线/RRF/limit/ranking 行为的 synthetic fixture 与期望，可作为 product 对应单元测试的形态参考（不复制依赖，仅借鉴断言结构）。
- `A8-benchmark-ripgrep.mjs`：可复现的 rg 路线基准（无依赖），product 采纳 R1 后可直接作为回归基准。

---

## 14. 未决问题（unresolved questions，交 A0）

1. product 检索的**首要落点**是哪个面板/场景（全局命令面板？文件内容？知识库？浏览器历史？）——决定 R1 的集成面。
2. FTS/vector 的**索引存储位置**与 product 既有持久化（`.zvec-grep` 约定 vs product 自有 store）——需 A1/A10 对齐。
3. 是否引入 `@zvec/zvec`（COPY）还是另选轻量 BM25 库——待 A3 采纳裁决与 A10 台账。
4. 向量路线默认模型与设备（CPU/Metal/Vulkan/CUDA）在 product 目标平台的可用性——W18-R 内不下模型，留待 W19 真实基准。
5. Tauri 侧 rg 二进制分发：用 `@vscode/ripgrep` 随包，还是系统 rg（zvec-grep 二者皆试，bundled 优先）——product 打包策略待定。

---

## 15. 诚实边界声明（HONEST BOUNDARY）

- FTS/BM25/vector/hybrid 路线的**执行级基准**在本沙箱不可得（dispatch 禁止装依赖/下模型/起 daemon/连远端）。本节相关结论来自**源码 + 架构/嵌入文档 + 模型表**的逆向，已明确标注「源码级论证，待 W19 补执行基准」。
- managed-ripgrep 路线已**真实执行**基准（系统 `rg 14.1.0`），结果可复现。
- 未对 `@zvec/zvec` 内部实现做执行级验证（其源码不在本快照）。
- 未修改任何产品代码、未安装依赖、未 push（符合 W18-R 研究边界）。
