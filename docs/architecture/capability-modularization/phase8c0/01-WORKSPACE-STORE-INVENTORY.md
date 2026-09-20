# 01 — Workspace Store Inventory（Phase 8C-0 审计）

> 目标：完整分类 `src/stores/useWorkspaceStore.ts`（≈1150 行 / ≈70 个成员）每一条语义，
> 判定它真实属于哪个域，作为后续 owner 抽取的唯一事实源。
> 生成：2026-09-20（独立 Agent 在 8B.1 验收后切入 8C-0）。
> 判定规则见 `02-DOMAIN-BOUNDARIES.md`。

## 关键发现（与接管指令候选域的偏差）

接管指令 §0 假定 monolith 混合 `files / vault / repo / git / editor / workspace`。
**实测代码偏差**：本 store 实际还持有 **Knowledge(笔记 Vault) / Script 库 / Snippet 库 / Repo-Sync / Audit**，
且 `git` 不在本 store（GitPanel 走 bridge git 命令，无 store 态）。因此：

- `vault` ≈ 笔记 Vault ≈ 本 store 的 `tree/current/edit*/collectSelection`（Knowledge 域）
- `editor` 不是独立域，是 Files 的 overlay/inline 编辑表面（与 Files 同 owner）
- `git` 不在此 store（不归 8C-0 抽取）
- 额外发现：`scripts/snippets`（脚本库）/ `audit`（审计）/ `repos+form+preview+busy+job`（仓库同步）也混在此 store

`UNKNOWN` 必须 = 0；下列已全部分类（含"其他域"的知识/脚本/审计）。

---

## 决策枚举

```
WORKSPACE_CORE  仅 Workspace 自身语义 + 跨域编排（refresh / recents / selected 编排）
FILES           文件系统浏览/编辑/树/目录预览/行内编辑/右键菜单/拖拽移动
EDITOR          Files 的编辑表面（overlay filePath/fileContent + inline），非独立 owner → 并入 FILES
REPO           Repository 配置 + 同步（repos/form/preview/busy/job/saveRepo/requestSync/confirmSync）
ARTIFACT       Knowledge/笔记 Vault（tree/current/edit*/selected/flatArtifacts/collectSelection/ctxMenu*）
SCRIPT         脚本库 CRUD（scripts/scriptForm/saveScript...）
SNIPPET        命令片段库 CRUD（snippets/snippetForm/saveSnippet...）
AUDIT          审计日志（audit）
CROSS_DOMAIN   recents（url+file 混合，跨域，暂留 Workspace Core）
IMPLEMENTATION_DETAIL 预览队列/缓存等内部实现变量
UNKNOWN        无
```

---

## 主表（按 store 成员顺序）

