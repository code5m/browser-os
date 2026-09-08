# A11 · M5-W18-R2B 进度与验收总包（滚动更新 2）

```text
LANE=A11
DISPATCH=M5-W18-R2B
STATUS=READY_FOR_REVIEW
  - W18-R2B 全 11 lane 整包已完成（A4 R2B 证据闭环、A10 R2B 独立复核均已补齐）
  - 残留“跨 lane 待 A0 裁决”项不阻塞 A11 交付，仅作为 W19 开放前的决策输入
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a11/mvp-browser-os-v3
BRANCH=codex/m5-w18-a11
BASE=434e63f  (origin/master: "docs(M5-W18): define daily workbench and second research dispatch")
HEAD=f43196dc7aabfb24d22262850320f18204e7d7ce
NO_PRODUCT_CODE=true
NO_PUSH=true
```

> 本文件为第 2 次滚动审核产出：相对首版，A1–A10 全部推进到 R2B，A4/A10 两项 `WAITING_DEPENDENCY` 缺口均已闭合。首版历史保留，不重写。

## 0. 本包交付物（A11 允许范围）

1. `A11-R2B-progress.md`（本文件，滚动更新）
2. `A11-R2B-integration-manifest.json`
3. `A11-R2B-A0-brief.md`（≤120 行）
4. `A11-R2B-S0-S5-task-cards.md`（候选卡 + 测试矩阵，PROPOSED；未改）
5. 旧 `A11-verification-architecture-R2-20260908.md` §10 已更正：不再声称“peer 无 R2”，并补注 A4/A10 已补 R2B。

## 1. 逐 lane 状态（固定 SHA，均经 `git show <sha>:<path>` 只读消费）

| Lane | HEAD (full SHA) | R2→R2B 状态 | 新增 R2B 交付 | 缺口 / 债务 | 依赖 |
|---|---|---|---|---|---|
| A1 | eba0d52f0882b3a6a648fc7c8c75c23cec93a692 | R2B_READY_FOR_REVIEW | `A1-workbench-shell-R2B-20260908.md`、`A1-wireframe-prototype-R2B.html`、`A1-checkpoint-R2B-20260908.md` | W17-D1~D6（D5 原生视觉 headless BLOCKED） | A0 清 W17 债 |
| A2 | 1eb86084ebacded1057b85e49f103d48b5c8f16a | R2B_PASS_WITH_DEBT | `A2-R2B-make_vault.py`/`A2-R2B-resolve.py`(19/19 PASS)、`A2-R2B-note-semantics-evidence.md`（按 A0 audit #4 撤回部分 R2 主张） | B1(节点上限)、B2 | A1 容量决策 |
| A3 | ab455c1db9d3fe732cdff1aa95a2dff9c51955e6 | R2B_PASS_WITH_DEBT | （本轮未动）图谱/笔记导航，`check-graph-ui-logic` 113 pass | 待 A2 W19 接入 | A2、A1 |
| A4 | 7f282d0b9891deadd1e2d3be628361794c3b33b9 | R2B_READY_FOR_REVIEW | `A4-R2B-backend-evidence-closure.md`、`A4-M5-W18-R2B-checkpoint.md` | **原缺口已闭**：DbValue 根因(B8-1)、23 `#[test]` 声明、dbx 身份=t8y2/dbx | — |
| A5 | 94c988bdcc92b5b73415475bf14a8d50d0a9baf0 | R2B_READY_FOR_REVIEW | `A5-R2B-design.md`、`A5-R2B-state-model.mjs`、`A5-R2B-wireframe.html`、`A5-checkpoint-R2B-20260908.md` | 依赖 A4 DTO、A6 生命周期 | A4、A6 |
| A6 | ac239538e6185b595784ddbfbafd0d1d5ce27263 | R2B_PASS_WITH_DEBT | `A6-R2B-resource-lifecycle-recovery.md`、`A6-checkpoint-R2B-20260908.md` | 多文档结果策略待 A5 定稿；**A4 R2B 合同现已到位（D1 解）** | A4、A5 |
| A7 | 37be95eed91f27526615a3c50b90fc3c376b8f0a | R2B_PASS_WITH_DEBT | `A7-R2B-retrieval-access-index-transaction.md`、`A7-checkpoint-R2B-20260908.md`（§9 R2B 更正；与 A8 追平） | 缓存命名空间冲突待 A10 台账 | A10、A9 |
| A8 | ed5c0c8a8c1e0e3490a34cc85dfc9227f519bd8a | R2B_READY_FOR_REVIEW | `A8-R2B-retrieval-metrics-correction.md`、`A8-R2B-benchmark-metrics.mjs`（修正 A0 #7/#8 指标名/RSS 边界） | 与 A7 已追平 | A1/A3/A10 |
| A9 | 66bd51140dc07829840ada02deb2306762d4d72a | R2B_PASS | `A9-R2B-capability-budgets.md`、`A9-R2B-trust-boundary.md`、`A9-R2B-webview-boundary.md`、`A9-checkpoint-R2B-20260908.md` | B2/B3/B7 仍 W19 前必修 | A3 采纳裁决(无依赖直至 A0) |
| A10 | 530d7086c2e4f33c79f06f3646741888aa3a675b | R2B_READY_FOR_REVIEW | **`A10-R2B-review-findings.md`**（独立复核，F1–F5）、ledger §9 Z8 和解 | **原缺口已闭** | A1–A9 |
| A11 | f43196dc7aabfb24d22262850320f18204e7d7ce | R2B_READY_FOR_REVIEW | 本包（progress/manifest/brief/cards/checkpoint） | 无 | 全部 |

