# HANDOFF — CURRENT PROJECT STATE

> 下一 Agent 无需重扫全仓即可继续。本文件为**当前真实状态快照**，事实来自 `git` 与已落地文件。
> 最后更新：2026-09-19
>
> ✅ **git 对象损坏已清理**（Phase 1.7）：`git fsck --full` 现已 0 error。
>
> ✅ **Phase 2/3/4/5 语义治理已收口**：Workspace/FilePanel（tag phase2）、Bookmark（tag phase3）、
> Terminal Lifecycle（tag phase4）、Credential Security（tag phase5）均纳入 Semantic Registry；真实扫描 fail=0。
>
> ✅ **Phase 5.1 Credential Security Hardening 已收口**（tag semantic-phase5.1-credential-hardening-pass）：
> Owner 通用化（R3）+ 敏感输入护栏（R7）+ Keyring 副作用契约门禁（check-sensitive-side-effects.mjs）；
> Debt-5-1/5-2/5-3 全部收口；真实扫描 fail=0。

---

## 1. Current Milestone

```text
Phase 0                     CLOSED
Phase 1 Browser/Grid        CLOSED   （tag: semantic-phase1-browser-grid-pass）
Phase 1.5 Semantic Registry  CLOSED
Phase 1.6 Semantic Reg. Acc  PASS
Phase 1.7 Git Integrity      CLOSED   （tag: semantic-phase1.7-git-integrity-pass）
Phase 2 Workspace/FilePanel  CLOSED   （tag: semantic-phase2-workspace-pass）
Phase 3 Bookmark             CLOSED   （tag: semantic-phase3-bookmark-pass）
Phase 4 Terminal Lifecycle   CLOSED   （tag: semantic-phase4-terminal-pass）
Phase 5 Credential Security    CLOSED   （tag: semantic-phase5-credential-pass）
Phase 5.1 Credential Hardening  CLOSED   （tag: semantic-phase5.1-credential-hardening-pass）
```

Phase 4 验收结论：

```text
SEMANTIC REGISTRY self-test: PASS  （R1..R6 全部 positive/negative/false-positive 夹具 ALL_PASS）
SEMANTIC REGISTRY real scan:  PASS  （fail=0；warn=6 pre-existing R5 非阻断）
R2 TERMINAL GOVERN:           PASS  （useSystemStore.ts 16 声明全登记，无第二面板列表 FAIL）
R4 TERMINAL INTENT:           PASS  （terminal 重复名 newTerm 等未定义；注入 newTerm 定义被 R4 检出）
NO REGRESSION:                PASS  （R1..R6 未削弱；self-test 仍 ALL_PASS）

Phase 5 验收结论：

```text
SEMANTIC REGISTRY self-test: PASS  （R1..R6 全部夹具 ALL_PASS）
SEMANTIC REGISTRY real scan:  PASS  （fail=0；warn=6 pre-existing R5 非阻断；info=80）
R4 CREDENTIAL REJECT:          PASS  （注入 exposePassword 定义被 R4 检出；代码无泄露意图）
CREDENTIAL REGISTRY:           PASS  （states 3 / intents 5+rejected 3 / owner 1 / side-effects 2 全登记）
NO REGRESSION:                 PASS  （R1..R6 未削弱；未改业务代码）

Phase 5.1 验收结论：
SEMANTIC REGISTRY self-test: PASS  （R1..R7 全部夹具 ALL_PASS；+credential owner R3 NEG / sensitiveInput R7 NEG）
SEMANTIC REGISTRY real scan:  PASS  （fail=0；warn=6 pre-existing R5；info=80；R7 无新失败）
SENSITIVE_SIDE_EFFECT self-test: PASS  （S1/S2 POS+NEG 夹具 ALL_PASS）
SENSITIVE_SIDE_EFFECT real scan: PASS  （fail=0；BrowserCredentialItem 无 password 字段；无 expose/copy/exportCredential）
R3 CREDENTIAL OWNER:          PASS  （前端直调 KeyringStore.save_token 被 R3 检出；bridge.fillBrowserCredential 正确流不误报）
R7 SENSITIVE INPUT:            PASS  （DatabasePanel.password 仅走 db.connect+清空，无泄露汇；注入 password→console.log 被 R7 检出）
NO REGRESSION:                 PASS  （R1..R7 + 新门禁均未削弱既有规则；未改业务代码）
```
```

