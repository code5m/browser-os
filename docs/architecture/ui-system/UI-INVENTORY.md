# UI INVENTORY — Phase UI-0A

> 审计对象：`mvp-browser-os-v3`
> 审计 HEAD：`3f57a92e5fb7d9a6bc978f1572964d3c14fb8caf`
> 保护 tag：`universal-hotplug-capability-platform-v1-resource-fix-code-pass`（= 同一 commit，未移动）
> 本文件性质：**只审计，不重构**。所有分类均有真实代码证据（file:line）。

---

## 0. 统计总览

| 指标 | 数量 |
|---|---|
| `.vue` 组件 | **65** |
| `.css` 文件 | 1（`src/styles/global.css`，401 行） |
| UI composables | 5 |
| UI utils（纯展示/格式化/a11y） | 15 |
| **UI 相关模块合计** | **86** |
| 已注册 Capability（运行时） | 4（browser / workspace / terminal / bookmark） |
| 文档声明但未注册的域 | 14（git / database / agent / skill / plugin / task / knowledge_graph / notes / grid / credential / script / session / resource_collection / workbench） |

---

## 1. 分层分类结果（TARGET_LAYER，按主归属计数，合计 = 65）

| TARGET_LAYER | 数量 | 说明 |
|---|---:|---|
| TOKEN | **0** | **全仓零 CSS 自定义属性**（`:root{--*}` 不存在）。当前无 Design Token 层。 |
| SHARED_PRIMITIVE | **1** | `ImageLightbox.vue` |
| SHARED_PATTERN | **1** | `ImageGallery.vue` |
| WORKBENCH | **17** | Shell 外壳（layout / home / system） |
| CAPABILITY_UI | **46** | 19 个已在 `capabilities/*/ui/`，27 个语义属于能力但物理仍在 `src/components/**` |
| UNKNOWN | **0** | ✅ 目标达成 |

> **LEGACY_COUPLING 不是独立层**，是叠加标记：26 个组件携带至少一处遗留耦合（见 §5）。

---

## 2. WORKBENCH 层（17）

| # | COMPONENT_ID | FILE | 行数 | BUSINESS_SEMANTICS | STATE_DEPS | NATIVE | 备注 |
|---|---|---|---|---|---|---|---|
| 1 | `shell.root` | `src/App.vue` | 339 | **重**（见 §5） | 12 stores | 大量 bridge 事件 + `getCurrentWindow` | 外壳骨架 + 全局事件总线宿主 |
| 2 | `shell.activity-bar` | `components/layout/ActivityBar.vue` | 656 | 重（Browser 宫格/导航） | 8 stores | `bridge.resourceStats` 2s 轮询 | **成熟 UI，必须保留** |
| 3 | `shell.main-area` | `components/layout/MainArea.vue` | 287 | 中（view→面板路由残留硬编码） | layout, browser | 无 | 唯一 surface 贡献宿主 |
| 4 | `shell.tab-bar` | `components/layout/UnifiedTabBar.vue` | 308 | 中 | layout,browser,fs,system,term | `getCurrentWindow` 窗口控制 | 自绘标题栏 |
| 5 | `shell.status-bar` | `components/layout/StatusBar.vue` | 204 | 中（资源/终端/仓库计数） | 6 stores | `bridge.resourceStats` 3s | |
| 6 | `shell.commands` | `components/layout/WorkbenchCommands.vue` | 52 | 轻（通用导航） | workbench | 无 | Ctrl+K 命令面板 |
| 7 | `shell.rail` | `components/layout/WorkbenchRail.vue` | 10 | 中（硬编码 6 个工具入口） | workbench, layout | 无 | |
| 8 | `shell.top-bar` | `components/layout/TopBar.vue` | 24 | 轻（Browser） | browser, layout | 无 | **孤儿**（零 import，仅 global.css:156,174 残留样式） |
| 9 | `shell.sidebar-resizer` | `components/layout/SidebarResizer.vue` | 33 | 无 | layout, browser | 无 | **孤儿**（零 import） |
| 10 | `shell.home` | `components/home/HomePanel.vue` | 132 | 中（主页域） | home | 无 | |
| 11 | `shell.home-launchers` | `components/home/HomeLaunchers.vue` | 148 | 中-重（17 个业务入口硬编码） | layout,browser,system | 无 | **成熟 UI** |
| 12 | `shell.home-recents` | `components/home/HomeRecents.vue` | 247 | 中 | home | 无 | |
| 13 | `shell.home-shortcuts` | `components/home/HomeShortcuts.vue` | 304 | 中 | home | 无 | **成熟 UI** |
| 14 | `shell.home-shortcut-editor` | `components/home/HomeShortcutEditor.vue` | 216 | 中 | home | 无 | 复用 `utils/modalA11y` |
| 15 | `shell.app-panel` | `components/system/AppPanel.vue` | 40 | 中（系统应用） | system, layout | `convertFileSrc` | |
| 16 | `shell.clipboard-panel` | `components/system/ClipboardPanel.vue` | 42 | 中 | system, layout | 无（经 store） | |
| 17 | `shell.settings` | `components/system/SettingsPanel.vue` | 234 | 中 | settings, layout | `getDefaultBrowser` / `setDefaultBrowser` | |

