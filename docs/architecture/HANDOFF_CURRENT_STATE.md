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
>
> ✅ **Phase 6A Semantic Migration Core Closure 已收口**（tag semantic-phase6a-core-closure-pass）：
> 三项最小语义迁移——M2-a `aiNavOpen` 唯一 owner（删 useLayoutStore 死重复）、M2-b 面板边界收敛
> （5 个面板开关升 GOVERNED，`bmPanelOpen` 定为派生 implementation-detail）、M2-c `gridSession` owner 澄清
> （内存缓存失效纪元）；新增 checker **R8**（SEMANTIC_STATE_MULTI_OWNER）+ `check-semantic-closure-logic.mjs`；
> 真实扫描 fail=0（`SEMANTIC_REGISTRY_RESULT=PASS`）。
>
> ✅ **Phase 6B Semantic Writer Enforcement 已收口**（tag semantic-phase6b-writer-enforcement-pass）：
> 把「Owner 唯一」提升为「Owner 唯一 + Writer 唯一 + Checker 可证明」；新增 checker **R9**
> （SEMANTIC_STATE_WRITER_VIOLATION，registry 驱动 + 函数作用域 + 读/写区分），机器化 `gridSession`
> 函数级单写者（Debt-6A-2 收口）；覆盖全部 7 个 `single_owner_required` 状态；Registry schema 零改动
> （既有 canonical_writer/forbidden_writers/owner/derived/single_owner_required 已齐备，无需扩 YAML）；
> 真实扫描 fail=0、self-test ALL_PASS（R1..R9）、closure-logic 27/27。

> ✅ **Semantic Governance v1 Final Acceptance 已收口并发布（RELEASED）**（tag `semantic-governance-v1` → commit `316130d`；master 已快进至该提交并 `git push origin master`，tag 已 `git push origin semantic-governance-v1`）：
> Registry（State/Intent/Owner/Writer/SideEffect 四源）+ Checker（R1–R9 + closure + sensitive-side-effect）
> + Gate（pre-merge Phase 03）+ Recovery（snapshot/diagnostics/rollback/git-integrity）四支柱齐备；
> 全 13 个语义治理阶段 = CLOSED。纯验收文档交付（`docs/delivery/`），未改业务代码、未扩大治理域。
>
> 🔒 **RELEASED / FROZEN**（2026-09-19 远程发布确认）：`master == origin/master == 316130d`；tag `semantic-governance-v1` 已 push（`a4131de` 即该 tag 对象自身，解引用为 `316130d`）。Remote = PUSHED。下一步：**Product Evolution / New Feature Development**。禁止移动或删除该 tag；新功能在已治理语义骨架上复用 canonical intent / owner。

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
Phase 6A Core Closure（迁移）  CLOSED   （tag: semantic-phase6a-core-closure-pass）
Phase 6B Writer Enforcement        CLOSED   （tag: semantic-phase6b-writer-enforcement-pass）
Semantic Governance v1      CLOSED / RELEASED   （tag: semantic-governance-v1, Baseline: 316130d, Remote: PUSHED）
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
branch:    master（已快进至 Semantic Governance v1 验收点；feature/phase6b-writer-enforcement 现已并入 master 历史）
HEAD:      316130d（docs(delivery): Semantic Governance v1 final acceptance + handoff）
remote:    origin/master == 316130d（已 PUSHED；fast-forward e05160a..316130d）
tag:      semantic-governance-v1 已 push origin（-> 316130d；a4131de 为该 tag 对象本身）
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
semantic-phase6a-core-closure-pass        -> <phase6a-tip>
semantic-phase6b-writer-enforcement-pass   -> <phase6b-tip>
semantic-governance-v1               -> <gov-v1-tip>（Semantic Governance v1 总验收基线：Registry+Checker+Gate+Recovery）

semantic-registry-v1:  NOT EXISTS（原 Phase 1.6 可选动作；本次改以 semantic-governance-v1 作为总验收 tag，见 §4j）
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
scripts/check-semantic-registry.mjs（R1-R9）/ scripts/pre-merge.sh（已接入）
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

## 4h. Phase 6A Core Closure（Semantic Migration）

```text
docs/architecture/semantic-registry/states.yaml（aiNavOpen / gridSession / sidebarOpen / clipOpen / fileEditorOpen / browserDockOpen / browserDockTab 升 GOVERNED；observed 对应移除）
docs/architecture/semantic-registry/owners.yaml（browser_grid_lifecycle + aiNavOpen / gridSession）
scripts/check-semantic-registry.mjs（R8 SEMANTIC_STATE_MULTI_OWNER / SEMANTIC_DERIVED_PANEL_STORED + fixtures）
scripts/check-semantic-closure-logic.mjs（Phase 6A 静态+真实 store 功能测试，26 断言）
scripts/pre-merge.sh（Phase 03 gate 接入 check-semantic-closure-logic）
docs/architecture/semantic-governance/phase6a-core-closure/{ADR,panel-state-decision,MIGRATION-REPORT,CHECKER-REPORT,TEST-REPORT,FINAL-REPORT}.md
```

