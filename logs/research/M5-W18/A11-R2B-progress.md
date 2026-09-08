# A11 · M5-W18-R2B 进度与验收总包

```text
LANE=A11
DISPATCH=M5-W18-R2B
STATUS=WAITING_DEPENDENCY
  - A11 整包自身：READY_FOR_REVIEW
  - 全 W18 证据闭环缺口：A4 R2 证据闭环缺失、A10 R2B 独立复核 findings 未产出
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a11/mvp-browser-os-v3
BRANCH=codex/m5-w18-a11
BASE=434e63f  (origin/master: "docs(M5-W18): define daily workbench and second research dispatch")
HEAD=abbbf63cb2f8b180f63d59571fead7381209ad68
NO_PRODUCT_CODE=true
NO_PUSH=true
```

## 0. 本包交付物（A11 允许范围）

1. `A11-R2B-progress.md`（本文件）
2. `A11-R2B-integration-manifest.json`
3. `A11-R2B-A0-brief.md`（≤120 行）
4. `A11-R2B-S0-S5-task-cards.md`（候选卡 + 测试矩阵，PROPOSED）
5. 旧 R2 报告 `A11-verification-architecture-R2-20260908.md` §10 已**更正**：不再声称“peer 均无 R2”，改为指向本 R2B 包并登记 A4/A10 缺口。

> 旧 R2 报告与 `A11-verification-architecture-20260908-0800.md`（R1）保留为历史，未重写。

## 1. 逐 lane 状态（固定 SHA，均经 `git show <sha>:<path>` 只读消费）

| Lane | Baseline | HEAD (full SHA) | R2 状态 | 缺口 / 债务 | 依赖 | 建议下一步 |
|---|---|---|---|---|---|---|
| A1 | d6127c4 | 526e2ef628c0f10617a816c7783a9b3ef72de3ad | PASS_WITH_DEBT（R2 证据闭环，6 项 R1 纠正 + 机器可查事实附录） | W17-D1~D5、DbValue 双源漂移 G4、前端单测网 G5 | — | A0 清 W17 债；A10/A11 统一 G4；A11 引导 G5 |
| A2 | d6127c4 | bcdfc3bca922c398422f8efd652900896eb1652e | PASS_WITH_DEBT（R2 证据闭环，合成夹具 20/20） | B1: 8370 笔记 > GRAPH_MAX_NODES=5000（待 A1 容量决策）；B2: heading/block/embed 本库不可观测 | A1 容量决策 | W19 只读 vault 导入卡；rename/delete DEFER |
| A3 | 434e63f(rebase) | ab455c1db9d3fe732cdff1aa95a2dff9c51955e6 | R2B_PASS_WITH_DEBT（图谱/笔记导航设计；`check-graph-ui-logic.mjs` 113 pass） | 待 A2 W19 接入、消费 A1 G1 | A2 bcdfc3b、A1 a54eed1 | 右侧反链/局部图，不持有引擎裁决权 |
| A4 | 78d2cfb | b737e5d4cc285ceea507154cc6e0f261e2e8c01e | **R1 only，R2 证据闭环缺失** | 无 R2 commit；DbValue 契约/参考身份核对未以 R2 闭环 | A1 基线 | **WAITING：A4 须补 R2 证据闭环（dispatch 原要求）** |
| A5 | d6127c4 | a42b9154a8e2524a13f64d26f7a9815cc1345515 | PASS_WITH_DEBT（R2 8 项纠正；`check-database-ui-logic.mjs` 119 断言 pass） | 依赖 A4 DTO 冻结、A6 凭据/取消生命周期 | A4、A6、A3 | W19 打磨 DatabasePanel |
| A6 | 78d2cfb | 38faa2da7d521840966c83575cf152a573a68f91 | PASS_WITH_DEBT（R2_VERDICT=REWORK_CLOSED）凭据源到汇 + 竞态时间线 + fail-closed | 依赖 A4 源映射一致 | A4 | 取消/恢复/旧调度保证 |
| A7 | 78d2cfb | 07fda2f87c319f6731d065f69da1e00dae944a00 | PASS（R2 闭环，HEAD 8ec92bb）原生绑定/索引事务 | A0 抽查：跨绑定互换未实测、缓存命名冲突（`.zvec-grep` 根清理） | A10（Apache→Mulan 清单）、A9（workspace 授权门） | 待 A10 台账 + A9 门；首片 FTS-only |
| A8 | d6127c4 | f4a4f3bab71df7322a96c499facf201e35ffd441 | PASS_WITH_DEBT（R2 真实执行 benchmark） | A0 抽查：指标名错误（vRecall/fRecall 分母=结果数）、perf 边界（RSS/时长≠全程 p95） | A1/A3/A10 | W19 先 ADAPT managed-ripgrep；FTS/vector sidecar |
| A9 | 434e63f(rebase) | 28a934add0e09e26d42cd4759438e29f1300cba3 | PASS（R2 threat model）远端嵌入 egress 门强 | B2/B3/B7 三项 W19 前必修 | A3 采纳裁决（A0 批准前无依赖） | 若 A3 采纳：开 W19 切片（授权门/令牌/keychain/redactor） |
| A10 | d6127c4 | b81fa690e9ae3d4e12267cbabc8548ab1095e96f | PASS_WITH_DEBT（HOLD finalization pending peer R2） | **R2B review-findings 未产出**；peer R2 现已齐，应重读 | A1–A9 | **WAITING：A10 产出 `A10-R2B-review-findings.md`** |
| A11 | 434e63f | <HEAD> | R2B 整包 READY_FOR_REVIEW（本文件） | 无（A4/A10 为外部缺口） | 全部 | 交 A0 整合 |

