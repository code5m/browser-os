# HANDOFF — CURRENT PROJECT STATE

> 下一 Agent 无需重扫全仓即可继续。本文件为**当前真实状态快照**，事实来自 `git` 与已落地文件。
> 最后更新：2026-09-19
>
> ✅ **git 对象损坏已清理**（见 Phase 1.7）：经验证「不被任何引用包含」后，
> 经 `scripts/git-recover.sh --prune-orphans` 安全删除；`git fsck --full` 现已 0 error。
>
> ✅ **Phase 2 Workspace/FilePanel 语义治理已收口**：useWorkspaceStore.ts + FilePanel.vue 纳入
> Semantic Registry 治理；新增 R6（派生状态禁止存储/赋值）固化 `currentLocalPath` 派生不变量；
> 真实仓库扫描 `fail=0`（`SEMANTIC_REGISTRY_RESULT=PASS`）。

---

## 1. Current Milestone

```text
Phase 0                     CLOSED   （Single Semantics Governance）
Phase 1 Browser/Grid        CLOSED   （tag: semantic-phase1-browser-grid-pass）
Phase 1.5 Semantic Registry  CLOSED   （Registry + Checker + SCR + 接入 pre-merge）
Phase 1.6 Semantic Reg. Acc  PASS    （验收闭环：registry/checker/pre-merge 均通过）
Phase 1.7 Git Integrity      CLOSED   （tag: semantic-phase1.7-git-integrity-pass）
Phase 2 Workspace/FilePanel  CLOSED   （tag: semantic-phase2-workspace-pass）
```

Phase 2 验收结论：

```text
SEMANTIC REGISTRY self-test: PASS  （R1..R6 全部 positive/negative/false-positive 夹具 ALL_PASS）
SEMANTIC REGISTRY real scan:  PASS  （fail=0；warn=6 为 pre-existing R5 提示级，非阻断）
R6 DERIVED GUARD:           PASS  （currentLocalPath 为 computed 且无 .value=；注入 ref+.value= 夹具被检出）
R2 WORKSPACE GOVERN:        PASS  （useWorkspaceStore.ts 42 声明 = 5 治理 + 39 observed，无遗漏 FAIL）
NO REGRESSION:              PASS  （R1..R5 逻辑未削弱；self-test 仍 ALL_PASS）
```

---

## 2. Git State

```text
branch:    master（Phase 2 以 feature/phase2-workspace 实现，ff-merge 入 master）
HEAD:      semantic-phase2-workspace-pass（annotated tag = master tip）
working tree: 干净（仅未跟踪 .snapshots/ 与 diagnostics/ —— 取证产物，不入库）
```

### latest commits（Phase 2 在其上）

```text
<phase-tip>  docs(phase2): closeout + handoff update
<phase-feat> feat(phase2): workspace/filepanel semantic governance
b36703e      docs(phase1.7): closeout + handoff update
77a6b57      feat(phase1.7): git integrity gate + recovery procedure
c209325      docs(handoff): rebuild current-state handoff; mark Phase 1.6 acceptance PASS
2b030eb      gates(phase1.5): wire semantic registry gate into pre-merge
3874bef      feat(phase1.5): semantic registry + semantic gate (governance infra only)
2ca7fb3      docs(phase1): closeout - final report + known debt registry
```

### latest tags（及分类）

```text
semantic-phase0-infra-pass        -> 481fbf7   [governance milestone]  Phase 0 基础设施通过
semantic-phase0-policy-pass        -> 04e4cbc   [governance milestone]  Phase 0 策略通过
semantic-phase1-browser-grid-code-pass -> a30fd57 [code milestone]      Phase 1 代码+静态门禁通过
semantic-phase1-browser-grid-pass       -> 7e8d867 [architecture milestone] Phase 1 完整验收（含 GUI）
semantic-phase1.7-git-integrity-pass    -> b36703e [infra/governance milestone] Phase 1.7 仓库完整性+恢复
semantic-phase2-workspace-pass          -> <phase-tip> [architecture milestone] Phase 2 Workspace/FilePanel 语义治理

semantic-registry-v1:  NOT EXISTS（属原 Phase 1.6 可选动作，本次未创建；如需创建见 §7）
```

