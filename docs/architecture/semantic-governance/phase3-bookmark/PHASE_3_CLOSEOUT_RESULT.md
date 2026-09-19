# PHASE_3_CLOSEOUT_RESULT — Bookmark Semantic Governance

> Phase: 3
> Branch: feature/phase3-bookmark（ff-merge 入 master）
> Baseline: semantic-phase2-workspace-pass

---

## STATUS

```text
PASS
```

Bookmark 域（useBookmarkStore.ts）的语义已冻结并机器可强制：6 个状态纳入治理
（items 唯一真源 / sorted 派生）、5 个意图单一入口、owner 收敛为 useBookmarkStore、
与主页快捷方式（useHomeStore）显式隔离（rejected mergeBookmarksIntoHome）。

---

## ARCHITECTURE

```text
受治理状态（owner = useBookmarkStore）：
  items      收藏夹列表（唯一真源；后端 data_dir/bookmarks.json 镜像）
  loaded     是否已加载（避免重复 load）
  busy       异步操作进行中
  error      最近 IPC 失败（必须可见）
  panelOpen  收藏夹侧栏开关（刻意不进 layout）
  sorted     [DERIVED] = items 按 created_at 倒序（R6 护栏）

关键不变量：
  INV-3-1  items 唯一收藏列表真源；add 按 id/normalizeUrl splice 替换，禁止 push 重复
  INV-3-2  收藏身份 = normalizeUrl(url)；判定必须走 normalizeUrl
  INV-3-3  Bookmark（后端持久化）≠ 主页快捷方式（localStorage）；rejected 合并
```

---

## IMPLEMENTATION

```text
1. 扩展 Semantic Registry（docs/architecture/semantic-registry/*.yaml）：
   - states.yaml   : governed_files 增加 useBookmarkStore.ts；新增 6 状态（全治理，无 observed 负担）
   - intents.yaml  : 新增 5 个 bookmark intent + duplicate_names + rejected mergeBookmarksIntoHome
   - owners.yaml   : 新增 bookmark owner + violation_patterns（COMPONENT_WRITES_BOOKMARK）
   - side-effects.yaml : 文档级登记 bookmarkPersist（后端写盘，requires_declaration=false）

2. 复用 Semantic Gate（scripts/check-semantic-registry.mjs）：
   - R2 自动覆盖 useBookmarkStore.ts（6 声明全登记 → 任何第二份 Bookmark[] 未登记即 FAIL）
   - R4 自动覆盖 bookmark intent 重复入口
   - R6 自动覆盖 sorted（derived:true → 禁止 ref/.value=）
   - 新增 self-test 夹具：bookmark 派生 computed（FP）+ 治理域第二列表（R2 NEG）

3. 文档：Phase3-design.md + SCR-20260919-bookmark.md

未修改任何业务代码（src/ src-tauri/）。
```

---

## CHECKERS

```text
R2  [EXTENDED] useBookmarkStore.ts 纳入治理，6 声明全登记；第二份 Bookmark[] 未登记即 FAIL
R4  [EXTENDED] bookmark intent 重复入口定义 → FAIL
R6  [REUSE]     sorted 为 derived:true → 禁止存储/赋值
R3  [NOTE]      COMPONENT_WRITES_BOOKMARK 已登记 violation_patterns（R3 本身为 browser 专用，文档级约束）
R1/R5          未变动
```

---

## TESTS

```text
check-semantic-registry.mjs --self-test : SELF_TEST_RESULT=ALL_PASS
  ✓ positive fixture 0 fail/warn
  ✓ negative R1..R6 全部检出（含新增 bookmark R2 第二列表 / bookmark 派生 computed FP）
  ✓ false-positive fixture 0 fail/warn

真实仓库扫描：
  fail=0  warn=6（pre-existing R5，warn-level 非阻断）  info=75
  SEMANTIC_REGISTRY_RESULT=PASS
```

---

## RUNTIME

```text
GUI / Native：本 Phase 为治理/门禁层，无运行时 GUI 行为变更；无需人工 GUI 验收。
门禁运行时：pre-merge.sh 已接入，扩展后自动覆盖，无新增阻塞。
```

---

## KNOWN_DEBT

```text
DEBT-3-1  normalizeUrl 身份键未做 checker 强制
  状态: KNOWN DEBT  当前 Phase: 不处理（静态强制成本高、易误报，交专项）

DEBT-3-2  panelOpen 置于 bookmark store 而非 layout
  状态: KNOWN DEBT（有意设计）  当前 Phase: 不处理（登记为 CURRENT_FACT，不移动）

DEBT-3-3  bookmarkPersist 副作用仅文档化（requires_declaration=false）
  状态: KNOWN DEBT  当前 Phase: 不处理（同 Phase 2 writeFile 口径）

Debt-001~004 / Debt-1.7-1~2 / Debt-2-1~3 均显式继承，未触碰或隐藏。
```

---

## COMMITS

```text
feat(phase3): bookmark semantic governance
docs(phase3): closeout + handoff update
```

---

## TAG

```text
semantic-phase3-bookmark-pass  (annotated)
```

---

## NEXT_PHASE

```text
Phase 4 — Terminal Lifecycle Governance（tag: semantic-phase4-terminal-pass）
```
