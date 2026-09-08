# Lane A8 · M5-W18-R2B Checkpoint

> LANE=A8 · DISPATCH=M5-W18-R2B · STATUS=READY_FOR_REVIEW · W19=CLOSED
> 工作树：`/home/ainfinit/.codex/worktrees/m5-w18-a8/mvp-browser-os-v3`（branch `codex/m5-w18-a8`，rebase 于 `origin/master` d6127c4 + R2B 文档之上）

## 1. 本轮产物（仅 logs/，零产品代码）
| 文件 | 内容 |
|---|---|
| `logs/research/M5-W18/A8-R2B-retrieval-metrics-correction.md` | R2B 主报告：指标修正、三路线对照、统一搜索体验、A7/A8 追平 |
| `logs/research/M5-W18/A8-R2B-benchmark-metrics.mjs` | 修正指标基准脚本（precision/recall/MRR/OS-峰值 RSS，CN/EN/code，重复试验） |
| `logs/research/M5-W18/A8-zvec-grep-retrieval.md` | R2 报告，追加 §10「R2B 更正声明」（正文保留） |
| `logs/research/M5-W18/A8-benchmark-routes.mjs` / `A8-benchmark-ripgrep.mjs` | R2 既有基准（保留，可复现） |
| `logs/checkpoints/A8-M5-W18-R2B-checkpoint.md` | 本文件 |

## 2. 对 A0 R2B #7/#8 的纠正（已实复测）
- **(#7) 指标命名**：原 `recall=1.0` 实为 precision 类。重测：FTS `precision@10=1.0 / recall@10=0.05`（200 相关仅 10 入 top-10，k=50 时 recall=0.25）；向量路线同为 precision@10=1.0/recall@10=0.05，且**仅机制、非语义**。
- **(#8) RSS 边界**：原单点采样 ~294MB 非 OS 峰值。补 `/proc/self/status` VmHWM → **真实 OS 峰值 ~309MB**（>294MB）；cold/warm ≥5 次重复报 mean/std/min/max。

## 3. 消费 A7 证据（f157eb4）与追平
- A7 证明引擎经 C API 有**官方 Rust crate `zvec-rust` 0.7.0**（动态链 `libzvec_c_api`）→ 引擎可在 Rust 原生 ADAPT；npm `@zvec/zvec` 仅 Node N-API（REJECT for Rust）。
- 故 R2「COPY=0，仅 sidecar/FFI」修正为：**npm 包 REJECT、引擎经 zvec-rust ADAPT**；路线行为跨宿主一致、证据可迁移；集成路径选择**交 A0 裁决（A0 #9），A3 不承担引擎裁决权**。

## 4. 三路线对照（体积/安装/延迟/内存）
- managed-ripgrep：0 索引，11–17ms/420 文件 `[EXECUTED]`。
- 轻量 FTS/BM25（zvec FTS-only，A7 §6 首片）：~6.55MB/1000docs，2–7ms，~309MB OS 峰值，依赖 `zvec-rust + libzvec_c_api + cmake/C++`。
- zvec 向量/混合：同引擎 + 嵌入运行时；向量 ~1.8ms + FTS 融合；需模型（沙箱 NOT_RUN）。

## 5. 统一搜索体验（接 A1/J4，PROPOSED_NOT_AUTHORIZED）
四分类（file/text/note/action）、范围（A9 授权根）、结果定位、取消/旧请求失效（requestId）、stale/截断/缺模型状态（缺模型显式提示并降级 FTS，绝不显示「无结果」）。候选实现卡 §6，待 A7 `search_index.rs` + A9 授权 + A1 壳层 requestId 契约。

## 6. 验证 / 未运行 / 自检
- **已执行**：`A8-R2B-benchmark-metrics.mjs` 两次可复现（索引 6.55MB、OS 峰值 ~309MB、FTS precision@10=1.0/recall@10=0.05、向量机制 precision@10=1.0/recall@10=0.05、延迟与 std 见报告）。
- **NOT_RUN（明确标记）**：真实 embedding 链路（沙箱无模型）、native/GUI 验收、语义 precision/recall（需人工相关性判定）。
- **自检**：仅写 `logs/`，未动 `src/`、`src-tauri/`、依赖/锁、ACL/capabilities、主矩阵/他 lane；未 push；rebase `origin/master` 通过；工作树干净。

## 7. consumed_peers / 待裁决
```
CONSUMED_PEERS=A7 f157eb4eac5e29f88331a25fb9eda35f542b07e2
OPEN_DECISIONS=引擎集成路径(npm sidecar vs zvec-rust FFI)→A0(#9); 工作区根授权→A9; 语义路线门控于模型可用+A9
NEXT=整包交 A10 独立审查 / A11 状态看板; 不自行 push
```
