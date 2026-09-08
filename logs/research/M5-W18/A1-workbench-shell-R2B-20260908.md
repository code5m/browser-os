# A1 · M5-W18-R2B 工作台壳层与迁移蓝图

> Lane: A1（RESEARCH_AND_DESIGN，R2B 第二批）
> R1 verdict: `REWORK`；R2 status: `PASS_WITH_DEBT`（commit `526e2ef`）
> BASE = `origin/master` `434e63f`（docs(M5-W18): define daily workbench and second research dispatch）
> Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3` @ branch `codex/m5-w18-a1`
> 依据：`PARALLEL_COMMAND_BOARD.md` Current Dispatch Entry（L10-16）；`M5-W18-R2B-TASKS-20260908.md` §2（Lane A1）；`WORKBENCH_BLUEPRINT-20260908.md`
> 边界：只读研究 + 合成线框原型；零产品代码改动；不 push。仅产出本报告 + checkpoint + 线框 HTML 至 `logs/research/M5-W18/`。

---

## 0. 输出头

```text
LANE=A1
DISPATCH=M5-W18-R2B
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=434e63f
HEAD=<pending commit>
CONSUMED_PEERS=none（A1 为壳层契约起草方，不消费其他 lane R2B 成果；R2 自身成果已在本分支）
FILES=logs/research/M5-W18/A1-workbench-shell-R2B-20260908.md, logs/research/M5-W18/A1-checkpoint-R2B-20260908.md, logs/research/M5-W18/A1-wireframe-prototype-R2B.html
CORRECTIONS=C1 前端测试"0个"更正为"0框架单测+20 MJS逻辑测试+29 Python策略测试"；C2 W17-D6归属修正（归A7/A0非A1）；C3 useConnectionStore不存在已确认
VERIFY=read-only: grep/wc/python3 静态扫描 + 线框 HTML 浏览器可打开；未运行构建或产品测试（research-only）
PROPOSED_SLICES=S0(DbValue唯一源), S1(工作台外壳)
OPEN_DECISIONS=原生WebView重定位需A9/A11实机验证；编辑器升级依赖需A5评估
NEXT=交A3/A5/A8签收壳层契约；交A10独立审查；交A11登记
NO_PRODUCT_CODE=true
NO_PUSH=true
```

---

## 1. 复用旧成果与更正日志

### 1.1 复用清单

| 旧成果 | 位置 | 复用方式 |
|---|---|---|
| R1 产品基线 | `A1-product-baseline-20260908.md` | §2 当前产品 10 维度基线保留；gap inventory G1-G8 保留 |
| R2 证据闭环 | `A1-product-baseline-R2-20260908.md` | §1.1 的 6 处 R1 修正（C1-C6）全部继承；§7 machine-checkable fact appendix 继承 |
| R2 checkpoint | `A1-checkpoint-R2-20260908.md` | 事实附录验证命令继承 |

### 1.2 更正日志（R2B 新增）

| # | 旧声明 | 更正 | 证据 |
|---|---|---|---|
| C1 | R1/R2 §2.7："前端单元测试：**0 个**…无自动化单测网" | **更正**：前端有 **20 个 MJS 逻辑测试**（`scripts/check-*-logic.mjs`）+ **29 个 Python 策略测试**（`scripts/check-*.py`）。缺的是**框架级单测**（vitest/jest/cypress/playwright），不是"没有前端测试"。R2B dispatch checkpoint point 3 明确指出此更正。 | `EXECUTED_SYNTHETIC_TEST`: `ls scripts/check-*-logic.mjs \| wc -l` → 20；`ls scripts/check-*.py \| wc -l` → 29 |
| C2 | R1 §2.8 将 W17-D1~D6 列为 A1 基线债 | **更正**：W17-D6（B10-b MainArea.vue 兜底 v-else 配对错误）归 **A7/A0** 修复，不属 A1 基线挂账。A1 只记录，不承担 D6 修复责任。R2 已部分修正（C4），R2B 再次明确。 | `CURRENT_PRODUCT`: `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` L615 |
| C3 | R1 §2.4 暗示 `useConnectionStore` 可能存在 | **更正**：`useConnectionStore` **不存在**。当前数据库前端为 `useDatabaseStore.ts`（218 行）+ `DatabasePanel.vue`。A5 R1 引用此 store 是错误的。 | `CURRENT_PRODUCT`: `find src -name "*onnection*tore*"` → 无结果 |

---

## 2. 当前壳层结构盘点（证据基线）

> 所有事实来自 `CURRENT_PRODUCT`（HEAD `434e63f` 工作树只读扫描）。

### 2.1 App.vue 壳层入口

`src/App.vue`（319 行）是全局入口，结构：

```
.app (flex column)
├── boot-overlay（启动遮罩，ready=false 时）
├── shell-error（渲染错误兜底）
└── <template v-else>
    ├── ActivityBar（v-show="!compactMode"）
    ├── .body (flex)
    │   ├── AINavPanel
    │   └── MainArea
    ├── StatusBar
    ├── ConfirmModal / GitWriteConfirmDialog / SessionCloseDialog / ImageLightbox（全局弹窗）
