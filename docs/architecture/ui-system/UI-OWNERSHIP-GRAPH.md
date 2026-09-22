# UI OWNERSHIP GRAPH — Phase UI-0D

> 回答每个 UI 的六问：**谁拥有 / 谁控显隐 / 谁控 state / 谁控 lifecycle / 谁控 resource / 谁提供 contribution / 谁真正 render**

---

## 1. 渲染链路总图

```
main.ts
  └─ App.vue ................................. 外壳骨架 + 全局事件宿主
       ├─ UnifiedTabBar  (:258, !compactMode) .. 页签条 + 自绘标题栏
       ├─ ActivityBar    (:259, !compactMode) .. 导航 + 地址栏 + 扩展行
       │     └─ [address-bar-actions]  ← bookmark.star
       │     └─ [activity-bar-trailing] ← bookmark.entry-button
       │     └─ GridArchiveBar (:373)  ⚠️ Shell 直渲业务
       ├─ WorkbenchCommands (:260) ............ Ctrl+K 命令面板
       ├─ WorkbenchRail    (:262, !compactMode)
       ├─ AINavPanel       (:263)  ⚠️ Shell 直渲业务（browser 域）
       ├─ MainArea         (:264) ............. 唯一 surface 宿主
       │     ├─ [workbench-main-resident] ← terminal.view   (:133)
       │     ├─ mainView='browser'|'grid'
       │     │    ├─ [browser-sidebar]  ← bookmark.panel    (:152)
       │     │    ├─ [browser-host]     ← browser.host      (:159)
       │     │    └─ .browser-dock (:162-178)
       │     │         ├─ dock 'files'   ← workspace.file-panel (:170)
       │     │         ├─ dock 'net'     ← browser.resource-waterfall (:172)
       │     │         ├─ dock 'session' ← browser.session-panel (:174)
       │     │         └─ dock 'term'    ← terminal.dock-panel (:177)
       │     ├─ [workbench-main] 动态 (:183-185) ← workspace 7 个 main 面板
       │     ├─ clip → ClipboardPanel (:189) ⚠️ Shell 直渲
       │     ├─ apps → AppPanel       (:194)
       │     ├─ tools → ToolBox       (:199) ⚠️ 业务
       │     ├─ db    → DatabasePanel (:204) ⚠️ 业务
       │     ├─ vault → VaultPanel    (:207) ⚠️ 业务
       │     ├─ tasks → TaskPanel     (:210) ⚠️ 业务
       │     ├─ plugin→ PluginManager (:215) ⚠️ 业务
       │     ├─ skills→ SkillManager  (:218) ⚠️ 业务
       │     ├─ agents→ AgentManager  (:221) ⚠️ 业务
       │     ├─ graph → GraphPanel    (:224) ⚠️ 业务
       │     └─ settings → SettingsPanel (:229)
       ├─ StatusBar       (:266)
       ├─ ConfirmModal    (:267) ⚠️ 实为 Git 域
       ├─ GitWriteConfirmDialog (:269) ⚠️ Git 域，App 全局挂载
       └─ ImageLightbox   (:273)
```

---

## 2. Contribution 系统审计（§11）— 真实状况

### 2.1 事实：只有 **2 种** contribution 类型，**7 个槽**

```
src/capability/contribution/types.ts:15
  type ContributionType = "surface" | "navigation"
```

| 槽常量 | 字符串 | 定义 | 注册数 | 宿主渲染点 |
|---|---|---:|---|---|
| `BROWSER_SIDEBAR` | `browser-sidebar` | types.ts:43 | 1 | `MainArea.vue:152` ✅ |
| `ADDRESS_BAR_ACTIONS` | `address-bar-actions` | types.ts:44 | 1 | `ActivityBar.vue:321` ✅ |
| `ACTIVITY_BAR_TRAILING` | `activity-bar-trailing` | types.ts:45 | 1 | `ActivityBar.vue:330` ✅ |
| `WORKBENCH_MAIN` | `workbench-main` | types.ts:46 | 7 | `MainArea.vue:183-185` ✅ |
| `WORKBENCH_MAIN_RESIDENT` | `workbench-main-resident` | types.ts:47 | 1 | `MainArea.vue:133` ✅ |
| `BROWSER_HOST` | `browser-host` | types.ts:48 | 1 | `MainArea.vue:159` ✅ |
| `BROWSER_DOCK` | `browser-dock` | types.ts:49 | 4 | `MainArea.vue:170-177` ✅ |

