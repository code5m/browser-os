# 20 · SHELL CAPABILITY KNOWLEDGE（Shell/Core 对能力的知晓审计）

> 扫描 `App.vue` / `MainArea.vue` / `ActivityBar.vue` / Dock / Toolbar / `useLayoutStore`
> 及其它 Shell/Core 文件，找出对 Browser / Grid / Terminal / Workspace / Bookmark / Git /
> Database / Agent / Skill / Plugin 的 import、分支、生命周期调用。
>
> 分类：`GENERIC_SHELL_BEHAVIOR` / `DECLARED_CONTRIBUTION` / `PUBLIC_CONTRACT_USAGE` /
> `LEGACY_COUPLING` / `RESOURCE_CREATION_VIOLATION`
>
> 硬指标：**RESOURCE_CREATION_VIOLATION = 0**（已达成）。
> 本阶段**不**强行把 LEGACY_COUPLING 清零 —— `保护 UI > 一次性大重构`。

---

## 1. 汇总

| 分类 | 条目数 | 处置 |
|---|---|---|
| GENERIC_SHELL_BEHAVIOR | 12 | 保留 |
| DECLARED_CONTRIBUTION | 7（槽） | 保留 |
| PUBLIC_CONTRACT_USAGE | 6 | 保留（合法） |
| LEGACY_COUPLING | 9 | **保留 UX**，逐项迁移 ownership |
| **RESOURCE_CREATION_VIOLATION** | **0** | 门禁持续守护 |

---

## 2. 逐文件审计

### 2.1 `src/App.vue`

| 符号 | 引用 | 分类 |
|---|---|---|
| `useBrowserStore` (import) | `capabilities/browser/public` | PUBLIC_CONTRACT_USAGE |
| `browser.activateGrid()`（L206，km.grid 快捷键） | 键盘意图 | PUBLIC_CONTRACT_USAGE |
| `browser.tabNew/tabClose/tabSwitch/reloadActive` | 快捷键 | PUBLIC_CONTRACT_USAGE |
| `browser.setResources / setTitle / setNavigated / handleTabRecovery` | 事件订阅回填 | PUBLIC_CONTRACT_USAGE |
| `layout.toggleBrowserDock("term")` | 容器开关 | GENERIC_SHELL_BEHAVIOR |
| `term.loadM0Config / activateTerm` | 视图导航 | PUBLIC_CONTRACT_USAGE |
| `useWorkspaceStore / useArtifactStore / useRepoStore / useFileStore / useGitStore / useTerminalStore / useSystemStore` | import | PUBLIC_CONTRACT_USAGE（经 public.ts） |

### 2.2 `src/components/layout/MainArea.vue`

| 符号 | 分类 |
|---|---|
| `WORKBENCH_MAIN_RESIDENT` / `WORKBENCH_MAIN` / `browser-sidebar` / `browser-host` / `browser-dock` 槽遍历 | **DECLARED_CONTRIBUTION**（Shell 只认槽名，不认能力） |
| `panelLoading` / `panelError` 通用异步兜底 | GENERIC_SHELL_BEHAVIOR |
| 「当前视图不可用」兜底面板 | GENERIC_SHELL_BEHAVIOR |
| `layout.browserDockOpen / browserDockTab` | LEGACY_COUPLING（Browser 语义状态落在 Shell） |

### 2.3 `src/components/layout/ActivityBar.vue`

| 符号 | 分类 |
|---|---|
| `TOP_NAV_ITEMS` / `NAV_MENU_SECTIONS` / `MODULE_META`（store 真源） | GENERIC_SHELL_BEHAVIOR |
| `ADDRESS_BAR_ACTIONS` / `ACTIVITY_BAR_TRAILING` 贡献位 | **DECLARED_CONTRIBUTION** |
| `browser.activateGrid()` L167 | PUBLIC_CONTRACT_USAGE |
| `browser.rebuildGrid()` L137/L148 | PUBLIC_CONTRACT_USAGE |
| `browser.goBack/goForward/reloadActive` L304-306 | LEGACY_COUPLING |
| `browser.gridCount / gridLayout / gridUrls / gridAiInput / gridOpen / gridMode` 直读写 | LEGACY_COUPLING |
| `layout.toggleBrowserDock(...)` L334-335 | LEGACY_COUPLING |
| `browser.toggleAiNav()` L339-348 | LEGACY_COUPLING |
| `art.collectSelection` L349-352 | LEGACY_COUPLING（Workspace 语义，Shell 直连） |