---

## 4i. Phase 6B Writer Enforcement（Semantic Writer Enforcement）

```text
docs/architecture/semantic-registry/states.yaml（gridSession canonical_writer/buildGrid/forceGridRelayout 已在 Phase 6A 登记；本阶段零改动 schema）
docs/architecture/semantic-registry/{owners,intents,side-effects}.yaml（未被修改）
scripts/check-semantic-registry.mjs（R9 SEMANTIC_STATE_WRITER_VIOLATION / findFunctionRanges / enclosingFunction + 夹具）
scripts/check-semantic-closure-logic.mjs（gridSession 函数级 writer 唯一静态断言，27 断言）
scripts/pre-merge.sh（Phase 03 gate 已含 check-semantic-registry / check-semantic-closure-logic，自动覆盖 R9）
docs/architecture/semantic-governance/phase6b-writer-enforcement/{ADR,FINAL-REPORT,CHECKER-REPORT,TEST-REPORT,MIGRATION-REPORT}.md
```

R9 判定手段（registry 驱动，不硬编码）：
  - 仅对 single_owner_required === true 且声明 canonical_writer 的存储态强制（7 个：aiNavOpen / gridSession / sidebarOpen / clipOpen / fileEditorOpen / browserDockOpen / browserDockTab）。
  - owner 文件内：写入须位于 canonical_writer 函数体内（brace 配对取最内层 enclosing 函数）；否则 FAIL。
  - 非 owner 文件（组件 / 其它 store / composable）：任何 .value= 直写 = FAIL。
  - 读取（.value 后非赋值，含 === / =>）不误报。
  - Terminal/Bookmark/credential 等无 single_owner_required 标志 → 自动跳过（不扩大范围）。
```

---

## 4j. Semantic Governance v1 Final Acceptance

```text
docs/delivery/SEMANTIC_GOVERNANCE_V1_ACCEPTANCE.md（全局架构验收：五模型/四链路/Checker/Gate/Recovery）
docs/delivery/PHASE_ACCEPTANCE_MATRIX.md（Phase 0 → 6B + v1 验收矩阵 + tag）
docs/delivery/KNOWN_DEBT_V1.md（已知债务总表：已关闭 5 条 / 保留 7 条语义内 + 产品级债）
docs/delivery/{README,01-PROJECT-OVERVIEW,02-ARCHITECTURE-EVOLUTION,03-SEMANTIC-GOVERNANCE,
              04-PHASE-RESULTS,05-CHECKER-QUALITY,06-RECOVERY-CAPABILITY,07-KNOWN-DEBT,08-ROADMAP}.md
scripts/check-semantic-registry.mjs（R1-R9，self-test ALL_PASS）
scripts/check-semantic-closure-logic.mjs（27 断言）
scripts/check-sensitive-side-effects.mjs（R5/R7）
scripts/pre-merge.sh（Phase 03 gate 含上述三语义 Checker；exit 0/1）
```

四支柱验收结论：
  - Registry：states/intents/owners/side-effects 四 YAML 均被 Checker 解析（self-test ALL_PASS），格式有效无漂移。
  - Checker：R1-R9 全在 check-semantic-registry.mjs；closure 27/27；sensitive-side-effect 独立脚本。
  - Gate：pre-merge Phase 03 循环含 check-semantic-registry / check-semantic-closure-logic / check-sensitive-side-effects；违规 exit 1、通过 exit 0。
  - Recovery：.snapshots/ + diagnostics/ + phase1.7-git-integrity/RECOVERY-PROCEDURE.md + git-recover.sh；git fsck 演练已闭环。
  - 性质：纯文档/验收交付，未改业务代码（src/ src-tauri/）、未新增治理域、未扩大 M4。

---

## 4k. Capability Platform（Phase 7A — 审计与设计）

```text
分支: feature/capability-platform-v1（基线 semantic-governance-v1 / 316130d；开工点 c0069b7）
文档: docs/architecture/capability-platform/phase7a-capability-architecture/
      {Capability-Inventory,Dependency-Graph,Lifecycle-Model,
       Resource-Model,Shell-Boundary,Migration-Plan,FINAL-REPORT}.md
