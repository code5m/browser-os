# Phase 5.1 — Credential Security Hardening（Final Report）

> STATUS: PASS
> Tag: `semantic-phase5.1-credential-hardening-pass`

## 范围

Phase 5 已把凭据域（git token / DB 密码 / 浏览器自动填充凭据）登记进 Semantic Registry，
但三项债未收口：

- Debt-5-1：DatabasePanel 的 `password` 表单字段未被静态护栏覆盖
- Debt-5-2：R3 Owner 检查未通用化到 credential owner
- Debt-5-3：keyringWrite/keyringDelete 副作用仅文档登记，无机器约束

本 Phase 三项加固（A/B/C）分别收口上述三债，**未修改任何业务代码**，仅扩展 Registry + Checker。

## 三项加固

| 子阶段 | 债 | 交付 | 门禁 |
|---|---|---|---|
| A Owner 通用化 | Debt-5-2 | R3 从 Browser/Grid 通用化到 credential owner（registry 驱动） | R3 新增 credential NEG/正确流 POS/安全流 FP |
| B 敏感输入护栏 | Debt-5-1 | states.yaml `databaseCredentialInput`(sensitive) + side-effects `sensitiveInput` + R7 | R7 新增泄露汇 NEG + db.connect 安全流 FP |
| C Keyring 副作用契约 | Debt-5-3 | side-effects `keyringRead` + 新脚本 `check-sensitive-side-effects.mjs`(S1/S2) + 接入 pre-merge | S1/S2 POS+NEG 夹具 |

## 验收（全部 PASS）

```text
SEMANTIC REGISTRY self-test:  ALL_PASS（R1..R7；+credential owner R3 NEG / sensitiveInput R7 NEG）
SEMANTIC REGISTRY real scan:   PASS（fail=0；warn=6 pre-existing R5 非阻断；info=80；R7 无新失败）
SENSITIVE_SIDE_EFFECT self-test: ALL_PASS（S1/S2 POS+NEG 夹具）
SENSITIVE_SIDE_EFFECT real scan: PASS（fail=0；BrowserCredentialItem 无 password 字段；无 expose/copy/exportCredential）
R3 CREDENTIAL OWNER:           PASS（前端直调 KeyringStore.save_token 被 R3 检出；bridge.fillBrowserCredential 正确流不误报）
R7 SENSITIVE INPUT:             PASS（DatabasePanel.password 仅走 db.connect+清空，无泄露汇；注入 password→console.log 被 R7 检出）
NO REGRESSION:                  PASS（R1..R7 + 新门禁均未削弱既有规则；未改业务代码）
```

## 提交 / Tag

```text
<phase51-a>  feat(phase5.1): enforce credential owner boundaries
<phase51-b>  feat(phase5.1): add sensitive credential input guard
<phase51-c>  feat(phase5.1): enforce keyring side effect contracts
<phase51-tip> docs(phase5.1): closeout + handoff update
semantic-phase5.1-credential-hardening-pass  （annotated，master tip，ff-merge 后打）
```

## 原则遵守

- 未削弱已有规则（R1..R6 行为字节级不变；R7 为新增规则）
- 未增加 allow-list 绕过（R7 仅显式允许 db.connect/清空/redactSecrets 安全流）
- 未隐藏债务（Debt-5-1/5-2/5-3 显式标记 CLOSED 并留证据）
- 未为 PASS 修改 Checker 语义（NEG 夹具为真实违规模式）
- 先 Registry（SCR-20260919-credential-hardening）→ 再代码

## 停止

Phase 5.1 完成，按指令停止，不进入新治理域。