## 2. 场景分列（不虚构总进度条）

- **Design（研究/设计）**：A1 壳层+线框、A2 语义+合成夹具、A3 导航、A4 后端契约、A5 设计+状态模型、A6 生命周期、A7 检索接入、A8 指标修正、A9 能力预算/信任/WebView、A10 复核台账 —— 全部 RESEARCH 完成，标记 READY_FOR_REVIEW / PASS_WITH_DEBT。
- **Code（产品代码）**：全部 `PROPOSED_NOT_AUTHORIZED`；W19 切片 S0–S5 见 `A11-R2B-S0-S5-task-cards.md`。本包及所有 peer 包均未写产品代码。
- **Native（原生验收）**：仅 A3/A5/A8/A9 有合成/脚本验证；真实 GUI/IPC/debug-release 均 **NOT_RUN**（研究边界，W19 才跑）。
- **User-trial（用户试用）**：全部 **NOT_RUN**；蓝图 §9 明确 J1–J6 状态 `UNVERIFIED`，不等同功能未实现。

## 3. 跨 lane 待 A0 裁决差异（源自 `A0-M5-W18-R2B-dispatch-20260908.md` §3，本轮已消项标注）

1. ~~参考身份未对齐~~ → **已解**：A4 R2B 确认本地 dbx = `t8y2/dbx`（Apache-2.0），非 IDEA；IDEA 身份 UNKNOWN，通用查询 UX 研究继续，精确复刻声明保持 UNKNOWN。
2. A0 自身数字更正：`database.rs` 实际 **23** 行独立 `#[test]`（R1 audit 22、A4 R1“zero”均错）→ A10 F2 已独立确认 23；真实缺口是 live-DB 集成测试。
3. 前端测试口误：A1/A11“0 frontend”未覆盖既有 `scripts/check-*-ui-logic.mjs` → 应说“缺某框架”而非“无”。
4. ~~A2 行为证据混淆~~ → A2 已按 A0 audit #4 **撤回部分 R2 推断**（alias/同名/大小写），补可复现夹具 19/19。
5. A7 接入结论：跨绑定互读/崩溃实测未提供 → 仍待 A7/A9/A10 复核（A8 R2B 已与 A7 追平）。
6. A7 缓存命名冲突（`.zvec-grep` 根清理）→ 仍待改为产品自有缓存命名空间 + 代际提交（A10 台账）。
7. ~~A8 指标名/perf 边界~~ → **已修**：A8 R2B `retrieval-metrics-correction.md` + `benchmark-metrics.mjs` 修正 A0 #7/#8（precision/recall 命名、RSS 边界）。
8. ~~A7/A8 矛盾~~ → **已追平**：A8 R2B 明确 npm N-API sidecar 路线，与 A7 一致；Z8 HOLD-(a) 由 A10 F5 解除（官方 `zvec-rust` crate 已证，ADAPT-sidecar，双方 Apache-2.0 无许可冲突）。
9. 许可：产品根 LICENSE = **MulanPSL-2.0**（非 Apache-2.0）；dbx/部分上游 Apache-2.0，移植署名归 A10。

