# Semantic Change Request — SCR-20260919-bookmark

> Phase 3：Bookmark 语义治理。本 SCR 汇总本 Phase 新增/冻结的语义。

---

## New Semantic

名称: items / loaded / busy / error / panelOpen / sorted（State）；add / remove / toggle / importFile / togglePanel（Intent）；bookmark（Owner）

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

已查询条目：Phase 1 states.yaml 仅 Browser/Grid；Phase 2 加 Workspace/FilePanel。Bookmark 域完全未治理。

无法表示的原因：Bookmark 收藏夹真源（useBookmarkStore.items，后端持久化）未在任何 registry 条目中。

## Existing Alternatives

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
| useHomeStore 主页快捷方式 | useHomeStore | localStorage、仅本机，与后端收藏夹生命周期不同；rejected 合并 |
| filePath 等 | useWorkspaceStore | 文件路径，非网页收藏 |

## State / Intent / Owner Impact

影响的状态: items / loaded / busy / error / panelOpen / sorted（derived）

影响的 Intent: add / remove / toggle / importFile / togglePanel

Owner 是否变化: 是（新增 bookmark = useBookmarkStore）

## Second Source of Truth Risk

- [x] 有风险（说明如何避免）

说明：items 必须为唯一收藏列表真源；任何第二份 Bookmark[] 或裸 url 收藏判定都会漂移。
本 SCR 通过 R2（useBookmarkStore.ts 全部 6 声明登记）禁止第二列表；sorted 经 R6 固化为 derived。

## Checker Impact

需要新增/修改的规则或条目:

- [x] `states.yaml`（governed_files + 6 states）
- [x] `intents.yaml`（5 intents + duplicate_names + rejected mergeBookmarksIntoHome）
- [x] `owners.yaml`（bookmark owner + violation pattern）
- [ ] `side-effects.yaml`（仅文档化 bookmarkPersist，未改 requires_declaration）
- [x] `check-semantic-registry.mjs`（R2/R3/R4 域扩展；R6 复用 sorted derived）

预期 checker 结果：--self-test ALL_PASS；真实仓库 fail=0（sorted 派生无违规）。

## ADR Required

- [ ] 需要
- [x] 不需要（纯登记 + 派生不变量护栏，无语义裁决冲突）

ADR 编号/文件: 无（沿用 ADR-P1A-3 派生量禁止存储原则）

## Reviewer Decision

- [x] APPROVED
- [ ] REJECTED
- [ ] NEEDS REVISION

裁决理由：useBookmarkStore 内 items/loaded/busy/error/panelOpen/sorted 为真实存在的收藏夹状态；
冻结为单一 owner + 派生不变量可防第二收藏源；不修改业务代码，零回归风险。

是否确认为"当前事实"而非"提案":

- [x] 是（代码中已存在并可验证）

Reviewer: autonomous-phase-executor

日期: 2026-09-19

## 落地检查（合并前）

- [x] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [x] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [x] Registry YAML 已更新
- [x] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