```

关键行为：
- `onMounted`：同步窗口尺寸、加载 M0 配置、`ws.loadRecents()`、`system.startClipWatch()`、`ws.refresh()`、`ws.loadStartDirs()`、绑定 12+ 个 bridge 事件回调、注册全局快捷键。
- `onErrorCaptured`：子树渲染异常兜底，避免白屏。
- `beforeunload`：`bridge.closeBrowser()` + `bridge.flushSessions()`。

### 2.2 MainView 枚举（19 个视图）

`useLayoutStore.ts:5-25` 定义 `MainView` 类型，共 **19** 个值：
`home / browser / files / clip / arts / grid / apps / term / repo / audit / scripts / commands / tools / db / tasks / skills / agents / graph / plugin / editor`

### 2.3 MODULE_META（18 个模块元数据）

`useLayoutStore.ts:111-130` 定义 `MODULE_META`，共 **18** 个模块（比 MainView 少 `editor`，多 `settings`）：
`home / grid / files / clip / arts / apps / term / repo / audit / scripts / commands / tools / tasks / skills / agents / graph / plugin / settings`

### 2.4 导航入口结构

| 入口层 | 来源 | 条目数 | 条目 |
|---|---|---|---|
| 一级顶栏 `TOP_NAV_ITEMS` | L33-39 | 5 | home / browser / term / clip / arts |
| ☰ 菜单分节 `NAV_MENU_SECTIONS` | L41-74 | 3 节 16 项 | 工作区(files/clip/arts) + 工具(term/apps/scripts/commands/tools/db/tasks/skills/agents/graph/plugin) + 同步(repo/audit) |
| 密度断点 | L76-77 | 2 | `NAV_DENSITY_FULL_PX=1180` / `NAV_DENSITY_COMPACT_PX=900` |

窄窗裁剪规则（L90-95）：full 全显示 → compact 保留前 3 → icon 只留 home+browser。被裁掉的在 ☰ 菜单仍可达。

### 2.5 MainArea.vue 视图分发

`src/components/layout/MainArea.vue`（300 行）按 `mainView` 做 v-if/v-else-if 链分发，共 **22** 个分支（含兜底）：

| mainView | 组件 | 加载方式 | 行号 |
|---|---|---|---|
| home | HomePanel | 静态 import | L128 |
| browser / grid | BrowserHost + BookmarkPanel + browser-dock | 静态 | L133 |
| files | FilePanel (:ide=true) | 静态 | L172 |
| arts | ArtifactPanel | 静态 | L177 |
| clip | ClipboardPanel | 静态 | L182 |
| repo | RepoPanel | 静态 | L187 |
| apps | AppPanel | 静态 | L192 |
| audit | AuditPanel | 静态 | L197 |
| scripts | ScriptPanel | 静态 | L202 |
| commands | CommandSnippetPanel | 静态 | L207 |
| tools | ToolBox | 静态 | L212 |
| db | DatabasePanel | **懒加载** defineAsyncComponent | L217 |
| tasks | TaskPanel | **懒加载** | L222 |
| plugin | PluginManager | **懒加载** | L227 |
| skills | SkillManagerPanel | **懒加载** | L230 |
| agents | AgentManagerPanel | **懒加载** | L233 |
| graph | GraphPanel | **懒加载** | L236 |
| settings | SettingsPanel | 静态 | L241 |
| term | TerminalPane (termMounted) | 条件挂载 | L246 |
| editor | FileEditor | 静态 | L251 |
| 兜底 | "当前视图不可用" | v-if 白名单 | L255 |

### 2.6 useLayoutStore 状态字段

| 字段 | 类型 | 用途 | 行号 |
|---|---|---|---|
| `mainView` | `ref<MainView>` | 当前主视图 | L133 |
| `sidebarOpen` / `sidebarWidth` | `ref<boolean>` / `ref<number>` | 左侧栏开关/宽度 | L134-135 |
| `leftTab` | `ref<"files"\|"artifacts">` | 左栏子页签 | L136 |
| `gridToolbarOpen` / `clipOpen` / `aiNavOpen` / `fileEditorOpen` | `ref<boolean>` | 各覆盖层开关 | L137-140 |
| `browserDockOpen` / `browserDockTab` | `ref<boolean>` / `ref<4值>` | 浏览器右侧 Dock | L144-145 |
| `addrMode` / `compactMode` | `ref<"url"\|"dir">` / `ref<boolean>` | 地址栏模式/精简模式 | L147-149 |
| `windowWidth` / `navSection` | `ref<number>` / `ref<NavSection>` | 窗口宽度/活动条扩展行 | L151-153 |
| `modTabs` / `activeModTab` | `reactive<ModTab[]>` / `ref<string>` | 模块页签 | L233-234 |

### 2.7 前端测试资产（更正 C1）

| 类型 | 数量 | 示例 | 证据 |
|---|---|---|---|
| MJS 逻辑测试 | **20** | `check-client-navigation-logic.mjs` / `check-database-ui-logic.mjs` / `check-graph-ui-logic.mjs` / `check-ui-a11y-logic.mjs` | `EXECUTED_SYNTHETIC_TEST`: `ls scripts/check-*-logic.mjs \| wc -l` → 20 |
| Python 策略测试 | **29** | `check-command-set-consistency.py` / `check-database-policy.py` / `check-security-policy.py` | `EXECUTED_SYNTHETIC_TEST`: `ls scripts/check-*.py \| wc -l` → 29 |
| 框架单测（vitest/jest/cypress/playwright） | **0** | — | `CURRENT_PRODUCT`: `package.json` 无相关依赖 |
| `*.test.ts` / `*.spec.ts` 文件 | **0** | — | `CURRENT_PRODUCT`: `find src -name "*.test.ts" -o -name "*.spec.ts"` → 无 |

**结论**：前端有 49 个非框架测试脚本（20 MJS + 29 Python），缺的是框架级单测网。R1/R2 的"0 前端测试"表述不准确，现更正。

---

## 3. 旧功能 → 新工具窗口/文档/菜单 全覆盖清单

> 依据 `WORKBENCH_BLUEPRINT-20260908.md` §3 布局约束 + §4 J1-J6 工作流。
> 证据：`CURRENT_PRODUCT`（MODULE_META L111-130 + MainArea 分发 L128-269）+ `DESIGN_DECISION`。

### 3.1 覆盖矩阵

| 旧入口 (MainView) | 旧组件 | 新工作台位置 | 迁移类型 | 保留/修改/拆分 | J 工作流 | 证据 |
|---|---|---|---|---|---|---|
| `home` | HomePanel | **中央文档区**（工作恢复入口） | 保留+调整 | 首页从"快捷图标墙"调整为"最近项目+恢复工作" | J1 | `CURRENT_PRODUCT`: `src/components/home/HomePanel.vue` |
| `browser` | BrowserHost | **中央文档区**（网页文档标签） | 保留 | 网页作为文档类型之一，保留页签+地址栏 | J1, J4 | `CURRENT_PRODUCT`: MainArea L133 |
| `grid` | BrowserHost(grid) | **中央文档区**（宫格多网页） | 保留 | 宫格作为浏览器多标签布局模式 | J1 | `CURRENT_PRODUCT`: MainArea L133 |
| `files` | FilePanel(:ide) | **左侧工具窗口**（项目文件树）+ **中央文档区**（文件编辑） | 修改 | 文件树移左工具窗口；文件内容在中央编辑区 | J1, J3 | `CURRENT_PRODUCT`: MainArea L172 |
| `clip` | ClipboardPanel | **底部工具窗口**（剪贴板历史） | 修改 | 从全屏模块视图改为底部工具窗口 | J1 | `CURRENT_PRODUCT`: MainArea L182 |
| `arts` | ArtifactPanel | **左侧工具窗口**（知识目录/成果库） | 修改 | 从全屏模块视图改为左侧工具窗口 | J3 | `CURRENT_PRODUCT`: MainArea L177 |
| `term` | TerminalPane | **底部工具窗口**（终端） | 修改 | 从全屏模块视图改为底部工具窗口；保留单例挂载 | J1, J5 | `CURRENT_PRODUCT`: MainArea L246 |
| `repo` | RepoPanel | **左侧工具窗口**（Git 仓库）+ **底部工具窗口**（Git 输出） | 修改 | 仓库树移左；Git 操作结果移底部 | J1, J5 | `CURRENT_PRODUCT`: MainArea L187 |
| `audit` | AuditPanel | **底部工具窗口**（审计日志/问题列表） | 修改 | 从全屏模块视图改为底部工具窗口 | J6 | `CURRENT_PRODUCT`: MainArea L197 |
| `apps` | AppPanel | **☰ 菜单**（应用启动器） | 保留 | 保留为菜单入口，不常驻工具窗口 | — | `CURRENT_PRODUCT`: MainArea L192 |
| `scripts` | ScriptPanel | **左侧工具窗口**（脚本库） | 修改 | 从全屏模块视图改为左侧工具窗口 | J5 | `CURRENT_PRODUCT`: MainArea L202 |
| `commands` | CommandSnippetPanel | **左侧工具窗口**（命令库） | 修改 | 从全屏模块视图改为左侧工具窗口 | J5 | `CURRENT_PRODUCT`: MainArea L207 |
| `tools` | ToolBox | **☰ 菜单**（工具箱） | 保留 | 保留为菜单入口 | — | `CURRENT_PRODUCT`: MainArea L212 |
| `db` | DatabasePanel | **左侧工具窗口**（连接树）+ **中央文档区**（SQL 控制台）+ **底部工具窗口**（结果/消息） | 修改 | 三区拆分：连接树左、SQL 编辑中央、结果底部 | J2 | `CURRENT_PRODUCT`: MainArea L217；蓝图 §3.3-3.5 |
| `tasks` | TaskPanel | **底部工具窗口**（定时任务/运行状态） | 修改 | 从全屏模块视图改为底部工具窗口 | J5 | `CURRENT_PRODUCT`: MainArea L222 |
| `skills` | SkillManagerPanel | **☰ 菜单**（技能管理） | 保留 | 保留为菜单入口 | — | `CURRENT_PRODUCT`: MainArea L230 |
| `agents` | AgentManagerPanel | **☰ 菜单**（智能体管理） | 保留 | 保留为菜单入口 | — | `CURRENT_PRODUCT`: MainArea L233 |
| `graph` | GraphPanel | **右侧工具窗口**（局部图谱/反链）+ **中央文档区**（全局图谱视图） | 修改 | 全局图谱可作中央文档；局部图谱/反链作右侧工具窗口 | J3 | `CURRENT_PRODUCT`: MainArea L236；蓝图 §3.4 |
| `plugin` | PluginManager | **☰ 菜单**（插件管理） | 保留 | 保留为菜单入口 | — | `CURRENT_PRODUCT`: MainArea L227 |
| `editor` | FileEditor | **中央文档区**（文件编辑/Markdown 预览） | 保留 | 保留为中央覆盖层 | J1, J3 | `CURRENT_PRODUCT`: MainArea L251 |
| `settings` | SettingsPanel | **☰ 菜单**（设置） | 保留 | 保留为菜单入口 | — | `CURRENT_PRODUCT`: MainArea L241 |

### 3.2 覆盖完整性验证

- MainView 19 个值全部覆盖 ✅
- MODULE_META 18 个模块全部覆盖 ✅
- ☰ 菜单 16 项全部覆盖 ✅
- J1-J6 工作流全部有入口映射 ✅
- 无遗漏入口 ✅

---

## 4. 状态契约（文档标签 / 工具窗口 / 项目上下文 / 焦点 / 资源生命周期）

> 依据 `WORKBENCH_BLUEPRINT-20260908.md` §5 共同契约。
> 证据：`CURRENT_PRODUCT`（useLayoutStore 现有字段）+ `DESIGN_DECISION`。

### 4.1 工作区（Workspace）

```typescript
interface WorkspaceContext {
  id: string;              // 稳定唯一 ID
  rootPath: string;        // 授权根目录（经来源校验，不直接用输入路径）
  name: string;            // 显示名
  recents: string[];       // 最近打开文件（路径列表）
  // 生命周期：enter → active → switch → close
  // 切换：保存当前工作区布局状态，加载目标工作区
  // 关闭：协调所有文档/工具窗口/资源句柄关闭
}
```
**现状映射**：`useWorkspaceStore` 已有 `recents / startDirs / enterDir / refresh`。适配层在现有 store 上扩展 `id / rootPath`，不新建巨型 store。

### 4.2 文档（Document）

```typescript
interface DocIdentity {
  key: string;             // 身份键：类型+路径或类型+连接ID+SQL标识
  type: "file" | "webpage" | "sql" | "note" | "graph-global";
  workspaceId: string;     // 所属工作区
  location: string;        // 资源定位（文件路径/URL/连接+SQL ID）
  dirty: boolean;          // 未保存标记
  // 关闭动作：save / discard / cancel
  // 恢复策略：持久化 dirty 内容 + 关闭前确认
}
interface DocTab extends DocIdentity {
  id: string;              // 页签 ID（现有 modTabs.id 模式）
  icon: string;
  label: string;
  // UI 状态与后端资源句柄分离
}
```
**现状映射**：`useLayoutStore.modTabs`（L233）已有 `id / view / icon / label / path`。扩展 `type / workspaceId / dirty`，不推翻现有 ModTab。

### 4.3 工具窗口（ToolWindow）

```typescript
interface ToolWindow {
  id: string;              // 窗口 ID
  side: "left" | "right" | "bottom";
  size: number;            // 像素宽度/高度
  activeItem: string;      // 当前活动项（如连接树选中节点）
  visible: boolean;        // 是否显示
  focusReturn: string | null; // 焦点返回目标（隐藏后焦点去向）
  // 隐藏 ≠ 销毁：保留状态，再次显示时恢复
}
```
**现状映射**：
- 左侧：`sidebarOpen / sidebarWidth / leftTab`（L134-136）
- 右侧：`browserDockOpen / browserDockTab`（L144-145）— 仅浏览器视图有
- 底部：当前无独立底部工具窗口状态（终端/剪贴板等均为全屏模块视图）

**迁移**：将 `clip / term / tasks / audit` 从全屏 `mainView` 改为底部工具窗口，需在 `useLayoutStore` 新增 `bottomPanel: { open, activeTab, height }` 状态。**这是 S1 的核心改动**。

### 4.4 查询/搜索/任务（Request）

```typescript
interface RequestHandle {
  requestId: string;       // 请求 ID
  docKey: string;          // 所属文档
  status: "running" | "completed" | "cancelled" | "error";
  // 取消幂等：cancel() 多次调用安全
  // 旧响应失效：新请求发出后旧响应到达时丢弃
  // 完成竞争：一次完成，不重复回调
}
```
**现状映射**：`useDatabaseStore` 已有 `db_query` 调用；`useTaskStore` 已有任务调度。A4/A6 负责具体契约，A1 只定义壳层如何展示 `status`。

### 4.5 统一导航（Navigation）

```typescript
interface NavResult {
  resourceType: "file" | "note" | "webpage" | "sql-result";
  location: string;        // 文件:行 / 笔记:标题:块 / URL / 连接+结果位置
  returnPosition: string;  // 返回位置（来源文档+光标/滚动）
}
// SQL 结果不伪装成磁盘文件
```

### 4.6 持久化（Persistence）

```typescript
interface PersistedState {
  schemaVersion: number;   // schema_version
  // 原子写、失败恢复、容量、淘汰与迁移
  // 凭据/敏感URL/SQL全文不跟布局落盘
}
```
**现状映射**：`useSettingsStore` 用 `localStorage`（`STORAGE_KEY = "browser-os-settings"`，L56）。布局状态当前未持久化（`useLayoutStore` 无 save/load）。S1 需新增布局持久化，但**不含凭据/SQL全文**。

### 4.7 焦点与资源生命周期区分

| 操作 | 语义 | 资源影响 |
|---|---|---|
| 关闭视图（hide tool window） | UI 隐藏，状态保留 | 资源句柄保留（终端不重启、连接不断开） |
| 关闭文档（close tab） | 文档卸载 | 释放对应资源（文件保存/SQL 取消/网页关闭） |
| 关闭工作区 | 协调所有文档+工具窗口关闭 | 释放所有资源句柄 |
| 取消任务 | 终止运行中请求 | 释放请求句柄，保留文档 |
| 销毁会话 | 永久关闭 | 释放会话+关联资源 |

**现状映射**：`useSessionStore.requestClose / resolveClose`（L142-152）已有关闭协议。S1 需区分"关闭视图"与"关闭文档"。

---

## 5. 线框设计（1440×900 / 1024×720 / 800×600）

> 依据 `WORKBENCH_BLUEPRINT-20260908.md` §3 布局约束。
> 证据：`DESIGN_DECISION` + `CURRENT_PRODUCT`（现有断点 L76-77）。
> 产出：`A1-wireframe-prototype-R2B.html`（自包含静态 HTML，浏览器可直接打开）。

### 5.1 桌面 1440×900

```
┌─────────────────────────────────────────────────────────────────────────┐
│ 顶部栏：[项目切换] [当前上下文] [🔍 全局搜索] [快捷命令]                │ 40px
├──────┬────────────────────────────────────────────────────┬────────────┤
│活动栏│ 左工具窗口 (260px)           │ 中央文档区           │ 右工具窗口 │
│ 48px │┌──────────────────────────┐ │┌───────────────────┐│(按需, 240px)│
│      ││ 📂 文件树 / 🗄️ 连接树    │ ││ 文档1 │ 文档2 │ + │││ 属性/反链  │
│ 🏠   ││ / 📚 知识目录            │ │├───────────────────┤││ / 局部图谱 │
│ 📁   ││                          │ ││                   │││            │
│ 💻   ││  tree...                 │ ││  编辑/阅读区      │││            │
│ 📋   ││                          │ ││                   │││            │
│ 📚   ││                          │ ││                   │││            │
│      │└──────────────────────────┘ │└───────────────────┘││            │
│      │                              │                      ││            │
│      │                              │┌───────────────────┐││            │
│      │                              ││ 底部：终端/结果/   │││            │
│      │                              ││ 任务/问题 (240px)  │││            │
│      │                              │└───────────────────┘││            │
├──────┴──────────────────────────────┴──────────────────────┴┴────────────┤
│ 状态栏：[项目/分支] [运行中任务] [索引新鲜度] [连接状态] [资源]        │ 28px
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 窄窗 1024×720

