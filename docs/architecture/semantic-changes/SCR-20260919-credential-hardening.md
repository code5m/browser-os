# Semantic Change Request — SCR-20260919-credential-hardening

> Phase 5.1：Credential Security Hardening。本 SCR 汇总本 Phase 对凭据域的三项加固
> （Owner 通用化 / 敏感输入护栏 / Keyring 副作用契约），收口 Debt-5-1/5-2/5-3。

---

## New Semantic

名称:
- (A) 通用化 `credential` Owner 越界检查（R3 从 Browser/Grid 扩到 Credential）
- (B) State `databaseCredentialInput`（sensitive: true，owner: credential）；Side Effect `sensitiveInput`（forbidden: log/plainStorage/export；allowed: secureCredentialFlow）
- (C) Side Effect `keyringRead`（level: critical，owner: credential，forbidden: frontend_return/log/clipboard）

类型:

- [x] State（B）
- [ ] Intent
- [x] Owner（A：通用化既有 credential owner 的越界检查）
- [x] Side Effect（B: sensitiveInput；C: keyringRead）

分类:

- [x] CURRENT_FACT（代码中已存在并可验证：KeyringStore 薄封装 + 三命名空间 + CredentialList 安全红线）
- [ ] ACCEPTED_ADR
- [ ] TARGET_CONTRACT
- [ ] PROPOSED_CHANGE
- [ ] KNOWN_DEBT

## Why Existing Semantic Cannot Represent

Phase 5 已登记 `credential` owner 与三态（gitRepoToken/dbCredential/browserCredential），但：
- Debt-5-2：R3 仅硬编码 Browser/Grid，未对 `credential` owner 通用化 → 组件直调 keyring 原语无机器护栏。
- Debt-5-1：`DatabasePanel.vue` 的 `password` 瞬时表单字段未被任何静态护栏覆盖 → 潜在泄露无阻断。
- Debt-5-3：`keyringWrite`/`keyringDelete` 仅文档登记（requires_declaration=false），无机器约束；
  keyring 读出的秘密不得回传前端/日志/剪贴板，缺独立数据流门禁。

## Existing Alternatives

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
| owners.yaml `FRONTEND_CREDENTIAL_LEAK` violation_pattern | owners.yaml | 仅为文档声明，R3 未读它（未武装，故 Debt-5-2） |
| side-effects.yaml `keyringWrite`/`keyringDelete` | side-effects.yaml | 仅文档级登记，无 requires_declaration 机器约束（故 Debt-5-3） |
| redact.ts 前端脱敏 | src/utils/redact.ts | 展示层兜底，非凭据输入/keyring 数据流护栏 |

## State / Intent / Owner / Side Effect Impact

影响的状态: databaseCredentialInput（新增，sensitive；Debt-5-1 收口）
影响的 Intent: 无新增（复用 Phase 5 的 save/get/delete/fill/listBrowserCredentials）
Owner 是否变化: 否（`credential` = KeyringStore 已存在；本 SCR 仅将其越界检查通用化到 R3）
影响的 Side Effect: 新增 sensitiveInput（B）、keyringRead（C）

## Second Source of Truth Risk

- [x] 有风险（说明如何避免）

说明：凭据真源 = 系统密钥库，本就不进应用内存（天然无第二内存真源）。本 Phase 风险点是
**前端**误把敏感输入/keyring 秘密流入泄露汇。规避：
- (A) R3 通用化：`credential` owner_only_api（save_token/get_token/delete_token）禁止前端任何文件直调；
  组件只经 bridge 意图入口（fillBrowserCredential/listBrowserCredentials/db_connect）。
- (B) R7：sensitive: true 状态的前端标识符（password）不得流入 console/localStorage/export；
  允许 db.connect(password.value) 安全凭据流与 password.value="" 清空。
- (C) check-sensitive-side-effects.mjs：keyring 秘密不得出现在回传前端的 DTO（Serialize 结构不得含
  password 字段）；rejected intent exposePassword/copyPassword/exportCredential 阻断级。

## Checker Impact

需要新增/修改的规则或条目:

- [x] `owners.yaml`（A：更新 `credential` owner rationale，移除"R3 未通用化"旧注；补 `manages` 明确 keyring 生命周期）
- [x] `states.yaml`（B：新增 databaseCredentialInput；observed_not_governed 标记 Debt-5-1 已收口）
- [x] `side-effects.yaml`（B: sensitiveInput；C: keyringRead）
- [x] `check-semantic-registry.mjs`（A：R3 通用化 credential owner；B：新增 R7 敏感输入泄露；夹具）
- [x] `check-sensitive-side-effects.mjs`（C：新增独立门禁脚本，--help/--self-test/--json/--strict）
- [x] `pre-merge.sh`（C：Phase 03 门禁循环 + --self-test 接入 check-sensitive-side-effects.mjs）

预期 checker 结果：
- check-semantic-registry.mjs --self-test ALL_PASS（含 R3 credential / R7 敏感输入 NEG + 安全流 FP）
- check-sensitive-side-effects.mjs --self-test ALL_PASS（含 DTO 含 password 的 NEG + BrowserCredentialItem 无 password 的 POS）
- 真实仓库 fail=0（无 exposePassword/copyPassword/exportCredential；BrowserCredentialItem 无 password 字段；
  DatabasePanel.password 仅走 db.connect + 清空，无泄露汇）

## ADR Required

- [ ] 需要
- [x] 不需要（纯加固 + 注册表护栏，无语义裁决冲突；沿用 CredentialList.vue 既有安全红线与 ADR-P1A-3）

ADR 编号/文件: 无

## Reviewer Decision

- [x] APPROVED
- [ ] REJECTED
- [ ] NEEDS REVISION

裁决理由：三项加固均为"凭据真源=系统密钥库、前端不持原始值"契约的机器化收口，不修改业务代码，
零回归风险； Debt-5-1/5-2/5-3 经本 Phase 显式收口（非隐藏）。

是否确认为"当前事实"而非"提案":

- [x] 是（代码中已存在并可验证）

Reviewer: autonomous-phase-executor

日期: 2026-09-19

## 落地检查（合并前）

- [x] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [x] `node scripts/check-sensitive-side-effects.mjs --self-test` PASS
- [x] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [x] Registry YAML 已更新
- [x] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