---

## 3. CAPABILITY_UI 层（46）

### 3.1 已在 `src/capabilities/*/ui/`（19）— 位置正确

| # | COMPONENT_ID | FILE | 行数 | 归属 | 渲染宿主 |
|---|---|---|---|---|---|
| 1 | `bookmark.entry-button` | `bookmark/ui/BookmarkEntryButton.vue` | 45 | bookmark | `ActivityBar.vue:330-332`（slot `activity-bar-trailing`） |
| 2 | `bookmark.panel` | `bookmark/ui/BookmarkPanel.vue` | 289 | bookmark | `MainArea.vue:152-154`（slot `browser-sidebar`） |
| 3 | `bookmark.star` | `bookmark/ui/BookmarkStar.vue` | 91 | bookmark | `ActivityBar.vue:321-323`（slot `address-bar-actions`） |
| 4 | `browser.host` | `browser/ui/BrowserHost.vue` | 22 | browser | `MainArea.vue:159`（slot `browser-host`） |
| 5 | `browser.resource-waterfall` | `browser/ui/ResourceWaterfall.vue` | 247 | browser | `MainArea.vue:172`（dock `net`） |
| 6 | `browser.session-panel` | `browser/ui/SessionPanel.vue` | 366 | browser | `MainArea.vue:174`（dock `session`） |
| 7 | `terminal.dock-panel` | `terminal/ui/TerminalDockPanel.vue` | 60 | terminal | `MainArea.vue:177`（dock `term`） |
| 8 | `terminal.pane` | `terminal/ui/TerminalPane.vue` | 326 | terminal | `TerminalView.vue:56` / `TerminalDockPanel.vue:14` |
| 9 | `terminal.view` | `terminal/ui/TerminalView.vue` | 155 | terminal | `MainArea.vue:133`（resident slot）**PTY 出生点** |
| 10 | `workspace.artifact` | `workspace/ui/ArtifactPanel.vue` | 57 | workspace | `MainArea.vue:184` |
| 11 | `workspace.audit` | `workspace/ui/AuditPanel.vue` | 26 | workspace | `MainArea.vue:184` |
| 12 | `workspace.commands` | `workspace/ui/CommandSnippetPanel.vue` | 189 | workspace | `MainArea.vue:184` |
| 13 | `workspace.file-editor` | `workspace/ui/FileEditor.vue` | 39 | workspace | `MainArea.vue:239` |
| 14 | `workspace.file-ide` | `workspace/ui/FileIdeView.vue` | 10 | workspace | `MainArea.vue:184` |
| 15 | `workspace.file-panel` | `workspace/ui/FilePanel.vue` | 447 | workspace | `MainArea.vue:170`（dock `files`）+ `FileIdeView.vue:8` |
| 16 | `workspace.file-tree-node` | `workspace/ui/FileTreeNode.vue` | 125 | workspace | `FilePanel.vue:85`（递归） |
| 17 | `workspace.repo` | `workspace/ui/RepoPanel.vue` | 53 | workspace | `MainArea.vue:184` |
| 18 | `workspace.script` | `workspace/ui/ScriptPanel.vue` | 174 | workspace | `MainArea.vue:184` |
| 19 | `workspace.script-history` | `workspace/ui/ScriptRunHistory.vue` | 156 | workspace | `ScriptPanel.vue:98` |