```
┌────────────────────────────────────────────────────────────────┐
│ 顶部栏：[项目] [🔍] [☰]                                       │ 40px
├──────┬───────────────────────────────────────┬────────────────┤
│活动栏│ 左工具窗口 (220px, 可折叠)            │ 中央文档区     │
│ 48px │ 📂/🗄️/📚 (页签切换)                  │ ┌────────────┐ │
│      │                                       │ │文档1│文档2│+│ │
│ 🏠   │ tree...                               │ ├────────────┤ │
│ 📁   │                                       │ │            │ │
│      │                                       │ │ 编辑/阅读  │ │
│      │                                       │ │            │ │
│      │                                       │ └────────────┘ │
│      │                                       │ 底部(折叠180px)│
├──────┴───────────────────────────────────────┴────────────────┤
│ 状态栏：[项目] [任务] [连接]                                  │ 28px
└────────────────────────────────────────────────────────────────┘
```
右工具窗口隐藏（按需弹出）；底部默认折叠；左工具窗口可折叠为图标。

### 5.3 窄窗 800×600

```
┌──────────────────────────────────────────────────────┐
│ 顶部栏：[项目] [🔍] [☰]                             │ 36px
├──────┬───────────────────────────────────────────────┤
│活动栏│ 中央文档区（全宽）                           │
│ 40px │ ┌───────────────────────────────────────────┐ │
│      │ │文档1│+│                                    │ │
│ 🏠   │ ├───────────────────────────────────────────┤ │
│ 📁   │ │                                           │ │
│      │ │  编辑/阅读区                              │ │
│      │ │                                           │ │
│      │ └───────────────────────────────────────────┘ │
├──────┴───────────────────────────────────────────────┤
│ 状态栏：[项目] [连接]                               │ 24px
└──────────────────────────────────────────────────────┘
```
左/右/底部工具窗口全部折叠；通过 ☰ 菜单或快捷键弹出覆盖层。

