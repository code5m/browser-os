# HANDOFF — CURRENT PROJECT STATE

> 下一 Agent 无需重扫全仓即可继续。本文件为**当前真实状态快照**，事实来自 `git` 与已落地文件。
> 最后更新：2026-09-19
>
> ✅ **git 对象损坏已清理**（Phase 1.7）：`git fsck --full` 现已 0 error。
>
> ✅ **Phase 2/3/4 语义治理已收口**：Workspace/FilePanel（tag phase2）、Bookmark（tag phase3）、
> Terminal Lifecycle（tag phase4）均纳入 Semantic Registry；真实扫描 fail=0。

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
```

Phase 4 验收结论：

```text
SEMANTIC REGISTRY self-test: PASS  （R1..R6 全部 positive/negative/false-positive 夹具 ALL_PASS）
SEMANTIC REGISTRY real scan:  PASS  （fail=0；warn=6 pre-existing R5 非阻断）
R2 TERMINAL GOVERN:           PASS  （useSystemStore.ts 16 声明全登记，无第二面板列表 FAIL）
R4 TERMINAL INTENT:           PASS  （terminal 重复名 newTerm 等未定义；注入 newTerm 定义被 R4 检出）
NO REGRESSION:                PASS  （R1..R6 未削弱；self-test 仍 ALL_PASS）
```

---

## 2. Git State

```text
branch:    master（Phase 4 以 feature/phase4-terminal 实现，ff-merge 入 master）
HEAD:      semantic-phase4-terminal-pass（annotated tag = master tip）
working tree: 干净（仅未跟踪 .snapshots/ 与 diagnostics/ —— 取证产物，不入库）
```

### latest commits（Phase 4 在其上）

```text
<phase-tip>  docs(phase4): closeout + handoff update
<phase-feat> feat(phase4): terminal lifecycle semantic governance
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
semantic-phase4-terminal-pass           -> <phase-tip>

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

## 5. Current Task Status

```text
Completed: Phase 0 / 1 / 1.5 / 1.6 / 1.7 / 2 / 3 / 4 全部完成

Pending: 无

Blocked: 无

Next recommended task:
  Phase 5 — Credential Security Governance（tag: semantic-phase5-credential-pass）
  注意：Phase 5 须在独立 feature/phase5-credential 分支，独立提交/打 tag/更新 Handoff
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
node scripts/check-semantic-registry.mjs --self-test   # ALL_PASS（R1..R6）
node scripts/check-semantic-registry.mjs              # fail=0（warn 非阻断）
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
KNOWN_DEBT isolated: PASS（Debt-001~004 / 1.7-1~2 / 2-1~3 / 3-1~3 / 4-1~3 均显式记录）
```