### 2.4 `src/components/layout/UnifiedTabBar.vue`

| 符号 | 分类 |
|---|---|
| `browser.openGrid()` L172（宫格页签重新激活） | PUBLIC_CONTRACT_USAGE |
| `browser.tabs / activeTabId / tabSwitch / tabClose` | PUBLIC_CONTRACT_USAGE |
| `layout.isBrowserView()` | GENERIC_SHELL_BEHAVIOR |

### 2.5 `src/stores/useLayoutStore.ts`（★ 本次修复对象）

| 符号 | 修复前 | 修复后 |
|---|---|---|
| `useBrowserStore()` in `toggleGridToolbar` | **RESOURCE_CREATION_VIOLATION**（且 `useBrowserStore` 未 import → 潜在 ReferenceError） | **已移除** |
| `browser.openGrid()` / `browser.closeGrid()` | RESOURCE_CREATION_VIOLATION | **已移除** |
| `useBrowserStore().url = p`（L307-308 动态 import） | PUBLIC_CONTRACT_USAGE（M0-4.b 已记录取舍） | 保留（omnibox 目录路径同步） |
| `gridToolbarOpen` | 被当作资源创建许可（PROBLEM B） | **纯 UI 偏好**（Shell 只持有，不翻译为资源） |
| `browserDockOpen / browserDockTab` | LEGACY_COUPLING | 保留（UI preservation） |
| `MainView` 含 `browser/grid/term/files/...` | GENERIC_SHELL_BEHAVIOR（通用视图枚举） | 保留 |

### 2.6 `src/stores/useWorkbenchStore.ts`

| 符号 | 分类 |
|---|---|
| `import { useBrowserStore } from "../capabilities/browser/public"` | PUBLIC_CONTRACT_USAGE |
| `browser.activateGrid()`（命令面板 grid） | PUBLIC_CONTRACT_USAGE |
| `browser.aiNavOpen` 快照/恢复（toggleTools） | LEGACY_COUPLING |

### 2.7 `src/components/layout/StatusBar.vue`

| 符号 | 分类 |
|---|---|
| 视图中文名映射（22 视图） | GENERIC_SHELL_BEHAVIOR |
| 「终端 就绪/未启」「仓库 已配置」「审计 N 条」 | PUBLIC_CONTRACT_USAGE（只读展示） |
| 资源摘要（内存/每宫格 RSS/预算） | LEGACY_COUPLING（宫格专属遥测） |

### 2.8 `src/components/layout/WorkbenchRail.vue` / `WorkbenchCommands.vue`

| 符号 | 分类 |
|---|---|
| 5 个视图按钮（files/db/vault/repo/term） | GENERIC_SHELL_BEHAVIOR |
| 命令面板（聚合入口） | GENERIC_SHELL_BEHAVIOR |

---

## 3. RESOURCE_CREATION_VIOLATION 判定口径

门禁 `scripts/check-capability-resource-boundary.mjs` 按**已声明契约**区分两类，避免一刀切：

| 类别 | 符号 | Shell 调用是否违规 |
|---|---|---|
| 公开意图入口（能力声明给组件用） | `openGrid` / `closeGrid` / `activateGrid` / `rebuildGrid` / `closeGridCell` / `spawnTerm` / `ensureTerm` / `addTermPane` | **否**（入口内部才做资源决策，且必须过能力闸） |
| 能力内部实现 | `buildGrid` / `closeGridAll` / `closeGridOne` | **是** |
| 绕过能力直触 native 出生点 | `bridge.createGrid` / `bridge.closeGrid` / `bridge.termSpawnChannel` / `invoke("create_grid")` / `invoke("term_spawn…")` | **是** |
| import 能力内部 `state/` | `from ".../capabilities/*/state/"` | **是** |

> 为什么公开意图入口不算违规：若把它也禁掉，Shell 只能绕过能力自建资源，
> 与「资源只能由能力生命周期创建」的目标**相反**。
> 该口径由负例 BR-06 / BR-06b 双向守住（既不放过真违规，也不误伤合法用法）。

---

## 4. 当前结果

```
BR-01  Shell/Core 10 个文件均未 import 能力内部实现（state/）        PASS
BR-02  Shell/Core 均未绕过能力契约直触资源出生点（0 VIOLATION）      PASS
BR-02b Shell 使用能力公开意图入口（PUBLIC_CONTRACT_USAGE，合法）      PASS（4 处）
```

保留的 9 项 LEGACY_COUPLING 全部**不改 UX**，仅登记为后续迁移项。
