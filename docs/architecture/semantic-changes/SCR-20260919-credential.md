# Semantic Change Request — SCR-20260919-credential

> Phase 5：Credential Security 语义治理。本 SCR 汇总本 Phase 新增/冻结的语义。

---

## New Semantic

名称: gitRepoToken / dbCredential / browserCredential（State，均 keyring 命名空间）；saveCredential / getCredential / deleteCredential / fillBrowserCredential / listBrowserCredentials（Intent）；credential（Owner）；keyringWrite / keyringDelete（Side Effect）

类型:

- [x] State
- [x] Intent
- [x] Owner
- [x] Side Effect

分类:

- [x] CURRENT_FACT（代码中已存在并可验证：KeyringStore 薄封装 + 三处命名空间调用）
- [ ] ACCEPTED_ADR
- [ ] TARGET_CONTRACT
- [ ] PROPOSED_CHANGE
- [ ] KNOWN_DEBT

## Why Existing Semantic Cannot Represent

已查询条目：Phase 1-4 治理 Browser/Grid/View/Workspace/Bookmark/Terminal。凭据（git token /
DB 密码 / 浏览器自动填充凭据）全部落系统密钥库（OS keyring），由 `core/keyring_store.rs`
薄封装，调用点分散在 sync.rs / bridge.rs（git 仓库凭据、db_connect/disconnect、凭据导入/填充）。

无法表示的原因：凭据域无任何 State/Intent/Owner/SideEffect 登记；三套命名空间
（repo.id / db:<conn_id> / cred.key）未文档化；安全红线（密码绝不进前端）未被任何
registry 条目或 rejected_intent 固化。

## Existing Alternatives

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
| redact.ts 前端脱敏 | src/utils/redact.ts | 仅是展示层兜底，非语义治理真源；不约束凭据存储 owner |
| KeyringStore（Rust） | core/keyring_store.rs | 是真实入口，但未被登记为 credential owner，调用方无 owner 护栏 |

## State / Intent / Owner Impact

影响的状态: gitRepoToken / dbCredential / browserCredential（均为 keyring 命名空间，前端镜像 none/metadata_only）

影响的 Intent: saveCredential / getCredential / deleteCredential / fillBrowserCredential / listBrowserCredentials

Owner 是否变化: 是（新增 credential = KeyringStore，Rust 侧 OS keyring 薄封装）

## Second Source of Truth Risk

- [x] 有风险（说明如何避免）

说明：凭据真源 = 系统密钥库，本就不在应用内存（天然无第二内存真源）。风险点是**前端**
误把原始凭据值存入 reactive 状态 / 展示 / 导出。本 SCR 通过：
- `frontend_mirror: none / metadata_only` 固化三态的"前端不持原始值"契约；
- rejected_intents（exposePassword / copyPassword / exportCredential）由 R4 阻断级拒绝；
- owners.yaml `FRONTEND_CREDENTIAL_LEAK` 越界模式登记（待后续 R7 静态护栏，见 Debt-5-2）。
- 已观察到但暂不治理：`DatabasePanel.vue` 的 `password` 表单字段（瞬时 input，提交即入
  keyring）登记为 Debt-5-1，不静默消失。

## Checker Impact

需要新增/修改的规则或条目:

- [ ] `states.yaml`（governed_files 未扩展；新增 3 个 credential states + observed_not_governed 登记 DatabasePanel.password）
- [x] `intents.yaml`（5 credential intents + duplicate_names；rejected exposePassword/copyPassword/exportCredential）
- [x] `owners.yaml`（credential owner + violation_patterns FRONTEND_CREDENTIAL_LEAK / CREDENTIAL_DISPLAYED_OR_EXPORTED）
- [x] `side-effects.yaml`（keyringWrite / keyringDelete，requires_declaration=false 文档级登记）
- [x] `check-semantic-registry.mjs`（R4 域扩展夹具：注入 exposePassword 定义被 R4 检出）

预期 checker 结果：--self-test ALL_PASS；真实仓库 fail=0（无 exposePassword/copyPassword/exportCredential 出现在代码）。

## ADR Required

- [ ] 需要
- [x] 不需要（纯登记 + 注册表护栏，无语义裁决冲突）

ADR 编号/文件: 无（沿用 CredentialList.vue 既有安全红线注释 + ADR-P1A-3 派生量禁止存储原则）

## Reviewer Decision

- [x] APPROVED
- [ ] REJECTED
- [ ] NEEDS REVISION

裁决理由：KeyringStore 凭据三命名空间为真实存在且可验证；登记为单一 credential owner +
R4 拒绝泄露意图 + keyringWrite/Delete 副作用文档化，可防凭据语义漂移与前端泄露；
不修改业务代码，零回归风险。

是否确认为"当前事实"而非"提案":

- [x] 是（代码中已存在并可验证）

Reviewer: autonomous-phase-executor

日期: 2026-09-19

## 落地检查（合并前）

- [x] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [x] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [x] Registry YAML 已更新
- [x] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