## 2. 场景分列（不虚构总进度条）

- **Design（研究/设计）**：A1 基线、A2 语义、A3 导航、A4 架构、A5 蓝图、A6 审计、A7 索引、A8 检索、A9 威胁、A10 移植台账 —— 均 RESEARCH 完成，标记 PASS_WITH_DEBT / HOLD。
- **Code（产品代码）**：全部 `PROPOSED_NOT_AUTHORIZED`；W19 切片 S0–S5 见 `A11-R2B-S0-S5-task-cards.md`。本包及所有 peer 包均未写产品代码。
- **Native（原生验收）**：仅 A3/A5/A8/A9 有合成/脚本验证；真实 GUI/IPC/debug-release 均 **NOT_RUN**（研究边界，W19 才跑）。
- **User-trial（用户试用）**：全部 **NOT_RUN**；蓝图 §9 明确 J1–J6 状态 `UNVERIFIED`，不等同功能未实现。

## 3. 跨 lane 待 A0 裁决差异（源自 `A0-M5-W18-R2B-dispatch-20260908.md` §3）

1. 参考身份未对齐：本地 `dbx-src/README.md` 指向 `t8y2/dbx`，不能据此认定即用户 IDEA 功能 → A4/A5 区分体验参照与源码参照。
2. A0 自身数字更正：`src-tauri/src/database.rs` 实际有 **23** 行独立 `#[test]`（R1 audit 写 22 不准确）；声明计数 ≠ 本次执行通过数。
3. 前端测试被漏算：A1/A11“0 frontend”未覆盖既有 `scripts/check-*-ui-logic.mjs` 脚本 → 应说“缺某框架”而非“无前端测试”。
4. A2 行为证据混淆：§5 自写 resolver 20/20 部分 alias/同名/大小写推断未见独立验证 → 兼容性结论需官方/实际观察或标设计选择。
5. A7 接入结论过早：跨绑定互读/崩溃实测未提供，不能直接 PROVE/ADOPT → A7/A9/A10 复核。
6. A7 缓存设计冲突：以 `force_rebuild` 清理用户根下 `.zvec-grep` 会与外部索引归属冲突 → 改为产品自有缓存命名空间与代际提交。
7. A8 指标名称错误：vRecall/fRecall/zRecall 以返回结果数或 10 为分母，是相关结果占比，非真实召回率；其向量为 topic 中心加噪声，不测真实 embedding 语义。
8. A8 性能边界：仅操作后读 RSS、少量 cold/warm，不能等价全程 OS 峰值或稳定 p95。
9. A7/A8 未互相追平：A7 认为可用 Rust binding，A8 总结称只能 Node sidecar → 分别验证 npm 绑定与引擎其他接入路线，再由 A0 裁决。
10. 许可：产品根 LICENSE = **MulanPSL-2.0**（非 Apache-2.0）；dbx = Apache-2.0，移植署名归 A10。