---

## 3. Frozen Architecture Rules（不可违反）

```text
mainView
  = 用户当前希望看到的 Main Surface
  owner = useLayoutStore（setView 是唯一底层写入口）

gridOpen
  = Grid resource 是否存在
  owner = useBrowserStore

desiredGridVisibility
  = gridOpen && mainView === "grid"（纯派生，禁止存储）

isBrowserVisible
  = mainView === "browser"（不得重新耦合 gridOpen）

禁止:
  gridVisible stored state         （第二真源，C4/C5 常驻守护）
  exitGrid(mode)                   （多义万能 API，已 ADR 否决，出现即阻断）
  GridLifecycle enum / 新 Rust show_hide / 拆 position↔show
  （保留 position → show 既有 Native 契约）

Owner 收敛:
  View Navigation   = useLayoutStore
  Browser/Grid Life = useBrowserStore
  Native Execution  = useBrowserHost / bridge / Rust（只执行，不得成 domain owner）

Phase 2 新增（Workspace/FilePanel，owner = useWorkspaceStore）:
  filePath      = 文件浏览器/覆盖编辑器当前激活位置（地址真源；overloaded 语义冻结为单一存储态）
  inlineFile    = 行内编辑器当前打开文件（与覆盖编辑器隔离）
  previewDir    = 目录预览当前目录
  pathInput     = 地址栏 UI 缓冲（镜像 filePath，非真源）
  currentLocalPath [DERIVED] = inlineFile || previewDir || modTabs.path || filePath
                 禁止任何 .value = 写入 / ref-reactive 声明（R6 护栏；类比 desiredGridVisibility）
  组件禁止直写 filePath/inlineFile/previewDir/pathInput（R3 violation COMPONENT_WRITES_WORKSPACE_PATH）
  openFile / openFileInline / enterDir / saveFile / closeFileEditor 为单一意图入口
  禁止 mergeOpenFileIntoOpenFileInline（rejected intent）

Phase 1.7 新增（仓库完整性，ops 层，不进产品 Semantic Registry）:
  git 损坏对象 = 0 字节松散对象 / cat-file 失败的松散对象
  恢复 = 仅删「损坏且不可达」的松散对象；禁止 blind reset / 禁止删被引用对象
```

---

## 4. Semantic Registry 入口

下一 Agent 修改**任何**状态 / Intent / API / Owner / Side Effect 前，**必须先查 Registry**：

```text
docs/architecture/semantic-registry/
  states.yaml       # 11 受治理状态（6 Phase1 + 5 Phase2）+ observed_not_governed
  intents.yaml      # 15 intent（10 Phase1 + 5 Phase2）+ rejected(exitGrid, mergeOpenFileIntoOpenFileInline) + proposed_not_present
  owners.yaml       # 5 owner（含 workspace_filepanel）；owner-only vs public intent 区分
  side-effects.yaml # 7 副作用（含 Phase2 writeFile 文档级登记）
  README.md         # 分类：CURRENT FACT / TARGET CONTRACT / ACCEPTED ADR / PROPOSED CHANGE / KNOWN DEBT
```

配套：

```text
scripts/check-semantic-registry.mjs   # R1-R6，--self-test / --json / --strict
scripts/pre-merge.sh                  # 已接入（正式门禁 + self-test 两段）
docs/architecture/semantic-changes/SCR-template.md  # 新增语义必须走 SCR
docs/architecture/semantic-changes/SCR-20260919-workspace-filepanel.md  # Phase 2 SCR
```

流程：`查 Registry → 已有则用之 → 无则填 SCR → Review → ADR → 更新 Registry → 才许写代码`

---

## 4b. Git Integrity / Recovery 入口（Phase 1.7 新增，ops 层）

下一 Agent 在**每个 Phase 开始前**与**怀疑仓库损坏时**：