### 3.2 语义属于能力、物理仍在 `src/components/**`（27）— 待迁移

**BROWSER（4）**
| COMPONENT_ID | FILE | 行数 | 现状 |
|---|---|---|---|
| `browser.ai-nav` | `components/browser/AINavPanel.vue` | 28 | 被 **App.vue:263 直接渲染**；未注册为 contribution |
| `browser.credential-list` | `components/browser/CredentialList.vue` | 269 | 被 **bookmark 能力**渲染（`BookmarkPanel.vue:141`）→ 跨能力错配 |
| `browser.grid-archive-bar` | `components/browser/GridArchiveBar.vue` | 14 | 被 **ActivityBar.vue:373 直接渲染** |
| `browser.resource-panel` | `components/browser/ResourcePanel.vue` | 50 | **孤儿**（已被 `ResourceWaterfall.vue` 取代） |

**GIT（4，域未注册）** — `GitPanel.vue` 153 / `GitHistory.vue` 37 / `GitDiffViewer.vue` 53 / `GitWriteConfirmDialog.vue` 87
> `GitWriteConfirmDialog` 被 **App.vue:269 全局挂载**（耦合最深）。`GitPanel`/`GitHistory` 被 workspace 的 `RepoPanel.vue:22-23` 消费 → GIT UI 寄生在 workspace 能力内。

**DATABASE（1，未注册）** — `DatabasePanel.vue` 258（被 `MainArea.vue:204` 渲染；反向写 `layout.sidebarOpen`/`showToast`）

**TASK（2，未注册）** — `TaskPanel.vue` 327 / `TaskEditDialog.vue` 260

**AGENT（2，未注册）** — `AgentManagerPanel.vue` 90 / `AgentChatPanel.vue` 44

**SKILL（1，未注册）** — `SkillManagerPanel.vue` 97（复用 `useAgentStore`，无独立 store）

**VAULT/NOTES（1，未注册）** — `VaultPanel.vue` 56（MainArea 唯一**同步静态 import** 的业务面板）

**WORKSPACE 子域（3，域已注册，文件住错地方）** — `ToolBox.vue` 149（`tools` 未列入 workspace provides）/ `ScriptRunDialog.vue` 230 / `ScriptParamForm.vue` 94
> 后两者已被 `ScriptPanel` + `CommandSnippetPanel` 正确消费（2 个消费者），仅位置错误。

**AGENT/SKILL 共享纯展示（2）** — `PermissionPreviewModal.vue` 62（纯 props/emit）/ `RunHistoryModal.vue` 52（纯 props/emit，**孤儿**）

**KNOWLEDGE_GRAPH（5，未注册）** — `GraphPanel.vue` 119 / `GraphViewer.vue` 104 / `GraphFilter.vue` 48 / `NodeDetail.vue` 33 / `EdgeDetail.vue` 33

**PLUGIN（1，未注册）** — `PluginManager.vue` 424（`usePluginStore` 含 17 处 bridge，全组最高）

### 3.3 误放在 `shared/` 的能力组件（1）