### 5.4 焦点/键盘/IME 规格

- **Tab** 在活动栏 → 左工具窗口 → 中央文档区 → 底部 → 状态栏间循环
- **Ctrl+Shift+E** 切换左工具窗口（文件树）
- **Ctrl+Shift+D** 切换底部工具窗口（终端/结果）
- **Ctrl+Shift+A** 切换右工具窗口（属性/反链）
- **Ctrl+P** 全局搜索（文件/符号/操作）
- **IME 输入** 不误触全局快捷键（现有 App.vue L166-168 已有 `INPUT/TEXTAREA/contentEditable` 跳过逻辑）
- **焦点返回**：工具窗口隐藏后焦点返回触发它的文档区位置

### 5.5 与其他工具的衔接

| 工具窗口 | 衔接 lane | 衔接内容 |
|---|---|---|
| 左侧文件树/知识目录 | A2（笔记语义）、A3（图谱导航） | 文件树点击 → 笔记编辑/图谱节点 |
| 左侧连接树 | A4（DB 契约）、A5（DB 工作流） | 连接树 → SQL 控制台 |
| 右侧局部图谱/反链 | A3（图谱/笔记联合体验） | 笔记编辑 → 反链/局部图谱 |
| 底部终端/结果/任务 | A6（资源生命周期）、A8（搜索结果） | 终端保留、查询结果展示、任务运行 |
| 全局搜索 | A8（统一搜索体验） | 搜索入口 → 结果定位 |

