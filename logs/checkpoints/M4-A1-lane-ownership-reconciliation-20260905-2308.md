# M4 Lane 归属漂移清理（Lane A1 · 第二批）

> 执行者：Lane A1（CodeBuddy 会话，Hy4 / 腾讯混元），`IMPLEMENTER_MODEL=CodeBuddy Hy4`
> 时间：2026-09-05 23:08 CST　基线：`85d2d7b`
> 触发：重读 `WORKSPACE_IDENTITY.md` + `PARALLEL_COMMAND_BOARD.md`（23:55 版）后按 Batch Implementation Dispatch 领取任务
> 性质：**纯文档，产品代码零改动**（`src/` `src-tauri/` `scripts/` `package*.json` 一律未触碰）
> 状态：`PASS_WITH_DEBT`（本卡范围内的漂移已清；残留 3 项需 A0/A2 处置，见 §5）

---

## 1. 为什么 A1 这一批**没有**做「完整代码包」

用户提示词为指挥板 §One-Line Resume Prompt 的通用模板（「按 Batch Implementation Dispatch 做完整代码包」）。逐条比对最新指挥板后，**A1 无法按字面执行**，依据是指挥板自身的硬停规则：

| 硬停依据 | 指挥板原文位置 | 实测事实 |
|---|---|---|
| 任务要求改 Allowed Scope 外的文件 | Startup Gate 硬停第 57 行「The task would require changing files outside the lane's allowed scope」 | A1 Allowed Scope = `AI-模型切换与接手清单.md` / `详细设计与实施计划.md` / `后续需求TODO.md` / `logs/checkpoints/`。「完整代码包」= `src/`+`src-tauri/`+`scripts/`，**全部越界** |
| 工作树被其他 Lane 占用 | Startup Gate 硬停第 56 行「The worktree is dirty with changes from another lane」 | 开工时工作树含 **A6**（`scripts/pre-merge.sh`、`scripts/check-scheduler-policy.py`、`logs/checkpoints/A6-M4-5.d-20260905-2350.md`）、**A11**（`logs/assist/M4-A11-debt-ledger-*`、`logs/checkpoints/M4-A11-verification-matrix-*`）、**A10**（`logs/assist/A10-M4-security-recheck-A1A2A6-*`）。A1 若动这些文件即为覆盖他人改动 |
| Batch Implementation Dispatch **无 A1 条目** | §Batch Implementation Dispatch 仅覆盖 A2~A12 | 指挥板第 110 行明确：**「A1 M4 expansion is integrated.」** |
| 不得混 Lane 范围 | 第 61 行「Do not blend scopes. Stop and report the conflict unless the controller has already corrected the lane」 | A1 的 Must Deliver（M4-1~M4-8 展开卡 + 依赖图）已由 A0 于 `6d73ce1` 集成完毕 |
| 不是产品代码 Lane | §Batch Implementation Rule 第 33 行「A **product-code lane** must not stop after only a note」 | A1 的 Purpose = 「M4 task-card expansion」，非产品代码 Lane，第 33 行不适用 |

**结论：A1 不写代码、不越界。改为执行「A1 职责内剩余且当前唯一有价值的工作」——清理 A0 覆盖裁决后残留在三份主文档里的 Lane 归属漂移。**

---

## 2. A0 覆盖裁决（2026-09-05 23:30）与 A1 原裁决的冲突

A0 在 `logs/checkpoints/M4-20260905-2225.md` 顶部追加了覆盖裁决，**推翻 A1 上一批的 R-A1-1**：

| 项 | A1 原裁决（22:25） | **A0 覆盖裁决（23:30，现行）** |
|---|---|---|
| SQL 风险分类器 / 生产判定 | 拆为 `M4-2.s`，归 **A4**，merge 3，先于 A3 | 归 **A3**，作为 M4-2 全卡的一部分，merge 3 |
| `M4-2` 连接池 / 查询核心 | 归 A3（`M4-2.a~d`），merge 4 | 仍归 **A3**，合并为 M4-2 全卡，merge 3 |
| `M4-3` 命令层 | 归 A3（与 M4-2 同 Lane） | 归 **A4**，merge 4 |
| `M4-2.s` 编号 | 新建 | **作废** |

**A1 接受该覆盖裁决，不再争 R-A1-1。** 与最新指挥板 Lane 表（A3 行含 `src-tauri/src/security_policy.rs`、A4 行为命令层）及 §Code Dispatch Now（A4 行「First implement M4-2.s safety gate first」）逐条比对，A0 口径自洽。

---

## 3. 本次清理的漂移（8 处，全部在 A1 允许范围内）

A0 的覆盖裁决只改了两处：`logs/checkpoints/M4-20260905-2225.md` 顶部加注、`后续需求TODO.md` 第 9 行加注。**A1 上一批写进三份主文档的归属表述未同步**，会导致 A3/A4 按相反指引施工。本次逐处修正：

