# Phase 5.1 — Runtime Acceptance

> 凭据加固为**静态护栏**（编译期/门禁期），不引入新运行时端点。运行时行为本身由既有代码保证，
> 本 Phase 仅用门禁固化并验证。

## 验证方式

| 运行时行为（既有） | 静态护栏（本 Phase） | 验收 |
|---|---|---|
| `DatabasePanel.vue`：`password` 仅 `db.connect(password.value)` 后 `password.value = ""` | R7 禁止 password 流入 console/localStorage/export；放行 db.connect+清空 | PASS（真实扫描 0 失败） |
| `CredentialList.vue`：组件只发 `credential_id`，Rust 读 keyring 注入 | R3 禁止前端直调 `KeyringStore.*`；放行 `bridge.fillBrowserCredential` | PASS（真实扫描 0 失败） |
| `BrowserCredentialItem`（回传前端 DTO）仅含 `has_password`，无 `password` | S2 禁止 Serialize 凭据 DTO 含原始密钥字段 | PASS（真实扫描 0 失败） |
| keyring 读出密钥不回传/不日志/不剪贴板 | S1 禁止 `expose/copy/exportCredential` | PASS（全仓无此类调用） |

## 门禁运行证据（本地）

```text
# 静态门禁（CI / pre-merge 运行）
node scripts/check-semantic-registry.mjs --self-test      → ALL_PASS（R1..R7）
node scripts/check-sensitive-side-effects.mjs --self-test → ALL_PASS（S1/S2）
bash scripts/pre-merge.sh --self-test                      → semantic + git-integrity 绿（terminal 债为既有 FAIL）

# 真实仓库扫描
node scripts/check-semantic-registry.mjs                  → fail=0
node scripts/check-sensitive-side-effects.mjs             → fail=0
```

## 说明

- 无运行时 smoke 端点新增；凭据流路径（db_connect / fill_browser_credential / list_browser_credentials）
  保持既有实现不变，本 Phase 仅加护栏，不改动其行为。
- 因是纯静态加固，RUNTIME_ACCEPTANCE 以"门禁在真实代码上 PASS + 关键运行时契约被静态规则覆盖"为准。