```text
scripts/snapshot.sh                  # 只读快照 HEAD/branch/tags/dirty → .snapshots/
scripts/check-git-repo-integrity.sh  # 只读门禁：git fsck --full 解析；^error:→FAIL
scripts/git-recover.sh               # --diagnose / --prune-orphans（受控清理孤儿损坏）
scripts/pre-merge.sh                 # 已接入 check-git-repo-integrity（正式 + self-test）
docs/architecture/semantic-governance/phase1.7-git-integrity/Phase1.7-design.md
docs/architecture/semantic-governance/phase1.7-git-integrity/RECOVERY-PROCEDURE.md
```

---

## 4c. Workspace/FilePanel Governance 入口（Phase 2 新增）

下一 Agent 修改 **useWorkspaceStore.ts / FilePanel.vue** 中文件导航语义前：

```text
docs/architecture/semantic-registry/states.yaml    # filePath/inlineFile/previewDir/pathInput/currentLocalPath 受治理
docs/architecture/semantic-registry/intents.yaml   # openFile/openFileInline/enterDir/saveFile/closeFileEditor
docs/architecture/semantic-registry/owners.yaml     # workspace_filepanel owner
scripts/check-semantic-registry.mjs                # R6（派生状态禁止存储）+ R2/R3/R4 workspace 域
docs/architecture/semantic-governance/phase2-workspace/Phase2-design.md
docs/architecture/semantic-governance/phase2-workspace/PHASE_2_CLOSEOUT_RESULT.md
```

---

## 5. Current Task Status

```text
Completed:
  Phase 0 / 1 / 1.5 / 1.6 / 1.7 / 2 全部完成

Pending:
  无（Phase 2 验收 PASS；semantic-registry-v1 tag 仍可选，未创建）

Blocked:
  无

Next recommended task:
  Phase 3 — Bookmark Semantic Governance（tag: semantic-phase3-bookmark-pass）
  注意：Phase 3 须在独立 feature/phase3-bookmark 分支，独立提交/打 tag/更新 Handoff
```

---

## 6. Known Debt

```text
Debt-001  Grid UDS socket cleanup
  状态:    KNOWN DEBT
  来源:    sock/ 累计残留
  当前 Phase: 不处理
  禁止误处理: 清理会碰用户目录，非业务代码范围

Debt-002  toggleGridToolbar dead code
  状态:    KNOWN DEBT
  当前 Phase: 不处理
  禁止误处理: 不要补 import / 删除 / 重构（零 caller，改动会引入循环依赖风险）

Debt-003  closeGridCell orphan API
  状态:    KNOWN DEBT
  当前 Phase: 不处理
  禁止误处理: 不要删除；不要新增 caller 绕过 SCR

Debt-004  Terminal checker debt
  状态:    KNOWN DEBT
  来源:    check-terminal-policy.py / check-terminal-ui-logic.mjs 自检失败
  当前 Phase: 不处理（Phase 外）
  禁止误处理: 禁止修改 Terminal；该失败是 pre-merge --self-test 的既有 FAIL，非本 Phase 引入

Debt-1.7-1  pack 内损坏对象无自动恢复
  状态:    KNOWN DEBT
  当前 Phase: 记录（超出 Phase 1.7 范围，交专项）
  禁止误处理: 不要盲目 git gc 掩盖 pack 损坏

Debt-1.7-2  无周期性后台完整性巡检
  状态:    KNOWN DEBT
  当前 Phase: 记录（交 ops）
  禁止误处理: 不要为每次 commit 自动 gc

Debt-2-1  filePath overloaded 语义（编辑器=文件 / 浏览器=目录）未拆分
  状态:    KNOWN DEBT
  来源:    useWorkspaceStore filePath 单存储态承载两种语义
  当前 Phase: 不处理（需业务重构，超出治理范围）

Debt-2-2  R2 正则不识别 reactive<Set/Map>(new ...) 嵌套泛型
  状态:    KNOWN DEBT
  来源:    checker 正则盲区；selected/treeChildren/treeExpanded/treeLoading/treeErrors 不受 R2 治理
  当前 Phase: 不处理（交专项，不影响本 Phase 治理目标）

Debt-2-3  writeFile 副作用仅文档化（requires_declaration=false）
  状态:    KNOWN DEBT
  来源:    Phase 2 为避免对既有合法调用产生 R5 噪声，未强制调用点加 side-effect 标记
  当前 Phase: 不处理（收紧需改 side-effects.yaml + 业务代码加标记）
```

