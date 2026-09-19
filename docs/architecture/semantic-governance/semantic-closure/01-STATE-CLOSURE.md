# 01 — State Closure Audit

> 对照 `states.yaml` 与代码中的 store 状态。Decision 取值：
> GOVERNED / OBSERVED / DUPLICATE / MIGRATION_REQUIRED / DEPRECATED / IMPLEMENTATION_DETAIL

## 1. 已闭合（GOVERNED，范围内）

| ID | Concept | Current Expressions | Owner | Registry | Decision |
|---|---|---|---|---|---|
| S-01 | 当前主表面 | `mainView` (useLayoutStore:138) | useLayoutStore | states.mainView (ACCEPTED_ADR) | GOVERNED |
| S-02 | 宫格资源存在 | `gridOpen` (useBrowserStore:20) | useBrowserStore | states.gridOpen (ACCEPTED_ADR) | GOVERNED |
| S-03 | 激活页签 | `activeTabId` (useBrowserStore:19) | useBrowserStore | states.activeTabId | GOVERNED |
| S-04 | Grid 可见派生 | `desiredGridVisibility` / `isBrowserVisible` | useBrowserStore | states (ACCEPTED_ADR, derived) | GOVERNED |
| S-05 | 文件地址真源 | `filePath`/`inlineFile`/`previewDir`/`pathInput` (useWorkspaceStore) | useWorkspaceStore | states (Phase 2) | GOVERNED |
| S-06 | 派生本地位置 | `currentLocalPath` (useWorkspaceStore:579) | useWorkspaceStore | states (derived) | GOVERNED |
| S-07 | 收藏夹列表 | `items`/`loaded`/`busy`/`error`/`panelOpen`/`sorted` (useBookmarkStore) | useBookmarkStore | states (Phase 3) | GOVERNED |
| S-08 | 终端面板注册表 | `termPanes`/`terminalOpen`/`termGrid`/`termGridCount`/`activeTermId`/`autoConfirmCli`/`termProbeOn`/`m0Cfg`/`m0StartTs`/`droppedChunks`/`droppedBytes` (useSystemStore) | useSystemStore | states (Phase 4, 11 项) | GOVERNED |
| S-09 | 凭据真源 | `gitRepoToken`/`dbCredential`/`browserCredential` (keyring) | credential | states (Phase 5) | GOVERNED |
| S-10 | DB 敏感输入 | `databaseCredentialInput` (password 瞬时表单) | credential | states (Phase 5.1-B, sensitive) | GOVERNED |

## 2. 范围内 — 真实重复语义（DUPLICATE / MIGRATION_REQUIRED）

| ID | Concept | Current Expressions | Owner | Registry | Decision |
|---|---|---|---|---|---|
| S-11 | AI 导航面板开关 | `aiNavOpen` **同时** 在 useBrowserStore:43 与 useLayoutStore:146 | 不明（双声明） | 均未登记 | **DUPLICATE → MIGRATION_REQUIRED** |
| S-12 | 面板显隐布尔族 | `panelOpen`(bookmark) / `gridToolbarOpen`(layout) / `clipOpen` / `fileEditorOpen` / `browserDockOpen` / `browserDockTab`(layout) | 分散 | 部分 GOVERNED / 部分 OBSERVED | **MIGRATION_REQUIRED（候选）**：建议统一 panel 注册表，但每个确为独立 UI 表面，低优先级 |

> S-11 是本次审计在范围内发现的**唯一硬重复**（同名状态两个真源）。需代码核对哪一个为 canonical；
> 若二者表达同一概念（浏览器 AI 导航开合），应合并到单一 owner（倾向 useLayoutStore，因导航面板归 layout）。

## 3. 范围内 — 同名但不同概念（非重复，澄清即可）