**槽全部有注册 + 有宿主，无空槽。**

### 2.2 运行时注册（16 条，与 manifest 声明 16 条一致）

**surface（14）**：`bookmark.sidebar`、`browser.host`、`browser.dock.net`、`browser.dock.session`、`terminal.main.term`、`terminal.dock.term`、`workspace.main.{files,arts,repo,scripts,commands,audit,editor}`、`workspace.dock.files`
**navigation（2）**：`bookmark.address-star`、`bookmark.entry-button`

### 2.3 GAP（按要求记录，不立即造 API）

| GAP | 说明 | 是否满足 RULE OF TWO |
|---|---|---|
| **无 Command 贡献类型** | 菜单/命令面板硬编码于 `ActivityBar` / `WorkbenchCommands`，能力无法扩展 | ❌ 目前仅需求来自未注册域 → **不抽象** |
| **无 Menu 贡献类型** | `NAV_MENU_SECTIONS` 硬编码于 `useLayoutStore.ts:43-75` | ❌ 同上 |
| **无 StatusBar 贡献类型** | `StatusBar` 直接读 6 个 store | ❌ 同上 |
| **无 Toolbar / Panel / Dock 第一类类型** | Dock 是 `MainArea.vue:162-178` 内联模板，4 Tab 硬编码 | ⚠️ **此条已有真实需求**（见下） |
| `Contribution.order` 字段 | 存在（`types.ts:70`）但 16 条注册**无一使用**，排序恒等 | — |
| `registry.getBySlot()` | 仅 orchestrator 用，UI 宿主不用 | — |
| `Contribution.component` 可选 | 所有宿主无条件 `<component :is>`，无 component 会渲染警告 | — |
| `unregisterContribution` | 只有 orchestrator 调用，UI 无热卸载 | — |

### 2.4 Dock 的真实状态 —— **存在但非组件化**

- 槽存在：`CONTRIBUTION_SLOTS.BROWSER_DOCK`
- 宿主：`MainArea.vue:162-178` **内联模板**，4 个硬编码 Tab（`:164-167`）
- 状态真源：`useLayoutStore.ts:150-151` `browserDockOpen` / `browserDockTab: "files"|"term"|"net"|"session"`
- **关键缺陷**：
  1. 能力的 `browser-dock` 贡献若 `view` 不在 4 个白名内 → **永远不渲染**（if/else-if 链）
  2. Terminal 能力 absent 时「💻 终端」按钮**仍显示**，点击渲染空 → 死 Tab
  3. 无 `DockHost.vue` 独立组件

> **这是唯一已有真实消费者（4 个能力/子面板）却未抽象的扩展点。** 但按「NO BIG BANG」，Dock 组件化列入后续批次，本阶段仅登记。

### 2.5 RULE OF TWO 判定

| 候选抽象 | 真实消费者 | 结论 |
|---|---|---|
| Command 贡献 | 0（全部硬编码） | ❌ 不抽象 |
| Menu 贡献 | 0（全部硬编码） | ❌ 不抽象 |
| StatusBar 贡献 | 0（直接读 store） | ❌ 不抽象 |
| DockHost 组件 | **4**（files/term/net/session） | ✅ 有需求，但属重构，列入后续批次 |
| EmptyState | **20+** | ✅ **Pilot 首选** |
| LoadingState | **15** | ✅ **Pilot 首选** |
| ModalShell | **7** | ✅ **Pilot 首选** |

---

## 3. 所有权分类（§9 六问矩阵）

### 3.1 Shell 组件的所有权

