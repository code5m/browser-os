# Semantic Change Request — SCR-20260919-terminal

> Phase 4：Terminal Lifecycle 语义治理。本 SCR 汇总本 Phase 新增/冻结的语义。

---

## New Semantic

名称: terminalOpen / termPanes / termGrid / termGridCount / activeTermId / autoConfirmCli / termProbeOn / m0Cfg / m0StartTs / droppedChunks / droppedBytes（State）；addTermPane / killTerm / termWrite / setActiveTerm / restartTerm / bindTermWriter / replayTermHistory（Intent）；terminal（Owner）

类型:

- [x] State
- [x] Intent
- [x] Owner
- [ ] Side Effect

分类:

- [x] CURRENT_FACT（代码中已存在并可验证）
- [ ] ACCEPTED_ADR
- [ ] TARGET_CONTRACT
- [ ] PROPOSED_CHANGE
- [ ] KNOWN_DEBT

## Why Existing Semantic Cannot Represent

已查询条目：Phase 1-3 治理 Browser/Grid/View/Workspace/Bookmark。Terminal 生命周期状态
（termPanes/activeTermId/terminalOpen 等）散落在 useSystemStore.ts，完全未治理。

无法表示的原因：useSystemStore 未被加入 governed_files；终端生命周期无单一入口登记。

## Existing Alternatives

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
| mainView="term" | useLayoutStore | 仅视图导航，非面板注册表/PTY 生命周期 |
| activateTerm | useLayoutStore | 定义为 setView("term")，属导航非面板聚焦 |

## State / Intent / Owner Impact

影响的状态: terminalOpen / termPanes / termGrid / termGridCount / activeTermId / autoConfirmCli / termProbeOn / m0Cfg / m0StartTs / droppedChunks / droppedBytes

影响的 Intent: addTermPane / killTerm / termWrite / setActiveTerm / restartTerm / bindTermWriter / replayTermHistory

Owner 是否变化: 是（新增 terminal = useSystemStore）

## Second Source of Truth Risk

- [x] 有风险（说明如何避免）

说明：termPanes 必须为唯一面板注册表；任何第二份 {id,cwd}[] 或裸 activeTermId 直写都会漂移。
本 SCR 通过 R2（useSystemStore.ts 全部 16 声明登记）禁止第二注册表；activeTermId 经 killTerm 重置。

## Checker Impact

需要新增/修改的规则或条目:

- [x] `states.yaml`（governed_files + 11 terminal states + observed_not_governed 5）
- [x] `intents.yaml`（7 terminal intents + duplicate_names；rejected 组件直写 termPanes）
- [x] `owners.yaml`（terminal owner + violation_patterns COMPONENT_WRITES_TERMINAL）
- [ ] `side-effects.yaml`（仅文档化 termProcess，未改 requires_declaration）
- [x] `check-semantic-registry.mjs`（R2/R4 域扩展；R6 复用）

预期 checker 结果：--self-test ALL_PASS；真实仓库 fail=0（termPanes 派生无违规）。

## ADR Required

- [ ] 需要
- [x] 不需要（纯登记 + 注册表护栏，无语义裁决冲突）

ADR 编号/文件: 无（沿用 ADR-P1A-3 派生量禁止存储原则；生命周期由 check-terminal-policy.py 覆盖）

## Reviewer Decision

- [x] APPROVED
- [ ] REJECTED
- [ ] NEEDS REVISION

裁决理由：useSystemStore 内终端生命周期状态（termPanes/activeTermId/terminalOpen 等）为真实存在；
冻结为单一 owner + 注册表护栏可防第二面板真源；不修改业务代码，零回归风险。

是否确认为"当前事实"而非"提案":

- [x] 是（代码中已存在并可验证）

Reviewer: autonomous-phase-executor

日期: 2026-09-19

## 落地检查（合并前）

- [x] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [x] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [x] Registry YAML 已更新
- [x] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