## 4. S0–S5 候选卡与测试矩阵

见 `A11-R2B-S0-S5-task-cards.md`（全部 `PROPOSED_NOT_AUTHORIZED`，命令均可解析，未实际运行）。

## 5. A0 摘要

见 `A11-R2B-A0-brief.md`（≤120 行）。

## 6. CONSUMED_PEERS（固定 SHA，只读消费）

- A1 `526e2ef628c0f10617a816c7783a9b3ef72de3ad` → `A1-product-baseline-R2-20260908.md`, `A1-checkpoint-R2-20260908.md`
- A2 `bcdfc3bca922c398422f8efd652900896eb1652e` → `A2-obsidian-vault-semantics.md`, `A2-fixtures.md`, `A2-M5-W18-R-20260908.md`
- A3 `ab455c1db9d3fe732cdff1aa95a2dff9c51955e6` → `A3-R2B-graph-note-nav-20260908.md`, `A3-M5-W18-R2B-20260908.md`
- A4 `b737e5d4cc285ceea507154cc6e0f261e2e8c01e` → `A4-dbx-backend-architecture-map.md`, `A4-checkpoint.md`（**R1 only**）
- A5 `a42b9154a8e2524a13f64d26f7a9815cc1345515` → `A5-workbench-ux-map.md`, `A5-replication-blueprint.md`, `A5-checkpoint.md`
- A6 `38faa2da7d521840966c83575cf152a573a68f91` → `A6-security-lifecycle-audit.md`, `A6-checkpoint.md`
- A7 `07fda2f87c319f6731d065f69da1e00dae944a00` → `A7-zvec-grep-R2-evidence-closure.md`, `A7-zvec-grep-ingestion-index-architecture.md`, `A7-lane-checkpoint.md`
- A8 `f4a4f3bab71df7322a96c499facf201e35ffd441` → `A8-zvec-grep-retrieval.md`, `A8-benchmark-ripgrep.mjs`, `A8-benchmark-routes.mjs`, `A8-checkpoint.md`
- A9 `28a934add0e09e26d42cd4759438e29f1300cba3` → `A9-threat-model.md`, `A9-source-map.md`, `A9-checkpoint.md`
- A10 `b81fa690e9ae3d4e12267cbabc8548ab1095e96f` → `A10-dependency-bom.md`, `A10-source-transplant-ledger.md`, `A10-notice-and-review-gate.md`, `A10-checkpoint.md`
- A0 调度文档（在 origin/master `434e63f`）：`WORKBENCH_BLUEPRINT-20260908.md`、`A0-HANDOFF-LOW-COST-20260908.md`、`A0-M5-W18-R2B-dispatch-20260908.md`、`M5-W18-R2B-TASKS-20260908.md`、`PARALLEL_COMMAND_BOARD.md`、`WORKSPACE_IDENTITY.md`

## 7. 自检（交付前）

- [x] 未改任何产品代码 / `src` / `src-tauri` / `scripts` / ACL / 其他 lane / 主矩阵。
- [x] 工作树干净（除本包新增文件）；已 rebase 到 `origin/master` `434e63f`。
- [x] 未 push（仅 A0 整合/推送）。
- [x] 未自签本人产物；A10 复核矩阵由 A10 负责，缺口已标 `WAITING_DEPENDENCY`。
- [x] CONSUMED_PEERS 使用固定完整 SHA。
- [x] `WAITING_DEPENDENCY` 精确列出 SHA 缺口（A4 `b737e5d` R2 缺失、A10 `b81fa69` R2B findings 缺失）；不重复轮询。
- [x] 测试命令均可实际解析（无双 cargo 过滤伪命令）。
