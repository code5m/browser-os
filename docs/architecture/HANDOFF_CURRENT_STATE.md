# HANDOFF — CURRENT PROJECT STATE

> 下一 Agent 无需重扫全仓即可继续。本文件为**当前真实状态快照**，事实来自 `git` 与已落地文件。
> 最后更新：2026-09-19
>
> ✅ **git 对象损坏已清理**（见 Phase 1.7）：经 `scripts/git-recover.sh --prune-orphans` 安全删除；
> `git fsck --full` 现已 0 error。
>
> ✅ **Phase 2 Workspace/FilePanel 语义治理已收口**（tag: semantic-phase2-workspace-pass）：
> useWorkspaceStore.ts + FilePanel.vue 纳入 Registry；R6 固化 currentLocalPath 派生不变量；真实扫描 fail=0。
>
> ✅ **Phase 3 Bookmark 语义治理已收口**（tag: semantic-phase3-bookmark-pass）：
> useBookmarkStore.ts 6 状态纳入治理（items 唯一真源 / sorted 派生）；5 意图单一入口；
> 与主页快捷方式显式隔离（rejected mergeBookmarksIntoHome）；真实扫描 fail=0。

---

## 1. Current Milestone

```text
Phase 0                     CLOSED   （Single Semantics Governance）
Phase 1 Browser/Grid        CLOSED   （tag: semantic-phase1-browser-grid-pass）
Phase 1.5 Semantic Registry  CLOSED   （Registry + Checker + SCR + 接入 pre-merge）
Phase 1.6 Semantic Reg. Acc  PASS    （验收闭环：registry/checker/pre-merge 均通过）
Phase 1.7 Git Integrity      CLOSED   （tag: semantic-phase1.7-git-integrity-pass）
Phase 2 Workspace/FilePanel  CLOSED   （tag: semantic-phase2-workspace-pass）
Phase 3 Bookmark             CLOSED   （tag: semantic-phase3-bookmark-pass）
```

Phase 3 验收结论：

```text
SEMANTIC REGISTRY self-test: PASS  （R1..R6 全部 positive/negative/false-positive 夹具 ALL_PASS）
SEMANTIC REGISTRY real scan:  PASS  （fail=0；warn=6 为 pre-existing R5 提示级，非阻断）
R2 BOOKMARK GOVERN:          PASS  （useBookmarkStore.ts 6 声明全登记，无第二 Bookmark[] FAIL）
R6 DERIVED GUARD:            PASS  （sorted 为 computed 且无 .value=；注入 ref 夹具被 R6 检出）
NO REGRESSION:               PASS  （R1..R6 逻辑未削弱；self-test 仍 ALL_PASS）
```

---

## 2. Git State

```text
branch:    master（Phase 3 以 feature/phase3-bookmark 实现，ff-merge 入 master）
HEAD:      semantic-phase3-bookmark-pass（annotated tag = master tip）
working tree: 干净（仅未跟踪 .snapshots/ 与 diagnostics/ —— 取证产物，不入库）
```

### latest commits（Phase 3 在其上）

```text
<phase-tip>  docs(phase3): closeout + handoff update
<phase-feat> feat(phase3): bookmark semantic governance
7e881f4      docs(phase2): closeout + handoff update
bff14af      feat(phase2): workspace/filepanel semantic governance
b36703e      docs(phase1.7): closeout + handoff update
77a6b57      feat(phase1.7): git integrity gate + recovery procedure
c209325      docs(handoff): rebuild current-state handoff; mark Phase 1.6 acceptance PASS
2b030eb      gates(phase1.5): wire semantic registry gate into pre-merge
3874bef      feat(phase1.5): semantic registry + semantic gate (governance infra only)
2ca7fb3      docs(phase1): closeout - final report + known debt registry
```

### latest tags（及分类）