| COMPONENT_ID | FILE | 行数 | 判定 |
|---|---|---|---|
| `git.sync-confirm` | `components/shared/ConfirmModal.vue` | 27 | **名为 shared，实为 Git 专属**：`L3 import useRepoStore from capabilities/workspace/public`，UI 硬编码仓库名/远程地址/成果清单（`L14-19`）。→ **CAPABILITY_UI（GIT），应迁出 shared/** |

---

## 4. SHARED 层（2）

| COMPONENT_ID | FILE | 行数 | 业务语义 | 判定 |
|---|---|---|---|---|
| `shared.image-lightbox` | `components/shared/ImageLightbox.vue` | 329 | **无**（纯看图：缩放/切换/拖拽/键盘） | SHARED_PRIMITIVE 候选。问题：全局单例直接读 `useImagePreviewStore`，无 props 接口 → 提炼前需 props 化 |
| `shared.image-gallery` | `components/shared/ImageGallery.vue` | 240 | **弱**（依赖领域类型 `ImageRef`） | SHARED_PATTERN 候选。已有 `defineProps<{images, loading?}>`，被 `ArtifactPanel.vue:50` 消费 |

---

## 5. 遗留耦合清单（LEGACY_COUPLING = 26 个组件）

### 5.1 Shell 直接渲染业务面板（11 处）— 核心债务

| # | Shell 文件:行 | 业务组件 | 域 | 注册? |
|---|---|---|---|---|
| 1 | `App.vue:269` | `GitWriteConfirmDialog` | GIT | 否 |
| 2 | `App.vue:263` | `AINavPanel` | BROWSER | 域是，UI 未注册 |
| 3 | `MainArea.vue:207` | `VaultPanel` | VAULT | 否 |
| 4 | `MainArea.vue:199` | `ToolBox` | WORKSPACE(tools) | 子域未声明 |
| 5 | `MainArea.vue:204` | `DatabasePanel` | DATABASE | 否 |
| 6 | `MainArea.vue:210` | `TaskPanel` | TASK | 否 |
| 7 | `MainArea.vue:218` | `SkillManagerPanel` | SKILL | 否 |
| 8 | `MainArea.vue:221` | `AgentManagerPanel` | AGENT | 否 |
| 9 | `MainArea.vue:224` | `GraphPanel` | GRAPH | 否 |
| 10 | `MainArea.vue:215` | `PluginManager` | PLUGIN | 否 |
| 11 | `ActivityBar.vue:373` | `GridArchiveBar` | BROWSER(grid) | 否（挂 browser 下） |

> **关键对照**：`MainArea.vue:83-110` **已经**通过 `contributionRegistry` 解耦渲染（注释明写「Shell 不持有 Workspace/Bookmark 专属知识，C3 关键」）。上表 11 个面板全部**绕过**该机制，走 `layout.mainView === 'xxx'` 硬编码分支（`MainArea.vue:188/193/198/203/207/209/214/217/220/223/228`）。
> → 通用机制已存在，迁移路径是**接入已有 contribution**，不是新建抽象。

### 5.2 App.vue 直接持有能力 store（7 处）
`App.vue:5,8,9,10,11,14`（browser/workspace/artifact/repo/file/terminal 的 public store）+ `App.vue:12 useGitStore`（legacy GIT 业务 store）。

### 5.3 能力反向依赖 legacy shell 组件（8 处，5 个文件）
| 能力文件 | 引入的 legacy 组件 |
|---|---|
| `bookmark/ui/BookmarkPanel.vue:8` | `components/browser/CredentialList.vue` |
| `workspace/ui/ScriptPanel.vue:7,8` | `ScriptParamForm.vue` / `ScriptRunDialog.vue` |
| `workspace/ui/CommandSnippetPanel.vue:13,14` | `ScriptParamForm.vue` / `ScriptRunDialog.vue` |
| `workspace/ui/RepoPanel.vue:5,6` | `GitPanel.vue` / `GitHistory.vue` |
| `workspace/ui/ArtifactPanel.vue:3` | `shared/ImageGallery.vue` |

→ 这 5 个能力组件**无法脱离 `src/components/**` 独立存在**。

### 5.4 跨能力 import（3 处，2 合规 / 1 瑕疵）
| 文件:行 | 方向 | 判定 |
|---|---|---|
| `bookmark/ui/BookmarkStar.vue:3` | bookmark → browser `public` | ✅ 合规（manifest 声明 `optionalDependencies: ["browser"]`） |
| `bookmark/ui/BookmarkPanel.vue:4` | bookmark → browser `public` | ✅ 合规 |
| `workspace/ui/FileEditor.vue:4` | workspace → browser `public` | ⚠️ **workspace manifest 未声明 browser 依赖**（CB-03 语义瑕疵），全仓唯一真实边界瑕疵 |

### 5.5 业务面板反向写 Shell store
`DatabasePanel.vue:81`(`layout.sidebarOpen`)、`VaultPanel.vue:42`(`workbench.collapsed`)、`CredentialList.vue:70/74/83/85`(`layout.showToast`)、`ResourcePanel.vue:26`、`AuditPanel.vue:12`
→ 即使面板下沉到能力，仍需 Shell 提供 **UI 服务接口**（toast / layout）解耦。

---

## 6. 孤儿组件（3，零 import）

| FILE | 行数 | 说明 |
|---|---|---|
| `components/layout/TopBar.vue` | 24 | 已被 UnifiedTabBar + ActivityBar 取代 |
| `components/layout/SidebarResizer.vue` | 33 | 仅支持鼠标拖拽，无键盘路径 |
| `components/browser/ResourcePanel.vue` | 50 | 已被 `ResourceWaterfall.vue` 取代 |
| `components/workspace/RunHistoryModal.vue` | 52 | 仅被 `scripts/check-ui.mjs:41` 选择器校验，从未挂载 |

> 孤儿**不代表删除**。按 §1「UI PRESERVATION」原则：先确认无回归价值，再单独批次处理；本阶段仅登记。

---

## 7. 审计中发现的事实性问题（仅登记，未修改）

| # | 问题 | 位置 | 严重度 |
|---|---|---|---|
| 1 | **`bridge` 未 import 却在 3 处使用** → 右键「在文件管理器中显示/复制路径」抛 `ReferenceError` | `UnifiedTabBar.vue:127,137,145` | 高（真实 bug） |
| 2 | `ConfirmModal.vue` 位于 `shared/` 却硬耦合 Git 域 | `shared/ConfirmModal.vue:3,14-19` | 中（架构误导） |
| 3 | `FileEditor.vue` 依赖 browser 但未声明 | `workspace/ui/FileEditor.vue:4` + `workspace/manifest.ts:49-50` | 中（边界瑕疵） |
| 4 | CredentialList（browser 域）被 bookmark 能力渲染 | `BookmarkPanel.vue:8,141` | 中（跨能力错配） |
| 5 | GitPanel 同时依赖 capability store + legacy store（半迁移） | `GitPanel.vue:3,4` | 低 |
| 6 | `ScriptRunHistory.vue:2` 注释路径已失效（物理已迁移） | `workspace/ui/ScriptRunHistory.vue:2` | 低（文档漂移） |

---

## 8. 语义治理锚点（物理移动会打破的点）

| 锚定对象 | 声明位置 | 移动时必须同步 |
|---|---|---|
| `capabilities/workspace/ui/FilePanel.vue` | `docs/architecture/semantic-registry/states.yaml:25-27`（`governed_files`） | ✅ 必须同步 `governed_files` |
| 所有 state store（useFileStore/useBookmarkStore/useTerminalStore…） | `states.yaml:33-90`（`owner_implementations`，多候选） | ✅ 加到候选首项，否则 `SEMANTIC_IMPLEMENTATION_UNRESOLVED` |
| `terminal/ui/TerminalView.vue`、`TerminalDockPanel.vue` | `scripts/check-terminal-owners.mjs:31,146`（PTY 出生点白名单） | ✅ 必须同步 |
| `bookmark/ui/BookmarkPanel.vue` | `scripts/check-semantic-closure-logic.mjs:121` | ✅ 必须同步 |
| 其余 14 个 capability UI | 仅经 `capabilities/*/index.ts` 动态 import | 只需改 `index.ts` 相对路径 |

> `canonical_writer` 均为 `owner.method` 形式，**不含文件路径** → 不直接锚定 UI 文件。
> `SEMANTIC_LOCATORS_VALID = YES`（当前全部可解析）。

---

## 9. 结论

- **UNKNOWN = 0** ✅
- 真正通用、无业务语义的组件**极少**（仅 `ImageLightbox` 1 个）→ 印证「不要预设 shared/ui 有大量现成组件」。
- 最大债务不是「缺少组件」，而是 **27 个业务 UI 住在 Shell 目录 + 11 处 Shell 直接渲染业务面板**。
- 通用 contribution 机制**已存在且被验证有效**（workspace/bookmark/browser/terminal 16 条注册全部生效）→ 迁移应**复用已有机制**，不新造抽象。
