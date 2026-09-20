# TRAIN D — Terminal Capability Isolation: AUDIT

> 阶段：Capability Modularization / Train D（Phase 8E）
> 日期：2026-09-20
> 基线：`b663ca7`（branch `feature/capability-platform-v1`，上游 tag `capability-phase8d-browser-composable-code-pass`）
> 目标：Terminal ≥ C3，`CURRENTLY_COMPOSABLE` 3 → 4。
> 硬约束：不 push / 不 sudo / 不动用户数据 / 不覆盖已装；终端的**冻结语义一律不变**。

---

## 1. 为什么要先审计（而不是直接搬目录）

`useSystemStore` 是 **Terminal + Clipboard + Apps 三个域共用的一个物理文件**，也是 Semantic Registry 中
`terminal` owner 的当前实现（Debt-7A-2 / deps `known_coupling` 已登记）。

直接 `git mv useSystemStore.ts → capabilities/terminal/state/` 会造成两个错误：

1. **Clipboard / Apps 被静默并入 Terminal 能力**（能力边界撒谎：Terminal absent 会把剪贴板一起带走）；
2. **owner 语义漂移**（Clipboard 的 owner 变成 `useTerminalStore`，Semantic Registry 失真）。

因此必须先做语义边界审计，再决定哪些 state 迁出、owner 是否迁移。

---

## 2. Terminal 相关资产全量盘点（审计对象）

### 2.1 语义 state（Semantic Registry `states.yaml` 已登记，owner=useSystemStore）

| # | state | 语义 | 类别 |
|---|-------|------|------|
| 1 | `terminalOpen` | 终端面板是否可见 | GOVERNED（Phase 4） |
| 2 | `termPanes` | 面板注册表唯一真源 `[{id,cwd}]` | GOVERNED（INV-4-1） |
| 3 | `termGrid` | 宫格模式 | GOVERNED |
| 4 | `termGridCount` | 宫格数（1/2/4/9） | GOVERNED |
| 5 | `activeTermId` | 当前聚焦面板（须 ∈ termPanes.id） | GOVERNED（INV-4-2） |
| 6 | `autoConfirmCli` | 自动确认 CLI True Color 提示（localStorage 键 `terminal-auto-confirm-cli`） | GOVERNED |
| 7 | `termProbeOn` | 终端对账 probe 开关（后端经 spawn 响应下发） | GOVERNED |
| 8 | `m0Cfg` | M0 吞吐测量配置（null=未启用） | GOVERNED |
| 9 | `m0StartTs` | M0 测量起始时间戳 | GOVERNED |
| 10 | `droppedChunks` | 背压丢弃 chunk 数（遥测） | GOVERNED |
| 11 | `droppedBytes` | 背压丢弃字节数（遥测） | GOVERNED |

### 2.2 非响应式会话运行时（per-pane，非 state，登记为 owner 内部实现细节）

`termWriters` / `termHistories` / `termBuffers`（Map<id, …>）、`confirmScans` / `lastAutoConfirmAt`（Map<id, …>）、
`probeRecv`（Map<id, …>）、`m0ThroughputStart`（函数引用）、`TERM_TEMP_HISTORY_LIMIT=40`（常量）。

红线（M3.c 冻结）：临时历史**只**在会话内存里；不落盘、不进审计、后端零留存。

### 2.3 Intent（`intents.yaml` 已登记，owner=useSystemStore）

`addTermPane` / `killTerm` / `termWrite` / `setActiveTerm` / `restartTerm` / `bindTermWriter` / `replayTermHistory`
（另有 owner-only：`spawnTerm` / `pushTermHistory`）。
Rejected：`componentWritesTermPanes`（组件直写 termPanes/activeTermId 即违规，checker `COMPONENT_WRITES_TERMINAL`）。

### 2.4 UI / 消费点