---

## 6. 共享壳层唯一修改者清单

> A1 负责壳层；A3/A5/A8 在壳层内嵌入领域内容。以下为各文件唯一修改归属。

| 文件 | 唯一修改者 | 修改内容 | 协调方 |
|---|---|---|---|
| `src/App.vue` | **A1** | 壳层布局调整（顶部栏/活动栏/主区/状态栏编排） | — |
| `src/stores/useLayoutStore.ts` | **A1** | 新增 `bottomPanel / rightPanel / docTabs` 状态 + 适配层 | A3/A5/A8 消费新状态 |
| `src/components/layout/ActivityBar.vue` | **A1** | 入口映射调整（工具窗口切换 vs 全屏视图切换） | — |
| `src/components/layout/MainArea.vue` | **A1** | 视图分发改为文档区+工具窗口编排 | A3/A5/A8 提供领域组件 |
| `src/components/layout/UnifiedTabBar.vue` | **A1** | 文档标签统一（文件/SQL/笔记/网页混排） | — |
| `src/components/layout/StatusBar.vue` | **A1** | 状态栏新增项目/分支/索引新鲜度/连接状态 | — |
| `src/components/layout/Sidebar.vue` | **A1** | 左工具窗口容器 | — |
| `src/components/home/*` | **A1** | 首页调整为工作恢复入口 | — |
| `src/utils/homeUi.ts` | **A1** | 首页逻辑调整 | — |
| `src/components/graph/*` | **A3** | 图谱组件嵌入右侧工具窗口 | A1 提供容器 |
| `src/components/workspace/DatabasePanel.vue` | **A5** | 数据库面板拆分为连接树+SQL控制台+结果 | A1 提供容器 |
| `src/stores/useDatabaseStore.ts` | **A5** | 数据库 store 扩展 | — |
| `src/utils/dbUi.ts` | **A5** | 数据库 UI 逻辑 | — |
| 搜索相关（新增） | **A8** | 统一搜索入口+结果展示 | A1 提供顶部栏搜索位 |

