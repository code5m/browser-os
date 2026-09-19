# Phase 5 — Credential Security Governance（Design）

> 目标：把凭据（git token / DB 密码 / 浏览器自动填充凭据）这一安全敏感域纳入 Semantic Registry，
> 固化"凭据真源 = 系统密钥库、前端不持原始值"的契约与拒绝泄露意图。纯治理 + 注册表护栏，不修改业务代码。

---

## 1. 域边界（Scope）

| 维度 | 真源 | 前端可见性 |
|---|---|---|
| git 仓库 token | OS keyring，键 `repo.id` | none（绝不进网页 JS） |
| DB 连接密码 | OS keyring，键 `db:<conn_id>` | none |
| 浏览器自动填充凭据 | OS keyring，键 `cred.key` | 仅 metadata（id/username/origin/has_password） |

实现入口：`src-tauri/src/core/keyring_store.rs`（KeyringStore，OS keyring 薄封装）。
调用方：sync.rs（repo token）、bridge.rs（git 仓库凭据、db_connect/disconnect、凭据导入/填充）。

## 2. State（已登记）

- `gitRepoToken`（owner: credential，kind: keyring，frontend_mirror: none）
- `dbCredential`（owner: credential，kind: keyring，key_namespace: `db:<conn_id>`，frontend_mirror: none）
- `browserCredential`（owner: credential，kind: keyring，key_namespace: `cred.key`，frontend_mirror: metadata_only）

三态均非 derived、无 duplicate_names → R1/R6 不触发；不在 governed_files → R2 不触发。

## 3. Intent（已登记 + 已否决）

| Intent | owner | 说明 |
|---|---|---|
| saveCredential | credential | KeyringStore.save_token（含 dbConnect 别名） |
| getCredential | credential | KeyringStore.get_token（仅 Rust 侧） |
| deleteCredential | credential | KeyringStore.delete_token（含 dbDisconnect 别名） |
| fillBrowserCredential | credential | 仅传 credential_id + tab id，Rust 读 keyring 注入 |
| listBrowserCredentials | credential | 仅返回 BrowserCredentialItem 元数据 |

**已否决（R4 阻断级）**：`exposePassword` / `copyPassword` / `exportCredential`（安全红线）。

## 4. Owner（已登记）

`credential` = KeyringStore（Rust）。owns: gitRepoToken / dbCredential / browserCredential。
owner_only_api: save_token / get_token / delete_token。
forbidden_callers: 任何前端 store / component（组件只经 bridge.fillBrowserCredential 发 credential_id）。

## 5. Lifecycle（凭据生命周期）

```text
配置/导入 → saveCredential(KeyringStore.save_token, 命名空间键) → OS keyring 落地
使用     → getCredential / fillBrowserCredential（Rust 读 keyring 注入，前端不接触原文）
撤销     → deleteCredential（db_disconnect 一并撤销 db:<conn_id>）
前端      → 只持不透明 metadata（credential_id / has_password），绝不持原始值
```

## 6. Side Effect（已登记）

- `keyringWrite`（call_sites: KeyringStore.save_token；requires_declaration: false）
- `keyringDelete`（call_sites: KeyringStore.delete_token；requires_declaration: false）

R5 对 requires_declaration=false 跳过 → 不新增 warn。

## 7. Checker（check-semantic-registry.mjs）

- R4 域扩展：NEG 夹具注入 `function exposePassword(){...}`，自检验证被 SEMANTIC_INTENT_DUPLICATE 检出。
- 真实仓库：rejected 意图名（exposePassword/copyPassword/exportCredential）代码中不存在 → fail=0。
- 不削弱 R1..R6；不扩展 governed_files（避免误伤既有表单字段）。

## 8. Invariants

- INV-5-1：凭据真源恒为系统密钥库；应用内存不长期持有原始凭据值。
- INV-5-2：前端持有的凭据信息仅限于不透明 metadata（credential_id / has_password），绝不含量 password/token 原文。
- INV-5-3：三命名空间隔离（repo.id / db:<conn_id> / cred.key），删除互不牵连。

## 9. Acceptance Matrix

| 项 | 结果 |
|---|---|
| R4 拒绝 exposePassword/copyPassword/exportCredential | PASS（NEG 夹具检出） |
| 三凭据态登记且不影响真实扫描 fail | PASS（fail=0） |
| keyringWrite/keyringDelete 副作用文档化 | PASS |
| 无业务代码改动 | PASS |
| self-test ALL_PASS | PASS |

## 10. Debt（Phase 5 继承/新增）

- Debt-5-1：DatabasePanel.vue `password` 表单字段（瞬时 input，提交即入 keyring）目前未受静态护栏
  约束；已登记于 observed_not_governed，待后续 hardening（不静默消失）。
- Debt-5-2：owners.yaml 已登记 FRONTEND_CREDENTIAL_LEAK 越界模式，但 R3 当前为 browser 专用，
  未通用化到 credential owner；建议后续新增 R7（凭据泄露静态护栏）再武装，避免对既有表单字段
  误判失败。
- Debt-5-3：keyringWrite/keyringDelete 副作用仅文档级登记（requires_declaration=false），同
  writeFile/termProcess 口径，不新增 R5 噪声。