试点选择: Bookmark（LIGHT · 无 native · 已登记 Owner useBookmarkStore）
今晚真实状态: COMPATIBILITY_WRAPPED —— 无能力可物理启停（不可写成 TARGET 已实现）
推迟: Browser / Grid / Terminal / Plugin / Agent / Skill = TARGET_COMPOSABLE（需单独 ADR/SCR）
资源口径: DECLARED RESOURCE CLASS（无按能力实测，Debt-7A-1）
边界: Runtime 只做编排，绝不成为业务状态 Owner；Capability Registry 不得重复 Semantic Registry 事实
tag: capability-phase7a-architecture-pass
```

> Semantic Governance v1 仍是**冻结基线**：新 State/Intent/Owner/Writer/SideEffect 必须先走 SCR。

---

## 4l. Capability Registry（Phase 7B — 契约与登记表）

```text
docs/architecture/capability-registry/
  capabilities.yaml   18 个 CAPABILITY 的 manifest（id/name/category/provides/dependsOn/
                      optionalDependencies/lifecycle/resources/permissions/persistence/entrypoint）
  dependencies.yaml   依赖边 + 共享基础设施(bridge/pinia/security_policy/shell) + 禁止边 + 已知耦合
  resources.yaml      Resource Class(13) / Resource Lifecycle(5) / 每能力 policy + 测量现状
  README.md           manifest 字段契约 + 硬规则 R-A..R-H + Capability SDK 最小接口

src/capability/types.ts   CapabilityDefinition / CapabilityLifecycle /
                          CapabilityContext / CapabilityResourcePolicy（仅类型，无实现、无 DI）
scripts/check-capability-registry.mjs   C1..C10，--help/--self-test/--json/--strict
  自检 13/13 PASS（1 positive + 9 negative + 3 false-positive 夹具）
  真实扫描 fail=0 warn=0；--strict 亦 PASS

未登记 Owner（activatable 强制 false）: skill / notes / script  → governanceStatus OWNER_PENDING_SCR
LOCKED: plugin（runtime 冻结，不可装配）
tag: capability-phase7b-contract-pass
```

---

## 4m. Capability Runtime（Phase 7C — 最小运行时）

```text
src/capability/runtime.ts   createCapabilityRuntime(): register/resolve/enable/disable/
                            activate/suspend/inspect/reset
                            关键拒绝: activate 要求 status=COMPATIBILITY_WRAPPED 且 activatable=true
                            → 从机制上杜绝"把 TARGET/NOT_INTEGRATED 当已实现"
                            disable 仅允许"已治理 + 非常驻"能力
scripts/check-capability-runtime.mjs   RT-01..RT-15，esbuild 转译真实 TS 后测试 → 15/15 PASS
边界: Runtime 只持有 id/definition/state/enabled（编排元数据），
      不含 tabs/termPanes/items/credential/gridSession（RT-13 静态断言强制）
未引入: DI 容器 / 反射加载 / God Runtime
债务: Debt-7C-1 无物理卸载(destroy/hibernate 未实现) · Debt-7C-2 无能力间通信 · Debt-7C-3 inspect 无实测
tag: capability-phase7c-runtime-pass
```

---

## 5. Current Task Status

```text
Completed: Phase 0 / 1 / 1.5 / 1.6 / 1.7 / 2 / 3 / 4 / 5 / 5.1 / 6A / 6B / Semantic Governance v1 全部完成

Pending: 无

Blocked: 无

Next recommended task:
  Semantic Governance v1 已 CLOSED（tag semantic-governance-v1）。
  语义治理主线（Phase 1.7→2→3→4→5）+ Closure Audit v1 + Phase 6A + Phase 6B + v1 验收全部收口。
  停止条件已达成：**未进入新治理域、未开始新业务开发，等待下一阶段**。
  Next: Product Evolution / New Feature Development（在已治理语义骨架上做新功能，复用 canonical intent / owner）
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
Debt-6A-1 M4 其余 14 域仍为 observed_not_governed   KNOWN DEBT（Phase 6A 范围外；须另派 SCR/迁移，非本阶段目标）
Debt-6A-2 gridSession “函数级”单写者未机器强制     CLOSED（Phase 6B：R9 机器化 writer 约束，buildGrid/forceGridRelayout 为唯一合法 writer）
Debt-6A-3 R8 声明形态仅识别 const X = ref/reactive  KNOWN DEBT（与 R2 同源盲区：解构/动态声明不识别；登记于 Known-Debt）
Debt-6B-1 R9 brace 配对对“无参 parenless 箭头”函数不识别  KNOWN DEBT（仅影响极少数 `const f = x => {...}` 写法；真实代码写入点均为 `function NAME()` 形式，未触发误报；若未来出现 parenless 箭头 writer 需补识别）
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
禁止削弱任何 Checker（R1..R9 / check-git-repo-integrity 的 ^error: 判定）
禁止 blind git reset / 删被引用对象（恢复只走 git-recover.sh 护栏）
```

**可选（固化 release tag）：**

```bash
git tag -a semantic-registry-v1 -m "Semantic Registry + Checker + pre-merge gate; Phase 2/3/4 governance PASS"
# 本地，不推送
```

**快速自查：**

```bash
node scripts/check-semantic-registry.mjs --self-test   # ALL_PASS（R1..R9）
node scripts/check-semantic-registry.mjs              # fail=0（warn 非阻断）
node scripts/check-semantic-closure-logic.mjs          # SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (27/27)
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

