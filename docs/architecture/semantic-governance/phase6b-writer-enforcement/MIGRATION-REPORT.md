# Phase 6B — MIGRATION REPORT

> 记录 Phase 6B 从「决策 → Registry 评估 → Checker 实现 → 测试 → 收口」的迁移路径。
> 原则：不重构、不删历史代码、不扩大范围；Registry 是真源，脚本只实现「如何依据 registry 判定」。

## 1. 迁移前状态（Phase 6A 收口）

- `gridSession`（owner=useBrowserStore）写入点仅 `buildGrid` / `forceGridRelayout`，但函数级单写者
  约束**未机器化**（Debt-6A-2）；R8 仅保证声明级唯一 owner。
- `states.yaml` 已含 `canonical_writer` / `forbidden_writers` / `owner` / `derived` /
  `single_owner_required` 字段（Phase 6A 登记）。

## 2. 决策（SCR → ADR → Registry → Checker → Test → Closeout）

| 步骤 | 动作 | 结果 |
|-|-|-|
| SCR | 评估是否需新语义 | 否——既有字段已表达 writer 约束，无需新增 SCR |
| ADR | ADR-SEM-P6B-1（R9 机器强制）/ ADR-SEM-P6B-2（Registry schema 评估：无需扩 YAML） | ACCEPTED |
| Registry | 复用既有 `canonical_writer` / `forbidden_writers` / `owner` | **零改动**（不对正确语义动手）|
| Checker | `check-semantic-registry.mjs` 新增 R9；`check-semantic-closure-logic.mjs` 增强 gridSession writer 断言 | 实现 |
| Test | self-test positive/negative/false-positive + real scan + runtime | PASS |
| Closeout | 5 份文档 + HANDOFF 更新 + tag | 完成 |

## 3. 代码改动清单（仅 checker，零业务代码）

| 文件 | 改动 |
|-|-|
| `scripts/check-semantic-registry.mjs` | 新增 `findFunctionRanges` / `enclosingFunction` 辅助 + `rule9`；`RULES` 数组加入 `rule9`；help 文本补 R9；self-test 增 R9 positive/negative(R9a/b/c)/false-positive 夹具 |
| `scripts/check-semantic-closure-logic.mjs` | 新增同名辅助；新增「gridSession 两处写入均在 canonical_writer 函数内」静态断言（27→原 26 +1）|
| `docs/architecture/semantic-governance/phase6b-writer-enforcement/*.md` | 5 份收口文档 |
| `docs/architecture/HANDOFF_CURRENT_STATE.md` | 更新 Phase 6B 状态 / Commit / Tag / Known Debt |

## 4. 不处理范围（任务 §13）

明确**不**在 Phase 6B 处理：M4 其余 14 域、Terminal 全面治理、Bookmark 新治理、Plugin、MCP、
Clipboard、Script。R9 通过 `single_owner_required` 标志自然将这些域排除在强制之外（不静默消失，
显式登记为 Debt-6A-1）。

## 5. 收口证据

- real scan `fail=0`（与 Phase 6A 一致）。
- self-test `ALL_PASS`（R1..R9）。
- closure-logic `27/27`。
- 未修改 `src/` / `src-tauri/` 任何业务代码。