| # | Symbol | Type | Meaning | 主要 Writers（store 内） | SideEffect | Persistence | ProposedDomain | Decision |
|---|--------|------|---------|------------------------|-----------|-------------|---------------|----------|
| 1 | `tree` | ref<WorkspaceTree> | 知识库/笔记 Vault 树（browseWorkspace） | `refresh` | 读后端 | 后端持久化镜像 | ARTIFACT | ARTIFACT |
| 2 | `current` | ref<Artifact\|null> | 当前编辑的知识条目 | `openArtifact`,`removeArtifact` | — | — | ARTIFACT | ARTIFACT |
| 3 | `editTitle` | ref<string> | 知识条目编辑缓冲 | `openArtifact`,`saveEdit` | — | — | ARTIFACT | ARTIFACT |
| 4 | `editTags` | ref<string> | 同上（tags） | 同上 | — | — | ARTIFACT | ARTIFACT |
| 5 | `editText` | ref<string> | 同上（正文） | 同上 | — | — | ARTIFACT | ARTIFACT |
| 6 | `selected` | reactive<Set> | 知识条目多选（toggle + requestSync 同步勾选） | `toggle`,`requestSync` 读 | — | — | ARTIFACT | ARTIFACT |
| 7 | `fileEntries` | ref<DirEntry[]> | 文件浏览器当前目录列表 | `enterDir` | 读 FS | — | FILES | FILES |
| 8 | `filePath` | ref<string> | 文件浏览器/overlay 编辑器当前位置（地址真源） | `enterDir`,`openFile`,`openFileInline`,`openMd` | — | — | FILES | FILES |
| 9 | `pathInput` | ref<string> | 地址栏输入缓冲（镜像 filePath） | `goPath`,`enterDir` | — | — | FILES | FILES |
| 10 | `fileContent` | ref<string> | overlay 编辑器内容 | `openFile`,`openMd`,`saveFile` | 写 FS | — | FILES | FILES |
| 11 | `editingFile` | ref<boolean> | overlay 编辑模式 | `openFile`,`saveFile` | — | — | FILES | FILES |
| 12 | `mdPreview` | ref<boolean> | Markdown 预览模式 | `openFile`,`openMd` | — | — | FILES | FILES |
| 13 | `mdHtml` | ref<string> | 渲染后的 MD HTML | `openMd`,`openFileInline`,`saveInline` | — | — | FILES | FILES |
| 14 | `startDirs` | ref<DirEntry[]> | 起始目录集合（多根） | `loadStartDirs`,`refreshTree`,`locateTo` | 读 FS | — | FILES | FILES |
| 15 | `previewDir` | ref<string> | 目录预览（图片画廊）当前目录 | `openDirPreview` | — | — | FILES | FILES |
| 16 | `previewEntries` | ref<DirEntry[]> | 目录预览条目 | `openDirPreview` | 读 FS | — | FILES | FILES |
| 17 | `previewImages` | ref<Record> | 目录预览缩略图缓存（响应式） | `putPreviewImage`,`openDirPreview` | — | — | FILES | IMPLEMENTATION_DETAIL→FILES |
| 18 | `previewImageErrors` | ref<Record> | 缩略图错误 | `putPreviewImageError` | — | — | FILES | IMPLEMENTATION_DETAIL→FILES |
| 19 | `previewLoading` | ref<boolean> | 目录预览加载中 | `openDirPreview` | — | — | FILES | FILES |
| 20 | `previewError` | ref<string> | 目录预览错误 | `openDirPreview` | — | — | FILES | FILES |
| 21 | `previewTileSize` | ref<number> | 缩略图瓦片尺寸 | `setPreviewTileSize` | — | — | FILES | FILES |
| 22 | `compareImages` | ref<DirEntry[]> | 对比图集 | `toggleCompareImage`,`moveCompareImage`,`clearCompareImages` | — | — | FILES | FILES |
| 23 | `inlineFile` | ref<string> | 行内编辑器当前文件 | `openFileInline`,`closeInline`,`saveInline` | — | — | FILES | FILES |
| 24 | `inlineText` | ref<string> | 行内编辑缓冲 | `openFileInline`,`inlineToggleEdit`,`saveInline` | — | — | FILES | FILES |
| 25 | `inlineIsMd` | ref<boolean> | 行内是否 MD | `openFileInline` | — | — | FILES | FILES |
| 26 | `inlineEdit` | ref<boolean> | 行内编辑/预览切换 | `inlineToggleEdit`,`openFileInline` | — | — | FILES | FILES |
| 27 | `inlineHtml` | ref<string> | 行内 MD 渲染 | `openFileInline`,`inlineToggleEdit`,`saveInline` | — | — | FILES | FILES |
| 28 | `repos` | ref<RepoConfig[]> | 已配置仓库列表 | `refresh`,`saveRepo` | 读后端 | 后端持久化 | REPO | REPO |
| 29 | `form` | reactive | 仓库配置表单（含 token 输入瞬时） | `saveRepo`,`loadGiteeExample` | — | — | REPO | REPO |
| 30 | `preview` | ref<SyncPreview\|null> | 同步预览（待确认任务） | `requestSync`,`confirmSync`,`onSyncCompleted` | — | — | REPO | REPO |
| 31 | `busy` | ref<boolean> | 同步进行中 | `confirmSync`,`onSyncCompleted` | — | — | REPO | REPO |
| 32 | `job` | ref<SyncJob\|null> | 同步任务状态 | `confirmSync`,`onSyncCompleted` | — | — | REPO | REPO |
| 33 | `audit` | ref<AuditEntry[]> | 审计日志 | `refresh` | 读后端 | 后端持久化 | AUDIT | AUDIT |
| 34 | `scripts` | ref<ScriptMeta[]> | 脚本库列表 | `loadScripts`,`saveScript`,`removeScript` | 读/写后端 | 后端持久化 | SCRIPT | SCRIPT |
| 35 | `scriptForm` | reactive | 脚本表单 | `openScriptForm`,`saveScript` | — | — | SCRIPT | SCRIPT |
| 36 | `snippets` | ref<CommandSnippet[]> | 命令片段库列表 | `loadSnippets`,`saveSnippet`,`removeSnippet` | 读/写后端 | 后端持久化 | SNIPPET | SNIPPET |
| 37 | `snippetForm` | reactive | 片段表单 | `openSnippetForm`,`saveSnippet` | — | — | SNIPPET | SNIPPET |
| 38 | `recents` | reactive<RecentItem[]> | 最近访问（url + file 混合） | `addRecentUrl`,`addRecentFile`,`openRecent` | localStorage | localStorage | CROSS_DOMAIN | CROSS_DOMAIN（暂留 Workspace Core） |
| 39 | `fileCtx` | reactive | 文件右键菜单状态 | `onFileContext`,`closeFileCtx`,`ctxNewFile`,`ctxNewDir`,`ctxDelete`,`ctxRename`,`ctxFavorite`,`copyPath`,`ctxOpenInNewTab`,`ctxOpenInExplorer`,`ctxOpenInTerminal` | — | — | FILES | FILES |
| 40 | `ctxMenu` | reactive | 知识条目右键菜单状态 | `onArtifactContext`,`closeCtx`,`ctxReveal`,`ctxOpenSource`,`ctxRemove` | — | — | ARTIFACT | ARTIFACT |
| 41 | `treeRoots` | ref<DirEntry[]> | 文件树根（单根=当前目录 / 多根=startDirs） | `loadTree`,`refreshTree`,`locateTo`,`expandAllTree` | 读 FS | — | FILES | FILES |
| 42 | `treeChildren` | reactive<Map> | 文件树子节点缓存 | `ensureTreeChildren`,`refreshTree` | 读 FS | — | FILES | FILES |
| 43 | `treeExpanded` | reactive<Set> | 文件树展开集合 | `loadTree`,`toggleTreeDir`,`refreshTree`,`locateTo`,`expandTreeEntry` | — | — | FILES | FILES |
| 44 | `treeLoading` | reactive<Set> | 文件树加载集合 | `ensureTreeChildren` | — | — | FILES | FILES |
| 45 | `treeErrors` | reactive<Map> | 文件树错误 | `ensureTreeChildren`,`refreshTree` | — | — | FILES | FILES |
| 46 | `locateTarget` | ref<string> | 文件树定位高亮 | `locateTo`,`collapseAllTree` | — | — | FILES | FILES |
| 47 | `dragSource` | ref<string> | 拖拽源路径 | `startDrag`,`onDirDrop` | — | — | FILES | FILES |
| 48 | `dropTarget` | ref<string> | 拖拽目标路径 | `onDirDragOver`,`onDirDrop` | — | — | FILES | FILES |
| 49 | `moveConfirm` | reactive | 移动确认弹窗状态 | `requestMove`,`cancelMove`,`confirmMove` | — | — | FILES | FILES |
| 50 | `currentLocalPath` | computed | 派生：inlineFile‖previewDir‖modTabs.path‖filePath | 派生 | — | 派生 | FILES | FILES |
| 51 | `flatArtifacts` | computed | 知识条目扁平视图 | 派生 | — | 派生 | ARTIFACT | ARTIFACT |
| 52 | `previewRequest` | let | 目录预览请求纪元（内部） | `openDirPreview` | — | — | FILES | IMPLEMENTATION_DETAIL |
| 53 | `previewImageGeneration` | let | 缩略图队列代际（内部） | 队列函数 | — | — | FILES | IMPLEMENTATION_DETAIL |
| 54 | `previewImageActive` | let | 缩略图并发计数（内部） | 队列函数 | — | — | FILES | IMPLEMENTATION_DETAIL |
| 55 | `previewImageQueue` | const[] | 缩略图待加载队列（内部） | 队列函数 | — | — | FILES | IMPLEMENTATION_DETAIL |
| 56 | `previewImagePending` | const Set | 缩略图 pending（内部） | 队列函数 | — | — | FILES | IMPLEMENTATION_DETAIL |
| 57 | `previewImageCache` | const Map | 缩略图 LRU 缓存（内部） | `rememberPreviewImage` | — | — | FILES | IMPLEMENTATION_DETAIL |