---

## 7. Next Agent Instructions

**开始前先读取（不要重扫全仓）：**

```text
docs/architecture/HANDOFF_CURRENT_STATE.md
docs/architecture/semantic-registry/README.md
docs/architecture/semantic-registry/states.yaml
docs/architecture/semantic-registry/intents.yaml
docs/architecture/semantic-registry/owners.yaml
docs/architecture/semantic-registry/side-effects.yaml
docs/architecture/semantic-governance/phase1-browser-grid/FINAL-REPORT.md
docs/architecture/semantic-governance/Known-Debt.md
docs/architecture/semantic-governance/phase1.7-git-integrity/Phase1.7-design.md
docs/architecture/semantic-governance/phase1.7-git-integrity/RECOVERY-PROCEDURE.md
docs/architecture/semantic-governance/phase2-workspace/Phase2-design.md
docs/architecture/semantic-governance/phase2-workspace/PHASE_2_CLOSEOUT_RESULT.md
```

**禁止：**

```text
禁止重新扫描全仓历史上下文（以上文档已固化当前状态）
禁止修改业务代码（src/ src-tauri/）除非对应 Phase 明确要求
禁止清理 Known Debt（Debt-001~004 / Debt-1.7-1~2 / Debt-2-1~3）
禁止扩大 Semantic Registry 范围（先走 SCR；治理域外不判失败）
禁止削弱任何 Checker（尤其 check-semantic-registry 的 R1..R6 / check-git-repo-integrity 的 ^error: 判定）
禁止 blind git reset / 删被引用对象（恢复只走 git-recover.sh 护栏）
```

**可选（如需固化 Phase 1.6 / 1.7 / 2 为 release tag）：**

```bash
git tag -a semantic-registry-v1 -m "Semantic Registry + Checker + pre-merge gate accepted; Phase 2 workspace governance PASS"
# 本地，不推送
```

**快速自查命令：**

```bash
node scripts/check-semantic-registry.mjs --self-test   # 应 ALL_PASS（R1..R6）
node scripts/check-semantic-registry.mjs              # 应 fail=0（warn 非阻断）
bash scripts/check-git-repo-integrity.sh --self-test  # 应 SELF_TEST: PASS
bash scripts/check-git-repo-integrity.sh              # 真实仓库应 GIT_INTEGRITY: PASS
bash scripts/pre-merge.sh --self-test                 # semantic + git-integrity 项应绿（terminal 债为既有 FAIL）
```

---

## 8. 当前验证状态（Phase 2 验收证据）

```text
SEMANTIC REGISTRY self-test:
  ALL_PASS  （R1..R6 positive/negative/false-positive 夹具全过；R6 检出派生状态被存为 ref+.value=）
SEMANTIC REGISTRY real scan:
  PASS  （fail=0；warn=6 为 pre-existing R5 提示级，非阻断；info=76）
R6 DERIVED GUARD:
  PASS  （currentLocalPath 为 computed 且无 .value=；注入 ref 声明 + .value= 夹具被 R6 检出）
R2 WORKSPACE GOVERN:
  PASS  （useWorkspaceStore.ts 全部 44 声明 = 5 受治理 + 39 observed_not_governed，无遗漏 FAIL）
NO REGRESSION:
  PASS  （R1..R5 逻辑未削弱；self-test 仍 ALL_PASS；未改任何业务代码）
KNOWN_DEBT isolated:
  PASS  （Debt-001~004 / Debt-1.7-1~2 / Debt-2-1~3 均显式记录，未被触碰或隐藏）
```
