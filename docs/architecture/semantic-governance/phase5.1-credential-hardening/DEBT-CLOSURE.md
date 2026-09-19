# Phase 5.1 — Debt Closure

> 本文件记录 Phase 5 三笔债的收口证据（非隐藏、非静默消失）。

## Debt-5-1 — DatabasePanel.password 表单字段未受静态护栏

**状态：CLOSED（Phase 5.1-B）**

证据：
- `states.yaml` 新增 `databaseCredentialInput`：`sensitive: true`、`owner: credential`、
  `frontend_ident: [password]`、`frontend_mirror: local_transient`。
- `side-effects.yaml` 新增 `sensitiveInput`：`forbidden: [log, plainStorage, export]`、
  `allowed: [secureCredentialFlow]`。
- `check-semantic-registry.mjs` 新增 **R7（SENSITIVE_INPUT_LEAK）**：任何前端文件把密码类标识符
  流入 `console.*` / `localStorage.setItem` / `export const x = <密码>` 即阻断级失败；
  显式允许 `db.connect(password.value)` 安全凭据流与 `password.value = ""` 清空、以及 `redactSecrets()`。
- `states.yaml` 的 `observed_not_governed` 中 `DatabasePanel.vue: password` 条目已移除（已治理，不再"未治理"）。
- 真实代码校验：`DatabasePanel.vue` 的 `password` 仅 `db.connect(password.value)` 后 `password.value = ""`，
  无泄露汇 → R7 真实扫描 0 失败。
- NEG 夹具：注入 `password → console.log` 被 R7 检出。

## Debt-5-2 — R3 未通用化到 credential owner

**状态：CLOSED（Phase 5.1-A）**

证据：
- `check-semantic-registry.mjs` 的 `rule3` 新增 credential owner 通用化分支（registry 驱动，读取
  `owners.credential.owner_only_api`）：任何前端文件（组件/store/composable）直调
  `save_token` / `get_token` / `delete_token` 或引用 `KeyringStore` 即 `SEMANTIC_OWNER_VIOLATION`。
- 既有 Browser/Grid 检查字节级不变，仅新增 credential 域覆盖。
- `owners.yaml` 的 `credential` owner 更新 `rationale`（移除"R3 未通用化"旧注）+ 新增 `manages`
  （keyring 生命周期）。
- 真实代码校验：前端无任何文件直调 keyring 原语 → 真实扫描 0 失败。
- NEG 夹具：注入 `KeyringStore.save_token(...)` 被 R3 检出；POS 夹具：`bridge.fillBrowserCredential` 正确流不误报。

## Debt-5-3 — keyringWrite/keyringDelete 副作用仅文档登记

**状态：CLOSED（Phase 5.1-C）**

证据：
- `side-effects.yaml` 新增 `keyringRead`：`level: critical`、`owner: credential`、
  `forbidden: [frontend_return, log, clipboard]`、`allowed: [injectViaCredentialId]`。
- 新建 `scripts/check-sensitive-side-effects.mjs`（Sensitive Side Effect Contract 门禁）：
  - **S1 REJECTED_CREDENTIAL_INTENT**：任何代码调用 `exposePassword` / `copyPassword` / `exportCredential`（Phase 5 已否决意图）即失败。
  - **S2 KEYRING_SECRET_IN_DTO**：回传前端的凭据 DTO（`struct *Credential*` 且派生 `Serialize`）
    不得含原始密钥字段（password/secret/api_key/access_token）。每个 struct 配对自身 derive 属性，
    且检测前剥离行注释（避免文档注释里的 `password` 误报）。
  - 支持 `--help` / `--self-test` / `--json` / `--strict`。
- `scripts/pre-merge.sh`：Phase 03 checker 循环 + self-test 段均接入 `check-sensitive-side-effects.mjs`。
- 真实代码校验：`BrowserCredentialItem` 仅含 `has_password`（无 `password` 字段）→ S2 0 失败；
  全仓无 `expose/copy/exportCredential` → S1 0 失败。
- NEG 夹具：注入 `BrowserCredentialRow{password}`（Serialize）+ `exposePassword()` 均被检出。

## 遗留（非本 Phase，未隐藏）

- Debt-004（Terminal checker self-test FAIL）：既有债，pre-merge --self-test 既有 FAIL，非本阶段引入，按惯例不处理。
- 其余 Debt-001~003 / 1.7-1~2 / 2-1~3 / 3-1~3 / 4-1~3 维持原状。
