# Semantic Change Request — vault 语义 Owner 登记（STAGE I-A）

> 命名：SCR-20260923-vault-owner。本 SCR 对应能力 registry `vault`（原 `notes` id）。
> 流程：本 SCR 为对已在代码中存在的语义 owner 的事实登记（CURRENT_FACT），不引入新语义真源。

---

## New Semantic

名称: vault（笔记库 / Obsidian Markdown 目录浏览、笔记链接图、跟随跳转）

类型:

- [ ] State
- [x] Intent
- [x] Owner
- [ ] Side Effect

分类（见 `semantic-registry/README.md` 第 3 节）:

- [x] CURRENT_FACT（代码中已存在并可验证：`useVaultStore` 是 VaultPanel 唯一消费者与唯一写入者）

## Why Existing Semantic Cannot Represent

已查询条目:

- `capabilities.yaml` 原 `id: notes`：`semanticOwner: null`、`governanceStatus: OWNER_PENDING_SCR`、`status: NOT_INTEGRATED`。
- `FULL-STACK-CAPABILITY-INVENTORY.md` 已知债务 #4：`notes registry owner 偏差：semanticOwner:null 但实际 useVaultStore 存在`。

无法表示的原因:

- 原 registry 把 vault 的语义 owner 留空（`null`），但实际代码中 `useVaultStore`（id=`vault`）是整个 vault 特性的**唯一**状态 owner、唯一 canonical writer、`VaultPanel` 的**唯一**消费者。
- 留空 owner 会令贡献宿主（MainArea）与 ContribRegistry 失去「vault 真源」定位依据，且触发治理 HARD STOP 前置条件（owner 未裁决不得迁移）。本 SCR 将 owner 事实登记，解除 STAGE I-A 阻塞。

## Existing Alternatives

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
| `useVaultStore`（id=vault） | `src/capabilities/vault/state/useVaultStore.ts`（迁入自 `src/stores/useVaultStore.ts`） | 即本 SCR 选定的 owner，唯一真实 owner，无替代 |
| `useNotesStore` / 其它笔记 store | 不存在 | 项目中无第二份笔记状态，不存在竞争真源 |

## State / Intent / Owner Impact

影响的状态:

- `path / root / notes / selected / query / line / anchor / busy / error / warning / choices / sourceMode`（均为 useVaultStore 内部 state，stored）
- 派生状态：`current / results / edges / backlinks`（computed，由 notes + 文本派生）

影响的 Intent:

- `open / pickDirectory / select / follow`（useVaultStore action，唯一 writer）

Owner 是否变化:

- [x] 是（说明新的 owner 与授权调用方）
  - 新 owner：`useVaultStore`（id=`vault`），位于 `src/capabilities/vault/state/useVaultStore.ts`
  - 授权调用方：仅 `src/capabilities/vault/ui/VaultPanel.vue`（经 `src/capabilities/vault/public.ts` 出口）
  - 跨能力消费者：无（grep 确认 `useVaultStore` 仅被 VaultPanel 引用，无任何 bookmark/workspace/file 偷读）

## Second Source of Truth Risk

- [x] 无风险

说明:

- vault 状态只有一个写入入口（`useVaultStore`），且只被 VaultPanel 消费。物理迁入 `capabilities/vault/` 后，owner_implementations locator 指向新路径（旧 `src/stores/useVaultStore.ts` 已删除），不残留第二真源。
- `public.ts` 仅 re-export `useVaultStore`，不构成第二真源（与 `bookmark/public.ts` 同构）。
- 沿袭「ONE SEMANTIC / ONE OWNER」：capability id `vault` == store id `vault` == MainArea view key `vault` == 面板 `VaultPanel`，消除原 `notes`/`vault` 命名不一致。

## Checker Impact

需要新增/修改的规则或条目:

- [x] `states.yaml`（在 `owner_implementations` 增加 `useVaultStore`，paths 指向 `src/capabilities/vault/state/useVaultStore.ts`）
- [ ] `intents.yaml`
- [ ] `owners.yaml`
- [ ] `side-effects.yaml`
- [ ] `check-semantic-registry.mjs`

预期 checker 结果:

- `node scripts/check-semantic-registry.mjs` 无新增 FAIL（owner_implementations 仅新增条目，不改变既有状态真源）
- `node scripts/check-capability-registry.mjs` PASS（`vault` 现已 `GOVERNED` + `semanticOwner` 已填）
- `npm run check` 全绿（已验证：UI-03 基线移除 MainArea 静态渲染 VaultPanel；UI-10 catalog 更新新路径）

## ADR Required

- [ ] 需要（影响语义真源 / Owner / 生命周期）
- [x] 不需要（纯登记，无语义裁决：owner 在代码中已唯一且可验证）

ADR 编号/文件: 不适用（事实登记）

## Reviewer Decision

- [x] APPROVED

裁决理由:

- 代码证据充分：`useVaultStore` 是 vault 特性的唯一状态 owner、唯一 writer、唯一消费者；无任何第二真源或跨能力偷读。
- 不存在「两个同样合理且会改变产品核心语义的 owner」情形（HARD STOP A 不成立）。
- 物理迁移不改变语义 owner（仅移动文件 + 统一命名），符合「先裁决 owner 再搬 UI」。

是否确认为"当前事实"而非"提案":

- [x] 是（代码中已存在并可验证）

Reviewer: autonomous-release-train (STAGE I-A)

日期: 2026-09-23

## 落地检查（合并前）

- [x] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [x] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [x] Registry YAML 已更新（`capabilities.yaml` notes→vault + `states.yaml` owner_implementations）
- [x] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