| ID | Concept | Current Expressions | 是否重复 | Decision |
|---|---|---|---|---|
| S-13 | 宫格数（浏览器 vs 终端） | `gridCount`(useBrowserStore:24) vs `termGridCount`(useSystemStore:127) | 否（浏览器 quad-grid 与终端 grid 是不同生命周期） | GOVERNED（各自 owner） |
| S-14 | 最近列表 | `recentlyClosed`(browser:47) / `recents`(home:46) / `recents`(workspace:220) | 否（关闭页签 / 主页快捷 / 工作区历史，生命周期不同） | OBSERVED（各自 store） |
| S-15 | 选中集 | `selected`(workspace:50) / `selected`(git:84) / `selected`(vault:10) / `selectedSkillId`+`selectedAgentId` | 否（不同域各自选中） | IMPLEMENTATION_DETAIL |
| S-16 | 后端就绪门 | `backendReady`(agent:48)/`useGraphStore:60`/`useTaskStore:47` | 否（各域独立 readiness，镜像各自 AVAILABLE const） | IMPLEMENTATION_DETAIL |
| S-17 | 预览对象 | `preview`(workspace SyncPreview:79) / `preview`(git GitWritePreview:90) / `useImagePreviewStore` | 否（同步预览 / git 写入预览 / 图片预览三态） | IMPLEMENTATION_DETAIL（图片预览是独立 store） |
| S-18 | 宫格 webview 镜像 | `gridRects`/`gridUrls`/`gridLayout`/`gridMode`/`gridSession`(useBrowserStore) | 否（Rust webview 生命周期的前端镜像，非独立语义） | OBSERVED；`gridSession` 边界权属待澄清（前端 epoch vs Rust 生命周期） |

## 4. 范围外 — 新增业务域（无 Registry，标注 gap，不擅自治理）

| ID | 域 | 代表状态（store） | Owner | Registry | Decision |
|---|---|---|---|---|---|
| S-19 | Git | `useGitStore`：status/branches/diff/repoId/commitMessage/... | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |
| S-20 | Agent/Skill | `useAgentStore`：skills/agents/sessions/pendingConfirms/... | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |
| S-21 | Task/Scheduler | `useTaskStore`：tasks/runs/targets/draft/... | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |
| S-22 | Database | `useDatabaseStore`：connections/configs/schema/risk/verdict/... | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |
| S-23 | Graph | `useGraphStore`：nodes/edges/selectedNodeId/... | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |
| S-24 | Session | `useSessionStore`：sessions/detail/policy | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |
| S-25 | Vault/Knowledge | `useVaultStore`：path/notes/query/results/backlinks | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |
| S-26 | Image/Resource/Plugin/Settings/Home/GridArchive/Workbench | 各自 store | 无 | 无 | OUT_OF_SCOPE → 需 SCR 治理 |

> 这些域是真实业务语义，但**故意不在当前 registry_scope 内**（Phase 1–5.1 之后才引入）。
> 本审计不把它们塞入 Registry；是否扩展治理见 08-MIGRATION-PLAN.md（属大型迁移，需人工决策）。

## 5. 小模型风险（Small-model Risk）

| DUP-ID | Concept | WHY_SMALL_MODEL_MAY_MISUNDERSTAND | POSSIBLE_WRONG_CHANGE | PREVENTION |
|---|---|---|---|---|
| DUP-001 | Browser 可见性 | `mainView`/`gridOpen`/`isBrowserVisible`/`browserDockOpen` 都像"是否可见" | 新增第 N 个 visible 标志，或把 dock 当作独立 view | Registry + R3/R6 护栏；可见性必须是派生量 |
| DUP-002 | AI 导航开合 | `aiNavOpen` 两处同名，模型可能当作同一状态分别改 | 在错误 store 写，造成双真源漂移 | 见 S-11，合并到单一 owner |
| DUP-003 | 面板开关 | `clipOpen`/`fileEditorOpen`/`panelOpen` 同名模式 | 把新面板直接加 `xxxOpen` 而不登记 owner | 统一 panel 注册表建议（S-12） |
| DUP-004 | 选中集 | 多个 `selected` 同名 | 复用别域的 selected 作为本域真源 | 各域独立 selection，勿共享 |