---

## Action 归属（不重复列 Reads，仅标 Domain）

| Action | Domain | 备注 |
|--------|--------|------|
| `loadScripts/openScriptForm/saveScript/removeScript` | SCRIPT | |
| `loadSnippets/openSnippetForm/saveSnippet/removeSnippet` | SNIPPET | |
| `onFileContext/closeFileCtx/ctxBaseDir/ctxNewFile/ctxNewDir/currentBaseDir/quickNew/ctxDelete/ctxRename/ctxFavorite/copyPath/ctxOpenInNewTab/ctxOpenInExplorer/ctxOpenInTerminal` | FILES | |
| `refresh` | WORKSPACE_CORE（编排 browseWorkspace+listRepos+auditLog） | 跨 Knowledge/Repo/Audit |
| `openArtifact/saveEdit/removeArtifact/onArtifactContext/closeCtx/ctxReveal/ctxOpenSource/ctxRemove/toggle` | ARTIFACT | |
| `ensureTreeChildren/loadTree/toggleTreeDir/refreshTree/expandAllTree/collapseAllTree/expandTreeEntry/locateTo/locateCurrent` | FILES | |
| `startDrag/onDirDragOver/onDirDragLeave/onDirDrop/requestMove/cancelMove/confirmMove` | FILES | |
| `openFileInline/inlineToggleEdit/saveInline/closeInline/isImageEntry/rememberPreviewImage/putPreviewImage/putPreviewImageError/clearPreviewImageQueue/drainPreviewImageQueue/loadPreviewImage/openDirPreview/setPreviewTileSize/toggleCompareImage/moveCompareImage/clearCompareImages` | FILES | |
| `loadStartDirs/enterDir/goUp/goPath/openFile/openMd/saveFile` | FILES | |
| `saveRepo/requestSync/confirmSync/onSyncCompleted/loadGiteeExample` | REPO | |
| `loadRecents/addRecentUrl/addRecentFile/openRecent` | CROSS_DOMAIN（recents） | |
| `collectSelection` | ARTIFACT（采集到知识库） | 经 refresh 编排 |
| `enterDirCompat` | FILES | `enterDir` 别名（兼容） |

---

## 结论（UNKNOWN = 0）

- 真实域数 = **7 + 1 跨域**：FILES / ARTIFACT(Knowledge·笔记 Vault) / SCRIPT / SNIPPET / REPO(Sync) / AUDIT / WORKSPACE_CORE / CROSS_DOMAIN(recents)
- 接管指令候选 `editor` 实为 FILES 编辑表面（不独立）；`vault` 实为 ARTIFACT；`git` 不在本 store（GitPanel 走 bridge）。
- 第一刀（8C-0A）只抽 **FILES**（文件系统浏览/编辑/树/目录预览/行内编辑/右键/拖拽移动）—— 它是 store 内最大且最自洽的子域，且被 `FilePanel/FileTreeNode/FileEditor` 独占消费。
- `IMPLEMENTATION_DETAIL`（缩略图队列/缓存）随 FILES 一起搬，但不在 registry 公开治理列中。
- 其余 6 域在 8C-0B..0E 与后续阶段处理；本阶段不动它们。