| 文件 | 角色 | Terminal 用法 |
|---|---|---|
| `src/components/system/TerminalPane.vue` | 面板（xterm + resize + probe + 历史回放） | 内部 |
| `src/components/layout/MainArea.vue` | **Shell** | 终端模块视图（工具栏/宫格/空态）＋ Dock 终端 Tab ＋ `watch(mainView==='term') → ensureTerm()` |
| `src/components/layout/StatusBar.vue` | **Shell** | `terminalOpen && termPanes.length` → "终端就绪/未启" |
| `src/App.vue` | **Shell** | `loadM0Config` / `m0Cfg` / `onTermData`（Event sink 订阅）/ 快捷键 → `layout.activateTerm()` |
| `src/components/layout/UnifiedTabBar.vue` | **Shell** | `m0Cfg?.driver` 测量模式短路 |
| `src/components/layout/ActivityBar.vue` | **Shell** | 同上 |
| `src/capabilities/workspace/state/useFileStore.ts` | **Workspace 能力** | `ctxOpenInTerminal` → `useSystemStore().openTerminalAt(dir)` |
| `src/composables/useTerminalResize.ts` | shared | resize 静默窗口（去重 + 140ms + 500ms 硬上界） |

### 2.5 Native / bridge / Rust

- 前端唯一入口：`bridge.createTermChannel` + `bridge.termSpawnChannel`（Channel 单播，M3.a F4）；另有
  `termWrite` / `termResize` / `termKill` / `onTermData`（Event sink，仅 M0 `term_spawn` 内部驱动）。
- Rust：`src-tauri/src/terminal.rs`（worker→mpsc(128)→pump→sink 三段式、DropCounter、聚合窗口、
  killpg 进程组回收、退避预算、零 tokio 依赖）+ `bridge.rs` 的 `term_*` + `main.rs` 注册 +
  `permissions/default-commands.toml` ACL。
- **本 Train 不改 `src-tauri/**` 一行**（无新增命令、无 ACL 变更、无依赖变更）。

### 2.6 side effect（`side-effects.yaml`）

`termProcess`（子进程生命周期）已文档化登记。用户数据写入：`terminal-auto-confirm-cli`（localStorage，UI 偏好）。

---

## 3. 边界裁决

### 3.1 Terminal 语义边界（IN）

PTY 生命周期（spawn/write/resize/kill）、面板注册表、宫格布局参数、聚焦 id、CLI 自动确认偏好、
对账 probe 开关、M0 吞吐测量配置与计时、背压丢弃遥测、会话内临时历史。

### 3.2 明确 OUT（不属 Terminal）

| 资产 | 真实归属 | 处置 |
|---|---|---|
| `clipText` / `clipHistory` / `CLIP_CAP` / `clipFocusBound` | Clipboard 域 | **留在 `useSystemStore`**，不迁入 terminal |
| `apps` / `appFilter` / `brokenIcons` / `filteredApps` | Apps 域 | **留在 `useSystemStore`** |
| `mainView==='term'` | View Navigation（owner `useLayoutStore`） | 不迁；Terminal 只消费 |
| `TerminalPane` 的容器几何 | Shell 布局 | 由 Terminal 自己的 UI 组件承接（不再由 Shell 渲染终端 DOM） |

### 3.3 owner 是否迁移 —— 裁决：**是，且必须走 SCR**

现状：`terminal` 的语义 owner 符号 = `useSystemStore`，但它同时是 Clipboard/Apps 的 owner。
要让「Terminal domain owner ≠ Clipboard owner」，唯一诚实的做法是**给 Terminal 一个独立 owner**：

```
新 owner 符号：useTerminalStore
物理实现：    src/capabilities/terminal/state/useTerminalStore.ts
```

- **禁止**把 Capability Runtime 当 owner（Runtime 只持编排元数据，RT-13 静态断言强制）。
- **禁止**建立第二份 Terminal state（`termPanes` 仍是唯一面板注册表真源，只是换了 owner 文件）。
- Clipboard/Apps 仍同处 `useSystemStore` → 记为显式债务 **Debt-8E-1**（不静默消失，见 §6）。

因此走：AUDIT（本文）→ SCR（`SCR-20260920-terminal-owner-extraction.md`）→ REGISTRY → CODE → CHECKER → TEST。

---

## 4. C3 达成路径（结构设计）