---

## 2. Git State

```text
branch:    master（Phase 5.1 以 feature/phase5.1-credential-hardening 实现，ff-merge 入 master）
HEAD:      semantic-phase5.1-credential-hardening-pass（annotated tag = master tip）
working tree: 干净（仅未跟踪 .snapshots/ 与 diagnostics/ —— 取证产物，不入库）
```

### latest commits（Phase 4 在其上）

```text
<phase-tip>  docs(phase5): closeout + handoff update
<phase-feat> feat(phase5): credential security semantic governance
<phase51-tip> docs(phase5.1): closeout + handoff update
<phase51-a>  feat(phase5.1): enforce credential owner boundaries
<phase51-b>  feat(phase5.1): add sensitive credential input guard
<phase51-c>  feat(phase5.1): enforce keyring side effect contracts
<phase-tip4> docs(phase4): closeout + handoff update
<phase-feat4> feat(phase4): terminal lifecycle semantic governance
793c2c3      docs(phase3): closeout + handoff update
145854a      feat(phase3): bookmark semantic governance
7e881f4      docs(phase2): closeout + handoff update
bff14af      feat(phase2): workspace/filepanel semantic governance
b36703e      docs(phase1.7): closeout + handoff update
77a6b57      feat(phase1.7): git integrity gate + recovery procedure
2b030eb      gates(phase1.5): wire semantic registry gate into pre-merge
3874bef      feat(phase1.5): semantic registry + semantic gate (governance infra only)
```

### latest tags（及分类）

```text
semantic-phase0-infra-pass        -> 481fbf7
semantic-phase0-policy-pass        -> 04e4cbc
semantic-phase1-browser-grid-code-pass -> a30fd57
semantic-phase1-browser-grid-pass       -> 7e8d867
semantic-phase1.7-git-integrity-pass    -> b36703e
semantic-phase2-workspace-pass          -> 7e881f4
semantic-phase3-bookmark-pass           -> 793c2c3
semantic-phase4-terminal-pass           -> <phase-tip4>
semantic-phase5-credential-pass          -> <phase-tip>
semantic-phase5.1-credential-hardening-pass -> <phase51-tip>

semantic-registry-v1:  NOT EXISTS（属原 Phase 1.6 可选动作，本次未创建；如需创建见 §7）
```

---

## 3. Frozen Architecture Rules（不可违反）

```text
mainView = 用户当前希望看到的 Main Surface（owner: useLayoutStore）

gridOpen = Grid resource 是否存在（owner: useBrowserStore）
desiredGridVisibility / isBrowserVisible / currentLocalPath / sorted = 纯派生，禁止存储/赋值（R6）

禁止:
  gridVisible stored / exitGrid(mode) / 拆 position↔show
  filePath/inlineFile/previewDir/pathInput 组件直写（owner: useWorkspaceStore）
  items/panelOpen 组件直写，或第二份 Bookmark[]（owner: useBookmarkStore）
  termPanes/activeTermId/terminalOpen 组件直写，或第二份面板列表（owner: useSystemStore）
  收藏夹与主页快捷方式合并 / activateTerm 作为 setActiveTerm 重复名

Owner 收敛:
  View Navigation   = useLayoutStore
  Browser/Grid Life = useBrowserStore
  Native Execution  = useBrowserHost / bridge / Rust
  Workspace/FilePanel = useWorkspaceStore
  Bookmark          = useBookmarkStore
  Terminal          = useSystemStore
  Credential         = KeyringStore（Rust 侧 OS keyring 薄封装）

Phase 5 新增（Credential Security）：
  gitRepoToken / dbCredential / browserCredential = 凭据三命名空间（repo.id / db:<conn_id> / cred.key），真源=系统密钥库
  saveCredential/getCredential/deleteCredential/fillBrowserCredential/listBrowserCredentials 单一入口
  前端只持不透明 metadata（credential_id / has_password），绝不持原始密码/token
  已否决：exposePassword / copyPassword / exportCredential（R4 阻断级，凭据泄露红线）

Phase 5.1 新增（Credential Security Hardening）：
  R3 已通用化到 credential owner：前端任何文件（组件/store/composable）直调 KeyringStore 原语
    （save_token/get_token/delete_token）或引用 KeyringStore 即 SEMANTIC_OWNER_VIOLATION；
    组件只经 bridge 意图入口（fillBrowserCredential/listBrowserCredentials/db_connect）
  R7 敏感输入护栏：sensitive:true 状态的前端标识符（password）不得流入 console/localStorage/export；
    允许 db.connect(password.value) 安全凭据流与 password.value="" 清空（DatabasePanel 收口 Debt-5-1）
  check-sensitive-side-effects.mjs（S1/S2）：keyring 读出的密钥禁止返回前端/日志/剪贴板；
    credential DTO（*Credential* + Serialize）不得含原始密钥字段（Debt-5-3 收口）

Phase 4 新增（Terminal）：
  termPanes = 面板注册表唯一真源（spawn→push，kill→filter）
  activeTermId 必须 ∈ termPanes.id（kill 后重置为首剩余）
  addTermPane/killTerm/termWrite/setActiveTerm/restartTerm/bindTermWriter/replayTermHistory 单一入口
```

