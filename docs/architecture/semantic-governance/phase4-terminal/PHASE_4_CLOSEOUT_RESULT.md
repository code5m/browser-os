# PHASE_4_CLOSEOUT_RESULT — Terminal Lifecycle Semantic Governance

> Phase: 4
> Branch: feature/phase4-terminal（ff-merge 入 master）
> Baseline: semantic-phase3-bookmark-pass

---

## STATUS

```text
PASS
```

Terminal 生命周期语义（useSystemStore.ts 的 termPanes/activeTermId/terminalOpen 等）已冻结并机器可强制：
11 个终端状态纳入治理（termPanes 唯一面板注册表）、7 个意图单一入口、owner 收敛为 useSystemStore、
与视图导航 setView("term") 显式隔离（activateTerm 不作 setActiveTerm 重复名）。

---

## ARCHITECTURE

```text
受治理状态（owner = useSystemStore）：
  terminalOpen  面板可见
  termPanes     [面板注册表唯一真源] [{id,cwd}]；spawn→push，kill→filter
  termGrid / termGridCount  宫格模式/数
  activeTermId  聚焦面板 id（必须 ∈ termPanes.id；kill 后重置为首剩余）
  autoConfirmCli / termProbeOn / m0Cfg / m0StartTs / droppedChunks / droppedBytes

关键不变量：
  INV-4-1  termPanes 唯一面板注册表；addTermPane/killTerm 是唯一增删入口
  INV-4-2  activeTermId 必须始终 ∈ termPanes.id
  INV-4-3  打开 terminalOpen 且 termPanes 空 → 自动 spawnTerm
```

---

## IMPLEMENTATION

```text
1. 扩展 Semantic Registry（docs/architecture/semantic-registry/*.yaml）：
   - states.yaml   : governed_files += src/stores/useSystemStore.ts；
                     11 terminal states 治理 + 5 observed_not_governed（clipboard/apps 子域）
   - intents.yaml  : 7 terminal intents（addTermPane/killTerm/termWrite/setActiveTerm/restartTerm/bindTermWriter/replayTermHistory）
                     + duplicate_names；rejected componentWritesTermPanes
   - owners.yaml   : 新增 terminal owner = useSystemStore；violation_patterns COMPONENT_WRITES_TERMINAL
   - side-effects.yaml : 文档级登记 termProcess（PTY spawn/kill，requires_declaration=false）

2. 复用 Semantic Gate（scripts/check-semantic-registry.mjs）：
   - R2 自动覆盖 useSystemStore.ts（16 声明全登记；第二份面板列表未登记即 FAIL）
   - R4 自动覆盖 terminal intent 重复入口
   - 新增 self-test 夹具：terminal 第二面板列表（R2 NEG）+ newTerm 重复定义（R4 NEG）

3. 文档：Phase4-design.md + SCR-20260919-terminal.md

未修改任何业务代码（src/ src-tauri/）。
```

---

## CHECKERS

```text
R2  [EXTENDED] useSystemStore.ts 纳入治理，16 声明全登记；第二份面板列表未登记即 FAIL
R4  [EXTENDED] terminal intent 重复入口定义 → FAIL
R3  [NOTE]     COMPONENT_WRITES_TERMINAL 已登记 violation_patterns（R3 本身为 browser 专用，文档级约束）
R6  [REUSE]    无 terminal 派生状态；既有 derived 不受影响
R1/R5          未变动

既有终端门禁（不属本 Phase 范围，仅记录）：
  check-terminal-policy.py (29 断言) / check-terminal-ui-logic.mjs (25 断言) 已接入 pre-merge
  Debt-004 其自检 FAIL 为既有债，本 Phase 不修、不隐藏
```

---

## TESTS

```text
check-semantic-registry.mjs --self-test : SELF_TEST_RESULT=ALL_PASS
  ✓ positive fixture 0 fail/warn
  ✓ negative R1..R6 全部检出（含新增 terminal R2 第二面板列表 / terminal R4 newTerm）
  ✓ false-positive fixture 0 fail/warn

真实仓库扫描：
  fail=0  warn=6（pre-existing R5，warn-level 非阻断）  info=80
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
DEBT-4-1  termPanes 第二注册表静态护栏限于 R2（新增未登记 ref 才报）；spawn/kill 资源策略
          由 check-terminal-policy.py 覆盖，未并入 Semantic Registry（两套门禁并存）
DEBT-4-2  R3 不扫描组件对 termPanes/activeTermId 的直写（R3 为 browser 专用）
DEBT-4-3  termProcess 副作用仅文档化（requires_declaration=false）
DEBT-004  terminal 检查器自检 FAIL（既有债，Phase 外，不修不隐藏）
Debt-001~004 / Debt-1.7-1~2 / Debt-2-1~3 / Debt-3-1~3 均显式继承，未触碰或隐藏。
```

---

## COMMITS

```text
feat(phase4): terminal lifecycle semantic governance
docs(phase4): closeout + handoff update
```

---

## TAG

```text
semantic-phase4-terminal-pass  (annotated)
```

---

## NEXT_PHASE

```text
Phase 5 — Credential Security Governance（tag: semantic-phase5-credential-pass）
```