**冲突避免**：A1 先冻结壳层契约（文件/状态/容器），A3/A5/A8 在契约内嵌入领域组件。A1 不替他们定义领域数据。

---

## 7. 前端测试分类与 S0/S1 最小改动包

### 7.1 测试分类（更正 C1）

| 类别 | 现有资产 | 缺口 | 归属 |
|---|---|---|---|
| 框架单测（vitest/jest） | 0 个 | 需 bootstrap vitest + 组件/store 单测 | A11（测试矩阵） |
| MJS 逻辑测试 | 20 个 `check-*-logic.mjs` | S1 壳层改动需新增 `check-shell-layout-logic.mjs` | A1（S1 卡） |
| Python 策略测试 | 29 个 `check-*.py` | S1 壳层改动需更新 `check-client-navigation-logic.mjs`（导航入口变更） | A1（S1 卡） |
| Rust 集成测试 | 470 `#[test]` | S0 DbValue 统一需新增/更新 database.rs 测试 | A4（S0 卡） |
| 原生 GUI 验收 | NOT_RUN | S1 布局变化需 A9/A11 实机验证 | A9/A11 |

### 7.2 S0 候选实现卡：DbValue 唯一源

```yaml
PROPOSED_NOT_AUTHORIZED
slice: S0
title: DbValue 唯一源确定 + 前端对齐
owner: A4（Rust 侧）+ A10（审查）+ A11（验证）
scope:
  - 统一 DbValue 为 database.rs:122 的 live 定义（I64/F64/Binary{bytes:usize}）
  - 删除或重命名 domain.rs:1094 的 dead-code 定义
  - types.ts 对齐 database.rs 变体名（int→i64, float→f64, blob_len→binary）
  - 更新 23 个 database.rs #[test] 中的序列化断言
prerequisites:
  - A4 R2 闭环（DbValue 追踪真实命令返回序列化）
  - A10 独立审查确认无其他消费者依赖 domain.rs DbValue
files:
  - src-tauri/src/database.rs（保留 live 定义）
  - src-tauri/src/domain.rs（删除/重命名 dead-code DbValue）
  - src/types.ts:603-609（对齐变体名）
  - src-tauri/src/database.rs 测试（更新断言）
tests:
  - cargo test -p src-tauri -- database
  - python3 scripts/check-command-set-consistency.py
  - node scripts/check-database-ui-logic.mjs
rollback: 恢复 domain.rs dead-code 定义 + types.ts 旧变体名
stop_conditions:
  - domain.rs DbValue 有非 dead-code 消费者 → 阻塞
  - 序列化兼容性破坏 → 需迁移层
```