---

## 4. Semantic Registry 入口

```text
docs/architecture/semantic-registry/{states,intents,owners,side-effects}.yaml
scripts/check-semantic-registry.mjs（R1-R6）/ scripts/pre-merge.sh（已接入）
docs/architecture/semantic-changes/SCR-template.md
已归档 SCR：20260919-workspace-filepanel / 20260919-bookmark / 20260919-terminal
```

流程：`查 Registry → 已有则用之 → 无则填 SCR → Review → ADR → 更新 Registry → 才许写代码`

---

## 4b. Git Integrity / Recovery（Phase 1.7）

```text
scripts/snapshot.sh / check-git-repo-integrity.sh / git-recover.sh
docs/architecture/semantic-governance/phase1.7-git-integrity/{Phase1.7-design,RECOVERY-PROCEDURE}.md
```

---

## 4c. Workspace/FilePanel Governance（Phase 2）

```text
docs/architecture/semantic-registry/states.yaml（filePath/inlineFile/previewDir/pathInput/currentLocalPath）
docs/architecture/semantic-registry/intents.yaml（openFile/openFileInline/enterDir/saveFile/closeFileEditor）
docs/architecture/semantic-governance/phase2-workspace/{Phase2-design,PHASE_2_CLOSEOUT_RESULT}.md
```

---

## 4d. Bookmark Governance（Phase 3）

```text
docs/architecture/semantic-registry/states.yaml（items/loaded/busy/error/panelOpen/sorted）
docs/architecture/semantic-registry/intents.yaml（add/remove/toggle/importFile/togglePanel + rejected mergeBookmarksIntoHome）
docs/architecture/semantic-governance/phase3-bookmark/{Phase3-design,PHASE_3_CLOSEOUT_RESULT}.md
```

---

## 4e. Terminal Lifecycle Governance（Phase 4）

```text
docs/architecture/semantic-registry/states.yaml（terminalOpen/termPanes/termGrid/termGridCount/activeTermId/autoConfirmCli/termProbeOn/m0Cfg/m0StartTs/droppedChunks/droppedBytes）
docs/architecture/semantic-registry/intents.yaml（addTermPane/killTerm/termWrite/setActiveTerm/restartTerm/bindTermWriter/replayTermHistory）
docs/architecture/semantic-registry/owners.yaml（terminal owner + COMPONENT_WRITES_TERMINAL）
scripts/check-semantic-registry.mjs（R2/R4 terminal 域 + 夹具）
docs/architecture/semantic-governance/phase4-terminal/{Phase4-design,PHASE_4_CLOSEOUT_RESULT}.md
```

---

## 4f. Credential Security Governance（Phase 5）

```text
docs/architecture/semantic-registry/states.yaml（gitRepoToken/dbCredential/browserCredential）
docs/architecture/semantic-registry/intents.yaml（save/get/delete/fill/listBrowserCredentials + rejected expose/copy/exportPassword）
docs/architecture/semantic-registry/owners.yaml（credential owner + FRONTEND_CREDENTIAL_LEAK）
docs/architecture/semantic-registry/side-effects.yaml（keyringWrite / keyringDelete）
scripts/check-semantic-registry.mjs（R4 凭据泄露意图夹具）
docs/architecture/semantic-governance/phase5-credential/{Phase5-design,PHASE_5_CLOSEOUT_RESULT}.md
```