| 组件 | 拥有 | 控显隐 | 控 state | 控 lifecycle | 控 resource | 提供 contribution | 真正 render |
|---|---|---|---|---|---|---|---|
| `App.vue` | Shell | Shell | Shell + **6 个能力 store** | 全局事件宿主 | ❌（已闸） | 宿主 | main.ts |
| `ActivityBar` | Shell | Shell | Shell + browser/term/ws | — | ❌ | **宿主**（2 navigation 槽） | App.vue |
| `MainArea` | Shell | Shell | Shell + browser | — | ❌ | **宿主**（5 surface 槽） | App.vue |
| `UnifiedTabBar` | Shell | Shell | Shell + browser/fs/term | 窗口控制 | ❌ | — | App.vue |
| `StatusBar` | Shell | Shell | Shell + browser/term/ws/rp | — | ❌ | — | App.vue |
| `Home*` | Shell | Shell | `useHomeStore` | — | ❌ | — | HomePanel |
| `SettingsPanel` | Shell | Shell | `useSettingsStore` | — | ❌ | — | MainArea |

### 3.2 能力组件的所有权（正确范式）

| 组件 | 拥有 | 控显隐 | 控 state | 控 lifecycle | 控 resource | contribution | render |
|---|---|---|---|---|---|---|---|
| `browser.host` | browser | Shell(v-show) | browser store | browser | **BrowserHost 常驻**（visibility 非 display:none） | `browser-host` | MainArea:159 |
| `terminal.view` | terminal | Shell(v-show) | terminal store | terminal | **PTY 出生点**（已闸） | `workbench-main-resident` | MainArea:133 |
| `terminal.pane` | terminal | terminal | terminal store | terminal | **xterm + PTY**（`bridge` 直连） | — | TerminalView/DockPanel |
| `workspace.file-panel` | workspace | Shell(dock tab) | workspace store | workspace | ❌ | `browser-dock` | MainArea:170 |
| `bookmark.panel` | bookmark | **自管**（`panelOpen && mainView==='browser'`） | bookmark store | bookmark | ❌ | `browser-sidebar` | MainArea:152 |

### 3.3 耦合分类

| 分类 | 数量 | 明细 |
|---|---:|---|
| **PUBLIC_CONTRACT** | 3 | `BookmarkStar→browser/public`（已声明 optionalDeps）、`BookmarkPanel→browser/public`（已声明）、`MainArea/ActivityBar→capabilities/*/public`（合规） |
| **GENERIC_CONTRIBUTION** | 16 | 全部 16 条运行时注册（4 能力 × 7 槽） |
| **LEGACY_COUPLING** | 26 组件 | 见 `UI-INVENTORY.md §5` |
| **BOUNDARY_VIOLATION** | **1** | `workspace/ui/FileEditor.vue:4` 依赖 browser 但 manifest 未声明 |
| **RESOURCE_CREATION_VIOLATION** | **0** | ✅ Grid 已闸（`buildGrid`）、PTY 已闸（`spawnTerm`）；framework profile 实测 grid-child=0 / ptmx=0 |

---

## 4. Shell → Capability internal 专项扫描（§9 重点）

| 检查项 | 结果 |
|---|---|
| Shell import 能力**内部**（`state/` `ui/` 私有路径） | **0** ✅（全部走 `public` 出口或贡献槽） |
| Shell import 业务 **UI 组件** | **11**（见 INVENTORY §5.1）⚠️ |
| Shell 持有能力 **store** | **7**（`App.vue:5,8,9,10,11,12,14`）⚠️ |
| Shell 直接调 **bridge** 业务命令 | `ActivityBar`(resourceStats/debugLog)、`StatusBar`(resourceStats)、`SettingsPanel`(getDefaultBrowser/setDefaultBrowser)、`UnifiedTabBar`(**未 import 的 bridge** ⚠️) |
| Shell 触发**资源创建** | **0** ✅ |

> 结论：**资源边界已干净**（H-G 修复生效），**UI 归属边界仍未完成**。

---

## 5. 状态

- `UI_OWNERSHIP_GRAPH = CREATED`
- `WORKBENCH_CAPABILITY_COUPLINGS = 11`（Shell 直渲业务面板）
- `RESOURCE_CREATION_VIOLATIONS = 0`
- `BOUNDARY_VIOLATION = 1`
- Contribution GAP：**8 项**（记录，不立即抽象）
- RULE OF TWO 已满足的抽象：**3**（EmptyState / LoadingState / ModalShell）
