# Semantic Change Request

> 用途：任何**新增或修改** State / Intent / Owner / Side Effect 之前必须填写。
> 流程：填写 → Reviewer 裁决 →（如需）ADR → 更新 `semantic-registry/*.yaml` → **才允许写业务代码**。
> 命名：复制本模板为 `SCR-<YYYYMMDD>-<name>.md`，放在本目录下。

---

## New Semantic

名称:

类型:

- [ ] State
- [ ] Intent
- [ ] Owner
- [ ] Side Effect

分类（见 `semantic-registry/README.md` 第 3 节）:

- [ ] CURRENT_FACT
- [ ] ACCEPTED_ADR
- [ ] TARGET_CONTRACT
- [ ] PROPOSED_CHANGE
- [ ] KNOWN_DEBT

## Why Existing Semantic Cannot Represent

> 必须先在 Registry 中查过；引用查过的条目名。若只是"没查清"，驳回。

已查询条目:

无法表示的原因:

## Existing Alternatives

> 列出所有相近的既有语义，并说明为何不能用（禁止重复造轮子）。

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
|  |  |  |

## State / Intent / Owner Impact

影响的状态:

影响的 Intent:

Owner 是否变化:

- [ ] 否
- [ ] 是（说明新的 owner 与授权调用方）

## Second Source of Truth Risk

> 关键：是否可能与既有状态/Intent 形成第二真源？

- [ ] 无风险
- [ ] 有风险（说明如何避免：改为派生 / 收敛入口 / 删除旧语义）

说明:

## Checker Impact

需要新增/修改的规则或条目:

- [ ] `states.yaml`
- [ ] `intents.yaml`
- [ ] `owners.yaml`
- [ ] `side-effects.yaml`
- [ ] `check-semantic-registry.mjs`（仅在 YAML 无法表达时才改脚本，需说明理由）

预期 checker 结果（新增后应仍 PASS，或说明新增的 negative fixture）:

## ADR Required

- [ ] 需要（影响语义真源 / Owner / 生命周期）
- [ ] 不需要（纯登记，无语义裁决）

ADR 编号/文件:

## Reviewer Decision

- [ ] APPROVED
- [ ] REJECTED
- [ ] NEEDS REVISION

裁决理由:

是否确认为"当前事实"而非"提案":

- [ ] 是（代码中已存在并可验证）
- [ ] 否（属 PROPOSED_CHANGE，不得当作现状引用）
- [ ] 不适用

Reviewer:

日期:

## 落地检查（合并前）

- [ ] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [ ] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [ ] Registry YAML 已更新
- [ ] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
