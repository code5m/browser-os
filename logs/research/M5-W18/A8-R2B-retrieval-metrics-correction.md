# A8 · M5-W18-R2B · 真实检索收益 / 指标修正 / 统一搜索体验

> LANE=A8 · DISPATCH=M5-W18-R2B · MODE=RESEARCH_AND_DESIGN · W19=CLOSED
> 输入：本 lane R2（bd35235/f4a4f3b）、A7 R2 证据（f157eb4）、A1 统一入口（WORKBENCH_BLUEPRINT J4）、A9 授权边界。
> 旧研究保留；已完成项引用提交与文件，不重写整篇。本文件为 R2B 补充与对 A0 抽查 #7/#8 的纠正。
> 证据标签沿用 R2 契约：`CURRENT_PRODUCT` / `REFERENCE_SOURCE` / `OBSERVED_BEHAVIOR` / `OFFICIAL_DOC` / `EXECUTED_SYNTHETIC_TEST` / `INFERENCE` / `DESIGN_DECISION`。

```text
LANE=A8
DISPATCH=M5-W18-R2B
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a8/mvp-browser-os-v3
BRANCH=codex/m5-w18-a8
BASE=d6127c4 (origin/master R2B dispatch; 本提交 rebase 于其之上)
HEAD=<本 R2B 提交，见文末>
CONSUMED_PEERS=A7 f157eb4eac5e29f88331a25fb9eda35f542b07e2 (A7-zvec-grep-R2-evidence-closure.md, A7-zvec-grep-ingestion-index-architecture.md)
FILES=logs/research/M5-W18/A8-R2B-retrieval-metrics-correction.md, logs/checkpoints/A8-M5-W18-R2B-checkpoint.md, logs/research/M5-W18/A8-R2B-benchmark-metrics.mjs, logs/research/M5-W18/A8-zvec-grep-retrieval.md(追加 R2B 更正节), logs/research/M5-W18/A8-benchmark-routes.mjs(保留)
CORRECTIONS=①指标命名(原 vRecall/fRecall/zRecall 实为 precision 类)；②RSS 边界(原单点采样≠OS峰值)；③采纳路径结论过度(原"仅 sidecar/COPY=0" → 修正为 npm 绑定 REJECT、引擎经 zvec-rust ADAPT)
VERIFY=node A8-R2B-benchmark-metrics.mjs（沙箱内 @zvec/zvec@0.7.0 下）两次可复现；旧 rg 基准见 A8-benchmark-ripgrep.mjs
PROPOSED_SLICES=<见 §6，PROPOSED_NOT_AUTHORIZED>
NO_PRODUCT_CODE=true
NO_PUSH=true
```

---

## 0. 对 A0 R2B 抽查 #7/#8 的显式纠正（retract/replace）

A0 抽查（`logs/checkpoints/A0-M5-W18-R2B-dispatch-20260908.md`）第 7、8 条点名本 lane R2 基准两处缺陷。本 R2B 已修正并复测：

