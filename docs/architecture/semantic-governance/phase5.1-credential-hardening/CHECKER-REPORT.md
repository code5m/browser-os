# Phase 5.1 — Checker Report

> 本 Phase 新增/扩展的检查器与自测证据。

## 1. check-semantic-registry.mjs（R1..R7）

### 新增/扩展

- **R3（扩展，Phase 5.1-A）**：通用化到 `credential` owner。读取 `owners.credential.owner_only_api`
  （`save_token`/`get_token`/`delete_token`），前端文件（组件/store/composable）直调这些原语或引用
  `KeyringStore` → `SEMANTIC_OWNER_VIOLATION`（fail）。既有 Browser/Grid 逻辑不变。
- **R7（新增，Phase 5.1-B）**：`SENSITIVE_INPUT_LEAK`。读取 `states.yaml` 中 `sensitive: true` 状态的
  `frontend_ident`（如 `password`）；任何前端文件把该标识符流入 `console.log/error/warn/debug/info`、
  `localStorage/sessionStorage.setItem`、`export const x = <密码>` → fail。显式放行
  `db.connect(password.value)`、`password.value = ""`、`redactSecrets()`。

### self-test 夹具（新增）

- R3 NEG：`src/components/browser/BadCred.vue` 调 `KeyringStore.save_token(...)` → 期望 fail（命中）。
- R3 POS：`src/components/browser/CredentialList.vue` 调 `bridge.fillBrowserCredential(...)` → 期望 0 fail。
- R3 FP：`src/components/workspace/DatabasePanel.vue` 仅 `db.connect(password.value)` + 清空 → 期望 0 fail/warn。
- R7 NEG：`src/components/workspace/Leak.vue` 调 `console.log("pw", password.value)` → 期望 fail（命中）。
- R7 FP：复用 R3 FP（DatabasePanel 安全流）→ 期望 0 fail/warn。

### 真实扫描

```text
node scripts/check-semantic-registry.mjs --self-test   → SELF_TEST_RESULT=ALL_PASS（R1..R7）
node scripts/check-semantic-registry.mjs              → fail=0 warn=6 info=80
```

## 2. check-sensitive-side-effects.mjs（S1/S2，新增脚本）

### 规则

- **S1 REJECTED_CREDENTIAL_INTENT**：任何文件出现 `exposePassword(` / `copyPassword(` / `exportCredential(` → fail。
- **S2 KEYRING_SECRET_IN_DTO**：Rust 中 `struct *Credential*` 且派生 `Serialize`，其字段含
  `password`/`secret`/`api_key`/`access_token`/`plain_token`（行注释已剥离）→ fail。

### CLI

- `--help`：打印规则与用法。
- `--self-test`：内嵌 5 个夹具（2 POS + 3 NEG）→ `SELF_TEST_RESULT=ALL_PASS`。
- `--json`：输出 `{result, fail, warn, findings}`。
- `--strict`：warn 级也计为失败（本脚本无 warn，语义同默认）。

### self-test 夹具

| 名称 | 类型 | 期望 |
|---|---|---|
| BrowserCredentialItem 无 password 字段（Serialize DTO） | POS | pass |
| 入站凭据结构含 password 但仅 Deserialize（不回传前端） | POS | pass |
| 凭据 DTO 派生 Serialize 且含 password（泄露到前端） | NEG | fail (S2) |
| 调用 exposePassword（前端返回密钥） | NEG | fail (S1) |
| 调用 exportCredential（导出密钥） | NEG | fail (S1) |

### 真实扫描

```text
node scripts/check-sensitive-side-effects.mjs --self-test   → SELF_TEST_RESULT=ALL_PASS
node scripts/check-sensitive-side-effects.mjs              → SENSITIVE_SIDE_EFFECT_RESULT=PASS fail=0 warn=0
```

## 3. pre-merge.sh 接入

- Phase 03 checker 循环新增 `check-sensitive-side-effects`（真实扫描门禁）。
- self-test 段新增 `check-sensitive-side-effects.mjs --self-test`。

## 4. 防误报说明

- S2 每个 struct 仅配对紧随的 `#[derive]`（正则 `(?!#\[derive)` 阻断跨 derive 误配对）。
- S2 字段扫描前 `body.replace(/\/\/[^\n]*/g, "")` 剥离 `///` 文档注释，避免注释里的
  `password` 字样误报（真实 `BrowserCredentialItem` 注释含「拿不到 password」但无 password 字段）。
- R7 仅匹配 `sensitive: true` 状态的 `frontend_ident`，不泛扫全部 `password` 字样，避免误伤。