```
src/capabilities/terminal/
  manifest.ts    CapabilityDefinition（semanticOwner=useTerminalStore；status=COMPATIBILITY_WRAPPED；activatable=true）
  public.ts      纯再导出（useTerminalStore）——外部唯一入口，非第二真源
  index.ts       defineAsyncComponent + contributionRegistry.registerContribution(...) + <id>Capability（onActivate）
  state/
    useTerminalStore.ts    Terminal owner（物理迁出 useSystemStore）
  ui/
    TerminalPane.vue       单面板（迁入）
    TerminalView.vue       主视图（工具栏/宫格/空态；挂载即 ensureTerm）
    TerminalDockPanel.vue  Dock 终端（竖排）
    useTerminalResize.ts   resize 静默窗口（迁入）
```

Contribution（复用 8B.1 通用模型，零新契约）：
- `terminal.main.term` → slot `workbench-main`, `view="term"`
- `terminal.dock.term` → slot `browser-dock`, `view="term"`

**未创建 `adapters/`**：现有 bookmark/workspace/browser 三个能力均无 `adapters/` 目录，
凭空造一个会激活 CB-07（warn 级）噪声并使 Terminal 偏离既定形态；native 访问收敛在能力包内
（`state/` + `ui/`），Shell 完全不触碰，已满足 C3 的「Shell 不得触达 PTY 实现」。
（登记为后续可选加固项 **Debt-8E-2**。）

**Shell 侧解耦**：
- MainArea：删掉 `TerminalPane` / `useSystemStore` / 终端模块视图 / Dock 终端硬编码 / `ensureTerm` watch；
  终端的 PTY 创建权移交给 `TerminalView.vue` 的 `onMounted` —— 这是 **absent→无 PTY** 的关键机制。
- StatusBar/App/UnifiedTabBar/ActivityBar：经 `capabilities/terminal/public` 消费（与 Browser 同口径）。
- Workspace → Terminal 方向依赖：用 **窄缝** `src/composables/terminalNav.ts`（shared，与既有
  `browserNav` / `recentsNav` 同族），避免声明 `workspace dependsOn terminal`（那会让 Terminal 缺失时
  Workspace 无法装配，与「absent 可启动」目标矛盾）。

---

## 5. C3 ABSENT 判据（本 Train 必须证明，且是真证明）

Terminal **NOT REGISTERED**（contribution 未注册 / 未 activate）时：

| 判据 | 机制 |
|---|---|
| Shell 正常启动 | MainArea 该 view 的贡献为空 → 不渲染终端 DOM；无 import 终端内部 |
| 其它 capability 正常存在 | registry 空槽返回 `[]`；bookmark/workspace/browser 不受影响 |
| **不创建 PTY** | 唯一的出生点 `spawnTerm()` 只被 `TerminalView.vue`（能力贡献组件）与终端 owner 内部调用；Shell 无任何调用点 |
| **不创建 Terminal 子进程** | 同上（`term_spawn_channel` 无调用方） |
| **不触发 terminal backend initialization** | 无 `term_spawn_channel` / `term_kill` / `term_resize` invoke |

「只隐藏按钮」不算 C3 —— 本 Train 的判据是**资源不产生**，由动态测试（假 bridge 记录 invoke）证明。

## 6. 历史债务分类（§9 要求：必须区分三类，禁止归因漂移）

审计对象：`scripts/check-terminal-policy.py`（29 变异自检）与 `scripts/check-terminal-ui-logic.mjs`（25 断言）。

### 6.1 PRE_EXISTING CHECKER DEBT（Train D 开工前已存在，证据可复现）

| 项 | 症状 | 基线证据 | diff 证据 | 路径重叠证据 |
|---|---|---|---|---|
| A | `check-terminal-ui-logic.mjs` **崩溃**（`TypeError: system.startShell is not a function`），自 M3.c 后从未更新 | 开工前实跑即崩：`node scripts/check-terminal-ui-logic.mjs` → exit≠0 | 该文件 git 历史**只有 1 个提交** `0dd4cf4 (M3.c)` | `2a96cb1`（"文件树定位…"）把 store 从单终端 (`startShell`/`termId`/`termHistory`/`killShell`) 改成多面板 (`spawnTerm`/`termPanes`/`termHistories`/`killTerm`)，**未触碰 `scripts/check-terminal-ui-logic.mjs`**（`git show --stat 2a96cb1` 无 scripts/ 该文件） |
| B | `check-terminal-policy.py` FAIL：`TERM_HISTORY_CLEAR_MISSING`（断言 `store.count("clearTermHistory()") >= 3`） | 开工前实跑即 FAIL（含 `--self-test` 因「当前仓库自身存在违规」直接退出 1） | 同一提交 `2a96cb1` 把三条 `clearTermHistory()` 调用点改成 `termHistories.delete(id)` | 同上：`2a96cb1` 只改 `src/**`，未改 `scripts/check-terminal-policy.py` |

