# Semantic Coverage Matrix（Phase · Semantic Governance Coverage Remediation）

> 机器真源：`docs/architecture/semantic-registry/{states,owners,intents,side-effects}.yaml` + 各 `capabilities/<id>/manifest.ts` 的 `semanticOwner`。
> 本矩阵是**现状快照**，非第二真源。评级定义见各 YAML / README。
>
> STATUS 取值（官方）：
> FULLY_GOVERNED · PARTIALLY_GOVERNED · OWNER_DECLARED_ONLY · REGISTRY_DECLARED_ONLY · OWNER_PENDING_SCR · UNGOVERNED · NOT_APPLICABLE · UNKNOWN
>
> 原则：UNKNOWN=0 为目标；但 FULLY_GOVERNED 不人为提高。OWNER_DECLARED_ONLY 表示 manifest 已声明 semanticOwner，但语义 registry 尚未治理其 state/intent/side-effect（需 SCR 才能纳入，或判定 NOT_APPLICABLE）。

## A. 14 个 D3 文档化模块（逐行）

| MODULE_ID | DOMAIN | DECLARED_SEMANTIC_OWNER | OWNER_REGISTERED | STATES_REGISTERED | INTENTS_REGISTERED | WRITERS_REGISTERED | SIDE_EFFECTS_REGISTERED | IMPL_LOCATORS_VALID | CHECKER_COVERAGE | PUBLIC_CONTRACT_ALIGNED | STATUS |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bookmark | 收藏夹 | useBookmarkStore | yes | yes | yes | yes | yes | yes | yes(R1-R5) | yes | **FULLY_GOVERNED** |
| browser | 浏览器/宫格 | useBrowserStore | yes | yes | yes | yes | yes | yes | yes | yes | **FULLY_GOVERNED** |
| terminal | 终端 | useTerminalStore | yes | yes | yes | yes | yes | yes | yes | yes | **FULLY_GOVERNED** |
| vault | 笔记库 | useVaultStore | yes | yes | no | yes | no | yes | partial(R2) | yes | **PARTIALLY_GOVERNED** |
| clipboard | 剪贴板 | useClipboardStore | partial(locator only) | no | no | no | no | yes | info-only | yes | **OWNER_DECLARED_ONLY** |
| apps | 应用 | useAppsStore | partial(locator only) | no | no | no | no | yes | info-only | yes | **OWNER_DECLARED_ONLY** |
| tools | 工具箱 | useToolsStore | partial(locator only) | no | no | no | no | yes | info-only | yes | **OWNER_DECLARED_ONLY** |
| task | 定时任务 | useTaskStore | yes | yes | yes | yes | yes | yes | yes(R1-R5) | yes | **FULLY_GOVERNED** |
| graph | 图谱 | (Rust GraphStore，无前端 store) | no | no | no | no | no | n/a | no | yes | **OWNER_DECLARED_ONLY** |
| agent | 智能体 | useAgentStore (src/stores/) | no | no | no | no | no | n/a | no | yes | **OWNER_DECLARED_ONLY** |
| database | 数据库 | useDatabaseStore (src/stores/) | no | no | no | no | no | n/a | no | yes | **OWNER_DECLARED_ONLY** |
| home | 主页 | useHomeStore | no | no | no | no | no | n/a | no | yes | **OWNER_DECLARED_ONLY** |
| plugin | 插件 | (manifest semanticOwner) | no | no | no | no | no | n/a | no | yes | **OWNER_DECLARED_ONLY** |
| settings | 框架偏好(SERVICE) | useSettingsStore (src/stores/) | no | no | no | no | no | n/a | no | yes | **OWNER_DECLARED_ONLY** (framework) |

> 注：mainView / gridToolbarOpen / sidebarOpen / clipOpen / fileEditorOpen / browserDockOpen / browserDockTab 属 `view_navigation` 域（owner=useLayoutStore），不在 14 模块之列，但已在 registry 治理（见 §B）。

## B. registry 已治理、但不在 14 文档化模块中的域（供完整图景）

| MODAIN | OWNER | STATES | INTENTS | WRITERS | SIDE_EFFECTS | STATUS |
|---|---|---|---|---|---|---|
| view_navigation | useLayoutStore | mainView, gridToolbarOpen, sidebarOpen, clipOpen, fileEditorOpen, browserDockOpen, browserDockTab | activateView/activateBrowser/activateHome | yes | n/a(纯导航) | **FULLY_GOVERNED** |
| files (Workspace/FilePanel) | useFileStore | filePath, inlineFile, previewDir, pathInput, currentLocalPath(der) | openFile/openFileInline/enterDir/saveFile/closeFileEditor | yes | writeFile | **FULLY_GOVERNED** |
| artifact | useArtifactStore | tree,current,editTitle,editTags,editText,selected,flatArtifacts(der),ctxMenu | openArtifact/saveEdit/removeArtifact/toggle/collectSelection | yes | n/a | **FULLY_GOVERNED** |
| repo | useRepoStore | repos,form,preview,busy,job | saveRepo/requestSync/confirmSync/clearPreview/loadGiteeExample | yes | n/a | **FULLY_GOVERNED** |
| script | useScriptStore | scripts,scriptForm | loadScripts/saveScript/removeScript | yes | n/a | **FULLY_GOVERNED** |
| snippet | useSnippetStore | snippets,snippetForm | loadSnippets/saveSnippet/removeSnippet | yes | n/a | **FULLY_GOVERNED** |
| credential | KeyringStore(Rust) | gitRepoToken,dbCredential,browserCredential,databaseCredentialInput | saveCredential/getCredential/deleteCredential/fillBrowserCredential/listBrowserCredentials | yes | keyringWrite/keyringDelete/keyringRead/sensitiveInput | **FULLY_GOVERNED** |