### A10 独立复核要点（F1–F5，A11 不重复裁决）
- F1 DbValue 序列化 ✅ 独立确认（`database.rs:118-168`）。
- F2 DB 测试数 = **23**（非 22、非“zero”）；真实缺口 = live-DB 集成测试。
- F3 同步驱动模型 ✅ 确认（无 tokio 第二运行时）。
- F4 dbx 上游 pin `c0a7be12` ✅ 一致（忽略 R1 误 `01a6e16`）。
- F5 zvec 绑定/格式 ✅ 确认；修正 A10 Z8；npm N-API sidecar；原生占用 `.node` 41.7MB + 引擎 RSS ~294MB（bundle 影响交 A3/A10 评估）。

## 4. S0–S5 候选卡与测试矩阵

见 `A11-R2B-S0-S5-task-cards.md`（全部 `PROPOSED_NOT_AUTHORIZED`，命令均可解析，未实际运行；本轮未改动）。

## 5. A0 摘要

见 `A11-R2B-A0-brief.md`（≤120 行）。

## 6. CONSUMED_PEERS（固定 SHA，只读消费）

- A1 `eba0d52f0882b3a6a648fc7c8c75c23cec93a692` → `A1-workbench-shell-R2B-20260908.md`, `A1-wireframe-prototype-R2B.html`, `A1-checkpoint-R2B-20260908.md`
- A2 `1eb86084ebacded1057b85e49f103d48b5c8f16a` → `A2-R2B-make_vault.py`, `A2-R2B-resolve.py`, `A2-R2B-run-20260908.out`, `A2-R2B-note-semantics-evidence.md`
- A3 `ab455c1db9d3fe732cdff1aa95a2dff9c51955e6` → `A3-R2B-graph-note-nav-20260908.md`, `A3-M5-W18-R2B-20260908.md`
- A4 `7f282d0b9891deadd1e2d3be628361794c3b33b9` → `A4-R2B-backend-evidence-closure.md`, `A4-M5-W18-R2B-checkpoint.md`
- A5 `94c988bdcc92b5b73415475bf14a8d50d0a9baf0` → `A5-R2B-design.md`, `A5-R2B-state-model.mjs`, `A5-R2B-wireframe.html`, `A5-checkpoint-R2B-20260908.md`
- A6 `ac239538e6185b595784ddbfbafd0d1d5ce27263` → `A6-R2B-resource-lifecycle-recovery.md`, `A6-checkpoint-R2B-20260908.md`
- A7 `37be95eed91f27526615a3c50b90fc3c376b8f0a` → `A7-R2B-retrieval-access-index-transaction.md`, `A7-checkpoint-R2B-20260908.md`
- A8 `ed5c0c8a8c1e0e3490a34cc85dfc9227f519bd8a` → `A8-R2B-retrieval-metrics-correction.md`, `A8-R2B-benchmark-metrics.mjs`, `A8-checkpoint-R2B-20260908.md`
- A9 `66bd51140dc07829840ada02deb2306762d4d72a` → `A9-R2B-capability-budgets.md`, `A9-R2B-trust-boundary.md`, `A9-R2B-webview-boundary.md`, `A9-checkpoint-R2B-20260908.md`
- A10 `530d7086c2e4f33c79f06f3646741888aa3a675b` → `A10-R2B-review-findings.md`, `A10-source-transplant-ledger.md`(更新 Z8+§9)
- A0 调度文档（origin/master `434e63f`）：`WORKBENCH_BLUEPRINT-20260908.md`、`A0-HANDOFF-LOW-COST-20260908.md`、`A0-M5-W18-R2B-dispatch-20260908.md`、`M5-W18-R2B-TASKS-20260908.md`、`PARALLEL_COMMAND_BOARD.md`、`WORKSPACE_IDENTITY.md`

## 7. 自检（交付前）

- [x] 未改任何产品代码 / `src` / `src-tauri` / `scripts` / ACL / 其他 lane / 主矩阵。
- [x] 工作树干净（除本包新增文件）；已 rebase 到 `origin/master` `434e63f`。
- [x] 未 push（仅 A0 整合/推送）。
- [x] 未自签本人产物；A10 复核由 A10 负责并已产出 findings。
- [x] CONSUMED_PEERS 使用固定完整 SHA（本轮全部重消费）。
- [x] 两项 `WAITING_DEPENDENCY`（A4 R2、A10 R2B findings）**均已闭合**，整包升级为 `READY_FOR_REVIEW`。
- [x] 测试命令均可实际解析（无双 cargo 过滤伪命令）。