### 7.3 S1 候选实现卡：工作台外壳

```yaml
PROPOSED_NOT_AUTHORIZED
slice: S1
title: 工作台外壳 — 项目上下文 + 工具窗口 + 文档标签 + 旧入口映射
owner: A1（壳层）+ A3/A5/A8（签收）+ A9/A11（原生验证）
scope:
  - useLayoutStore 新增 bottomPanel/rightPanel/docTabs 状态 + 旧入口适配层
  - MainArea.vue 视图分发改为文档区+工具窗口编排
  - ActivityBar.vue 入口映射调整（工具窗口切换 vs 全屏视图）
  - UnifiedTabBar.vue 文档标签统一
  - StatusBar.vue 新增项目/分支/索引/连接状态
  - 首页调整为工作恢复入口
  - 新增 check-shell-layout-logic.mjs（壳层状态断言）
  - 更新 check-client-navigation-logic.mjs（导航入口变更）
prerequisites:
  - S0 闭环（DbValue 唯一源）
  - A1 壳层契约冻结（本报告 §4）
  - A3/A5/A8 签收契约
  - A9 原生 WebView 重定位验证方案
files:
  - src/App.vue
  - src/stores/useLayoutStore.ts
  - src/components/layout/ActivityBar.vue
  - src/components/layout/MainArea.vue
  - src/components/layout/UnifiedTabBar.vue
  - src/components/layout/StatusBar.vue
  - src/components/layout/Sidebar.vue
  - src/components/home/*（调整）
  - src/utils/homeUi.ts
  - scripts/check-shell-layout-logic.mjs（新增）
  - scripts/check-client-navigation-logic.mjs（更新）
tests:
  - node scripts/check-shell-layout-logic.mjs
  - node scripts/check-client-navigation-logic.mjs
  - node scripts/check-home-ui-logic.mjs
  - node scripts/check-home-store-logic.mjs
  - python3 scripts/check-command-set-consistency.py
  - bash scripts/pre-merge.sh
  - 原生 GUI 验收（A9/A11）
rollback: 恢复旧 mainView 分发模式 + 旧 ActivityBar 入口
stop_conditions:
  - 原生 WebView 重定位失败（子窗口位置/遮挡）→ 需 A9 方案
  - 旧入口有遗漏 → 补全覆盖矩阵
  - 布局跳动（工具栏控件因文字/加载态导致）→ 需 CSS 约束
```

---

## 8. 候选模型盘点

> 依据 `M5-W18-R2B-TASKS-20260908.md` §2 候选模型行 + `A0-HANDOFF-LOW-COST-20260908.md`。

| 模型 | 可用性 | 适合任务 | 证据 |
|---|---|---|---|
| 免费模型（Hy4/华为智能体） | 当前免费额度可用 | 壳层设计/线框/覆盖清单/测试卡 | `INFERENCE`：蓝图 §2 建议免费模型承担常规工作 |
| Kimi K3 | 按账户可用 | 契约复核（状态契约/迁移兼容） | `INFERENCE`：任务卡 §2 建议做一次契约复核 |
| GPT-5.5 | 按账户可用 | 契约复核（同上） | `INFERENCE`：任务卡 §2 备选 |
| GPT-5.4-mini | 按账户可用 | 按契约细化 | `INFERENCE`：任务卡 §3 建议 |

**本报告由免费模型完成**（壳层设计/线框/覆盖清单属常规工作）。契约复核可由 K3 或 GPT-5.5 做一次定点审查。

---

## 9. J1/J6 完整性验证

### J1 打开并接续项目