Phase 6A 验收证据（2026-09-19）：
  SEMANTIC REGISTRY self-test: ALL_PASS（R1..R8；+R8 第二 owner / 派生面板存态 / 非 owner 声明 NEG）
  SEMANTIC REGISTRY real scan: PASS（fail=0；warn=6 pre-existing R5；info=72）
  SEMANTIC_CLOSURE_LOGIC: PASS（26/26；aiNavOpen 打开关闭 writer 唯一 / 5 面板独立不误合并 / gridSession 自增+非持久化）
  aiNavOpen: PASS（useBrowserStore 唯一声明；useLayoutStore 死重复已删；toggleAiNav 唯一 toggle 入口）
  Panel: PASS（5 面板各单一 owner；bmPanelOpen 保持 MainArea computed 派生）
  gridSession: PASS（useBrowserStore 唯一 owner；仅 buildGrid/forceGridRelayout 写入；不落盘）
  NO REGRESSION: PASS（R1..R7 未削弱；未改语义行为；lint 0 error；git diff --check 干净）
  KNOWN_DEBT: Debt-6A-1/6A-2/6A-3 显式登记（M4 其余 14 域 / 函数级单写者 / R8 声明形态盲区）

Phase 6B 验收证据（2026-09-19）：
  SEMANTIC REGISTRY self-test: ALL_PASS（R1..R9；+R9 positive / Negative R9a(跨store) / R9b(组件) / R9c(owner内非canonical) / 读取 false-positive 夹具）
  SEMANTIC REGISTRY real scan: PASS（fail=0；warn=6 pre-existing R5；info=72）—— R9 零新增失败
  SEMANTIC_CLOSURE_LOGIC: PASS（27/27；gridSession 两处写入均在 canonical_writer 函数 buildGrid/forceGridRelayout 内）
  aiNavOpen: PASS（owner=useBrowserStore；writer=toggleAiNav/gotoAI 均 canonical；R9 不误报）
  gridSession: PASS（owner=useBrowserStore；writer=buildGrid/forceGridRelayout 均 canonical；非 owner 直写被 R9 拦）
  R9_CHECKER: PASS（registry 驱动 + 函数作用域 + 读/写区分；不硬编码 allow-list）
  NO REGRESSION: PASS（R1..R8 未削弱；未改业务代码；node --check 两脚本语法 OK；git diff --check 干净）
  KNOWN_DEBT: Debt-6A-2 CLOSED（R9 机器化）；Debt-6A-1/6A-3 保留；新增 Debt-6B-1（parenless 箭头盲区，未触发误报）

Semantic Governance v1 验收证据（2026-09-19）：
  REGISTRY self-test: ALL_PASS（R1..R9 全部 positive/negative/false-positive 夹具）
  REGISTRY real scan: PASS（fail=0；warn=6 pre-existing R5；info=72）
  CLOSURE_LOGIC: PASS（27/27）
  SENSITIVE_SIDE_EFFECT self-test: PASS（S1/S2）
  SENSITIVE_SIDE_EFFECT real scan: PASS（fail=0）
  GATE: pre-merge Phase 03 含 check-semantic-registry / check-semantic-closure-logic / check-sensitive-side-effects；exit 0 通过 / exit 1 阻断
  RECOVERY: .snapshots/ + diagnostics/ + RECOVERY-PROCEDURE.md + git-recover.sh；git fsck 演练闭环
  DELIVERY DOCS: docs/delivery/{SEMANTIC_GOVERNANCE_V1_ACCEPTANCE,PHASE_ACCEPTANCE_MATRIX,KNOWN_DEBT_V1,README,01..08}.md 全交付
  NO CODE MODIFIED: 未改 src/ src-tauri/；未新增治理域；未扩大 M4
  KNOWN_DEBT: 已关闭 Debt-5-1/5-2/5-3/6A-aiNavOpen/6A-2；保留 Debt-001~004/6A-1/6A-3/6B-1 + 产品级债（见 KNOWN_DEBT_V1.md）
  SEMANTIC_GOVERNANCE_V1_RESULT: PASS（tag semantic-governance-v1）
```