| # | 文件 | 位置 | 修正前（陈旧） | 修正后 |
|---|---|---|---|---|
| 1 | `详细设计与实施计划.md` | L2 本版变更 ③ | 库链 `M4-1 → M4-2.s(A4) → M4-2.a~d(A3) → M4-3.a~d(A3) → M4-4(A5)` | 库链 `M4-1(A2) → M4-2(A3) → M4-3(A4) → M4-4(A5)`，并注明 A3 含分类器/生产判定 |
| 2 | `详细设计与实施计划.md` | L2 本版变更 ④ | 「裁决 R-A1-1…以内容为据、编号沿用 WBS，前置切片编号 `M4-2.s`」 | `~~裁决 R-A1-1~~ 已被 A0 覆盖裁决推翻`；`M4-2.s` 编号即日作废 |
| 3 | `详细设计与实施计划.md` | L45 里程碑表 | `A4=M4-2.s / A3=M4-2,M4-3` | `A3=M4-2（merge 3）/ A4=M4-3（merge 4）`，并补 A9~A12 共 12 Lane |
| 4 | `详细设计与实施计划.md` | L456 M4-2 主卡 | 「子卡 M4-2.s（Lane A4…）+ M4-2.a/b/c/d（Lane A3，merge 4）」 | `M4-2.s 已作废`；`M4-2.a/b/c/d 全部归 A3，merge 3` |
| 5 | `详细设计与实施计划.md` | L457 M4-3 主卡 | **「子卡 M4-3.a/b/c/d（Lane A3，merge 4）」** ← 高危 | **「子卡 M4-3.a/b/c/d（Lane A4，merge 4）」** |
| 6 | `AI-模型切换与接手清单.md` | L27 下一检查点 | `M4-2.s(A4，merge 3) → M4-2.a~d + M4-3.a~d(A3，merge 4)`；「10 Lane」 | `M4-2(A3，merge 3) → M4-3.a~d(A4，merge 4)`；「12 Lane（含 A10/A11/A12）」 |
| 7 | `AI-模型切换与接手清单.md` | L28 下一任务路由 | 「M4-2：M4-2.s `AI:DEEP / R:xhigh`、M4-2.a~d…」 | 「M4-2：`AI:DEEP / R:xhigh`（A3 全卡…`M4-2.s` 不再单列路由）」 |
| 8 | `后续需求TODO.md` | L36 ③ / L66 库链 | `M4-1(A2) → M4-2.s(A4) → M4-2.a~d(A3) → M4-3.a~d(A3) → M4-4(A5)` | `M4-1(A2) → M4-2(A3) → M4-3(A4) → M4-4(A5)` |

**第 5 处是本次最高危项**：原文把 M4-3 命令层判给 A3，与 A0 覆盖裁决（A4=命令层）**正好相反**。A3 与 A4 若各自照主文档施工，会在 `bridge.rs` / `main.rs` / ACL 三个高冲突文件上直接撞车。

---

## 4. 自验证

```text
git status --short --branch          → ## master...origin/master [领先 2]；仅 A1 的 3 份主文档被改
python3 scripts/check-plan-routing.py → EXIT=0，ok (50 WBS rows)
git diff --check                      → EXIT=0
grep "A4=M4-2|A4 先于 A3|A3 依赖 A4"   → 三份主文档零命中
grep "M4-2.s"                         → 仅剩 7 处「作废 / 覆盖裁决」说明性引用，无施工性指向
产品代码零改动                          → git status 过滤 src/ src-tauri/ scripts/ package* → 空
未提交、未 push                        → git log -1 仍为 85d2d7b
```

`bash scripts/pre-merge.sh` **本轮未跑**：当前工作树含 A6 对 `scripts/pre-merge.sh` 与 `scripts/check-scheduler-policy.py` 的未提交改动，跑门禁会把 A6 的中间态计入结果，也会与 A6 争用同一文件；A1 的改动是纯 Markdown 文档，不影响门禁所校验的任何脚本/代码产物。**建议 A0 在 A6 落地 pre-merge 改动后统一复跑**（见 §5）。

---

## 5. 残留风险与移交

| 编号 | 事项 | 归属 |
|---|---|---|
| **O-A1-8** | **M4-1.c 仍是 `STOPPED_EMPTY_ARTIFACT`**（`logs/checkpoints/M4-1.c-20260905-2255.md` L126/L134）。该文件体量 8.5 KB、含完整裁定书正文，但结论段标 STOPPED——属 A2 的活（指挥板 §Code Dispatch Now A2 行），**A1 不得代写**。A3「LIMITED START」与 A4「START」都被它卡着 | **A2 → A3/A4** |
| **O-A1-9** | 指挥板 §Code Dispatch Now A4 行仍写「First implement **M4-2.s** safety gate first」，用的是**已作废的编号**，与同板 Lane 表（A3=安全策略）自相矛盾。建议 A0 把该行改为「M4-2 safety gate 归 A3；A4 仅做 M4-3 命令层」 | **A0** |
| **O-A1-10** | A3 与 A4 的 Allowed Scope 都含 `src-tauri/src/database*`（A3 行 84、A4 行 85），且 A3 含 `security_policy.rs`。若两侧同时新建/改 `database.rs`，会直接冲突。建议 A0 明确「A3 先建 `database.rs` 并冻结内部 API，A4 只消费不创建」 | **A0 → A3/A4** |
| **O-A1-3（沿用，未决）** | M4 验收门禁要求三库各有连接/只读查询/断开/失败用例；无真实 MySQL/PG 实例时只能 `SKIPPED`，需 A0/负责人裁定接受还是补实例。**不得伪造 PASS** | **A0 / 负责人** |

---

## 6. 交付形态

按 §Batch Implementation Rule 第 7 条，A1 留下**单一补丁 + 检查点**：

- 检查点：本文件 `logs/checkpoints/M4-A1-lane-ownership-reconciliation-20260905-2308.md`
- 补丁：`logs/checkpoints/Lane-A1-lane-ownership-reconciliation-20260905-2308.patch`
- 未提交、未 push；不跨 Lane 改文件
