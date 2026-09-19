# Phase 4 — Terminal Lifecycle Semantic Governance

> 状态：DESIGN（实现见 `docs/architecture/semantic-registry/*.yaml` 扩展 + 复用 `scripts/check-semantic-registry.mjs` R2/R4）
> 基线：继承 `semantic-phase3-bookmark-pass` + Semantic Registry + Semantic Gate + Recovery Layer + Handoff System
> 触发：终端生命周期状态散落在 useSystemStore.ts（TerminalPane.vue 仅渲染），存在第二面板注册表 /
>       越权直写 activeTermId / 重复入口风险。既有 check-terminal-policy.py(29) / check-terminal-ui-logic.mjs(25)
>       已覆盖 Rust 端策略与 UI 逻辑（Debt-004 为其自检 FAIL，本 Phase 不修，仅冻结语义契约）。

---

## 1. State Model（受治理状态）

```text
terminalOpen   [STORED]
  = 终端面板是否可见（toggle；打开且 0 面板时自动 spawnTerm）
termPanes      [STORED, pane registry / single source]
  = 终端面板注册表 [{id, cwd}]；spawn→push，kill→filter（唯一真源）
termGrid       [STORED] 宫格模式（多终端并排）
termGridCount  [STORED] 宫格数（默认 4 = 2×2）
activeTermId   [STORED] 当前聚焦面板 id（kill 后置为首剩余，禁止悬空）
autoConfirmCli [STORED] 自动确认 CLI True Color 提示（来自 localStorage）
termProbeOn    [STORED] 终端对账 probe 开关
m0Cfg         [STORED] M0 吞吐测量配置（null=未启用）
m0StartTs     [STORED] M0 测量起始时间戳
droppedChunks [STORED] 过快输出丢弃的 chunk 数（遥测）
droppedBytes  [STORED] 过快输出丢弃的字节数（遥测）
```

### 关键不变量

```text
INV-4-1  termPanes 是唯一面板注册表；任何面板增删须经 addTermPane/killTerm，禁止第二份列表。
INV-4-2  activeTermId 必须始终 ∈ termPanes 的 id 集合；killTerm 同步重置为 termPanes[0]?.id。
INV-4-3  打开 terminalOpen 且 termPanes 空 → 自动 spawnTerm（不遗留空白面板）。
```

### observed_not_governed（本文件其余声明）

useSystemStore.ts 的 clipboard/apps 子域（clipText/clipHistory/apps/appFilter/filteredApps）
本 Phase 不治理终端生命周期，显式登记（info 级）。

---

## 2. Intent Model（意图）

```text
intent: addTermPane
  语义：新建终端面板（spawn）
  owner：useSystemStore.addTermPane（内部调 spawnTerm）
  duplicate_names：spawnTermPane / newTerm / openTerm

intent: killTerm
  语义：按 id 关闭面板（filter + 重置 activeTermId）
  owner：useSystemStore.killTerm
  duplicate_names：closeTerm / removeTerm / terminateTerm

intent: termWrite
  语义：向 PTY 写入（用户输入 / 自动确认）
  owner：useSystemStore.termWrite（→ bridge.termWrite）
  duplicate_names：sendTerm / writeTerm

intent: setActiveTerm
  语义：聚焦某面板（activeTermId = id）
  owner：useSystemStore.setActiveTerm
  duplicate_names：focusTerm

intent: restartTerm
  语义：重启面板（killTerm + addTermPane，TerminalPane.restart）
  owner：useSystemStore（restart = killTerm+addTermPane）
  duplicate_names：restartTerminal

intent: bindTermWriter
  语义：注册面板输出 sink（PTY→xterm）
  owner：useSystemStore.bindTermWriter
  duplicate_names：setTermWriter

intent: replayTermHistory
  语义：面板重建时回放会话内历史（≤40 条，不落盘）
  owner：useSystemStore.replayTermHistory
  duplicate_names：replayTerm
```

### rejected（明确不做）

```text
rejected: 组件直写 termPanes / activeTermId（第二注册表 / 悬空 activeTermId）
  原因：破坏 INV-4-1/4-2；面板增删只经 store action
rejected: activateTerm 作为 setActiveTerm 重复名
  原因：activateTerm 已在 useLayoutStore 定义为导航（setView("term")），属视图切换非面板聚焦；
        二者语义不同，禁止合并为同一入口（setActiveTerm 仅用 focusTerm 作重复名）
```

---

## 3. Owner Model（Owner）

```text
terminal
  owner = useSystemStore
  owns  = terminalOpen / termPanes / termGrid / termGridCount / activeTermId /
          autoConfirmCli / termProbeOn / m0Cfg / m0StartTs / droppedChunks / droppedBytes
  owner_only_api = addTermPane / killTerm / termWrite / setActiveTerm / restartTerm /
                   bindTermWriter / replayTermHistory / spawnTerm / pushTermHistory
  forbidden_callers = components/**（TerminalPane.vue 只能调 store action，不得直写 termPanes/activeTermId）
  authorized_callers = useSystemStore（自身）/ TerminalPane.vue（消费）/ useLayoutStore（导航）
```