结论：**A / B 均为 PRE_EXISTING CHECKER DEBT**（检测器相对 per-pane 现实失配），
不是 REAL ARCHITECTURE VIOLATION（`termPanes` 单一真源、历史不落盘等不变量在 `2a96cb1` 后仍成立），
也不是 Train D 引入的 NEW REGRESSION。

### 6.2 处置（不许 skip / 删 checker / 放宽规则 / 大 allow-list）

1. **修检测器，不修不变量**：
   - A：把 `check-terminal-ui-logic.mjs` 重新对接到 **per-pane 真实 API**（`spawnTerm` / `bindTermWriter(id,fn)` /
     `onTermChannelMsg` / `replayTermHistory(id)` / `killTerm(id)`），并**新增**「killTerm 后同 pane 回放为空」等断言。
   - B：把 `TERM_HISTORY_CLEAR_MISSING` 的判定从「文本计数 ≥3」升级为「会话结束确实清空」——
     接受 `clearTermHistory(` 或 `termHistories.delete(`，并要求 owner 文件在 kill 路径上二者至少出现其一；
     同时**在新增的 `check-terminal-owners.mjs` 里给出行为级证明**（真实 store + killTerm → 回放为空）。
     这是**检测手段的重基线化**（静态文本计数 → 静态+行为），不变量本身未被削弱。
2. **路径同步**（防「新回归」）：本 Train 把终端物理迁入 `src/capabilities/terminal/`，若不同步
   `check-terminal-policy.py` / `check-semantic-registry.mjs` 夹具 / `check-clipboard-persistence-logic.mjs`
   的读取路径，会产生**由本迁移引起的新 FAIL** —— 一律同步，并逐个复跑证明。
3. **不隐藏 warn/info**：报告如实列出各 checker 的 fail/warn/info 数，不新增 allow-list 条目。

### 6.3 本次新增债务（显式登记，不静默）

| id | 内容 | 归口 |
|---|---|---|
| Debt-8E-1 | Clipboard 与 Apps 仍共处 `useSystemStore`（二者 owner 同为 `useSystemStore`）。Terminal 已与二者解耦；Clipboard/Apps 拆分不属 Train D 范围 | 后续 Train（未派发） |
| Debt-8E-2 | Terminal 未建 `adapters/` 层（native 访问在能力包内 state/ui 直接触达 bridge）。CB-07 因此未对 Terminal 武装 | 后续加固 |
| Debt-8E-3 | `terminal-auto-confirm-cli` 仍写 localStorage（UI 偏好，非敏感；B11-1 只针对剪贴板/凭据） | 既有事实，维持 |
| Debt-8E-4 | `check-terminal-policy.py` 的 `TERM_HISTORY_CLEAR_MISSING` 由文本计数改为「两者其一 + 行为证明」 | 本 Train 已重基线化；行为证明落在 `check-terminal-owners.mjs` |

---

## 7. 本 Train 的验收清单（详见 §TRAIN-D-CLOSEOUT）

TERM-01 Terminal state 单 owner｜TERM-02 无第二 truth｜TERM-03 Shell 不 import Terminal internals｜
TERM-04 absent → Shell boots｜TERM-05 absent → no PTY｜TERM-06 absent → no child process｜
TERM-07 present → PTY/process 正常｜TERM-08 destroy → cleanup｜TERM-09 Semantic Registry PASS｜
TERM-10 Writer Enforcement PASS｜TERM-11 Capability Boundary PASS｜TERM-12 Build PASS｜TERM-13 Independent Review PASS。