## C. 覆盖率汇总（14 模块）

- FULLY_GOVERNED: 4 (bookmark, browser, terminal, task)
- PARTIALLY_GOVERNED: 1 (vault)
- OWNER_DECLARED_ONLY: 9 (clipboard, apps, tools, graph, agent, database, home, plugin, settings)
- REGISTRY_DECLARED_ONLY: 0
- OWNER_PENDING_SCR: 0
- UNGOVERNED: 0
- NOT_APPLICABLE: 0（settings 仍判 OWNER_DECLARED_ONLY，因框架偏好 owner 已声明；如需改 NOT_APPLICABLE 须单独 ADR）
- UNKNOWN: 0

## D. Registry Drift 验证（Part D 分类）

| 候选漂移 | 分类 | 处置 |
|---|---|---|
| `02-STATE-SOURCES.md` 引 `useSystemStore`(终端旧主)/`useGridArchiveStore`(grid)/`useWorkbenchStore`(aiNavOpen) | STALE_DOCUMENTATION | 该文件是 Agent A 只读审计快照（页首标明 READ-ONLY），活动真源是 states/owners YAML；建议加页眉“已被 semantic-registry/* 取代”，不改写审计结论 |
| 02-STATE-SOURCES 引 `activeTermId` owner=useSystemStore | STALE_DOCUMENTATION | 终端 owner 已迁 useTerminalStore（Phase 8E/Train D，states.yaml 已更新）；registry 为准 |
| home/apps/settings/vault/agent/database 不在 states.yaml | MISSING_REGISTRY_ENTRY（非 bug） | 这些是 OWNER_DECLARED_ONLY；须走 SCR 纳入或判 NOT_APPLICABLE，禁止静默 |
| `task` 的 `useTaskStore` locator / states / owners / intents / side-effects 已全部登记（SG-4） | RESOLVED | task 已升格为 FULLY_GOVERNED（checker self-test ALL_PASS + real scan fail=0）；原 MISSING_OWNER_LOCATOR 已消除 |
| `clipboard`/`apps`/`tools` 在 `owner_implementations` 但不在 `owners.yaml` 的 `owners:` | PARTIAL_REGISTRY（locator 有、owner 条目缺） | 与 task 同类；SCR 纳入时一并补 owners 条目 |
| `useGridArchiveStore`(gridOpen/gridSession 读者) 仍在 src/stores | STALE_LOCATOR（物理债务，非语义缺陷） | 属 browser 物理债务（D3 README §8 已登记）；影响的是 state 物理位置，非 registry 真值 |
| checker 解析 `src/stores/useLayoutStore.ts` 与 `src/capabilities/workspace/state/useScriptStore.ts` 无 RI-UNRESOLVED | VALID_NOT_APPLICABLE | locator 首候选存在即命中，无漂移失败 |

## E. Browser/Grid Frozen Semantics（Part E 确认）

以下冻结语义**未被本阶段改变**（本阶段只新增 D3 文档 + 本矩阵，未动任何 registry 业务条目/代码）：

- mainView owner = useLayoutStore ✔（states.yaml `mainView.owner: useLayoutStore`）
- gridOpen owner = useBrowserStore ✔（`gridOpen.owner: useBrowserStore`）
- desiredGridVisibility = gridOpen && mainView === "grid" ✔（derived，formula 锁定）
- isBrowserVisible = mainView === "browser" ✔（derived）
- Grid → Browser/Home/Workspace = HIDE ONLY ✔（side-effects: hideWebview/hideAllWebviews = move offscreen，资源存活）
- explicit closeGrid = DESTROY ✔（side-effects: closeGrid = destroy native webviews；intents: closeGrid forbidden_effects 含 navigate）
- mainView === "grid" ⇒ gridOpen === true ✔（`activateGrid` allowed_effects 含 ensure grid exists + mainView=grid；closeGrid 派生收敛由守卫，非直写）
- MULTIPLE_WRITERS 观察：gridOpen canonical_writer=buildGrid/closeGridAll；forbidden_writers 含 useLayoutStore/components —— 属治理内约束，未改动。

**BROWSER_GRID_FROZEN_SEMANTICS: UNCHANGED**

## F. 下一阶段（SG-1..SG-5）待办（未在本轮伪造）

对每个 OWNER_DECLARED_ONLY 模块，须按 Part F 裁决：证明 domain state / STORED vs DERIVED / canonical owner / writers / public intent / side effect / native side effect / implementation locator 真实存在，再 SCR → Registry → Checker。
候选进入治理（需 SCR）：task, graph(Rust GraphStore), agent, database, home, plugin, apps, tools, clipboard, settings(判定 framework NOT_APPLICABLE 或纳入)。
本轮**仅文档化现状**，未改 registry YAML、未伪造 semantic objects。