```text
semantic-phase0-infra-pass        -> 481fbf7   [governance milestone]
semantic-phase0-policy-pass        -> 04e4cbc   [governance milestone]
semantic-phase1-browser-grid-code-pass -> a30fd57 [code milestone]
semantic-phase1-browser-grid-pass       -> 7e8d867 [architecture milestone]
semantic-phase1.7-git-integrity-pass    -> b36703e [infra/governance milestone]
semantic-phase2-workspace-pass          -> 7e881f4 [architecture milestone]
semantic-phase3-bookmark-pass           -> <phase-tip> [architecture milestone]

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

desiredGridVisibility / isBrowserVisible / currentLocalPath / sorted
  = 纯派生（derived:true），禁止存储、禁止 .value=（R6 护栏）

禁止:
  gridVisible stored state / exitGrid(mode) / GridLifecycle enum / 拆 position↔show
  filePath/inlineFile/previewDir/pathInput 被组件直写（owner: useWorkspaceStore）
  items/panelOpen 被组件直写，或维护第二份 Bookmark[]（owner: useBookmarkStore）
  收藏夹（后端持久化）与主页快捷方式（localStorage）合并（rejected mergeBookmarksIntoHome）

Owner 收敛:
  View Navigation   = useLayoutStore
  Browser/Grid Life = useBrowserStore
  Native Execution  = useBrowserHost / bridge / Rust
  Workspace/FilePanel = useWorkspaceStore
  Bookmark          = useBookmarkStore

Phase 2 新增（Workspace/FilePanel）:
  filePath      = 地址真源（overloaded：编辑器=文件 / 浏览器=目录）
  currentLocalPath [DERIVED] = inlineFile || previewDir || modTabs.path || filePath

Phase 3 新增（Bookmark）:
  items      = 收藏夹唯一真源（后端 data_dir/bookmarks.json 镜像）
  sorted     [DERIVED] = items 按 created_at 倒序
  toggle     = 地址栏 ⭐ 唯一入口
  add 必须按 id/normalizeUrl splice 替换（禁止 push 重复）
```

---

## 4. Semantic Registry 入口

下一 Agent 修改**任何**状态 / Intent / API / Owner / Side Effect 前，**必须先查 Registry**：

```text
docs/architecture/semantic-registry/
  states.yaml       # 22 受治理状态（6 Phase1 + 5 Phase2 + 6 Phase3 Phase? -> 实际 17）+ observed_not_governed
  intents.yaml      # 20 intent（10 Phase1 + 5 Phase2 + 5 Phase3）+ rejected + proposed_not_present
  owners.yaml       # 6 owner（含 workspace_filepanel / bookmark）；owner-only vs public intent 区分
  side-effects.yaml # 8 副作用（含 Phase2 writeFile / Phase3 bookmarkPersist 文档级登记）
  README.md         # 分类说明
```

配套：`scripts/check-semantic-registry.mjs`（R1-R6）/ `scripts/pre-merge.sh`（已接入）/
`docs/architecture/semantic-changes/SCR-template.md` / 已归档 SCR（20260919-workspace-filepanel / 20260919-bookmark）。

流程：`查 Registry → 已有则用之 → 无则填 SCR → Review → ADR → 更新 Registry → 才许写代码`

---

## 4b. Git Integrity / Recovery 入口（Phase 1.7 新增，ops 层）

```text
scripts/snapshot.sh / check-git-repo-integrity.sh / git-recover.sh
docs/architecture/semantic-governance/phase1.7-git-integrity/{Phase1.7-design,RECOVERY-PROCEDURE}.md
```

---

## 4c. Workspace/FilePanel Governance 入口（Phase 2 新增）

```text
docs/architecture/semantic-registry/states.yaml    # filePath/inlineFile/previewDir/pathInput/currentLocalPath
docs/architecture/semantic-registry/intents.yaml   # openFile/openFileInline/enterDir/saveFile/closeFileEditor
docs/architecture/semantic-registry/owners.yaml     # workspace_filepanel owner
scripts/check-semantic-registry.mjs                # R6 + R2/R3/R4 workspace 域
docs/architecture/semantic-governance/phase2-workspace/{Phase2-design,PHASE_2_CLOSEOUT_RESULT}.md
```

---

## 4d. Bookmark Governance 入口（Phase 3 新增）

