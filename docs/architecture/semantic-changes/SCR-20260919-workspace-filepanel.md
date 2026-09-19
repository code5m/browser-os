# Semantic Change Request — SCR-20260919-workspace-filepanel

> Phase 2：Workspace / FilePanel 语义治理。本 SCR 汇总本 Phase 新增/冻结的语义。

---

## New Semantic

名称: filePath / inlineFile / previewDir / pathInput / currentLocalPath（State）；openFile / openFileInline / enterDir / saveFile / closeFileEditor（Intent）；workspace_filepanel（Owner）

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

已查询条目：Phase 1 states.yaml 仅含 Browser/Grid + View Navigation；intents.yaml 无 workspace 域；
owners.yaml 无 workspace_filepanel。Workspace/FilePanel 的「当前位置」概念完全未治理。

无法表示的原因：Phase 1 治理域未覆盖 useWorkspaceStore.ts。

## Existing Alternatives

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
| mainView (files/editor) | useLayoutStore | 只表达“视图”，不表达“文件路径” |
| desiredGridVisibility | useBrowserStore | 派生量模板，非文件位置 |

## State / Intent / Owner Impact

影响的状态: filePath / inlineFile / previewDir / pathInput / currentLocalPath（derived）

影响的 Intent: openFile / openFileInline / enterDir / saveFile / closeFileEditor

Owner 是否变化: 是（新增 workspace_filepanel = useWorkspaceStore）

## Second Source of Truth Risk

- [x] 有风险（说明如何避免）

说明：currentLocalPath 曾可能被误存为第二真源。本 SCR 通过 R6 将其固化为 derived（computed），
禁止 `.value =` 与 `ref/reactive` 声明。filePath 为唯一地址真源，inlineFile/previewDir 为隔离态。

## Checker Impact

需要新增/修改的规则或条目:

- [x] `states.yaml`（governed_files + 5 states + observed_not_governed）
- [x] `intents.yaml`（5 intents + duplicate_names）
- [x] `owners.yaml`（workspace_filepanel）
- [ ] `side-effects.yaml`（仅文档化 writeFile，未改 requires_declaration）
- [x] `check-semantic-registry.mjs`（R2/R3/R4 域扩展 + 新增 R6）

预期 checker 结果：--self-test ALL_PASS；真实仓库 fail=0（R6 对 currentLocalPath 无违规）。

## ADR Required

- [ ] 需要
- [x] 不需要（纯登记 + 派生不变量护栏，无语义裁决冲突）

ADR 编号/文件: 无（沿用 ADR-P1A-3 派生量禁止存储原则）

## Reviewer Decision

- [x] APPROVED
- [ ] REJECTED
- [ ] NEEDS REVISION

裁决理由：useWorkspaceStore 内 filePath/inlineFile/previewDir/currentLocalPath/pathInput 为真实存在的
文件导航状态；冻结为单一 owner + derived 不变量可防第二真源；不修改业务代码，零回归风险。

是否确认为"当前事实"而非"提案":

- [x] 是（代码中已存在并可验证）

Reviewer: autonomous-phase-executor

日期: 2026-09-19

## 落地检查（合并前）

- [x] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [x] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [x] Registry YAML 已更新
- [x] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