| 步骤 | 壳层支持 | 证据 |
|---|---|---|
| 打开工作区 | `useWorkspaceStore.enterDir` + 首页"最近项目" | `CURRENT_PRODUCT`: App.vue L84-87 |
| 找到文件 | 左工具窗口文件树 | §3.1 files → 左侧工具窗口 |
| 编辑/阅读 | 中央文档区 FileEditor | §3.1 editor → 中央文档区 |
| 底部终端工作 | 底部工具窗口 TerminalPane（单例挂载） | §3.1 term → 底部工具窗口 |
| 切到网页查资料 | 中央文档区网页标签 | §3.1 browser → 中央文档区 |
| 回到原文件 | 文档标签切换（UnifiedTabBar） | §4.3 文档标签保留状态 |
| 焦点/滚动/选区不丢 | 文档切换隐藏≠销毁 | §4.7 关闭视图 vs 关闭文档 |
| 重启恢复 | 布局持久化（§4.6）+ 文档恢复 | S1 卡含布局持久化 |

**J1 完整** ✅

### J6 正常退出与异常恢复

| 步骤 | 壳层支持 | 证据 |
|---|---|---|
| 未保存文档 → 关闭请求 | `useSessionStore.requestClose` | `CURRENT_PRODUCT`: App.vue L105 |
| 保存/放弃/取消 | `SessionCloseDialog` + `resolveClose` | `CURRENT_PRODUCT`: App.vue L249 |
| 协调终端/数据库/任务关闭 | `bridge.closeBrowser()` + `bridge.flushSessions()` | `CURRENT_PRODUCT`: App.vue L209-211 |
| 重开恢复 | 布局持久化 + 文档恢复 | §4.6 持久化 |
| 区分布局/文档/运行/凭据 | §4.7 焦点与资源生命周期区分 | `DESIGN_DECISION` |
| 失败注入/旧格式兼容/损坏备份/回滚 | S1 卡 rollback + stop_conditions | §7.3 |

**J6 完整** ✅

---

## 10. 未解问题

| # | 问题 | 归属 | 证据类型 |
|---|---|---|---|
| U1 | 原生 WebView 重定位/遮挡：布局变化改变子窗口位置/焦点/遮挡，需 A9/A11 实机验证 | A9/A11 | `INFERENCE`：蓝图 §3 "原生 WebView 坐标仍遵守现有 CSS 像素契约" |
| U2 | 编辑器升级（如 CodeMirror）的依赖/包体成本评估 | A5 | `INFERENCE`：任务卡 §6 "编辑器升级需对照现有依赖、功能收益和包体成本" |
| U3 | 布局持久化 schema 设计（schema_version/原子写/容量/淘汰） | A11 | `INFERENCE`：蓝图 §5 持久化契约 |
| U4 | 现有 `modTabs` → 新 `docTabs` 的迁移兼容（旧页签数据不丢） | A1（S1 实施时） | `DESIGN_DECISION`：§4.2 现状映射 |
| U5 | 体积预算：S1 壳层改动的 JS delta 是否 ≤25.2% | A11（复测） | `INFERENCE`：蓝图 §7 包体门槛 |

---

## 11. R2B 合规声明

- [x] Rebase onto `origin/master`（`434e63f`，2 个 R2 提交已 rebase）
- [x] 仅修改 `logs/research/M5-W18/A1-*` 文件 + checkpoint + 线框 HTML
- [x] 未修改产品代码、manifest、lockfile、脚本、capability、ACL、vault
- [x] 每个事实声明标注证据类型（`CURRENT_PRODUCT` / `EXECUTED_SYNTHETIC_TEST` / `INFERENCE` / `DESIGN_DECISION`）
- [x] 显式更正 R1/R2 的 3 处错误（§1.2 C1-C3）
- [x] 保留 R1/R2 报告作为草稿，未静默改写
- [x] 未 push
- [x] 旧入口全覆盖清单无遗漏（§3.2）
- [x] J1/J6 完整（§9）
- [x] 每条新状态有读写方/关闭语义（§4）
- [x] 原生 WebView 重定位和遮挡已交给 A9/A11 验证（§10 U1）
- [x] 候选实现卡标 `PROPOSED_NOT_AUTHORIZED`（§7.2, §7.3）

---

## 12. 结论

`STATUS=READY_FOR_REVIEW`

A1 R2B 已完成工作台壳层与迁移蓝图：
- §3 旧功能 → 新工具窗口全覆盖清单（19 MainView + 18 MODULE_META 全覆盖）
- §4 状态契约（工作区/文档/工具窗口/请求/导航/持久化/焦点与资源生命周期区分）
- §5 三尺寸线框（1440×900/1024×720/800×600）+ 焦点/键盘/IME 规格
- §6 共享壳层唯一修改者清单（A1 壳层 + A3/A5/A8 领域嵌入）
- §7 前端测试分类更正 + S0/S1 候选实现卡
- §9 J1/J6 完整性验证

**残留**（不阻塞 READY_FOR_REVIEW）：
- U1-U5 未解问题分派至 A5/A9/A11
- S0/S1 为候选卡，非编码授权（`PROPOSED_NOT_AUTHORIZED`）
- 原生 WebView 实机验证待 A9/A11