```text
docs/architecture/semantic-registry/states.yaml    # items/loaded/busy/error/panelOpen/sorted
docs/architecture/semantic-registry/intents.yaml   # add/remove/toggle/importFile/togglePanel + rejected mergeBookmarksIntoHome
docs/architecture/semantic-registry/owners.yaml     # bookmark owner + COMPONENT_WRITES_BOOKMARK
scripts/check-semantic-registry.mjs                # R2/R4/R6 bookmark 域
docs/architecture/semantic-governance/phase3-bookmark/{Phase3-design,PHASE_3_CLOSEOUT_RESULT}.md
```

---

## 5. Current Task Status

```text
Completed:
  Phase 0 / 1 / 1.5 / 1.6 / 1.7 / 2 / 3 全部完成

Pending:
  无（Phase 3 验收 PASS；semantic-registry-v1 tag 仍可选，未创建）

Blocked:
  无

Next recommended task:
  Phase 4 — Terminal Lifecycle Governance（tag: semantic-phase4-terminal-pass）
  注意：Phase 4 须在独立 feature/phase4-terminal 分支，独立提交/打 tag/更新 Handoff
```

---

## 6. Known Debt

```text
Debt-001  Grid UDS socket cleanup              KNOWN DEBT（不处理）
Debt-002  toggleGridToolbar dead code          KNOWN DEBT（不处理）
Debt-003  closeGridCell orphan API             KNOWN DEBT（不处理）
Debt-004  Terminal checker debt               KNOWN DEBT（不处理；pre-merge --self-test 既有 FAIL，非本阶段引入）
Debt-1.7-1 pack 内损坏对象无自动恢复          KNOWN DEBT（交专项）
Debt-1.7-2 无周期性后台完整性巡检             KNOWN DEBT（交 ops）
Debt-2-1 filePath overloaded 语义未拆分       KNOWN DEBT（需业务重构，超出治理范围）
Debt-2-2 R2 正则不识别 reactive<Set/Map> 嵌套泛型  KNOWN DEBT（checker 盲区）
Debt-2-3 writeFile 副作用仅文档化             KNOWN DEBT（收紧需业务代码改动）
Debt-3-1 normalizeUrl 身份键未做 checker 强制  KNOWN DEBT（静态强制易误报，交专项）
Debt-3-2 panelOpen 置于 bookmark store 而非 layout KNOWN DEBT（有意设计，不移动）
Debt-3-3 bookmarkPersist 副作用仅文档化       KNOWN DEBT（同 Phase 2 writeFile 口径）
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
```

**禁止：**

```text
禁止重新扫描全仓历史上下文（以上文档已固化当前状态）
禁止修改业务代码（src/ src-tauri/）除非对应 Phase 明确要求
禁止清理 Known Debt（Debt-001~004 / 1.7-1~2 / 2-1~3 / 3-1~3）
禁止扩大 Semantic Registry 范围（先走 SCR；治理域外不判失败）
禁止削弱任何 Checker（R1..R6 / check-git-repo-integrity 的 ^error: 判定）
禁止 blind git reset / 删被引用对象（恢复只走 git-recover.sh 护栏）
```

**可选（如需固化 Phase 1.6/1.7/2/3 为 release tag）：**

```bash
git tag -a semantic-registry-v1 -m "Semantic Registry + Checker + pre-merge gate; Phase 2/3 governance PASS"
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

## 8. 当前验证状态（Phase 3 验收证据）

```text
SEMANTIC REGISTRY self-test:
  ALL_PASS  （R1..R6 positive/negative/false-positive 夹具全过；含 Phase3 bookmark 派生 computed + 第二列表 NEG）
SEMANTIC REGISTRY real scan:
  PASS  （fail=0；warn=6 pre-existing R5 提示级，非阻断；info=75）
R2 BOOKMARK GOVERN:
  PASS  （useBookmarkStore.ts 6 声明全登记；注入第二份 ref<Bookmark[]> 夹具被 R2 检出）
R6 DERIVED GUARD:
  PASS  （sorted 为 computed 且无 .value=；注入 sorted=ref([]) 夹具被 R6 检出）
NO REGRESSION:
  PASS  （R1..R6 逻辑未削弱；self-test 仍 ALL_PASS；未改任何业务代码）
KNOWN_DEBT isolated:
  PASS  （Debt-001~004 / 1.7-1~2 / 2-1~3 / 3-1~3 均显式记录，未被触碰或隐藏）
```