---

## 4g. Credential Security Hardening（Phase 5.1）

```text
docs/architecture/semantic-registry/states.yaml（databaseCredentialInput sensitive + observed 移除）
docs/architecture/semantic-registry/side-effects.yaml（sensitiveInput / keyringRead）
docs/architecture/semantic-registry/owners.yaml（credential owner manages + R3 通用化）
scripts/check-semantic-registry.mjs（R3 credential 通用化 + R7 敏感输入）
scripts/check-sensitive-side-effects.mjs（S1/S2 keyring 数据流契约；--help/--self-test/--json/--strict）
scripts/pre-merge.sh（Phase 03 循环 + self-test 接入新门禁）
docs/architecture/semantic-governance/phase5.1-credential-hardening/{FINAL-REPORT,DEBT-CLOSURE,CHECKER-REPORT,RUNTIME-ACCEPTANCE}.md
```

---

## 5. Current Task Status

```text
Completed: Phase 0 / 1 / 1.5 / 1.6 / 1.7 / 2 / 3 / 4 / 5 / 5.1 全部完成

Pending: 无

Blocked: 无

Next recommended task:
  语义治理主线（Phase 1.7→2→3→4→5）已全部收口。
  可选：semantic-registry-v1 release tag（见 §7）
  或按用户新指令开启新治理域（须先走 SCR）
```

---

## 6. Known Debt

```text
Debt-001  Grid UDS socket cleanup              KNOWN DEBT（不处理）
Debt-002  toggleGridToolbar dead code          KNOWN DEBT（不处理）
Debt-003  closeGridCell orphan API             KNOWN DEBT（不处理）
Debt-004  Terminal checker self-test FAIL      KNOWN DEBT（不处理；pre-merge --self-test 既有 FAIL，非本阶段引入）
Debt-1.7-1 pack 内损坏对象无自动恢复          KNOWN DEBT（交专项）
Debt-1.7-2 无周期性后台完整性巡检             KNOWN DEBT（交 ops）
Debt-2-1 filePath overloaded 语义未拆分       KNOWN DEBT
Debt-2-2 R2 正则不识别 reactive<Set/Map> 嵌套泛型  KNOWN DEBT（checker 盲区）
Debt-2-3 writeFile 副作用仅文档化             KNOWN DEBT
Debt-3-1 normalizeUrl 身份键未做 checker 强制  KNOWN DEBT
Debt-3-2 panelOpen 置于 bookmark store 而非 layout KNOWN DEBT（有意设计）
Debt-3-3 bookmarkPersist 副作用仅文档化       KNOWN DEBT
Debt-4-1 termPanes 第二注册表护栏限于 R2（spawn/kill 资源策略在 check-terminal-policy.py，未并入 Registry） KNOWN DEBT
Debt-4-2 R3 不扫描组件直写 termPanes/activeTermId（R3 为 browser 专用） KNOWN DEBT
Debt-4-3 termProcess 副作用仅文档化           KNOWN DEBT
Debt-5-1 DatabasePanel.password 表单字段未受静态护栏  CLOSED（Phase 5.1-B：databaseCredentialInput sensitive + R7 护栏；已从 observed 移除）
Debt-5-2 R3 未通用化到 credential owner（FRONTEND_CREDENTIAL_LEAK 未武装） CLOSED（Phase 5.1-A：R3 通用化 credential owner）
Debt-5-3 keyringWrite/keyringDelete 副作用仅文档化   CLOSED（Phase 5.1-C：check-sensitive-side-effects.mjs S1/S2 机器约束）
```

---

## 7. Next Agent Instructions

**开始前先读取（不要重扫全仓）：**

```text
docs/architecture/HANDOFF_CURRENT_STATE.md
docs/architecture/semantic-registry/README.md
docs/architecture/semantic-registry/{states,intents,owners,side-effects}.yaml
docs/architecture/semantic-governance/phase1-browser-grid/FINAL-REPORT.md
docs/architecture/semantic-governance/Known-Debt.md
docs/architecture/semantic-governance/phase1.7-git-integrity/{Phase1.7-design,RECOVERY-PROCEDURE}.md
docs/architecture/semantic-governance/phase2-workspace/{Phase2-design,PHASE_2_CLOSEOUT_RESULT}.md
docs/architecture/semantic-governance/phase3-bookmark/{Phase3-design,PHASE_3_CLOSEOUT_RESULT}.md
docs/architecture/semantic-governance/phase4-terminal/{Phase4-design,PHASE_4_CLOSEOUT_RESULT}.md
docs/architecture/semantic-governance/phase5-credential/{Phase5-design,PHASE_5_CLOSEOUT_RESULT}.md
docs/architecture/semantic-governance/phase5.1-credential-hardening/{FINAL-REPORT,DEBT-CLOSURE,CHECKER-REPORT,RUNTIME-ACCEPTANCE}.md
```