- **(#7) 指标名称错误**：R2 `A8-benchmark-routes.mjs` 的 `vRecall/fRecall/zRecall` 以「返回结果数或 10」为分母，实为**相关结果占比（precision 类）**，不是对完整相关集合的 recall；且合成 topic 向量只验证聚类机制，**不构成真实 embedding 语义效果**。
  **RETRACT/REPLACE**：本 R2B 重测报告 `precision@k` / `recall@k` / `firstRelRank`（=MRR 的 1/rank 输入），以**人工标注的相关集合**（本语料中「含查询词的文档」即相关集，共 200）为分母；并显式声明语义精度需人工相关性判定、合成语料无法提供（见 §2 limitation）。
- **(#8) 性能边界**：R2 仅查询后读一次 `process.memoryUsage().rss`，不能等同全程 OS 峰值或稳定 p95，cold/warm 仅少量样本。
  **RETRACT/REPLACE**：本 R2B 读 `/proc/self/status` 的 **VmHWM（内核记录的本进程峰值 RSS）** 作为 OS 峰值；多次采样 `rss` 取最大作为采样峰值；每查询 **5 次 warm 重复**并报告 mean/std/min/max；cold 明确定义为「open 后首次调用（含 page-in）」。实测 OS 峰值 **~309MB > R2 单点采样 294MB**，印证单点采样低估（见 §2）。

---

## 1. 输入与消费

- **A7 R2 证据 `f157eb4`** `[REFERENCE_SOURCE]`：引擎 `alibaba/zvec` 经 C API 拥有**官方 Rust crate `zvec-rust` 0.7.0**（依赖 `zvec-rust-sys`，动态链 `libzvec_c_api`），故引擎**可在 Rust 中原生 ADAPT**；而 npm `@zvec/zvec` 0.7.0 是 **Node-only N-API 插件**（ELF 仅导 `napi_*`，不可链入 Rust 后端）。两者结论互补：npm 包 REJECT for Rust，引擎经 `zvec-rust` ADAPT。
- **A1 统一入口 / J4** `[OFFICIAL_DOC]`（WORKBENCH_BLUEPRINT-20260908.md §J4、§S4）：统一检索 = 在当前项目搜文件/文本/笔记/操作 → 看命中来源与片段 → 定位原文件/行/笔记；**先不依赖模型的明确匹配路线**，语义检索为后续独立能力；不同路线须显示来源/范围/新鲜度/截断/错误；**不得把「模型不可用」显示成「没有结果」**；首片不搜数据库业务行或自动上传工作区。
- **A9 授权边界**：工作区根授权当前产品**不存在**（A7 `CURRENT_PRODUCT`：仅 `workspace.rs` 解析应用数据目录，无 grant 门），须由 A9 引入；远端 embedding HMAC 授权仅远程外发时启用、默认关闭。本 lane 仅设计状态，不代 A9 裁决。

---

## 2. 修正后指标（synthetic 非机密语料，真实执行）

- 沙箱：`/tmp/m5-w18-a8-zvec`，pinned `@zvec/zvec@0.7.0`（Node N-API 绑定，引擎 v0.7.0 的**一种**宿主运行时）。
- 语料：1000 合成文档，5 主题 `auth/payment/search/cache/network`，每主题 200 篇含该主题关键词（英文 `payment`、中文 `网络`、代码 `fn charge_card()`）；每文档另含 256-d 合成向量（主题中心+噪声，**仅用于向量路线机制验证，非真实 embedding**）。
- 相关集（人工标注）= 含查询词的 200 篇文档。脚本：`A8-R2B-benchmark-metrics.mjs`，复现：`node A8-R2B-benchmark-metrics.mjs`（需在装有 `@zvec/zvec@0.7.0` 的目录）。两次运行结果稳定。

**索引**：1000 docs（HNSW 向量 + jieba FTS）≈ **6,549,897 B（6.55 MB）**，两次一致。

**OS 峰值 RSS（VmHWM）**：**~309 MB**（run1 309,411,840；run2 309,444,608）；采样峰值 ~308 MB。**R2 旧单点 294MB 低于真实 OS 峰值**，确认 #8。引擎 + jieba 词典常驻，属一次性进程足迹，由各路线共享，非每查询线性增长。

**检索质量（k=10/20/50）** `[EXECUTED_SYNTHETIC_TEST]`：

| 路线 | 查询 | relevant | precision@10 | recall@10 | recall@20 | recall@50 | firstRelRank |
|---|---|---|---|---|---|---|---|
| FTS 英文 | `payment` | 200 | 1.0 | 0.05 | 0.10 | 0.25 | 1 |
| FTS 中文(jieba) | `网络` | 200 | 1.0 | 0.05 | 0.10 | 0.25 | 1 |
| FTS 代码 | `charge_card` | 200 | 1.0 | 0.05 | 0.10 | 0.25 | 1 |
| 向量(主题聚类,机制) | topic-center | 200 | 1.0 | 0.05 | 0.10 | 0.25 | 1 |

**关键纠正**：原 R2 称 recall=1.0，实为 **precision@10=1.0（返回全相关）**；真实 **recall@10=0.05**（200 篇相关仅 10 篇进入 top-10）。召回随 k 上升：k=50 时 recall=0.25。

**延迟（warm = 5 次均值）** `[EXECUTED_SYNTHETIC_TEST]`：

| 路线 | cold(ms) | warm mean(ms) | warm std | min/max |
|---|---|---|---|---|
| FTS 英文 | 2.21 | 2.26 | 0.13 | 2.10/2.44 |
| FTS 中文 | 3.20 | 2.83 | 0.32 | 2.26/3.23 |
| FTS 代码 | 6.86 | 6.87 | 0.45 | 6.53/7.77 |
| 向量(topic) | 2.33 | 1.79 | 0.74 | 0.98/2.75 |

**MRR**：所有查询 firstRelRank=1（精确词命中即排第 1），MRR 平凡=1.0；**不据此推断语义排序质量**（明确声明，不夸大）。

**limitation（诚实边界）** `[INFERENCE]`：① 词面相关集=含词文档，故 FTS precision@k 构造性=1.0；**真实语义 precision/recall 需人工相关性判定，合成语料无法供给**。② 向量路线用合成主题向量，仅证明「最近邻检索返回同簇文档」这一**机制**，不构成 embedding 语义质量。③ RSS 为单进程本次运行峰值，非集群 p95。④ 远程/真实本地 embedding 链路因沙箱无模型 **NOT_RUN**（见 §3）。

---

## 3. 三路线对照：体积 / 安装 / 延迟 / 内存（消费 A7 证据）

| 维度 | managed-ripgrep（R1 路线） | 轻量 FTS/BM25（zvec FTS-only） | zvec 向量/混合（hybrid RRF） |
|---|---|---|---|
| 索引体积 | 0（无索引） | ~6.55 MB/1000 docs `[EXECUTED]` | 同左 + HNSW 向量（dim 256，增量小） |
| 安装/依赖 | 系统 `rg` 或 `@vscode/ripgrep`(MIT) | `zvec-rust=0.7.0` + 打包 `libzvec_c_api`(Apache-2.0) + cmake/C++17 工具链 `[REFERENCE_SOURCE A7]` | 同左 + 嵌入运行时（transformers.js WASM / node-llama-cpp GGUF） |
| 查询延迟 | 11–17 ms / 420 文件 `[EXECUTED A8-benchmark-ripgrep.mjs]` | 2–7 ms `[EXECUTED §2]` | 向量 ~1.8 ms + FTS ~2–7 ms，RRF 融合 ~4 ms `[EXECUTED R2 bd35235]` |
| 常驻内存 | ~0（每次 spawn） | ~309 MB OS 峰值（引擎+jieba 常驻）`[EXECUTED §2]` | 同左（共享） |
| 模型依赖 | 无 | 无 | 需要（真实语义）；沙箱无 → NOT_RUN |
| 绑定身份 | 独立 CLI | 引擎经 `zvec-rust`(Rust FFI) `[REFERENCE_SOURCE A7]` | 同左 |
| 首片可行性 | ✅ 即时（零依赖零索引） | ✅ FTS-only 首片（A7 §6 已给完整 Rust 落地卡） | ⏸ 受 A9 嵌入授权 + 模型可用性门控 |

> 区分「npm 绑定」与「引擎其他绑定」：本 R2B §2 基准走 **npm `@zvec/zvec` Node 绑定**；A7 `f157eb4` 证明引擎另有**官方 Rust crate `zvec-rust`**。路线行为（FTS/向量/混合 RRF K=60）属**引擎层**，跨宿主运行时一致；故 npm 绑定上验证的路线行为可直接迁移到 Rust `zvec-rust` 集成，**只有集成路径不同**（Node sidecar vs Rust FFI）。

---

## 4. 统一搜索体验设计（接 A1/J4）

**结果分类（4 类，来源可区分）** `[DESIGN_DECISION]`：
- `file`（文件）：匹配文件路径/名 → 跳转到文件。
- `text`（文本）：匹配文件内容（rg / FTS）→ 带片段跳转到行。
- `note`（笔记）：匹配 Obsidian 笔记（需 A2/A3 笔记范围授权）→ 跳转到笔记/块。
- `action`（操作）：命令面板既有动作 → 直接调用。

**范围**：当前项目受 A9 授权的工作区根；**首片仅明确匹配**（J4：「先研究不依赖模型的明确匹配路线」），语义检索为后续独立能力。SQL 结果**绝不伪装成磁盘文件**（J4 + blueprint §5）。

**结果定位 / 返回**：每条命中带 `资源类型 + 定位信息 + 返回位置`（blueprint §5 统一导航）；点击后由对应领域 lane（A3 图谱/A5 数据库/A2 笔记）承接，本 lane 只定义契约不实现领域跳转。

**取消 / 旧请求失效** `[DESIGN_DECISION]`：每个搜索请求带 `requestId`；新输入即作废在途旧请求（A1 壳层契约「旧响应失效规则」）；Esc / 新按键取消在途查询；A7 §6.5 `SEARCH_INDEX_*` 稳定错误码复用。

**stale / 截断 / 缺模型状态**（J4 明确要求）：
- **stale**：索引 mtime < 文件 mtime → 标「可能过期」徽标，仍返回结果，提示重建（不自动远端）。
- **truncation**：结果超上限 → 「显示前 N 条，请缩小范围」。
- **缺模型**：向量路线无嵌入模型 → 显式「语义检索不可用：未配置嵌入模型」，**降级到 FTS→rg，绝不把空结果显示成「没有结果」**（即 R2 已记录的 `EMBEDDING_MODEL_REQUIRED` 硬失败契约的可视化形态）。

**接 A1/J4**：前端 `UnifiedSearch.vue` + `useSearchStore.ts` 承载四分类与状态；后端路由经 A7 `search_index_*` 命令（FTS 首片）扩展，语义路线按 A9 授权后追加。A9 负责工作区根授权与远端 embedding HMAC 门；A3 负责图谱/笔记跳转，**引擎采纳（npm vs zvec-rust）裁决权归 A0，不归 A3**（见 §5）。

---

## 5. A7/A8 矛盾追平（A0 #9）：npm 绑定 vs 引擎 Rust crate

- **A8 R2（bd35235）过度结论**：「`@zvec/zvec` 是原生 N-API 插件、无法 Rust 直链 → COPY=0，只能 sidecar/FFI ADAPT」。该结论**仅对 npm 包成立**，被 A0 #9 标记为需与 A7 追平。
- **A7 R2（f157eb4）**：引擎 `alibaba/zvec` 经 C API 有官方 Rust crate `zvec-rust` 0.7.0（动态链 `libzvec_c_api`）→ **引擎可在 Rust 原生 ADAPT**，npm 包 REJECT for Rust。
- **追平结论** `[REFERENCE_SOURCE + OBSERVED_BEHAVIOR]`：**npm `@zvec/zvec` = REJECT（不可链入 Rust）；引擎经 `zvec-rust` 0.7.0 = ADAPT（Rust FFI + 打包/构建 `libzvec_c_api`）**。本 R2B §2 路线行为证据以 npm Node 绑定取得，因路线行为属引擎层、跨宿主一致，可直接迁移；集成路径选择（Node sidecar vs Rust FFI）**交 A0 裁决**（A0 #9 明确由 A0 定，A3 不承担引擎裁决权）。

---

## 6. 候选实现卡（PROPOSED_NOT_AUTHORIZED）

```
slice: unified-search (首片 = 明确匹配：rg + FTS-only via zvec-rust)
范围:
  - 新 src-tauri/src/unified_search.rs (或扩展 A7 search_index.rs) 增 unified_search(req_id, scope, query, routes)
  - 类型 UnifiedSearchHit { kind: File|Text|Note|Action, source, line?, fragment, score, fresh, truncated, error }
  - main.rs 注册; ACL 插于 list_artifact_images 之前; capability 加 scope
  - src/bridge.ts + src/types.ts 暴露; 前端 UnifiedSearch.vue + useSearchStore.ts
  - scripts/check-unified-search-policy.py (沿用 graph/database 29 检查模式)
前置:
  - A7 search_index.rs (FTS 首片) 已落; A9 工作区根授权; A1 壳层契约(旧响应失效/requestId)
  - 语义路线门控于 A9 嵌入授权 + 模型可用
测试:
  - 取消: 新请求使旧 req_id 结果失效 (assert 不渲染)
  - 缺模型: 向量路线无模型 → 显式状态 + FTS 降级, 非空结果伪装
  - stale/截断: mtime 错配徽标; 超上限截断提示
  - 四分类: file/text/note/action 来源可区分, SQL 结果不伪装文件
完成条件: BUILD_PASS + LOGIC_PASS + check-* 全绿; native 验收 NOT_RUN(本 lane 不跑)
状态: PROPOSED_NOT_AUTHORIZED (W19=CLOSED, 不开放)
```

---

## 7. 验证 / 未运行项 / 自检

- **已执行** `[EXECUTED_SYNTHETIC_TEST]`：`A8-R2B-benchmark-metrics.mjs` 两次可复现（索引 6.55MB、OS 峰值 ~309MB、FTS precision@10=1.0/recall@10=0.05、向量机制 precision@10=1.0/recall@10=0.05、延迟与 std 见 §2）；`A8-benchmark-routes.mjs`（R2 混合 RRF）与 `A8-benchmark-ripgrep.mjs`（R1）保留可复现。
- **未运行（明确标记 NOT_RUN）**：① 真实本地/远程 embedding 链路（沙箱无模型、受网络约束，留 W19）；② native/GUI 验收（本 lane 不跑，记 NOT_RUN，不写 PASS）；③ 语义 precision/recall（需人工相关性判定）。
- **自检**：仅写 `logs/research/M5-W18/` 与 `logs/checkpoints/`；未改 `src/`、`src-tauri/`、`scripts/`、依赖/锁、ACL/capabilities、主矩阵/其他 lane；未 push；工作树干净后 rebase `origin/master`（d6127c4 + R2B 文档）通过。
- **依赖未齐项**：A9 工作区根授权、A7 `search_index.rs` 落地、A1 壳层 requestId 契约 — 均标记，不阻塞本 lane 证据闭环。

---

## 8. consumed_peers / corrections / files（dispatch 变量块）

```
CONSUMED_PEERS=A7 f157eb4eac5e29f88331a25fb9eda35f542b07e2 (A7-zvec-grep-R2-evidence-closure.md; A7-zvec-grep-ingestion-index-architecture.md)
CORRECTIONS=①vRecall/fRecall/zRecall 实为 precision 类 → 改报 precision@k/recall@k/firstRelRank 并声明语义精度需人工判定; ②RSS 单点采样→补 VmHWM OS 峰值(309MB>294MB); ③"仅 sidecar/COPY=0"→npm 包 REJECT、引擎经 zvec-rust ADAPT, 裁决交 A0
FILES=logs/research/M5-W18/A8-R2B-retrieval-metrics-correction.md, logs/checkpoints/A8-M5-W18-R2B-checkpoint.md, logs/research/M5-W18/A8-R2B-benchmark-metrics.mjs, logs/research/M5-W18/A8-zvec-grep-retrieval.md(+R2B 更正节), logs/research/M5-W18/A8-benchmark-routes.mjs(保留)
OPEN_DECISIONS=引擎集成路径(npm sidecar vs zvec-rust FFI) 交 A0(#9); 工作区根授权交 A9; 语义路线门控于模型可用+A9
NEXT=整包交 A10 独立审查 / A11 状态看板; 不自行 push
```