---

## 4. Lifecycle（生命周期）

```text
打开终端
  └─ toggleTerminal()  → terminalOpen=!terminalOpen；若为真且 termPanes 空 → spawnTerm()
spawn 面板
  └─ addTermPane() → spawnTerm() → termPanes.push({id,cwd})；activeTermId=id
关闭面板
  └─ killTerm(id)     → bridge.closeTerm(id)；termPanes=filter(!=id)；
                         activeTermId = termPanes[0]?.id ?? ""（INV-4-2）
聚焦
  └─ setActiveTerm(id) → activeTermId = id（必须 ∈ termPanes）
重启
  └─ restartTerm()    → killTerm(id) + addTermPane()
```

---

## 5. Side Effect（副作用）

```text
termProcess（bridge.spawnTerm / bridge.closeTerm / bridge.termWrite）
  - 真实行为：创建/销毁 OS PTY 进程（资源）；向 PTY 写输入
  - 声明：registry 文档化（requires_declaration=false）
  - 既有 check-terminal-policy.py 已覆盖 Rust 端 spawn/kill 资源策略（Debt-004 自检 FAIL 不属本 Phase）
```

---

## 6. Checker Plan（门禁设计）

### 6.1 复用 `scripts/check-semantic-registry.mjs`（registry 驱动）

- **R2（扩展）**：将 `src/stores/useSystemStore.ts` 加入 `states.yaml` 的 `governed_files`。
  其内 16 个声明（11 terminal 治理 + 5 clipboard/apps observed）全部登记 →
  任何新 `ref/reactive/computed` 声明若未登记 → FAIL（强制 termPanes 单一注册表）。
- **R4（扩展）**：7 个 terminal intent 的 `duplicate_names` 在任意文件被定义即 FAIL（一个意图一个入口）。
- **R3（文档）**：`terminal` owner 的 `forbidden_callers`（components/** 直写 termPanes/activeTermId）
  登记为 violation_patterns（COMPONENT_WRITES_TERMINAL）；R3 本身为 browser 专用，属文档级约束。
- **R6（复用）**：当前无 terminal 派生状态；既有 derived 状态不受影响。

### 6.2 接入 pre-merge

`check-semantic-registry.mjs` 自 Phase 1.5 已接入 pre-merge；本 Phase 扩展 YAML 后自动覆盖。

---

## 7. Acceptance Matrix（验收矩阵）

```text
AC-1  R2：useSystemStore.ts 内 16 声明全登记（11 terminal + 5 observed）→ 真实仓库 fail=0   [REAL]
AC-2  R4：terminal intent 的 duplicate_names 在真实仓库无定义 → fail=0                     [REAL]
AC-3  R2：在 useSystemStore.ts 注入第二份 ref<{id,cwd}[]>（未登记）→ 检出 FAIL               [NEG FIXTURE]
AC-4  R4：注入 fixture（function newTerm(){...}）→ 检出 SEMANTIC_INTENT_DUPLICATE           [NEG FIXTURE]
AC-5  check-semantic-registry.mjs --self-test 全量 ALL_PASS（含 terminal 域）                 [SELF-TEST]
AC-6  pre-merge 中 semantic-registry 项仍绿（不引入新 FAIL；terminal 债 Debt-004 为既有）     [GATE]
AC-7  不降低任何既有 Checker；未改业务代码                                            [NO REGRESSION]
AC-8  HANDOFF_CURRENT_STATE.md 更新 Phase 4 状态 + tag 分类                               [HANDOFF]
```

---

## 8. 与既基础设施的关系

```text
check-semantic-registry.mjs   复用并扩展（R2/R4 域扩展），不重造
check-terminal-policy.py / check-terminal-ui-logic.mjs   既有终端生命周期/UI 门禁（Rust 端 + UI 逻辑），
                                                         Deｂt-004 其自检 FAIL 为既有债，本 Phase 不修、不隐藏
states/intents/owners/side-effects.yaml   扩展 terminal 域，不重造
pre-merge.sh               已接入（registry checker + 两 terminal checker），无需改接线
```

---

## 9. Known Debt（本 Phase 内）

```text
DEBT-4-1  termPanes 第二注册表静态护栏限于 R2（新增未登记 ref 才报）；既有 sp‌awn/kill 资源策略
          由 check-terminal-policy.py 覆盖，未并入 Semantic Registry（两套门禁并存）
  当前 Phase: 不处理（交专项收敛）

DEBT-4-2  R3 不扫描组件对 termPanes/activeTermId 的直写（R3 为 browser 专用）
  当前 Phase: 登记 violation_patterns 为文档约束；强化需扩展 R3 通用化（业务/checker 改动）

DEBT-004   terminal 检查器自检 FAIL（check-terminal-policy.py / check-terminal-ui-logic.mjs）
  状态: KNOWN DEBT（既有，Phase 外）  当前 Phase: 不修、不隐藏、不视为本 Phase 引入

DEBT-4-3  termProcess 副作用仅文档化（requires_declaration=false）
  当前 Phase: 不处理（同 Phase 2/3 口径）
```