**禁止：**

```text
禁止重新扫描全仓历史上下文
禁止修改业务代码（src/ src-tauri/）除非对应 Phase 明确要求
禁止清理 Known Debt
禁止扩大 Semantic Registry 范围（先走 SCR；治理域外不判失败）
禁止削弱任何 Checker（R1..R6 / check-git-repo-integrity 的 ^error: 判定）
禁止 blind git reset / 删被引用对象（恢复只走 git-recover.sh 护栏）
```

**可选（固化 release tag）：**

```bash
git tag -a semantic-registry-v1 -m "Semantic Registry + Checker + pre-merge gate; Phase 2/3/4 governance PASS"
# 本地，不推送
```

**快速自查：**

```bash
node scripts/check-semantic-registry.mjs --self-test   # ALL_PASS（R1..R7）
node scripts/check-semantic-registry.mjs              # fail=0（warn 非阻断）
node scripts/check-sensitive-side-effects.mjs --self-test  # ALL_PASS（S1/S2）
node scripts/check-sensitive-side-effects.mjs             # fail=0
bash scripts/check-git-repo-integrity.sh --self-test  # SELF_TEST: PASS
bash scripts/pre-merge.sh --self-test                 # semantic + git-integrity 绿（terminal 债为既有 FAIL）
```

---

## 8. 当前验证状态（Phase 4 验收证据）

```text
SEMANTIC REGISTRY self-test: ALL_PASS（R1..R6；+terminal 第二面板列表 R2 NEG / newTerm R4 NEG）
SEMANTIC REGISTRY real scan: PASS（fail=0；warn=6 pre-existing R5；info=80）
R2 TERMINAL GOVERN: PASS（useSystemStore.ts 16 声明全登记；注入第二份 ref<{id,cwd}[]> 被 R2 检出）
R4 TERMINAL INTENT: PASS（terminal 重复名未定义；注入 newTerm 定义被 R4 检出）
NO REGRESSION: PASS（R1..R6 未削弱；未改业务代码）
KNOWN_DEBT isolated: PASS（Debt-001~004 / 1.7-1~2 / 2-1~3 / 3-1~3 / 4-1~3 / 5-1~3 均显式记录）

Phase 5 验收证据（2026-09-19）：
  SEMANTIC REGISTRY self-test: ALL_PASS（R1..R6；+credential 泄露意图 exposePassword R4 NEG）
  SEMANTIC REGISTRY real scan: PASS（fail=0；warn=6 pre-existing R5；info=80）
  R4 CREDENTIAL REJECT: PASS（rejected 意图无代码出现；注入定义被检出）
  NO REGRESSION: PASS（R1..R6 未削弱；未改业务代码）

Phase 5.1 验收证据（2026-09-19）：
  SEMANTIC REGISTRY self-test: ALL_PASS（R1..R7；+credential owner R3 NEG / sensitiveInput R7 NEG）
  SEMANTIC REGISTRY real scan: PASS（fail=0；warn=6 pre-existing R5；info=80）
  SENSITIVE_SIDE_EFFECT self-test: ALL_PASS（S1/S2 POS+NEG）
  SENSITIVE_SIDE_EFFECT real scan: PASS（fail=0）
  R3 CREDENTIAL OWNER: PASS（KeyringStore.save_token 直调被检出；bridge 意图入口不误报）
  R7 SENSITIVE INPUT: PASS（DatabasePanel.password 仅 db.connect+清空；注入 password→console.log 被检出）
  NO REGRESSION: PASS（R1..R7 + 新门禁未削弱既有规则；未改业务代码）
  KNOWN_DEBT: Debt-5-1/5-2/5-3 CLOSED（Phase 5.1 收口，非隐藏）
```
