# PHASE_2_CLOSEOUT_RESULT — Workspace / FilePanel Semantic Governance

> Phase: 2
> Branch: feature/phase2-workspace（ff-merge 入 master）
> Baseline: semantic-phase1.7-git-integrity-pass

---

## STATUS

```text
PASS
```

Workspace/FilePanel 域（useWorkspaceStore.ts + FilePanel.vue）的语义已冻结并机器可强制：
5 个文件导航状态纳入治理、37 个既有声明显式登记为 observed_not_governed、5 个意图单一入口、
owner 收敛为 useWorkspaceStore、派生位置 currentLocalPath 经 R6 固化为 derived（禁止存储/赋值）。

---

## ARCHITECTURE

```text
受治理状态（owner = useWorkspaceStore）：
  filePath        当前激活位置（地址真源；overloaded：编辑器模式=文件 / 浏览器模式=目录）
  inlineFile     行内编辑器当前文件（与覆盖编辑器隔离）
  previewDir     目录预览当前目录
  pathInput      地址栏 UI 缓冲（镜像 filePath，非真源）
  currentLocalPath [DERIVED] = inlineFile || previewDir || modTabs.path || filePath

Owner 收敛：
  workspace_filepanel = useWorkspaceStore
  forbidden_callers: components/**（不得直写 filePath/inlineFile/previewDir/pathInput）

Intent 单一入口：
  openFile / openFileInline / enterDir / saveFile / closeFileEditor
  rejected 重复合并：mergeOpenFileIntoOpenFileInline
```

---

## IMPLEMENTATION

```text
1. 扩展 Semantic Registry（docs/architecture/semantic-registry/*.yaml）：
   - states.yaml   : governed_files 增加 useWorkspaceStore.ts + FilePanel.vue；
                     新增 5 状态；observed_not_governed 登记 39 个既有声明（含 fileCtx/ctxMenu）
   - intents.yaml  : 新增 5 个 workspace intent + duplicate_names + rejected_intents
   - owners.yaml   : 新增 workspace_filepanel owner + violation_patterns（COMPONENT_WRITES_WORKSPACE_PATH）
   - side-effects.yaml : 文档级登记 writeFile（文件系统变更，requires_declaration=false）

2. 扩展 Semantic Gate（scripts/check-semantic-registry.mjs）：
   - 新增 R6 SEMANTIC_DERIVED_STATE_STORED：任意 derived:true 状态禁止
     (a) 声明为 ref/reactive/shallowRef 存储态 (b) .value = 赋值
   - R2/R3/R4 随 governed_files + 新域自动覆盖 workspace
   - 新增 R6 / workspace 派生态 self-test 夹具（positive + negative + false-positive）

3. 文档：Phase2-design.md + SCR-20260919-workspace-filepanel.md + registry README 范围说明

未修改任何业务代码（src/ src-tauri/）。
```

---

## CHECKERS

```text
R6 SEMANTIC_DERIVED_STATE_STORED  [NEW] 派生状态禁止存储/赋值 —— 固化 currentLocalPath 派生不变量
R2 [EXTENDED]                     useWorkspaceStore.ts + FilePanel.vue 纳入治理，未登记声明即 FAIL
R3 [EXTENDED]                     components/** 直写 workspace 路径 → FAIL
R4 [EXTENDED]                     workspace intent 重复入口定义 → FAIL
R1/R5                             未变动（phase 1 既有）
```

---

## TESTS

```text
check-semantic-registry.mjs --self-test : SELF_TEST_RESULT=ALL_PASS
  ✓ positive fixture 0 fail/warn
  ✓ negative R1..R6 全部检出（含 R6 派生状态被存为 ref + .value=）
  ✓ false-positive fixture 0 fail/warn（派生字段名/注释/治理域外/授权调用不误报）

真实仓库扫描：
  fail=0  warn=6（pre-existing R5，warn-level 非阻断）  info=76
  SEMANTIC_REGISTRY_RESULT=PASS
```

---

## RUNTIME

```text
GUI / Native：本 Phase 为治理/门禁层，无运行时 GUI 行为变更；无需人工 GUI 验收。
门禁运行时：pre-merge.sh 已接入（自 Phase 1.5），本 Phase 扩展后自动覆盖，无新增阻塞。
```

---

## KNOWN_DEBT

```text
DEBT-2-1  filePath overloaded 语义（编辑器=文件 / 浏览器=目录）未拆分
  状态: KNOWN DEBT  当前 Phase: 不处理（需业务重构，超出治理范围）

DEBT-2-2  R2 正则不识别 reactive<Set/Map>(new ...) 嵌套泛型
  状态: KNOWN DEBT  selected/treeChildren/treeExpanded/treeLoading/treeErrors 不受 R2 治理
  当前 Phase: 不处理（checker 盲区，交专项）

DEBT-2-3  writeFile 副作用仅文档化（requires_declaration=false）
  状态: KNOWN DEBT  当前 Phase: 不处理（收紧需改 side-effects.yaml + 业务代码加标记）

DEBT-001~004 / Debt-1.7-1~2 均显式继承，未触碰或隐藏。
```

---

## COMMITS

```text
feat(phase2): workspace/filepanel semantic governance
docs(phase2): closeout + handout update
```

---

## TAG

```text
semantic-phase2-workspace-pass  (annotated)
```

---

## NEXT_PHASE

```text
Phase 3 — Bookmark Semantic Governance（tag: semantic-phase3-bookmark-pass）
```
